//! Callable wrappers.
use std::fmt::{Debug, Formatter};
use std::marker::PhantomData;
use std::sync::Arc;

#[repr(C)]
struct RawHeapFn<Args, T> {
    ptr: *const (),
    drop: fn(*const ()),
    call: fn(*const (), Args) -> T,
}

impl<Args, T> RawHeapFn<Args, T> {
    #[inline]
    unsafe fn new(ptr: *const (), drop: fn(*const ()), call: fn(*const (), Args) -> T) -> Self {
        Self { ptr, drop, call }
    }

    #[inline]
    fn call(&self, args: Args) -> T {
        (self.call)(self.ptr, args)
    }
}

impl<Args, T> Drop for RawHeapFn<Args, T> {
    #[inline]
    fn drop(&mut self) {
        (self.drop)(self.ptr)
    }
}

impl<Args, T> Debug for RawHeapFn<Args, T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RawHeapFn")
            .field("ptr", &self.ptr)
            .field("drop", &self.drop)
            .field("call", &self.call)
            .finish()
    }
}

/// A [`FnOnce`] wrapped by reference.
pub struct RefFnOnce<'a, Args, T> {
    raw: RawHeapFn<Args, T>,
    _phantom: PhantomData<fn() -> &'a mut ()>,
}

impl<'a, Args, T> RefFnOnce<'a, Args, T> {
    /// Constructs a new `RefFnOnce` with a callable reference.
    ///
    /// # Safety
    ///
    /// Once called, `f` is owned by the `RefFnOnce` and may not be used anymore.
    /// An exception to this rule is, if the `RefFnOnce` was forgotten with [`std::mem::forget`].
    #[inline]
    pub unsafe fn new<F: FnOnce<Args, Output = T>>(f: &'a mut F) -> Self {
        let raw = f as *mut F;
        Self {
            // we own `f` so we drop the value.
            raw: RawHeapFn::new(raw as *const (), drop_value::<F>, Self::call_ref::<F>),
            _phantom: Default::default(),
        }
    }

    /// Constructs a new `RefFnOnce`.
    #[inline]
    pub fn new_boxed<F: FnOnce<Args, Output = T>>(f: Box<F>) -> RefFnOnce<'static, Args, T> {
        let raw = Box::into_raw(f);
        RefFnOnce {
            raw: unsafe {
                // drop the Box
                RawHeapFn::new(raw as *const (), drop_boxed::<F>, Self::call_boxed::<F>)
            },
            _phantom: Default::default(),
        }
    }

    fn call_boxed<F: FnOnce<Args, Output = T>>(ptr: *const (), args: Args) -> T {
        let raw = ptr as *mut F;
        let boxed = unsafe { Box::from_raw(raw) };
        <Box<F> as FnOnce<Args>>::call_once(boxed, args)
    }

    fn call_ref<F: FnOnce<Args, Output = T>>(ptr: *const (), args: Args) -> T {
        let raw = ptr as *mut F;
        let f = unsafe { std::ptr::read(raw) };
        <F as FnOnce<Args>>::call_once(f, args)
    }
}

impl<'a, Args, T> FnOnce<Args> for RefFnOnce<'a, Args, T> {
    type Output = T;

    #[inline]
    extern "rust-call" fn call_once(self, args: Args) -> Self::Output {
        let res = self.raw.call(args);

        // the internal value is dropped when called.
        std::mem::forget(self);
        res
    }
}

impl<'a, Args, T> Debug for RefFnOnce<'a, Args, T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        Debug::fmt(&self.raw, f)
    }
}

/// A [`FnMut`] wrapped by reference.
pub struct RefFnMut<'a, Args, T> {
    raw: RawHeapFn<Args, T>,
    _phantom: PhantomData<fn() -> &'a mut ()>,
}

impl<'a, Args, T> RefFnMut<'a, Args, T> {
    /// Constructs a new `RefFnMut` by wrapping `f`.
    #[inline]
    pub fn new<F: FnMut<Args, Output = T>>(f: &'a mut F) -> Self {
        let raw = f as *mut F;
        Self {
            // we wrap by reference, so we don't need to drop.
            raw: unsafe { RawHeapFn::new(raw as *const (), drop_forget, Self::call_ref::<F>) },
            _phantom: Default::default(),
        }
    }

    /// Constructs a new `RefFnMut`.
    #[inline]
    pub fn new_boxed<F: FnMut<Args, Output = T>>(f: Box<F>) -> RefFnMut<'static, Args, T> {
        let raw = Box::into_raw(f);
        RefFnMut {
            // the box needs to be dropped.
            raw: unsafe { RawHeapFn::new(raw as *const (), drop_boxed::<F>, Self::call_ref::<F>) },
            _phantom: Default::default(),
        }
    }

    fn call_ref<F: FnMut<Args, Output = T>>(ptr: *const (), args: Args) -> T {
        let raw = ptr as *mut F;
        let f = unsafe { &mut *raw };
        <F as FnMut<Args>>::call_mut(f, args)
    }
}

impl<'a, Args, T> FnOnce<Args> for RefFnMut<'a, Args, T> {
    type Output = T;

    #[inline]
    extern "rust-call" fn call_once(mut self, args: Args) -> Self::Output {
        <Self as FnMut<Args>>::call_mut(&mut self, args)
    }
}

impl<'a, Args, T> FnMut<Args> for RefFnMut<'a, Args, T> {
    #[inline]
    extern "rust-call" fn call_mut(&mut self, args: Args) -> Self::Output {
        self.raw.call(args)
    }
}

impl<'a, Args, T> Debug for RefFnMut<'a, Args, T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        Debug::fmt(&self.raw, f)
    }
}

/// A [`Fn`] wrapped by reference.
pub struct RefFn<'a, Args, T> {
    raw: RawHeapFn<Args, T>,
    _phantom: PhantomData<fn() -> &'a ()>,
}

impl<'a, Args, T> RefFn<'a, Args, T> {
    /// Constructs a new `RefFn` by wrapping `f`.
    #[inline]
    pub fn new<F: Fn<Args, Output = T>>(f: &'a F) -> Self {
        let raw = f as *const F;
        Self {
            raw: unsafe {
                // references do not need dropping
                RawHeapFn::new(raw as *const (), drop_forget, Self::call_ref::<F>)
            },
            _phantom: Default::default(),
        }
    }

    /// Constructs a new `RefFn.
    #[inline]
    pub fn new_boxed<F: Fn<Args, Output = T>>(f: Box<F>) -> RefFn<'static, Args, T> {
        let raw = Box::into_raw(f);
        RefFn {
            raw: unsafe {
                // drop the Box
                RawHeapFn::new(raw as *const (), drop_boxed::<F>, Self::call_ref::<F>)
            },
            _phantom: Default::default(),
        }
    }

    /// Constructs a new `RefFn`.
    #[inline]
    pub fn new_arc<F: Fn<Args, Output = T>>(f: Arc<F>) -> RefFn<'static, Args, T> {
        let raw = Arc::into_raw(f);
        RefFn {
            raw: unsafe {
                // drop the Arc
                RawHeapFn::new(raw as *const (), drop_arc::<F>, Self::call_ref::<F>)
            },
            _phantom: Default::default(),
        }
    }

    fn call_ref<F: Fn<Args, Output = T>>(ptr: *const (), args: Args) -> T {
        let raw = ptr as *const F;
        let f = unsafe { &*raw };
        <F as Fn<Args>>::call(f, args)
    }
}

impl<'a, Args, T> FnOnce<Args> for RefFn<'a, Args, T> {
    type Output = T;

    #[inline]
    extern "rust-call" fn call_once(mut self, args: Args) -> Self::Output {
        <Self as FnMut<Args>>::call_mut(&mut self, args)
    }
}

impl<'a, Args, T> FnMut<Args> for RefFn<'a, Args, T> {
    #[inline]
    extern "rust-call" fn call_mut(&mut self, args: Args) -> Self::Output {
        <Self as Fn<Args>>::call(self, args)
    }
}

impl<'a, Args, T> Fn<Args> for RefFn<'a, Args, T> {
    #[inline]
    extern "rust-call" fn call(&self, args: Args) -> Self::Output {
        self.raw.call(args)
    }
}

impl<'a, Args, T> Debug for RefFn<'a, Args, T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        Debug::fmt(&self.raw, f)
    }
}

/// A [`FnOnce`] allocated on the heap.
pub struct HeapFnOnce<Args, T> {
    raw: RefFnOnce<'static, Args, T>,
}

impl<Args, T> HeapFnOnce<Args, T> {
    /// Constructs a new `HeapFnOnce` by boxing the callable.
    #[inline]
    pub fn new<F: FnOnce<Args, Output = T>>(f: F) -> Self {
        Self::new_boxed(Box::new(f))
    }

    /// Constructs a new `HeapFnOnce`.
    #[inline]
    pub fn new_boxed<F: FnOnce<Args, Output = T>>(f: Box<F>) -> Self {
        Self {
            raw: RefFnOnce::new_boxed(f),
        }
    }
}

impl<Args, T> FnOnce<Args> for HeapFnOnce<Args, T> {
    type Output = T;

    #[inline]
    extern "rust-call" fn call_once(self, args: Args) -> Self::Output {
        <RefFnOnce<'static, Args, T> as FnOnce<Args>>::call_once(self.raw, args)
    }
}

impl<Args, T> Debug for HeapFnOnce<Args, T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        Debug::fmt(&self.raw, f)
    }
}

/// A [`FnMut`] allocated on the heap.
pub struct HeapFnMut<Args, T> {
    raw: RefFnMut<'static, Args, T>,
}

impl<Args, T> HeapFnMut<Args, T> {
    /// Constructs a new `HeapFnMut` by boxing the callable.
    #[inline]
    pub fn new<F: FnMut<Args, Output = T>>(f: F) -> Self {
        Self::new_boxed(Box::new(f))
    }

    /// Constructs a new `HeapFnMut`.
    #[inline]
    pub fn new_boxed<F: FnMut<Args, Output = T>>(f: Box<F>) -> Self {
        Self {
            raw: RefFnMut::new_boxed(f),
        }
    }
}

impl<Args, T> FnOnce<Args> for HeapFnMut<Args, T> {
    type Output = T;

    #[inline]
    extern "rust-call" fn call_once(mut self, args: Args) -> Self::Output {
        <Self as FnMut<Args>>::call_mut(&mut self, args)
    }
}

impl<Args, T> FnMut<Args> for HeapFnMut<Args, T> {
    #[inline]
    extern "rust-call" fn call_mut(&mut self, args: Args) -> Self::Output {
        <RefFnMut<'static, Args, T> as FnMut<Args>>::call_mut(&mut self.raw, args)
    }
}

impl<Args, T> Debug for HeapFnMut<Args, T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        Debug::fmt(&self.raw, f)
    }
}

/// A [`Fn`] allocated on the heap.
pub struct HeapFn<Args, T> {
    raw: RefFn<'static, Args, T>,
}

impl<Args, T> HeapFn<Args, T> {
    /// Constructs a new `HeapFn` by boxing the callable.
    #[inline]
    pub fn new<F: Fn<Args, Output = T>>(f: F) -> Self {
        Self::new_boxed(Box::new(f))
    }

    /// Constructs a new `HeapFn`.
    #[inline]
    pub fn new_boxed<F: Fn<Args, Output = T>>(f: Box<F>) -> Self {
        Self {
            raw: RefFn::new_boxed(f),
        }
    }

    /// Constructs a new `HeapFn`.
    #[inline]
    pub fn new_arc<F: Fn<Args, Output = T>>(f: Arc<F>) -> Self {
        Self {
            raw: RefFn::new_arc(f),
        }
    }
}

impl<Args, T> FnOnce<Args> for HeapFn<Args, T> {
    type Output = T;

    #[inline]
    extern "rust-call" fn call_once(mut self, args: Args) -> Self::Output {
        <Self as FnMut<Args>>::call_mut(&mut self, args)
    }
}

impl<Args, T> FnMut<Args> for HeapFn<Args, T> {
    #[inline]
    extern "rust-call" fn call_mut(&mut self, args: Args) -> Self::Output {
        <Self as Fn<Args>>::call(self, args)
    }
}

impl<Args, T> Fn<Args> for HeapFn<Args, T> {
    #[inline]
    extern "rust-call" fn call(&self, args: Args) -> Self::Output {
        <RefFn<'static, Args, T> as Fn<Args>>::call(&self.raw, args)
    }
}

impl<Args, T> Debug for HeapFn<Args, T> {
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
