//! Definition of the `fimo-tasks` interface.
#![warn(
    missing_docs,
    rust_2018_idioms,
    missing_debug_implementations,
    rustdoc::broken_intra_doc_links
)]
#![feature(const_fn_fn_ptr_basics)]
#![feature(const_fn_trait_bound)]
#![feature(negative_impls)]
#![feature(thread_local)]

use crate::raw::TaskHandle;
use crate::runtime::IRuntime;
use fimo_ffi::marker::SendSyncMarker;
use fimo_module::{fimo_interface, fimo_vtable, Error, ReleaseType, Version};
use std::ptr::NonNull;

pub mod raw;
pub mod runtime;
pub mod task;

fimo_interface! {
    /// Type-erased `fimo-tasks` interface.
    #![vtable = IFimoTasksVTable]
    pub struct IFimoTasks {
        name: "fimo::interfaces::core::fimo_tasks",
        version: Version::new_long(0, 1, 0, ReleaseType::Unstable, 0),
    }
}

impl IFimoTasks {
    /// Fetches a reference to the task runtime.
    #[inline]
    pub fn runtime(&self) -> &IRuntime {
        let (ptr, vtable) = self.into_raw_parts();
        unsafe { (vtable.runtime)(ptr).as_ref() }
    }

    /// Runs a task to completion on the task runtime.
    ///
    /// Blocks the current task until the new task has been completed.
    ///
    /// # Panics
    ///
    /// This function panics if the provided function panics.
    #[inline]
    #[track_caller]
    pub fn block_on<F: FnOnce(&IRuntime) -> R + Send, R: Send>(
        &self,
        f: F,
        wait_on: &[TaskHandle],
    ) -> Result<R, Error> {
        self.runtime().block_on_and_enter(f, wait_on)
    }
}

fimo_vtable! {
    /// VTable of an [`IFimoTasks`].
    #[derive(Copy, Clone, Debug, Ord, PartialOrd, Eq, PartialEq, Hash)]
    #![marker = SendSyncMarker]
    #![uuid(0xe4a1d023, 0x2261, 0x4b8f, 0xb237, 0x9bf25c8c65ef)]
    pub struct IFimoTasksVTable {
        /// Fetches a reference to the task runtime.
        pub runtime: fn(*const ()) -> NonNull<IRuntime>
    }
}
