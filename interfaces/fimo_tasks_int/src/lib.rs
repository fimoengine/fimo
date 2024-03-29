//! Definition of the `fimo-tasks` interface.
#![warn(
    missing_docs,
    rust_2018_idioms,
    missing_debug_implementations,
    rustdoc::broken_intra_doc_links
)]
#![feature(strict_provenance)]
#![feature(negative_impls)]
#![feature(const_mut_refs)]
#![feature(thread_local)]
#![feature(try_blocks)]
#![feature(c_unwind)]
#![feature(unsize)]

use crate::raw::TaskHandle;
use crate::runtime::{IRuntime, IRuntimeExt};
use fimo_ffi::{interface, DynObj};
use fimo_module::context::IInterface;
use fimo_module::Queryable;

pub mod sync;

pub mod raw;
pub mod runtime;
pub mod task;

impl Queryable for dyn IFimoTasks + '_ {
    const NAME: &'static str = "fimo::interfaces::tasks";
    const CURRENT_VERSION: fimo_ffi::Version = fimo_ffi::Version::new_short(0, 1, 0);
    const EXTENSIONS: &'static [(Option<fimo_ffi::Version>, &'static str)] = &[];
}

interface! {
    #![interface_cfg(
        uuid = "e4a1d023-2261-4b8f-b237-9bf25c8c65ef",
    )]

    /// Type-erased `fimo-tasks` interface.
    pub frozen interface IFimoTasks: IInterface @ version("0.0") {
        /// Fetches a reference to the task runtime.
        fn runtime(&self) -> &DynObj<dyn IRuntime>;
    }
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
