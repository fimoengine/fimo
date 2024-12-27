//! Async subsystem.

use crate::{
    bindings,
    context::{private::SealedContext, ContextView},
    error::{to_result, to_result_indirect, to_result_indirect_in_place, Error},
    ffi::{FFISharable, FFITransferable},
};
use std::{
    marker::PhantomData,
    mem::{ManuallyDrop, MaybeUninit},
    pin::Pin,
    task::{Context, Poll},
};

/// A handle to an event loop executing futures.
#[derive(Debug)]
#[repr(transparent)]
pub struct EventLoop(bindings::FimoAsyncEventLoop);

impl EventLoop {
    /// Initializes a new event loop.
    ///
    /// There can only be one event loop at a time, and it will keep
    /// the context alive until it completes its execution.
    pub fn new(ctx: ContextView<'_>) -> Result<Self, Error> {
        // Safety: Is always set.
        let f = unsafe { ctx.vtable().async_v0.start_event_loop.unwrap_unchecked() };

        // Safety: FFI call is safe.
        let event_loop = unsafe {
            to_result_indirect_in_place(|error, event_loop| {
                *error = f(ctx.data(), event_loop.as_mut_ptr());
            })?
        };

        Ok(Self(event_loop))
    }

    /// Utilize the current thread to complete all tasks in the event loop.
    ///
    /// The intended purpose of this function is to complete all remaining tasks
    /// before cleanup, as the context can not be destroyed until the queue is empty.
    /// Upon the completion of all tasks, the function will return to the caller.
    pub fn flush_with_current_thread(ctx: ContextView<'_>) -> Result<(), Error> {
        // Safety: Is always set.
        let f = unsafe { ctx.vtable().async_v0.run_to_completion.unwrap_unchecked() };

        // Safety: FFI call is safe.
        unsafe {
            to_result_indirect(|error| {
                *error = f(ctx.view().data());
            })
        }
    }

    /// Signals the event loop to complete the remaining jobs and exit afterward.
    ///
    /// The caller will block until the event loop has completed executing.
    pub fn join(self) {
        let this = ManuallyDrop::new(self);

        // Safety: The VTable is initialized.
        let f = unsafe { (*this.0.vtable).join.unwrap_unchecked() };

        // Safety: We own the value.
        unsafe { f(this.0.data) }
    }

    /// Signals the event loop to complete the remaining jobs and exit afterward.
    ///
    /// The caller will exit immediately.
    pub fn detach(self) {
        drop(self);
    }
}

// Safety: The EventLoop is `Send` and `Sync`.
unsafe impl Send for EventLoop {}

// Safety: The EventLoop is `Send` and `Sync`.
unsafe impl Sync for EventLoop {}

impl FFITransferable<bindings::FimoAsyncEventLoop> for EventLoop {
    fn into_ffi(self) -> bindings::FimoAsyncEventLoop {
        let this = ManuallyDrop::new(self);
        this.0
    }

    unsafe fn from_ffi(ffi: bindings::FimoAsyncEventLoop) -> Self {
        Self(ffi)
    }
}

impl Drop for EventLoop {
    fn drop(&mut self) {
        // Safety: The VTable is initialized.
        let f = unsafe { (*self.0.vtable).detach.unwrap_unchecked() };

        // Safety: We own the value.
        unsafe { f(self.0.data) }
    }
}

/// A non-owning reference to a waker.
#[derive(Debug, Copy, Clone)]
#[repr(transparent)]
pub struct WakerView<'a>(bindings::FimoAsyncWaker, PhantomData<&'a ()>);

impl WakerView<'_> {
    /// Acquires a strong reference to the waker.
    pub fn acquire(&self) -> Waker {
        // Safety: The VTable is initialized.
        let f = unsafe { (*self.0.vtable).acquire.unwrap_unchecked() };

        // Safety: Is sound, as we own the reference to the waker.
        let waker = unsafe { f(self.0.data) };
        Waker(WakerView(waker, PhantomData))
    }

    /// Notifies the task bound to the waker.
    pub fn wake_by_ref(&self) {
        // Safety: The VTable is initialized.
        let f = unsafe { (*self.0.vtable).wake.unwrap_unchecked() };

        // Safety: Is sound, as we own the reference to the waker.
        unsafe { f(self.0.data) };
    }
}

impl FFISharable<bindings::FimoAsyncWaker> for WakerView<'_> {
    type BorrowedView<'a> = WakerView<'a>;

    fn share_to_ffi(&self) -> bindings::FimoAsyncWaker {
        self.0
    }

    unsafe fn borrow_from_ffi<'a>(ffi: bindings::FimoAsyncWaker) -> Self::BorrowedView<'a> {
        WakerView(ffi, PhantomData)
    }
}

impl FFITransferable<bindings::FimoAsyncWaker> for WakerView<'_> {
    fn into_ffi(self) -> bindings::FimoAsyncWaker {
        self.0
    }

    unsafe fn from_ffi(ffi: bindings::FimoAsyncWaker) -> Self {
        Self(ffi, PhantomData)
    }
}

/// An owned reference to a waker.
#[derive(Debug)]
#[repr(transparent)]
pub struct Waker(WakerView<'static>);

impl Waker {
    /// Returns a view to the waker.
    pub fn view(&self) -> WakerView<'_> {
        self.0
    }

    /// Notifies the task bound to the waker and drop the waker.
    pub fn wake(self) {
        let this = ManuallyDrop::new(self);

        // Safety: The VTable is initialized.
        let f = unsafe { (*this.0 .0.vtable).wake_release.unwrap_unchecked() };

        // Safety: Is sound, as we own the reference to the waker.
        unsafe { f(this.0 .0.data) };
    }

    /// Notifies the task bound to the waker.
    pub fn wake_by_ref(&self) {
        self.0.wake_by_ref();
    }
}

impl Clone for Waker {
    fn clone(&self) -> Self {
        self.0.acquire()
    }
}

impl Drop for Waker {
    fn drop(&mut self) {
        // Safety: The VTable is initialized.
        let f = unsafe { (*self.0 .0.vtable).release.unwrap_unchecked() };

        // Safety: Is sound, as we own the reference to the waker.
        unsafe { f(self.0 .0.data) };
    }
}

impl FFISharable<bindings::FimoAsyncWaker> for Waker {
    type BorrowedView<'a> = WakerView<'a>;

    fn share_to_ffi(&self) -> bindings::FimoAsyncWaker {
        self.0 .0
    }

    unsafe fn borrow_from_ffi<'a>(ffi: bindings::FimoAsyncWaker) -> Self::BorrowedView<'a> {
        WakerView(ffi, PhantomData)
    }
}

impl FFITransferable<bindings::FimoAsyncWaker> for Waker {
    fn into_ffi(self) -> bindings::FimoAsyncWaker {
        let this = ManuallyDrop::new(self);
        this.0 .0
    }

    unsafe fn from_ffi(ffi: bindings::FimoAsyncWaker) -> Self {
        Self(WakerView(ffi, PhantomData))
    }
}

/// State of an enqueued future.
#[allow(dead_code)]
pub struct OpaqueState(*mut std::ffi::c_void);

// Safety:
unsafe impl Send for OpaqueState {}

// Safety:
unsafe impl Sync for OpaqueState {}

/// Type of futures that have been enqueued.
pub type EnqueuedFuture<R> = Future<OpaqueState, R>;

/// Result of a fallible future.
#[repr(C)]
#[derive(Debug)]
pub struct Fallible<T> {
    result: bindings::FimoResult,
    value: MaybeUninit<T>,
}

impl<T> Fallible<T> {
    /// Constructs a new instance from a value.
    pub fn new(value: T) -> Self {
        Self {
            result: Ok::<(), Error>(()).into_ffi(),
            value: MaybeUninit::new(value),
        }
    }

    /// Constructs a new instance from a result.
    pub fn new_result(res: Result<T, Error>) -> Self {
        match res {
            Ok(v) => Self {
                result: Ok::<(), Error>(()).into_ffi(),
                value: MaybeUninit::new(v),
            },
            Err(err) => Self {
                result: err.into_ffi(),
                value: MaybeUninit::uninit(),
            },
        }
    }

    /// Extracts the result.
    pub fn unwrap(self) -> Result<T, Error> {
        // Safety: Is initialized.
        match unsafe { to_result(self.result) } {
            // Safety: Must be initialized.
            Ok(_) => Ok(unsafe { self.value.assume_init() }),
            Err(e) => Err(e),
        }
    }
}

/// A future from the async subsystem.
#[repr(C)]
#[derive(Debug)]
pub struct Future<T, R> {
    state: ManuallyDrop<T>,
    poll_fn: unsafe extern "C" fn(*mut T, WakerView<'_>, *mut R) -> bool,
    cleanup_fn: Option<unsafe extern "C" fn(*mut T)>,
}

impl<T, R> Future<T, R> {
    /// Constructs a new future from a Rust future.
    pub fn new(fut: impl std::future::IntoFuture<Output = R, IntoFuture = T>) -> Self {
        fut.into_future_spec()
    }

    /// Creates a new future from its raw parts.
    ///
    /// # Safety
    ///
    /// The caller must follow the following contract.
    ///
    /// ## poll
    ///
    /// The `poll` function is called each time that the future is polled, where it may try to make
    /// some progress. On completion of the future, the function must return `true` and write its
    /// result into the address of the third parameter. A pending future must return `false` and not
    /// write any result. Further, the second parameter provides a reference to a waker. The caller
    /// must guarantee that the waker is used to notify that the task may make some progress.
    /// Failure of doing so will result in a deadlock. The caller may assume, that the function
    /// won't be called once the function signals a completion.
    ///
    /// ## cleanup
    ///
    /// Upon dropping of the future it will call the provided `cleanup` function. It is strongly
    /// encouraged to provide a `cleanup` function for all `T` that implement [`Drop`].
    pub unsafe fn from_raw_parts(
        state: T,
        poll: unsafe extern "C" fn(*mut T, WakerView<'_>, *mut R) -> bool,
        cleanup: Option<unsafe extern "C" fn(*mut T)>,
    ) -> Self {
        Self {
            state: ManuallyDrop::new(state),
            poll_fn: poll,
            cleanup_fn: cleanup,
        }
    }

    /// Enqueues the future on the subsystem.
    pub fn enqueue(self, ctx: ContextView<'_>) -> Result<EnqueuedFuture<R>, Error>
    where
        T: Send + 'static,
        R: Send + 'static,
    {
        extern "C" fn poll<T, R>(
            data: *mut std::ffi::c_void,
            waker: bindings::FimoAsyncWaker,
            result: *mut std::ffi::c_void,
        ) -> bool {
            // Safety: The pointer is valid.
            let (f, state) = unsafe {
                let this = data.cast::<Future<T, R>>();
                ((*this).poll_fn, &raw mut *(*this).state)
            };
            let waker = WakerView(waker, PhantomData);
            let result = result.cast::<R>();

            // Safety: Is safe by contract.
            unsafe { f(state, waker, result) }
        }

        extern "C" fn drop<T>(data: *mut std::ffi::c_void) {
            let data = data.cast::<T>();

            // Safety: We own the unique pointer.
            unsafe { std::ptr::drop_in_place(data) };
        }

        // Safety: The VTable is initialized.
        let f = unsafe { ctx.vtable().async_v0.future_enqueue.unwrap_unchecked() };

        let this = ManuallyDrop::new(self);

        // Safety: FFI call is safe.
        let fut = unsafe {
            to_result_indirect_in_place(|error, fut| {
                *error = f(
                    ctx.data(),
                    (&raw const this).cast(),
                    size_of::<Self>(),
                    align_of::<Self>(),
                    size_of::<R>(),
                    align_of::<R>(),
                    Some(poll::<T, R>),
                    if std::mem::needs_drop::<Self>() {
                        Some(drop::<Self>)
                    } else {
                        None
                    },
                    if std::mem::needs_drop::<R>() {
                        Some(drop::<R>)
                    } else {
                        None
                    },
                    fut.as_mut_ptr(),
                );
            })?
        };

        // Safety: They share the same layout.
        let fut = unsafe {
            std::mem::transmute::<bindings::FimoAsyncOpaqueFuture, Future<OpaqueState, R>>(fut)
        };
        Ok(fut)
    }

    fn poll_ffi(self: Pin<&mut Self>, waker: WakerView<'_>) -> Poll<R> {
        // Safety: The contract of the future ensures that this is safe.
        unsafe {
            let this = self.get_unchecked_mut();
            let mut result = MaybeUninit::uninit();
            if !(this.poll_fn)(&mut *this.state, waker, result.as_mut_ptr()) {
                Poll::Pending
            } else {
                Poll::Ready(result.assume_init())
            }
        }
    }
}

impl<T, R> std::future::Future for Future<T, R> {
    type Output = R;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        enum WakerWrapper<'a> {
            Ref(&'a std::task::Waker),
            Owned(std::task::Waker),
        }

        unsafe extern "C" fn ffi_waker_acquire(
            data: *mut std::ffi::c_void,
        ) -> bindings::FimoAsyncWaker {
            // Safety: We control the waker datatype.
            let wrapper: &WakerWrapper<'_> = unsafe { &*data.cast() };
            let clone = match wrapper {
                WakerWrapper::Ref(w) => (*w).clone(),
                WakerWrapper::Owned(w) => w.clone(),
            };
            let wrapper = Box::new(WakerWrapper::Owned(clone));
            bindings::FimoAsyncWaker {
                data: Box::into_raw(wrapper).cast(),
                vtable: &VTABLE,
            }
        }

        unsafe extern "C" fn ffi_waker_release(data: *mut std::ffi::c_void) {
            // Safety: We control the waker datatype.
            unsafe {
                let wrapper: *mut WakerWrapper<'_> = data.cast();
                assert!(matches!(*wrapper, WakerWrapper::Owned(_)));
                _ = Box::from_raw(wrapper);
            }
        }

        unsafe extern "C" fn ffi_waker_wake(data: *mut std::ffi::c_void) {
            // Safety: We control the waker datatype.
            unsafe {
                let wrapper: *mut WakerWrapper<'_> = data.cast();
                assert!(matches!(*wrapper, WakerWrapper::Owned(_)));
                let wrapper = Box::from_raw(wrapper);
                match *wrapper {
                    WakerWrapper::Ref(_) => std::hint::unreachable_unchecked(),
                    WakerWrapper::Owned(w) => w.wake(),
                }
            }
        }

        unsafe extern "C" fn ffi_waker_wake_by_ref(data: *mut std::ffi::c_void) {
            // Safety: We control the waker datatype.
            let wrapper: &WakerWrapper<'_> = unsafe { &*data.cast() };
            match wrapper {
                WakerWrapper::Ref(w) => w.wake_by_ref(),
                WakerWrapper::Owned(w) => w.wake_by_ref(),
            };
        }

        const VTABLE: bindings::FimoAsyncWakerVTableV0 = bindings::FimoAsyncWakerVTableV0 {
            acquire: Some(ffi_waker_acquire),
            release: Some(ffi_waker_release),
            wake_release: Some(ffi_waker_wake),
            wake: Some(ffi_waker_wake_by_ref),
            next: std::ptr::null(),
        };

        let waker = cx.waker();
        let wrapper = WakerWrapper::Ref(waker);
        let waker = WakerView(
            bindings::FimoAsyncWaker {
                data: std::ptr::from_ref(&wrapper).cast_mut().cast(),
                vtable: &VTABLE,
            },
            PhantomData,
        );
        self.poll_ffi(waker)
    }
}

impl<T, R> Drop for Future<T, R> {
    fn drop(&mut self) {
        if let Some(cleanup) = self.cleanup_fn {
            // Safety: We own the future.
            unsafe {
                cleanup(&mut *self.state);
            }
        }
    }
}

trait IntoFutureSpec<State, Output> {
    fn into_future_spec(self) -> Future<State, Output>;
}

impl<T: std::future::IntoFuture> IntoFutureSpec<T::IntoFuture, T::Output> for T {
    default fn into_future_spec(self) -> Future<T::IntoFuture, T::Output> {
        enum WakerWrapper<'a> {
            Ref(WakerView<'a>),
            Owned(Waker),
        }

        fn waker_clone(waker: *const ()) -> std::task::RawWaker {
            // Safety: We know that the type matches.
            let waker = unsafe { &*waker.cast::<WakerWrapper<'_>>() };
            let waker = match waker {
                WakerWrapper::Ref(w) => w.acquire(),
                WakerWrapper::Owned(w) => w.clone(),
            };
            let waker = Box::new(WakerWrapper::Owned(waker));
            std::task::RawWaker::new(Box::into_raw(waker).cast(), &VTABLE)
        }

        fn waker_wake(waker: *const ()) {
            // Safety: We know that the type matches.
            unsafe {
                let waker = waker.cast::<WakerWrapper<'_>>().cast_mut();
                assert!(matches!(*waker, WakerWrapper::Owned(_)));
                let waker = Box::from_raw(waker);
                match *waker {
                    WakerWrapper::Ref(_) => std::hint::unreachable_unchecked(),
                    WakerWrapper::Owned(w) => w.wake(),
                }
            }
        }

        fn waker_wake_by_ref(waker: *const ()) {
            // Safety: We know that the type matches.
            let waker = unsafe { &*waker.cast::<WakerWrapper<'_>>() };
            match waker {
                WakerWrapper::Ref(w) => w.wake_by_ref(),
                WakerWrapper::Owned(w) => w.wake_by_ref(),
            }
        }

        fn waker_drop(waker: *const ()) {
            // Safety: We know that the type matches.
            unsafe {
                let waker = waker.cast::<WakerWrapper<'_>>();
                if matches!(*waker, WakerWrapper::Ref(_)) {
                    return;
                }
                _ = Box::from_raw(waker.cast_mut());
            }
        }

        const VTABLE: std::task::RawWakerVTable =
            std::task::RawWakerVTable::new(waker_clone, waker_wake, waker_wake_by_ref, waker_drop);

        extern "C" fn poll<T, R>(data: *mut T, waker: WakerView<'_>, result: *mut R) -> bool
        where
            T: std::future::Future<Output = R>,
        {
            // Safety: The value is pinned by contract.
            let fut = unsafe { Pin::new_unchecked(&mut *data) };

            let waker = WakerWrapper::Ref(waker);
            let waker =
                // Safety:
                unsafe { std::task::Waker::new(std::ptr::from_ref(&waker).cast(), &VTABLE) };

            let mut cx = Context::from_waker(&waker);
            match <T as std::future::Future>::poll(fut, &mut cx) {
                Poll::Ready(v) => {
                    // Safety: The pointer is valid.
                    unsafe { result.write(v) };
                    true
                }
                Poll::Pending => false,
            }
        }

        extern "C" fn cleanup<T>(data: *mut T) {
            // Safety: We have a unique reference to the value.
            unsafe { std::ptr::drop_in_place(data) }
        }

        let fut = self.into_future();
        Future {
            state: ManuallyDrop::new(fut),
            poll_fn: poll::<T::IntoFuture, T::Output>,
            cleanup_fn: if std::mem::needs_drop::<T::IntoFuture>() {
                Some(cleanup::<T::IntoFuture>)
            } else {
                None
            },
        }
    }
}

impl<T, R> IntoFutureSpec<T, R> for Future<T, R> {
    fn into_future_spec(self) -> Future<T, R> {
        self
    }
}

/// A context that blocks the current thread until it is notified.
pub struct BlockingContext(bindings::FimoAsyncBlockingContext);

impl BlockingContext {
    /// Constructs a new blocking context.
    pub fn new(ctx: ContextView<'_>) -> Result<Self, Error> {
        // Safety: Is always set.
        let f = unsafe {
            ctx.vtable()
                .async_v0
                .context_new_blocking
                .unwrap_unchecked()
        };

        // Safety: FFI call is safe.
        let context = unsafe {
            to_result_indirect_in_place(|error, context| {
                *error = f(ctx.data(), context.as_mut_ptr());
            })?
        };
        Ok(Self(context))
    }

    /// Returns a reference to the contained waker.
    pub fn waker(&self) -> WakerView<'_> {
        // Safety: Is always set.
        let f = unsafe { (*self.0.vtable).waker_ref.unwrap_unchecked() };

        // Safety: FFI call is safe.
        let waker = unsafe { f(self.0.data) };
        WakerView(waker, PhantomData)
    }

    /// Blocks the current thread until the waker has been notified.
    pub fn wait(&self) {
        // Safety: Is always set.
        let f = unsafe { (*self.0.vtable).block_until_notified.unwrap_unchecked() };

        // Safety: FFI call is safe.
        unsafe { f(self.0.data) };
    }

    /// Block the thread until the future is ready.
    pub fn block_on<R>(&self, fut: impl std::future::IntoFuture<Output = R>) -> R {
        let waker = self.waker();
        let mut f = std::pin::pin!(Future::<_, R>::new(fut));
        loop {
            match f.as_mut().poll_ffi(waker) {
                Poll::Ready(v) => break v,
                Poll::Pending => self.wait(),
            }
        }
    }
}

impl Drop for BlockingContext {
    fn drop(&mut self) {
        // Safety: Is always set.
        let f = unsafe { (*self.0.vtable).release.unwrap_unchecked() };

        // Safety: FFI call is safe.
        unsafe { f(self.0.data) };
    }
}

impl FFITransferable<bindings::FimoAsyncBlockingContext> for BlockingContext {
    fn into_ffi(self) -> bindings::FimoAsyncBlockingContext {
        let this = ManuallyDrop::new(self);
        this.0
    }

    unsafe fn from_ffi(ffi: bindings::FimoAsyncBlockingContext) -> Self {
        Self(ffi)
    }
}

/// Block the thread until the future is ready.
pub fn block_on<F>(ctx: ContextView<'_>, fut: F) -> F::Output
where
    F: std::future::IntoFuture,
{
    let context = BlockingContext::new(ctx).expect("blocking context creation failed");
    context.block_on(fut)
}
