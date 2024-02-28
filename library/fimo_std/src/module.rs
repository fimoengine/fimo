//! Module backend.

use core::{ffi::CStr, ops::Deref};

use crate::{
    bindings,
    context::{private::SealedContext, ContextView},
    error::{self, to_result, to_result_indirect_in_place, Error},
    ffi::FFISharable,
};

mod loading_set;
mod module_export;
mod module_info;
mod parameter;
mod symbol;

pub use loading_set::*;
pub use module_export::*;
pub use module_info::*;
pub use parameter::*;
pub use symbol::*;

/// Definition of the module backend.
pub trait ModuleBackend<'ctx>: SealedContext<'ctx> {
    /// Locks the module context.
    ///
    /// The calling thread will wait until the lock can be acquired.
    fn lock_module_backend(&self) -> Result<ModuleBackendGuard<'ctx>, Error>;

    /// Unlocks the module context.
    ///
    /// # Safety
    ///
    /// The calling thread must own the lock to the module context.
    unsafe fn unlock_module_backend(&self) -> error::Result;
}

impl<'ctx> ModuleBackend<'ctx> for ContextView<'ctx> {
    fn lock_module_backend(&self) -> Result<ModuleBackendGuard<'ctx>, Error> {
        // Safety: FFI call is safe.
        let error = unsafe { bindings::fimo_module_lock(self.0) };
        to_result(error)?;

        Ok(ModuleBackendGuard(*self))
    }

    unsafe fn unlock_module_backend(&self) -> error::Result {
        // Safety: Is sound due to the contract of the function.
        let error = unsafe { bindings::fimo_module_unlock(self.0) };
        to_result(error)
    }
}

/// A guard providing access to the critical section
/// of the [`ModuleBackend`].
///
/// Forgetting to drop an instance of a `ModuleBackendGuard`
/// may result in deadlocks.
#[repr(transparent)]
pub struct ModuleBackendGuard<'ctx>(ContextView<'ctx>);

impl<'ctx> ModuleBackendGuard<'ctx> {
    /// Checks for the presence of a namespace in the module backend.
    ///
    /// A namespace exists, if at least one loaded module exports one symbol in said namespace.
    pub fn namespace_exists(&self, namespace: &CStr) -> Result<bool, Error> {
        // Safety: Either we get an error, or we initialize the module.
        unsafe {
            to_result_indirect_in_place(|error, exists| {
                *error = bindings::fimo_module_namespace_exists(
                    self.share_to_ffi(),
                    namespace.as_ptr(),
                    exists.as_mut_ptr(),
                );
            })
        }
    }
}

impl<'ctx> Deref for ModuleBackendGuard<'ctx> {
    type Target = ContextView<'ctx>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Drop for ModuleBackendGuard<'_> {
    fn drop(&mut self) {
        // Safety: We know that the are allowed to unlock the context, as
        // we locked it when we constructed the `ModuleContextGuard`.
        unsafe {
            self.0
                .unlock_module_backend()
                .expect("the context should be unlocked");
        }
    }
}
