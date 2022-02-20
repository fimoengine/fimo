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

use crate::raw::{IRawTask, TaskHandle, TaskScheduleStatus};
use crate::runtime::{get_runtime, is_worker, Builder, IRuntime};
use fimo_ffi::impl_vtable;
use fimo_ffi::marker::SendSyncMarker;
use fimo_module::{fimo_interface, fimo_vtable, is_object, Error, ReleaseType, Version};
use log::trace;
use std::cell::UnsafeCell;
use std::mem::MaybeUninit;
use std::ptr::NonNull;
use std::sync::{Condvar, Mutex};

pub mod raw;
pub mod runtime;

fimo_interface! {
    /// Type-erased `fimo-tasks` interface.
    #![vtable = IFimoTasksVTable]
    pub struct IFimoTasks {
        name: "fimo::interfaces::core::fimo_core",
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
    pub fn block_on<F: FnOnce(&'static IRuntime) -> R + Send, R: Send>(
        &self,
        f: F,
        wait_on: &[TaskHandle],
    ) -> Result<R, Error> {
        // if we are already owned by the runtime we can reuse the existing implementation.
        // otherwise we must implement the join functionality.
        if is_worker() {
            let runtime = unsafe { get_runtime() };
            runtime.block_on(move || unsafe { f(get_runtime()) }, wait_on)
        } else {
            trace!("Entering the runtime");

            // safety: the runtime will outlive the worker so we can extend it's lifetime.
            let runtime: &'static IRuntime = unsafe { &*(self.runtime() as *const _) };

            // task synchronisation is implemented with condition variables.
            struct CleanupData {
                condvar: Condvar,
                completed: Mutex<bool>,
            }
            is_object! { #![uuid(0x47c0e60c, 0x8cd9, 0x4dd1, 0x8b21, 0x79037a93278c)] CleanupData }
            fimo_vtable! {
                #[derive(Copy, Clone, Debug, Ord, PartialOrd, Eq, PartialEq, Hash)]
                #![marker = SendSyncMarker]
                #![uuid(0x90e5fb0b, 0x8186, 0x4593, 0xb79d, 0x262d145238f2)]
                struct CleanupVTable;
            }
            impl_vtable! { impl CleanupVTable => CleanupData {} }

            // initialize the condvar and hold the mutex until we try to join.
            let data = CleanupData {
                condvar: Default::default(),
                completed: Mutex::new(false),
            };
            let mut completed = data.completed.lock().unwrap();

            let f = move || f(runtime);
            let cleanup = move |data: Option<NonNull<CleanupData>>| unsafe {
                trace!("Notify owner thread");

                // after locking the mutex we are guaranteed that the owner
                // thread is waiting on the condvar, so we set the flag and notify it.
                let data: &CleanupData = data.unwrap().as_ref();
                let mut completed = data.completed.lock().unwrap();
                *completed = true;
                data.condvar.notify_all();
            };
            let join = |task: &IRawTask, value: &UnsafeCell<MaybeUninit<R>>| {
                trace!("Joining task on owner thread");

                // check if the task has been completed...
                while !*completed {
                    // ... if it isn't the case, wait.
                    completed = data.condvar.wait(completed).unwrap();
                }

                // by this point the task has finished so we can unregister it.
                runtime.enter_scheduler(|s, _| {
                    assert!(matches!(s.unregister_task(task), Ok(_)));
                });

                // safety: the task is unowned.
                unsafe {
                    let context = task.scheduler_context_mut();
                    match context.schedule_status() {
                        TaskScheduleStatus::Aborted => Err(context.take_panic_data()),
                        TaskScheduleStatus::Finished => Ok(value.get().read().assume_init()),
                        _ => unreachable!(),
                    }
                }
            };

            Builder::new().block_on_complex(f, cleanup, NonNull::from(&data), wait_on, join)
        }
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
