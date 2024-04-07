//! Fimo context.

use alloc::boxed::Box;
use core::{marker::PhantomData, mem::ManuallyDrop, ops::Deref, pin::Pin};

use crate::{
    allocator::FimoAllocator,
    bindings,
    error::to_result,
    ffi::{FFISharable, FFITransferable},
    version::Version,
};

/// View of the context of the fimo library.
///
/// The context is a reference counted pointer, providing
/// access to the different subsystems of the fimo library,
/// like the tracing, or module subsystems. To avoid naming
/// conflicts, each subsystem is exposed through an own trait.
#[derive(Clone, Copy, Debug)]
pub struct ContextView<'a>(pub(crate) bindings::FimoContext, PhantomData<&'a ()>);

impl ContextView<'_> {
    /// Current `Context` version of the library.
    pub const CURRENT_VERSION: Version = Version::new_long(
        bindings::FIMO_VERSION_MAJOR,
        bindings::FIMO_VERSION_MINOR,
        bindings::FIMO_VERSION_PATCH,
        bindings::FIMO_VERSION_BUILD_NUMBER as u64,
    );

    /// Checks that the version of the `Context` is compatible.
    pub fn check_version(&self) -> crate::error::Result {
        // Safety: The call is safe, as we own a reference to the context.
        let error = unsafe { bindings::fimo_context_check_version(self.0) };
        to_result(error)
    }

    /// Promotes the context view to a context, by increasing the reference count.
    pub fn to_context(&self) -> Context {
        // Safety: We own a valid reference to the context.
        unsafe { bindings::fimo_context_acquire(self.into_ffi()) }
        Context(ContextView(self.0, PhantomData))
    }
}

// Safety: A `FimoContext` can be sent to other threads
unsafe impl Send for ContextView<'_> {}

// Safety: A `FimoContext` is basically an Arc, so it is sync.
unsafe impl Sync for ContextView<'_> {}

impl PartialEq for ContextView<'_> {
    fn eq(&self, other: &Self) -> bool {
        (self.0.data == other.0.data) && (self.0.vtable == other.0.vtable)
    }
}

impl Eq for ContextView<'_> {}

impl FFISharable<bindings::FimoContext> for ContextView<'_> {
    type BorrowedView<'a> = ContextView<'a>;

    fn share_to_ffi(&self) -> bindings::FimoContext {
        self.into_ffi()
    }

    unsafe fn borrow_from_ffi<'a>(ffi: bindings::FimoContext) -> Self::BorrowedView<'a> {
        // Safety: Is safe, as we are only a wrapper.
        unsafe { ContextView::from_ffi(ffi) }
    }
}

impl FFITransferable<bindings::FimoContext> for ContextView<'_> {
    fn into_ffi(self) -> bindings::FimoContext {
        self.0
    }

    unsafe fn from_ffi(ffi: bindings::FimoContext) -> Self {
        Self(ffi, PhantomData)
    }
}

/// Context of the fimo library.
///
/// The context is a reference counted pointer, providing
/// access to the different subsystems of the fimo library,
/// like the tracing, or module subsystems. To avoid naming
/// conflicts, each subsystem is exposed through an own trait.
#[repr(transparent)]
#[derive(Debug)]
pub struct Context(ContextView<'static>);

impl Context {
    /// Constructs a new `Context` with the default options.
    pub fn new() -> Result<Self, crate::error::Error> {
        // Safety: The context is either initialized, or the function returns an error.
        let ctx = unsafe {
            crate::error::to_result_indirect_in_place(|err, ctx| {
                *err = bindings::fimo_context_init(core::ptr::null_mut(), ctx.as_mut_ptr());
            })?
        };
        Ok(Self(ContextView(ctx, PhantomData)))
    }
}

impl Clone for Context {
    fn clone(&self) -> Self {
        self.to_context()
    }
}

impl Deref for Context {
    type Target = ContextView<'static>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Drop for Context {
    fn drop(&mut self) {
        // Safety: We own the reference to the context.
        unsafe { bindings::fimo_context_release(self.share_to_ffi()) }
    }
}

impl FFISharable<bindings::FimoContext> for Context {
    type BorrowedView<'a> = ContextView<'a>;

    fn share_to_ffi(&self) -> bindings::FimoContext {
        self.0.into_ffi()
    }

    unsafe fn borrow_from_ffi<'a>(ffi: bindings::FimoContext) -> Self::BorrowedView<'a> {
        // Safety: `ContextView` is a wrapper around a `FimoContext`.
        unsafe { ContextView::from_ffi(ffi) }
    }
}

impl FFITransferable<bindings::FimoContext> for Context {
    fn into_ffi(self) -> bindings::FimoContext {
        let this = ManuallyDrop::new(self);
        this.0 .0
    }

    unsafe fn from_ffi(ffi: bindings::FimoContext) -> Self {
        Self(ContextView(ffi, PhantomData))
    }
}

/// A builder for a [`Context`].
#[derive(Debug, Default)]
pub struct ContextBuilder<const N: usize = 0> {
    tracing: Option<Pin<Box<crate::tracing::Config<N>, FimoAllocator>>>,
}

impl<const N: usize> ContextBuilder<N> {
    /// Constructs a new builder.
    pub fn new() -> ContextBuilder<0> {
        ContextBuilder { tracing: None }
    }

    /// Adds a config for the tracing subsystem.
    pub fn with_tracing_config<const M: usize>(
        self,
        config: Pin<Box<crate::tracing::Config<M>, FimoAllocator>>,
    ) -> ContextBuilder<M> {
        ContextBuilder {
            tracing: Some(config),
        }
    }

    /// Builds the context.
    pub fn build(self) -> Result<Context, crate::error::Error> {
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
            // Safety: The context is either initialized, or the function returns an error.
            let ctx = unsafe {
                crate::error::to_result_indirect_in_place(|err, ctx| {
                    *err = bindings::fimo_context_init(options.as_mut_ptr(), ctx.as_mut_ptr());
                })?
            };
            Ok(Context(ContextView(ctx, PhantomData)))
        }
    }
}

pub(crate) mod private {
    use super::ContextView;
    use crate::{
        bindings,
        ffi::{FFISharable, FFITransferable},
    };

    pub trait SealedContext:
        FFISharable<bindings::FimoContext> + FFITransferable<bindings::FimoContext>
    {
    }
    impl SealedContext for ContextView<'_> {}
}
