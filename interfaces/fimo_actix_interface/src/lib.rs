//! Definition of the `fimo-actix` interface.
#![feature(const_fn_fn_ptr_basics)]
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
use fimo_ffi_core::ArrayString;
use fimo_module_core::rust::ModuleObject;
use fimo_module_core::{
    rust::ModuleInterfaceArc, DynArc, DynArcBase, DynArcCaster, ModuleInterfaceDescriptor,
    ModulePtr,
};

/// Name of the interface.
pub const INTERFACE_NAME: &str = "fimo-actix";

/// Implemented interface version.
pub const INTERFACE_VERSION: Version = Version::new_long(0, 1, 0, ReleaseType::Unstable, 0);

/// Implements part of the [fimo_module_core::rust::ModuleInterface] vtable
/// for the `fimo-actix` interface.
#[macro_export]
macro_rules! fimo_actix_interface_impl {
    (id) => {
        "fimo::interface::actix"
    };
    (version) => {
        $crate::INTERFACE_VERSION
    };
    (to_ptr, $vtable: expr) => {
        fimo_module_core::ModulePtr::Slim(&$vtable as *const _ as *const u8)
    };
}

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

/// The fimo-actix interface.
pub struct FimoActix {
    inner: ModuleObject<FimoActixVTable>,
}

impl FimoActix {
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

    /// Splits the reference into a data- and vtable- pointer.
    #[inline]
    pub fn into_raw_parts(&self) -> (*const (), &'static FimoActixVTable) {
        self.inner.into_raw_parts()
    }

    /// Constructs a `*const FimoActix` from a data- and vtable- pointer.
    #[inline]
    pub fn from_raw_parts(
        data_address: *const (),
        vtable: &'static FimoActixVTable,
    ) -> *const Self {
        ModuleObject::from_raw_parts(data_address, vtable) as *const Self
    }
}

impl std::fmt::Debug for FimoActix {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "(FimoActix)")
    }
}

unsafe impl Send for FimoActix {}
unsafe impl Sync for FimoActix {}

/// VTable of the fimo-actix interface.
#[repr(C)]
#[allow(clippy::type_complexity)]
#[derive(PartialEq, Copy, Clone, Debug)]
pub struct FimoActixVTable {
    start: unsafe fn(*const ()) -> ServerStatus,
    stop: unsafe fn(*const ()) -> ServerStatus,
    pause: unsafe fn(*const ()) -> ServerStatus,
    resume: unsafe fn(*const ()) -> ServerStatus,
    restart: unsafe fn(*const ()) -> ServerStatus,
    get_server_status: unsafe fn(*const ()) -> ServerStatus,
    register_scope: unsafe fn(*const (), *const str, ScopeBuilder) -> Option<ScopeBuilderId>,
    unregister_scope: unsafe fn(*const (), ScopeBuilderId),
    register_callback: unsafe fn(*const (), Callback) -> CallbackId,
    unregister_callback: unsafe fn(*const (), CallbackId),
}

impl FimoActixVTable {
    /// Constructs a new VTable.
    #[allow(clippy::too_many_arguments, clippy::type_complexity)]
    pub const fn new(
        start: unsafe fn(*const ()) -> ServerStatus,
        stop: unsafe fn(*const ()) -> ServerStatus,
        pause: unsafe fn(*const ()) -> ServerStatus,
        resume: unsafe fn(*const ()) -> ServerStatus,
        restart: unsafe fn(*const ()) -> ServerStatus,
        get_server_status: unsafe fn(*const ()) -> ServerStatus,
        register_scope: unsafe fn(*const (), *const str, ScopeBuilder) -> Option<ScopeBuilderId>,
        unregister_scope: unsafe fn(*const (), ScopeBuilderId),
        register_callback: unsafe fn(*const (), Callback) -> CallbackId,
        unregister_callback: unsafe fn(*const (), CallbackId),
    ) -> Self {
        Self {
            start,
            stop,
            pause,
            resume,
            restart,
            get_server_status,
            register_scope,
            unregister_scope,
            register_callback,
            unregister_callback,
        }
    }
}

/// [`DynArc`] caster for [`FimoActix`].
#[derive(PartialEq, Copy, Clone, Debug)]
pub struct FimoActixCaster {
    vtable: &'static FimoActixVTable,
}

impl FimoActixCaster {
    /// Constructs a new `FimoActixCaster`.
    pub fn new(vtable: &'static FimoActixVTable) -> Self {
        Self { vtable }
    }
}

impl DynArcCaster<FimoActix> for FimoActixCaster {
    unsafe fn as_self_ptr<'a>(&self, base: *const (dyn DynArcBase + 'a)) -> *const FimoActix {
        let data = base as *const ();
        FimoActix::from_raw_parts(data, self.vtable)
    }
}

/// A RAII guard for scopes.
#[derive(Debug)]
pub struct ScopeBuilderGuard<'a> {
    id: ScopeBuilderId,
    interface: &'a FimoActix,
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
    interface: &'a FimoActix,
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
    data: *const (),
    func: fn(*const (), Scope) -> Scope,
    drop_in_place: fn(*const ()),
}

impl FnOnce<(Scope,)> for ScopeBuilder {
    type Output = Scope;

    #[inline]
    extern "rust-call" fn call_once(mut self, args: (Scope,)) -> Self::Output {
        self.call_mut(args)
    }
}

impl FnMut<(Scope,)> for ScopeBuilder {
    #[inline]
    extern "rust-call" fn call_mut(&mut self, args: (Scope,)) -> Self::Output {
        self.call_once(args)
    }
}

impl Fn<(Scope,)> for ScopeBuilder {
    #[inline]
    extern "rust-call" fn call(&self, args: (Scope,)) -> Self::Output {
        (self.func)(self.data, args.0)
    }
}

impl Drop for ScopeBuilder {
    fn drop(&mut self) {
        (self.drop_in_place)(self.data)
    }
}

impl<F: Fn(Scope) -> Scope + Send + Sync> From<Box<F>> for ScopeBuilder {
    fn from(data: Box<F>) -> Self {
        let data = Box::leak(data);
        let data_ptr = data as *const _ as *const _;
        let wrapped_builder = |ptr: *const (), scope: Scope| {
            let f = unsafe { &*(ptr as *const F) };
            f(scope)
        };
        let drop_func = |ptr: *const ()| {
            // safety: the pointer was added by the call to register and is therefore valid
            let boxed = unsafe { Box::from_raw(ptr as *const F as *mut F) };
            drop(boxed);
        };

        Self {
            data: data_ptr,
            func: wrapped_builder,
            drop_in_place: drop_func,
        }
    }
}

unsafe impl Send for ScopeBuilder {}
unsafe impl Sync for ScopeBuilder {}

/// A callback.
#[derive(Debug)]
pub struct Callback {
    data: *const (),
    func: fn(*const (), ServerEvent),
    drop_in_place: fn(*const ()),
}

impl FnOnce<(ServerEvent,)> for Callback {
    type Output = ();

    #[inline]
    extern "rust-call" fn call_once(self, args: (ServerEvent,)) -> Self::Output {
        (self.func)(self.data, args.0)
    }
}

impl FnMut<(ServerEvent,)> for Callback {
    #[inline]
    extern "rust-call" fn call_mut(&mut self, args: (ServerEvent,)) -> Self::Output {
        (self.func)(self.data, args.0)
    }
}

impl Drop for Callback {
    fn drop(&mut self) {
        (self.drop_in_place)(self.data)
    }
}

impl<F: FnMut(ServerEvent) + Send + Sync> From<Box<F>> for Callback {
    fn from(data: Box<F>) -> Self {
        let data = Box::leak(data);
        let data_ptr = data as *const _ as *const _;
        let callback_wrapper = |ptr: *const (), event: ServerEvent| {
            let f = unsafe { &mut *(ptr as *const F as *mut F) };
            f(event)
        };
        let drop_func = |ptr: *const ()| {
            // safety: the pointer was added by the call to register and is therefore valid
            let boxed = unsafe { Box::from_raw(ptr as *const F as *mut F) };
            drop(boxed);
        };

        Self {
            data: data_ptr,
            func: callback_wrapper,
            drop_in_place: drop_func,
        }
    }
}

unsafe impl Send for Callback {}
unsafe impl Sync for Callback {}

/// Casts an generic interface to a `fimo-actix` interface.
///
/// # Safety
///
/// This function is highly unsafe as the compiler can not check the
/// validity of the cast. The interface **must** be implemented using the
/// [`fimo_actix_interface_impl!{}`] macro.
pub unsafe fn cast_interface(
    interface: ModuleInterfaceArc,
) -> std::result::Result<DynArc<FimoActix, FimoActixCaster>, std::io::Error> {
    #[allow(unused_unsafe)]
    if interface.get_raw_type_id() != fimo_actix_interface_impl! {id} {
        return Err(std::io::Error::new(
            std::io::ErrorKind::Other,
            "Type mismatch",
        ));
    }

    if !INTERFACE_VERSION.is_compatible(&interface.get_version()) {
        return Err(std::io::Error::new(
            std::io::ErrorKind::Other,
            format!(
                "Versions incompatible. Requested {}, available {}.",
                INTERFACE_VERSION,
                interface.get_version()
            ),
        ));
    }

    match interface.get_raw_ptr() {
        ModulePtr::Slim(ptr) => {
            let vtable = &*(ptr as *const FimoActixVTable);
            let caster = FimoActixCaster::new(vtable);

            let (base, _) = ModuleInterfaceArc::into_inner(interface);
            Ok(DynArc::from_inner((base, caster)))
        }
        _ => Err(std::io::Error::new(
            std::io::ErrorKind::Other,
            "Pointer layout mismatch",
        )),
    }
}

/// Builds the [`ModuleInterfaceDescriptor`] for the interface.
pub fn build_interface_descriptor() -> ModuleInterfaceDescriptor {
    ModuleInterfaceDescriptor {
        name: unsafe { ArrayString::from_utf8_unchecked(INTERFACE_NAME.as_bytes()) },
        version: INTERFACE_VERSION,
        extensions: Default::default(),
    }
}
