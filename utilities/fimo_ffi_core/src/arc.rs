//! Implementation of the `Arc<T>` and `Weak<T>` types.
use crate::{NonNullConst, Optional, TypeWrapper};
use std::borrow::Borrow;
use std::cmp::Ordering;
use std::fmt::{Debug, Display, Formatter, Pointer};
use std::hash::{Hash, Hasher};
use std::marker::PhantomData;
use std::mem::forget;
use std::ops::Deref;

/// An atomically reference-counted value.
#[repr(C)]
pub struct Arc<T: ?Sized> {
    data: NonNullConst<T>,
    vtable: NonNullConst<ArcVTable<T>>,
    phantom: PhantomData<T>,
}

/// Function pointer to the internal drop function for an `Arc<T>`.
pub type ArcCleanupFn<T> = TypeWrapper<unsafe extern "C-unwind" fn(NonNullConst<T>)>;

/// Function pointer to the internal clone function for an `Arc<T>`.
pub type ArcCloneFn<T> = TypeWrapper<unsafe extern "C-unwind" fn(NonNullConst<T>) -> Arc<T>>;

/// Function pointer to the internal downgrade function for an `Arc<T>`.
pub type ArcDowngradeFn<T> = TypeWrapper<unsafe extern "C-unwind" fn(NonNullConst<T>) -> Weak<T>>;

/// VTable of an `Arc<T>` type.
#[repr(C)]
#[derive(Debug, Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Hash)]
pub struct ArcVTable<T: ?Sized> {
    /// Cleanup function pointer.
    pub cleanup_fn: ArcCleanupFn<T>,
    /// Clone function pointer.
    pub clone_fn: ArcCloneFn<T>,
    /// Downgrade function pointer.
    pub downgrade_fn: ArcDowngradeFn<T>,
}

unsafe impl<T: Sync + Send + ?Sized> Send for Arc<T> {}
unsafe impl<T: Sync + Send + ?Sized> Sync for Arc<T> {}

impl<T: ?Sized> Arc<T> {
    /// Consumes the `Arc<T>` and turns it into a pair of raw pointers.
    ///
    /// The result can be turned back into the `Arc<T>` with [Arc::from_raw]
    /// to avoid a memory leakage.
    pub fn into_raw(self) -> (NonNullConst<T>, NonNullConst<ArcVTable<T>>) {
        let pointers = (self.data, self.vtable);
        std::mem::forget(self);
        pointers
    }

    /// Creates a new `Arc<T>` from a data and vtable pointer.
    ///
    /// # Safety
    ///
    /// The data pointer must originate from a call to [Arc::into_raw]
    /// or from a function in the vtable.
    pub unsafe fn from_raw(data: NonNullConst<T>, vtable: NonNullConst<ArcVTable<T>>) -> Self {
        Self {
            data,
            vtable,
            phantom: PhantomData,
        }
    }

    /// Created a new weak reference from the strong reference.
    pub fn downgrade(&self) -> Weak<T> {
        unsafe { (self.vtable.as_ref().downgrade_fn)(self.data) }
    }
}

impl<T: ?Sized> Drop for Arc<T> {
    fn drop(&mut self) {
        unsafe { (self.vtable.as_ref().cleanup_fn)(self.data) }
    }
}

impl<T: ?Sized> Clone for Arc<T> {
    fn clone(&self) -> Self {
        unsafe { (self.vtable.as_ref().clone_fn)(self.data) }
    }
}

impl<T: ?Sized> AsRef<T> for Arc<T> {
    fn as_ref(&self) -> &T {
        unsafe { self.data.as_ref() }
    }
}

impl<T: ?Sized> Borrow<T> for Arc<T> {
    fn borrow(&self) -> &T {
        self.as_ref()
    }
}

impl<T: Debug + ?Sized> Debug for Arc<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        Debug::fmt(self.as_ref(), f)
    }
}

impl<T: ?Sized> Deref for Arc<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.as_ref()
    }
}

impl<T: Display + ?Sized> Display for Arc<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        Display::fmt(self.as_ref(), f)
    }
}

impl<T: Hash + ?Sized> Hash for Arc<T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        Hash::hash(self.as_ref(), state)
    }
}

impl<T: PartialEq + ?Sized> PartialEq for Arc<T> {
    fn eq(&self, other: &Self) -> bool {
        self.as_ref().eq(other.as_ref())
    }
}

impl<T: PartialEq + Eq + ?Sized> Eq for Arc<T> {}

impl<T: PartialOrd + ?Sized> PartialOrd for Arc<T> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.as_ref().partial_cmp(other.as_ref())
    }
}

impl<T: PartialOrd + Ord + ?Sized> Ord for Arc<T> {
    fn cmp(&self, other: &Self) -> Ordering {
        self.as_ref().cmp(other.as_ref())
    }
}

impl<T: ?Sized> Pointer for Arc<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        Pointer::fmt(unsafe { &self.data.into_mut() }, f)
    }
}

impl<T: ?Sized> From<std::sync::Arc<T>> for Arc<T> {
    fn from(ptr: std::sync::Arc<T>) -> Self {
        Self {
            data: unsafe { NonNullConst::new_unchecked(std::sync::Arc::into_raw(ptr)) },
            vtable: NonNullConst::from(&<std::sync::Arc<T>>::VTABLE),
            phantom: PhantomData {},
        }
    }
}

/// Trait for creating an `Arc<T>` vtable.
pub trait ArcVTableProvider<T: ?Sized> {
    /// VTable of the `Arc<T>`.
    const VTABLE: ArcVTable<T> = ArcVTable {
        cleanup_fn: TypeWrapper(Self::cleanup),
        clone_fn: TypeWrapper(Self::clone),
        downgrade_fn: TypeWrapper(Self::downgrade),
    };

    /// Implementation of the `Drop` trait.
    ///
    /// Drops the value and deallocates the memory.
    ///
    /// # Safety
    ///
    /// This function requires that `ptr` originates from an `Arc<T>` that matches the function.
    unsafe extern "C-unwind" fn cleanup(ptr: NonNullConst<T>);

    /// Implementation of the `Clone` trait.
    ///
    /// Increases the internal reference count.
    ///
    /// # Safety
    ///
    /// This function requires that `ptr` originates from an `Arc<T>` that matches the function.
    unsafe extern "C-unwind" fn clone(ptr: NonNullConst<T>) -> Arc<T>;

    /// Creates a `Weak<T>` for the pointer.
    ///
    /// # Safety
    ///
    /// This function requires that `ptr` originates from an `Arc<T>` that matches the function.
    unsafe extern "C-unwind" fn downgrade(ptr: NonNullConst<T>) -> Weak<T>;
}

impl<T: ?Sized> ArcVTableProvider<T> for std::sync::Arc<T> {
    unsafe extern "C-unwind" fn cleanup(ptr: NonNullConst<T>) {
        drop(std::sync::Arc::from_raw(ptr.as_ptr()))
    }

    unsafe extern "C-unwind" fn clone(ptr: NonNullConst<T>) -> Arc<T> {
        let original = std::sync::Arc::from_raw(ptr.as_ptr());
        let clone = original.clone();
        forget(original);
        clone.into()
    }

    unsafe extern "C-unwind" fn downgrade(ptr: NonNullConst<T>) -> Weak<T> {
        let arc = std::sync::Arc::from_raw(ptr.as_ptr());
        let weak = std::sync::Arc::downgrade(&arc);
        forget(arc);
        weak.into()
    }
}

/// A weak reference to a value.
#[repr(C)]
pub struct Weak<T: ?Sized> {
    data: NonNullConst<T>,
    vtable: NonNullConst<WeakVTable<T>>,
}

/// Function pointer to the internal drop function for an `Weak<T>`.
pub type WeakCleanupFn<T> = TypeWrapper<unsafe extern "C-unwind" fn(NonNullConst<T>)>;

/// Function pointer to the internal clone function for an `Weak<T>`.
pub type WeakCloneFn<T> = TypeWrapper<unsafe extern "C-unwind" fn(NonNullConst<T>) -> Weak<T>>;

/// Function pointer to the internal upgrade function for an `Weak<T>`.
pub type WeakUpgradeFn<T> =
    TypeWrapper<unsafe extern "C-unwind" fn(NonNullConst<T>) -> Optional<Arc<T>>>;

/// VTable of the `Weak<T>` type.
#[repr(C)]
#[derive(Debug, Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Hash)]
pub struct WeakVTable<T: ?Sized> {
    /// Cleanup function pointer.
    pub cleanup_fn: WeakCleanupFn<T>,
    /// Clone function pointer.
    pub clone_fn: WeakCloneFn<T>,
    /// Upgrade function pointer.
    pub upgrade_fn: WeakUpgradeFn<T>,
}

unsafe impl<T: Sync + Send + ?Sized> Send for Weak<T> {}
unsafe impl<T: Sync + Send + ?Sized> Sync for Weak<T> {}

impl<T: ?Sized> Weak<T> {
    /// Consumes the `Weak<T>` and turns it into a pair of raw pointers.
    ///
    /// The result can be turned back into the `Weak<T>` with [Weak::from_raw]
    /// to avoid a memory leakage.
    pub fn into_raw(self) -> (NonNullConst<T>, NonNullConst<WeakVTable<T>>) {
        let pointers = (self.data, self.vtable);
        std::mem::forget(self);
        pointers
    }

    /// Creates a new `Weak<T>` from a data and vtable pointer.
    ///
    /// # Safety
    ///
    /// The data pointer must originate from a call to [Weak::into_raw]
    /// or from a function in the vtable.
    pub unsafe fn from_raw(data: NonNullConst<T>, vtable: NonNullConst<WeakVTable<T>>) -> Self {
        Self { data, vtable }
    }

    /// Attempts to upgrade the `Weak<T>` to an `Arc<T>`.
    pub fn upgrade(&self) -> Optional<Arc<T>> {
        unsafe { (self.vtable.as_ref().upgrade_fn)(self.data) }
    }
}

impl<T: ?Sized> Drop for Weak<T> {
    fn drop(&mut self) {
        unsafe { (self.vtable.as_ref().cleanup_fn)(self.data) }
    }
}

impl<T: ?Sized> Clone for Weak<T> {
    fn clone(&self) -> Self {
        unsafe { (self.vtable.as_ref().clone_fn)(self.data) }
    }
}

impl<T: Debug + ?Sized> Debug for Weak<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "(Weak)")
    }
}

impl<T: ?Sized> From<std::sync::Weak<T>> for Weak<T> {
    fn from(ptr: std::sync::Weak<T>) -> Self {
        Self {
            data: unsafe { NonNullConst::new_unchecked(std::sync::Weak::into_raw(ptr)) },
            vtable: NonNullConst::from(&<std::sync::Weak<T>>::VTABLE),
        }
    }
}

/// Trait for creating an `Weak<T>` vtable.
pub trait WeakVTableProvider<T: ?Sized> {
    /// VTable of the `Weak<T>`.
    const VTABLE: WeakVTable<T> = WeakVTable {
        cleanup_fn: TypeWrapper(Self::cleanup),
        clone_fn: TypeWrapper(Self::clone),
        upgrade_fn: TypeWrapper(Self::upgrade),
    };

    /// Implementation of the `Drop` trait.
    ///
    /// # Safety
    ///
    /// This function requires that `ptr` originates from an `Weak<T>` that matches the function.
    unsafe extern "C-unwind" fn cleanup(ptr: NonNullConst<T>);

    /// Implementation of the `Clone` trait.
    ///
    /// # Safety
    ///
    /// This function requires that `ptr` originates from an `Weak<T>` that matches the function.
    unsafe extern "C-unwind" fn clone(ptr: NonNullConst<T>) -> Weak<T>;

    /// Tries to create an `Arc<T>` for the pointer.
    ///
    /// # Safety
    ///
    /// This function requires that `ptr` originates from an `Weak<T>` that matches the function.
    unsafe extern "C-unwind" fn upgrade(ptr: NonNullConst<T>) -> Optional<Arc<T>>;
}

impl<T: ?Sized> WeakVTableProvider<T> for std::sync::Weak<T> {
    unsafe extern "C-unwind" fn cleanup(ptr: NonNullConst<T>) {
        drop(std::sync::Weak::from_raw(ptr.as_ptr()))
    }

    unsafe extern "C-unwind" fn clone(ptr: NonNullConst<T>) -> Weak<T> {
        let original = std::sync::Weak::from_raw(ptr.as_ptr());
        let clone = original.clone();
        forget(original);
        clone.into()
    }

    unsafe extern "C-unwind" fn upgrade(ptr: NonNullConst<T>) -> Optional<Arc<T>> {
        let weak = std::sync::Weak::from_raw(ptr.as_ptr());
        let arc = weak.upgrade().map(|arc| arc.into());
        forget(weak);
        arc.into()
    }
}
