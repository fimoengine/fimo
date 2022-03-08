//! Definition of an object-aware box type.
use crate::obj_arc::CGlobal;
use crate::ptr::{
    CastInto, DowncastSafe, DowncastSafeInterface, DynObj, FetchVTable, ObjInterface, ObjectId,
    RawObjMut,
};
use std::alloc::{handle_alloc_error, Allocator, Global, Layout};
use std::borrow::{Borrow, BorrowMut};
use std::cmp::Ordering;
use std::error::Error;
use std::fmt::{Debug, Display, Formatter, Pointer};
use std::hash::{Hash, Hasher};
use std::marker::{PhantomData, Unsize};
use std::mem::{align_of_val_raw, size_of_val_raw, ManuallyDrop, MaybeUninit};
use std::ops::{Deref, DerefMut};
use std::ptr::NonNull;

/// A pointer type for heap allocation, akin to a [`Box`].
#[repr(C)]
pub struct ObjBox<T: ?Sized, A: Allocator = Global>(Unique<T>, A);

#[repr(transparent)]
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

impl<'a, T: ?Sized + 'a, A: Allocator> ObjBox<DynObj<T>, A> {
    /// Coerces a `ObjBox<U, A>` to an `ObjBox<DynObj<T>, A>`.
    #[inline]
    pub fn coerce_obj<U>(b: ObjBox<U, A>) -> Self
    where
        U: FetchVTable<T::Base> + Unsize<T> + 'a,
        T: ObjInterface,
    {
        let (ptr, alloc) = ObjBox::into_raw_parts(b);
        let obj = crate::ptr::coerce_obj_mut_raw(ptr);
        unsafe { ObjBox::from_raw_parts(obj, alloc) }
    }

    /// Returns whether the contained object is of type `U`.
    #[inline]
    pub fn is<U>(b: &Self) -> bool
    where
        U: DowncastSafe + ObjectId + Unsize<T>,
    {
        crate::ptr::is::<U, _>(&**b)
    }

    /// Returns the downcasted box if it is of type `U`.
    #[inline]
    pub fn downcast<U>(b: Self) -> Option<ObjBox<U, A>>
    where
        U: DowncastSafe + ObjectId + Unsize<T>,
    {
        let (ptr, alloc) = ObjBox::into_raw_parts(b);
        if let Some(ptr) = crate::ptr::downcast_mut::<U, _>(ptr) {
            unsafe { Some(ObjBox::from_raw_parts(ptr, alloc)) }
        } else {
            unsafe { ObjBox::from_raw_parts(ptr, alloc) };
            None
        }
    }

    /// Returns a box to the super object.
    #[inline]
    pub fn cast_super<U>(b: Self) -> ObjBox<DynObj<U>, A>
    where
        T: CastInto<U>,
        U: ObjInterface + ?Sized + 'a,
    {
        let (ptr, alloc) = ObjBox::into_raw_parts(b);
        let ptr = crate::ptr::cast_super_mut::<U, _>(ptr);
        unsafe { ObjBox::from_raw_parts(ptr, alloc) }
    }

    /// Returns whether a certain interface is contained.
    #[inline]
    pub fn is_interface<U>(b: &Self) -> bool
    where
        U: DowncastSafeInterface + Unsize<T> + ?Sized + 'a,
    {
        crate::ptr::is_interface::<U, _>(&**b)
    }

    /// Returns a box to the downcasted interface if it is contained.
    #[inline]
    pub fn downcast_interface<U>(b: Self) -> Option<ObjBox<DynObj<U>, A>>
    where
        U: DowncastSafeInterface + Unsize<T> + ?Sized + 'a,
    {
        let (ptr, alloc) = ObjBox::into_raw_parts(b);
        if let Some(ptr) = crate::ptr::downcast_interface_mut(ptr) {
            unsafe { Some(ObjBox::from_raw_parts(ptr, alloc)) }
        } else {
            unsafe { ObjBox::from_raw_parts(ptr, alloc) };
            None
        }
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

        let f = unsafe { std::ptr::read((*uninit).as_ptr()) };
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

impl<T: Eq + ?Sized, A: Allocator> Eq for ObjBox<T, A> {}

impl<T: ?Sized, A: Allocator + 'static> Unpin for ObjBox<T, A> {}

/// FFI-safe wrapper for an `ObjBox<DynObj<T>>`.
#[repr(C)]
pub struct RawObjBox<T, A: Allocator = CGlobal> {
    ptr: T,
    alloc: ManuallyDrop<A>,
}

impl<T: ?Sized, A: Allocator> RawObjBox<RawObjMut<T>, A> {
    /// Consumes the `RawObjBox<T>` and turns it into a raw pointer.
    #[inline]
    pub fn into_raw_parts(self) -> (RawObjMut<T>, A) {
        let ptr = unsafe { std::ptr::read(&self.ptr) };
        let alloc = unsafe { std::ptr::read(&self.alloc) };
        std::mem::forget(self);
        (ptr, ManuallyDrop::into_inner(alloc))
    }

    /// Converts a raw pointer previously created by [`RawObjBox::into_raw_parts`] back into
    /// `RawObjBox<T>` in the provided allocator.
    ///
    /// # Safety
    ///
    /// See [`ObjBox::from_raw_parts`].
    #[inline]
    pub unsafe fn from_raw_parts(ptr: RawObjMut<T>, alloc: A) -> RawObjBox<RawObjMut<T>, A> {
        Self {
            ptr,
            alloc: ManuallyDrop::new(alloc),
        }
    }
}

impl<T: ?Sized, A: Allocator> From<ObjBox<DynObj<T>, A>> for RawObjBox<RawObjMut<T>, A> {
    fn from(v: ObjBox<DynObj<T>, A>) -> Self {
        let (ptr, alloc) = ObjBox::into_raw_parts(v);
        let ptr = crate::ptr::into_raw_mut(ptr);
        unsafe { RawObjBox::from_raw_parts(ptr, alloc) }
    }
}

impl<T: ?Sized> From<ObjBox<DynObj<T>, Global>> for RawObjBox<RawObjMut<T>, CGlobal> {
    fn from(v: ObjBox<DynObj<T>, Global>) -> Self {
        let (ptr, _) = ObjBox::into_raw_parts(v);
        let ptr = crate::ptr::into_raw_mut(ptr);
        unsafe { RawObjBox::from_raw_parts(ptr, CGlobal { _v: 0 }) }
    }
}

impl<T: ?Sized, A: Allocator> From<RawObjBox<RawObjMut<T>, A>> for ObjBox<DynObj<T>, A> {
    fn from(v: RawObjBox<RawObjMut<T>, A>) -> Self {
        let (ptr, alloc) = v.into_raw_parts();
        let ptr = crate::ptr::from_raw_mut(ptr);
        unsafe { ObjBox::from_raw_parts(ptr, alloc) }
    }
}

impl<T: ?Sized> From<RawObjBox<RawObjMut<T>, CGlobal>> for ObjBox<DynObj<T>, Global> {
    fn from(v: RawObjBox<RawObjMut<T>, CGlobal>) -> Self {
        let (ptr, _) = v.into_raw_parts();
        let ptr = crate::ptr::from_raw_mut(ptr);
        unsafe { ObjBox::from_raw_parts(ptr, Global) }
    }
}

impl<T: Debug, A: Allocator> Debug for RawObjBox<T, A> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "(RawObjBox)")
    }
}

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

// `DynObj<T>` and it's wrappers do not work with the current type-system.
// As a workaround we manually retrieve to layout of the object.
impl<T: ?Sized> ConstructLayoutRaw for DynObj<T> {
    #[inline]
    unsafe fn size_of_val_raw(ptr: *const Self) -> usize {
        crate::ptr::size_of_val(ptr)
    }

    #[inline]
    unsafe fn align_of_val_raw(ptr: *const Self) -> usize {
        crate::ptr::align_of_val(ptr)
    }

    #[inline]
    unsafe fn layout_for_raw(ptr: *const Self) -> Layout {
        crate::ptr::layout_of_val(ptr)
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
impl<T: ?Sized> PtrDrop for DynObj<T> {
    #[inline]
    unsafe fn drop_in_place(ptr: *mut Self) {
        crate::ptr::drop_in_place(ptr)
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
