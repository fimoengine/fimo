//! Callable wrappers.
use crate::marker::NoneMarker;
use fimo_object::vtable::MarkerCompatible;
use std::fmt::{Debug, Formatter};
use std::marker::PhantomData;
use std::mem::MaybeUninit;
use std::sync::Arc;

#[repr(C)]
struct RawCallable<Args, T, M> {
    ptr: *const (),
    drop: fn(*const ()),
    call: fn(*const (), Args) -> T,
    _marker: PhantomData<M>,
}

impl<Args, T, M> RawCallable<Args, T, M> {
    #[inline]
    unsafe fn new(ptr: *const (), drop: fn(*const ()), call: fn(*const (), Args) -> T) -> Self {
        Self {
            ptr,
            drop,
            call,
            _marker: PhantomData,
        }
    }

    #[inline]
    fn call(&self, args: Args) -> T {
        (self.call)(self.ptr, args)
    }
}

unsafe impl<Args, T, M: Send> Send for RawCallable<Args, T, M> {}
unsafe impl<Args, T, M: Sync> Sync for RawCallable<Args, T, M> {}

impl<Args, T, M> Drop for RawCallable<Args, T, M> {
    #[inline]
    fn drop(&mut self) {
        (self.drop)(self.ptr)
    }
}

impl<Args, T, M> Debug for RawCallable<Args, T, M> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RawHeapFn")
            .field("ptr", &self.ptr)
            .field("drop", &self.drop)
            .field("call", &self.call)
            .finish()
    }
}

/// A [`FnOnce`] reference without lifetimes.
#[repr(C)]
pub struct RawFnOnce<Args, T, M = NoneMarker> {
    raw: RawCallable<Args, T, M>,
}

impl<Args, T, M> RawFnOnce<Args, T, M> {
    /// Constructs a new `RawFnOnce` with a callable reference.
    ///
    /// # Safety
    ///
    /// The reference `f` must outlive the `RawFnOnce` and properly initialized.
    /// Once called, `f` is owned by the `RawFnOnce` and may not be used anymore.
    /// An exception to this rule is, if the `RawFnOnce` was forgotten with [`std::mem::forget`].
    #[inline]
    pub unsafe fn new<F: FnOnce<Args, Output = T>>(f: &'_ mut MaybeUninit<F>) -> Self
    where
        M: MarkerCompatible<F>,
    {
        let raw = f as *mut MaybeUninit<F>;
        Self {
            // we own `f` so we drop the value.
            raw: RawCallable::new(raw as *const (), drop_value::<F>, Self::call_ref::<F>),
        }
    }

    /// Constructs a new `RawFnOnce`.
    #[inline]
    pub fn new_boxed<F: FnOnce<Args, Output = T>>(f: Box<F>) -> Self
    where
        M: MarkerCompatible<F>,
    {
        let raw = Box::into_raw(f);
        Self {
            raw: unsafe {
                // drop the Box
                RawCallable::new(raw as *const (), drop_boxed::<F>, Self::call_boxed::<F>)
            },
        }
    }

    /// Calls the function consuming the wrapper.
    ///
    /// # Safety
    ///
    /// This function is unsafe, because one must ensure, that the wrapped function
    /// has not been dropped.
    #[inline]
    pub unsafe extern "rust-call" fn call_once(self, args: Args) -> T {
        let res = self.raw.call(args);

        // the internal value is dropped when called.
        std::mem::forget(self);
        res
    }

    fn call_boxed<F: FnOnce<Args, Output = T>>(ptr: *const (), args: Args) -> T {
        let raw = ptr as *mut F;
        let boxed = unsafe { Box::from_raw(raw) };
        <Box<F> as FnOnce<Args>>::call_once(boxed, args)
    }

    fn call_ref<F: FnOnce<Args, Output = T>>(ptr: *const (), args: Args) -> T {
        let f = unsafe {
            let raw = ptr as *mut MaybeUninit<F>;
            let raw = std::ptr::read(raw);
            raw.assume_init()
        };
        <F as FnOnce<Args>>::call_once(f, args)
    }
}

impl<Args, T, M> Debug for RawFnOnce<Args, T, M> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        Debug::fmt(&self.raw, f)
    }
}

/// A [`FnMut`] reference without lifetimes.
#[repr(C)]
pub struct RawFnMut<Args, T, M = NoneMarker> {
    raw: RawCallable<Args, T, M>,
}

impl<Args, T, M> RawFnMut<Args, T, M> {
    /// Constructs a new `RawFnMut` by wrapping `f`.
    ///
    /// # Safety
    ///
    /// The caller must ensure, that `f` outlives the `RawFnMut`.
    #[inline]
    pub unsafe fn new<F: FnMut<Args, Output = T>>(f: &mut F) -> Self
    where
        M: MarkerCompatible<F>,
    {
        let raw = f as *mut F;
        Self {
            // we wrap by reference, so we don't need to drop.
            raw: RawCallable::new(raw as *const (), drop_forget, Self::call_ref::<F>),
        }
    }

    /// Constructs a new `RawFnMut`.
    #[inline]
    pub fn new_boxed<F: FnMut<Args, Output = T>>(f: Box<F>) -> Self
    where
        M: MarkerCompatible<F>,
    {
        let raw = Box::into_raw(f);
        Self {
            // the box needs to be dropped.
            raw: unsafe {
                RawCallable::new(raw as *const (), drop_boxed::<F>, Self::call_ref::<F>)
            },
        }
    }

    /// Calls the function consuming the wrapper.
    ///
    /// # Safety
    ///
    /// This function is unsafe, because one must ensure, that the wrapped function
    /// has not been dropped.
    #[inline]
    pub unsafe extern "rust-call" fn call_once(mut self, args: Args) -> T {
        self.call_mut(args)
    }

    /// Calls the function by mutable reference.
    ///
    /// # Safety
    ///
    /// This function is unsafe, because one must ensure, that the wrapped function
    /// has not been dropped.
    #[inline]
    pub unsafe extern "rust-call" fn call_mut(&mut self, args: Args) -> T {
        self.raw.call(args)
    }

    fn call_ref<F: FnMut<Args, Output = T>>(ptr: *const (), args: Args) -> T {
        let raw = ptr as *mut F;
        let f = unsafe { &mut *raw };
        <F as FnMut<Args>>::call_mut(f, args)
    }
}

impl<Args, T, M> Debug for RawFnMut<Args, T, M> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        Debug::fmt(&self.raw, f)
    }
}

/// A [`Fn`] reference without lifetimes.
#[repr(C)]
pub struct RawFn<Args, T, M = NoneMarker> {
    raw: RawCallable<Args, T, M>,
}

impl<Args, T, M> RawFn<Args, T, M> {
    /// Constructs a new `RawFn` by wrapping `f`.
    ///
    /// # Safety
    ///
    /// The caller must ensure, that `f` outlives the `RawFn`.
    #[inline]
    pub unsafe fn new<F: Fn<Args, Output = T>>(f: &F) -> Self
    where
        M: MarkerCompatible<F>,
    {
        let raw = f as *const F;
        Self {
            // references do not need dropping
            raw: RawCallable::new(raw as *const (), drop_forget, Self::call_ref::<F>),
        }
    }

    /// Constructs a new `RawFn.
    #[inline]
    pub fn new_boxed<F: Fn<Args, Output = T>>(f: Box<F>) -> Self
    where
        M: MarkerCompatible<F>,
    {
        let raw = Box::into_raw(f);
        Self {
            raw: unsafe {
                // drop the Box
                RawCallable::new(raw as *const (), drop_boxed::<F>, Self::call_ref::<F>)
            },
        }
    }

    /// Constructs a new `RawFn`.
    #[inline]
    pub fn new_arc<F: Fn<Args, Output = T>>(f: Arc<F>) -> Self
    where
        M: MarkerCompatible<F>,
    {
        let raw = Arc::into_raw(f);
        Self {
            raw: unsafe {
                // drop the Arc
                RawCallable::new(raw as *const (), drop_arc::<F>, Self::call_ref::<F>)
            },
        }
    }

    /// Calls the function consuming the wrapper.
    ///
    /// # Safety
    ///
    /// This function is unsafe, because one must ensure, that the wrapped function
    /// has not been dropped.
    #[inline]
    pub unsafe extern "rust-call" fn call_once(mut self, args: Args) -> T {
        self.call_mut(args)
    }

    /// Calls the function by mutable reference.
    ///
    /// # Safety
    ///
    /// This function is unsafe, because one must ensure, that the wrapped function
    /// has not been dropped.
    #[inline]
    pub unsafe extern "rust-call" fn call_mut(&mut self, args: Args) -> T {
        self.call(args)
    }

    /// Calls the function by reference.
    ///
    /// # Safety
    ///
    /// This function is unsafe, because one must ensure, that the wrapped function
    /// has not been dropped.
    #[inline]
    pub unsafe extern "rust-call" fn call(&self, args: Args) -> T {
        self.raw.call(args)
    }

    fn call_ref<F: Fn<Args, Output = T>>(ptr: *const (), args: Args) -> T {
        let raw = ptr as *const F;
        let f = unsafe { &*raw };
        <F as Fn<Args>>::call(f, args)
    }
}

impl<Args, T, M> Debug for RawFn<Args, T, M> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        Debug::fmt(&self.raw, f)
    }
}

/// A [`FnOnce`] wrapped by reference.
#[repr(C)]
pub struct RefFnOnce<'a, Args, T, M = NoneMarker> {
    raw: RawFnOnce<Args, T, M>,
    _phantom: PhantomData<fn() -> &'a mut ()>,
}

impl<'a, Args, T, M> RefFnOnce<'a, Args, T, M> {
    /// Constructs a new `RefFnOnce` with a callable reference.
    ///
    /// # Safety
    ///
    /// The function `f` must be properly initialized.
    /// Once called, `f` is owned by the `RefFnOnce` and may not be used anymore.
    /// An exception to this rule is, if the `RefFnOnce` was forgotten with [`std::mem::forget`].
    #[inline]
    pub unsafe fn new<F: FnOnce<Args, Output = T>>(f: &'a mut MaybeUninit<F>) -> Self
    where
        M: MarkerCompatible<F>,
    {
        Self {
            raw: RawFnOnce::new(f),
            _phantom: Default::default(),
        }
    }

    /// Constructs a new `RefFnOnce`.
    #[inline]
    pub fn new_boxed<F: FnOnce<Args, Output = T>>(f: Box<F>) -> RefFnOnce<'static, Args, T, M>
    where
        M: MarkerCompatible<F>,
    {
        RefFnOnce {
            raw: RawFnOnce::new_boxed(f),
            _phantom: Default::default(),
        }
    }

    /// Extracts the raw wrapper.
    #[inline]
    pub fn into_raw(self) -> RawFnOnce<Args, T, M> {
        self.raw
    }

    /// Constructs a new `RefFnOnce` from a [`RawFnOnce`].
    ///
    /// # Safety
    ///
    /// Construction from a raw value is inherently unsafe,
    /// because that allows for the wrapped value to be moved.
    #[inline]
    pub unsafe fn from_raw(f: RawFnOnce<Args, T, M>) -> Self {
        RefFnOnce {
            raw: f,
            _phantom: Default::default(),
        }
    }
}

impl<Args, T, M> FnOnce<Args> for RefFnOnce<'_, Args, T, M> {
    type Output = T;

    #[inline]
    extern "rust-call" fn call_once(self, args: Args) -> Self::Output {
        unsafe { self.raw.call_once(args) }
    }
}

impl<Args, T, M> Debug for RefFnOnce<'_, Args, T, M> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        Debug::fmt(&self.raw, f)
    }
}

/// A [`FnMut`] wrapped by reference.
#[repr(C)]
pub struct RefFnMut<'a, Args, T, M = NoneMarker> {
    raw: RawFnMut<Args, T, M>,
    _phantom: PhantomData<fn() -> &'a mut ()>,
}

impl<'a, Args, T, M> RefFnMut<'a, Args, T, M> {
    /// Constructs a new `RefFnMut` by wrapping `f`.
    #[inline]
    pub fn new<F: FnMut<Args, Output = T>>(f: &'a mut F) -> Self
    where
        M: MarkerCompatible<F>,
    {
        Self {
            // `f` will outlive self because of the `'a` lifetime.
            raw: unsafe { RawFnMut::new(f) },
            _phantom: Default::default(),
        }
    }

    /// Constructs a new `RefFnMut`.
    #[inline]
    pub fn new_boxed<F: FnMut<Args, Output = T>>(f: Box<F>) -> RefFnMut<'static, Args, T, M>
    where
        M: MarkerCompatible<F>,
    {
        RefFnMut {
            raw: RawFnMut::new_boxed(f),
            _phantom: Default::default(),
        }
    }

    /// Extracts the raw wrapper.
    #[inline]
    pub fn into_raw(self) -> RawFnMut<Args, T, M> {
        self.raw
    }

    /// Constructs a new `RefFnMut` from a [`RawFnMut`].
    ///
    /// # Safety
    ///
    /// Construction from a raw value is inherently unsafe,
    /// because that allows for the wrapped value to be moved.
    #[inline]
    pub unsafe fn from_raw(f: RawFnMut<Args, T, M>) -> Self {
        RefFnMut {
            raw: f,
            _phantom: Default::default(),
        }
    }
}

impl<'a, Args, T, M> FnOnce<Args> for RefFnMut<'a, Args, T, M> {
    type Output = T;

    #[inline]
    extern "rust-call" fn call_once(mut self, args: Args) -> Self::Output {
        <Self as FnMut<Args>>::call_mut(&mut self, args)
    }
}

impl<'a, Args, T, M> FnMut<Args> for RefFnMut<'a, Args, T, M> {
    #[inline]
    extern "rust-call" fn call_mut(&mut self, args: Args) -> Self::Output {
        unsafe { self.raw.call_mut(args) }
    }
}

impl<'a, Args, T, M> Debug for RefFnMut<'a, Args, T, M> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        Debug::fmt(&self.raw, f)
    }
}

/// A [`Fn`] wrapped by reference.
#[repr(C)]
pub struct RefFn<'a, Args, T, M = NoneMarker> {
    raw: RawFn<Args, T, M>,
    _phantom: PhantomData<fn() -> &'a ()>,
}

impl<'a, Args, T, M> RefFn<'a, Args, T, M> {
    /// Constructs a new `RefFn` by wrapping `f`.
    #[inline]
    pub fn new<F: Fn<Args, Output = T>>(f: &'a F) -> Self
    where
        M: MarkerCompatible<F>,
    {
        Self {
            // `f` will outlive self because of the `'a` lifetime.
            raw: unsafe { RawFn::new(f) },
            _phantom: Default::default(),
        }
    }

    /// Constructs a new `RefFn.
    #[inline]
    pub fn new_boxed<F: Fn<Args, Output = T>>(f: Box<F>) -> RefFn<'static, Args, T, M>
    where
        M: MarkerCompatible<F>,
    {
        RefFn {
            raw: RawFn::new_boxed(f),
            _phantom: Default::default(),
        }
    }

    /// Constructs a new `RefFn`.
    #[inline]
    pub fn new_arc<F: Fn<Args, Output = T>>(f: Arc<F>) -> RefFn<'static, Args, T, M>
    where
        M: MarkerCompatible<F>,
    {
        RefFn {
            raw: RawFn::new_arc(f),
            _phantom: Default::default(),
        }
    }

    /// Extracts the raw wrapper.
    #[inline]
    pub fn into_raw(self) -> RawFn<Args, T, M> {
        self.raw
    }

    /// Constructs a new `RefFn` from a [`RawFn`].
    ///
    /// # Safety
    ///
    /// Construction from a raw value is inherently unsafe,
    /// because that allows for the wrapped value to be moved.
    #[inline]
    pub unsafe fn from_raw(f: RawFn<Args, T, M>) -> Self {
        RefFn {
            raw: f,
            _phantom: Default::default(),
        }
    }
}

impl<'a, Args, T, M> FnOnce<Args> for RefFn<'a, Args, T, M> {
    type Output = T;

    #[inline]
    extern "rust-call" fn call_once(mut self, args: Args) -> Self::Output {
        <Self as FnMut<Args>>::call_mut(&mut self, args)
    }
}

impl<'a, Args, T, M> FnMut<Args> for RefFn<'a, Args, T, M> {
    #[inline]
    extern "rust-call" fn call_mut(&mut self, args: Args) -> Self::Output {
        <Self as Fn<Args>>::call(self, args)
    }
}

impl<'a, Args, T, M> Fn<Args> for RefFn<'a, Args, T, M> {
    #[inline]
    extern "rust-call" fn call(&self, args: Args) -> Self::Output {
        unsafe { self.raw.call(args) }
    }
}

impl<'a, Args, T, M> Debug for RefFn<'a, Args, T, M> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        Debug::fmt(&self.raw, f)
    }
}

/// A [`FnOnce`] allocated on the heap.
#[repr(C)]
pub struct HeapFnOnce<Args, T, M = NoneMarker> {
    raw: RefFnOnce<'static, Args, T, M>,
}

impl<Args, T, M> HeapFnOnce<Args, T, M> {
    /// Constructs a new `HeapFnOnce` by boxing the callable.
    #[inline]
    pub fn new<F: FnOnce<Args, Output = T>>(f: F) -> Self
    where
        M: MarkerCompatible<F>,
    {
        Self::new_boxed(Box::new(f))
    }

    /// Constructs a new `HeapFnOnce`.
    #[inline]
    pub fn new_boxed<F: FnOnce<Args, Output = T>>(f: Box<F>) -> Self
    where
        M: MarkerCompatible<F>,
    {
        Self {
            raw: RefFnOnce::new_boxed(f),
        }
    }
}

impl<Args, T, M> FnOnce<Args> for HeapFnOnce<Args, T, M> {
    type Output = T;

    #[inline]
    extern "rust-call" fn call_once(self, args: Args) -> Self::Output {
        <RefFnOnce<'static, Args, T, M> as FnOnce<Args>>::call_once(self.raw, args)
    }
}

impl<Args, T, M> Debug for HeapFnOnce<Args, T, M> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        Debug::fmt(&self.raw, f)
    }
}

/// A [`FnMut`] allocated on the heap.
#[repr(C)]
pub struct HeapFnMut<Args, T, M = NoneMarker> {
    raw: RefFnMut<'static, Args, T, M>,
}

impl<Args, T, M> HeapFnMut<Args, T, M> {
    /// Constructs a new `HeapFnMut` by boxing the callable.
    #[inline]
    pub fn new<F: FnMut<Args, Output = T>>(f: F) -> Self
    where
        M: MarkerCompatible<F>,
    {
        Self::new_boxed(Box::new(f))
    }

    /// Constructs a new `HeapFnMut`.
    #[inline]
    pub fn new_boxed<F: FnMut<Args, Output = T>>(f: Box<F>) -> Self
    where
        M: MarkerCompatible<F>,
    {
        Self {
            raw: RefFnMut::new_boxed(f),
        }
    }
}

impl<Args, T, M> FnOnce<Args> for HeapFnMut<Args, T, M> {
    type Output = T;

    #[inline]
    extern "rust-call" fn call_once(mut self, args: Args) -> Self::Output {
        <Self as FnMut<Args>>::call_mut(&mut self, args)
    }
}

impl<Args, T, M> FnMut<Args> for HeapFnMut<Args, T, M> {
    #[inline]
    extern "rust-call" fn call_mut(&mut self, args: Args) -> Self::Output {
        <RefFnMut<'static, Args, T, M> as FnMut<Args>>::call_mut(&mut self.raw, args)
    }
}

impl<Args, T, M> Debug for HeapFnMut<Args, T, M> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        Debug::fmt(&self.raw, f)
    }
}

/// A [`Fn`] allocated on the heap.
#[repr(C)]
pub struct HeapFn<Args, T, M = NoneMarker> {
    raw: RefFn<'static, Args, T, M>,
}

impl<Args, T, M> HeapFn<Args, T, M> {
    /// Constructs a new `HeapFn` by boxing the callable.
    #[inline]
    pub fn new<F: Fn<Args, Output = T>>(f: F) -> Self
    where
        M: MarkerCompatible<F>,
    {
        Self::new_boxed(Box::new(f))
    }

    /// Constructs a new `HeapFn`.
    #[inline]
    pub fn new_boxed<F: Fn<Args, Output = T>>(f: Box<F>) -> Self
    where
        M: MarkerCompatible<F>,
    {
        Self {
            raw: RefFn::new_boxed(f),
        }
    }

    /// Constructs a new `HeapFn`.
    #[inline]
    pub fn new_arc<F: Fn<Args, Output = T>>(f: Arc<F>) -> Self
    where
        M: MarkerCompatible<F>,
    {
        Self {
            raw: RefFn::new_arc(f),
        }
    }
}

impl<Args, T, M> FnOnce<Args> for HeapFn<Args, T, M> {
    type Output = T;

    #[inline]
    extern "rust-call" fn call_once(mut self, args: Args) -> Self::Output {
        <Self as FnMut<Args>>::call_mut(&mut self, args)
    }
}

impl<Args, T, M> FnMut<Args> for HeapFn<Args, T, M> {
    #[inline]
    extern "rust-call" fn call_mut(&mut self, args: Args) -> Self::Output {
        <Self as Fn<Args>>::call(self, args)
    }
}

impl<Args, T, M> Fn<Args> for HeapFn<Args, T, M> {
    #[inline]
    extern "rust-call" fn call(&self, args: Args) -> Self::Output {
        <RefFn<'static, Args, T, M> as Fn<Args>>::call(&self.raw, args)
    }
}

impl<Args, T, M> Debug for HeapFn<Args, T, M> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        Debug::fmt(&self.raw, f)
    }
}

fn drop_forget(_ptr: *const ()) {}

fn drop_value<F>(ptr: *const ()) {
    let raw = ptr as *mut F;
    let f = unsafe { std::ptr::read(raw) };
    drop(f);
}

fn drop_boxed<F>(ptr: *const ()) {
    let raw = ptr as *mut F;
    let boxed = unsafe { Box::from_raw(raw) };
    drop(boxed);
}

fn drop_arc<F>(ptr: *const ()) {
    let raw = ptr as *const F;
    let arc = unsafe { Arc::from_raw(raw) };
    drop(arc);
}
