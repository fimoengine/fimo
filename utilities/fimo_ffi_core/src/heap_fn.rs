//! Heap allocated callables.
use std::fmt::{Debug, Formatter};
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

/// A [`FnOnce`] allocated on the heap.
pub struct HeapFnOnce<Args, T> {
    raw: RawHeapFn<Args, T>,
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
        let raw = Box::into_raw(f);
        Self {
            raw: unsafe {
                RawHeapFn::new(raw as *const (), drop_boxed::<F>, Self::call_boxed::<F>)
            },
        }
    }

    fn call_boxed<F: FnOnce<Args, Output = T>>(ptr: *const (), args: Args) -> T {
        let raw = ptr as *mut F;
        let boxed = unsafe { Box::from_raw(raw) };
        <Box<F> as FnOnce<Args>>::call_once(boxed, args)
    }
}

impl<Args, T> FnOnce<Args> for HeapFnOnce<Args, T> {
    type Output = T;

    #[inline]
    extern "rust-call" fn call_once(self, args: Args) -> Self::Output {
        let res = self.raw.call(args);

        // the internal value is dropped when called.
        std::mem::forget(self);
        res
    }
}

impl<Args, T> Debug for HeapFnOnce<Args, T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        Debug::fmt(&self.raw, f)
    }
}

/// A [`FnMut`] allocated on the heap.
pub struct HeapFnMut<Args, T> {
    raw: RawHeapFn<Args, T>,
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
        let raw = Box::into_raw(f);
        Self {
            raw: unsafe { RawHeapFn::new(raw as *const (), drop_boxed::<F>, Self::call_ref::<F>) },
        }
    }

    fn call_ref<F: FnMut<Args, Output = T>>(ptr: *const (), args: Args) -> T {
        let raw = ptr as *mut F;
        let f = unsafe { &mut *raw };
        <F as FnMut<Args>>::call_mut(f, args)
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
        self.raw.call(args)
    }
}

impl<Args, T> Debug for HeapFnMut<Args, T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        Debug::fmt(&self.raw, f)
    }
}

/// A [`Fn`] allocated on the heap.
pub struct HeapFn<Args, T> {
    raw: RawHeapFn<Args, T>,
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
        let raw = Box::into_raw(f);
        Self {
            raw: unsafe { RawHeapFn::new(raw as *const (), drop_boxed::<F>, Self::call_ref::<F>) },
        }
    }

    /// Constructs a new `HeapFn`.
    #[inline]
    pub fn new_arc<F: Fn<Args, Output = T>>(f: Arc<F>) -> Self {
        let raw = Arc::into_raw(f);
        Self {
            raw: unsafe { RawHeapFn::new(raw as *const (), drop_arc::<F>, Self::call_ref::<F>) },
        }
    }

    fn call_ref<F: Fn<Args, Output = T>>(ptr: *const (), args: Args) -> T {
        let raw = ptr as *const F;
        let f = unsafe { &*raw };
        <F as Fn<Args>>::call(f, args)
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
        self.raw.call(args)
    }
}

impl<Args, T> Debug for HeapFn<Args, T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        Debug::fmt(&self.raw, f)
    }
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
