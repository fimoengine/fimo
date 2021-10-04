use std::cmp::Ordering;
use std::marker::PhantomData;
use std::panic::{RefUnwindSafe, UnwindSafe};
use std::pin::Pin;
use std::sync::{Arc, Weak};

/// A thread-safe reference-counting pointer.
///
/// Equivalent to an `Arc<dyn DynArcBase>`, but is mapped
/// to a `T` upon dereference. See [`Arc`] for more info.
pub struct DynArc<T: DynArcBase + ?Sized, C: DynArcCaster<T>> {
    caster: C,
    inner: Arc<dyn DynArcBase>,
    _phantom: PhantomData<fn() -> T>,
}

/// Weak version of [`DynArc`].
///
/// See [`Weak`] for more info.
pub struct DynWeak<T: DynArcBase + ?Sized, C: DynArcCaster<T>> {
    caster: C,
    inner: Weak<dyn DynArcBase>,
    _phantom: PhantomData<fn() -> T>,
}

/// Base type of the [`DynArc`] and [`DynWeak`] types.
pub trait DynArcBase {}

impl<T: ?Sized> DynArcBase for T {}

/// Caster type for [`DynArc`] and [`DynWeak`].
pub trait DynArcCaster<T: DynArcBase + ?Sized>: Copy {
    /// Casts `&dyn DynArcBase` to a `&T`.
    ///
    /// # Safety
    ///
    /// The value of `data` must be compatible with the caster.
    unsafe fn as_self(&self, base: &dyn DynArcBase) -> &T {
        let base_ptr = base as *const _;
        let self_ptr = self.as_self_ptr(base_ptr);
        &*self_ptr
    }

    /// Casts `&mut dyn DynArcBase` to a `&mut T`.
    ///
    /// # Safety
    ///
    /// The value of `data` must be compatible with the caster.
    unsafe fn as_self_mut(&self, base: &mut dyn DynArcBase) -> &mut T {
        let base_ptr = base as *mut _ as *const _;
        let self_ptr = self.as_self_ptr(base_ptr);
        &mut *(self_ptr as *mut T)
    }

    /// Casts `*const dyn DynArcBase` to a `*const T`.
    ///
    /// # Safety
    ///
    /// The value of `data` must be compatible with the caster.
    unsafe fn as_self_ptr<'a>(&self, base: *const (dyn DynArcBase + 'a)) -> *const T;
}

impl<T: 'static + DynArcBase, C: DynArcCaster<T>> DynArc<T, C> {
    /// Constructs a new `DynArc<T>`.
    #[inline]
    pub fn new(data: T, caster: C) -> Self {
        Self {
            caster,
            inner: Arc::new(data) as _,
            _phantom: PhantomData,
        }
    }

    /// Constructs a new `Pin<DynArc<T>>`.
    #[inline]
    pub fn pin(data: T, caster: C) -> Pin<Self> {
        unsafe { Pin::new_unchecked(Self::new(data, caster)) }
    }
}

impl<T: DynArcBase + ?Sized, C: DynArcCaster<T>> DynArc<T, C> {
    /// Fetches a raw pointer to the data.
    #[inline]
    pub fn as_ptr(this: &DynArc<T, C>) -> *const T {
        let ptr = Arc::as_ptr(&this.inner);
        unsafe { this.caster.as_self_ptr(ptr) }
    }

    /// Consumes the `DynArc`, returning the inner [`Arc`] and caster.
    #[inline]
    pub fn into_inner(this: DynArc<T, C>) -> (Arc<dyn DynArcBase>, C) {
        (this.inner, this.caster)
    }

    /// Constructs a `DynArc` with the inner value.
    ///
    /// # Safety
    ///
    /// The caller must ensure, that the `C` is compatible with the
    /// type-erased inner value.
    #[inline]
    pub unsafe fn from_inner(inner: (Arc<dyn DynArcBase>, C)) -> Self {
        Self {
            inner: inner.0,
            caster: inner.1,
            _phantom: PhantomData,
        }
    }

    /// Creates a new [`DynWeak`] pointer to this allocation.
    #[inline]
    pub fn downgrade(this: &DynArc<T, C>) -> DynWeak<T, C> {
        let weak = Arc::downgrade(&this.inner);
        let caster = this.caster;
        let inner = (weak, caster);
        unsafe { DynWeak::from_inner(inner) }
    }

    /// Fetches the number of [`DynWeak`] pointers pointing to this allocation.
    #[inline]
    pub fn weak_count(this: &DynArc<T, C>) -> usize {
        Arc::weak_count(&this.inner)
    }

    /// Fetches the number of `DynArc` pointers pointing to this allocation.
    #[inline]
    pub fn strong_count(this: &DynArc<T, C>) -> usize {
        Arc::strong_count(&this.inner)
    }

    /// Returns `true` if the two `DynArc`s point to the same allocation.
    #[inline]
    pub fn ptr_eq(this: &DynArc<T, C>, other: &DynArc<T, C>) -> bool {
        #[allow(clippy::vtable_address_comparisons)]
        Arc::ptr_eq(&this.inner, &other.inner)
    }

    /// Returns a mutable reference into the allocated value, if there
    /// are no other `DynArc` or [`DynWeak`] pointers to the same allocation.
    #[inline]
    pub fn get_mut(this: &mut DynArc<T, C>) -> Option<&mut T> {
        let inner = &mut this.inner;
        let caster = &this.caster;
        Arc::get_mut(inner).map(move |b| unsafe { caster.as_self_mut(b) })
    }
}

impl<T: DynArcBase + ?Sized, C: DynArcCaster<T>> AsRef<T> for DynArc<T, C> {
    #[inline]
    fn as_ref(&self) -> &T {
        &**self
    }
}

impl<T: DynArcBase + ?Sized, C: DynArcCaster<T>> std::borrow::Borrow<T> for DynArc<T, C> {
    #[inline]
    fn borrow(&self) -> &T {
        &**self
    }
}

impl<T: DynArcBase + ?Sized, C: DynArcCaster<T>> Clone for DynArc<T, C> {
    #[inline]
    fn clone(&self) -> Self {
        let inner = (self.inner.clone(), self.caster);
        unsafe { Self::from_inner(inner) }
    }
}

impl<T: std::fmt::Debug + DynArcBase + ?Sized, C: DynArcCaster<T>> std::fmt::Debug
    for DynArc<T, C>
{
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Debug::fmt(&**self, f)
    }
}

impl<T: 'static + Default + DynArcBase, C: Default + DynArcCaster<T>> Default for DynArc<T, C> {
    #[inline]
    fn default() -> Self {
        Self::new(Default::default(), Default::default())
    }
}

impl<T: DynArcBase + ?Sized, C: DynArcCaster<T>> std::ops::Deref for DynArc<T, C> {
    type Target = T;

    #[inline]
    fn deref(&self) -> &Self::Target {
        unsafe { self.caster.as_self(&*self.inner) }
    }
}

impl<T: std::fmt::Display + DynArcBase + ?Sized, C: DynArcCaster<T>> std::fmt::Display
    for DynArc<T, C>
{
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(&**self, f)
    }
}

impl<T: 'static + DynArcBase, C: DynArcCaster<T>> From<(T, C)> for DynArc<T, C> {
    #[inline]
    fn from(data: (T, C)) -> Self {
        Self::new(data.0, data.1)
    }
}

impl<T: std::hash::Hash + DynArcBase + ?Sized, C: DynArcCaster<T>> std::hash::Hash
    for DynArc<T, C>
{
    #[inline]
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        std::hash::Hash::hash(&**self, state)
    }
}

impl<T: Ord + DynArcBase + ?Sized, C: DynArcCaster<T>> Ord for DynArc<T, C> {
    #[inline]
    fn cmp(&self, other: &Self) -> Ordering {
        Ord::cmp(&**self, &**other)
    }
}

impl<T: PartialEq<T> + DynArcBase + ?Sized, C: DynArcCaster<T>> PartialEq<DynArc<T, C>>
    for DynArc<T, C>
{
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        PartialEq::eq(&**self, &**other)
    }
}

impl<T: PartialOrd<T> + DynArcBase + ?Sized, C: DynArcCaster<T>> PartialOrd<DynArc<T, C>>
    for DynArc<T, C>
{
    #[inline]
    fn partial_cmp(&self, other: &DynArc<T, C>) -> Option<Ordering> {
        PartialOrd::partial_cmp(&**self, &**other)
    }
}

impl<T: DynArcBase + ?Sized, C: DynArcCaster<T>> std::fmt::Pointer for DynArc<T, C> {
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Pointer::fmt(&self.inner, f)
    }
}

impl<T: Eq + DynArcBase + ?Sized, C: DynArcCaster<T>> Eq for DynArc<T, C> {}

unsafe impl<T: Sync + Send + DynArcBase + ?Sized, C: Sync + Send + DynArcCaster<T>> Send
    for DynArc<T, C>
{
}

unsafe impl<T: Sync + Send + DynArcBase + ?Sized, C: Sync + Send + DynArcCaster<T>> Sync
    for DynArc<T, C>
{
}

impl<T: DynArcBase + ?Sized, C: DynArcCaster<T>> Unpin for DynArc<T, C> {}

impl<T: RefUnwindSafe + DynArcBase + ?Sized, C: RefUnwindSafe + DynArcCaster<T>> UnwindSafe
    for DynArc<T, C>
{
}

impl<T: 'static + DynArcBase, C: DynArcCaster<T>> DynWeak<T, C> {
    /// Constructs a new `DynWeak`.
    #[inline]
    pub fn new(caster: C) -> Self {
        Self {
            caster,
            inner: Weak::<T>::new() as _,
            _phantom: PhantomData,
        }
    }
}

impl<T: DynArcBase + ?Sized, C: DynArcCaster<T>> DynWeak<T, C> {
    /// Fetches a raw pointer to the data.
    #[inline]
    pub fn as_ptr(&self) -> *const T {
        let ptr = self.inner.as_ptr();
        unsafe { self.caster.as_self_ptr(ptr) }
    }

    /// Fetches the inner [`Weak`] pointer and Caster.
    #[inline]
    pub fn into_inner(self) -> Weak<dyn DynArcBase> {
        self.inner
    }

    /// Constructs a `DynWeak` with the inner value.
    ///
    /// # Safety
    ///
    /// The caller must ensure, that the `C` is compatible with the
    /// type-erased inner value.
    #[inline]
    pub unsafe fn from_inner(inner: (Weak<dyn DynArcBase>, C)) -> Self {
        Self {
            inner: inner.0,
            caster: inner.1,
            _phantom: PhantomData,
        }
    }

    /// Attempts to upgrade the `DynWeak` pointer to a [`DynArc`].
    ///
    /// Returns [`None`] if the inner value has since been dropped.
    #[inline]
    pub fn upgrade(&self) -> Option<DynArc<T, C>> {
        self.inner.upgrade().map(|inner | {
            unsafe { DynArc::from_inner((inner, self.caster)) }
        })
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
    pub fn ptr_eq(&self, other: &DynWeak<T, C>) -> bool {
        self.inner.ptr_eq(&other.inner)
    }
}

impl<T: DynArcBase + ?Sized, C: DynArcCaster<T>> Clone for DynWeak<T, C> {
    #[inline]
    fn clone(&self) -> Self {
        let inner = (self.inner.clone(), self.caster);
        unsafe { Self::from_inner(inner) }
    }
}

impl<T: std::fmt::Debug + DynArcBase + ?Sized, C: DynArcCaster<T>> std::fmt::Debug
    for DynWeak<T, C>
{
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "(Weak)")
    }
}

impl<T: 'static + DynArcBase, C: Default + DynArcCaster<T>> Default for DynWeak<T, C> {
    #[inline]
    fn default() -> Self {
        Self::new(Default::default())
    }
}

unsafe impl<T: Sync + Send + DynArcBase + ?Sized, C: Sync + Send + DynArcCaster<T>> Send
    for DynWeak<T, C>
{
}

unsafe impl<T: Sync + Send + DynArcBase + ?Sized, C: Sync + Send + DynArcCaster<T>> Sync
    for DynWeak<T, C>
{
}
