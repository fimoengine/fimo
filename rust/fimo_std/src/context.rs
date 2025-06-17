//! Fimo context.

use crate::{
    bindings,
    error::{self, AnyError, AnyResult},
    handle,
    module::symbols::{AssertSharable, Share},
    utils::{View, Viewable},
    version::Version,
};
use core::panic;
use std::{marker::PhantomData, mem::MaybeUninit};

/// Status code.
///
/// All positive values are interpreted as successfull operations.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Status(pub i32);

impl Status {
    /// Operation completed successfully
    pub const OK: Self = Self(0);
    /// Operation failed with an unspecified error.
    ///
    /// The specific error may be accessible through the context.
    pub const FAILURE: Self = Self(-1);

    /// Checks if the status indicates a success.
    pub const fn is_ok(self) -> bool {
        self.0 >= 0
    }

    /// Checks if the status indicates an error.
    pub const fn is_error(self) -> bool {
        self.0 < 0
    }
}

handle!(pub handle ContextHandle: Send + Sync + Share);

/// Virtual function table of a [`ContextView`].
#[repr(C)]
#[derive(Debug)]
pub struct VTable {
    pub header: VTableHeader,
    pub core_v0: CoreVTableV0,
    pub tracing_v0: crate::tracing::VTableV0,
    pub module_v0: crate::module::VTableV0,
    pub async_v0: crate::r#async::VTableV0,
    _private: PhantomData<()>,
}

impl VTable {
    cfg_internal! {
        /// Constructs a new `VTable`.
        ///
        /// # Unstable
        ///
        /// **Note**: This is an [unstable API][unstable]. The public API of this type may break
        /// with any semver compatible release. See
        /// [the documentation on unstable features][unstable] for details.
        ///
        /// [unstable]: crate#unstable-features
        pub const fn new(
            header: VTableHeader,
            core_v0: CoreVTableV0,
            tracing_v0: crate::tracing::VTableV0,
            module_v0: crate::module::VTableV0,
            async_v0: crate::r#async::VTableV0
        ) -> Self {
            Self {
                header,
                core_v0,
                tracing_v0,
                module_v0,
                async_v0,
                _private: PhantomData,
            }
        }
    }
}

/// Abi-stable header of the virtual function table of a [`ContextView`].
#[repr(C)]
#[derive(Debug)]
pub struct VTableHeader {
    pub check_version:
        unsafe extern "C" fn(handle: ContextHandle, version: &Version<'_>) -> AnyResult,
}

/// Core virtual function table of a [`ContextView`].
#[repr(C)]
#[derive(Debug)]
pub struct CoreVTableV0 {
    pub acquire: unsafe extern "C" fn(handle: ContextHandle),
    pub release: unsafe extern "C" fn(handle: ContextHandle),
    pub has_error_result: unsafe extern "C" fn(handle: ContextHandle) -> bool,
    pub replace_result: unsafe extern "C" fn(handle: ContextHandle, new: AnyResult) -> AnyResult,
}

/// View of the context of the fimo library.
///
/// The context is a reference counted pointer, providing access to the different subsystems of the
/// fimo library, like the tracing, or module subsystems. To avoid naming conflicts, each subsystem
/// is exposed through an own trait.
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct ContextView<'a> {
    pub handle: ContextHandle,
    pub vtable: &'a AssertSharable<VTable>,
    _private: PhantomData<()>,
}

sa::assert_impl_all!(ContextView<'_>: Send, Sync);
sa::assert_impl_all!(ContextView<'static>: Share);

impl ContextView<'_> {
    /// Current `Context` version of the library.
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

    /// Checks that the version of the `Context` is compatible.
    pub fn check_version(&self) -> error::Result {
        let f = self.vtable.header.check_version;
        unsafe { f(self.handle, &Self::CURRENT_VERSION).into() }
    }

    /// Checks whether the context has an error stored for the current thread.
    pub fn has_error_result(&self) -> bool {
        let f = self.vtable.core_v0.has_error_result;
        unsafe { f(self.handle) }
    }

    /// Replaces the thread-local result stored in the context with a new one.
    ///
    /// The old result is returned.
    pub fn replace_result(&self, with: error::Result) -> error::Result {
        let f = self.vtable.core_v0.replace_result;
        unsafe { f(self.handle, with.into()).into_result() }
    }

    /// Swaps out the thread-local result with the `Ok` result.
    pub fn take_result(&self) -> error::Result {
        self.replace_result(Ok(()))
    }

    /// Sets the thread-local result, destroying the old one.
    pub fn set_result(&self, result: error::Result) {
        _ = self.replace_result(result);
    }

    /// Promotes the context view to a context, by increasing the reference count.
    pub fn to_context(&self) -> Context {
        let f = self.vtable.core_v0.acquire;
        unsafe {
            f(self.handle);
        }
        Context(ContextView {
            handle: self.handle,
            vtable: unsafe {
                std::mem::transmute::<&AssertSharable<_>, &'static AssertSharable<_>>(self.vtable)
            },
            _private: PhantomData,
        })
    }
}

impl View for ContextView<'_> {}

#[link(name = "fimo_std", kind = "static")]
unsafe extern "C" {
    fn fimo_context_init(
        options: *mut *const bindings::FimoBaseStructIn,
        ctx: &mut MaybeUninit<Context>,
    ) -> AnyResult;
}

/// Context of the fimo library.
///
/// The context is a reference counted pointer, providing access to the different subsystems of the
/// fimo library, like the tracing, or module subsystems. To avoid naming conflicts, each subsystem
/// is exposed through an own trait.
#[repr(transparent)]
#[derive(Debug)]
pub struct Context(ContextView<'static>);

sa::assert_impl_all!(Context: Send, Sync, Share);

impl Context {
    /// Constructs a new `Context` with the default options.
    pub fn new() -> Result<Self, AnyError> {
        let mut ctx = MaybeUninit::uninit();
        unsafe {
            fimo_context_init(std::ptr::null_mut(), &mut ctx).into_result()?;
            Ok(ctx.assume_init())
        }
    }
}

impl Clone for Context {
    fn clone(&self) -> Self {
        self.view().to_context()
    }
}

impl<'a> Viewable<ContextView<'a>> for &'a Context {
    fn view(self) -> ContextView<'a> {
        self.0
    }
}

impl Drop for Context {
    fn drop(&mut self) {
        unsafe {
            let view = self.view();
            let f = view.vtable.core_v0.release;
            f(view.handle);
        }
    }
}

/// ID of the fimo std interface types.
#[repr(i32)]
#[non_exhaustive]
#[derive(Debug, Copy, Clone, PartialEq, Eq, Ord, PartialOrd, Hash)]
pub enum TypeId {
    TracingConfig,
    ModuleConfig,
}

/// A builder for a [`Context`].
#[derive(Debug, Default)]
pub struct ContextBuilder<'a> {
    tracing: Option<crate::tracing::Config<'a>>,
    module: Option<crate::module::Config<'a>>,
}

impl<'a> ContextBuilder<'a> {
    /// Constructs a new builder.
    pub const fn new() -> Self {
        Self {
            tracing: None,
            module: None,
        }
    }

    /// Adds a config for the tracing subsystem.
    pub const fn with_tracing_config(mut self, config: crate::tracing::Config<'a>) -> Self {
        self.tracing = Some(config);
        self
    }

    /// Adds a config for the module subsystem.
    pub const fn with_module_config(mut self, config: crate::module::Config<'a>) -> Self {
        self.module = Some(config);
        self
    }

    /// Builds the context.
    pub fn build(self) -> Result<Context, AnyError> {
        let mut counter = 0;
        let mut options: [*const bindings::FimoBaseStructIn; 3] = [core::ptr::null(); 3];
        if let Some(cfg) = self.tracing.as_ref() {
            options[counter] = (&raw const *cfg).cast();
            counter += 1;
        }
        if let Some(cfg) = self.module.as_ref() {
            options[counter] = (&raw const *cfg).cast();
            counter += 1;
        }

        if counter == 0 {
            Context::new()
        } else {
            let mut ctx = MaybeUninit::uninit();
            unsafe {
                fimo_context_init(options.as_mut_ptr(), &mut ctx).into_result()?;
                Ok(ctx.assume_init())
            }
        }
    }
}
