//! Definition of an object-aware box type.
use crate::object::ObjectWrapper;
use crate::raw::CastError;
use crate::vtable::{BaseInterface, VTable};
use crate::{CoerceObjectMut, Object};
use std::alloc::{handle_alloc_error, Allocator, Global, Layout};
use std::borrow::{Borrow, BorrowMut};
use std::cmp::Ordering;
use std::error::Error;
use std::fmt::{Debug, Display, Formatter, Pointer};
use std::hash::{Hash, Hasher};
use std::marker::{PhantomData, Unsize};
use std::mem::{align_of_val_raw, size_of_val_raw, MaybeUninit};
use std::ops::{CoerceUnsized, Deref, DerefMut};
use std::ptr::NonNull;

/// A pointer type for heap allocation, akin to a [`Box`].
#[repr(C)]
pub struct ObjBox<T: ?Sized, A: Allocator = Global>(Unique<T>, A);

#[allow(missing_debug_implementations)]
struct Unique<T: ?Sized>(NonNull<T>, PhantomData<T>);

impl<T: ?Sized> Unique<T> {
    fn new(ptr: NonNull<T>) -> Self {
        Self(ptr, Default::default())
    }

    fn as_ptr(&self) -> *mut T {
        self.0.as_ptr()
    }

    fn as_nonnull(&self) -> NonNull<T> {
        self.0
    }
}

impl<T: Unsize<U> + ?Sized, U: ?Sized> CoerceUnsized<Unique<U>> for Unique<T> {}

impl<T> ObjBox<T, Global> {
    /// Allocates memory on the heap and then places `x` into it.
    ///
    /// This doesn't actually allocate if `T` is zero-sized. See [`Box::new()`].
    pub fn new(x: T) -> ObjBox<T, Global> {
        ObjBox::new_in(x, Global)
    }

    /// Constructs a new box with uninitialized contents.
    ///
    /// This doesn't actually allocate if `T` is zero-sized. See [`Box::new_uninit()`].
    pub fn new_uninit() -> ObjBox<MaybeUninit<T>, Global> {
        ObjBox::new_uninit_in(Global)
    }

    /// Constructs a new box with zeroed contents.
    ///
    /// This doesn't actually allocate if `T` is zero-sized. See [`Box::new_zeroed()`].
    pub fn new_zeroed() -> ObjBox<MaybeUninit<T>, Global> {
        ObjBox::new_zeroed_in(Global)
    }
}

impl<T, A: Allocator> ObjBox<T, A> {
    /// Allocates memory on the heap with the provided allocator and then places `x` into it.
    ///
    /// This doesn't actually allocate if `T` is zero-sized. See [`Box::new_in()`].
    pub fn new_in(x: T, alloc: A) -> ObjBox<T, A> {
        let ptr = ObjBox::new_uninit_in(alloc);
        unsafe {
            (ptr.0.as_ptr() as *mut T).write(x);
            ptr.assume_init()
        }
    }

    /// Constructs a new box with uninitialized contents using the provided allocator.
    ///
    /// This doesn't actually allocate if `T` is zero-sized. See [`Box::new_uninit_in()`].
    pub fn new_uninit_in(alloc: A) -> ObjBox<MaybeUninit<T>, A> {
        let layout = Layout::new::<MaybeUninit<T>>();
        if layout.size() == 0 {
            return unsafe { ObjBox::from_raw_parts(NonNull::dangling().as_ptr(), alloc) };
        }

        unsafe {
            match alloc.allocate(layout) {
                Ok(ptr) => ObjBox::from_raw_parts(ptr.cast().as_ptr(), alloc),
                Err(_) => handle_alloc_error(layout),
            }
        }
    }

    /// Constructs a new box with zeroed contents using the provided allocator.
    ///
    /// This doesn't actually allocate if `T` is zero-sized. See [`Box::new_uninit_in()`].
    pub fn new_zeroed_in(alloc: A) -> ObjBox<MaybeUninit<T>, A> {
        let layout = Layout::new::<MaybeUninit<T>>();
        if layout.size() == 0 {
            return unsafe { ObjBox::from_raw_parts(NonNull::dangling().as_ptr(), alloc) };
        }

        unsafe {
            match alloc.allocate_zeroed(layout) {
                Ok(ptr) => ObjBox::from_raw_parts(ptr.cast().as_ptr(), alloc),
                Err(_) => handle_alloc_error(layout),
            }
        }
    }
}

impl<O: ObjectWrapper, A: Allocator> ObjBox<O, A> {
    /// Coerces a `ObjBox<T, A>` to an `ObjBox<O, A>`.
    pub fn coerce_object<T: CoerceObjectMut<O::VTable>>(b: ObjBox<T, A>) -> ObjBox<O, A> {
        let (ptr, alloc) = ObjBox::into_raw_parts(b);
        let obj = unsafe { T::coerce_obj_mut(&mut *ptr) };
        let ptr = O::from_object_mut(obj);
        unsafe { ObjBox::from_raw_parts(ptr, alloc) }
    }

    /// Tries to revert from an `ObjBox<O, A>` to an `ObjBox<T, A>`.
    pub fn try_object_cast<T: CoerceObjectMut<O::VTable>>(
        b: ObjBox<O, A>,
    ) -> Result<ObjBox<T, A>, CastError<ObjBox<O, A>>> {
        let (ptr, alloc) = ObjBox::into_raw_parts(b);
        let obj = unsafe { &mut *O::as_object_mut(ptr) };

        unsafe {
            match obj.try_cast_obj_mut::<T>() {
                Ok(casted) => Ok(ObjBox::from_raw_parts(casted, alloc)),
                Err(err) => Err(CastError {
                    obj: ObjBox::from_raw_parts(ptr, alloc),
                    required: err.required,
                    available: err.available,
                }),
            }
        }
    }

    /// Tries casting the object to another object.
    pub fn try_cast<U: ObjectWrapper>(
        b: ObjBox<O, A>,
    ) -> Result<ObjBox<U, A>, CastError<ObjBox<O, A>>> {
        let (ptr, alloc) = ObjBox::into_raw_parts(b);
        let obj = unsafe { &mut *O::as_object_mut(ptr) };

        unsafe {
            match obj.try_cast_mut::<U::VTable>() {
                Ok(casted) => Ok(ObjBox::from_raw_parts(U::from_object_mut(casted), alloc)),
                Err(err) => Err(CastError {
                    obj: ObjBox::from_raw_parts(ptr, alloc),
                    required: err.required,
                    available: err.available,
                }),
            }
        }
    }

    /// Casts an `ObjBox<O, A>` to an `ObjBox<Object<BaseInterface>>`.
    pub fn cast_base(b: ObjBox<O, A>) -> ObjBox<Object<BaseInterface>, A> {
        let (ptr, alloc) = ObjBox::into_raw_parts(b);
        let obj = unsafe { &mut *O::as_object_mut(ptr) };
        let obj = obj.cast_base_mut();
        unsafe { ObjBox::from_raw_parts(obj, alloc) }
    }
}

impl<T, A: Allocator> ObjBox<MaybeUninit<T>, A> {
    /// Converts to `ObjBox<T, A>`.
    ///
    /// # Safety
    ///
    /// See [Box::assume_init()].
    pub unsafe fn assume_init(self) -> ObjBox<T, A> {
        let (raw, alloc) = ObjBox::into_raw_parts(self);
        ObjBox::from_raw_parts(raw as *mut T, alloc)
    }
}

impl<T: ?Sized> ObjBox<T, Global> {
    /// Constructs a box from a raw pointer.
    ///
    /// # Safety
    ///
    /// See [Box::from_raw()].
    pub unsafe fn from_raw(raw: *mut T) -> ObjBox<T, Global> {
        ObjBox::from_raw_parts(raw, Global)
    }
}

impl<T: ?Sized, A: Allocator> ObjBox<T, A> {
    /// Constructs a box from a raw pointer and an allocator.
    ///
    /// # Safety
    ///
    /// See [Box::from_raw_in()].
    pub unsafe fn from_raw_parts(raw: *mut T, alloc: A) -> ObjBox<T, A> {
        ObjBox(Unique::new(NonNull::new_unchecked(raw)), alloc)
    }

    /// Consumes the `ObjBox`, returning a wrapped raw pointer.
    ///
    /// See [Box::into_raw()].
    pub fn into_raw(b: ObjBox<T, A>) -> *mut T {
        let ptr = b.0.as_ptr();
        std::mem::forget(b);
        ptr
    }

    /// Consumes the `ObjBox`, returning a wrapped raw pointer and the allocator.
    ///
    /// See [Box::into_raw_with_allocator()].
    pub fn into_raw_parts(b: ObjBox<T, A>) -> (*mut T, A) {
        let ptr = b.0.as_ptr();
        let alloc = unsafe { std::ptr::read(&b.1) };
        std::mem::forget(b);
        (ptr, alloc)
    }

    /// Returns a reference to the underlying allocator.
    pub fn allocator(b: &ObjBox<T, A>) -> &A {
        &b.1
    }

    /// Consumes and leaks the `ObjBox`, returning a mutable reference `&'a mut T`.
    ///
    /// See [`Box::leak`].
    pub fn leak<'a>(b: ObjBox<T, A>) -> &'a mut T
    where
        T: 'a,
    {
        unsafe { &mut *ObjBox::into_raw(b) }
    }
}

unsafe impl<T: ?Sized + Send, A: Allocator + Send> Send for ObjBox<T, A> {}
unsafe impl<T: ?Sized + Sync, A: Allocator + Sync> Sync for ObjBox<T, A> {}

impl<T: ?Sized, A: Allocator> AsRef<T> for ObjBox<T, A> {
    fn as_ref(&self) -> &T {
        &**self
    }
}

impl<T: ?Sized, A: Allocator> AsMut<T> for ObjBox<T, A> {
    fn as_mut(&mut self) -> &mut T {
        &mut **self
    }
}

impl<T: ?Sized, A: Allocator> Borrow<T> for ObjBox<T, A> {
    fn borrow(&self) -> &T {
        &**self
    }
}

impl<T: ?Sized, A: Allocator> BorrowMut<T> for ObjBox<T, A> {
    fn borrow_mut(&mut self) -> &mut T {
        &mut **self
    }
}

impl<T: Clone, A: Allocator + Clone> Clone for ObjBox<T, A> {
    fn clone(&self) -> Self {
        let mut boxed = ObjBox::new_uninit_in(self.1.clone());
        unsafe {
            (**self).write_clone_into_raw(boxed.as_mut_ptr());
            boxed.assume_init()
        }
    }
}

impl<T: Debug + ?Sized, A: Allocator> Debug for ObjBox<T, A> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        Debug::fmt(&**self, f)
    }
}

impl<T: Default> Default for ObjBox<T, Global> {
    fn default() -> Self {
        ObjBox::new(Default::default())
    }
}

impl<T: Display + ?Sized, A: Allocator> Display for ObjBox<T, A> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        Display::fmt(&**self, f)
    }
}

impl<T: ?Sized, A: Allocator> Deref for ObjBox<T, A> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        unsafe { &*self.0.as_ptr() }
    }
}

impl<T: ?Sized, A: Allocator> DerefMut for ObjBox<T, A> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { &mut *self.0.as_ptr() }
    }
}

unsafe impl<#[may_dangle] T: ?Sized, A: Allocator> Drop for ObjBox<T, A> {
    default fn drop(&mut self) {
        let layout = unsafe { T::layout_for_raw(self.0.as_ptr()) };

        // drop the value
        unsafe { PtrDrop::drop_in_place(self.0.as_ptr()) };

        if layout.size() == 0 {
            return;
        }

        unsafe { self.1.deallocate(self.0.as_nonnull().cast(), layout) }
    }
}

impl<T: Error> Error for ObjBox<T, Global> {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Error::source(&**self)
    }

    #[allow(deprecated, deprecated_in_future)]
    fn description(&self) -> &str {
        Error::description(&**self)
    }

    #[allow(deprecated)]
    fn cause(&self) -> Option<&dyn Error> {
        Error::cause(&**self)
    }
}

impl<Args, F: FnOnce<Args>, A: Allocator> FnOnce<Args> for ObjBox<F, A> {
    type Output = <F as FnOnce<Args>>::Output;

    extern "rust-call" fn call_once(self, args: Args) -> Self::Output {
        let (ptr, alloc) = ObjBox::into_raw_parts(self);
        let uninit = unsafe { ObjBox::from_raw_parts(ptr as *mut MaybeUninit<F>, alloc) };

        let f = unsafe { std::ptr::read((&*uninit).as_ptr()) };
        <F as FnOnce<Args>>::call_once(f, args)
    }
}

impl<Args, F: FnMut<Args>, A: Allocator> FnMut<Args> for ObjBox<F, A> {
    extern "rust-call" fn call_mut(&mut self, args: Args) -> Self::Output {
        <F as FnMut<Args>>::call_mut(self, args)
    }
}

impl<Args, F: Fn<Args>, A: Allocator> Fn<Args> for ObjBox<F, A> {
    extern "rust-call" fn call(&self, args: Args) -> Self::Output {
        <F as Fn<Args>>::call(self, args)
    }
}

impl<T> From<T> for ObjBox<T, Global> {
    fn from(v: T) -> Self {
        ObjBox::new(v)
    }
}

impl<T: Hash + ?Sized, A: Allocator> Hash for ObjBox<T, A> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        Hash::hash(&**self, state)
    }
}

impl<T: Hasher + ?Sized, A: Allocator> Hasher for ObjBox<T, A> {
    fn finish(&self) -> u64 {
        (**self).finish()
    }
    fn write(&mut self, bytes: &[u8]) {
        (**self).write(bytes)
    }
    fn write_u8(&mut self, i: u8) {
        (**self).write_u8(i)
    }
    fn write_u16(&mut self, i: u16) {
        (**self).write_u16(i)
    }
    fn write_u32(&mut self, i: u32) {
        (**self).write_u32(i)
    }
    fn write_u64(&mut self, i: u64) {
        (**self).write_u64(i)
    }
    fn write_u128(&mut self, i: u128) {
        (**self).write_u128(i)
    }
    fn write_usize(&mut self, i: usize) {
        (**self).write_usize(i)
    }
    fn write_i8(&mut self, i: i8) {
        (**self).write_i8(i)
    }
    fn write_i16(&mut self, i: i16) {
        (**self).write_i16(i)
    }
    fn write_i32(&mut self, i: i32) {
        (**self).write_i32(i)
    }
    fn write_i64(&mut self, i: i64) {
        (**self).write_i64(i)
    }
    fn write_i128(&mut self, i: i128) {
        (**self).write_i128(i)
    }
    fn write_isize(&mut self, i: isize) {
        (**self).write_isize(i)
    }
}

impl<T: Iterator + ?Sized, A: Allocator> Iterator for ObjBox<T, A> {
    type Item = T::Item;

    fn next(&mut self) -> Option<Self::Item> {
        Iterator::next(&mut **self)
    }
}

impl<T: Ord + ?Sized, A: Allocator> Ord for ObjBox<T, A> {
    fn cmp(&self, other: &Self) -> Ordering {
        Ord::cmp(&**self, &**other)
    }
}

impl<T: PartialEq<T> + ?Sized, A: Allocator> PartialEq<ObjBox<T, A>> for ObjBox<T, A> {
    fn eq(&self, other: &ObjBox<T, A>) -> bool {
        PartialEq::eq(&**self, &**other)
    }
}

impl<T: PartialOrd<T> + ?Sized, A: Allocator> PartialOrd<ObjBox<T, A>> for ObjBox<T, A> {
    fn partial_cmp(&self, other: &ObjBox<T, A>) -> Option<Ordering> {
        PartialOrd::partial_cmp(&**self, &**other)
    }
}

impl<T: ?Sized, A: Allocator> Pointer for ObjBox<T, A> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let ptr: *const T = &**self;
        Pointer::fmt(&ptr, f)
    }
}

impl<T: Unsize<U> + ?Sized, A: Allocator, U: ?Sized> CoerceUnsized<ObjBox<U, A>> for ObjBox<T, A> {}

impl<T: Eq + ?Sized, A: Allocator> Eq for ObjBox<T, A> {}

impl<T: ?Sized, A: Allocator + 'static> Unpin for ObjBox<T, A> {}

pub(crate) trait ConstructLayoutRaw {
    unsafe fn size_of_val_raw(ptr: *const Self) -> usize;
    unsafe fn align_of_val_raw(ptr: *const Self) -> usize;
    unsafe fn layout_for_raw(ptr: *const Self) -> Layout;
}

impl<T: ?Sized> ConstructLayoutRaw for T {
    #[inline]
    default unsafe fn size_of_val_raw(ptr: *const Self) -> usize {
        size_of_val_raw(ptr)
    }

    #[inline]
    default unsafe fn align_of_val_raw(ptr: *const Self) -> usize {
        align_of_val_raw(ptr)
    }

    #[inline]
    default unsafe fn layout_for_raw(ptr: *const Self) -> Layout {
        Layout::from_size_align_unchecked(size_of_val_raw(ptr), align_of_val_raw(ptr))
    }
}

// `Object<T>` and it's wrappers do not work with the current type-system.
// As a workaround we manually retrieve to layout of the object.
impl<T: ObjectWrapper + ?Sized> ConstructLayoutRaw for T {
    #[inline]
    unsafe fn size_of_val_raw(ptr: *const Self) -> usize {
        let obj = T::as_object(ptr);
        crate::object::size_of_val(obj)
    }

    #[inline]
    unsafe fn align_of_val_raw(ptr: *const Self) -> usize {
        let obj = T::as_object(ptr);
        crate::object::align_of_val(obj)
    }

    #[inline]
    unsafe fn layout_for_raw(ptr: *const Self) -> Layout {
        let (_, vtable) = T::into_raw_parts(ptr);
        Layout::from_size_align_unchecked(vtable.size_of(), vtable.align_of())
    }
}

pub(crate) trait PtrDrop {
    unsafe fn drop_in_place(ptr: *mut Self);
}

impl<T: ?Sized> PtrDrop for T {
    #[inline]
    default unsafe fn drop_in_place(ptr: *mut Self) {
        std::ptr::drop_in_place(ptr)
    }
}

// The drop procedure is contained inside the vtable of the object.
impl<T: ObjectWrapper + ?Sized> PtrDrop for T {
    #[inline]
    unsafe fn drop_in_place(ptr: *mut Self) {
        let obj = T::as_object_mut(ptr);
        crate::object::drop_in_place(obj)
    }
}

/// Specialize clones into pre-allocated, uninitialized memory.
/// Used by `ObjBox::clone` and `ObjArc::make_mut`.
pub(crate) trait WriteCloneIntoRaw: Sized {
    unsafe fn write_clone_into_raw(&self, target: *mut Self);
}

impl<T: Clone> WriteCloneIntoRaw for T {
    #[inline]
    default unsafe fn write_clone_into_raw(&self, target: *mut Self) {
        // Having allocated *first* may allow the optimizer to create
        // the cloned value in-place, skipping the local and move.
        target.write(self.clone());
    }
}

impl<T: Copy> WriteCloneIntoRaw for T {
    #[inline]
    unsafe fn write_clone_into_raw(&self, target: *mut Self) {
        // We can always copy in-place, without ever involving a local value.
        target.copy_from_nonoverlapping(self, 1);
    }
}
