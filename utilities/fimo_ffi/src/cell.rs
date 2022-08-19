//! Implementation of the `RefCell` type.
// This is a modified version of the RefCell type found in the std library,
// which is dual-licensed under Apache 2.0 and MIT terms.

use std::{
    cell::{Cell, UnsafeCell},
    cmp::Ordering,
    marker::Unsize,
    ops::{Deref, DerefMut},
    process::abort,
    sync::atomic::AtomicUsize,
};

use crate::{
    ptr::{CastInto, DowncastSafeInterface, FetchVTable, ObjInterface},
    DynObj, ObjectId,
};

/// A mutable memory location with dynamically checked borrow rules
#[repr(C)]
pub struct RefCell<T: ?Sized> {
    borrow: Cell<BorrowFlag>,
    value: UnsafeCell<T>,
}

/// A mutable memory location with dynamically and atomically checked borrow rules.
#[repr(C)]
pub struct AtomicRefCell<T: ?Sized> {
    borrow: AtomicBorrowFlag,
    value: UnsafeCell<T>,
}

// Positive values represent the number of `Ref` active. Negative values
// represent the number of `RefMut` active. Multiple `RefMut`s can only be
// active at a time if they refer to distinct, nonoverlapping components of a
// `RefCell` (e.g., different ranges of a slice).
//
// `Ref` and `RefMut` are both two words in size, and so there will likely never
// be enough `Ref`s or `RefMut`s in existence to overflow half of the `usize`
// range. Thus, a `BorrowFlag` will probably never overflow or underflow.
// However, this is not a guarantee, as a pathological program could repeatedly
// create and then mem::forget `Ref`s or `RefMut`s. Thus, all code must
// explicitly check for overflow and underflow in order to avoid unsafety, or at
// least behave correctly in the event that overflow or underflow happens (e.g.,
// see BorrowRef::new).
type BorrowFlag = isize;
const UNUSED: BorrowFlag = 0;

/// Use atomic usizes for the count of active readers and writers. A count of
/// less than `WRITING_ATOMIC` indicates that the value inside the cell is being
/// shared. A value of at least `WRITING_ATOMIC` (i.e. the last bit is set)
/// indicates an exclusive borrow. Unlike with an `RefCell` the count is not
/// allowed to overflow and instead aborts when too many threads try to acquire
/// a shared borrow, i. e. the count reaches `MAX_ATOMIC_COUNT + 1`. On a system
/// with 64bit usize it would allow us about 2^61 concurrent threads.
type AtomicBorrowFlag = AtomicUsize;
const UNUSED_ATOMIC: usize = 0;
const WRITING_ATOMIC: usize = !(usize::MAX >> 1);
const MAX_ATOMIC_COUNT: usize = WRITING_ATOMIC + (WRITING_ATOMIC >> 1);

#[inline(always)]
fn is_writing(x: BorrowFlag) -> bool {
    x < UNUSED
}

#[inline(always)]
fn is_reading(x: BorrowFlag) -> bool {
    x > UNUSED
}

#[inline(always)]
fn atomic_is_writing(x: usize) -> bool {
    x >= WRITING_ATOMIC
}

#[inline(always)]
fn atomic_is_reading(x: usize) -> bool {
    !atomic_is_writing(x)
}

/// An error returned by [`RefCell::try_borrow`].
#[repr(C)]
pub struct BorrowError {
    _v: u8,
}

impl std::fmt::Debug for BorrowError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BorrowError").finish()
    }
}

impl std::fmt::Display for BorrowError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt("already mutably borrowed", f)
    }
}

impl std::error::Error for BorrowError {}

impl crate::fmt::IDebug for BorrowError {
    fn fmt(&self, f: &mut crate::fmt::Formatter<'_>) -> Result<(), crate::fmt::Error> {
        write!(f, "{self:?}")
    }
}

impl crate::fmt::IDisplay for BorrowError {
    fn fmt(&self, f: &mut crate::fmt::Formatter<'_>) -> Result<(), crate::fmt::Error> {
        write!(f, "{self}")
    }
}

impl crate::error::IError for BorrowError {}

/// An error returned by [`RefCell::try_borrow_mut`].
#[repr(C)]
pub struct BorrowMutError {
    _v: u8,
}

impl std::fmt::Debug for BorrowMutError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BorrowMutError").finish()
    }
}

impl std::fmt::Display for BorrowMutError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt("already borrowed", f)
    }
}

impl std::error::Error for BorrowMutError {}

impl crate::fmt::IDebug for BorrowMutError {
    fn fmt(&self, f: &mut crate::fmt::Formatter<'_>) -> Result<(), crate::fmt::Error> {
        write!(f, "{self:?}")
    }
}

impl crate::fmt::IDisplay for BorrowMutError {
    fn fmt(&self, f: &mut crate::fmt::Formatter<'_>) -> Result<(), crate::fmt::Error> {
        write!(f, "{self}")
    }
}

impl crate::error::IError for BorrowMutError {}

impl<T> RefCell<T> {
    /// Creates a new `RefCell` containing `value`.
    ///
    /// # Examples
    ///
    /// ```
    /// use fimo_ffi::cell::RefCell;
    ///
    /// let c = RefCell::new(5);
    /// ```
    #[inline]
    pub const fn new(value: T) -> RefCell<T> {
        RefCell {
            borrow: Cell::new(UNUSED),
            value: UnsafeCell::new(value),
        }
    }

    /// Consumes the `RefCell`, returning the wrapped value.
    ///
    /// # Examples
    ///
    /// ```
    /// use fimo_ffi::cell::RefCell;
    ///
    /// let c = RefCell::new(5);
    ///
    /// let five = c.into_inner();
    /// ```
    #[inline]
    pub fn into_inner(self) -> T {
        self.value.into_inner()
    }

    /// Replaces the wrapped value with a new one, returning the old value,
    /// without deinitializing either one.
    ///
    /// This function corresponds to [`std::mem::replace`](../mem/fn.replace.html).
    ///
    /// # Panics
    ///
    /// Panics if the value is currently borrowed.
    ///
    /// # Examples
    ///
    /// ```
    /// use fimo_ffi::cell::RefCell;
    /// let cell = RefCell::new(5);
    /// let old_value = cell.replace(6);
    /// assert_eq!(old_value, 5);
    /// assert_eq!(cell, RefCell::new(6));
    /// ```
    #[inline]
    pub fn replace(&self, t: T) -> T {
        std::mem::replace(&mut *self.borrow_mut(), t)
    }

    /// Replaces the wrapped value with a new one computed from `f`, returning
    /// the old value, without deinitializing either one.
    ///
    /// # Panics
    ///
    /// Panics if the value is currently borrowed.
    ///
    /// # Examples
    ///
    /// ```
    /// use fimo_ffi::cell::RefCell;
    /// let cell = RefCell::new(5);
    /// let old_value = cell.replace_with(|&mut old| old + 1);
    /// assert_eq!(old_value, 5);
    /// assert_eq!(cell, RefCell::new(6));
    /// ```
    #[inline]
    pub fn replace_with<F: FnOnce(&mut T) -> T>(&self, f: F) -> T {
        let mut_borrow = &mut *self.borrow_mut();
        let replacement = f(mut_borrow);
        std::mem::replace(mut_borrow, replacement)
    }

    /// Swaps the wrapped value of `self` with the wrapped value of `other`,
    /// without deinitializing either one.
    ///
    /// This function corresponds to [`std::mem::swap`](../mem/fn.swap.html).
    ///
    /// # Panics
    ///
    /// Panics if the value in either `RefCell` is currently borrowed.
    ///
    /// # Examples
    ///
    /// ```
    /// use fimo_ffi::cell::RefCell;
    /// let c = RefCell::new(5);
    /// let d = RefCell::new(6);
    /// c.swap(&d);
    /// assert_eq!(c, RefCell::new(6));
    /// assert_eq!(d, RefCell::new(5));
    /// ```
    #[inline]
    pub fn swap(&self, other: &Self) {
        std::mem::swap(&mut *self.borrow_mut(), &mut *other.borrow_mut())
    }
}

impl<T: ?Sized> RefCell<T> {
    /// Immutably borrows the wrapped value.
    ///
    /// The borrow lasts until the returned `Ref` exits scope. Multiple
    /// immutable borrows can be taken out at the same time.
    ///
    /// # Panics
    ///
    /// Panics if the value is currently mutably borrowed. For a non-panicking variant, use
    /// [`try_borrow`](#method.try_borrow).
    ///
    /// # Examples
    ///
    /// ```
    /// use fimo_ffi::cell::RefCell;
    ///
    /// let c = RefCell::new(5);
    ///
    /// let borrowed_five = c.borrow();
    /// let borrowed_five2 = c.borrow();
    /// ```
    ///
    /// An example of panic:
    ///
    /// ```should_panic
    /// use fimo_ffi::cell::RefCell;
    ///
    /// let c = RefCell::new(5);
    ///
    /// let m = c.borrow_mut();
    /// let b = c.borrow(); // this causes a panic
    /// ```
    #[inline]
    pub fn borrow(&self) -> Ref<'_, T> {
        self.try_borrow().expect("already mutably borrowed")
    }

    /// Immutably borrows the wrapped value, returning an error if the value is currently mutably
    /// borrowed.
    ///
    /// The borrow lasts until the returned `Ref` exits scope. Multiple immutable borrows can be
    /// taken out at the same time.
    ///
    /// This is the non-panicking variant of [`borrow`](#method.borrow).
    ///
    /// # Examples
    ///
    /// ```
    /// use fimo_ffi::cell::RefCell;
    ///
    /// let c = RefCell::new(5);
    ///
    /// {
    ///     let m = c.borrow_mut();
    ///     assert!(c.try_borrow().is_err());
    /// }
    ///
    /// {
    ///     let m = c.borrow();
    ///     assert!(c.try_borrow().is_ok());
    /// }
    /// ```
    #[inline]
    pub fn try_borrow(&self) -> Result<Ref<'_, T>, BorrowError> {
        match BorrowRef::new(&self.borrow) {
            Some(b) => {
                // SAFETY: `BorrowRef` ensures that there is only immutable access
                // to the value while borrowed.
                Ok(Ref {
                    value: unsafe { &*self.value.get() },
                    borrow: b,
                })
            }
            None => Err(BorrowError { _v: 0 }),
        }
    }

    /// Mutably borrows the wrapped value.
    ///
    /// The borrow lasts until the returned `RefMut` or all `RefMut`s derived
    /// from it exit scope. The value cannot be borrowed while this borrow is
    /// active.
    ///
    /// # Panics
    ///
    /// Panics if the value is currently borrowed. For a non-panicking variant, use
    /// [`try_borrow_mut`](#method.try_borrow_mut).
    ///
    /// # Examples
    ///
    /// ```
    /// use fimo_ffi::cell::RefCell;
    ///
    /// let c = RefCell::new("hello".to_owned());
    ///
    /// *c.borrow_mut() = "bonjour".to_owned();
    ///
    /// assert_eq!(&*c.borrow(), "bonjour");
    /// ```
    ///
    /// An example of panic:
    ///
    /// ```should_panic
    /// use fimo_ffi::cell::RefCell;
    ///
    /// let c = RefCell::new(5);
    /// let m = c.borrow();
    ///
    /// let b = c.borrow_mut(); // this causes a panic
    /// ```
    #[inline]
    pub fn borrow_mut(&self) -> RefMut<'_, T> {
        self.try_borrow_mut().expect("already borrowed")
    }

    /// Mutably borrows the wrapped value, returning an error if the value is currently borrowed.
    ///
    /// The borrow lasts until the returned `RefMut` or all `RefMut`s derived
    /// from it exit scope. The value cannot be borrowed while this borrow is
    /// active.
    ///
    /// This is the non-panicking variant of [`borrow_mut`](#method.borrow_mut).
    ///
    /// # Examples
    ///
    /// ```
    /// use fimo_ffi::cell::RefCell;
    ///
    /// let c = RefCell::new(5);
    ///
    /// {
    ///     let m = c.borrow();
    ///     assert!(c.try_borrow_mut().is_err());
    /// }
    ///
    /// assert!(c.try_borrow_mut().is_ok());
    /// ```
    #[inline]
    pub fn try_borrow_mut(&self) -> Result<RefMut<'_, T>, BorrowMutError> {
        match BorrowRefMut::new(&self.borrow) {
            Some(b) => {
                // SAFETY: `BorrowRef` guarantees unique access.
                Ok(RefMut {
                    value: unsafe { &mut *self.value.get() },
                    borrow: b,
                })
            }
            None => Err(BorrowMutError { _v: 0 }),
        }
    }

    /// Returns a raw pointer to the underlying data in this cell.
    ///
    /// # Examples
    ///
    /// ```
    /// use fimo_ffi::cell::RefCell;
    ///
    /// let c = RefCell::new(5);
    ///
    /// let ptr = c.as_ptr();
    /// ```
    #[inline]
    pub fn as_ptr(&self) -> *mut T {
        self.value.get()
    }

    /// Returns a mutable reference to the underlying data.
    ///
    /// This call borrows `RefCell` mutably (at compile-time) so there is no
    /// need for dynamic checks.
    ///
    /// However be cautious: this method expects `self` to be mutable, which is
    /// generally not the case when using a `RefCell`. Take a look at the
    /// [`borrow_mut`] method instead if `self` isn't mutable.
    ///
    /// Also, please be aware that this method is only for special circumstances and is usually
    /// not what you want. In case of doubt, use [`borrow_mut`] instead.
    ///
    /// [`borrow_mut`]: RefCell::borrow_mut()
    ///
    /// # Examples
    ///
    /// ```
    /// use fimo_ffi::cell::RefCell;
    ///
    /// let mut c = RefCell::new(5);
    /// *c.get_mut() += 1;
    ///
    /// assert_eq!(c, RefCell::new(6));
    /// ```
    #[inline]
    pub fn get_mut(&mut self) -> &mut T {
        self.value.get_mut()
    }

    /// Undo the effect of leaked guards on the borrow state of the `RefCell`.
    ///
    /// This call is similar to [`get_mut`] but more specialized. It borrows `RefCell` mutably to
    /// ensure no borrows exist and then resets the state tracking shared borrows. This is relevant
    /// if some `Ref` or `RefMut` borrows have been leaked.
    ///
    /// [`get_mut`]: RefCell::get_mut()
    ///
    /// # Examples
    ///
    /// ```
    /// use fimo_ffi::cell::RefCell;
    ///
    /// let mut c = RefCell::new(0);
    /// std::mem::forget(c.borrow_mut());
    ///
    /// assert!(c.try_borrow().is_err());
    /// c.undo_leak();
    /// assert!(c.try_borrow().is_ok());
    /// ```
    pub fn undo_leak(&mut self) -> &mut T {
        *self.borrow.get_mut() = UNUSED;
        self.get_mut()
    }

    /// Immutably borrows the wrapped value, returning an error if the value is
    /// currently mutably borrowed.
    ///
    /// # Safety
    ///
    /// Unlike `RefCell::borrow`, this method is unsafe because it does not
    /// return a `Ref`, thus leaving the borrow flag untouched. Mutably
    /// borrowing the `RefCell` while the reference returned by this method
    /// is alive is undefined behaviour.
    ///
    /// # Examples
    ///
    /// ```
    /// use fimo_ffi::cell::RefCell;
    ///
    /// let c = RefCell::new(5);
    ///
    /// {
    ///     let m = c.borrow_mut();
    ///     assert!(unsafe { c.try_borrow_unguarded() }.is_err());
    /// }
    ///
    /// {
    ///     let m = c.borrow();
    ///     assert!(unsafe { c.try_borrow_unguarded() }.is_ok());
    /// }
    /// ```
    #[inline]
    pub unsafe fn try_borrow_unguarded(&self) -> Result<&T, BorrowError> {
        if !is_writing(self.borrow.get()) {
            // SAFETY: We check that nobody is actively writing now, but it is
            // the caller's responsibility to ensure that nobody writes until
            // the returned reference is no longer in use.
            // Also, `self.value.get()` refers to the value owned by `self`
            // and is thus guaranteed to be valid for the lifetime of `self`.
            Ok(&*self.value.get())
        } else {
            Err(BorrowError { _v: 0 })
        }
    }
}

impl<T: ?Sized> RefCell<T> {
    /// Coerces a `RefCell<T>` to an `RefCell<DynObj<U>>`.
    ///
    /// # Examples
    ///
    /// ```
    /// #![feature(const_trait_impl)]
    /// #![feature(unsize)]
    ///
    /// use fimo_ffi::{DynObj, ObjectId, interface};
    /// use fimo_ffi::ptr::{CastInto, IBase};
    /// use fimo_ffi::cell::RefCell;
    ///
    /// // Define a custom interface.
    /// interface! {
    ///     #![interface_cfg(uuid = "59dc47cf-fd2e-4d58-bcd4-5a31adc68a44")]
    ///     interface Obj: marker IBase {
    ///         fn add(&self, num: usize) -> usize;
    ///     }
    /// }
    ///
    /// // Define a custom object implementing the interface.
    /// #[derive(ObjectId)]
    /// #[fetch_vtable(uuid = "7ecb22c2-9426-46da-a7cb-8ad99eef582f", interfaces(Obj))]
    /// struct MyObj(usize);
    ///
    /// impl Obj for MyObj {
    ///     fn add(&self, num: usize) -> usize {
    ///         self.0 + num
    ///     }
    /// }
    ///
    /// let x = RefCell::new(MyObj(5));
    /// let x: &RefCell<DynObj<dyn Obj>> = x.coerce_obj();
    ///
    /// {
    ///     let x = x.borrow();
    ///     assert_eq!(x.add(0), 5);
    ///     assert_eq!(x.add(1), 6);
    ///     assert_eq!(x.add(5), 10);
    /// }
    /// ```
    #[inline]
    pub fn coerce_obj<U>(&self) -> &RefCell<DynObj<U>>
    where
        T: FetchVTable<U::Base> + Unsize<U>,
        U: ObjInterface + ?Sized,
    {
        let vtable = T::fetch_interface();
        let metadata = crate::ptr::ObjMetadata::<U>::new(vtable);
        let obj = crate::ptr::from_raw_parts(std::ptr::null(), metadata);

        let metadata = std::ptr::metadata(obj);
        let ref_obj = std::ptr::from_raw_parts(self as *const _ as _, metadata);

        // SAFETY: both the data pointer and metadata are valid
        unsafe { &*ref_obj }
    }

    /// Coerces a `RefCell<T>` to an `RefCell<DynObj<U>>`.
    ///
    /// # Examples
    ///
    /// ```
    /// #![feature(const_trait_impl)]
    /// #![feature(unsize)]
    ///
    /// use fimo_ffi::{DynObj, ObjectId, interface};
    /// use fimo_ffi::ptr::{CastInto, IBase};
    /// use fimo_ffi::cell::RefCell;
    ///
    /// // Define a custom interface.
    /// interface! {
    ///     #![interface_cfg(uuid = "59dc47cf-fd2e-4d58-bcd4-5a31adc68a44")]
    ///     interface Obj: marker IBase {
    ///         fn set(&mut self, num: usize);
    ///     }
    /// }
    ///
    /// // Define a custom object implementing the interface.
    /// #[derive(ObjectId)]
    /// #[fetch_vtable(uuid = "7ecb22c2-9426-46da-a7cb-8ad99eef582f", interfaces(Obj))]
    /// struct MyObj(usize);
    ///
    /// impl Obj for MyObj {
    ///     fn set(&mut self, num: usize) {
    ///         self.0 = num
    ///     }
    /// }
    ///
    /// let mut x = RefCell::new(MyObj(5));
    ///
    /// {
    ///     let x: &mut RefCell<DynObj<dyn Obj>> = x.coerce_obj_mut();
    ///     let mut x = x.borrow_mut();
    ///     x.set(5);
    /// }
    ///
    /// assert_eq!(x.get_mut().0, 5)
    /// ```
    #[inline]
    pub fn coerce_obj_mut<U>(&mut self) -> &mut RefCell<DynObj<U>>
    where
        T: FetchVTable<U::Base> + Unsize<U>,
        U: ObjInterface + ?Sized,
    {
        let vtable = T::fetch_interface();
        let metadata = crate::ptr::ObjMetadata::<U>::new(vtable);
        let obj = crate::ptr::from_raw_parts(std::ptr::null(), metadata);

        let metadata = std::ptr::metadata(obj);
        let ref_obj = std::ptr::from_raw_parts_mut(self as *mut _ as _, metadata);

        // SAFETY: both the data pointer and metadata are valid
        unsafe { &mut *ref_obj }
    }
}

impl<'a, T: ?Sized + 'a> RefCell<DynObj<T>> {
    /// Returns whether the contained object is of type `U`.
    ///
    /// # Examples
    ///
    /// ```
    /// #![feature(unsize)]
    ///
    /// use fimo_ffi::cell::RefCell;
    /// use fimo_ffi::{DynObj, ObjectId};
    /// use fimo_ffi::ptr::{ObjInterface, IBase};
    ///
    /// // Define a custom object implementing the interface.
    /// #[derive(ObjectId)]
    /// #[fetch_vtable(uuid = "7ecb22c2-9426-46da-a7cb-8ad99eef582f")]
    /// struct SomeObj;
    ///
    /// #[derive(ObjectId)]
    /// #[fetch_vtable(uuid = "b745c60c-e258-4edc-86a9-9fb6b1191ce9")]
    /// struct OtherObj;
    ///
    /// let x = RefCell::new(SomeObj);
    /// let x: &RefCell<DynObj<dyn IBase>> = x.coerce_obj();
    /// assert_eq!(x.is::<SomeObj>(), true);
    /// assert_eq!(x.is::<OtherObj>(), false);
    /// ```
    #[inline]
    pub fn is<U>(&self) -> bool
    where
        U: ObjectId + Unsize<T> + 'static,
    {
        let obj = self.as_ptr();
        fimo_ffi::ptr::is::<U, _>(obj)
    }

    /// Returns the downcasted `RefCell` if it is of type `U`.
    ///
    /// # Examples
    ///
    /// ```
    /// #![feature(unsize)]
    ///
    /// use fimo_ffi::cell::RefCell;
    /// use fimo_ffi::{DynObj, ObjectId};
    /// use fimo_ffi::ptr::{ObjInterface, IBase};
    ///
    /// // Define a custom object implementing the interface.
    /// #[derive(ObjectId)]
    /// #[fetch_vtable(uuid = "7ecb22c2-9426-46da-a7cb-8ad99eef582f")]
    /// struct SomeObj;
    ///
    /// #[derive(ObjectId)]
    /// #[fetch_vtable(uuid = "b745c60c-e258-4edc-86a9-9fb6b1191ce9")]
    /// struct OtherObj;
    ///
    /// let x = RefCell::new(SomeObj);
    /// let x: &RefCell<DynObj<dyn IBase>> = x.coerce_obj();
    /// assert!(matches!(x.downcast::<SomeObj>(), Some(_)));
    /// assert!(matches!(x.downcast::<OtherObj>(), None));
    /// ```
    #[inline]
    pub fn downcast<U>(&self) -> Option<&RefCell<U>>
    where
        U: ObjectId + Unsize<T> + 'static,
    {
        let obj = self.as_ptr();
        if crate::ptr::downcast::<U, _>(obj).is_some() {
            let cell = self as *const _ as *const RefCell<U>;
            unsafe { Some(&*cell) }
        } else {
            None
        }
    }
    /// Returns the downcasted `RefCell` if it is of type `U`.
    ///
    /// # Examples
    ///
    /// ```
    /// #![feature(unsize)]
    ///
    /// use fimo_ffi::cell::RefCell;
    /// use fimo_ffi::{DynObj, ObjectId};
    /// use fimo_ffi::ptr::{ObjInterface, IBase};
    ///
    /// // Define a custom object implementing the interface.
    /// #[derive(ObjectId)]
    /// #[fetch_vtable(uuid = "7ecb22c2-9426-46da-a7cb-8ad99eef582f")]
    /// struct SomeObj;
    ///
    /// #[derive(ObjectId)]
    /// #[fetch_vtable(uuid = "b745c60c-e258-4edc-86a9-9fb6b1191ce9")]
    /// struct OtherObj;
    ///
    /// let mut x = RefCell::new(SomeObj);
    /// let x: &mut RefCell<DynObj<dyn IBase>> = x.coerce_obj_mut();
    /// assert!(matches!(x.downcast_mut::<SomeObj>(), Some(_)));
    /// assert!(matches!(x.downcast_mut::<OtherObj>(), None));
    /// ```
    #[inline]
    pub fn downcast_mut<U>(&mut self) -> Option<&mut RefCell<U>>
    where
        U: ObjectId + Unsize<T> + 'static,
    {
        let obj = self.as_ptr();
        if crate::ptr::downcast::<U, _>(obj).is_some() {
            let cell = self as *mut _ as *mut RefCell<U>;
            unsafe { Some(&mut *cell) }
        } else {
            None
        }
    }

    /// Returns an arc to the super object.
    ///
    /// # Examples
    ///
    /// ```
    /// #![feature(const_trait_impl)]
    /// #![feature(unsize)]
    ///
    /// use fimo_ffi::{DynObj, ObjectId, interface};
    /// use fimo_ffi::cell::RefCell;
    /// use fimo_ffi::ptr::IBase;
    ///
    /// // Define a custom interface.
    /// interface! {
    ///     #![interface_cfg(uuid = "59dc47cf-fd2e-4d58-bcd4-5a31adc68a44")]
    ///     interface Obj: marker IBase { }
    /// }
    ///
    /// // Define a custom object implementing the interface.
    /// #[derive(ObjectId)]
    /// #[fetch_vtable(uuid = "7ecb22c2-9426-46da-a7cb-8ad99eef582f", interfaces(Obj))]
    /// struct MyObj(usize);
    ///
    /// impl Obj for MyObj { }
    ///
    /// let x = RefCell::new(MyObj(5));
    /// let x: &RefCell<DynObj<dyn Obj>> = x.coerce_obj();
    /// let x: &RefCell<DynObj<dyn IBase>> = x.cast_super();
    /// ```
    #[inline]
    pub fn cast_super<U>(&self) -> &RefCell<DynObj<U>>
    where
        T: CastInto<U>,
        U: ObjInterface + ?Sized,
    {
        let obj = self.as_ptr();
        let obj = crate::ptr::cast_super::<U, _>(obj);
        let metadata = std::ptr::metadata(obj);

        let ptr = self as *const _ as _;
        let ptr = std::ptr::from_raw_parts(ptr, metadata);

        unsafe { &*ptr }
    }

    /// Returns an arc to the super object.
    ///
    /// # Examples
    ///
    /// ```
    /// #![feature(const_trait_impl)]
    /// #![feature(unsize)]
    ///
    /// use fimo_ffi::{DynObj, ObjectId, interface};
    /// use fimo_ffi::cell::RefCell;
    /// use fimo_ffi::ptr::IBase;
    ///
    /// // Define a custom interface.
    /// interface! {
    ///     #![interface_cfg(uuid = "59dc47cf-fd2e-4d58-bcd4-5a31adc68a44")]
    ///     interface Obj: marker IBase { }
    /// }
    ///
    /// // Define a custom object implementing the interface.
    /// #[derive(ObjectId)]
    /// #[fetch_vtable(uuid = "7ecb22c2-9426-46da-a7cb-8ad99eef582f", interfaces(Obj))]
    /// struct MyObj(usize);
    ///
    /// impl Obj for MyObj { }
    ///
    /// let mut x = RefCell::new(MyObj(5));
    /// let x: &mut RefCell<DynObj<dyn Obj>> = x.coerce_obj_mut();
    /// let x: &mut RefCell<DynObj<dyn IBase>> = x.cast_super_mut();
    /// ```
    #[inline]
    pub fn cast_super_mut<U>(&mut self) -> &mut RefCell<DynObj<U>>
    where
        T: CastInto<U>,
        U: ObjInterface + ?Sized,
    {
        let obj = self.as_ptr();
        let obj = crate::ptr::cast_super_mut::<U, _>(obj);
        let metadata = std::ptr::metadata(obj);

        let ptr = self as *mut _ as _;
        let ptr = std::ptr::from_raw_parts_mut(ptr, metadata);

        unsafe { &mut *ptr }
    }

    /// Returns whether a certain interface is contained.
    ///
    /// # Examples
    ///
    /// ```
    /// #![feature(const_trait_impl)]
    /// #![feature(unsize)]
    ///
    /// use fimo_ffi::{DynObj, ObjectId, interface};
    /// use fimo_ffi::cell::RefCell;
    /// use fimo_ffi::ptr::IBase;
    ///
    /// // Define a custom interface.
    /// interface! {
    ///     #![interface_cfg(uuid = "59dc47cf-fd2e-4d58-bcd4-5a31adc68a44")]
    ///     interface Obj: marker IBase { }
    /// }
    ///
    /// // Define a custom object implementing the interface.
    /// #[derive(ObjectId)]
    /// #[fetch_vtable(uuid = "7ecb22c2-9426-46da-a7cb-8ad99eef582f", interfaces(Obj))]
    /// struct MyObj(usize);
    ///
    /// impl Obj for MyObj { }
    ///
    /// let x = RefCell::new(MyObj(5));
    /// let x: &RefCell<DynObj<dyn Obj>> = x.coerce_obj();
    /// let x: &RefCell<DynObj<dyn IBase>> = x.cast_super();
    /// assert_eq!(x.is_interface::<dyn Obj>(), true);
    /// assert_eq!(x.is_interface::<dyn IBase>(), false);
    #[inline]
    pub fn is_interface<U>(&self) -> bool
    where
        U: DowncastSafeInterface + Unsize<T> + Unsize<dyn crate::ptr::IBase + 'a> + ?Sized + 'a,
    {
        let obj = self.as_ptr();
        crate::ptr::is_interface::<U, _>(obj)
    }

    /// Returns a box to the downcasted interface if it is contained.
    ///
    /// # Examples
    ///
    /// ```
    /// #![feature(const_trait_impl)]
    /// #![feature(unsize)]
    ///
    /// use fimo_ffi::{DynObj, ObjectId, interface};
    /// use fimo_ffi::cell::RefCell;
    /// use fimo_ffi::ptr::IBase;
    ///
    /// // Define a custom interface.
    /// interface! {
    ///     #![interface_cfg(uuid = "59dc47cf-fd2e-4d58-bcd4-5a31adc68a44")]
    ///     interface Obj: marker IBase { }
    /// }
    ///
    /// // Define a custom object implementing the interface.
    /// #[derive(ObjectId)]
    /// #[fetch_vtable(uuid = "7ecb22c2-9426-46da-a7cb-8ad99eef582f", interfaces(Obj))]
    /// struct MyObj(usize);
    ///
    /// impl Obj for MyObj { }
    ///
    /// let x = RefCell::new(MyObj(5));
    /// let x: &RefCell<DynObj<dyn Obj>> = x.coerce_obj();
    /// let x: &RefCell<DynObj<dyn IBase>> = x.cast_super();
    /// assert!(matches!(x.downcast_interface::<dyn Obj>(), Some(_)));
    /// assert!(matches!(x.downcast_interface::<dyn IBase>(), None));
    #[inline]
    pub fn downcast_interface<U>(&self) -> Option<&RefCell<DynObj<U>>>
    where
        U: DowncastSafeInterface + Unsize<T> + Unsize<dyn crate::ptr::IBase + 'a> + ?Sized + 'a,
    {
        let obj = self.as_ptr();
        if let Some(obj) = crate::ptr::downcast_interface::<U, _>(obj) {
            let metadata = std::ptr::metadata(obj);
            let ptr = self as *const _ as _;
            let ptr = std::ptr::from_raw_parts(ptr, metadata);
            unsafe { Some(&*ptr) }
        } else {
            None
        }
    }

    /// Returns a box to the downcasted interface if it is contained.
    ///
    /// # Examples
    ///
    /// ```
    /// #![feature(const_trait_impl)]
    /// #![feature(unsize)]
    ///
    /// use fimo_ffi::{DynObj, ObjectId, interface};
    /// use fimo_ffi::cell::RefCell;
    /// use fimo_ffi::ptr::IBase;
    ///
    /// // Define a custom interface.
    /// interface! {
    ///     #![interface_cfg(uuid = "59dc47cf-fd2e-4d58-bcd4-5a31adc68a44")]
    ///     interface Obj: marker IBase { }
    /// }
    ///
    /// // Define a custom object implementing the interface.
    /// #[derive(ObjectId)]
    /// #[fetch_vtable(uuid = "7ecb22c2-9426-46da-a7cb-8ad99eef582f", interfaces(Obj))]
    /// struct MyObj(usize);
    ///
    /// impl Obj for MyObj { }
    ///
    /// let mut x = RefCell::new(MyObj(5));
    /// let x: &mut RefCell<DynObj<dyn Obj>> = x.coerce_obj_mut();
    /// let x: &mut RefCell<DynObj<dyn IBase>> = x.cast_super_mut();
    /// assert!(matches!(x.downcast_interface::<dyn Obj>(), Some(_)));
    /// assert!(matches!(x.downcast_interface::<dyn IBase>(), None));
    #[inline]
    pub fn downcast_interface_mut<U>(&mut self) -> Option<&mut RefCell<DynObj<U>>>
    where
        U: DowncastSafeInterface + Unsize<T> + Unsize<dyn crate::ptr::IBase + 'a> + ?Sized + 'a,
    {
        let obj = self.as_ptr();
        if let Some(obj) = crate::ptr::downcast_interface_mut::<U, _>(obj) {
            let metadata = std::ptr::metadata(obj);
            let ptr = self as *mut _ as _;
            let ptr = std::ptr::from_raw_parts_mut(ptr, metadata);
            unsafe { Some(&mut *ptr) }
        } else {
            None
        }
    }
}

impl<T: Default> RefCell<T> {
    /// Takes the wrapped value, leaving `Default::default()` in its place.
    ///
    /// # Panics
    ///
    /// Panics if the value is currently borrowed.
    ///
    /// # Examples
    ///
    /// ```
    /// use fimo_ffi::cell::RefCell;
    ///
    /// let c = RefCell::new(5);
    /// let five = c.take();
    ///
    /// assert_eq!(five, 5);
    /// assert_eq!(c.into_inner(), 0);
    /// ```
    pub fn take(&self) -> T {
        self.replace(Default::default())
    }
}

unsafe impl<T: ?Sized> Send for RefCell<T> where T: Send {}

impl<T: ?Sized> !Sync for RefCell<T> {}

impl<T: Clone> Clone for RefCell<T> {
    /// # Panics
    ///
    /// Panics if the value is currently mutably borrowed.
    #[inline]
    fn clone(&self) -> Self {
        RefCell::new(self.borrow().clone())
    }

    /// # Panics
    ///
    /// Panics if `other` is currently mutably borrowed.
    #[inline]
    fn clone_from(&mut self, other: &Self) {
        self.get_mut().clone_from(&other.borrow())
    }
}

impl<T: Default> Default for RefCell<T> {
    /// Creates a `RefCell<T>`, with the `Default` value for T.
    #[inline]
    fn default() -> RefCell<T> {
        RefCell::new(Default::default())
    }
}

impl<T: ?Sized + PartialEq> PartialEq for RefCell<T> {
    /// # Panics
    ///
    /// Panics if the value in either `RefCell` is currently borrowed.
    #[inline]
    fn eq(&self, other: &RefCell<T>) -> bool {
        *self.borrow() == *other.borrow()
    }
}

impl<T: ?Sized + Eq> Eq for RefCell<T> {}

impl<T: ?Sized + PartialOrd> PartialOrd for RefCell<T> {
    /// # Panics
    ///
    /// Panics if the value in either `RefCell` is currently borrowed.
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.borrow().partial_cmp(&*other.borrow())
    }

    /// # Panics
    ///
    /// Panics if the value in either `RefCell` is currently borrowed.
    #[inline]
    fn lt(&self, other: &Self) -> bool {
        *self.borrow() < *other.borrow()
    }

    /// # Panics
    ///
    /// Panics if the value in either `RefCell` is currently borrowed.
    #[inline]
    fn le(&self, other: &RefCell<T>) -> bool {
        *self.borrow() <= *other.borrow()
    }

    /// # Panics
    ///
    /// Panics if the value in either `RefCell` is currently borrowed.
    #[inline]
    fn gt(&self, other: &RefCell<T>) -> bool {
        *self.borrow() > *other.borrow()
    }

    /// # Panics
    ///
    /// Panics if the value in either `RefCell` is currently borrowed.
    #[inline]
    fn ge(&self, other: &RefCell<T>) -> bool {
        *self.borrow() >= *other.borrow()
    }
}

impl<T: ?Sized + Ord> Ord for RefCell<T> {
    /// # Panics
    ///
    /// Panics if the value in either `RefCell` is currently borrowed.
    #[inline]
    fn cmp(&self, other: &RefCell<T>) -> Ordering {
        self.borrow().cmp(&*other.borrow())
    }
}

impl<T: ?Sized + std::fmt::Debug> std::fmt::Debug for RefCell<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.try_borrow() {
            Ok(borrow) => f.debug_struct("RefCell").field("value", &borrow).finish(),
            Err(_) => {
                // The RefCell is mutably borrowed so we can't look at its value
                // here. Show a placeholder instead.
                struct BorrowedPlaceholder;

                impl std::fmt::Debug for BorrowedPlaceholder {
                    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        f.write_str("<borrowed>")
                    }
                }

                f.debug_struct("RefCell")
                    .field("value", &BorrowedPlaceholder)
                    .finish()
            }
        }
    }
}

impl<T> const From<T> for RefCell<T> {
    fn from(t: T) -> RefCell<T> {
        RefCell::new(t)
    }
}

impl<T> AtomicRefCell<T> {
    /// Creates a new `AtomicRefCell` containing `value`.
    ///
    /// # Examples
    ///
    /// ```
    /// use fimo_ffi::cell::AtomicRefCell;
    ///
    /// let c = AtomicRefCell::new(5);
    /// ```
    #[inline]
    pub const fn new(value: T) -> AtomicRefCell<T> {
        AtomicRefCell {
            borrow: AtomicBorrowFlag::new(UNUSED_ATOMIC),
            value: UnsafeCell::new(value),
        }
    }

    /// Consumes the `AtomicRefCell`, returning the wrapped value.
    ///
    /// # Examples
    ///
    /// ```
    /// use fimo_ffi::cell::AtomicRefCell;
    ///
    /// let c = AtomicRefCell::new(5);
    ///
    /// let five = c.into_inner();
    /// ```
    #[inline]
    pub fn into_inner(self) -> T {
        self.value.into_inner()
    }

    /// Replaces the wrapped value with a new one, returning the old value,
    /// without deinitializing either one.
    ///
    /// This function corresponds to [`std::mem::replace`](../mem/fn.replace.html).
    ///
    /// # Panics
    ///
    /// Panics if the value is currently borrowed.
    ///
    /// # Examples
    ///
    /// ```
    /// use fimo_ffi::cell::AtomicRefCell;
    /// let cell = AtomicRefCell::new(5);
    /// let old_value = cell.replace(6);
    /// assert_eq!(old_value, 5);
    /// assert_eq!(cell, AtomicRefCell::new(6));
    /// ```
    #[inline]
    pub fn replace(&self, t: T) -> T {
        std::mem::replace(&mut *self.borrow_mut(), t)
    }

    /// Replaces the wrapped value with a new one computed from `f`, returning
    /// the old value, without deinitializing either one.
    ///
    /// # Panics
    ///
    /// Panics if the value is currently borrowed.
    ///
    /// # Examples
    ///
    /// ```
    /// use fimo_ffi::cell::AtomicRefCell;
    /// let cell = AtomicRefCell::new(5);
    /// let old_value = cell.replace_with(|&mut old| old + 1);
    /// assert_eq!(old_value, 5);
    /// assert_eq!(cell, AtomicRefCell::new(6));
    /// ```
    #[inline]
    pub fn replace_with<F: FnOnce(&mut T) -> T>(&self, f: F) -> T {
        let mut_borrow = &mut *self.borrow_mut();
        let replacement = f(mut_borrow);
        std::mem::replace(mut_borrow, replacement)
    }

    /// Swaps the wrapped value of `self` with the wrapped value of `other`,
    /// without deinitializing either one.
    ///
    /// This function corresponds to [`std::mem::swap`](../mem/fn.swap.html).
    ///
    /// # Panics
    ///
    /// Panics if the value in either `AtomicRefCell` is currently borrowed.
    ///
    /// # Examples
    ///
    /// ```
    /// use fimo_ffi::cell::AtomicRefCell;
    /// let c = AtomicRefCell::new(5);
    /// let d = AtomicRefCell::new(6);
    /// c.swap(&d);
    /// assert_eq!(c, AtomicRefCell::new(6));
    /// assert_eq!(d, AtomicRefCell::new(5));
    /// ```
    #[inline]
    pub fn swap(&self, other: &Self) {
        std::mem::swap(&mut *self.borrow_mut(), &mut *other.borrow_mut())
    }
}

impl<T: ?Sized> AtomicRefCell<T> {
    /// Immutably borrows the wrapped value.
    ///
    /// The borrow lasts until the returned `AtomicRef` exits scope. Multiple
    /// immutable borrows can be taken out at the same time.
    ///
    /// # Panics
    ///
    /// Panics if the value is currently mutably borrowed. For a non-panicking variant, use
    /// [`try_borrow`](#method.try_borrow).
    ///
    /// # Examples
    ///
    /// ```
    /// use fimo_ffi::cell::AtomicRefCell;
    ///
    /// let c = AtomicRefCell::new(5);
    ///
    /// let borrowed_five = c.borrow();
    /// let borrowed_five2 = c.borrow();
    /// ```
    ///
    /// An example of panic:
    ///
    /// ```should_panic
    /// use fimo_ffi::cell::AtomicRefCell;
    ///
    /// let c = AtomicRefCell::new(5);
    ///
    /// let m = c.borrow_mut();
    /// let b = c.borrow(); // this causes a panic
    /// ```
    #[inline]
    pub fn borrow(&self) -> AtomicRef<'_, T> {
        self.try_borrow().expect("already mutably borrowed")
    }

    /// Immutably borrows the wrapped value, returning an error if the value is currently mutably
    /// borrowed.
    ///
    /// The borrow lasts until the returned `AtomicRef` exits scope. Multiple immutable borrows can be
    /// taken out at the same time.
    ///
    /// This is the non-panicking variant of [`borrow`](#method.borrow).
    ///
    /// # Examples
    ///
    /// ```
    /// use fimo_ffi::cell::AtomicRefCell;
    ///
    /// let c = AtomicRefCell::new(5);
    ///
    /// {
    ///     let m = c.borrow_mut();
    ///     assert!(c.try_borrow().is_err());
    /// }
    ///
    /// {
    ///     let m = c.borrow();
    ///     assert!(c.try_borrow().is_ok());
    /// }
    /// ```
    #[inline]
    pub fn try_borrow(&self) -> Result<AtomicRef<'_, T>, BorrowError> {
        match AtomicBorrowRef::new(&self.borrow) {
            Some(b) => {
                // SAFETY: `AtomicBorrowRef` ensures that there is only immutable access
                // to the value while borrowed.
                Ok(AtomicRef {
                    value: unsafe { &*self.value.get() },
                    borrow: b,
                })
            }
            None => Err(BorrowError { _v: 0 }),
        }
    }

    /// Mutably borrows the wrapped value.
    ///
    /// The borrow lasts until the returned `AtomicRefMut` or all `AtomicRefMut`s derived
    /// from it exit scope. The value cannot be borrowed while this borrow is
    /// active.
    ///
    /// # Panics
    ///
    /// Panics if the value is currently borrowed. For a non-panicking variant, use
    /// [`try_borrow_mut`](#method.try_borrow_mut).
    ///
    /// # Examples
    ///
    /// ```
    /// use fimo_ffi::cell::AtomicRefCell;
    ///
    /// let c = AtomicRefCell::new("hello".to_owned());
    ///
    /// *c.borrow_mut() = "bonjour".to_owned();
    ///
    /// assert_eq!(&*c.borrow(), "bonjour");
    /// ```
    ///
    /// An example of panic:
    ///
    /// ```should_panic
    /// use fimo_ffi::cell::AtomicRefCell;
    ///
    /// let c = AtomicRefCell::new(5);
    /// let m = c.borrow();
    ///
    /// let b = c.borrow_mut(); // this causes a panic
    /// ```
    #[inline]
    pub fn borrow_mut(&self) -> AtomicRefMut<'_, T> {
        self.try_borrow_mut().expect("already borrowed")
    }

    /// Mutably borrows the wrapped value, returning an error if the value is currently borrowed.
    ///
    /// The borrow lasts until the returned `AtomicRefMut` or all `AtomicRefMut`s derived
    /// from it exit scope. The value cannot be borrowed while this borrow is
    /// active.
    ///
    /// This is the non-panicking variant of [`borrow_mut`](#method.borrow_mut).
    ///
    /// # Examples
    ///
    /// ```
    /// use fimo_ffi::cell::AtomicRefCell;
    ///
    /// let c = AtomicRefCell::new(5);
    ///
    /// {
    ///     let m = c.borrow();
    ///     assert!(c.try_borrow_mut().is_err());
    /// }
    ///
    /// assert!(c.try_borrow_mut().is_ok());
    /// ```
    #[inline]
    pub fn try_borrow_mut(&self) -> Result<AtomicRefMut<'_, T>, BorrowMutError> {
        match AtomicBorrowRefMut::new(&self.borrow) {
            Some(b) => {
                // SAFETY: `BorrowRef` guarantees unique access.
                Ok(AtomicRefMut {
                    value: unsafe { &mut *self.value.get() },
                    borrow: b,
                })
            }
            None => Err(BorrowMutError { _v: 0 }),
        }
    }

    /// Returns a raw pointer to the underlying data in this cell.
    ///
    /// # Examples
    ///
    /// ```
    /// use fimo_ffi::cell::AtomicRefCell;
    ///
    /// let c = AtomicRefCell::new(5);
    ///
    /// let ptr = c.as_ptr();
    /// ```
    #[inline]
    pub fn as_ptr(&self) -> *mut T {
        self.value.get()
    }

    /// Returns a mutable reference to the underlying data.
    ///
    /// This call borrows `AtomicRefCell` mutably (at compile-time) so there is no
    /// need for dynamic checks.
    ///
    /// However be cautious: this method expects `self` to be mutable, which is
    /// generally not the case when using a `AtomicRefCell`. Take a look at the
    /// [`borrow_mut`] method instead if `self` isn't mutable.
    ///
    /// Also, please be aware that this method is only for special circumstances and is usually
    /// not what you want. In case of doubt, use [`borrow_mut`] instead.
    ///
    /// [`borrow_mut`]: AtomicRefCell::borrow_mut()
    ///
    /// # Examples
    ///
    /// ```
    /// use fimo_ffi::cell::AtomicRefCell;
    ///
    /// let mut c = AtomicRefCell::new(5);
    /// *c.get_mut() += 1;
    ///
    /// assert_eq!(c, AtomicRefCell::new(6));
    /// ```
    #[inline]
    pub fn get_mut(&mut self) -> &mut T {
        self.value.get_mut()
    }

    /// Undo the effect of leaked guards on the borrow state of the `AtomicRefCell`.
    ///
    /// This call is similar to [`get_mut`] but more specialized. It borrows `AtomicRefCell` mutably to
    /// ensure no borrows exist and then resets the state tracking shared borrows. This is relevant
    /// if some `AtomicRef` or `AtomicRefMut` borrows have been leaked.
    ///
    /// [`get_mut`]: AtomicRefCell::get_mut()
    ///
    /// # Examples
    ///
    /// ```
    /// use fimo_ffi::cell::AtomicRefCell;
    ///
    /// let mut c = AtomicRefCell::new(0);
    /// std::mem::forget(c.borrow_mut());
    ///
    /// assert!(c.try_borrow().is_err());
    /// c.undo_leak();
    /// assert!(c.try_borrow().is_ok());
    /// ```
    pub fn undo_leak(&mut self) -> &mut T {
        self.borrow
            .store(UNUSED_ATOMIC, std::sync::atomic::Ordering::Release);
        self.get_mut()
    }

    /// Immutably borrows the wrapped value, returning an error if the value is
    /// currently mutably borrowed.
    ///
    /// # Safety
    ///
    /// Unlike `AtomicRefCell::borrow`, this method is unsafe because it does not
    /// return a `AtomicRef`, thus leaving the borrow flag untouched. Mutably
    /// borrowing the `AtomicRefCell` while the reference returned by this method
    /// is alive is undefined behaviour.
    ///
    /// # Examples
    ///
    /// ```
    /// use fimo_ffi::cell::AtomicRefCell;
    ///
    /// let c = AtomicRefCell::new(5);
    ///
    /// {
    ///     let m = c.borrow_mut();
    ///     assert!(unsafe { c.try_borrow_unguarded() }.is_err());
    /// }
    ///
    /// {
    ///     let m = c.borrow();
    ///     assert!(unsafe { c.try_borrow_unguarded() }.is_ok());
    /// }
    /// ```
    #[inline]
    pub unsafe fn try_borrow_unguarded(&self) -> Result<&T, BorrowError> {
        if !atomic_is_writing(self.borrow.load(std::sync::atomic::Ordering::Acquire)) {
            // SAFETY: We check that nobody is actively writing now, but it is
            // the caller's responsibility to ensure that nobody writes until
            // the returned reference is no longer in use.
            // Also, `self.value.get()` refers to the value owned by `self`
            // and is thus guaranteed to be valid for the lifetime of `self`.
            Ok(&*self.value.get())
        } else {
            Err(BorrowError { _v: 0 })
        }
    }
}

impl<T: ?Sized> AtomicRefCell<T> {
    /// Coerces a `AtomicRefCell<T>` to an `AtomicRefCell<DynObj<U>>`.
    ///
    /// # Examples
    ///
    /// ```
    /// #![feature(const_trait_impl)]
    /// #![feature(unsize)]
    ///
    /// use fimo_ffi::{DynObj, ObjectId, interface};
    /// use fimo_ffi::ptr::{CastInto, IBase};
    /// use fimo_ffi::cell::AtomicRefCell;
    ///
    /// // Define a custom interface.
    /// interface! {
    ///     #![interface_cfg(uuid = "59dc47cf-fd2e-4d58-bcd4-5a31adc68a44")]
    ///     interface Obj: marker IBase {
    ///         fn add(&self, num: usize) -> usize;
    ///     }
    /// }
    ///
    /// // Define a custom object implementing the interface.
    /// #[derive(ObjectId)]
    /// #[fetch_vtable(uuid = "7ecb22c2-9426-46da-a7cb-8ad99eef582f", interfaces(Obj))]
    /// struct MyObj(usize);
    ///
    /// impl Obj for MyObj {
    ///     fn add(&self, num: usize) -> usize {
    ///         self.0 + num
    ///     }
    /// }
    ///
    /// let x = AtomicRefCell::new(MyObj(5));
    /// let x: &AtomicRefCell<DynObj<dyn Obj>> = x.coerce_obj();
    ///
    /// {
    ///     let x = x.borrow();
    ///     assert_eq!(x.add(0), 5);
    ///     assert_eq!(x.add(1), 6);
    ///     assert_eq!(x.add(5), 10);
    /// }
    /// ```
    #[inline]
    pub fn coerce_obj<U>(&self) -> &AtomicRefCell<DynObj<U>>
    where
        T: FetchVTable<U::Base> + Unsize<U>,
        U: ObjInterface + ?Sized,
    {
        let vtable = T::fetch_interface();
        let metadata = crate::ptr::ObjMetadata::<U>::new(vtable);
        let obj = crate::ptr::from_raw_parts(std::ptr::null(), metadata);

        let metadata = std::ptr::metadata(obj);
        let ref_obj = std::ptr::from_raw_parts(self as *const _ as _, metadata);

        // SAFETY: both the data pointer and metadata are valid
        unsafe { &*ref_obj }
    }

    /// Coerces a `AtomicRefCell<T>` to an `AtomicRefCell<DynObj<U>>`.
    ///
    /// # Examples
    ///
    /// ```
    /// #![feature(const_trait_impl)]
    /// #![feature(unsize)]
    ///
    /// use fimo_ffi::{DynObj, ObjectId, interface};
    /// use fimo_ffi::ptr::{CastInto, IBase};
    /// use fimo_ffi::cell::AtomicRefCell;
    ///
    ///
    /// // Define a custom interface.
    /// interface! {
    ///     #![interface_cfg(uuid = "59dc47cf-fd2e-4d58-bcd4-5a31adc68a44")]
    ///     interface Obj: marker IBase {
    ///         fn set(&mut self, num: usize);
    ///     }
    /// }
    ///
    /// // Define a custom object implementing the interface.
    /// #[derive(ObjectId)]
    /// #[fetch_vtable(uuid = "7ecb22c2-9426-46da-a7cb-8ad99eef582f", interfaces(Obj))]
    /// struct MyObj(usize);
    ///
    /// impl Obj for MyObj {
    ///     fn set(&mut self, num: usize) {
    ///         self.0 = num
    ///     }
    /// }
    ///
    /// let mut x = AtomicRefCell::new(MyObj(5));
    ///
    /// {
    ///     let x: &mut AtomicRefCell<DynObj<dyn Obj>> = x.coerce_obj_mut();
    ///     let mut x = x.borrow_mut();
    ///     x.set(5);
    /// }
    ///
    /// assert_eq!(x.get_mut().0, 5)
    /// ```
    #[inline]
    pub fn coerce_obj_mut<U>(&mut self) -> &mut AtomicRefCell<DynObj<U>>
    where
        T: FetchVTable<U::Base> + Unsize<U>,
        U: ObjInterface + ?Sized,
    {
        let vtable = T::fetch_interface();
        let metadata = crate::ptr::ObjMetadata::<U>::new(vtable);
        let obj = crate::ptr::from_raw_parts(std::ptr::null(), metadata);

        let metadata = std::ptr::metadata(obj);
        let ref_obj = std::ptr::from_raw_parts_mut(self as *mut _ as _, metadata);

        // SAFETY: both the data pointer and metadata are valid
        unsafe { &mut *ref_obj }
    }
}

impl<'a, T: ?Sized + 'a> AtomicRefCell<DynObj<T>> {
    /// Returns whether the contained object is of type `U`.
    ///
    /// # Examples
    ///
    /// ```
    /// #![feature(unsize)]
    ///
    /// use fimo_ffi::cell::AtomicRefCell;
    /// use fimo_ffi::{DynObj, ObjectId};
    /// use fimo_ffi::ptr::{ObjInterface, IBase};
    ///
    /// // Define a custom object implementing the interface.
    /// #[derive(ObjectId)]
    /// #[fetch_vtable(uuid = "7ecb22c2-9426-46da-a7cb-8ad99eef582f")]
    /// struct SomeObj;
    ///
    /// #[derive(ObjectId)]
    /// #[fetch_vtable(uuid = "b745c60c-e258-4edc-86a9-9fb6b1191ce9")]
    /// struct OtherObj;
    ///
    /// let x = AtomicRefCell::new(SomeObj);
    /// let x: &AtomicRefCell<DynObj<dyn IBase>> = x.coerce_obj();
    /// assert_eq!(x.is::<SomeObj>(), true);
    /// assert_eq!(x.is::<OtherObj>(), false);
    /// ```
    #[inline]
    pub fn is<U>(&self) -> bool
    where
        U: ObjectId + Unsize<T> + 'static,
    {
        let obj = self.as_ptr();
        fimo_ffi::ptr::is::<U, _>(obj)
    }

    /// Returns the downcasted `AtomicRefCell` if it is of type `U`.
    ///
    /// # Examples
    ///
    /// ```
    /// #![feature(unsize)]
    ///
    /// use fimo_ffi::cell::AtomicRefCell;
    /// use fimo_ffi::{DynObj, ObjectId};
    /// use fimo_ffi::ptr::{ObjInterface, IBase};
    ///
    /// // Define a custom object implementing the interface.
    /// #[derive(ObjectId)]
    /// #[fetch_vtable(uuid = "7ecb22c2-9426-46da-a7cb-8ad99eef582f")]
    /// struct SomeObj;
    ///
    /// #[derive(ObjectId)]
    /// #[fetch_vtable(uuid = "b745c60c-e258-4edc-86a9-9fb6b1191ce9")]
    /// struct OtherObj;
    ///
    /// let x = AtomicRefCell::new(SomeObj);
    /// let x: &AtomicRefCell<DynObj<dyn IBase>> = x.coerce_obj();
    /// assert!(matches!(x.downcast::<SomeObj>(), Some(_)));
    /// assert!(matches!(x.downcast::<OtherObj>(), None));
    /// ```
    #[inline]
    pub fn downcast<U>(&self) -> Option<&AtomicRefCell<U>>
    where
        U: ObjectId + Unsize<T> + 'static,
    {
        let obj = self.as_ptr();
        if crate::ptr::downcast::<U, _>(obj).is_some() {
            let cell = self as *const _ as *const AtomicRefCell<U>;
            unsafe { Some(&*cell) }
        } else {
            None
        }
    }
    /// Returns the downcasted `AtomicRefCell` if it is of type `U`.
    ///
    /// # Examples
    ///
    /// ```
    /// #![feature(unsize)]
    ///
    /// use fimo_ffi::cell::AtomicRefCell;
    /// use fimo_ffi::{DynObj, ObjectId};
    /// use fimo_ffi::ptr::{ObjInterface, IBase};
    ///
    /// // Define a custom object implementing the interface.
    /// #[derive(ObjectId)]
    /// #[fetch_vtable(uuid = "7ecb22c2-9426-46da-a7cb-8ad99eef582f")]
    /// struct SomeObj;
    ///
    /// #[derive(ObjectId)]
    /// #[fetch_vtable(uuid = "b745c60c-e258-4edc-86a9-9fb6b1191ce9")]
    /// struct OtherObj;
    ///
    /// let mut x = AtomicRefCell::new(SomeObj);
    /// let x: &mut AtomicRefCell<DynObj<dyn IBase>> = x.coerce_obj_mut();
    /// assert!(matches!(x.downcast_mut::<SomeObj>(), Some(_)));
    /// assert!(matches!(x.downcast_mut::<OtherObj>(), None));
    /// ```
    #[inline]
    pub fn downcast_mut<U>(&mut self) -> Option<&mut AtomicRefCell<U>>
    where
        U: ObjectId + Unsize<T> + 'static,
    {
        let obj = self.as_ptr();
        if crate::ptr::downcast::<U, _>(obj).is_some() {
            let cell = self as *mut _ as *mut AtomicRefCell<U>;
            unsafe { Some(&mut *cell) }
        } else {
            None
        }
    }

    /// Returns an arc to the super object.
    ///
    /// # Examples
    ///
    /// ```
    /// #![feature(const_trait_impl)]
    /// #![feature(unsize)]
    ///
    /// use fimo_ffi::{DynObj, ObjectId, interface};
    /// use fimo_ffi::cell::AtomicRefCell;
    /// use fimo_ffi::ptr::IBase;
    ///
    /// // Define a custom interface.
    /// interface! {
    ///     #![interface_cfg(uuid = "59dc47cf-fd2e-4d58-bcd4-5a31adc68a44")]
    ///     interface Obj: marker IBase { }
    /// }
    ///
    /// // Define a custom object implementing the interface.
    /// #[derive(ObjectId)]
    /// #[fetch_vtable(uuid = "7ecb22c2-9426-46da-a7cb-8ad99eef582f", interfaces(Obj))]
    /// struct MyObj(usize);
    ///
    /// impl Obj for MyObj { }
    ///
    /// let x = AtomicRefCell::new(MyObj(5));
    /// let x: &AtomicRefCell<DynObj<dyn Obj>> = x.coerce_obj();
    /// let x: &AtomicRefCell<DynObj<dyn IBase>> = x.cast_super();
    /// ```
    #[inline]
    pub fn cast_super<U>(&self) -> &AtomicRefCell<DynObj<U>>
    where
        T: CastInto<U>,
        U: ObjInterface + ?Sized,
    {
        let obj = self.as_ptr();
        let obj = crate::ptr::cast_super::<U, _>(obj);
        let metadata = std::ptr::metadata(obj);

        let ptr = self as *const _ as _;
        let ptr = std::ptr::from_raw_parts(ptr, metadata);

        unsafe { &*ptr }
    }

    /// Returns an arc to the super object.
    ///
    /// # Examples
    ///
    /// ```
    /// #![feature(const_trait_impl)]
    /// #![feature(unsize)]
    ///
    /// use fimo_ffi::{DynObj, ObjectId, interface};
    /// use fimo_ffi::cell::AtomicRefCell;
    /// use fimo_ffi::ptr::IBase;
    ///
    /// // Define a custom interface.
    /// interface! {
    ///     #![interface_cfg(uuid = "59dc47cf-fd2e-4d58-bcd4-5a31adc68a44")]
    ///     interface Obj: marker IBase { }
    /// }
    ///
    /// // Define a custom object implementing the interface.
    /// #[derive(ObjectId)]
    /// #[fetch_vtable(uuid = "7ecb22c2-9426-46da-a7cb-8ad99eef582f", interfaces(Obj))]
    /// struct MyObj(usize);
    ///
    /// impl Obj for MyObj { }
    ///
    /// let mut x = AtomicRefCell::new(MyObj(5));
    /// let x: &mut AtomicRefCell<DynObj<dyn Obj>> = x.coerce_obj_mut();
    /// let x: &mut AtomicRefCell<DynObj<dyn IBase>> = x.cast_super_mut();
    /// ```
    #[inline]
    pub fn cast_super_mut<U>(&mut self) -> &mut AtomicRefCell<DynObj<U>>
    where
        T: CastInto<U>,
        U: ObjInterface + ?Sized,
    {
        let obj = self.as_ptr();
        let obj = crate::ptr::cast_super_mut::<U, _>(obj);
        let metadata = std::ptr::metadata(obj);

        let ptr = self as *mut _ as _;
        let ptr = std::ptr::from_raw_parts_mut(ptr, metadata);

        unsafe { &mut *ptr }
    }

    /// Returns whether a certain interface is contained.
    ///
    /// # Examples
    ///
    /// ```
    /// #![feature(const_trait_impl)]
    /// #![feature(unsize)]
    ///
    /// use fimo_ffi::{DynObj, ObjectId, interface};
    /// use fimo_ffi::cell::AtomicRefCell;
    /// use fimo_ffi::ptr::IBase;
    ///
    /// // Define a custom interface.
    /// interface! {
    ///     #![interface_cfg(uuid = "59dc47cf-fd2e-4d58-bcd4-5a31adc68a44")]
    ///     interface Obj: marker IBase { }
    /// }
    ///
    /// // Define a custom object implementing the interface.
    /// #[derive(ObjectId)]
    /// #[fetch_vtable(uuid = "7ecb22c2-9426-46da-a7cb-8ad99eef582f", interfaces(Obj))]
    /// struct MyObj(usize);
    ///
    /// impl Obj for MyObj { }
    ///
    /// let x = AtomicRefCell::new(MyObj(5));
    /// let x: &AtomicRefCell<DynObj<dyn Obj>> = x.coerce_obj();
    /// let x: &AtomicRefCell<DynObj<dyn IBase>> = x.cast_super();
    /// assert_eq!(x.is_interface::<dyn Obj>(), true);
    /// assert_eq!(x.is_interface::<dyn IBase>(), false);
    #[inline]
    pub fn is_interface<U>(&self) -> bool
    where
        U: DowncastSafeInterface + Unsize<T> + Unsize<dyn crate::ptr::IBase + 'a> + ?Sized + 'a,
    {
        let obj = self.as_ptr();
        crate::ptr::is_interface::<U, _>(obj)
    }

    /// Returns a box to the downcasted interface if it is contained.
    ///
    /// # Examples
    ///
    /// ```
    /// #![feature(const_trait_impl)]
    /// #![feature(unsize)]
    ///
    /// use fimo_ffi::{DynObj, ObjectId, interface};
    /// use fimo_ffi::cell::AtomicRefCell;
    /// use fimo_ffi::ptr::IBase;
    ///
    /// // Define a custom interface.
    /// interface! {
    ///     #![interface_cfg(uuid = "59dc47cf-fd2e-4d58-bcd4-5a31adc68a44")]
    ///     interface Obj: marker IBase { }
    /// }
    ///
    /// // Define a custom object implementing the interface.
    /// #[derive(ObjectId)]
    /// #[fetch_vtable(uuid = "7ecb22c2-9426-46da-a7cb-8ad99eef582f", interfaces(Obj))]
    /// struct MyObj(usize);
    ///
    /// impl Obj for MyObj { }
    ///
    /// let x = AtomicRefCell::new(MyObj(5));
    /// let x: &AtomicRefCell<DynObj<dyn Obj>> = x.coerce_obj();
    /// let x: &AtomicRefCell<DynObj<dyn IBase>> = x.cast_super();
    /// assert!(matches!(x.downcast_interface::<dyn Obj>(), Some(_)));
    /// assert!(matches!(x.downcast_interface::<dyn IBase>(), None));
    #[inline]
    pub fn downcast_interface<U>(&self) -> Option<&AtomicRefCell<DynObj<U>>>
    where
        U: DowncastSafeInterface + Unsize<T> + Unsize<dyn crate::ptr::IBase + 'a> + ?Sized + 'a,
    {
        let obj = self.as_ptr();
        if let Some(obj) = crate::ptr::downcast_interface::<U, _>(obj) {
            let metadata = std::ptr::metadata(obj);
            let ptr = self as *const _ as _;
            let ptr = std::ptr::from_raw_parts(ptr, metadata);
            unsafe { Some(&*ptr) }
        } else {
            None
        }
    }

    /// Returns a box to the downcasted interface if it is contained.
    ///
    /// # Examples
    ///
    /// ```
    /// #![feature(const_trait_impl)]
    /// #![feature(unsize)]
    ///
    /// use fimo_ffi::{DynObj, ObjectId, interface};
    /// use fimo_ffi::cell::AtomicRefCell;
    /// use fimo_ffi::ptr::IBase;
    ///
    /// // Define a custom interface.
    /// interface! {
    ///     #![interface_cfg(uuid = "59dc47cf-fd2e-4d58-bcd4-5a31adc68a44")]
    ///     interface Obj: marker IBase { }
    /// }
    ///
    /// // Define a custom object implementing the interface.
    /// #[derive(ObjectId)]
    /// #[fetch_vtable(uuid = "7ecb22c2-9426-46da-a7cb-8ad99eef582f", interfaces(Obj))]
    /// struct MyObj(usize);
    ///
    /// impl Obj for MyObj { }
    ///
    /// let mut x = AtomicRefCell::new(MyObj(5));
    /// let x: &mut AtomicRefCell<DynObj<dyn Obj>> = x.coerce_obj_mut();
    /// let x: &mut AtomicRefCell<DynObj<dyn IBase>> = x.cast_super_mut();
    /// assert!(matches!(x.downcast_interface::<dyn Obj>(), Some(_)));
    /// assert!(matches!(x.downcast_interface::<dyn IBase>(), None));
    #[inline]
    pub fn downcast_interface_mut<U>(&mut self) -> Option<&mut AtomicRefCell<DynObj<U>>>
    where
        U: DowncastSafeInterface + Unsize<T> + Unsize<dyn crate::ptr::IBase + 'a> + ?Sized + 'a,
    {
        let obj = self.as_ptr();
        if let Some(obj) = crate::ptr::downcast_interface_mut::<U, _>(obj) {
            let metadata = std::ptr::metadata(obj);
            let ptr = self as *mut _ as _;
            let ptr = std::ptr::from_raw_parts_mut(ptr, metadata);
            unsafe { Some(&mut *ptr) }
        } else {
            None
        }
    }
}

impl<T: Default> AtomicRefCell<T> {
    /// Takes the wrapped value, leaving `Default::default()` in its place.
    ///
    /// # Panics
    ///
    /// Panics if the value is currently borrowed.
    ///
    /// # Examples
    ///
    /// ```
    /// use fimo_ffi::cell::AtomicRefCell;
    ///
    /// let c = AtomicRefCell::new(5);
    /// let five = c.take();
    ///
    /// assert_eq!(five, 5);
    /// assert_eq!(c.into_inner(), 0);
    /// ```
    pub fn take(&self) -> T {
        self.replace(Default::default())
    }
}

unsafe impl<T: ?Sized> Send for AtomicRefCell<T> where T: Send {}

unsafe impl<T: ?Sized> Sync for AtomicRefCell<T> where T: Send + Sync {}

impl<T: Clone> Clone for AtomicRefCell<T> {
    /// # Panics
    ///
    /// Panics if the value is currently mutably borrowed.
    #[inline]
    fn clone(&self) -> Self {
        AtomicRefCell::new(self.borrow().clone())
    }

    /// # Panics
    ///
    /// Panics if `other` is currently mutably borrowed.
    #[inline]
    fn clone_from(&mut self, other: &Self) {
        self.get_mut().clone_from(&other.borrow())
    }
}

impl<T: Default> Default for AtomicRefCell<T> {
    /// Creates a `AtomicRefCell<T>`, with the `Default` value for T.
    #[inline]
    fn default() -> AtomicRefCell<T> {
        AtomicRefCell::new(Default::default())
    }
}

impl<T: ?Sized + PartialEq> PartialEq for AtomicRefCell<T> {
    /// # Panics
    ///
    /// Panics if the value in either `AtomicRefCell` is currently borrowed.
    #[inline]
    fn eq(&self, other: &AtomicRefCell<T>) -> bool {
        *self.borrow() == *other.borrow()
    }
}

impl<T: ?Sized + Eq> Eq for AtomicRefCell<T> {}

impl<T: ?Sized + PartialOrd> PartialOrd for AtomicRefCell<T> {
    /// # Panics
    ///
    /// Panics if the value in either `AtomicRefCell` is currently borrowed.
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.borrow().partial_cmp(&*other.borrow())
    }

    /// # Panics
    ///
    /// Panics if the value in either `AtomicRefCell` is currently borrowed.
    #[inline]
    fn lt(&self, other: &Self) -> bool {
        *self.borrow() < *other.borrow()
    }

    /// # Panics
    ///
    /// Panics if the value in either `AtomicRefCell` is currently borrowed.
    #[inline]
    fn le(&self, other: &AtomicRefCell<T>) -> bool {
        *self.borrow() <= *other.borrow()
    }

    /// # Panics
    ///
    /// Panics if the value in either `AtomicRefCell` is currently borrowed.
    #[inline]
    fn gt(&self, other: &AtomicRefCell<T>) -> bool {
        *self.borrow() > *other.borrow()
    }

    /// # Panics
    ///
    /// Panics if the value in either `AtomicRefCell` is currently borrowed.
    #[inline]
    fn ge(&self, other: &AtomicRefCell<T>) -> bool {
        *self.borrow() >= *other.borrow()
    }
}

impl<T: ?Sized + Ord> Ord for AtomicRefCell<T> {
    /// # Panics
    ///
    /// Panics if the value in either `AtomicRefCell` is currently borrowed.
    #[inline]
    fn cmp(&self, other: &AtomicRefCell<T>) -> Ordering {
        self.borrow().cmp(&*other.borrow())
    }
}

impl<T: ?Sized + std::fmt::Debug> std::fmt::Debug for AtomicRefCell<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.try_borrow() {
            Ok(borrow) => f
                .debug_struct("AtomicRefCell")
                .field("value", &borrow)
                .finish(),
            Err(_) => {
                // The AtomicRefCell is mutably borrowed so we can't look at its value
                // here. Show a placeholder instead.
                struct BorrowedPlaceholder;

                impl std::fmt::Debug for BorrowedPlaceholder {
                    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        f.write_str("<borrowed>")
                    }
                }

                f.debug_struct("AtomicRefCell")
                    .field("value", &BorrowedPlaceholder)
                    .finish()
            }
        }
    }
}

impl<T> const From<T> for AtomicRefCell<T> {
    fn from(t: T) -> AtomicRefCell<T> {
        AtomicRefCell::new(t)
    }
}

#[repr(transparent)]
struct BorrowRef<'b> {
    borrow: &'b Cell<BorrowFlag>,
}

impl<'b> BorrowRef<'b> {
    #[inline]
    fn new(borrow: &'b Cell<BorrowFlag>) -> Option<BorrowRef<'b>> {
        let b = borrow.get().wrapping_add(1);
        if !is_reading(b) {
            // Incrementing borrow can result in a non-reading value (<= 0) in these cases:
            // 1. It was < 0, i.e. there are writing borrows, so we can't allow a read borrow
            //    due to Rust's reference aliasing rules
            // 2. It was isize::MAX (the max amount of reading borrows) and it overflowed
            //    into isize::MIN (the max amount of writing borrows) so we can't allow
            //    an additional read borrow because isize can't represent so many read borrows
            //    (this can only happen if you mem::forget more than a small constant amount of
            //    `Ref`s, which is not good practice)
            None
        } else {
            // Incrementing borrow can result in a reading value (> 0) in these cases:
            // 1. It was = 0, i.e. it wasn't borrowed, and we are taking the first read borrow
            // 2. It was > 0 and < isize::MAX, i.e. there were read borrows, and isize
            //    is large enough to represent having one more read borrow
            borrow.set(b);
            Some(BorrowRef { borrow })
        }
    }
}

impl Drop for BorrowRef<'_> {
    #[inline]
    fn drop(&mut self) {
        let borrow = self.borrow.get();
        debug_assert!(is_reading(borrow));
        self.borrow.set(borrow - 1);
    }
}

impl Clone for BorrowRef<'_> {
    #[inline]
    fn clone(&self) -> Self {
        // Since this Ref exists, we know the borrow flag
        // is a reading borrow.
        let borrow = self.borrow.get();
        debug_assert!(is_reading(borrow));
        // Prevent the borrow counter from overflowing into
        // a writing borrow.
        assert!(borrow != isize::MAX);
        self.borrow.set(borrow + 1);
        BorrowRef {
            borrow: self.borrow,
        }
    }
}

#[repr(transparent)]
struct AtomicBorrowRef<'b> {
    borrow: &'b AtomicBorrowFlag,
}

impl<'b> AtomicBorrowRef<'b> {
    #[inline]
    fn new(borrow: &'b AtomicBorrowFlag) -> Option<AtomicBorrowRef<'b>> {
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
        let b = borrow.fetch_add(1, std::sync::atomic::Ordering::Relaxed) + 1;
        if !atomic_is_reading(b) {
            // Incrementing borrow can result in a non-reading value (>= WRITING_ATOMIC) in these cases:
            // 1. It was >= WRITING_ATOMIC, i.e. there are writing borrows, so we can't allow a read borrow
            //    due to Rust's reference aliasing rules
            // 2. It was >= MAX_ATOMIC_COUNT (the max amount of reading borrows) and it overflowed
            //    into the upper region of the usize (the max amount of writing borrows) so we can't allow
            //    an additional read borrow because usize can't represent so many read borrows
            //    (this can only happen if you mem::forget more than a small constant amount of
            //    `AtomicRef`s, which is not good practice or use an absurd number of threads)

            if b > MAX_ATOMIC_COUNT {
                abort();
            }

            borrow.fetch_sub(1, std::sync::atomic::Ordering::Release);

            None
        } else {
            // Incrementing borrow can result in a reading value in these cases:
            // 1. It was = 0, i.e. it wasn't borrowed, and we are taking the first read borrow
            // 2. It was > 0 and < WRITING_ATOMIC, i.e. there were read borrows, and the counter
            //    is large enough to represent having one more read borrow
            Some(AtomicBorrowRef { borrow })
        }
    }
}

impl Drop for AtomicBorrowRef<'_> {
    #[inline]
    fn drop(&mut self) {
        let borrow = self
            .borrow
            .fetch_sub(1, std::sync::atomic::Ordering::Release);
        debug_assert!(atomic_is_reading(borrow));
    }
}

impl Clone for AtomicBorrowRef<'_> {
    #[inline]
    fn clone(&self) -> Self {
        Self::new(self.borrow).unwrap()
    }
}

/// Wraps a borrowed reference to a value in a `RefCell` box.
/// A wrapper type for an immutably borrowed value from a `RefCell<T>`.
#[must_not_suspend = "holding a Ref across suspend points can cause BorrowErrors"]
pub struct Ref<'b, T: ?Sized> {
    value: &'b T,
    borrow: BorrowRef<'b>,
}

impl<'b, T: ?Sized> Ref<'b, T> {
    /// Copies a `Ref`.
    ///
    /// The `RefCell` is already immutably borrowed, so this cannot fail.
    ///
    /// This is an associated function that needs to be used as
    /// `Ref::clone(...)`. A `Clone` implementation or a method would interfere
    /// with the widespread use of `r.borrow().clone()` to clone the contents of
    /// a `RefCell`.
    #[inline]
    #[must_use]
    #[allow(clippy::should_implement_trait)]
    pub fn clone(orig: &Ref<'b, T>) -> Ref<'b, T> {
        Ref {
            value: orig.value,
            borrow: orig.borrow.clone(),
        }
    }

    /// Makes a new `Ref` for a component of the borrowed data.
    ///
    /// The `RefCell` is already immutably borrowed, so this cannot fail.
    ///
    /// This is an associated function that needs to be used as `Ref::map(...)`.
    /// A method would interfere with methods of the same name on the contents
    /// of a `RefCell` used through `Deref`.
    ///
    /// # Examples
    ///
    /// ```
    /// use fimo_ffi::cell::{RefCell, Ref};
    ///
    /// let c = RefCell::new((5, 'b'));
    /// let b1: Ref<(u32, char)> = c.borrow();
    /// let b2: Ref<u32> = Ref::map(b1, |t| &t.0);
    /// assert_eq!(*b2, 5)
    /// ```
    #[inline]
    pub fn map<U: ?Sized, F>(orig: Ref<'b, T>, f: F) -> Ref<'b, U>
    where
        F: FnOnce(&T) -> &U,
    {
        Ref {
            value: f(orig.value),
            borrow: orig.borrow,
        }
    }

    /// Makes a new `Ref` for an optional component of the borrowed data. The
    /// original guard is returned as an `Err(..)` if the closure returns
    /// `None`.
    ///
    /// The `RefCell` is already immutably borrowed, so this cannot fail.
    ///
    /// This is an associated function that needs to be used as
    /// `Ref::filter_map(...)`. A method would interfere with methods of the same
    /// name on the contents of a `RefCell` used through `Deref`.
    ///
    /// # Examples
    ///
    /// ```
    /// use fimo_ffi::cell::{RefCell, Ref};
    ///
    /// let c = RefCell::new(vec![1, 2, 3]);
    /// let b1: Ref<Vec<u32>> = c.borrow();
    /// let b2: Result<Ref<u32>, _> = Ref::filter_map(b1, |v| v.get(1));
    /// assert_eq!(*b2.unwrap(), 2);
    /// ```
    #[inline]
    pub fn filter_map<U: ?Sized, F>(orig: Ref<'b, T>, f: F) -> Result<Ref<'b, U>, Self>
    where
        F: FnOnce(&T) -> Option<&U>,
    {
        match f(orig.value) {
            Some(value) => Ok(Ref {
                value,
                borrow: orig.borrow,
            }),
            None => Err(orig),
        }
    }

    /// Splits a `Ref` into multiple `Ref`s for different components of the
    /// borrowed data.
    ///
    /// The `RefCell` is already immutably borrowed, so this cannot fail.
    ///
    /// This is an associated function that needs to be used as
    /// `Ref::map_split(...)`. A method would interfere with methods of the same
    /// name on the contents of a `RefCell` used through `Deref`.
    ///
    /// # Examples
    ///
    /// ```
    /// use fimo_ffi::cell::{Ref, RefCell};
    ///
    /// let cell = RefCell::new([1, 2, 3, 4]);
    /// let borrow = cell.borrow();
    /// let (begin, end) = Ref::map_split(borrow, |slice| slice.split_at(2));
    /// assert_eq!(*begin, [1, 2]);
    /// assert_eq!(*end, [3, 4]);
    /// ```
    #[inline]
    pub fn map_split<U: ?Sized, V: ?Sized, F>(orig: Ref<'b, T>, f: F) -> (Ref<'b, U>, Ref<'b, V>)
    where
        F: FnOnce(&T) -> (&U, &V),
    {
        let (a, b) = f(orig.value);
        let borrow = orig.borrow.clone();
        (
            Ref { value: a, borrow },
            Ref {
                value: b,
                borrow: orig.borrow,
            },
        )
    }

    /// Convert into a reference to the underlying data.
    ///
    /// The underlying `RefCell` can never be mutably borrowed from again and will always appear
    /// already immutably borrowed. It is not a good idea to leak more than a constant number of
    /// references. The `RefCell` can be immutably borrowed again if only a smaller number of leaks
    /// have occurred in total.
    ///
    /// This is an associated function that needs to be used as
    /// `Ref::leak(...)`. A method would interfere with methods of the
    /// same name on the contents of a `RefCell` used through `Deref`.
    ///
    /// # Examples
    ///
    /// ```
    /// use fimo_ffi::cell::{RefCell, Ref};
    /// let cell = RefCell::new(0);
    ///
    /// let value = Ref::leak(cell.borrow());
    /// assert_eq!(*value, 0);
    ///
    /// assert!(cell.try_borrow().is_ok());
    /// assert!(cell.try_borrow_mut().is_err());
    /// ```
    pub fn leak(orig: Ref<'b, T>) -> &'b T {
        // By forgetting this Ref we ensure that the borrow counter in the RefCell can't go back to
        // UNUSED within the lifetime `'b`. Resetting the reference tracking state would require a
        // unique reference to the borrowed RefCell. No further mutable references can be created
        // from the original cell.
        std::mem::forget(orig.borrow);
        orig.value
    }
}

impl<T: ?Sized> Deref for Ref<'_, T> {
    type Target = T;

    #[inline]
    fn deref(&self) -> &T {
        self.value
    }
}

impl<T: ?Sized + std::fmt::Debug> std::fmt::Debug for Ref<'_, T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Debug::fmt(&**self, f)
    }
}

impl<T: ?Sized + std::fmt::Display> std::fmt::Display for Ref<'_, T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.value.fmt(f)
    }
}

/// Wraps a borrowed reference to a value in a `AtomicRefCell` box.
/// A wrapper type for an immutably borrowed value from a `AtomicRefCell<T>`.
#[must_not_suspend = "holding a Ref across suspend points can cause BorrowErrors"]
pub struct AtomicRef<'b, T: ?Sized> {
    value: &'b T,
    borrow: AtomicBorrowRef<'b>,
}

impl<'b, T: ?Sized> AtomicRef<'b, T> {
    /// Copies a `Ref`.
    ///
    /// The `AtomicRefCell` is already immutably borrowed, so this cannot fail.
    ///
    /// This is an associated function that needs to be used as
    /// `AtomicRef::clone(...)`. A `Clone` implementation or a method would interfere
    /// with the widespread use of `r.borrow().clone()` to clone the contents of
    /// a `RefCell`.
    #[inline]
    #[must_use]
    #[allow(clippy::should_implement_trait)]
    pub fn clone(orig: &AtomicRef<'b, T>) -> AtomicRef<'b, T> {
        AtomicRef {
            value: orig.value,
            borrow: orig.borrow.clone(),
        }
    }

    /// Makes a new `AtomicRef` for a component of the borrowed data.
    ///
    /// The `AtomicRefCell` is already immutably borrowed, so this cannot fail.
    ///
    /// This is an associated function that needs to be used as `AtomicRef::map(...)`.
    /// A method would interfere with methods of the same name on the contents
    /// of a `AtomicRefCell` used through `Deref`.
    ///
    /// # Examples
    ///
    /// ```
    /// use fimo_ffi::cell::{AtomicRefCell, AtomicRef};
    ///
    /// let c = AtomicRefCell::new((5, 'b'));
    /// let b1: AtomicRef<(u32, char)> = c.borrow();
    /// let b2: AtomicRef<u32> = AtomicRef::map(b1, |t| &t.0);
    /// assert_eq!(*b2, 5)
    /// ```
    #[inline]
    pub fn map<U: ?Sized, F>(orig: AtomicRef<'b, T>, f: F) -> AtomicRef<'b, U>
    where
        F: FnOnce(&T) -> &U,
    {
        AtomicRef {
            value: f(orig.value),
            borrow: orig.borrow,
        }
    }

    /// Makes a new `AtomicRef` for an optional component of the borrowed data. The
    /// original guard is returned as an `Err(..)` if the closure returns
    /// `None`.
    ///
    /// The `AtomicRefCell` is already immutably borrowed, so this cannot fail.
    ///
    /// This is an associated function that needs to be used as
    /// `AtomicRef::filter_map(...)`. A method would interfere with methods of the same
    /// name on the contents of a `AtomicRefCell` used through `Deref`.
    ///
    /// # Examples
    ///
    /// ```
    /// use fimo_ffi::cell::{AtomicRefCell, AtomicRef};
    ///
    /// let c = AtomicRefCell::new(vec![1, 2, 3]);
    /// let b1: AtomicRef<Vec<u32>> = c.borrow();
    /// let b2: Result<AtomicRef<u32>, _> = AtomicRef::filter_map(b1, |v| v.get(1));
    /// assert_eq!(*b2.unwrap(), 2);
    /// ```
    #[inline]
    pub fn filter_map<U: ?Sized, F>(orig: AtomicRef<'b, T>, f: F) -> Result<AtomicRef<'b, U>, Self>
    where
        F: FnOnce(&T) -> Option<&U>,
    {
        match f(orig.value) {
            Some(value) => Ok(AtomicRef {
                value,
                borrow: orig.borrow,
            }),
            None => Err(orig),
        }
    }

    /// Splits a `AtomicRef` into multiple `AtomicRef`s for different components of the
    /// borrowed data.
    ///
    /// The `AtomicRefCell` is already immutably borrowed, so this cannot fail.
    ///
    /// This is an associated function that needs to be used as
    /// `AtomicRef::map_split(...)`. A method would interfere with methods of the same
    /// name on the contents of a `AtomicRefCell` used through `Deref`.
    ///
    /// # Examples
    ///
    /// ```
    /// use fimo_ffi::cell::{AtomicRef, AtomicRefCell};
    ///
    /// let cell = AtomicRefCell::new([1, 2, 3, 4]);
    /// let borrow = cell.borrow();
    /// let (begin, end) = AtomicRef::map_split(borrow, |slice| slice.split_at(2));
    /// assert_eq!(*begin, [1, 2]);
    /// assert_eq!(*end, [3, 4]);
    /// ```
    #[inline]
    pub fn map_split<U: ?Sized, V: ?Sized, F>(
        orig: AtomicRef<'b, T>,
        f: F,
    ) -> (AtomicRef<'b, U>, AtomicRef<'b, V>)
    where
        F: FnOnce(&T) -> (&U, &V),
    {
        let (a, b) = f(orig.value);
        let borrow = orig.borrow.clone();
        (
            AtomicRef { value: a, borrow },
            AtomicRef {
                value: b,
                borrow: orig.borrow,
            },
        )
    }

    /// Convert into a reference to the underlying data.
    ///
    /// The underlying `AtomicRefCell` can never be mutably borrowed from again and will always appear
    /// already immutably borrowed. It is not a good idea to leak more than a constant number of
    /// references. The `AtomicRefCell` can be immutably borrowed again if only a smaller number of leaks
    /// have occurred in total.
    ///
    /// This is an associated function that needs to be used as
    /// `AtomicRef::leak(...)`. A method would interfere with methods of the
    /// same name on the contents of a `AtomicRefCell` used through `Deref`.
    ///
    /// # Examples
    ///
    /// ```
    /// use fimo_ffi::cell::{AtomicRefCell, AtomicRef};
    /// let cell = AtomicRefCell::new(0);
    ///
    /// let value = AtomicRef::leak(cell.borrow());
    /// assert_eq!(*value, 0);
    ///
    /// assert!(cell.try_borrow().is_ok());
    /// assert!(cell.try_borrow_mut().is_err());
    /// ```
    pub fn leak(orig: AtomicRef<'b, T>) -> &'b T {
        // By forgetting this AtomicRef we ensure that the borrow counter in the AtomicRefCell can't go back to
        // UNUSED within the lifetime `'b`. Resetting the reference tracking state would require a
        // unique reference to the borrowed AtomicRefCell. No further mutable references can be created
        // from the original cell.
        std::mem::forget(orig.borrow);
        orig.value
    }
}

impl<T: ?Sized> Deref for AtomicRef<'_, T> {
    type Target = T;

    #[inline]
    fn deref(&self) -> &T {
        self.value
    }
}

impl<T: ?Sized + std::fmt::Debug> std::fmt::Debug for AtomicRef<'_, T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Debug::fmt(&**self, f)
    }
}

impl<T: ?Sized + std::fmt::Display> std::fmt::Display for AtomicRef<'_, T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.value.fmt(f)
    }
}

#[repr(transparent)]
struct BorrowRefMut<'b> {
    borrow: &'b Cell<BorrowFlag>,
}

impl<'b> BorrowRefMut<'b> {
    #[inline]
    fn new(borrow: &'b Cell<BorrowFlag>) -> Option<BorrowRefMut<'b>> {
        // NOTE: Unlike BorrowRefMut::clone, new is called to create the initial
        // mutable reference, and so there must currently be no existing
        // references. Thus, while clone increments the mutable refcount, here
        // we explicitly only allow going from UNUSED to UNUSED - 1.
        match borrow.get() {
            UNUSED => {
                borrow.set(UNUSED - 1);
                Some(BorrowRefMut { borrow })
            }
            _ => None,
        }
    }

    // Clones a `BorrowRefMut`.
    //
    // This is only valid if each `BorrowRefMut` is used to track a mutable
    // reference to a distinct, nonoverlapping range of the original object.
    // This isn't in a Clone impl so that code doesn't call this implicitly.
    #[inline]
    fn clone(&self) -> BorrowRefMut<'b> {
        let borrow = self.borrow.get();
        debug_assert!(is_writing(borrow));
        // Prevent the borrow counter from underflowing.
        assert!(borrow != isize::MIN);
        self.borrow.set(borrow - 1);
        BorrowRefMut {
            borrow: self.borrow,
        }
    }
}

impl Drop for BorrowRefMut<'_> {
    #[inline]
    fn drop(&mut self) {
        let borrow = self.borrow.get();
        debug_assert!(is_writing(borrow));
        self.borrow.set(borrow + 1);
    }
}

#[repr(transparent)]
struct AtomicBorrowRefMut<'b> {
    borrow: &'b AtomicBorrowFlag,
}

impl<'b> AtomicBorrowRefMut<'b> {
    #[inline]
    fn new(borrow: &'b AtomicBorrowFlag) -> Option<AtomicBorrowRefMut<'b>> {
        // Check that there are no other borrows and mark the atomicRefCell as
        // mutably borrowed. This store is synchronized with `AtomicBorrowRefMut::drop`
        // which uses release.
        if borrow
            .compare_exchange(
                UNUSED_ATOMIC,
                WRITING_ATOMIC,
                std::sync::atomic::Ordering::Acquire,
                std::sync::atomic::Ordering::Relaxed,
            )
            .is_ok()
        {
            Some(AtomicBorrowRefMut { borrow })
        } else {
            None
        }
    }

    // Clones a `BorrowRefMut`.
    //
    // This is only valid if each `BorrowRefMut` is used to track a mutable
    // reference to a distinct, nonoverlapping range of the original object.
    // This isn't in a Clone impl so that code doesn't call this implicitly.
    #[inline]
    fn clone(&self) -> AtomicBorrowRefMut<'b> {
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
        let b = self
            .borrow
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed)
            + 1;
        debug_assert!(atomic_is_writing(b));

        // Prevent the borrow counter from overflowing.
        if b > MAX_ATOMIC_COUNT {
            abort();
        }

        AtomicBorrowRefMut {
            borrow: self.borrow,
        }
    }
}

impl Drop for AtomicBorrowRefMut<'_> {
    #[inline]
    fn drop(&mut self) {
        self.borrow.store(0, std::sync::atomic::Ordering::Release);
    }
}

/// A wrapper type for a mutably borrowed value from a `RefCell<T>`.
#[must_not_suspend = "holding a RefMut across suspend points can cause BorrowErrors"]
pub struct RefMut<'b, T: ?Sized> {
    value: &'b mut T,
    borrow: BorrowRefMut<'b>,
}

impl<'b, T: ?Sized> RefMut<'b, T> {
    /// Makes a new `RefMut` for a component of the borrowed data, e.g., an enum
    /// variant.
    ///
    /// The `RefCell` is already mutably borrowed, so this cannot fail.
    ///
    /// This is an associated function that needs to be used as
    /// `RefMut::map(...)`. A method would interfere with methods of the same
    /// name on the contents of a `RefCell` used through `Deref`.
    ///
    /// # Examples
    ///
    /// ```
    /// use fimo_ffi::cell::{RefCell, RefMut};
    ///
    /// let c = RefCell::new((5, 'b'));
    /// {
    ///     let b1: RefMut<(u32, char)> = c.borrow_mut();
    ///     let mut b2: RefMut<u32> = RefMut::map(b1, |t| &mut t.0);
    ///     assert_eq!(*b2, 5);
    ///     *b2 = 42;
    /// }
    /// assert_eq!(*c.borrow(), (42, 'b'));
    /// ```
    #[inline]
    pub fn map<U: ?Sized, F>(orig: RefMut<'b, T>, f: F) -> RefMut<'b, U>
    where
        F: FnOnce(&mut T) -> &mut U,
    {
        // FIXME(nll-rfc#40): fix borrow-check
        let RefMut { value, borrow } = orig;
        RefMut {
            value: f(value),
            borrow,
        }
    }

    /// Makes a new `RefMut` for an optional component of the borrowed data. The
    /// original guard is returned as an `Err(..)` if the closure returns
    /// `None`.
    ///
    /// The `RefCell` is already mutably borrowed, so this cannot fail.
    ///
    /// This is an associated function that needs to be used as
    /// `RefMut::filter_map(...)`. A method would interfere with methods of the
    /// same name on the contents of a `RefCell` used through `Deref`.
    ///
    /// # Examples
    ///
    /// ```
    /// use fimo_ffi::cell::{RefCell, RefMut};
    ///
    /// let c = RefCell::new(vec![1, 2, 3]);
    ///
    /// {
    ///     let b1: RefMut<Vec<u32>> = c.borrow_mut();
    ///     let mut b2: Result<RefMut<u32>, _> = RefMut::filter_map(b1, |v| v.get_mut(1));
    ///
    ///     if let Ok(mut b2) = b2 {
    ///         *b2 += 2;
    ///     }
    /// }
    ///
    /// assert_eq!(*c.borrow(), vec![1, 4, 3]);
    /// ```
    #[inline]
    pub fn filter_map<U: ?Sized, F>(orig: RefMut<'b, T>, f: F) -> Result<RefMut<'b, U>, Self>
    where
        F: FnOnce(&mut T) -> Option<&mut U>,
    {
        // FIXME(nll-rfc#40): fix borrow-check
        let RefMut { value, borrow } = orig;
        let value = value as *mut T;
        // SAFETY: function holds onto an exclusive reference for the duration
        // of its call through `orig`, and the pointer is only de-referenced
        // inside of the function call never allowing the exclusive reference to
        // escape.
        match f(unsafe { &mut *value }) {
            Some(value) => Ok(RefMut { value, borrow }),
            None => {
                // SAFETY: same as above.
                Err(RefMut {
                    value: unsafe { &mut *value },
                    borrow,
                })
            }
        }
    }

    /// Splits a `RefMut` into multiple `RefMut`s for different components of the
    /// borrowed data.
    ///
    /// The underlying `RefCell` will remain mutably borrowed until both
    /// returned `RefMut`s go out of scope.
    ///
    /// The `RefCell` is already mutably borrowed, so this cannot fail.
    ///
    /// This is an associated function that needs to be used as
    /// `RefMut::map_split(...)`. A method would interfere with methods of the
    /// same name on the contents of a `RefCell` used through `Deref`.
    ///
    /// # Examples
    ///
    /// ```
    /// use fimo_ffi::cell::{RefCell, RefMut};
    ///
    /// let cell = RefCell::new([1, 2, 3, 4]);
    /// let borrow = cell.borrow_mut();
    /// let (mut begin, mut end) = RefMut::map_split(borrow, |slice| slice.split_at_mut(2));
    /// assert_eq!(*begin, [1, 2]);
    /// assert_eq!(*end, [3, 4]);
    /// begin.copy_from_slice(&[4, 3]);
    /// end.copy_from_slice(&[2, 1]);
    /// ```
    #[inline]
    pub fn map_split<U: ?Sized, V: ?Sized, F>(
        orig: RefMut<'b, T>,
        f: F,
    ) -> (RefMut<'b, U>, RefMut<'b, V>)
    where
        F: FnOnce(&mut T) -> (&mut U, &mut V),
    {
        let (a, b) = f(orig.value);
        let borrow = orig.borrow.clone();
        (
            RefMut { value: a, borrow },
            RefMut {
                value: b,
                borrow: orig.borrow,
            },
        )
    }

    /// Convert into a mutable reference to the underlying data.
    ///
    /// The underlying `RefCell` can not be borrowed from again and will always appear already
    /// mutably borrowed, making the returned reference the only to the interior.
    ///
    /// This is an associated function that needs to be used as
    /// `RefMut::leak(...)`. A method would interfere with methods of the
    /// same name on the contents of a `RefCell` used through `Deref`.
    ///
    /// # Examples
    ///
    /// ```
    /// use fimo_ffi::cell::{RefCell, RefMut};
    /// let cell = RefCell::new(0);
    ///
    /// let value = RefMut::leak(cell.borrow_mut());
    /// assert_eq!(*value, 0);
    /// *value = 1;
    ///
    /// assert!(cell.try_borrow_mut().is_err());
    /// ```
    pub fn leak(orig: RefMut<'b, T>) -> &'b mut T {
        // By forgetting this BorrowRefMut we ensure that the borrow counter in the RefCell can't
        // go back to UNUSED within the lifetime `'b`. Resetting the reference tracking state would
        // require a unique reference to the borrowed RefCell. No further references can be created
        // from the original cell within that lifetime, making the current borrow the only
        // reference for the remaining lifetime.
        std::mem::forget(orig.borrow);
        orig.value
    }
}

impl<T: ?Sized> Deref for RefMut<'_, T> {
    type Target = T;

    #[inline]
    fn deref(&self) -> &T {
        self.value
    }
}

impl<T: ?Sized> DerefMut for RefMut<'_, T> {
    #[inline]
    fn deref_mut(&mut self) -> &mut T {
        self.value
    }
}

impl<T: ?Sized + std::fmt::Debug> std::fmt::Debug for RefMut<'_, T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Debug::fmt(&**self, f)
    }
}

impl<T: ?Sized + std::fmt::Display> std::fmt::Display for RefMut<'_, T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.value.fmt(f)
    }
}

/// A wrapper type for a mutably borrowed value from a `RefCell<T>`.
#[must_not_suspend = "holding a RefMut across suspend points can cause BorrowErrors"]
pub struct AtomicRefMut<'b, T: ?Sized> {
    value: &'b mut T,
    borrow: AtomicBorrowRefMut<'b>,
}

impl<'b, T: ?Sized> AtomicRefMut<'b, T> {
    /// Makes a new `AtomicRefMut` for a component of the borrowed data, e.g., an enum
    /// variant.
    ///
    /// The `AtomicRefCell` is already mutably borrowed, so this cannot fail.
    ///
    /// This is an associated function that needs to be used as
    /// `AtomicRefMut::map(...)`. A method would interfere with methods of the same
    /// name on the contents of a `AtomicRefCell` used through `Deref`.
    ///
    /// # Examples
    ///
    /// ```
    /// use fimo_ffi::cell::{AtomicRefCell, AtomicRefMut};
    ///
    /// let c = AtomicRefCell::new((5, 'b'));
    /// {
    ///     let b1: AtomicRefMut<(u32, char)> = c.borrow_mut();
    ///     let mut b2: AtomicRefMut<u32> = AtomicRefMut::map(b1, |t| &mut t.0);
    ///     assert_eq!(*b2, 5);
    ///     *b2 = 42;
    /// }
    /// assert_eq!(*c.borrow(), (42, 'b'));
    /// ```
    #[inline]
    pub fn map<U: ?Sized, F>(orig: AtomicRefMut<'b, T>, f: F) -> AtomicRefMut<'b, U>
    where
        F: FnOnce(&mut T) -> &mut U,
    {
        // FIXME(nll-rfc#40): fix borrow-check
        let AtomicRefMut { value, borrow } = orig;
        AtomicRefMut {
            value: f(value),
            borrow,
        }
    }

    /// Makes a new `AtomicRefMut` for an optional component of the borrowed data. The
    /// original guard is returned as an `Err(..)` if the closure returns
    /// `None`.
    ///
    /// The `AtomicRefCell` is already mutably borrowed, so this cannot fail.
    ///
    /// This is an associated function that needs to be used as
    /// `AtomicRefMut::filter_map(...)`. A method would interfere with methods of the
    /// same name on the contents of a `AtomicRefCell` used through `Deref`.
    ///
    /// # Examples
    ///
    /// ```
    /// use fimo_ffi::cell::{AtomicRefCell, AtomicRefMut};
    ///
    /// let c = AtomicRefCell::new(vec![1, 2, 3]);
    ///
    /// {
    ///     let b1: AtomicRefMut<Vec<u32>> = c.borrow_mut();
    ///     let mut b2: Result<AtomicRefMut<u32>, _> = AtomicRefMut::filter_map(b1, |v| v.get_mut(1));
    ///
    ///     if let Ok(mut b2) = b2 {
    ///         *b2 += 2;
    ///     }
    /// }
    ///
    /// assert_eq!(*c.borrow(), vec![1, 4, 3]);
    /// ```
    #[inline]
    pub fn filter_map<U: ?Sized, F>(
        orig: AtomicRefMut<'b, T>,
        f: F,
    ) -> Result<AtomicRefMut<'b, U>, Self>
    where
        F: FnOnce(&mut T) -> Option<&mut U>,
    {
        // FIXME(nll-rfc#40): fix borrow-check
        let AtomicRefMut { value, borrow } = orig;
        let value = value as *mut T;
        // SAFETY: function holds onto an exclusive reference for the duration
        // of its call through `orig`, and the pointer is only de-referenced
        // inside of the function call never allowing the exclusive reference to
        // escape.
        match f(unsafe { &mut *value }) {
            Some(value) => Ok(AtomicRefMut { value, borrow }),
            None => {
                // SAFETY: same as above.
                Err(AtomicRefMut {
                    value: unsafe { &mut *value },
                    borrow,
                })
            }
        }
    }

    /// Splits a `AtomicRefMut` into multiple `AtomicRefMut`s for different components of the
    /// borrowed data.
    ///
    /// The underlying `AtomicRefCell` will remain mutably borrowed until both
    /// returned `AtomicRefMut`s go out of scope.
    ///
    /// The `AtomicRefCell` is already mutably borrowed, so this cannot fail.
    ///
    /// This is an associated function that needs to be used as
    /// `AtomicRefMut::map_split(...)`. A method would interfere with methods of the
    /// same name on the contents of a `AtomicRefCell` used through `Deref`.
    ///
    /// # Examples
    ///
    /// ```
    /// use fimo_ffi::cell::{AtomicRefCell, AtomicRefMut};
    ///
    /// let cell = AtomicRefCell::new([1, 2, 3, 4]);
    /// let borrow = cell.borrow_mut();
    /// let (mut begin, mut end) = AtomicRefMut::map_split(borrow, |slice| slice.split_at_mut(2));
    /// assert_eq!(*begin, [1, 2]);
    /// assert_eq!(*end, [3, 4]);
    /// begin.copy_from_slice(&[4, 3]);
    /// end.copy_from_slice(&[2, 1]);
    /// ```
    #[inline]
    pub fn map_split<U: ?Sized, V: ?Sized, F>(
        orig: AtomicRefMut<'b, T>,
        f: F,
    ) -> (AtomicRefMut<'b, U>, AtomicRefMut<'b, V>)
    where
        F: FnOnce(&mut T) -> (&mut U, &mut V),
    {
        let (a, b) = f(orig.value);
        let borrow = orig.borrow.clone();
        (
            AtomicRefMut { value: a, borrow },
            AtomicRefMut {
                value: b,
                borrow: orig.borrow,
            },
        )
    }

    /// Convert into a mutable reference to the underlying data.
    ///
    /// The underlying `AtomicRefCell` can not be borrowed from again and will always appear already
    /// mutably borrowed, making the returned reference the only to the interior.
    ///
    /// This is an associated function that needs to be used as
    /// `AtomicRefMut::leak(...)`. A method would interfere with methods of the
    /// same name on the contents of a `AtomicRefCell` used through `Deref`.
    ///
    /// # Examples
    ///
    /// ```
    /// use fimo_ffi::cell::{AtomicRefCell, AtomicRefMut};
    /// let cell = AtomicRefCell::new(0);
    ///
    /// let value = AtomicRefMut::leak(cell.borrow_mut());
    /// assert_eq!(*value, 0);
    /// *value = 1;
    ///
    /// assert!(cell.try_borrow_mut().is_err());
    /// ```
    pub fn leak(orig: AtomicRefMut<'b, T>) -> &'b mut T {
        // By forgetting this BorrowRefMut we ensure that the borrow counter in the RefCell can't
        // go back to UNUSED within the lifetime `'b`. Resetting the reference tracking state would
        // require a unique reference to the borrowed RefCell. No further references can be created
        // from the original cell within that lifetime, making the current borrow the only
        // reference for the remaining lifetime.
        std::mem::forget(orig.borrow);
        orig.value
    }
}

impl<T: ?Sized> Deref for AtomicRefMut<'_, T> {
    type Target = T;

    #[inline]
    fn deref(&self) -> &T {
        self.value
    }
}

impl<T: ?Sized> DerefMut for AtomicRefMut<'_, T> {
    #[inline]
    fn deref_mut(&mut self) -> &mut T {
        self.value
    }
}

impl<T: ?Sized + std::fmt::Debug> std::fmt::Debug for AtomicRefMut<'_, T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Debug::fmt(&**self, f)
    }
}

impl<T: ?Sized + std::fmt::Display> std::fmt::Display for AtomicRefMut<'_, T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.value.fmt(f)
    }
}
