use std::cmp::Ordering;
use std::marker::PhantomData;
use std::panic::{RefUnwindSafe, UnwindSafe};
use std::pin::Pin;
use std::sync::{Arc, Weak};

/// A thread-safe reference-counting pointer.
///
/// Equivalent to an `Arc<dyn DynArcBase>`, but is mapped
/// to a `T` upon dereference. See [`Arc`] for more info.
pub struct DynArc<T: DynArcCompatible + ?Sized> {
    inner: Arc<dyn DynArcBase>,
    _phantom: PhantomData<fn() -> T>,
}

/// Weak version of [`DynArc`].
///
/// See [`Weak`] for more info.
pub struct DynWeak<T: DynArcCompatible + ?Sized> {
    inner: Weak<dyn DynArcBase>,
    _phantom: PhantomData<fn() -> T>,
}

/// Base type of the [`DynArc`] and [`DynWeak`] types.
pub auto trait DynArcBase {}

/// Types compatible with [`DynArc`] and [`DynWeak`].
pub trait DynArcCompatible: DynArcBase {
    /// Casts `&dyn DynArcBase` to a `&Self`.
    fn as_self(base: &dyn DynArcBase) -> &Self;

    /// Casts `&mut dyn DynArcBase` to a `&mut Self`.
    fn as_self_mut(base: &mut dyn DynArcBase) -> &mut Self;

    /// Casts `*const dyn DynArcBase` to a `*const Self`.
    fn as_self_ptr(base: *const dyn DynArcBase) -> *const Self;
}

impl<T: 'static + DynArcCompatible> DynArc<T> {
    /// Constructs a new `DynArc<T>`.
    #[inline]
    pub fn new(data: T) -> Self {
        Self {
            inner: Arc::new(data) as _,
            _phantom: PhantomData,
        }
    }

    /// Constructs a new `Pin<DynArc<T>>`.
    #[inline]
    pub fn pin(data: T) -> Pin<Self> {
        unsafe { Pin::new_unchecked(Self::new(data)) }
    }
}

impl<T: DynArcCompatible + ?Sized> DynArc<T> {
    /// Fetches a raw pointer to the data.
    #[inline]
    pub fn as_ptr(this: &DynArc<T>) -> *const T {
        let ptr = Arc::as_ptr(&this.inner);
        // safety: we know that the pointer is valid.
        unsafe { <T as DynArcCompatible>::as_self(&*ptr) as _ }
    }

    /// Consumes the `DynArc`, returning the inner [`Arc`].
    #[inline]
    pub fn into_inner(this: DynArc<T>) -> Arc<dyn DynArcBase> {
        this.inner
    }

    /// Constructs a `DynArc` with the inner value.
    ///
    /// # Safety
    ///
    /// The caller must ensure, that the `T` is compatible with the
    /// type-erased inner value.
    #[inline]
    pub unsafe fn from_inner(inner: Arc<dyn DynArcBase>) -> Self {
        Self {
            inner,
            _phantom: PhantomData,
        }
    }

    /// Creates a new [`DynWeak`] pointer to this allocation.
    #[inline]
    pub fn downgrade(this: &DynArc<T>) -> DynWeak<T> {
        let weak = Arc::downgrade(&this.inner);
        unsafe { DynWeak::from_inner(weak) }
    }

    /// Fetches the number of [`DynWeak`] pointers pointing to this allocation.
    #[inline]
    pub fn weak_count(this: &DynArc<T>) -> usize {
        Arc::weak_count(&this.inner)
    }

    /// Fetches the number of `DynArc` pointers pointing to this allocation.
    #[inline]
    pub fn strong_count(this: &DynArc<T>) -> usize {
        Arc::strong_count(&this.inner)
    }

    /// Returns `true` if the two `DynArc`s point to the same allocation.
    #[inline]
    pub fn ptr_eq(this: &DynArc<T>, other: &DynArc<T>) -> bool {
        #[allow(clippy::vtable_address_comparisons)]
        Arc::ptr_eq(&this.inner, &other.inner)
    }

    /// Returns a mutable reference into the allocated value, if there
    /// are no other `DynArc` or [`DynWeak`] pointers to the same allocation.
    #[inline]
    pub fn get_mut(this: &mut DynArc<T>) -> Option<&mut T> {
        Arc::get_mut(&mut this.inner).map(|b| <T as DynArcCompatible>::as_self_mut(b))
    }
}

impl<T: DynArcCompatible + ?Sized> AsRef<T> for DynArc<T> {
    #[inline]
    fn as_ref(&self) -> &T {
        &**self
    }
}

impl<T: DynArcCompatible + ?Sized> std::borrow::Borrow<T> for DynArc<T> {
    #[inline]
    fn borrow(&self) -> &T {
        &**self
    }
}

impl<T: DynArcCompatible + ?Sized> Clone for DynArc<T> {
    #[inline]
    fn clone(&self) -> Self {
        let inner = self.inner.clone();
        unsafe { Self::from_inner(inner) }
    }
}

impl<T: std::fmt::Debug + DynArcCompatible + ?Sized> std::fmt::Debug for DynArc<T> {
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Debug::fmt(&**self, f)
    }
}

impl<T: 'static + Default + DynArcCompatible> Default for DynArc<T> {
    #[inline]
    fn default() -> Self {
        Self::new(Default::default())
    }
}

impl<T: DynArcCompatible + ?Sized> std::ops::Deref for DynArc<T> {
    type Target = T;

    #[inline]
    fn deref(&self) -> &Self::Target {
        <T as DynArcCompatible>::as_self(&*self.inner)
    }
}

impl<T: std::fmt::Display + DynArcCompatible + ?Sized> std::fmt::Display for DynArc<T> {
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(&**self, f)
    }
}

impl<T: 'static + DynArcCompatible> From<T> for DynArc<T> {
    #[inline]
    fn from(data: T) -> Self {
        Self::new(data)
    }
}

impl<T: std::hash::Hash + DynArcCompatible + ?Sized> std::hash::Hash for DynArc<T> {
    #[inline]
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        std::hash::Hash::hash(&**self, state)
    }
}

impl<T: Ord + DynArcCompatible + ?Sized> Ord for DynArc<T> {
    #[inline]
    fn cmp(&self, other: &Self) -> Ordering {
        Ord::cmp(&**self, &**other)
    }
}

impl<T: PartialEq<T> + DynArcCompatible + ?Sized> PartialEq<DynArc<T>> for DynArc<T> {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        PartialEq::eq(&**self, &**other)
    }
}

impl<T: PartialOrd<T> + DynArcCompatible + ?Sized> PartialOrd<DynArc<T>> for DynArc<T> {
    #[inline]
    fn partial_cmp(&self, other: &DynArc<T>) -> Option<Ordering> {
        PartialOrd::partial_cmp(&**self, &**other)
    }
}

impl<T: DynArcCompatible + ?Sized> std::fmt::Pointer for DynArc<T> {
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Pointer::fmt(&self.inner, f)
    }
}

impl<T: Eq + DynArcCompatible + ?Sized> Eq for DynArc<T> {}

unsafe impl<T: Sync + Send + DynArcCompatible + ?Sized> Send for DynArc<T> {}

unsafe impl<T: Sync + Send + DynArcCompatible + ?Sized> Sync for DynArc<T> {}

impl<T: DynArcCompatible + ?Sized> Unpin for DynArc<T> {}

impl<T: RefUnwindSafe + DynArcCompatible + ?Sized> UnwindSafe for DynArc<T> {}

impl<T: 'static + DynArcCompatible> DynWeak<T> {
    /// Constructs a new `DynWeak`.
    #[inline]
    pub fn new() -> Self {
        Self {
            inner: Weak::<T>::new() as _,
            _phantom: PhantomData,
        }
    }
}

impl<T: DynArcCompatible + ?Sized> DynWeak<T> {
    /// Fetches a raw pointer to the data.
    #[inline]
    pub fn as_ptr(&self) -> *const T {
        let ptr = self.inner.as_ptr();
        unsafe { <T as DynArcCompatible>::as_self_ptr(&*ptr) }
    }

    /// Fetches the inner [`Weak`] pointer.
    #[inline]
    pub fn into_inner(self) -> Weak<dyn DynArcBase> {
        self.inner
    }

    /// Constructs a `DynWeak` with the inner value.
    ///
    /// # Safety
    ///
    /// The caller must ensure, that the `T` is compatible with the
    /// type-erased inner value.
    #[inline]
    pub unsafe fn from_inner(inner: Weak<dyn DynArcBase>) -> Self {
        Self {
            inner,
            _phantom: PhantomData,
        }
    }

    /// Fetches the number of [`DynArc`] pointers pointing to this allocation.
    #[inline]
    pub fn strong_count(&self) -> usize {
        self.inner.strong_count()
    }

    /// Fetches the number of `DynWeak` pointers pointing to this allocation.
    #[inline]
    pub fn weak_count(&self) -> usize {
        self.inner.weak_count()
    }

    /// Returns `true` if the two `DynWeak`s point to the same allocation.
    #[inline]
    pub fn ptr_eq(&self, other: &DynWeak<T>) -> bool {
        self.inner.ptr_eq(&other.inner)
    }
}

impl<T: DynArcCompatible + ?Sized> Clone for DynWeak<T> {
    #[inline]
    fn clone(&self) -> Self {
        let inner = self.inner.clone();
        unsafe { Self::from_inner(inner) }
    }
}

impl<T: std::fmt::Debug + DynArcCompatible + ?Sized> std::fmt::Debug for DynWeak<T> {
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "(Weak)")
    }
}

impl<T: 'static + DynArcCompatible> Default for DynWeak<T> {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

unsafe impl<T: Sync + Send + DynArcCompatible + ?Sized> Send for DynWeak<T> {}

unsafe impl<T: Sync + Send + DynArcCompatible + ?Sized> Sync for DynWeak<T> {}
