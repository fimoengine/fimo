//! Definition of an object-aware arc type.
// This is a heavily modified version of the arc and weak types
// found in the std library, which is dual-licensed under Apache 2.0 and MIT
// terms.

use crate::obj_box::{ObjBox, PtrDrop, WriteCloneIntoRaw};
use crate::object::{ObjPtrCompat, ObjectWrapper};
use crate::raw::CastError;
use crate::vtable::IBaseInterface;
use crate::{CoerceObject, Object};
use std::alloc::{Allocator, Global, Layout};
use std::borrow::Borrow;
use std::cmp::Ordering;
use std::error::Error;
use std::fmt::{Debug, Display, Formatter, Pointer};
use std::hash::{Hash, Hasher};
use std::marker::PhantomData;
use std::mem::MaybeUninit;
use std::ops::Deref;
use std::panic::{RefUnwindSafe, UnwindSafe};
use std::process::abort;
use std::ptr::NonNull;
use std::sync::atomic;
use std::sync::atomic::Ordering::{Acquire, Relaxed, Release, SeqCst};

#[cfg(test)]
mod test;

/// A soft limit on the amount of references that may be made to an `ObjArc`.
///
/// Going above this limit will abort your program (although not
/// necessarily) at _exactly_ `MAX_REFCOUNT + 1` references.
const MAX_REFCOUNT: usize = (isize::MAX) as usize;

#[cfg(not(sanitize = "thread"))]
macro_rules! acquire {
    ($x:expr) => {
        atomic::fence(Acquire)
    };
}

// ThreadSanitizer does not support memory fences. To avoid false positive
// reports in Arc / Weak implementation use atomic loads for synchronization
// instead.
#[cfg(sanitize = "thread")]
macro_rules! acquire {
    ($x:expr) => {
        $x.load(Acquire)
    };
}

/// A reference-counted pointer type for heap allocation, akin to an [`std::sync::Arc`].
#[repr(C)]
pub struct ObjArc<T: ObjPtrCompat + ?Sized, A: Allocator = Global> {
    ptr: NonNull<ObjArcInner<T>>,
    phantom: PhantomData<ObjArcInner<T>>,
    alloc: A,
}

impl<T> ObjArc<T> {
    /// Constructs a new `ObjArc` using the provided value.
    ///
    /// # Examples
    ///
    /// ```
    /// use fimo_object::ObjArc;
    ///
    /// let five = ObjArc::new(5);
    /// ```
    #[inline]
    pub fn new(data: T) -> ObjArc<T> {
        ObjArc::new_in(data, Global)
    }

    /// Constructs a new `ObjArc` with uninitialized contents.
    ///
    /// # Examples
    ///
    /// ```
    /// use fimo_object::ObjArc;
    ///
    /// let mut five = ObjArc::<u32>::new_uninit();
    ///
    /// let five = unsafe {
    ///     // Deferred initialization:
    ///     ObjArc::get_mut_unchecked(&mut five).as_mut_ptr().write(5);
    ///
    ///     five.assume_init()
    /// };
    ///
    /// assert_eq!(*five, 5)
    /// ```
    #[inline]
    #[must_use]
    pub fn new_uninit() -> ObjArc<MaybeUninit<T>> {
        ObjArc::new_uninit_in(Global)
    }

    /// Constructs a new `ObjArc` with zeroed contents.
    ///
    /// # Examples
    ///
    /// ```
    /// use fimo_object::ObjArc;
    ///
    /// let zero = ObjArc::<u32>::new_zeroed();
    /// let zero = unsafe { zero.assume_init() };
    ///
    /// assert_eq!(*zero, 0)
    /// ```
    #[inline]
    #[must_use]
    pub fn new_zeroed() -> ObjArc<MaybeUninit<T>> {
        ObjArc::new_zeroed_in(Global)
    }
}

impl<T, A: Allocator> ObjArc<T, A> {
    /// Constructs a new `ObjArc` using the provided value and allocator.
    ///
    /// # Examples
    ///
    /// ```
    /// #![feature(allocator_api)]
    ///
    /// use std::alloc::Global;
    /// use fimo_object::ObjArc;
    ///
    /// let five = ObjArc::new_in(5, Global);
    /// ```
    #[inline]
    pub fn new_in(data: T, alloc: A) -> ObjArc<T, A> {
        // construct an uninitialized version and write the value into it.
        let mut uninit = ObjArc::new_uninit_in(alloc);

        // safety: we can acquire a mutable reference, because at this point
        // we know that we are the only ones owning the arc.
        unsafe {
            ObjArc::get_mut_unchecked(&mut uninit).write(data);
            uninit.assume_init()
        }
    }

    /// Constructs a new `ObjArc` with uninitialized contents using the provided allocator.
    ///
    /// # Examples
    ///
    /// ```
    /// #![feature(allocator_api)]
    ///
    /// use std::alloc::Global;
    /// use fimo_object::ObjArc;
    ///
    /// let mut five = ObjArc::<u32>::new_uninit_in(Global);
    ///
    /// let five = unsafe {
    ///     // Deferred initialization:
    ///     ObjArc::get_mut_unchecked(&mut five).as_mut_ptr().write(5);
    ///
    ///     five.assume_init()
    /// };
    ///
    /// assert_eq!(*five, 5)
    /// ```
    #[inline]
    #[must_use]
    pub fn new_uninit_in(alloc: A) -> ObjArc<MaybeUninit<T>, A> {
        let x = ObjBox::new_in(
            ObjArcInner {
                strong: atomic::AtomicUsize::new(1),
                weak: atomic::AtomicUsize::new(1),
                data: MaybeUninit::<T>::uninit(),
            },
            alloc,
        );

        let (ptr, alloc) = ObjBox::into_raw_parts(x);

        ObjArc {
            // safety: we know that ptr is not null because it comes from an `ObjBox`.
            ptr: unsafe { NonNull::new_unchecked(ptr) },
            phantom: Default::default(),
            alloc,
        }
    }

    /// Constructs a new `ObjArc` with zeroed contents using the provided allocator.
    ///
    /// # Examples
    ///
    /// ```
    /// #![feature(allocator_api)]
    ///
    /// use std::alloc::Global;
    /// use fimo_object::ObjArc;
    ///
    /// let zero = ObjArc::<u32>::new_zeroed_in(Global);
    /// let zero = unsafe { zero.assume_init() };
    ///
    /// assert_eq!(*zero, 0)
    /// ```
    #[inline]
    #[must_use]
    pub fn new_zeroed_in(alloc: A) -> ObjArc<MaybeUninit<T>, A> {
        let x = ObjBox::new_in(
            ObjArcInner {
                strong: atomic::AtomicUsize::new(1),
                weak: atomic::AtomicUsize::new(1),
                data: MaybeUninit::<T>::zeroed(),
            },
            alloc,
        );

        let (ptr, alloc) = ObjBox::into_raw_parts(x);

        ObjArc {
            // safety: we know that ptr is not null because it comes from an `ObjBox`.
            ptr: unsafe { NonNull::new_unchecked(ptr) },
            phantom: Default::default(),
            alloc,
        }
    }
}

impl<O: ObjectWrapper + ?Sized, A: Allocator> ObjArc<O, A> {
    /// Coerces a `ObjArc<T, A>` to an `ObjArc<O, A>`.
    ///
    /// # Examples
    ///
    /// ```
    /// #![feature(const_fn_trait_bound)]
    /// #![feature(const_fn_fn_ptr_basics)]
    ///
    /// use fimo_object::{CoerceObject, fimo_vtable, is_object, impl_vtable, ObjArc, Object};
    /// use fimo_object::vtable::ObjectID;
    /// use fimo_object::object::{ObjectWrapper, ObjPtrCompat};
    ///
    /// // Define a custom interface vtable.
    /// fimo_vtable! {
    ///     #![uuid(0x59dc47cf, 0xfd2e, 0x4d58, 0xbcd4, 0x5a31adc68a44)]
    ///     struct ObjVTable {
    ///         add: fn(*const (), usize) -> usize
    ///     }
    /// }
    ///
    /// // Define a custom object implementing the interface.
    /// struct MyObj(usize);
    /// is_object! { #![uuid(0x5a7cc7de, 0x541d, 0x4fc2, 0xbc7e, 0xa799b180ee1e)] MyObj }
    /// impl_vtable! {
    ///     impl inline ObjVTable => MyObj {
    ///         |this, num| unsafe { (*(this as *const MyObj)).0 + num }
    ///     }
    /// }
    ///
    /// // Helper type for accessing the interface.
    /// struct Obj {
    ///     inner: Object<ObjVTable>
    /// }
    /// impl Obj {
    ///     pub fn add(&self, num: usize) -> usize {
    ///         let (ptr, vtable) = fimo_object::object::into_raw_parts(&self.inner);
    ///         (vtable.add)(ptr, num)
    ///     }
    /// }
    /// unsafe impl ObjPtrCompat for Obj {}
    /// unsafe impl ObjectWrapper for Obj {
    ///     type VTable = ObjVTable;
    ///
    ///     fn as_object_raw(ptr: *const Self) -> *const Object<Self::VTable> {
    ///         // `*const Self` and `*const Object<_>` have the same layout.
    ///         ptr as *const Object<_>
    ///     }
    ///
    ///     fn from_object_raw(obj: *const Object<Self::VTable>) -> *const Self {
    ///         // `*const Self` and `*const Object<_>` have the same layout.
    ///         obj as *const Self
    ///     }
    /// }
    ///
    /// let x = ObjArc::new(MyObj(5));
    /// assert_eq!(x.0, 5);
    ///
    /// let x: ObjArc<Obj> = ObjArc::coerce_object(x);
    /// assert_eq!(x.add(0), 5);
    /// assert_eq!(x.add(1), 6);
    /// assert_eq!(x.add(5), 10);
    /// ```
    pub fn coerce_object<T: CoerceObject<O::VTable>>(a: ObjArc<T, A>) -> ObjArc<O, A> {
        let (ptr, alloc) = ObjArc::into_raw_parts(a);
        let obj = T::coerce_obj_raw(ptr);
        let ptr = O::from_object_raw(obj);
        unsafe { ObjArc::from_raw_parts(ptr, alloc) }
    }

    /// Tries to revert from an `ObjArc<O, A>` to an `ObjArc<T, A>`.
    ///
    /// # Examples
    ///
    /// ```
    /// #![feature(const_fn_trait_bound)]
    /// #![feature(const_fn_fn_ptr_basics)]
    ///
    /// use fimo_object::{CoerceObject, fimo_vtable, is_object, impl_vtable, ObjArc, Object};
    /// use fimo_object::vtable::ObjectID;
    ///
    /// // Define a custom interface vtable.
    /// fimo_vtable! {
    ///     #![uuid(0x07b2f853, 0x3beb, 0x4abf, 0xb782, 0xd502fcb2ea7b)]
    ///     struct ObjVTable;
    /// }
    ///
    /// // Define custom objects implementing the interface.
    /// struct FirstObj(usize);
    /// struct SecondObj;
    /// is_object! { #![uuid(0xe98df5c9, 0x1d0e, 0x4289, 0x8f93, 0x55037e65c725)] FirstObj }
    /// is_object! { #![uuid(0xabd4e0e7, 0xf8bd, 0x41cc, 0xbb1d, 0x7f419ebf0315)] SecondObj }
    /// impl_vtable! { impl ObjVTable => FirstObj {} }
    /// impl_vtable! { impl ObjVTable => SecondObj {} }
    ///
    /// let x = ObjArc::new(FirstObj(5));
    /// let obj: ObjArc<Object<ObjVTable>> = ObjArc::coerce_object(x);
    /// let also_obj = ObjArc::clone(&obj);
    /// assert_eq!(ObjArc::try_object_cast::<FirstObj>(obj).unwrap().0, 5);
    /// assert!(ObjArc::try_object_cast::<SecondObj>(also_obj).is_err());
    ///
    /// ```
    pub fn try_object_cast<T: CoerceObject<O::VTable>>(
        a: ObjArc<O, A>,
    ) -> Result<ObjArc<T, A>, CastError<ObjArc<O, A>>> {
        let (ptr, alloc) = ObjArc::into_raw_parts(a);
        let obj = O::as_object_raw(ptr);

        unsafe {
            match Object::<O::VTable>::try_cast_obj_raw::<T>(obj) {
                Ok(casted) => Ok(ObjArc::from_raw_parts(casted, alloc)),
                Err(err) => Err(CastError {
                    obj: ObjArc::from_raw_parts(ptr, alloc),
                    required: err.required,
                    required_id: err.required_id,
                    available: err.available,
                    available_id: err.available_id,
                }),
            }
        }
    }

    /// Tries casting the object to another object.
    ///
    /// # Examples
    ///
    /// ```
    /// #![feature(const_fn_trait_bound)]
    /// #![feature(const_fn_fn_ptr_basics)]
    ///
    /// use fimo_object::{CoerceObject, fimo_vtable, is_object, impl_vtable, ObjArc, Object};
    /// use fimo_object::vtable::ObjectID;
    /// use fimo_object::object::{ObjectWrapper, ObjPtrCompat};
    ///
    /// // Define a custom interface vtable.
    /// fimo_vtable! {
    ///     #![uuid(0x0f42329a, 0x9abd, 0x44b6, 0xb03f, 0x70d2f82c809f)]
    ///     struct ObjVTable {
    ///         add: fn(*const (), usize) -> usize
    ///     }
    /// }
    ///
    /// // Define a custom object implementing the interface.
    /// struct MyObj(usize);
    /// is_object! { #![uuid(0x9558d810, 0x0053, 0x41a3, 0xa520, 0x9745f965567c)] MyObj }
    /// impl_vtable! {
    ///     impl inline ObjVTable => MyObj {
    ///         |this, num| unsafe { (*(this as *const MyObj)).0 + num }
    ///     }
    /// }
    ///
    /// // Helper type for accessing the interface.
    /// struct Obj {
    ///     inner: Object<ObjVTable>
    /// }
    /// impl Obj {
    ///     pub fn add(&self, num: usize) -> usize {
    ///         let (ptr, vtable) = fimo_object::object::into_raw_parts(&self.inner);
    ///         (vtable.add)(ptr, num)
    ///     }
    /// }
    /// unsafe impl ObjPtrCompat for Obj {}
    /// unsafe impl ObjectWrapper for Obj {
    ///     type VTable = ObjVTable;
    ///
    ///     fn as_object_raw(ptr: *const Self) -> *const Object<Self::VTable> {
    ///         // `*const Self` and `*const Object<_>` have the same layout.
    ///         ptr as *const Object<_>
    ///     }
    ///
    ///     fn from_object_raw(obj: *const Object<Self::VTable>) -> *const Self {
    ///         // `*const Self` and `*const Object<_>` have the same layout.
    ///         obj as *const Self
    ///     }
    /// }
    ///
    /// let x = ObjArc::new(MyObj(5));
    /// assert_eq!(x.0, 5);
    ///
    /// let x: ObjArc<Object<ObjVTable>> = ObjArc::coerce_object(x);
    /// let x: ObjArc<Obj> = ObjArc::try_cast(x).unwrap();
    /// assert_eq!(x.add(0), 5);
    /// assert_eq!(x.add(1), 6);
    /// assert_eq!(x.add(5), 10);
    /// ```
    pub fn try_cast<U: ObjectWrapper + ?Sized>(
        a: ObjArc<O, A>,
    ) -> Result<ObjArc<U, A>, CastError<ObjArc<O, A>>> {
        let (ptr, alloc) = ObjArc::into_raw_parts(a);
        let obj = O::as_object_raw(ptr);

        unsafe {
            match Object::<O::VTable>::try_cast_raw::<U::VTable>(obj) {
                Ok(casted) => Ok(ObjArc::from_raw_parts(U::from_object_raw(casted), alloc)),
                Err(err) => Err(CastError {
                    obj: ObjArc::from_raw_parts(ptr, alloc),
                    required: err.required,
                    required_id: err.required_id,
                    available: err.available,
                    available_id: err.available_id,
                }),
            }
        }
    }

    /// Casts an `ObjArc<O, A>` to an `ObjArc<Object<BaseInterface>>`.
    ///
    /// # Examples
    ///
    /// ```
    /// #![feature(const_fn_trait_bound)]
    /// #![feature(const_fn_fn_ptr_basics)]
    ///
    /// use fimo_object::{CoerceObject, fimo_vtable, is_object, impl_vtable, ObjArc, Object};
    /// use fimo_object::vtable::ObjectID;
    /// use fimo_object::object::{ObjectWrapper, ObjPtrCompat};
    ///
    /// // Define a custom interface vtable.
    /// fimo_vtable! {
    ///     #![uuid(0x93f88692, 0xceca, 0x4dc5, 0x813e, 0x8b008a0bb132)]
    ///     struct ObjVTable {
    ///         add: fn(*const (), usize) -> usize
    ///     }
    /// }
    ///
    /// // Define a custom object implementing the interface.
    /// struct MyObj(usize);
    /// is_object! { #![uuid(0xe0497698, 0xa0f1, 0x480e, 0xb158, 0x9cdbd7de426d)] MyObj }
    /// impl_vtable! {
    ///     impl inline ObjVTable => MyObj {
    ///         |this, num| unsafe { (*(this as *const MyObj)).0 + num }
    ///     }
    /// }
    ///
    /// // Helper type for accessing the interface.
    /// struct Obj {
    ///     inner: Object<ObjVTable>
    /// }
    /// impl Obj {
    ///     pub fn add(&self, num: usize) -> usize {
    ///         let (ptr, vtable) = fimo_object::object::into_raw_parts(&self.inner);
    ///         (vtable.add)(ptr, num)
    ///     }
    /// }
    /// unsafe impl ObjPtrCompat for Obj {}
    /// unsafe impl ObjectWrapper for Obj {
    ///     type VTable = ObjVTable;
    ///
    ///     fn as_object_raw(ptr: *const Self) -> *const Object<Self::VTable> {
    ///         // `*const Self` and `*const Object<_>` have the same layout.
    ///         ptr as *const Object<_>
    ///     }
    ///
    ///     fn from_object_raw(obj: *const Object<Self::VTable>) -> *const Self {
    ///         // `*const Self` and `*const Object<_>` have the same layout.
    ///         obj as *const Self
    ///     }
    /// }
    ///
    /// let x = ObjArc::new(MyObj(5));
    /// assert_eq!(x.0, 5);
    ///
    /// let x: ObjArc<Obj> = ObjArc::coerce_object(x);
    /// let x = ObjArc::cast_base(x);
    /// let x: ObjArc<Obj> = ObjArc::try_cast(x).unwrap();
    /// assert_eq!(x.add(0), 5);
    /// assert_eq!(x.add(1), 6);
    /// assert_eq!(x.add(5), 10);
    /// ```
    pub fn cast_base(a: ObjArc<O, A>) -> ObjArc<Object<IBaseInterface>, A> {
        let (ptr, alloc) = ObjArc::into_raw_parts(a);
        let obj = O::as_object_raw(ptr);
        let obj = Object::<O::VTable>::cast_base_raw(obj);
        unsafe { ObjArc::from_raw_parts(obj, alloc) }
    }
}

impl<T, A: Allocator> ObjArc<MaybeUninit<T>, A> {
    /// Converts to `ObjArc<T, A>`.
    ///
    /// # Safety
    ///
    /// See [std::sync::Arc::assume_init].
    ///
    /// # Examples
    ///
    /// ```
    /// use fimo_object::ObjArc;
    ///
    /// let mut five = ObjArc::<u32>::new_uninit();
    ///
    /// let five = unsafe {
    ///     // Deferred initialization:
    ///     ObjArc::get_mut_unchecked(&mut five).as_mut_ptr().write(5);
    ///
    ///     five.assume_init()
    /// };
    ///
    /// assert_eq!(*five, 5)
    /// ```
    #[inline]
    pub unsafe fn assume_init(self) -> ObjArc<T, A> {
        let (ptr, alloc) = ObjArc::into_raw_parts(self);
        ObjArc::from_raw_parts(ptr as *const T, alloc)
    }
}

impl<T: ObjPtrCompat + ?Sized> ObjArc<T> {
    /// Constructs an `ObjArc<T>` from a raw pointer.
    ///
    /// # Safety
    ///
    /// See [std::sync::Arc::from_raw].
    ///
    /// # Safety
    ///
    /// ```
    /// use fimo_object::ObjArc;
    ///
    /// let x = ObjArc::new("hello".to_owned());
    /// let x_ptr = ObjArc::into_raw(x);
    ///
    /// unsafe {
    ///     // Convert back to an `ObjArc` to prevent leak.
    ///     let x = ObjArc::from_raw(x_ptr);
    ///     assert_eq!(&*x, "hello");
    ///
    ///     // Further calls to `ObjArc::from_raw(x_ptr)` would be memory-unsafe.
    /// }
    ///
    /// // The memory was freed when `x` went out of scope above, so `x_ptr` is now dangling!
    /// ```
    #[inline]
    pub unsafe fn from_raw(ptr: *const T) -> ObjArc<T> {
        ObjArc::from_raw_parts(ptr, Global)
    }

    /// Increments the strong reference count on the `ObjArc<T>` associated with the
    /// provided pointer by one.
    ///
    /// # Safety
    ///
    /// See [std::sync::Arc::increment_strong_count].
    ///
    /// # Examples
    ///
    /// ```
    /// use fimo_object::ObjArc;
    ///
    /// let five = ObjArc::new(5);
    ///
    /// unsafe {
    ///     let ptr = ObjArc::into_raw(five);
    ///     ObjArc::increment_strong_count(ptr);
    ///
    ///     // This assertion is deterministic because we haven't shared
    ///     // the `ObjArc` between threads.
    ///     let five = ObjArc::from_raw(ptr);
    ///     assert_eq!(2, ObjArc::strong_count(&five));
    /// }
    /// ```
    #[inline]
    pub unsafe fn increment_strong_count(ptr: *const T) {
        ObjArc::increment_strong_count_in(ptr, Global)
    }

    /// Decrements the strong reference count on the `ObjArc<T>` associated with the
    /// provided pointer by one.
    ///
    /// # Safety
    ///
    /// See [std::sync::Arc::decrement_strong_count].
    ///
    /// # Examples
    ///
    /// ```
    /// use fimo_object::ObjArc;
    ///
    /// let five = ObjArc::new(5);
    ///
    /// unsafe {
    ///     let ptr = ObjArc::into_raw(five);
    ///     ObjArc::increment_strong_count(ptr);
    ///
    ///     // This assertion is deterministic because we haven't shared
    ///     // the `ObjArc` between threads.
    ///     let five = ObjArc::from_raw(ptr);
    ///     assert_eq!(2, ObjArc::strong_count(&five));
    ///     ObjArc::decrement_strong_count(ptr);
    ///     assert_eq!(1, ObjArc::strong_count(&five));
    /// }
    /// ```
    #[inline]
    pub unsafe fn decrement_strong_count(ptr: *const T) {
        ObjArc::decrement_strong_count_in(ptr, Global)
    }
}

impl<T: ObjPtrCompat + ?Sized, A: Allocator> ObjArc<T, A> {
    /// Constructs an `ObjArc<T>` from a raw pointer and the allocator.
    ///
    /// # Safety
    ///
    /// See [std::sync::Arc::from_raw].
    ///
    /// # Safety
    ///
    /// ```
    /// #![feature(allocator_api)]
    ///
    /// use std::alloc::Global;
    /// use fimo_object::ObjArc;
    ///
    /// let x = ObjArc::new_in("hello".to_owned(), Global);
    /// let (x_ptr, x_alloc) = ObjArc::into_raw_parts(x);
    ///
    /// unsafe {
    ///     // Convert back to an `ObjArc` to prevent leak.
    ///     let x = ObjArc::from_raw_parts(x_ptr, x_alloc);
    ///     assert_eq!(&*x, "hello");
    ///
    ///     // Further calls to `ObjArc::from_raw_parts(x_ptr, x_alloc)` would be memory-unsafe.
    /// }
    ///
    /// // The memory was freed when `x` went out of scope above, so `x_ptr` is now dangling!
    /// ```
    #[inline]
    pub unsafe fn from_raw_parts(ptr: *const T, alloc: A) -> ObjArc<T, A> {
        let offset = data_offset(ptr);

        // reverse the offset to find the original `ObjArcInner`.
        let arc_ptr = (ptr as *mut ObjArcInner<T>).set_ptr_value((ptr as *mut u8).offset(-offset));

        ObjArc {
            ptr: NonNull::new_unchecked(arc_ptr),
            phantom: Default::default(),
            alloc,
        }
    }

    /// Consumes the `ObjArc`, returning the wrapped pointer.
    ///
    /// # Examples
    ///
    /// ```
    /// use fimo_object::ObjArc;
    ///
    /// let x = ObjArc::new("hello".to_owned());
    /// let x_ptr = ObjArc::into_raw(x);
    /// assert_eq!(unsafe { &*x_ptr }, "hello");
    /// ```
    #[inline]
    pub fn into_raw(this: ObjArc<T, A>) -> *const T {
        let ptr: *const T = unsafe { std::ptr::addr_of!((*this.ptr.as_ptr()).data) };
        std::mem::forget(this);
        ptr
    }

    /// Consumes the `ObjArc`, returning the wrapped pointer and allocator.
    ///
    /// # Examples
    ///
    /// ```
    /// #![feature(allocator_api)]
    ///
    /// use std::alloc::Global;
    /// use fimo_object::ObjArc;
    ///
    /// let x = ObjArc::new_in("hello".to_owned(), Global);
    /// let (x_ptr, x_alloc) = ObjArc::into_raw_parts(x);
    /// assert_eq!(unsafe { &*x_ptr }, "hello");
    /// ```
    #[inline]
    pub fn into_raw_parts(this: ObjArc<T, A>) -> (*const T, A) {
        let (ptr, alloc): (*const T, A) = unsafe {
            (
                std::ptr::addr_of!((*this.ptr.as_ptr()).data),
                std::ptr::read(&this.alloc),
            )
        };

        std::mem::forget(this);
        (ptr, alloc)
    }

    /// Provides a raw pointer to the data.
    ///
    /// # Examples
    ///
    /// ```
    /// use fimo_object::ObjArc;
    ///
    /// let x = ObjArc::new("hello".to_owned());
    /// let y = ObjArc::clone(&x);
    /// let x_ptr = ObjArc::as_ptr(&x);
    /// assert_eq!(x_ptr, ObjArc::as_ptr(&y));
    /// assert_eq!(unsafe { &*x_ptr }, "hello");
    /// ```
    #[inline]
    #[must_use]
    pub fn as_ptr(this: &ObjArc<T, A>) -> *const T {
        unsafe { std::ptr::addr_of!((*this.ptr.as_ptr()).data) }
    }

    /// Returns a reference to the underlying allocator.
    ///
    /// # Examples
    ///
    /// ```
    /// #![feature(allocator_api)]
    ///
    /// use std::alloc::Global;
    /// use fimo_object::ObjArc;
    ///
    /// let x = ObjArc::new_in("hello".to_owned(), Global);
    /// let alloc = ObjArc::allocator(&x);
    /// ```
    #[inline]
    pub fn allocator(this: &ObjArc<T, A>) -> &A {
        &this.alloc
    }

    #[inline]
    fn inner(&self) -> &ObjArcInner<T> {
        // This unsafety is ok because while this arc is alive we're guaranteed
        // that the inner pointer is valid. Furthermore, we know that the
        // `ObjArcInner` structure itself is `Sync` because the inner data is
        // `Sync` as well, so we're ok loaning out an immutable pointer to these
        // contents.
        unsafe { self.ptr.as_ref() }
    }

    // Non-inlined part of `drop`.
    #[inline(never)]
    unsafe fn drop_slow(&mut self) {
        // Destroy the data at this time, even though we must not free the box
        // allocation itself (there might still be weak pointers lying around).
        PtrDrop::drop_in_place(Self::get_mut_unchecked(self));

        // Drop the weak ref collectively held by all strong references
        std::mem::drop(ObjWeak {
            ptr: self.ptr,
            alloc: &self.alloc,
        })
    }

    /// Gets the number of [`ObjWeak`] pointers to this allocation.
    ///
    /// # Examples
    ///
    /// ```
    /// use fimo_object::ObjArc;
    ///
    /// let five = ObjArc::new(5);
    /// let _weak_five = ObjArc::downgrade(&five);
    ///
    /// // This assertion is deterministic because we haven't shared
    /// // the `ObjArc` or `ObjWeak` between threads.
    /// assert_eq!(1, ObjArc::weak_count(&five));
    /// ```
    #[inline]
    pub fn weak_count(this: &Self) -> usize {
        let cnt = this.inner().weak.load(SeqCst);
        // If the weak count is currently locked, the value of the
        // count was 0 just before taking the lock.
        if cnt == usize::MAX {
            0
        } else {
            cnt - 1
        }
    }

    /// Gets the number of strong (`ObjArc`) pointers to this allocation.
    ///
    /// # Examples
    ///
    /// ```
    /// use fimo_object::ObjArc;
    ///
    /// let five = ObjArc::new(5);
    /// let _also_five = ObjArc::clone(&five);
    ///
    /// // This assertion is deterministic because we haven't shared
    /// // the `ObjArc` between threads.
    /// assert_eq!(2, ObjArc::strong_count(&five));
    /// ```
    #[inline]
    pub fn strong_count(this: &Self) -> usize {
        this.inner().strong.load(SeqCst)
    }

    /// Creates a new [`ObjWeak`] pointer to this allocation.
    ///
    /// # Examples
    ///
    /// ```
    /// use fimo_object::ObjArc;
    ///
    /// let five = ObjArc::new(5);
    ///
    /// let weak_five = ObjArc::downgrade(&five);
    /// ```
    #[inline]
    pub fn downgrade(this: &Self) -> ObjWeak<T, A>
    where
        A: Clone,
    {
        // This Relaxed is OK because we're checking the value in the CAS
        // below.
        let mut cur = this.inner().weak.load(Relaxed);

        loop {
            // check if the weak counter is currently "locked"; if so, spin.
            if cur == usize::MAX {
                std::hint::spin_loop();
                cur = this.inner().weak.load(Relaxed);
                continue;
            }

            // NOTE: this code currently ignores the possibility of overflow
            // into usize::MAX; in general both Rc and Arc need to be adjusted
            // to deal with overflow.

            // Unlike with Clone(), we need this to be an Acquire read to
            // synchronize with the write coming from `is_unique`, so that the
            // events prior to that write happen before this read.
            match this
                .inner()
                .weak
                .compare_exchange_weak(cur, cur + 1, Acquire, Relaxed)
            {
                Ok(_) => {
                    // Make sure we do not create a dangling Weak
                    debug_assert!(!is_dangling(this.ptr.as_ptr()));
                    return ObjWeak {
                        ptr: this.ptr,
                        alloc: this.alloc.clone(),
                    };
                }
                Err(old) => cur = old,
            }
        }
    }

    /// Increments the strong reference count on the `ObjArc<T>` associated with the
    /// provided pointer by one.
    ///
    /// # Safety
    ///
    /// See [std::sync::Arc::increment_strong_count].
    ///
    /// # Examples
    ///
    /// ```
    /// #![feature(allocator_api)]
    ///
    /// use std::alloc::Global;
    /// use fimo_object::ObjArc;
    ///
    /// let five = ObjArc::new_in(5, Global);
    ///
    /// unsafe {
    ///     let (ptr, alloc) = ObjArc::into_raw_parts(five);
    ///     ObjArc::increment_strong_count_in(ptr, alloc);
    ///
    ///     // This assertion is deterministic because we haven't shared
    ///     // the `ObjArc` between threads.
    ///     let five = ObjArc::from_raw_parts(ptr, alloc);
    ///     assert_eq!(2, ObjArc::strong_count(&five));
    /// }
    /// ```
    #[inline]
    pub unsafe fn increment_strong_count_in(ptr: *const T, alloc: A)
    where
        A: Clone,
    {
        // Retain Arc, but don't touch refcount by wrapping in ManuallyDrop
        let arc = std::mem::ManuallyDrop::new(ObjArc::from_raw_parts(ptr, alloc));
        // Now increase refcount, but don't drop new refcount either
        let _arc_clone: std::mem::ManuallyDrop<_> = arc.clone();
    }

    /// Decrements the strong reference count on the `ObjArc<T>` associated with the
    /// provided pointer by one.
    ///
    /// # Safety
    ///
    /// See [std::sync::Arc::decrement_strong_count].
    ///
    /// # Examples
    ///
    /// ```
    /// #![feature(allocator_api)]
    ///
    /// use std::alloc::Global;
    /// use fimo_object::ObjArc;
    ///
    /// let five = ObjArc::new_in(5, Global);
    ///
    /// unsafe {
    ///     let (ptr, alloc) = ObjArc::into_raw_parts(five);
    ///     ObjArc::increment_strong_count_in(ptr, alloc);
    ///
    ///     // This assertion is deterministic because we haven't shared
    ///     // the `ObjArc` between threads.
    ///     let five = ObjArc::from_raw_parts(ptr, alloc);
    ///     assert_eq!(2, ObjArc::strong_count(&five));
    ///     ObjArc::decrement_strong_count_in(ptr, alloc);
    ///     assert_eq!(1, ObjArc::strong_count(&five));
    /// }
    /// ```
    #[inline]
    pub unsafe fn decrement_strong_count_in(ptr: *const T, alloc: A) {
        std::mem::drop(ObjArc::from_raw_parts(ptr, alloc))
    }

    /// Returns `true` if the two `ObjArc`s point to the same allocation.
    ///
    /// # Examples
    ///
    /// ```
    /// use fimo_object::ObjArc;
    ///
    /// let five = ObjArc::new(5);
    /// let same_five = ObjArc::clone(&five);
    /// let other_five = ObjArc::new(5);
    ///
    /// assert!(ObjArc::ptr_eq(&five, &same_five));
    /// assert!(!ObjArc::ptr_eq(&five, &other_five));
    /// ```
    #[inline]
    pub fn ptr_eq(this: &Self, other: &Self) -> bool {
        this.ptr.as_ptr() == other.ptr.as_ptr()
    }
}

impl<T: ObjPtrCompat + ?Sized, A: Allocator> ObjArc<T, A> {
    /// Returns a mutable reference into the given `ObjArc`, if there are
    /// no other `ObjArc` or [`ObjWeak`] pointers to the same allocation.
    ///
    /// Returns [`None`] otherwise, because it is not safe to
    /// mutate a shared value.
    ///
    /// # Examples
    ///
    /// ```
    /// use fimo_object::ObjArc;
    ///
    /// let mut x = ObjArc::new(3);
    /// *ObjArc::get_mut(&mut x).unwrap() = 4;
    /// assert_eq!(*x, 4);
    ///
    /// let _y = ObjArc::clone(&x);
    /// assert!(ObjArc::get_mut(&mut x).is_none());
    /// ```
    #[inline]
    pub fn get_mut(this: &mut ObjArc<T, A>) -> Option<&mut T> {
        if this.is_unique() {
            // This unsafety is ok because we're guaranteed that the pointer
            // returned is the *only* pointer that will ever be returned to T. Our
            // reference count is guaranteed to be 1 at this point, and we required
            // the Arc itself to be `mut`, so we're returning the only possible
            // reference to the inner data.
            unsafe { Some(ObjArc::get_mut_unchecked(this)) }
        } else {
            None
        }
    }

    /// Returns a mutable reference into the given `ObjArc`,
    /// without any check.
    ///
    /// # Safety
    ///
    /// See [std::sync::Arc::get_mut_unchecked].
    ///
    /// # Examples
    ///
    /// ```
    /// use fimo_object::ObjArc;
    ///
    /// let mut x = ObjArc::new(String::new());
    /// unsafe {
    ///     ObjArc::get_mut_unchecked(&mut x).push_str("foo")
    /// }
    /// assert_eq!(*x, "foo");
    /// ```
    #[inline]
    pub unsafe fn get_mut_unchecked(this: &mut ObjArc<T, A>) -> &mut T {
        // We are careful to *not* create a reference covering the "count" fields, as
        // this would alias with concurrent access to the reference counts (e.g. by `ObjWeak`).
        &mut (*this.ptr.as_ptr()).data
    }

    /// Determine whether this is the unique reference (including weak refs) to
    /// the underlying data.
    ///
    /// Note that this requires locking the weak ref count.
    #[inline]
    fn is_unique(&mut self) -> bool {
        // lock the weak pointer count if we appear to be the sole weak pointer
        // holder.
        //
        // The acquire label here ensures a happens-before relationship with any
        // writes to `strong` (in particular in `Weak::upgrade`) prior to decrements
        // of the `weak` count (via `Weak::drop`, which uses release).  If the upgraded
        // weak ref was never dropped, the CAS here will fail so we do not care to synchronize.
        if self
            .inner()
            .weak
            .compare_exchange(1, usize::MAX, Acquire, Relaxed)
            .is_ok()
        {
            // This needs to be an `Acquire` to synchronize with the decrement of the `strong`
            // counter in `drop` -- the only access that happens when any but the last reference
            // is being dropped.
            let unique = self.inner().strong.load(Acquire) == 1;

            // The release write here synchronizes with a read in `downgrade`,
            // effectively preventing the above read of `strong` from happening
            // after the write.
            self.inner().weak.store(1, Release); // release the lock
            unique
        } else {
            false
        }
    }
}

impl<T: Clone, A: Allocator + Clone> ObjArc<T, A> {
    /// Makes a mutable reference into the given Arc.
    ///
    /// See [std::sync::Arc::make_mut].
    ///
    /// # Examples
    ///
    /// ```
    /// use fimo_object::ObjArc;
    ///
    /// let mut data = ObjArc::new(5);
    ///
    /// *ObjArc::make_mut(&mut data) += 1;          // Won't clone anything
    /// let mut other_data = ObjArc::clone(&data);  // Won't clone anything
    /// *ObjArc::make_mut(&mut data) += 1;          // Clones inner data
    /// *ObjArc::make_mut(&mut data) += 1;          // Won't clone anything
    /// *ObjArc::make_mut(&mut other_data) *= 2;    // Won't clone anything
    ///
    /// // Now `data` and `other_data` point to different allocations.
    /// assert_eq!(*data, 8);
    /// assert_eq!(*other_data, 12);
    /// ```
    #[inline]
    pub fn make_mut(this: &mut ObjArc<T, A>) -> &mut T {
        // Note that we hold both a strong reference and a weak reference.
        // Thus, releasing our strong reference only will not, by itself, cause
        // the memory to be deallocated.
        //
        // Use Acquire to ensure that we see any writes to `weak` that happen
        // before release writes (i.e., decrements) to `strong`. Since we hold a
        // weak count, there's no chance the ArcInner itself could be
        // deallocated.
        if this
            .inner()
            .strong
            .compare_exchange(1, 0, Acquire, Relaxed)
            .is_err()
        {
            // Another strong pointer exists, so we must clone.
            // Pre-allocate memory to allow writing the cloned value directly.
            let mut arc = Self::new_uninit_in(this.alloc.clone());
            unsafe {
                let data = ObjArc::get_mut_unchecked(&mut arc);
                (**this).write_clone_into_raw(data.as_mut_ptr());
                *this = arc.assume_init();
            }
        } else if this.inner().weak.load(Relaxed) != 1 {
            // Relaxed suffices in the above because this is fundamentally an
            // optimization: we are always racing with weak pointers being
            // dropped. Worst case, we end up allocated a new Arc unnecessarily.

            // We removed the last strong ref, but there are additional weak
            // refs remaining. We'll move the contents to a new Arc, and
            // invalidate the other weak refs.

            // Note that it is not possible for the read of `weak` to yield
            // usize::MAX (i.e., locked), since the weak count can only be
            // locked by a thread with a strong reference.

            // Materialize our own implicit weak pointer, so that it can clean
            // up the ArcInner as needed.
            let _weak = ObjWeak {
                ptr: this.ptr,
                alloc: this.alloc.clone(),
            };

            // Can just steal the data, all that's left is Weaks
            let mut arc = Self::new_uninit_in(this.alloc.clone());
            unsafe {
                let data = ObjArc::get_mut_unchecked(&mut arc);
                data.as_mut_ptr().copy_from_nonoverlapping(&**this, 1);
                std::ptr::write(this, arc.assume_init());
            }
        } else {
            // We were the sole reference of either kind; bump back up the
            // strong ref count.
            this.inner().strong.store(1, Release);
        }

        // As with `get_mut()`, the unsafety is ok because our reference was
        // either unique to begin with, or became one upon cloning the contents.
        unsafe { Self::get_mut_unchecked(this) }
    }
}

unsafe impl<T: ObjPtrCompat + ?Sized + Sync + Send, A: Allocator + Send> Send for ObjArc<T, A> {}

unsafe impl<T: ObjPtrCompat + ?Sized + Sync + Send, A: Allocator + Sync> Sync for ObjArc<T, A> {}

impl<T: ObjPtrCompat + RefUnwindSafe + ?Sized, A: Allocator + UnwindSafe> UnwindSafe
    for ObjArc<T, A>
{
}

impl<T: ObjPtrCompat + ?Sized, A: Allocator> Unpin for ObjArc<T, A> {}

impl<T: ObjPtrCompat + ?Sized, A: Allocator> AsRef<T> for ObjArc<T, A> {
    #[inline]
    fn as_ref(&self) -> &T {
        &**self
    }
}

impl<T: ObjPtrCompat + ?Sized, A: Allocator> Borrow<T> for ObjArc<T, A> {
    #[inline]
    fn borrow(&self) -> &T {
        &**self
    }
}

impl<T: ObjPtrCompat + ?Sized, A: Allocator + Clone> Clone for ObjArc<T, A> {
    #[inline]
    fn clone(&self) -> Self {
        // Using a relaxed ordering is alright here, as knowledge of the
        // original reference prevents other threads from erroneously deleting
        // the object.
        //
        // As explained in the [Boost documentation][1], Increasing the
        // reference counter can always be done with memory_order_relaxed: New
        // references to an object can only be formed from an existing
        // reference, and passing an existing reference from one thread to
        // another must already provide any required synchronization.
        //
        // [1]: (www.boost.org/doc/libs/1_55_0/doc/html/atomic/usage_examples.html)
        let old_size = self.inner().strong.fetch_add(1, Relaxed);

        // However we need to guard against massive refcounts in case someone
        // is `mem::forget`ing Arcs. If we don't do this the count can overflow
        // and users will use-after free. We racily saturate to `isize::MAX` on
        // the assumption that there aren't ~2 billion threads incrementing
        // the reference count at once. This branch will never be taken in
        // any realistic program.
        //
        // We abort because such a program is incredibly degenerate, and we
        // don't care to support it.
        if old_size > MAX_REFCOUNT {
            abort();
        }

        Self {
            ptr: self.ptr,
            phantom: Default::default(),
            alloc: self.alloc.clone(),
        }
    }
}

impl<T: ObjPtrCompat + ?Sized + Debug, A: Allocator> Debug for ObjArc<T, A> {
    #[inline]
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        std::fmt::Debug::fmt(&**self, f)
    }
}

impl<T: ObjPtrCompat + ?Sized + Display, A: Allocator> Display for ObjArc<T, A> {
    #[inline]
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(&**self, f)
    }
}

impl<T: ObjPtrCompat + ?Sized, A: Allocator> Pointer for ObjArc<T, A> {
    #[inline]
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        std::fmt::Pointer::fmt(&(&**self as *const T), f)
    }
}

impl<T: Default, A: Allocator + Default> Default for ObjArc<T, A> {
    #[inline]
    fn default() -> Self {
        ObjArc::new_in(Default::default(), Default::default())
    }
}

impl<T: ObjPtrCompat + ?Sized, A: Allocator> Deref for ObjArc<T, A> {
    type Target = T;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.inner().data
    }
}

unsafe impl<#[may_dangle] T: ObjPtrCompat + ?Sized, A: Allocator> Drop for ObjArc<T, A> {
    #[inline]
    fn drop(&mut self) {
        // Because `fetch_sub` is already atomic, we do not need to synchronize
        // with other threads unless we are going to delete the object. This
        // same logic applies to the below `fetch_sub` to the `weak` count.
        if self.inner().strong.fetch_sub(1, Release) != 1 {
            return;
        }

        // This fence is needed to prevent reordering of use of the data and
        // deletion of the data.  Because it is marked `Release`, the decreasing
        // of the reference count synchronizes with this `Acquire` fence. This
        // means that use of the data happens before decreasing the reference
        // count, which happens before this fence, which happens before the
        // deletion of the data.
        //
        // As explained in the [Boost documentation][1],
        //
        // > It is important to enforce any possible access to the object in one
        // > thread (through an existing reference) to *happen before* deleting
        // > the object in a different thread. This is achieved by a "release"
        // > operation after dropping a reference (any access to the object
        // > through this reference must obviously happened before), and an
        // > "acquire" operation before deleting the object.
        //
        // In particular, while the contents of an Arc are usually immutable, it's
        // possible to have interior writes to something like a Mutex<T>. Since a
        // Mutex is not acquired when it is deleted, we can't rely on its
        // synchronization logic to make writes in thread A visible to a destructor
        // running in thread B.
        //
        // Also note that the Acquire fence here could probably be replaced with an
        // Acquire load, which could improve performance in highly-contended
        // situations. See [2].
        //
        // [1]: (www.boost.org/doc/libs/1_55_0/doc/html/atomic/usage_examples.html)
        // [2]: (https://github.com/rust-lang/rust/pull/41714)
        acquire!(self.inner().strong);

        unsafe {
            self.drop_slow();
        }
    }
}

impl<T: Error, A: Allocator> Error for ObjArc<T, A> {
    #[inline]
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Error::source(&**self)
    }

    #[inline]
    #[allow(deprecated, deprecated_in_future)]
    fn description(&self) -> &str {
        Error::description(&**self)
    }

    #[inline]
    #[allow(deprecated)]
    fn cause(&self) -> Option<&dyn Error> {
        Error::cause(&**self)
    }
}

impl<T> From<T> for ObjArc<T> {
    #[inline]
    fn from(t: T) -> Self {
        ObjArc::new(t)
    }
}

impl<T: ObjPtrCompat + Hash + ?Sized, A: Allocator> Hash for ObjArc<T, A> {
    #[inline]
    fn hash<H: Hasher>(&self, state: &mut H) {
        Hash::hash(&**self, state)
    }

    #[inline]
    fn hash_slice<H: Hasher>(data: &[Self], state: &mut H)
    where
        Self: Sized,
    {
        for piece in data {
            piece.hash(state)
        }
    }
}

// Hack to allow specializing on `Eq` even though `Eq` has a method.
trait MarkerEq: PartialEq<Self> {}

impl<T: Eq> MarkerEq for T {}

trait ObjArcEqIdent<T: ObjPtrCompat + ?Sized + PartialEq, A: Allocator> {
    fn eq(&self, other: &ObjArc<T, A>) -> bool;
}

impl<T: ObjPtrCompat + ?Sized + PartialEq, A: Allocator> ObjArcEqIdent<T, A> for ObjArc<T, A> {
    #[inline]
    default fn eq(&self, other: &ObjArc<T, A>) -> bool {
        **self == **other
    }
}

impl<T: ObjPtrCompat + ?Sized + MarkerEq, A: Allocator> ObjArcEqIdent<T, A> for ObjArc<T, A> {
    #[inline]
    fn eq(&self, other: &ObjArc<T, A>) -> bool {
        ObjArc::ptr_eq(self, other) || **self == **other
    }
}

impl<T: ObjPtrCompat + ?Sized + PartialEq, A: Allocator> PartialEq for ObjArc<T, A> {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        ObjArcEqIdent::eq(self, other)
    }
}

impl<T: ObjPtrCompat + PartialOrd<T> + ?Sized, A: Allocator> PartialOrd for ObjArc<T, A> {
    fn partial_cmp(&self, other: &ObjArc<T, A>) -> Option<Ordering> {
        PartialOrd::partial_cmp(&**self, &**other)
    }
}

impl<T: ObjPtrCompat + Eq + ?Sized, A: Allocator> Eq for ObjArc<T, A> {}

impl<T: ObjPtrCompat + Ord + ?Sized, A: Allocator> Ord for ObjArc<T, A> {
    fn cmp(&self, other: &Self) -> Ordering {
        Ord::cmp(&**self, &**other)
    }
}

/// `ObjWeak` is a version of [`ObjArc`] that holds a non-owning reference to
/// the managed allocation, akin to a [`std::sync::Weak`].
#[repr(C)]
pub struct ObjWeak<T: ObjPtrCompat + ?Sized, A: Allocator = Global> {
    // This is a `NonNull` to allow optimizing the size of this type in enums,
    // but it is not necessarily a valid pointer.
    // `Weak::new` sets this to `usize::MAX` so that it doesn’t need
    // to allocate space on the heap.  That's not a value a real pointer
    // will ever have because RcBox has alignment at least 2.
    // This is only possible when `T: Sized`; unsized `T` never dangle.
    ptr: NonNull<ObjArcInner<T>>,
    alloc: A,
}

impl<T> ObjWeak<T> {
    /// Constructs a new `ObjWeak<T, A>`, without allocating any memory.
    ///
    /// # Examples
    ///
    /// ```
    /// use fimo_object::ObjWeak;
    ///
    /// let empty: ObjWeak<i64> = ObjWeak::new();
    /// assert!(empty.upgrade().is_none());
    /// ```
    #[must_use]
    pub fn new() -> ObjWeak<T> {
        ObjWeak::new_in(Global)
    }
}

impl<T, A: Allocator> ObjWeak<T, A> {
    /// Constructs a new `ObjWeak<T, A>`, without allocating any memory, technically in the provided
    /// allocator.
    ///
    /// # Examples
    ///
    /// ```
    /// #![feature(allocator_api)]
    ///
    /// use std::alloc::Global;
    /// use fimo_object::ObjWeak;
    ///
    /// let empty: ObjWeak<i64> = ObjWeak::new_in(Global);
    /// assert!(empty.upgrade().is_none());
    /// ```
    #[must_use]
    pub fn new_in(alloc: A) -> ObjWeak<T, A> {
        ObjWeak {
            ptr: NonNull::new(usize::MAX as *mut ObjArcInner<T>).expect("MAX is not 0"),
            alloc,
        }
    }
}

impl<O: ObjectWrapper + ?Sized, A: Allocator> ObjWeak<O, A> {
    /// Coerces a `ObjWeak<T, A>` to an `ObjWeak<O, A>`.
    pub fn coerce_object<T: CoerceObject<O::VTable>>(w: ObjWeak<T, A>) -> ObjWeak<O, A> {
        let (ptr, alloc) = ObjWeak::into_raw_parts(w);
        let obj = T::coerce_obj_raw(ptr);
        let ptr = O::from_object_raw(obj);
        unsafe { ObjWeak::from_raw_parts(ptr, alloc) }
    }

    /// Tries to revert from an `ObjWeak<O, A>` to an `ObjWeak<T, A>`.
    pub fn try_object_cast<T: CoerceObject<O::VTable>>(
        w: ObjWeak<O, A>,
    ) -> Result<ObjWeak<T, A>, CastError<ObjWeak<O, A>>> {
        let (ptr, alloc) = ObjWeak::into_raw_parts(w);
        let obj = O::as_object_raw(ptr);

        unsafe {
            match Object::<O::VTable>::try_cast_obj_raw::<T>(obj) {
                Ok(casted) => Ok(ObjWeak::from_raw_parts(casted, alloc)),
                Err(err) => Err(CastError {
                    obj: ObjWeak::from_raw_parts(ptr, alloc),
                    required: err.required,
                    required_id: err.required_id,
                    available: err.available,
                    available_id: err.available_id,
                }),
            }
        }
    }

    /// Tries casting the object to another object.
    pub fn try_cast<U: ObjectWrapper + ?Sized>(
        w: ObjWeak<O, A>,
    ) -> Result<ObjWeak<U, A>, CastError<ObjWeak<O, A>>> {
        let (ptr, alloc) = ObjWeak::into_raw_parts(w);
        let obj = O::as_object_raw(ptr);

        unsafe {
            match Object::<O::VTable>::try_cast_raw::<U::VTable>(obj) {
                Ok(casted) => Ok(ObjWeak::from_raw_parts(U::from_object_raw(casted), alloc)),
                Err(err) => Err(CastError {
                    obj: ObjWeak::from_raw_parts(ptr, alloc),
                    required: err.required,
                    required_id: err.required_id,
                    available: err.available,
                    available_id: err.available_id,
                }),
            }
        }
    }

    /// Casts an `ObjWeak<O, A>` to an `ObjWeak<Object<BaseInterface>>`.
    pub fn cast_base(w: ObjWeak<O, A>) -> ObjWeak<Object<IBaseInterface>, A> {
        let (ptr, alloc) = ObjWeak::into_raw_parts(w);
        let obj = O::as_object_raw(ptr);
        let obj = Object::<O::VTable>::cast_base_raw(obj);
        unsafe { ObjWeak::from_raw_parts(obj, alloc) }
    }
}

impl<T: ObjPtrCompat + ?Sized> ObjWeak<T> {
    /// Converts a raw pointer previously created by [`ObjWeak::into_raw`] back into `ObjWeak<T>`
    /// in the provided allocator.
    ///
    /// # Safety
    ///
    /// See [`std::sync::Weak::from_raw`].
    ///
    /// # Examples
    ///
    /// ```
    /// use fimo_object::{ObjArc, ObjWeak};
    ///
    /// let strong = ObjArc::new("hello".to_owned());
    ///
    /// let raw_1 = ObjArc::downgrade(&strong).into_raw();
    /// let raw_2 = ObjArc::downgrade(&strong).into_raw();
    ///
    /// assert_eq!(2, ObjArc::weak_count(&strong));
    ///
    /// assert_eq!("hello", &*unsafe { ObjWeak::from_raw(raw_1) }.upgrade().unwrap());
    /// assert_eq!(1, ObjArc::weak_count(&strong));
    ///
    /// drop(strong);
    ///
    /// // Decrement the last weak count.
    /// assert!(unsafe { ObjWeak::from_raw(raw_2) }.upgrade().is_none())
    /// ```
    pub unsafe fn from_raw(ptr: *const T) -> Self {
        Self::from_raw_parts(ptr, Global)
    }
}

impl<T: ObjPtrCompat + ?Sized, A: Allocator> ObjWeak<T, A> {
    /// Returns a raw pointer to the object `T` pointed to by this `ObjWeak<T>`.
    ///
    /// # Examples
    ///
    /// ```
    /// use fimo_object::ObjArc;
    /// use std::ptr;
    ///
    /// let strong = ObjArc::new("hello".to_owned());
    /// let weak = ObjArc::downgrade(&strong);
    /// // Both point to the same object
    /// assert!(ptr::eq(&*strong, weak.as_ptr()));
    /// // The strong here keeps it alive, so we can still access the object.
    /// assert_eq!("hello", unsafe { &*weak.as_ptr() });
    ///
    /// drop(strong);
    /// // But not any more. We can do weak.as_ptr(), but accessing the pointer would lead to
    /// // undefined behaviour.
    /// // assert_eq!("hello", unsafe { &*weak.as_ptr() });
    /// ```
    #[must_use]
    pub fn as_ptr(&self) -> *const T {
        let ptr: *mut ObjArcInner<T> = NonNull::as_ptr(self.ptr);

        if is_dangling(ptr) {
            // If the pointer is dangling, we return the sentinel directly. This cannot be
            // a valid payload address, as the payload is at least as aligned as ObjArcInner (usize).
            ptr as *const T
        } else {
            // SAFETY: if is_dangling returns false, then the pointer is dereferencable.
            // The payload may be dropped at this point, and we have to maintain provenance,
            // so use raw pointer manipulation.
            unsafe { std::ptr::addr_of_mut!((*ptr).data) }
        }
    }

    /// Consumes the `ObjWeak<T>` and turns it into a raw pointer.
    ///
    /// # Examples
    ///
    /// ```
    /// use fimo_object::{ObjArc, ObjWeak};
    ///
    /// let strong = ObjArc::new("hello".to_owned());
    /// let weak = ObjArc::downgrade(&strong);
    /// let raw = weak.into_raw();
    ///
    /// assert_eq!(1, ObjArc::weak_count(&strong));
    /// assert_eq!("hello", unsafe { &*raw });
    ///
    /// drop(unsafe { ObjWeak::from_raw(raw) });
    /// assert_eq!(0, ObjArc::weak_count(&strong));
    /// ```
    pub fn into_raw(self) -> *const T {
        let result = self.as_ptr();
        std::mem::forget(self);
        result
    }

    /// Consumes the `ObjWeak<T>` and turns it into a raw pointer.
    ///
    /// # Examples
    ///
    /// ```
    /// #![feature(allocator_api)]
    ///
    /// use std::alloc::Global;
    /// use fimo_object::{ObjArc, ObjWeak};
    ///
    /// let strong = ObjArc::new_in("hello".to_owned(), Global);
    /// let weak = ObjArc::downgrade(&strong);
    /// let (raw, alloc) = weak.into_raw_parts();
    ///
    /// assert_eq!(1, ObjArc::weak_count(&strong));
    /// assert_eq!("hello", unsafe { &*raw });
    ///
    /// drop(unsafe { ObjWeak::from_raw_parts(raw, alloc) });
    /// assert_eq!(0, ObjArc::weak_count(&strong));
    /// ```
    pub fn into_raw_parts(self) -> (*const T, A) {
        let ptr = self.as_ptr();
        let alloc = unsafe { std::ptr::read(&self.alloc) };
        std::mem::forget(self);
        (ptr, alloc)
    }

    /// Converts a raw pointer previously created by [`ObjWeak::into_raw_parts`] back into
    /// `ObjWeak<T>` in the provided allocator.
    ///
    /// # Safety
    ///
    /// See [`std::sync::Weak::from_raw`].
    ///
    /// # Examples
    ///
    /// ```
    /// #![feature(allocator_api)]
    ///
    /// use std::alloc::Global;
    /// use fimo_object::{ObjArc, ObjWeak};
    ///
    /// let strong = ObjArc::new_in("hello".to_owned(), Global);
    ///
    /// let (raw_1, alloc_1) = ObjArc::downgrade(&strong).into_raw_parts();
    /// let (raw_2, alloc_2) = ObjArc::downgrade(&strong).into_raw_parts();
    ///
    /// assert_eq!(2, ObjArc::weak_count(&strong));
    ///
    /// assert_eq!("hello", &*unsafe { ObjWeak::from_raw_parts(raw_1, alloc_1) }.upgrade().unwrap());
    /// assert_eq!(1, ObjArc::weak_count(&strong));
    ///
    /// drop(strong);
    ///
    /// // Decrement the last weak count.
    /// assert!(unsafe { ObjWeak::from_raw_parts(raw_2, alloc_2) }.upgrade().is_none())
    /// ```
    pub unsafe fn from_raw_parts(ptr: *const T, alloc: A) -> Self {
        // See ObjWeak::as_ptr for context on how the input pointer is derived.

        let ptr = if is_dangling(ptr as *mut T) {
            // This is a dangling Weak.
            ptr as *mut ObjArcInner<T>
        } else {
            // Otherwise, we're guaranteed the pointer came from a nondangling Weak.
            // SAFETY: data_offset is safe to call, as ptr references a real (potentially dropped) T.
            let offset = data_offset(ptr);
            // Thus, we reverse the offset to get the whole RcBox.
            // SAFETY: the pointer originated from a Weak, so this offset is safe.
            (ptr as *mut ObjArcInner<T>).set_ptr_value((ptr as *mut u8).offset(-offset))
        };

        // SAFETY: we now have recovered the original Weak pointer, so can create the Weak.
        ObjWeak {
            ptr: NonNull::new_unchecked(ptr),
            alloc,
        }
    }
}

impl<T: ObjPtrCompat + ?Sized, A: Allocator> ObjWeak<T, A> {
    /// Attempts to upgrade the `ObjWeak` pointer to an [`ObjArc`], delaying
    /// dropping of the inner value if successful.
    ///
    /// # Examples
    ///
    /// ```
    /// use fimo_object::ObjArc;
    ///
    /// let five = ObjArc::new(5);
    ///
    /// let weak_five = ObjArc::downgrade(&five);
    ///
    /// let strong_five: Option<ObjArc<_>> = weak_five.upgrade();
    /// assert!(strong_five.is_some());
    ///
    /// // Destroy all strong pointers.
    /// drop(strong_five);
    /// drop(five);
    ///
    /// assert!(weak_five.upgrade().is_none());
    /// ```
    pub fn upgrade(&self) -> Option<ObjArc<T, A>>
    where
        A: Clone,
    {
        // We use a CAS loop to increment the strong count instead of a
        // fetch_add as this function should never take the reference count
        // from zero to one.
        let inner = self.inner()?;

        // Relaxed load because any write of 0 that we can observe
        // leaves the field in a permanently zero state (so a
        // "stale" read of 0 is fine), and any other value is
        // confirmed via the CAS below.
        let mut n = inner.strong.load(Relaxed);

        loop {
            if n == 0 {
                return None;
            }

            // See comments in `Arc::clone` for why we do this (for `mem::forget`).
            if n > MAX_REFCOUNT {
                abort();
            }

            // Relaxed is fine for the failure case because we don't have any expectations about the new state.
            // Acquire is necessary for the success case to synchronise with `Arc::new_cyclic`, when the inner
            // value can be initialized after `Weak` references have already been created. In that case, we
            // expect to observe the fully initialized value.
            match inner
                .strong
                .compare_exchange_weak(n, n + 1, Acquire, Relaxed)
            {
                Ok(_) => {
                    return Some(ObjArc {
                        ptr: self.ptr,
                        phantom: Default::default(),
                        alloc: self.alloc.clone(),
                    })
                } // null checked above
                Err(old) => n = old,
            }
        }
    }

    /// Gets the number of strong (`ObjArc`) pointers pointing to this allocation.
    ///
    /// If `self` was created using [`ObjWeak::new`], this will return 0.
    #[must_use]
    pub fn strong_count(&self) -> usize {
        if let Some(inner) = self.inner() {
            inner.strong.load(SeqCst)
        } else {
            0
        }
    }

    /// Gets an approximation of the number of `ObjWeak` pointers pointing to this
    /// allocation.
    #[must_use]
    pub fn weak_count(&self) -> usize {
        self.inner()
            .map(|inner| {
                let weak = inner.weak.load(SeqCst);
                let strong = inner.strong.load(SeqCst);
                if strong == 0 {
                    0
                } else {
                    // Since we observed that there was at least one strong pointer
                    // after reading the weak count, we know that the implicit weak
                    // reference (present whenever any strong references are alive)
                    // was still around when we observed the weak count, and can
                    // therefore safely subtract it.
                    weak - 1
                }
            })
            .unwrap_or(0)
    }

    /// Returns `None` when the pointer is dangling and there is no allocated `ObjArcInner`,
    /// (i.e., when this `Weak` was created by `ObjWeak::new`).
    #[inline]
    fn inner(&self) -> Option<WeakInner<'_>> {
        if is_dangling(self.ptr.as_ptr()) {
            None
        } else {
            // We are careful to *not* create a reference covering the "data" field, as
            // the field may be mutated concurrently (for example, if the last `Arc`
            // is dropped, the data field will be dropped in-place).
            Some(unsafe {
                let ptr = self.ptr.as_ptr();
                WeakInner {
                    strong: &(*ptr).strong,
                    weak: &(*ptr).weak,
                }
            })
        }
    }

    /// Returns `true` if the two `ObjWeak`s point to the same allocation (similar to
    /// [`std::ptr::eq`]), or if both don't point to any allocation
    /// (because they were created with `ObjWeak::new()`).
    ///
    /// # Examples
    ///
    /// ```
    /// use fimo_object::ObjArc;
    ///
    /// let first_arc = ObjArc::new(5);
    /// let first = ObjArc::downgrade(&first_arc);
    /// let second = ObjArc::downgrade(&first_arc);
    ///
    /// assert!(first.ptr_eq(&second));
    ///
    /// let third_arc = ObjArc::new(5);
    /// let third = ObjArc::downgrade(&third_arc);
    ///
    /// assert!(!first.ptr_eq(&third));
    /// ```
    ///
    /// Comparing `ObjWeak::new`.
    ///
    /// ```
    /// use fimo_object::{ObjArc, ObjWeak};
    ///
    /// let first = ObjWeak::new();
    /// let second = ObjWeak::new();
    /// assert!(first.ptr_eq(&second));
    ///
    /// let third_arc = ObjArc::new(());
    /// let third = ObjArc::downgrade(&third_arc);
    /// assert!(!first.ptr_eq(&third));
    /// ```
    #[inline]
    #[must_use]
    pub fn ptr_eq(&self, other: &Self) -> bool {
        self.ptr.as_ptr() == other.ptr.as_ptr()
    }
}

unsafe impl<T: ObjPtrCompat + ?Sized + Sync + Send, A: Allocator + Send> Send for ObjWeak<T, A> {}

unsafe impl<T: ObjPtrCompat + ?Sized + Sync + Send, A: Allocator + Sync> Sync for ObjWeak<T, A> {}

impl<T: ObjPtrCompat + ?Sized, A: Allocator + Clone> Clone for ObjWeak<T, A> {
    fn clone(&self) -> Self {
        let inner = if let Some(inner) = self.inner() {
            inner
        } else {
            return ObjWeak {
                ptr: self.ptr,
                alloc: self.alloc.clone(),
            };
        };
        // See comments in Arc::clone() for why this is relaxed.  This can use a
        // fetch_add (ignoring the lock) because the weak count is only locked
        // where are *no other* weak pointers in existence. (So we can't be
        // running this code in that case).
        let old_size = inner.weak.fetch_add(1, Relaxed);

        // See comments in Arc::clone() for why we do this (for mem::forget).
        if old_size > MAX_REFCOUNT {
            abort();
        }

        ObjWeak {
            ptr: self.ptr,
            alloc: self.alloc.clone(),
        }
    }
}

impl<T: ObjPtrCompat + ?Sized + Debug, A: Allocator> Debug for ObjWeak<T, A> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "(ObjWeak)")
    }
}

impl<T, A: Allocator + Default> Default for ObjWeak<T, A> {
    fn default() -> Self {
        ObjWeak::new_in(Default::default())
    }
}

unsafe impl<#[may_dangle] T: ObjPtrCompat + ?Sized, A: Allocator> Drop for ObjWeak<T, A> {
    fn drop(&mut self) {
        // If we find out that we were the last weak pointer, then its time to
        // deallocate the data entirely. See the discussion in Arc::drop() about
        // the memory orderings
        //
        // It's not necessary to check for the locked state here, because the
        // weak count can only be locked if there was precisely one weak ref,
        // meaning that drop could only subsequently run ON that remaining weak
        // ref, which can only happen after the lock is released.
        let inner = if let Some(inner) = self.inner() {
            inner
        } else {
            return;
        };

        if inner.weak.fetch_sub(1, Release) == 1 {
            acquire!(inner.weak);
            unsafe {
                self.alloc
                    .deallocate(self.ptr.cast(), (&*self.ptr.as_ptr()).get_layout())
            }
        }
    }
}

#[repr(C)]
#[allow(missing_debug_implementations)]
struct ObjArcInner<T: ObjPtrCompat + ?Sized> {
    strong: atomic::AtomicUsize,

    // the value usize::MAX acts as a sentinel for temporarily "locking" the
    // ability to upgrade weak pointers or downgrade strong ones; this is used
    // to avoid races in `make_mut` and `get_mut`.
    weak: atomic::AtomicUsize,
    data: T,
}

impl<T: ObjPtrCompat + ?Sized> ObjArcInner<T> {
    fn get_layout(&self) -> Layout {
        let ptr: *const T = std::ptr::addr_of!(self.data);

        let layout = Layout::new::<ObjArcInner<()>>();
        let data_layout = unsafe { crate::obj_box::ConstructLayoutRaw::layout_for_raw(ptr) };

        layout
            .extend(data_layout)
            .expect("Layout extended")
            .0
            .pad_to_align()
    }
}

unsafe impl<T: ObjPtrCompat + ?Sized + Sync + Send> Send for ObjArcInner<T> {}

unsafe impl<T: ObjPtrCompat + ?Sized + Sync + Send> Sync for ObjArcInner<T> {}

/// Helper type to allow accessing the reference counts without
/// making any assertions about the data field.
#[allow(missing_debug_implementations)]
struct WeakInner<'a> {
    weak: &'a atomic::AtomicUsize,
    strong: &'a atomic::AtomicUsize,
}

/// Get the offset within an `ObjArcInner` for the payload behind a pointer.
///
/// # Safety
///
/// The pointer must point to (and have valid metadata for) a previously
/// valid instance of T, but the T is allowed to be dropped.
unsafe fn data_offset<T: ObjPtrCompat + ?Sized + crate::obj_box::ConstructLayoutRaw>(
    ptr: *const T,
) -> isize {
    // Align the unsized value to the end of the ObjArcInner.
    // Because ObjArcInner is repr(C), it will always be the last field in memory.
    data_offset_align(T::align_of_val_raw(ptr))
}

#[inline]
fn data_offset_align(align: usize) -> isize {
    let layout = Layout::new::<ObjArcInner<()>>();
    (layout.size() + layout.padding_needed_for(align)) as isize
}

fn is_dangling<T: ?Sized>(ptr: *mut T) -> bool {
    let address = ptr as *mut () as usize;
    address == usize::MAX
}
