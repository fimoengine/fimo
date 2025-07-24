//! Async subsystem.

use crate::{
    context::Handle,
    error::{AnyError, AnyResult, private},
    handle,
    module::symbols::{AssertSharable, Share},
    utils::{ConstNonNull, OpaqueHandle, View, Viewable},
};
use std::{
    marker::PhantomData,
    mem::{ManuallyDrop, MaybeUninit},
    pin::Pin,
    ptr::NonNull,
    task::{Context, Poll},
};

/// Virtual function table of the async subsystem.
#[repr(C)]
#[derive(Debug)]
pub struct VTableV0 {
    pub run_to_completion: unsafe extern "C" fn() -> AnyResult,
    pub start_event_loop:
        unsafe extern "C" fn(event_loop: &mut MaybeUninit<EventLoop>) -> AnyResult,
    pub new_blocking_context:
        unsafe extern "C" fn(context: &mut MaybeUninit<BlockingContext>) -> AnyResult,
    #[allow(clippy::type_complexity)]
    pub enqueue_future: unsafe extern "C" fn(
        data: Option<ConstNonNull<u8>>,
        data_len: usize,
        data_alignment: usize,
        result_len: usize,
        result_alignment: usize,
        poll: unsafe extern "C" fn(
            data: Option<NonNull<()>>,
            waker: WakerView<'_>,
            result: Option<NonNull<()>>,
        ) -> bool,
        drop_data: Option<unsafe extern "C" fn(data: Option<NonNull<()>>)>,
        drop_result: Option<unsafe extern "C" fn(data: Option<NonNull<()>>)>,
        future: &mut MaybeUninit<EnqueuedFuture<()>>,
    ) -> AnyResult,
}

handle!(pub handle EventLoopHandle: Send + Sync + Share + Unpin);

/// Virtual function table of an [`EventLoop`].
#[repr(C)]
#[derive(Debug)]
pub struct EventLoopVTable {
    pub join: unsafe extern "C" fn(handle: Option<EventLoopHandle>),
    pub detach: unsafe extern "C" fn(handle: Option<EventLoopHandle>),
    _private: PhantomData<()>,
}

impl EventLoopVTable {
    cfg_internal! {
        /// Constructs a new `EventLoopVTable`.
        ///
        /// # Unstable
        ///
        /// **Note**: This is an [unstable API][unstable]. The public API of this type may break
        /// with any semver compatible release. See
        /// [the documentation on unstable features][unstable] for details.
        ///
        /// [unstable]: crate#unstable-features
        pub const fn new(
            join: unsafe extern "C" fn(handle: Option<EventLoopHandle>),
            detach: unsafe extern "C" fn(handle: Option<EventLoopHandle>),
        ) -> Self {
            Self {
                join,
                detach,
                _private: PhantomData,
            }
        }
    }
}

/// A handle to an event loop executing futures.
#[repr(C)]
#[derive(Debug)]
pub struct EventLoop {
    pub handle: Option<EventLoopHandle>,
    pub vtable: &'static AssertSharable<EventLoopVTable>,
    _private: PhantomData<()>,
}

sa::assert_impl_all!(EventLoop: Send, Sync, Share, Unpin);

impl EventLoop {
    /// Initializes a new event loop.
    ///
    /// There can only be one event loop at a time, and it will keep the context alive until it
    /// completes its execution.
    pub fn new() -> Result<Self, AnyError> {
        let handle = unsafe { Handle::get_handle() };
        let f = handle.async_v0.start_event_loop;

        let mut out = MaybeUninit::uninit();
        unsafe {
            f(&mut out).into_result()?;
            Ok(out.assume_init())
        }
    }

    /// Utilize the current thread to complete all tasks in the event loop.
    ///
    /// The intended purpose of this function is to complete all remaining tasks before cleanup, as
    /// the context can not be destroyed until the queue is empty. Upon the completion of all tasks,
    /// the function will return to the caller.
    pub fn flush_with_current_thread() -> Result<(), AnyError> {
        let handle = unsafe { Handle::get_handle() };
        let f = handle.async_v0.run_to_completion;
        unsafe { f().into_result() }
    }

    /// Signals the event loop to complete the remaining jobs and exit afterward.
    ///
    /// The caller will block until the event loop has completed executing.
    pub fn join(self) {
        drop(self);
    }

    /// Signals the event loop to complete the remaining jobs and exit afterward.
    ///
    /// The caller will exit immediately.
    pub fn detach(self) {
        let this = ManuallyDrop::new(self);
        let f = this.vtable.detach;
        unsafe { f(this.handle) }
    }
}

impl Drop for EventLoop {
    fn drop(&mut self) {
        let f = self.vtable.join;
        unsafe { f(self.handle) }
    }
}

handle!(pub handle WakerHandle: Send + Sync + Share + Unpin);

/// Virtual function table of a [`WakerView`] and [`Waker`].
#[repr(C)]
#[derive(Debug)]
pub struct WakerVTable {
    pub acquire: unsafe extern "C" fn(handle: Option<WakerHandle>) -> Waker,
    pub release: unsafe extern "C" fn(handle: Option<WakerHandle>),
    pub wake_release: unsafe extern "C" fn(handle: Option<WakerHandle>),
    pub wake: unsafe extern "C" fn(handle: Option<WakerHandle>),
    pub next: Option<OpaqueHandle<dyn Send + Sync + Unpin>>,
}

/// A non-owning reference to a waker.
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct WakerView<'a> {
    pub handle: Option<WakerHandle>,
    pub vtable: &'a AssertSharable<WakerVTable>,
    _private: PhantomData<()>,
}

sa::assert_impl_all!(WakerView<'_>: Send, Sync, Share, Unpin);

impl WakerView<'_> {
    /// Acquires a strong reference to the waker.
    pub fn acquire(&self) -> Waker {
        let f = self.vtable.acquire;
        unsafe { f(self.handle) }
    }

    /// Notifies the task bound to the waker.
    pub fn wake_by_ref(&self) {
        let f = self.vtable.wake;
        unsafe { f(self.handle) }
    }
}

impl View for WakerView<'_> {}

/// An owned reference to a waker.
#[derive(Debug)]
#[repr(transparent)]
pub struct Waker(WakerView<'static>);

sa::assert_impl_all!(Waker: Send, Sync, Share);

impl Waker {
    /// Notifies the task bound to the waker and drop the waker.
    pub fn wake(self) {
        let this = ManuallyDrop::new(self);
        let f = this.0.vtable.wake_release;
        unsafe { f(this.0.handle) }
    }

    /// Notifies the task bound to the waker.
    pub fn wake_by_ref(&self) {
        self.0.wake_by_ref();
    }
}

impl<'a> Viewable<WakerView<'a>> for &'a Waker {
    fn view(self) -> WakerView<'a> {
        self.0
    }
}

impl Clone for Waker {
    fn clone(&self) -> Self {
        self.0.acquire()
    }
}

impl Drop for Waker {
    fn drop(&mut self) {
        let f = self.0.vtable.release;
        unsafe { f(self.0.handle) }
    }
}

handle!(pub handle EnqueuedHandle: Send + Sync);

/// Type of futures that have been enqueued.
pub type EnqueuedFuture<R> = Future<EnqueuedHandle, R>;

/// Result of a fallible future.
#[repr(C)]
#[derive(Debug)]
pub struct Fallible<T, E: private::Sealed + ?Sized = dyn Send + Sync + Share> {
    result: AnyResult<E>,
    value: MaybeUninit<T>,
}

impl<T, E: private::Sealed + ?Sized> Fallible<T, E> {
    /// Constructs a new instance from a value.
    pub fn new(value: T) -> Self {
        Self {
            result: Default::default(),
            value: MaybeUninit::new(value),
        }
    }

    /// Constructs a new instance from a result.
    pub fn new_result(res: Result<T, AnyError<E>>) -> Self {
        match res {
            Ok(v) => Self {
                result: Default::default(),
                value: MaybeUninit::new(v),
            },
            Err(err) => Self {
                result: err.into(),
                value: MaybeUninit::uninit(),
            },
        }
    }

    /// Extracts the result.
    pub fn unwrap(self) -> Result<T, AnyError<E>> {
        match self.result.into_result() {
            Ok(_) => Ok(unsafe { self.value.assume_init() }),
            Err(e) => Err(e),
        }
    }
}

unsafe impl<T: Send, E: private::Sealed + Send + ?Sized> Send for Fallible<T, E> {}
unsafe impl<T: Sync, E: private::Sealed + Sync + ?Sized> Sync for Fallible<T, E> {}

/// A future from the async subsystem.
#[repr(C)]
#[derive(Debug)]
pub struct Future<T, R> {
    pub state: ManuallyDrop<T>,
    pub poll_fn: unsafe extern "C" fn(*mut T, WakerView<'_>, *mut R) -> bool,
    pub cleanup_fn: Option<unsafe extern "C" fn(*mut T)>,
    _private: PhantomData<()>,
}

impl<T, R> Future<T, R> {
    /// Constructs a new future from a Rust future.
    pub fn new(fut: impl IntoFuture<Output = R, IntoFuture = T>) -> Self {
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
            _private: PhantomData,
        }
    }

    /// Enqueues the future on the subsystem.
    ///
    /// # Safety
    ///
    /// This function does not check that the future can be sent to another thread, or that it lives
    /// long enough.
    pub unsafe fn enqueue_unchecked(self) -> Result<EnqueuedFuture<R>, AnyError> {
        extern "C" fn poll<T, R>(
            data: Option<NonNull<()>>,
            waker: WakerView<'_>,
            result: Option<NonNull<()>>,
        ) -> bool {
            let data = data.map_or(std::ptr::null_mut(), |x| x.as_ptr());
            let result = result.map_or(std::ptr::null_mut(), |x| x.as_ptr());

            let (f, state) = unsafe {
                let this = data.cast::<Future<T, R>>();
                ((*this).poll_fn, (&raw mut (*this).state).cast())
            };
            let result = result.cast::<R>();

            unsafe { f(state, waker, result) }
        }

        extern "C" fn drop<T>(data: Option<NonNull<()>>) {
            let data = data
                .map_or(std::ptr::null_mut(), |x| x.as_ptr())
                .cast::<T>();
            unsafe { std::ptr::drop_in_place(data) };
        }

        let this = ManuallyDrop::new(self);
        let handle = unsafe { Handle::get_handle() };
        let f = handle.async_v0.enqueue_future;
        let mut out = MaybeUninit::uninit();

        unsafe {
            f(
                Some(ConstNonNull::new_unchecked(&raw const this).cast()),
                size_of::<Self>(),
                align_of::<Self>(),
                size_of::<R>(),
                align_of::<R>(),
                poll::<T, R>,
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
                &mut out,
            )
            .into_result()?;

            let out = out.assume_init();
            let out = std::mem::transmute::<EnqueuedFuture<()>, EnqueuedFuture<R>>(out);
            Ok(out)
        }
    }

    /// Enqueues the future on the subsystem.
    pub fn enqueue(self) -> Result<EnqueuedFuture<R>, AnyError>
    where
        T: Send + 'static,
        R: Send + 'static,
    {
        unsafe { self.enqueue_unchecked() }
    }

    fn poll_ffi(self: Pin<&mut Self>, waker: WakerView<'_>) -> Poll<R> {
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
            Ref(AssertSharable<&'a std::task::Waker>),
            Owned(AssertSharable<std::task::Waker>),
        }

        unsafe extern "C" fn ffi_waker_acquire(handle: Option<WakerHandle>) -> Waker {
            let handle = handle.map_or(std::ptr::null_mut(), |x| x.as_ptr::<WakerWrapper<'_>>());
            let wrapper = unsafe { &*handle };
            let clone = match wrapper {
                WakerWrapper::Ref(w) => unsafe { AssertSharable::new((**w).clone()) },
                WakerWrapper::Owned(w) => w.clone(),
            };
            let wrapper = Box::new(WakerWrapper::Owned(clone));
            Waker(WakerView {
                handle: unsafe { Some(WakerHandle::new_unchecked(Box::into_raw(wrapper))) },
                vtable: &VTABLE,
                _private: PhantomData,
            })
        }

        unsafe extern "C" fn ffi_waker_release(handle: Option<WakerHandle>) {
            unsafe {
                let handle =
                    handle.map_or(std::ptr::null_mut(), |x| x.as_ptr::<WakerWrapper<'_>>());
                assert!(matches!(*handle, WakerWrapper::Owned(_)));
                _ = Box::from_raw(handle);
            }
        }

        unsafe extern "C" fn ffi_waker_wake(handle: Option<WakerHandle>) {
            unsafe {
                let handle =
                    handle.map_or(std::ptr::null_mut(), |x| x.as_ptr::<WakerWrapper<'_>>());
                assert!(matches!(*handle, WakerWrapper::Owned(_)));
                let handle = Box::from_raw(handle);
                match *handle {
                    WakerWrapper::Ref(_) => std::hint::unreachable_unchecked(),
                    WakerWrapper::Owned(w) => w.into_inner().wake(),
                }
            }
        }

        unsafe extern "C" fn ffi_waker_wake_by_ref(handle: Option<WakerHandle>) {
            let handle = handle.map_or(std::ptr::null_mut(), |x| x.as_ptr::<WakerWrapper<'_>>());
            let handle = unsafe { &*handle };
            match handle {
                WakerWrapper::Ref(w) => w.wake_by_ref(),
                WakerWrapper::Owned(w) => w.wake_by_ref(),
            };
        }

        const VTABLE: AssertSharable<WakerVTable> = unsafe {
            AssertSharable::new(WakerVTable {
                acquire: ffi_waker_acquire,
                release: ffi_waker_release,
                wake_release: ffi_waker_wake,
                wake: ffi_waker_wake_by_ref,
                next: None,
            })
        };

        let waker = cx.waker();
        let wrapper = WakerWrapper::Ref(unsafe { AssertSharable::new(waker) });
        let waker = WakerView {
            handle: unsafe { Some(WakerHandle::new_unchecked((&raw const wrapper).cast_mut())) },
            vtable: &VTABLE,
            _private: PhantomData,
        };
        self.poll_ffi(waker)
    }
}

impl<T, R> Drop for Future<T, R> {
    fn drop(&mut self) {
        if let Some(cleanup) = self.cleanup_fn {
            unsafe {
                cleanup(&mut *self.state);
            }
        }
    }
}

trait IntoFutureSpec<State, Output> {
    fn into_future_spec(self) -> Future<State, Output>;
}

impl<T: IntoFuture> IntoFutureSpec<T::IntoFuture, T::Output> for T {
    default fn into_future_spec(self) -> Future<T::IntoFuture, T::Output> {
        enum WakerWrapper<'a> {
            Ref(WakerView<'a>),
            Owned(Waker),
        }

        fn waker_clone(waker: *const ()) -> std::task::RawWaker {
            let waker = unsafe { &*waker.cast::<WakerWrapper<'_>>() };
            let waker = match waker {
                WakerWrapper::Ref(w) => w.acquire(),
                WakerWrapper::Owned(w) => w.clone(),
            };
            let waker = Box::new(WakerWrapper::Owned(waker));
            std::task::RawWaker::new(Box::into_raw(waker).cast(), &VTABLE)
        }

        fn waker_wake(waker: *const ()) {
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
            let waker = unsafe { &*waker.cast::<WakerWrapper<'_>>() };
            match waker {
                WakerWrapper::Ref(w) => w.wake_by_ref(),
                WakerWrapper::Owned(w) => w.wake_by_ref(),
            }
        }

        fn waker_drop(waker: *const ()) {
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
            let fut = unsafe { Pin::new_unchecked(&mut *data) };

            let waker = WakerWrapper::Ref(waker);
            let waker =
                unsafe { std::task::Waker::new(std::ptr::from_ref(&waker).cast(), &VTABLE) };

            let mut cx = Context::from_waker(&waker);
            match <T as std::future::Future>::poll(fut, &mut cx) {
                Poll::Ready(v) => {
                    unsafe { result.write(v) };
                    true
                }
                Poll::Pending => false,
            }
        }

        extern "C" fn cleanup<T>(data: *mut T) {
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
            _private: PhantomData,
        }
    }
}

impl<T, R> IntoFutureSpec<T, R> for Future<T, R> {
    fn into_future_spec(self) -> Future<T, R> {
        self
    }
}

handle!(pub handle BlockingContextHandle: Send + Share);

/// Virtual function table of a [`BlockingContext`].
#[repr(C)]
#[derive(Debug)]
pub struct BlockingContextVTable {
    pub drop: unsafe extern "C" fn(handle: Option<BlockingContextHandle>),
    pub waker_ref:
        unsafe extern "C" fn(handle: Option<BlockingContextHandle>) -> WakerView<'static>,
    pub block_until_notified: unsafe extern "C" fn(handle: Option<BlockingContextHandle>),
    _private: PhantomData<()>,
}

impl BlockingContextVTable {
    cfg_internal! {
        /// Constructs a new `BlockingContextVTable`.
        ///
        /// # Unstable
        ///
        /// **Note**: This is an [unstable API][unstable]. The public API of this type may break
        /// with any semver compatible release. See
        /// [the documentation on unstable features][unstable] for details.
        ///
        /// [unstable]: crate#unstable-features
        pub const fn new(
            drop: unsafe extern "C" fn(handle: Option<BlockingContextHandle>),
            waker_ref: unsafe extern "C" fn(
                handle: Option<BlockingContextHandle>,
            ) -> WakerView<'static>,
            block_until_notified: unsafe extern "C" fn(handle: Option<BlockingContextHandle>),
        ) -> Self {
            Self {
                drop,
                waker_ref,
                block_until_notified,
                _private: PhantomData,
            }
        }
    }
}

/// A context that blocks the current thread until it is notified.
#[repr(C)]
pub struct BlockingContext {
    pub handle: Option<BlockingContextHandle>,
    pub vtable: &'static AssertSharable<BlockingContextVTable>,
    _private: PhantomData<()>,
}

sa::assert_impl_all!(BlockingContext: Send, Share);

impl BlockingContext {
    /// Constructs a new blocking context.
    pub fn new() -> Result<Self, AnyError> {
        let handle = unsafe { Handle::get_handle() };
        let f = handle.async_v0.new_blocking_context;

        let mut out = MaybeUninit::uninit();
        unsafe {
            f(&mut out).into_result()?;
            Ok(out.assume_init())
        }
    }

    /// Returns a reference to the contained waker.
    pub fn waker(&self) -> WakerView<'_> {
        let f = self.vtable.waker_ref;
        unsafe { f(self.handle) }
    }

    /// Blocks the current thread until the waker has been notified.
    pub fn wait(&self) {
        let f = self.vtable.block_until_notified;
        unsafe { f(self.handle) };
    }

    /// Block the thread until the future is ready.
    pub fn block_on<R>(&self, fut: impl IntoFuture<Output = R>) -> R {
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
        let f = self.vtable.drop;
        unsafe { f(self.handle) };
    }
}

/// Block the thread until the future is ready.
pub fn block_on<F>(fut: F) -> F::Output
where
    F: IntoFuture,
{
    let context = BlockingContext::new().expect("blocking context creation failed");
    context.block_on(fut)
}
