//! Definition of the Rust `fimo-tasks` interface.
use fimo_ffi_core::ArrayString;
use fimo_module_core::{
    fimo_interface, fimo_vtable, FimoInterface, ModuleInterfaceDescriptor, SendSyncMarker,
};
use fimo_version_core::{ReleaseType, Version};

thread_local! {static RUNTIME: std::cell::Cell<Option<&'static TaskRuntime>> = std::cell::Cell::new(None)}

pub mod sync;

mod raw_task;
mod task;
mod task_runtime;

pub use raw_task::{RawTask, Result, TaskHandle, TaskInner, TaskStatus};
pub use task::{Task, TaskCompletionStatus};
pub use task_runtime::{NotifyFn, SpawnAllFn, TaskRuntime, TaskRuntimeInner, WaitOnFn, WorkerId};

fimo_interface! {
    /// Trait describing the `fimo-tasks` interface.
    ///
    /// Must be `Send` and `Sync`.
    pub struct FimoTasks<vtable = FimoTasksVTable> {
        name: "fimo::interfaces::tasks::fimo_tasks",
        version: Version::new_long(0, 1, 0, ReleaseType::Unstable, 0)
    }
}

impl FimoTasks {
    /// Extracts a reference to the task runtime.
    pub fn as_task_runtime(&self) -> &TaskRuntime {
        let (ptr, vtable) = self.into_raw_parts();
        unsafe { &*(vtable.as_task_runtime)(ptr) }
    }
}

fimo_vtable! {
    /// `FimoTasks` interface vtable.
    pub struct FimoTasksVTable<id = "fimo::interfaces::tasks::fimo_tasks", marker = SendSyncMarker> {
        /// Extracts a reference to the task runtime.
        pub as_task_runtime: fn(*const ()) -> *const TaskRuntime
    }
}

/// Returns whether a thread is a managed by a runtime.
pub fn is_worker() -> bool {
    RUNTIME.with(|runtime| runtime.get().is_some())
}

/// Returns a reference to the runtime that owns the worker.
///
/// The reference remains valid as long as the worker thread is kept alive.
///
/// # Panics
///
/// **Must** be run from within a task.
pub fn get_runtime() -> &'static TaskRuntime {
    RUNTIME.with(|runtime| runtime.get().unwrap())
}

/// Initializes the bindings the the runtime.
///
/// Calling this function enables the use of the types like `Task` and `Mutex`
/// from the worker threads.
///
/// # Panics
///
/// **Must** be run from within a task.
pub fn initialize_local_bindings(runtime: &TaskRuntime) {
    // SAFETY: from the perspective of a worker thread, it will be
    // outlived by the runtime that manages it. So it is sound to
    // extend the lifetime of the reference.
    let static_runtime = unsafe { &*(runtime as *const _) };

    runtime
        .spawn_all(move || RUNTIME.with(|r| r.set(Some(static_runtime))), &[])
        .join()
        .unwrap()
}

/// Builds the [`ModuleInterfaceDescriptor`] for the interface.
pub fn build_interface_descriptor() -> ModuleInterfaceDescriptor {
    ModuleInterfaceDescriptor {
        name: unsafe { ArrayString::from_utf8_unchecked(FimoTasks::NAME.as_bytes()) },
        version: FimoTasks::VERSION,
        extensions: Default::default(),
    }
}
