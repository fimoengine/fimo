use crate::stack_allocator::StackAllocator;
use crate::task_manager::{AssertValidTask, Msg, MsgData, TaskManager};
use crate::worker_pool::WorkerPool;
use crate::Runtime;
use context::Context;
use fimo_ffi::{DynObj, ObjArc, ObjectId};
use fimo_module::Error;
use fimo_tasks_int::raw::{
    Builder, IRawTask, StatusRequest, TaskHandle, TaskRunStatus, TaskScheduleStatus, WorkerId,
};
use fimo_tasks_int::runtime::IScheduler;
use log::{debug, error, info, trace};
use std::cmp::Reverse;
use std::fmt::Debug;
use std::mem::ManuallyDrop;
use std::sync::mpsc::{Receiver, Sender};
use std::sync::Weak;
use std::time::SystemTime;

/// Task scheduler.
#[derive(ObjectId)]
#[fetch_vtable(uuid = "7f2cb683-26b4-46cb-a91d-3cbecf295ad8", interfaces(IScheduler))]
pub struct TaskScheduler {
    scheduler_task: ObjArc<DynObj<dyn IRawTask>>,
    stacks: ManuallyDrop<StackAllocator>,
    worker_pool: ManuallyDrop<WorkerPool>,
    task_manager: ManuallyDrop<TaskManager>,
}

unsafe impl Sync for TaskScheduler {}

impl TaskScheduler {
    pub(crate) fn new(
        stack_size: usize,
        pre_allocated: usize,
        preferred_num_allocations: usize,
        msg_receiver: Receiver<Msg<'static>>,
    ) -> Result<Self, Error> {
        trace!("Constructing scheduler");

        // construct a blocked task for the scheduler.
        let scheduler_task = Builder::new()
            .with_name("Task runtime scheduler".into())
            .blocked()
            .build(None, None);
        let scheduler_task = ObjArc::new(scheduler_task);
        let scheduler_task: ObjArc<DynObj<dyn IRawTask>> = ObjArc::coerce_obj(scheduler_task);

        let mut this = Self {
            scheduler_task: scheduler_task.clone(),
            stacks: ManuallyDrop::new(StackAllocator::new(
                stack_size,
                pre_allocated,
                preferred_num_allocations,
            )?),
            worker_pool: ManuallyDrop::new(WorkerPool::new()),
            task_manager: ManuallyDrop::new(TaskManager::new(msg_receiver)),
        };

        // register the scheduler task.
        unsafe { this.register_task(&scheduler_task, &[])? };

        Ok(this)
    }

    pub(crate) fn start_workers(
        &mut self,
        runtime: Weak<Runtime>,
        msg_sender: Sender<Msg<'static>>,
        workers: Option<usize>,
    ) -> Result<(), Error> {
        self.worker_pool.start_workers(runtime, msg_sender, workers)
    }

    /// Fetches the ids of all running workers.
    #[inline]
    pub fn worker_ids(&self) -> &[WorkerId] {
        self.worker_pool.workers()
    }

    /// Searches for a registered task.
    #[inline]
    pub fn find_task(&self, handle: TaskHandle) -> Result<&DynObj<dyn IRawTask + '_>, Error> {
        self.task_manager.find_task(handle)
    }

    /// Registers a task with the runtime for execution.
    ///
    /// # Safety
    ///
    /// The pointed to value must be kept alive until the runtime releases it.
    #[inline]
    pub unsafe fn register_task(
        &mut self,
        task: &DynObj<dyn IRawTask + '_>,
        deps: &[TaskHandle],
    ) -> Result<(), Error> {
        self.task_manager.register_task(task, deps)
    }

    /// Unregisters a task from the task runtime.
    pub fn unregister_task(&mut self, task: &DynObj<dyn IRawTask + '_>) -> Result<(), Error> {
        // we assert that we own the task.
        let task = unsafe { AssertValidTask::from_raw(task) };
        self.task_manager.unregister_task(task)
    }

    /// Registers a block for a task until the dependency completes.
    ///
    /// A task may not wait on itself.
    ///
    /// # Note
    ///
    /// Does not guarantee that the task will wait immediately if it is already scheduled.
    #[inline]
    pub fn wait_task_on(
        &mut self,
        task: &DynObj<dyn IRawTask + '_>,
        wait_on: &DynObj<dyn IRawTask + '_>,
    ) -> Result<(), Error> {
        // we assert that we own both tasks.
        let task = unsafe { AssertValidTask::from_raw(task) };
        let wait_on = unsafe { AssertValidTask::from_raw(wait_on) };
        self.task_manager.wait_task_on(task, wait_on)
    }

    #[inline]
    pub(crate) fn wait_on_scheduler(
        &mut self,
        task: &DynObj<dyn IRawTask + '_>,
    ) -> Result<(), Error> {
        let scheduler_task = self.scheduler_task.clone();
        self.wait_task_on(task, &scheduler_task)
    }

    #[inline]
    pub(crate) fn notify_one_scheduler_waiter(&mut self) -> Result<(), Error> {
        let scheduler_task = self.scheduler_task.clone();
        unsafe { self.notify_one(&scheduler_task)? };
        Ok(())
    }

    /// Wakes up one task.
    ///
    /// Wakes up the task with the highest priority, that is waiting on provided task to finish.
    /// Returns the number of remaining waiters, if the operation was successful and `Ok(None)`
    /// if there were no waiters.
    ///
    /// # Safety
    ///
    /// A waiting task may require that the task fully finishes before
    /// resuming execution. This function is mainly intended to be
    /// used for the implementation of condition variables.
    #[inline]
    pub unsafe fn notify_one(
        &mut self,
        task: &DynObj<dyn IRawTask + '_>,
    ) -> Result<Option<usize>, Error> {
        // we assert that we own the task.
        let task = AssertValidTask::from_raw(task);
        self.task_manager.notify_one(task)
    }

    /// Wakes up all tasks.
    ///
    /// Wakes up all waiting tasks. Returns the number of tasks that were woken up.
    ///
    /// # Safety
    ///
    /// A waiting task may require that the task fully finishes before
    /// resuming execution. This function is mainly intended to be
    /// used for the implementation of condition variables.
    #[inline]
    pub unsafe fn notify_all(&mut self, task: &DynObj<dyn IRawTask + '_>) -> Result<usize, Error> {
        // we assert that we own the task.
        let task = AssertValidTask::from_raw(task);
        self.task_manager.notify_all(task)
    }

    /// Unblocks a blocked task.
    #[inline]
    pub fn unblock_task(&mut self, task: &DynObj<dyn IRawTask + '_>) -> Result<(), Error> {
        // we assert that we own the task.
        let task = unsafe { AssertValidTask::from_raw(task) };
        self.task_manager.unblock_task(task)
    }

    fn process_finished(&mut self, task: AssertValidTask) {
        let context = task.context().borrow();

        trace!("Cleaning up task {}", context.handle());
        debug!("Task run status: {:?}", context.run_status());
        debug!("Task schedule status: {:?}", context.schedule_status());
        debug!("Number of waiters: {}", unsafe {
            context.scheduler_data().waiters.len()
        });
        debug!("Number of dependencies: {}", unsafe {
            context.scheduler_data().dependencies.len()
        });

        // task should not be running or have any dependencies at this point
        debug_assert_eq!(context.run_status(), TaskRunStatus::Idle);
        debug_assert!(matches!(
            context.schedule_status(),
            TaskScheduleStatus::Aborted | TaskScheduleStatus::Finished
        ));
        debug_assert!(unsafe { context.scheduler_data().dependencies.is_empty() });

        trace!("Notify waiters of {}", context.handle());
        drop(context);
        unsafe {
            self.task_manager
                .notify_all(task.clone())
                .expect("Invalid task")
        };
        let mut context = task.context().borrow_mut();

        trace!("Free task slot of {}", context.handle());
        if let Some(slot) = unsafe { context.scheduler_data_mut().slot.take() } {
            if let Err(e) = self.stacks.deallocate(slot) {
                error!("Could not free task slot, error: {}", e)
            }
        }

        trace!("Internal task cleanup");
        context.cleanup();

        trace!("Marking task {} as completed", context.handle());
        unsafe {
            context.set_run_status(TaskRunStatus::Completed);
        }
    }

    fn process_msg(&mut self) -> bool {
        trace!("Processing scheduler messages");

        let msgs = self.task_manager.take_messages();
        let stale = msgs.is_empty();

        trace!("Processing {} scheduler messages", msgs.len());
        for Msg { task, data } in msgs {
            let mut context = task.context().borrow_mut();
            let scheduler_data = unsafe { context.scheduler_data() };

            trace!(
                "Message for task-id {}, name: {:?}",
                context.handle(),
                task.resolved_name()
            );
            trace!("Message type: {}", data.msg_type());

            // cache the status of the task.
            // this should be alright, as they are only allowed to be modified by the scheduler.
            let run_status = context.run_status();
            let schedule_status = context.schedule_status();

            debug!("Task run status: {:?}", run_status);
            debug!("Task schedule status: {:?}", schedule_status);
            debug!("Number of waiters: {}", scheduler_data.waiters.len());
            debug!(
                "Number of dependencies: {}",
                scheduler_data.dependencies.len()
            );

            // task should not be running
            debug_assert_eq!(run_status, TaskRunStatus::Idle);
            debug_assert_eq!(schedule_status, TaskScheduleStatus::Processing);

            // set the processing flag.
            let scheduler_data = unsafe { context.scheduler_data_mut() };
            scheduler_data.processing = true;
            let scheduler_data = unsafe { context.scheduler_data() };

            // clear and fetch the status change request and apply it.
            let request = context.clear_request();
            unsafe {
                match request {
                    StatusRequest::None => {
                        if scheduler_data.dependencies.is_empty() {
                            context.set_schedule_status(TaskScheduleStatus::Runnable)
                        } else {
                            context.set_schedule_status(TaskScheduleStatus::Waiting)
                        }
                    }
                    StatusRequest::Block => {
                        context.set_schedule_status(TaskScheduleStatus::Blocked)
                    }
                    StatusRequest::Abort => {
                        context.set_schedule_status(TaskScheduleStatus::Aborted)
                    }
                }
            }

            // finally process the message
            trace!("Processing {} message", data.msg_type());
            drop(context);
            match data {
                MsgData::Yield { f } => {
                    let scheduler = fimo_ffi::ptr::coerce_obj_mut(self);
                    unsafe { f.assume_valid()(scheduler, task.as_raw()) }
                }
                MsgData::Completed { aborted } => {
                    let context = task.context().borrow();
                    if aborted {
                        trace!("Marking task {} as aborted", context.handle());
                        unsafe { context.set_schedule_status(TaskScheduleStatus::Aborted) }
                    } else {
                        trace!("Marking task {} as finished", context.handle());
                        unsafe { context.set_schedule_status(TaskScheduleStatus::Finished) }
                    }
                }
            }
            let mut context = task.context().borrow_mut();
            let scheduler_data = unsafe { context.scheduler_data() };

            // the status may have changed.
            trace!("Task {} processed", context.handle());
            debug!("Task run status: {:?}", context.run_status());
            debug!("Task schedule status: {:?}", context.schedule_status());
            debug!("Number of waiters: {}", scheduler_data.waiters.len());
            debug!(
                "Number of dependencies: {}",
                scheduler_data.dependencies.len()
            );

            // remove the processing flag.
            let scheduler_data = unsafe { context.scheduler_data_mut() };
            scheduler_data.processing = false;

            // the status must be queried again.
            match context.schedule_status() {
                TaskScheduleStatus::Runnable => {
                    trace!("Inserting task {} into processing queue", context.handle());
                    drop(context);
                    unsafe { self.task_manager.enqueue(task) };
                }
                TaskScheduleStatus::Scheduled | TaskScheduleStatus::Processing => {
                    error!("Invalid schedule status for task {}", context.handle());
                    panic!(
                        "Did not expect status {:?} for task {}",
                        context.schedule_status(),
                        context.handle()
                    );
                }
                TaskScheduleStatus::Aborted | TaskScheduleStatus::Finished => {
                    trace!("Task finished cleaning up");
                    drop(context);
                    self.process_finished(task);
                }
                TaskScheduleStatus::Waiting | TaskScheduleStatus::Blocked => {
                    trace!(
                        "Task {} suspended with status {:?}",
                        context.handle(),
                        context.schedule_status()
                    );
                }
            }
        }

        stale
    }

    #[inline]
    pub(crate) fn shutdown_worker_pool(&mut self) {
        unsafe { ManuallyDrop::drop(&mut self.worker_pool) };
    }

    pub(crate) fn schedule_tasks(&mut self) -> (bool, Option<SystemTime>) {
        trace!("Running scheduler");
        let no_msgs = self.process_msg();

        let time = SystemTime::now();
        let queue = self.task_manager.clear_queue();
        trace!("Scheduling {} tasks", queue.len());

        let mut stale = no_msgs;
        let mut min_time = None;
        for Reverse(task) in queue.into_sorted_vec() {
            let mut context = task.context().borrow_mut();

            let handle = context.handle();
            let empty_task = context.is_empty_task();
            let run_status = context.run_status();
            let resume_time = context.resume_time();
            let schedule_status = context.schedule_status();
            let scheduler_data = unsafe { context.scheduler_data_mut() };

            trace!(
                "Scheduling task {}, name {:?}",
                handle,
                task.resolved_name()
            );
            debug!("Task run status: {:?}", run_status);
            debug!("Task schedule status: {:?}", schedule_status);
            debug!("Number of waiters: {}", scheduler_data.waiters.len());
            debug!(
                "Number of dependencies: {}",
                scheduler_data.dependencies.len()
            );
            debug_assert_eq!(run_status, TaskRunStatus::Idle);

            // The `wait_task_on` method can change the status of a runnable
            // task from `Runnable` to `Waiting` even when it is already enqueued.
            // If that is the case we must remove the task from the queue.
            if schedule_status != TaskScheduleStatus::Runnable {
                info!("Task {} not runnable, skipping", handle);
                drop(context);
                continue;
            }

            // check that the requested time has passed.
            if time < resume_time {
                info!(
                    "Task resume time {:?} not reached, time {:?}, skipping.",
                    resume_time, time
                );
                drop(context);

                min_time = Some(min_time.map_or(resume_time, |t| std::cmp::min(t, resume_time)));

                // SAFETY: The task only appeared once in the queue before it was cleared
                // so moving it to the new queue won't cause any duplicates.
                unsafe { self.task_manager.enqueue(task) };
                continue;
            }

            // we found a runnable task so we are not stale
            stale = false;

            // allocate stack
            if scheduler_data.context.is_none() && !empty_task {
                // try to allocate the task or reinsert.
                let (slot, stack) = match self.stacks.allocate() {
                    Ok(r) => r,
                    Err(_) => {
                        error!("Unable to schedule task {}, retrying later", handle);
                        drop(context);
                        unsafe { self.task_manager.enqueue(task) };

                        if min_time.is_none() {
                            min_time = Some(time)
                        }

                        continue;
                    }
                };

                unsafe {
                    let context = Context::new(&stack, crate::worker_pool::task_main);
                    scheduler_data.slot = Some(slot);
                    scheduler_data.context = Some(context);
                }
            }

            // schedule task on worker
            drop(context);
            self.worker_pool.schedule_task(task);
        }

        self.notify_one_scheduler_waiter()
            .expect("can not notify tasks waiting on the scheduler");

        (stale, min_time)
    }
}

impl Debug for TaskScheduler {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TaskScheduler")
            .field("stacks", &self.stacks)
            .field("worker_pool", &self.worker_pool)
            .field("task_manager", &self.task_manager)
            .finish()
    }
}

impl Drop for TaskScheduler {
    fn drop(&mut self) {
        info!("Shutting down task scheduler");

        unsafe {
            // the worker pool is already shut down at this point, so
            // drop the task and the stacks with the allocated memory.
            ManuallyDrop::drop(&mut self.task_manager);
            ManuallyDrop::drop(&mut self.stacks);
        }
    }
}

impl IScheduler for TaskScheduler {
    fn worker_ids(&self) -> &[WorkerId] {
        self.worker_ids()
    }

    unsafe fn find_task_unbound<'a>(
        &self,
        handle: TaskHandle,
    ) -> fimo_module::Result<&'a DynObj<dyn IRawTask + 'a>> {
        match self.find_task(handle) {
            Ok(t) => Ok(std::mem::transmute(t)),
            Err(e) => Err(e),
        }
    }

    unsafe fn register_task(
        &mut self,
        task: &DynObj<dyn IRawTask + '_>,
        wait_on: &[TaskHandle],
    ) -> fimo_module::Result<()> {
        self.register_task(task, wait_on)
    }

    fn unregister_task(&mut self, task: &DynObj<dyn IRawTask + '_>) -> fimo_module::Result<()> {
        self.unregister_task(task)
    }

    fn wait_task_on(
        &mut self,
        task: &DynObj<dyn IRawTask + '_>,
        on: &DynObj<dyn IRawTask + '_>,
    ) -> fimo_module::Result<()> {
        self.wait_task_on(task, on)
    }

    unsafe fn notify_one(
        &mut self,
        task: &DynObj<dyn IRawTask + '_>,
    ) -> fimo_module::Result<Option<usize>> {
        self.notify_one(task)
    }

    unsafe fn notify_all(
        &mut self,
        task: &DynObj<dyn IRawTask + '_>,
    ) -> fimo_module::Result<usize> {
        self.notify_all(task)
    }

    fn unblock_task(&mut self, task: &DynObj<dyn IRawTask + '_>) -> fimo_module::Result<()> {
        self.unblock_task(task)
    }
}
