//! Definition of the `fimo-actix` interface.
#![warn(
    missing_docs,
    rust_2018_idioms,
    missing_debug_implementations,
    rustdoc::broken_intra_doc_links
)]
#![feature(unsize)]

use actix_web::Scope;

pub use actix_web as actix;
use fimo_ffi::{interface, FfiFn};
use fimo_module::{context::IInterface, Queryable};

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

impl Queryable for dyn IFimoActix + '_ {
    const NAME: &'static str = "fimo::interfaces::actix";
    const CURRENT_VERSION: fimo_ffi::Version = fimo_ffi::Version::new_short(0, 1, 0);
    const EXTENSIONS: &'static [(Option<fimo_ffi::Version>, &'static str)] = &[];
}

interface! {
    #![interface_cfg(uuid = "85fa7a5f-959d-40c6-8d7a-ccd4dea654cf")]

    /// The fimo-actix interface.
    pub frozen interface IFimoActix : IInterface @ version("0.0") {
        /// Starts the server if it is not running.
        fn start(&self) -> ServerStatus;

        /// Stops the server if it is running.
        fn stop(&self) -> ServerStatus;

        /// Pauses the server if it is running.
        fn pause(&self) -> ServerStatus;

        /// Resumes the server if it is paused.
        fn resume(&self) -> ServerStatus;

        /// Restarts the server if it is running.
        fn restart(&self) -> ServerStatus;

        /// Fetches the status of the server.
        fn get_server_status(&self) -> ServerStatus;

        /// Registers a new scope for the server.
        ///
        /// The provided builder function is called, when the server is starting.
        /// The builder may not call into the interface.
        fn register_scope_raw(
            &self,
            path: &str,
            builder: ScopeBuilder,
        ) -> fimo_module::Result<ScopeBuilderId>;

        /// Unregisters a scope.
        fn unregister_scope_raw(&self, id: ScopeBuilderId) -> fimo_module::Result<()>;

        /// Registers a callback that is called every time the server status changes.
        ///
        /// The function may not call into the interface.
        fn register_callback_raw(&self, f: Callback) -> fimo_module::Result<CallbackId>;

        /// Unregisters a callback.
        fn unregister_callback_raw(&self, id: CallbackId) -> fimo_module::Result<()>;
    }
}

/// Extension trait for all implementors of [`IFimoActix`].
pub trait IFimoActixExt: IFimoActix {
    /// Registers a new scope for the server.
    ///
    /// The provided builder function is called, when the server is starting.
    /// The builder may not call into the interface.
    fn register_scope<F>(
        &self,
        path: &str,
        f: F,
    ) -> fimo_module::Result<ScopeBuilderGuard<'_, Self>>
    where
        F: Fn(Scope) -> Scope + Send + Sync + 'static,
    {
        let f = ScopeBuilder::r#box(Box::new(f));
        let id = self.register_scope_raw(path, f)?;
        Ok(ScopeBuilderGuard {
            id,
            interface: self,
        })
    }

    /// Unregisters a scope.
    fn unregister_scope(&self, guard: ScopeBuilderGuard<'_, Self>) -> fimo_module::Result<()> {
        let id = unsafe { std::ptr::read(&guard.id) };
        std::mem::forget(guard);
        self.unregister_scope_raw(id)
    }

    /// Registers a callback that is called every time the server status changes.
    ///
    /// The function may not call into the interface.
    fn register_callback<F>(&self, f: F) -> fimo_module::Result<CallbackGuard<'_, Self>>
    where
        F: FnMut(ServerEvent) + Send + Sync + 'static,
    {
        let f = Callback::r#box(Box::new(f));
        let id = self.register_callback_raw(f)?;
        Ok(CallbackGuard {
            id,
            interface: self,
        })
    }

    /// Unregisters a callback.
    fn unregister_callback(&self, guard: CallbackGuard<'_, Self>) -> fimo_module::Result<()> {
        let id = unsafe { std::ptr::read(&guard.id) };
        std::mem::forget(guard);
        self.unregister_callback_raw(id)
    }
}

impl<T: IFimoActix + ?Sized> IFimoActixExt for T {}

/// A RAII guard for scopes.
#[derive(Debug)]
pub struct ScopeBuilderGuard<'a, I: IFimoActix + ?Sized> {
    id: ScopeBuilderId,
    interface: &'a I,
}

impl<'a, I: IFimoActix + ?Sized> ScopeBuilderGuard<'a, I> {
    /// Constructs a new `ScopeBuilderGuard` from its raw parts.
    ///
    /// # Safety
    ///
    /// The id must be registered and owned by the caller.
    pub unsafe fn from_raw_parts(id: ScopeBuilderId, interface: &'a I) -> Self {
        Self { id, interface }
    }

    /// Splits the `ScopeBuilderGuard` into its constituents without dropping it.
    pub fn into_raw_parts(self) -> (ScopeBuilderId, &'a I) {
        let id = unsafe { std::ptr::read(&self.id) };
        let interface = self.interface;
        std::mem::forget(self);
        (id, interface)
    }
}

impl<I: IFimoActix + ?Sized> Drop for ScopeBuilderGuard<'_, I> {
    #[inline]
    fn drop(&mut self) {
        let id = unsafe { std::ptr::read(&self.id) };
        self.interface
            .unregister_scope_raw(id)
            .expect("could not drop the ScopeBuilderGuard");
    }
}

/// Id of a callback.
#[derive(Debug)]
#[repr(transparent)]
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
pub struct CallbackGuard<'a, I: IFimoActix + ?Sized> {
    id: CallbackId,
    interface: &'a I,
}

impl<'a, I: IFimoActix + ?Sized> CallbackGuard<'a, I> {
    /// Constructs a new `CallbackGuard` from its raw parts.
    ///
    /// # Safety
    ///
    /// The id must be registered and owned by the caller.
    pub unsafe fn from_raw_parts(id: CallbackId, interface: &'a I) -> Self {
        Self { id, interface }
    }

    /// Splits the `CallbackGuard` into its constituents without dropping it.
    pub fn into_raw_parts(self) -> (CallbackId, &'a I) {
        let id = unsafe { std::ptr::read(&self.id) };
        let interface = self.interface;
        std::mem::forget(self);
        (id, interface)
    }
}

impl<I: IFimoActix + ?Sized> Drop for CallbackGuard<'_, I> {
    #[inline]
    fn drop(&mut self) {
        let id = unsafe { std::ptr::read(&self.id) };
        self.interface
            .unregister_callback_raw(id)
            .expect("could not drop the CallbackGuard");
    }
}

/// Id of a callback.
#[derive(Debug)]
#[repr(transparent)]
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
pub type ScopeBuilder = FfiFn<'static, dyn Fn(Scope) -> Scope + Send + Sync>;

/// A callback.
pub type Callback = FfiFn<'static, dyn FnMut(ServerEvent) + Send + Sync>;
