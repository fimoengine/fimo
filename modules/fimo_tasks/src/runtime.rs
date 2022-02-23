use crate::spin_wait::SpinWait;
use crate::task_manager::MsgData;
use crate::worker_pool::{yield_to_worker, WORKER};
use crate::TaskScheduler;
use fimo_ffi::ffi_fn::RawFfiFn;
use fimo_ffi::object::{CoerceObject, ObjectWrapper};
use fimo_ffi::Optional;
use fimo_module::{impl_vtable, is_object, Error};
use fimo_tasks_int::raw::{IRawTask, WorkerId};
use fimo_tasks_int::runtime::{IRuntime, IRuntimeVTable, IScheduler};
use log::{error, info, trace};
use parking_lot::Mutex;
use std::mem::MaybeUninit;
use std::ops::Deref;
use std::sync::mpsc::channel;
use std::sync::Arc;

/// A builder for a [`Runtime`].
#[derive(Debug)]
pub struct Builder {
    stack_size: usize,
    allocated_stacks: usize,
    preferred_num_allocations: usize,
    workers: Option<usize>,
}

impl Builder {
    /// Default stack size for new tasks.
    ///
    /// Is currently set to 2 MiB.
    pub const DEFAULT_STACK_SIZE: usize = 1024 * 1024 * 2; // 2 MiB

    /// Number of tasks allocated at startup.
    pub const DEFAULT_PRE_ALLOCATED_TASKS: usize = 128;

    /// Threshold at which the stack of tasks is deallocated.
    pub const DEFAULT_TASK_FREE_THRESHOLD: usize = 256;

    /// Default number of worker created by the runtime.
    ///
    /// Defaults to the number of available threads on the core.
    pub const DEFAULT_NUM_WORKERS: Option<usize> = None;

    /// Creates a new builder with the default settings.
    #[inline]
    pub fn new() -> Self {
        Self {
            stack_size: Self::DEFAULT_STACK_SIZE,
            allocated_stacks: Self::DEFAULT_PRE_ALLOCATED_TASKS,
            preferred_num_allocations: Self::DEFAULT_TASK_FREE_THRESHOLD,
            workers: Self::DEFAULT_NUM_WORKERS,
        }
    }

    /// Changes the stack size for new tasks.
    #[inline]
    pub fn stack_size(mut self, size: usize) -> Self {
        self.stack_size = size;
        self
    }

    /// Changes the number of tasks to allocate at startup.
    #[inline]
    pub fn allocated_tasks(mut self, allocated: usize) -> Self {
        self.allocated_stacks = allocated;
        self
    }

    /// Changes the threshold at which the memory of a completed task is freed.
    #[inline]
    pub fn free_threshold(mut self, threshold: usize) -> Self {
        self.preferred_num_allocations = threshold;
        self
    }

    /// Changes the number of workers.
    ///
    /// Setting `None` creates a worker per available thread.
    #[inline]
    pub fn workers(mut self, workers: Option<usize>) -> Self {
        self.workers = workers;
        self
    }

    /// Builds the Runtime with the provided settings.
    #[inline]
    pub fn build(self) -> Result<Arc<Runtime>, Error> {
        Runtime::new(
            self.stack_size,
            self.allocated_stacks,
            self.preferred_num_allocations,
            self.workers,
        )
    }
}

impl Default for Builder {
    fn default() -> Self {
        Self::new()
    }
}

/// A runtime for running tasks.
#[derive(Debug)]
pub struct Runtime {
    scheduler: Arc<Mutex<TaskScheduler>>,
}

impl Runtime {
    fn new(
        stack_size: usize,
        pre_allocated: usize,
        preferred_num_allocations: usize,
        workers: Option<usize>,
    ) -> Result<Arc<Self>, Error> {
        let (msg_send, msg_rcv) = channel();

        let this = Self {
            scheduler: Arc::new(Mutex::new(TaskScheduler::new(
                stack_size,
                pre_allocated,
                preferred_num_allocations,
                msg_rcv,
            )?)),
        };

        let this = Arc::new(this);
        let weak = Arc::downgrade(&this);
        this.scheduler
            .lock()
            .start_workers(weak, msg_send, workers)?;

        Ok(this)
    }

    /// Retrieves the id of the current worker.
    #[inline]
    pub fn worker_id(&self) -> Option<WorkerId> {
        WORKER.get().map(|w| w.shared_data().id())
    }

    /// Acquires a reference to the scheduler.
    ///
    /// The task will block, until the scheduler can be acquired.
    ///
    /// # Deadlock
    ///
    /// Trying to access the scheduler by other means than
    /// the provided reference may result in a deadlock.
    #[inline]
    pub fn enter_scheduler<F: FnOnce(&mut TaskScheduler, Option<&IRawTask>) -> R, R>(
        &self,
        f: F,
    ) -> R {
        if WORKER.get().is_none() {
            // outside worker.
            let mut scheduler = self.scheduler.lock();

            // call the function, then schedule the remaining tasks.
            let res = f(&mut *scheduler, None);
            scheduler.schedule_tasks();
            res
        } else {
            let mut spin_wait = SpinWait::new();
            loop {
                // try to lock the scheduler.
                if let Some(mut scheduler) = self.scheduler.try_lock() {
                    let current = unsafe {
                        WORKER
                            .get()
                            .unwrap_unchecked()
                            .current_task()
                            .unwrap()
                            .as_i_raw()
                    };

                    return f(&mut *scheduler, Some(current));
                }

                // if it could not be locked, try spinning a few times.
                if spin_wait.spin(move || self.yield_now()) {
                    continue;
                }

                // else block until the scheduler is free.
                self.yield_and_enter(move |s, c| {
                    if let Err(e) = s.wait_on_scheduler(c) {
                        error!("Unable to wait on scheduler, error: {}", e)
                    }
                });
                spin_wait.reset();
            }
        }
    }

    #[inline]
    pub(crate) fn schedule_tasks(this: Arc<Self>, spin_wait: &mut SpinWait) -> Option<bool> {
        debug_assert!(WORKER.get().is_some());

        let s = this.scheduler.clone();
        drop(this);
        let scheduler = {
            // try to lock the scheduler.
            if let Some(scheduler) = s.try_lock() {
                Some(scheduler)
            } else if spin_wait.spin_yield_thread() {
                None
            } else {
                Some(s.lock())
            }
        };

        if let Some(mut scheduler) = scheduler {
            let stale = scheduler.schedule_tasks();
            if stale {
                let worker = unsafe { WORKER.get().unwrap_unchecked() };
                worker.wait_on_tasks(&mut scheduler);
            }

            Some(stale)
        } else {
            None
        }
    }

    #[inline]
    fn enter_scheduler_inner(
        &self,
        f: RawFfiFn<dyn FnOnce(&mut IScheduler, Optional<&IRawTask>) + '_>,
    ) {
        let f = unsafe { f.assume_valid() };

        if WORKER.get().is_none() {
            // outside worker.
            let mut scheduler = self.scheduler.lock();

            // call the function, then schedule the remaining tasks.
            f((&mut **scheduler).into(), Optional::None);
            scheduler.schedule_tasks();
        } else {
            let mut spin_wait = SpinWait::new();
            loop {
                // try to lock the scheduler.
                if let Some(mut scheduler) = self.scheduler.try_lock() {
                    let current = unsafe {
                        WORKER
                            .get()
                            .unwrap_unchecked()
                            .current_task()
                            .unwrap()
                            .as_i_raw()
                    };

                    // call the function, then schedule the remaining tasks.
                    f((&mut **scheduler).into(), Optional::Some(current.into()));
                    scheduler.schedule_tasks();
                    return;
                }

                // if it could not be locked, try spinning a few times.
                if spin_wait.spin(move || self.yield_now()) {
                    continue;
                }

                // else block until the scheduler is free.
                self.yield_and_enter(move |s, c| {
                    if let Err(e) = s.wait_on_scheduler(c) {
                        error!("Unable to wait on scheduler, error: {}", e)
                    }
                });
                spin_wait.reset();
            }
        }
    }

    /// Yields the current task to the runtime.
    ///
    /// Yields the current task to the runtime, allowing other tasks to be
    /// run on the current worker. On the next run of the scheduler it will call
    /// the provided function.
    ///
    /// # Panics
    ///
    /// Can only be called from a worker thread.
    #[inline]
    pub fn yield_and_enter<F: FnOnce(&mut TaskScheduler, &IRawTask) -> R + Send, R: Send>(
        &self,
        f: F,
    ) -> R {
        if WORKER.get().is_none() {
            error!("Tried yielding from outside of a worker thread");
            panic!("Yielding is only supported from a worker thread");
        } else {
            trace!("Yielding to scheduler");
            let mut res = MaybeUninit::uninit();

            {
                let res = &mut res;
                let wrapper = move |s: &mut IScheduler, t: &IRawTask| unsafe {
                    trace!("Yielded to scheduler");
                    let obj = IScheduler::as_object_mut(s);
                    let scheduler = obj.try_cast_obj_mut().unwrap_unchecked();
                    res.write(f(scheduler, t));
                    trace!("Resuming task");
                };
                let mut wrapper = MaybeUninit::new(wrapper);
                let wrapper = unsafe { RawFfiFn::new_value(&mut wrapper) };

                self.yield_and_enter_inner(wrapper);
            }

            trace!("Task resumed");
            unsafe { res.assume_init() }
        }
    }

    #[inline]
    fn yield_and_enter_inner(
        &self,
        f: RawFfiFn<dyn FnOnce(&mut IScheduler, &IRawTask) + Send + '_>,
    ) {
        let msg_data = MsgData::Yield { f };
        unsafe { yield_to_worker(msg_data) }
    }
}

is_object! { #![uuid(0x99bab93c, 0x0db6, 0x4979, 0xa8fd, 0x2b298df4e3ec)] Runtime }

impl Deref for Runtime {
    type Target = IRuntime;

    #[inline]
    fn deref(&self) -> &Self::Target {
        IRuntime::from_object(self.coerce_obj())
    }
}

impl Drop for Runtime {
    fn drop(&mut self) {
        info!("Shutting down task runtime");
        self.scheduler.lock().shutdown_worker_pool();
    }
}

impl_vtable! {
    impl IRuntimeVTable => Runtime {
        unsafe extern "C" fn worker_id(this: *const ()) -> Optional<WorkerId> {
            let this = &*(this as *const Runtime);
            Runtime::worker_id(this).into()
        }

        #[allow(improper_ctypes_definitions)]
        unsafe extern "C" fn enter_scheduler(
            this: *const (),
            f: RawFfiFn<dyn FnOnce(&mut IScheduler, Optional<&IRawTask>) + '_>
        ) {
            let this = &*(this as *const Runtime);
            Runtime::enter_scheduler_inner(this, f)
        }

        #[allow(improper_ctypes_definitions)]
        unsafe extern "C" fn yield_and_enter(
            this: *const (),
            f: RawFfiFn<dyn FnOnce(&mut IScheduler, &IRawTask) + Send + '_>
        ) {
            let this = &*(this as *const Runtime);
            Runtime::yield_and_enter_inner(this, f)
        }
    }
}
