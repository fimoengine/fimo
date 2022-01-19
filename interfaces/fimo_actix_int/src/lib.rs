//! Definition of the `fimo-actix` interface.
#![feature(const_fn_fn_ptr_basics)]
#![feature(const_fn_trait_bound)]
#![feature(unboxed_closures)]
#![feature(fn_traits)]
#![warn(
    missing_docs,
    rust_2018_idioms,
    missing_debug_implementations,
    rustdoc::broken_intra_doc_links
)]
use actix_web::Scope;
use fimo_version_core::{ReleaseType, Version};

pub use actix_web as actix;
use fimo_ffi::fn_wrapper::{HeapFn, HeapFnMut};
use fimo_ffi::marker::SendSyncMarker;
use fimo_module_core::{fimo_interface, fimo_vtable};

/// Status of the server.
#[derive(Copy, Clone, PartialEq, PartialOrd, Eq, Ord, Debug, Hash)]
pub enum ServerStatus {
    /// The server is not running.
    Stopped,
    /// The server has been paused.
    Paused,
    /// The server is running.
    Running,
}

/// Possible server events.
#[derive(Copy, Clone, PartialEq, PartialOrd, Eq, Ord, Debug, Hash)]
pub enum ServerEvent {
    /// The server is going to start.
    Starting,
    /// The server will be paused.
    Pausing,
    /// The server is going to be resumed.
    Resuming,
    /// The server will be terminated.
    Stopping,
    /// The server will restart.
    Restarting,
    /// The server has been started.
    Started,
    /// The server has been paused.
    Paused,
    /// The server has been resumed.
    Resumed,
    /// The server has been stopped.
    Stopped,
    /// The operation has been aborted.
    Aborted,
}

fimo_interface! {
    /// The fimo-actix interface.
    #![vtable = IFimoActixVTable]
    pub struct IFimoActix {
        name: "fimo::interfaces::actix::fimo_actix",
        version: Version::new_long(0, 1, 0, ReleaseType::Unstable, 0)
    }
}

impl IFimoActix {
    /// Starts the server if it is not running.
    #[inline]
    pub fn start(&self) -> ServerStatus {
        let (ptr, vtable) = self.into_raw_parts();
        unsafe { (vtable.start)(ptr) }
    }

    /// Stops the server if it is running.
    #[inline]
    pub fn stop(&self) -> ServerStatus {
        let (ptr, vtable) = self.into_raw_parts();
        unsafe { (vtable.stop)(ptr) }
    }

    /// Pauses the server if it is running.
    #[inline]
    pub fn pause(&self) -> ServerStatus {
        let (ptr, vtable) = self.into_raw_parts();
        unsafe { (vtable.pause)(ptr) }
    }

    /// Resumes the server if it is paused.
    #[inline]
    pub fn resume(&self) -> ServerStatus {
        let (ptr, vtable) = self.into_raw_parts();
        unsafe { (vtable.resume)(ptr) }
    }

    /// Restarts the server if it is running.
    #[inline]
    pub fn restart(&self) -> ServerStatus {
        let (ptr, vtable) = self.into_raw_parts();
        unsafe { (vtable.restart)(ptr) }
    }

    /// Fetches the status of the server.
    #[inline]
    pub fn get_server_status(&self) -> ServerStatus {
        let (ptr, vtable) = self.into_raw_parts();
        unsafe { (vtable.get_server_status)(ptr) }
    }

    /// Registers a new scope for the server.
    ///
    /// The provided builder function is called, when the server is starting.
    /// The builder may not call into the interface.
    #[inline]
    pub fn register_scope<'a, F: Fn(Scope) -> Scope + Send + Sync>(
        &'a self,
        path: &'a str,
        builder: Box<F>,
    ) -> Option<ScopeBuilderGuard<'a>> {
        let scope_builder = ScopeBuilder::from(builder);

        let path_ptr = path as *const _;
        let (ptr, vtable) = self.into_raw_parts();

        let id = unsafe { (vtable.register_scope)(ptr, path_ptr, scope_builder) };

        id.map(|id| ScopeBuilderGuard {
            id,
            interface: self,
        })
    }

    /// Unregisters a scope.
    ///
    /// Equivalent to calling `drop(guard)`.
    #[inline]
    pub fn unregister_scope(&self, guard: ScopeBuilderGuard<'_>) {
        drop(guard)
    }

    #[inline]
    fn unregister_scope_inner(&self, id: ScopeBuilderId) {
        // unregister builder.
        let (ptr, vtable) = self.into_raw_parts();
        unsafe { (vtable.unregister_scope)(ptr, id) }
    }

    /// Registers a callback that is called every time the server status changes.
    ///
    /// The function may not call into the interface.
    pub fn register_callback<F: FnMut(ServerEvent) + Send + Sync>(
        &self,
        f: Box<F>,
    ) -> CallbackGuard<'_> {
        let callback = Callback::from(f);
        let (ptr, vtable) = self.into_raw_parts();

        let id = unsafe { (vtable.register_callback)(ptr, callback) };

        CallbackGuard {
            id,
            interface: self,
        }
    }

    /// Unregisters a callback.
    ///
    /// Equivalent to calling `drop(guard)`.
    #[inline]
    pub fn unregister_callback(&self, guard: CallbackGuard<'_>) {
        drop(guard)
    }

    #[inline]
    fn unregister_callback_inner(&self, id: CallbackId) {
        // unregister callback
        let (ptr, vtable) = self.into_raw_parts();
        unsafe { (vtable.unregister_callback)(ptr, id) }
    }
}

fimo_vtable! {
    /// VTable of a [`IFimoActix`].
    #[allow(clippy::type_complexity)]
    #[derive(Copy, Clone, Debug, Ord, PartialOrd, Eq, PartialEq, Hash)]
    #![marker = SendSyncMarker]
    #![uuid(0x85fa7a5f, 0x959d, 0x40c6, 0x8d7a, 0xccd4dea654cf)]
    pub struct IFimoActixVTable {
        /// Starts the server if it is not running.
        pub start: unsafe fn(*const ()) -> ServerStatus,
        /// Stops the server if it is running.
        pub stop: unsafe fn(*const ()) -> ServerStatus,
        /// Pauses the server if it is running.
        pub pause: unsafe fn(*const ()) -> ServerStatus,
        /// Resumes the server if it is paused.
        pub resume: unsafe fn(*const ()) -> ServerStatus,
        /// Restarts the server if it is running.
        pub restart: unsafe fn(*const ()) -> ServerStatus,
        /// Fetches the status of the server.
        pub get_server_status: unsafe fn(*const ()) -> ServerStatus,
        /// Registers a new scope for the server.
        ///
        /// The provided builder function is called, when the server is starting.
        /// The builder may not call into the interface.
        pub register_scope: unsafe fn(*const (), *const str, ScopeBuilder) -> Option<ScopeBuilderId>,
        /// Unregisters a scope.
        pub unregister_scope: unsafe fn(*const (), ScopeBuilderId),
        /// Registers a callback that is called every time the server status changes.
        ///
        /// The function may not call into the interface.
        pub register_callback: unsafe fn(*const (), Callback) -> CallbackId,
        /// Unregisters a callback.
        pub unregister_callback: unsafe fn(*const (), CallbackId),
    }
}

/// A RAII guard for scopes.
#[derive(Debug)]
pub struct ScopeBuilderGuard<'a> {
    id: ScopeBuilderId,
    interface: &'a IFimoActix,
}

impl Drop for ScopeBuilderGuard<'_> {
    #[inline]
    fn drop(&mut self) {
        let id = unsafe { std::ptr::read(&self.id) };
        self.interface.unregister_scope_inner(id);
    }
}

/// Id of a callback.
#[derive(Debug)]
pub struct ScopeBuilderId(usize);

impl ScopeBuilderId {
    /// Constructs a new `ScopeBuilderId` from an `usize`.
    ///
    /// # Safety
    ///
    /// The caller must guarantee, that the id is valid.
    pub unsafe fn from_usize(id: usize) -> Self {
        Self(id)
    }
}

impl From<ScopeBuilderId> for usize {
    fn from(id: ScopeBuilderId) -> Self {
        id.0
    }
}

/// A RAII guard for callbacks.
#[derive(Debug)]
pub struct CallbackGuard<'a> {
    id: CallbackId,
    interface: &'a IFimoActix,
}

impl Drop for CallbackGuard<'_> {
    #[inline]
    fn drop(&mut self) {
        let id = unsafe { std::ptr::read(&self.id) };
        self.interface.unregister_callback_inner(id);
    }
}

/// Id of a callback.
#[derive(Debug)]
pub struct CallbackId(usize);

impl CallbackId {
    /// Constructs a new `CallbackId` from an `usize`.
    ///
    /// # Safety
    ///
    /// The caller must guarantee, that the id is valid.
    pub unsafe fn from_usize(id: usize) -> Self {
        Self(id)
    }
}

impl From<CallbackId> for usize {
    fn from(id: CallbackId) -> Self {
        id.0
    }
}

/// A scope builder.
#[derive(Debug)]
pub struct ScopeBuilder {
    inner: HeapFn<(Scope,), Scope>,
}

impl FnOnce<(Scope,)> for ScopeBuilder {
    type Output = Scope;

    #[inline]
    extern "rust-call" fn call_once(self, args: (Scope,)) -> Self::Output {
        self.inner.call_once(args)
    }
}

impl FnMut<(Scope,)> for ScopeBuilder {
    #[inline]
    extern "rust-call" fn call_mut(&mut self, args: (Scope,)) -> Self::Output {
        self.inner.call_mut(args)
    }
}

impl Fn<(Scope,)> for ScopeBuilder {
    #[inline]
    extern "rust-call" fn call(&self, args: (Scope,)) -> Self::Output {
        self.inner.call(args)
    }
}

impl<F: Fn(Scope) -> Scope + Send + Sync> From<Box<F>> for ScopeBuilder {
    fn from(b: Box<F>) -> Self {
        Self {
            inner: HeapFn::new_boxed(b),
        }
    }
}

unsafe impl Send for ScopeBuilder {}
unsafe impl Sync for ScopeBuilder {}

/// A callback.
#[derive(Debug)]
pub struct Callback {
    inner: HeapFnMut<(ServerEvent,), ()>,
}

impl FnOnce<(ServerEvent,)> for Callback {
    type Output = ();

    #[inline]
    extern "rust-call" fn call_once(self, args: (ServerEvent,)) -> Self::Output {
        self.inner.call_once(args)
    }
}

impl FnMut<(ServerEvent,)> for Callback {
    #[inline]
    extern "rust-call" fn call_mut(&mut self, args: (ServerEvent,)) -> Self::Output {
        self.inner.call_mut(args)
    }
}

impl<F: FnMut(ServerEvent) + Send + Sync> From<Box<F>> for Callback {
    fn from(b: Box<F>) -> Self {
        Self {
            inner: HeapFnMut::new_boxed(b),
        }
    }
}

unsafe impl Send for Callback {}
unsafe impl Sync for Callback {}
