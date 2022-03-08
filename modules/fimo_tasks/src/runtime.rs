use crate::spin_wait::SpinWait;
use crate::task_manager::MsgData;
use crate::worker_pool::{yield_to_worker, WORKER};
use crate::TaskScheduler;
use fimo_ffi::ptr::IBaseExt;
use fimo_ffi::{DynObj, FfiFn, ObjectId};
use fimo_module::Error;
use fimo_tasks_int::raw::{IRawTask, WorkerId};
use fimo_tasks_int::runtime::{IRuntime, IRuntimeExt, IScheduler};
use log::{error, info, trace};
use parking_lot::Mutex;
use std::mem::MaybeUninit;
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
#[derive(Debug, ObjectId)]
#[fetch_vtable(uuid = "99bab93c-0db6-4979-a8fd-2b298df4e3ec", interfaces(IRuntime))]
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
    pub fn enter_scheduler<
        F: FnOnce(&mut TaskScheduler, Option<&DynObj<dyn IRawTask + '_>>) -> R,
        R,
    >(
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
                    let current =
                        unsafe { WORKER.get().unwrap_unchecked().current_task().unwrap() };

                    return f(&mut *scheduler, Some(current.as_raw()));
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
    pub fn yield_and_enter<
        F: FnOnce(&mut TaskScheduler, &DynObj<dyn IRawTask + '_>) -> R + Send,
        R: Send,
    >(
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
                let wrapper = move |s: &mut DynObj<dyn IScheduler + '_>,
                                    t: &DynObj<dyn IRawTask + '_>| unsafe {
                    trace!("Yielded to scheduler");
                    let scheduler = s.downcast_mut().unwrap_unchecked();
                    res.write(f(scheduler, t));
                    trace!("Resuming task");
                };
                let mut wrapper = MaybeUninit::new(wrapper);
                let wrapper = unsafe { FfiFn::new_value(&mut wrapper) };

                self.yield_and_enter_impl(wrapper);
            }

            trace!("Task resumed");
            unsafe { res.assume_init() }
        }
    }
}

impl Drop for Runtime {
    fn drop(&mut self) {
        info!("Shutting down task runtime");
        self.scheduler.lock().shutdown_worker_pool();
    }
}

impl IRuntime for Runtime {
    #[inline]
    fn worker_id(&self) -> Option<WorkerId> {
        self.worker_id()
    }

    #[inline]
    fn enter_scheduler_impl(
        &self,
        f: fimo_ffi::FfiFn<
            '_,
            dyn FnOnce(&mut DynObj<dyn IScheduler + '_>, Option<&DynObj<dyn IRawTask + '_>>) + '_,
        >,
    ) {
        if WORKER.get().is_none() {
            // outside worker.
            let mut scheduler = self.scheduler.lock();
            let s = fimo_ffi::ptr::coerce_obj_mut(&mut *scheduler);

            // call the function, then schedule the remaining tasks.
            f(s, None);
            scheduler.schedule_tasks();
        } else {
            let mut spin_wait = SpinWait::new();
            loop {
                // try to lock the scheduler.
                if let Some(mut scheduler) = self.scheduler.try_lock() {
                    let s = fimo_ffi::ptr::coerce_obj_mut(&mut *scheduler);
                    let current =
                        unsafe { WORKER.get().unwrap_unchecked().current_task().unwrap() };

                    // call the function, then schedule the remaining tasks.
                    f(s, Some(current.as_raw()));
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

    #[inline]
    fn yield_and_enter_impl(
        &self,
        f: fimo_ffi::FfiFn<
            '_,
            dyn FnOnce(&mut DynObj<dyn IScheduler + '_>, &DynObj<dyn IRawTask + '_>) + Send + '_,
        >,
    ) {
        let msg_data = MsgData::Yield { f: f.into_raw() };
        unsafe { yield_to_worker(msg_data) }
    }
}
