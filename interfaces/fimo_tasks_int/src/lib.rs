//! Definition of the `fimo-tasks` interface.
#![warn(
    missing_docs,
    rust_2018_idioms,
    missing_debug_implementations,
    rustdoc::broken_intra_doc_links
)]
#![feature(const_ptr_offset_from)]
#![feature(negative_impls)]
#![feature(const_mut_refs)]
#![feature(thread_local)]
#![feature(try_blocks)]
#![feature(c_unwind)]
#![feature(unsize)]

use crate::raw::TaskHandle;
use crate::runtime::{IRuntime, IRuntimeExt};
use fimo_ffi::{interface, DynObj};
use fimo_module::{FimoInterface, IModuleInterface, IModuleInterfaceVTable, ReleaseType, Version};

pub mod raw;
pub mod runtime;
pub mod task;

/// Type-erased `fimo-tasks` interface.
#[interface(
    uuid = "e4a1d023-2261-4b8f-b237-9bf25c8c65ef",
    vtable = "IFimoTasksVTable",
    generate(IModuleInterfaceVTable)
)]
pub trait IFimoTasks: IModuleInterface {
    /// Fetches a reference to the task runtime.
    #[vtable_info(
        return_type = "*const DynObj<dyn IRuntime>",
        from_expr = "unsafe { &*res }"
    )]
    fn runtime(&self) -> &DynObj<dyn IRuntime>;
}

impl<'a> FimoInterface for dyn IFimoTasks + 'a {
    const NAME: &'static str = "fimo::interfaces::core::fimo_tasks";

    const VERSION: Version = Version::new_long(0, 1, 0, ReleaseType::Unstable, 0);

    const EXTENSIONS: &'static [&'static str] = &[];
}

/// Extension trait for implementations of [`IFimoTasks`].
pub trait IFimoTasksExt: IFimoTasks {
    /// Runs a task to completion on the task runtime.
    ///
    /// Blocks the current task until the new task has been completed.
    ///
    /// # Panics
    ///
    /// This function panics if the provided function panics.
    #[inline]
    #[track_caller]
    fn block_on<F: FnOnce(&DynObj<dyn IRuntime>) -> R + Send, R: Send>(
        &self,
        f: F,
        wait_on: &[TaskHandle],
    ) -> fimo_module::Result<R> {
        self.runtime().block_on_and_enter(f, wait_on)
    }
}

impl<T: IFimoTasks + ?Sized> IFimoTasksExt for T {}
