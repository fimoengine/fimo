//! Fimo context.

use crate::{
    bindings, error,
    error::{AnyError, AnyResult},
    ffi::{FFISharable, VTablePtr, View, Viewable},
    handle,
    version::Version,
};
use std::{
    marker::PhantomData,
    mem::{ManuallyDrop, MaybeUninit},
    panic::{RefUnwindSafe, UnwindSafe},
    pin::Pin,
};

handle!(pub handle ContextHandle: Send + Sync + UnwindSafe + RefUnwindSafe + Unpin);

/// Virtual function table of a [`ContextView`].
///
/// Adding fields to the vtable is not a breaking change.
#[repr(C)]
#[derive(Debug)]
pub struct VTable {
    pub header: VTableHeader,
    pub core_v0: CoreVTableV0,
    pub tracing_v0: crate::tracing::VTableV0,
    pub module_v0: crate::module::VTableV0,
    pub async_v0: crate::r#async::VTableV0,
}

/// Abi-stable header of the virtual function table of a [`ContextView`].
#[repr(C)]
#[derive(Debug)]
pub struct VTableHeader {
    pub check_version: unsafe extern "C" fn(handle: ContextHandle, version: &Version) -> AnyResult,
}

/// Core virtual function table of a [`ContextView`].
///
/// Adding fields to the vtable is a breaking change.
#[repr(C)]
#[derive(Debug)]
pub struct CoreVTableV0 {
    pub acquire: unsafe extern "C" fn(handle: ContextHandle),
    pub release: unsafe extern "C" fn(handle: ContextHandle),
}

/// View of the context of the fimo library.
///
/// The context is a reference counted pointer, providing access to the different subsystems of the
/// fimo library, like the tracing, or module subsystems. To avoid naming conflicts, each subsystem
/// is exposed through an own trait.
#[repr(C)]
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct ContextView<'a> {
    pub handle: ContextHandle,
    pub vtable: VTablePtr<VTable>,
    pub _phantom: PhantomData<&'a ContextHandle>,
}

impl ContextView<'_> {
    /// Current `Context` version of the library.
    pub const CURRENT_VERSION: Version = Version::new_long(
        bindings::FIMO_VERSION_MAJOR,
        bindings::FIMO_VERSION_MINOR,
        bindings::FIMO_VERSION_PATCH,
        bindings::FIMO_VERSION_BUILD_NUMBER as u64,
    );

    pub(crate) fn data(&self) -> *mut std::ffi::c_void {
        self.handle.as_ptr()
    }

    pub(crate) fn vtable(&self) -> &bindings::FimoContextVTable {
        unsafe { &*(&raw const *self.vtable).cast() }
    }

    /// Checks that the version of the `Context` is compatible.
    pub fn check_version(&self) -> error::Result {
        let f = self.vtable.header.check_version;
        unsafe { f(self.handle, &Self::CURRENT_VERSION).into() }
    }

    /// Promotes the context view to a context, by increasing the reference count.
    pub fn to_context(&self) -> Context {
        let f = self.vtable.core_v0.acquire;
        unsafe {
            f(self.handle);
        }
        Context(ContextView {
            handle: self.handle,
            vtable: self.vtable,
            _phantom: PhantomData,
        })
    }
}

impl View for ContextView<'_> {}

impl FFISharable<bindings::FimoContext> for ContextView<'_> {
    type BorrowedView<'a> = ContextView<'a>;

    fn share_to_ffi(&self) -> bindings::FimoContext {
        unsafe { std::mem::transmute::<ContextView<'_>, bindings::FimoContext>(*self) }
    }

    unsafe fn borrow_from_ffi<'a>(ffi: bindings::FimoContext) -> Self::BorrowedView<'a> {
        unsafe { std::mem::transmute::<bindings::FimoContext, ContextView<'_>>(ffi) }
    }
}

#[link(name = "fimo_std", kind = "static")]
unsafe extern "C" {
    #[allow(clashing_extern_declarations)]
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

impl<'a> Viewable<ContextView<'a>> for Context {
    fn view(&self) -> ContextView<'a> {
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

impl FFISharable<bindings::FimoContext> for Context {
    type BorrowedView<'a> = ContextView<'a>;

    fn share_to_ffi(&self) -> bindings::FimoContext {
        self.0.share_to_ffi()
    }

    unsafe fn borrow_from_ffi<'a>(ffi: bindings::FimoContext) -> Self::BorrowedView<'a> {
        // Safety: `ContextView` is a wrapper around a `FimoContext`.
        unsafe { ContextView::borrow_from_ffi(ffi) }
    }
}

/// A builder for a [`Context`].
#[derive(Debug, Default)]
pub struct ContextBuilder<const N: usize = 0> {
    tracing: Option<Pin<Box<crate::tracing::Config<N>>>>,
}

impl<const N: usize> ContextBuilder<N> {
    /// Constructs a new builder.
    pub fn new() -> ContextBuilder<0> {
        ContextBuilder { tracing: None }
    }

    /// Adds a config for the tracing subsystem.
    pub fn with_tracing_config<const M: usize>(
        self,
        config: Pin<Box<crate::tracing::Config<M>>>,
    ) -> ContextBuilder<M> {
        ContextBuilder {
            tracing: Some(config),
        }
    }

    /// Builds the context.
    pub fn build(self) -> Result<Context, AnyError> {
        let tracing = ManuallyDrop::new(self.tracing);

        let mut counter = 0;
        let mut options: [*const bindings::FimoBaseStructIn; 2] = [core::ptr::null(); 2];
        if let Some(tracing) = &*tracing {
            options[counter] = tracing.as_ffi_option_ptr();
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
