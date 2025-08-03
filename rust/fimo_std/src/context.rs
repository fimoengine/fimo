//! Fimo context.

use crate::{
    bindings,
    error::{self, AnyError, AnyResult},
    version::Version,
};
use core::panic;
use std::{marker::PhantomData, mem::MaybeUninit};

#[derive(Debug)]
pub enum Error {
    OperationFailed(AnyError),
    OperationFailedWithoutReport,
    Unknown(Status),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::OperationFailed(any_error) => write!(f, "{any_error}"),
            Self::OperationFailedWithoutReport => write!(f, "operation failed"),
            Self::Unknown(status) => write!(f, "unknown error ({})", status.0),
        }
    }
}

impl std::error::Error for Error {}

impl<T: error::private::Sealed + ?Sized> From<AnyError<T>> for Error
where
    AnyError: From<AnyError<T>>,
{
    fn from(value: AnyError<T>) -> Self {
        Self::OperationFailed(value.into())
    }
}

/// Status code.
///
/// All positive values are interpreted as successfull operations.
#[repr(transparent)]
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Status(pub i32);

impl Status {
    /// Operation completed successfully
    pub const OK: Self = Self(0);
    /// Operation failed with an unspecified error.
    ///
    /// The specific error may be accessible through the context.
    pub const FAILURE: Self = Self(-1);
    /// Operation failed with an unspecified error.
    ///
    /// No error was provided to the context.
    pub const FAILURE_NO_REPORT: Self = Self(-2);

    /// Checks if the status indicates a success.
    pub const fn is_ok(self) -> bool {
        self.0 >= 0
    }

    /// Checks if the status indicates an error.
    pub const fn is_error(self) -> bool {
        self.0 < 0
    }

    pub(crate) fn into_result(self) -> Result<(), Error> {
        if self.is_ok() {
            Ok(())
        } else {
            match self.0 {
                -1 => Err(Error::OperationFailed(take_result().unwrap_err())),
                -2 => Err(Error::OperationFailedWithoutReport),
                _ => Err(Error::Unknown(self)),
            }
        }
    }
}

/// Handle to the global functions implemented by the context.
///
/// Is not intended to be instantiated outside of the current module, as it may gain additional
/// fields without being considered a breaking change.
#[repr(C)]
#[derive(Debug)]
pub struct Handle {
    pub get_version: unsafe extern "C" fn() -> Version<'static>,
    pub core_v0: CoreVTableV0,
    pub tracing_v0: crate::tracing::VTableV0,
    pub modules_v0: crate::modules::VTableV0,
    pub tasks_v0: crate::tasks::VTableV0,
    _private: PhantomData<()>,
}

static COUNTER: std::sync::Mutex<usize> = std::sync::Mutex::new(0);
static HANDLE: std::sync::atomic::AtomicPtr<Handle> =
    std::sync::atomic::AtomicPtr::new(std::ptr::null_mut());

impl Handle {
    cfg_internal! {
        /// Constructs a new `Handle`.
        ///
        /// # Unstable
        ///
        /// **Note**: This is an [unstable API][unstable]. The public API of this type may break
        /// with any semver compatible release. See
        /// [the documentation on unstable features][unstable] for details.
        ///
        /// [unstable]: crate#unstable-features
        pub const fn new(
            get_version: extern "C" fn() -> Version<'static>,
            core_v0: CoreVTableV0,
            tracing_v0: crate::tracing::VTableV0,
            modules_v0: crate::modules::VTableV0,
            tasks_v0: crate::tasks::VTableV0
        ) -> Self {
            Self {
                get_version,
                core_v0,
                tracing_v0,
                modules_v0,
                tasks_v0,
                _private: PhantomData,
            }
        }
    }

    #[doc(hidden)]
    pub fn register(&'static self) {
        COUNTER.clear_poison();
        let mut counter = COUNTER.lock().unwrap();
        let old = HANDLE.load(std::sync::atomic::Ordering::Relaxed);
        if old.is_null() {
            debug_assert!(*counter == 0);
            HANDLE
                .compare_exchange(
                    old,
                    (&raw const *self).cast_mut(),
                    std::sync::atomic::Ordering::Release,
                    std::sync::atomic::Ordering::Relaxed,
                )
                .unwrap();
        } else {
            debug_assert!(*counter != 0);
        }
        *counter += 1;
    }

    #[doc(hidden)]
    pub unsafe fn unregister() {
        let mut counter = COUNTER.lock().unwrap();
        *counter -= 1;
        if *counter == 0 {
            HANDLE.store(std::ptr::null_mut(), std::sync::atomic::Ordering::Release);
        }
    }

    pub(crate) unsafe fn get_handle<'a>() -> &'a Self {
        let handle = HANDLE.load(std::sync::atomic::Ordering::Acquire);
        unsafe { handle.cast_const().as_ref().unwrap() }
    }
}

/// Core virtual function table of a [`ContextView`].
#[repr(C)]
#[derive(Debug)]
pub struct CoreVTableV0 {
    pub deinit: unsafe extern "C" fn(),
    pub has_error_result: unsafe extern "C" fn() -> bool,
    pub replace_result: unsafe extern "C" fn(new: AnyResult) -> AnyResult,
}

/// Current context version of the library.
pub const CURRENT_VERSION: Version<'static> = {
    let major = bindings::FIMO_CONTEXT_VERSION_MAJOR as usize;
    let minor = bindings::FIMO_CONTEXT_VERSION_MINOR as usize;
    let patch = bindings::FIMO_CONTEXT_VERSION_PATCH as usize;
    let pre = if bindings::FIMO_CONTEXT_VERSION_PRE.is_empty() {
        None
    } else {
        match bindings::FIMO_CONTEXT_VERSION_PRE.to_str() {
            Ok(x) => Some(x),
            Err(_) => panic!("invalid pre release string"),
        }
    };
    let build = if bindings::FIMO_CONTEXT_VERSION_BUILD.is_empty() {
        None
    } else {
        match bindings::FIMO_CONTEXT_VERSION_BUILD.to_str() {
            Ok(x) => Some(x),
            Err(_) => panic!("invalid build string"),
        }
    };

    Version::new_full(major, minor, patch, pre, build)
};

/// Returns the version of the instantiated context.
pub fn get_version() -> Version<'static> {
    let handle = unsafe { Handle::get_handle() };
    let f = handle.get_version;
    unsafe { f() }
}

/// Checks whether the context has an error stored for the current thread.
pub fn has_error_result() -> bool {
    let handle = unsafe { Handle::get_handle() };
    let f = handle.core_v0.has_error_result;
    unsafe { f() }
}

/// Replaces the thread-local result stored in the context with a new one.
///
/// The old result is returned.
pub fn replace_result(with: error::Result) -> error::Result {
    let handle = unsafe { Handle::get_handle() };
    let f = handle.core_v0.replace_result;
    unsafe { f(with.into()).into_result() }
}

/// Swaps out the thread-local result with the `Ok` result.
pub fn take_result() -> error::Result {
    replace_result(Ok(()))
}

/// Sets the thread-local result, destroying the old one.
pub fn set_result(result: error::Result) {
    _ = replace_result(result);
}

#[link(name = "fimo_std", kind = "static")]
unsafe extern "C" {
    fn fimo_context_init(
        options: *mut *const bindings::FimoConfigHead,
        ctx: &mut MaybeUninit<&'static Handle>,
    ) -> AnyResult;
}

/// Context of the fimo library.
#[repr(transparent)]
#[derive(Debug)]
pub struct Context {
    cleanup: bool,
}

impl Context {
    /// Initializes a new context with the default options.
    ///
    /// Only one context may exist at a time.
    pub fn new() -> Result<Self, AnyError> {
        let mut ctx = MaybeUninit::uninit();
        unsafe {
            fimo_context_init(std::ptr::null_mut(), &mut ctx).into_result()?;
            Handle::register(ctx.assume_init());
            Ok(Self { cleanup: false })
        }
    }

    /// Indicates that the context must clear all resources uppon dropping of this type.
    ///
    /// Dropping may block until all resources have been cleaned up.
    ///
    /// # Safety
    ///
    /// Since the context is a global, the caller must ensure that noone is using the
    /// context when the value is dropped.
    pub unsafe fn enable_cleanup(&mut self) {
        self.cleanup = true;
    }
}

impl Drop for Context {
    fn drop(&mut self) {
        if self.cleanup {
            unsafe {
                let handle = Handle::get_handle();
                let f = handle.core_v0.deinit;
                f();
                Handle::unregister();
            }
        }
    }
}

/// ID of the fimo std interface types.
#[repr(i32)]
#[non_exhaustive]
#[derive(Debug, Copy, Clone, PartialEq, Eq, Ord, PartialOrd, Hash)]
pub enum ConfigId {
    TracingConfig,
    ModuleConfig,
}

/// A builder for a [`Context`].
#[derive(Debug, Default)]
pub struct ContextBuilder<'a> {
    tracing: Option<crate::tracing::Config<'a>>,
    modules: Option<crate::modules::Config<'a>>,
}

impl<'a> ContextBuilder<'a> {
    /// Constructs a new builder.
    pub const fn new() -> Self {
        Self {
            tracing: None,
            modules: None,
        }
    }

    /// Adds a config for the tracing subsystem.
    pub const fn with_tracing_config(mut self, config: crate::tracing::Config<'a>) -> Self {
        self.tracing = Some(config);
        self
    }

    /// Adds a config for the module subsystem.
    pub const fn with_modules_config(mut self, config: crate::modules::Config<'a>) -> Self {
        self.modules = Some(config);
        self
    }

    /// Initializes the context.
    ///
    /// Only one context may exist at a time.
    pub fn build(self) -> Result<Context, AnyError> {
        let mut counter = 0;
        let mut options: [*const bindings::FimoConfigHead; 3] = [core::ptr::null(); 3];
        if let Some(cfg) = self.tracing.as_ref() {
            options[counter] = (&raw const *cfg).cast();
            counter += 1;
        }
        if let Some(cfg) = self.modules.as_ref() {
            options[counter] = (&raw const *cfg).cast();
            counter += 1;
        }

        if counter == 0 {
            Context::new()
        } else {
            let mut ctx = MaybeUninit::uninit();
            unsafe {
                fimo_context_init(options.as_mut_ptr(), &mut ctx).into_result()?;
                Handle::register(ctx.assume_init());
                Ok(Context { cleanup: false })
            }
        }
    }
}
