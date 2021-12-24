//! Definition of the Rust `fimo-tasks` interface.
use fimo_ffi_core::ArrayString;
use fimo_module_core::{
    rust::{ModuleInterface, ModuleInterfaceArc},
    ModuleInterfaceDescriptor, ModulePtr,
};
use fimo_version_core::{ReleaseType, Version};
use std::any::Any;
use std::sync::Arc;

/// Name of the interface.
pub const INTERFACE_NAME: &str = "fimo-tasks";

/// Implemented interface version.
pub const INTERFACE_VERSION: Version = Version::new_long(0, 1, 0, ReleaseType::Unstable, 0);

thread_local! {static RUNTIME: std::cell::Cell<Option<&'static TaskRuntime>> = std::cell::Cell::new(None)}

pub mod sync;

mod raw_task;
mod task;
mod task_runtime;

pub use raw_task::{RawTask, Result, TaskHandle, TaskInner, TaskStatus};
pub use task::{Task, TaskCompletionStatus};
pub use task_runtime::{NotifyFn, SpawnAllFn, TaskRuntime, TaskRuntimeInner, WaitOnFn, WorkerId};

/// Implements part of the [`ModuleInterface`] vtable for the `fimo-tasks` interface.
#[macro_export]
macro_rules! fimo_tasks_interface_impl {
    (id) => {
        "fimo::interface::tasks"
    };
    (version) => {
        $crate::rust::INTERFACE_VERSION
    };
    (to_ptr, $interface: expr) => {
        unsafe {
            fimo_module_core::ModulePtr::Fat(std::mem::transmute(
                $interface as &dyn $crate::rust::FimoTasks,
            ))
        }
    };
}

/// Trait describing the `fimo-tasks` interface.
pub trait FimoTasks: Send + Sync {
    /// Extracts the interface version.
    fn get_interface_version(&self) -> Version;

    /// Extracts a reference to an extension from the interface.
    fn find_extension(&self, extension: &str) -> Option<&(dyn Any + 'static)>;

    /// Extracts a mutable reference to an extension from the interface.
    fn find_extension_mut(&mut self, extension: &str) -> Option<&mut (dyn Any + 'static)>;

    /// Extracts a reference to the task runtime.
    fn as_task_runtime(&self) -> &TaskRuntime;

    /// Casts the interface to a `&(dyn Any + 'static)`.
    fn as_any(&self) -> &(dyn Any + 'static);

    /// Casts the interface to a `&mut (dyn Any + 'static)`.
    fn as_any_mut(&mut self) -> &mut (dyn Any + 'static);
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

/// Casts an generic interface to a `fimo-task` interface.
///
/// # Safety
///
/// This function is highly unsafe as the compiler can not check the
/// validity of the cast. The interface **must** be implemented using the
/// [`fimo_tasks_interface_impl!{}`] macro.
pub unsafe fn cast_interface(
    interface: ModuleInterfaceArc,
) -> std::result::Result<Arc<dyn FimoTasks>, std::io::Error> {
    static_assertions::assert_eq_size!(&ModuleInterface, &dyn FimoTasks, (*const u8, *const u8));
    static_assertions::assert_eq_align!(&ModuleInterface, &dyn FimoTasks,);

    #[allow(unused_unsafe)]
    if interface.get_raw_type_id() != fimo_tasks_interface_impl! {id} {
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
        ModulePtr::Fat(ptr) => {
            std::mem::forget(interface);
            let tasks_interface: &dyn FimoTasks = std::mem::transmute(ptr);
            Ok(Arc::from_raw(tasks_interface as *const _))
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
