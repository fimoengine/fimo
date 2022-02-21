use crate::stack_allocator::StackAllocator;
use crate::task_manager::{Msg, MsgData, RawTask, TaskManager};
use crate::worker_pool::WorkerPool;
use crate::Runtime;
use context::Context;
use fimo_ffi::object::{CoerceObject, CoerceObjectMut, ObjectWrapper};
use fimo_ffi::{ObjArc, Optional, SpanInner};
use fimo_module::{impl_vtable, is_object, Error};
use fimo_tasks_int::raw::{
    Builder, IRawTask, StatusRequest, TaskHandle, TaskRunStatus, TaskScheduleStatus, WorkerId,
};
use fimo_tasks_int::runtime::{IScheduler, ISchedulerResult, ISchedulerVTable};
use log::{debug, error, info, trace};
use std::cmp::Reverse;
use std::mem::ManuallyDrop;
use std::ops::{Deref, DerefMut};
use std::ptr::NonNull;
use std::sync::mpsc::{Receiver, Sender};
use std::sync::Weak;
use std::time::SystemTime;

/// Task scheduler.
#[derive(Debug)]
pub struct TaskScheduler {
    scheduler_task: ObjArc<IRawTask>,
    stacks: ManuallyDrop<StackAllocator>,
    worker_pool: ManuallyDrop<WorkerPool>,
    task_manager: ManuallyDrop<TaskManager>,
}

impl TaskScheduler {
    pub(crate) fn new(
        stack_size: usize,
        pre_allocated: usize,
        preferred_num_allocations: usize,
        msg_receiver: Receiver<Msg>,
    ) -> Result<Self, Error> {
        trace!("Constructing scheduler");

        // construct a blocked task for the scheduler.
        let scheduler_task = Builder::new()
            .with_name("Task runtime scheduler".into())
            .blocked()
            .build(None, None, None);
        let scheduler_task = ObjArc::new(scheduler_task);
        let scheduler_task: ObjArc<IRawTask> = ObjArc::coerce_object(scheduler_task);

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
        msg_sender: Sender<Msg>,
        workers: Option<usize>,
    ) -> Result<(), Error> {
        self.worker_pool.start_workers(runtime, msg_sender, workers)
    }

    /// Searches for a registered task.
    #[inline]
    pub fn find_task(&self, handle: TaskHandle) -> Result<&IRawTask, Error> {
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
        task: &IRawTask,
        deps: &[TaskHandle],
    ) -> Result<(), Error> {
        let task: &'static IRawTask = &*(task as *const _);
        self.task_manager.register_task(task, deps)
    }

    /// Unregisters a task from the task runtime.
    pub fn unregister_task(&mut self, task: &IRawTask) -> Result<(), Error> {
        // we assert that we own the task.
        let task = unsafe { RawTask::from_i_raw(task) };
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
    pub fn wait_task_on(&mut self, task: &IRawTask, wait_on: &IRawTask) -> Result<(), Error> {
        // we assert that we own both tasks.
        let task: &'static RawTask = unsafe { &*(RawTask::from_i_raw(task) as *const _) };
        let wait_on: &'static RawTask = unsafe { &*(RawTask::from_i_raw(wait_on) as *const _) };
        self.task_manager.wait_task_on(task, wait_on)
    }

    #[inline]
    pub(crate) fn wait_on_scheduler(&mut self, task: &IRawTask) -> Result<(), Error> {
        let scheduler_task: &IRawTask = unsafe { &*(&*self.scheduler_task as *const _) };
        self.wait_task_on(task, scheduler_task)
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
    pub unsafe fn notify_one(&mut self, task: &IRawTask) -> Result<Option<usize>, Error> {
        // we assert that we own the task.
        let task = RawTask::from_i_raw(task);
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
    pub unsafe fn notify_all(&mut self, task: &IRawTask) -> Result<usize, Error> {
        // we assert that we own the task.
        let task = RawTask::from_i_raw(task);
        self.task_manager.notify_all(task)
    }

    /// Unblocks a blocked task.
    #[inline]
    pub fn unblock_task(&mut self, task: &'static IRawTask) -> Result<(), Error> {
        // we assert that we own the task.
        let task: &'static RawTask = unsafe { &*(task as *const _ as *const RawTask) };
        self.task_manager.unblock_task(task)
    }

    fn process_finished(&mut self, task: &RawTask) {
        // safety: we are inside the scheduler.
        let context = unsafe { task.scheduler_context_mut() };

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
        unsafe { self.task_manager.notify_all(task).expect("Invalid task") };

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
            let context = unsafe { task.scheduler_context_mut() };
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
            match data {
                MsgData::Yield { f } => {
                    let scheduler: &mut IScheduler =
                        IScheduler::from_object_mut(self.coerce_obj_mut());
                    unsafe {
                        f.assume_valid()(NonNull::from(scheduler), NonNull::from(task.as_i_raw()))
                    }
                }
                MsgData::Completed { aborted } => {
                    if aborted {
                        trace!("Marking task {} as aborted", context.handle());
                        unsafe { context.set_schedule_status(TaskScheduleStatus::Aborted) }
                    } else {
                        trace!("Marking task {} as finished", context.handle());
                        unsafe { context.set_schedule_status(TaskScheduleStatus::Finished) }
                    }
                }
            }

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

    pub(crate) fn schedule_tasks(&mut self) -> bool {
        trace!("Running scheduler");
        let stale = self.process_msg();

        let time = SystemTime::now();
        let queue = self.task_manager.clear_queue();
        trace!("Scheduling {} tasks", queue.len());

        let stale = stale && queue.is_empty();
        for Reverse(task) in queue.into_sorted_vec() {
            let context = unsafe { task.scheduler_context_mut() };

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

            // check that the task is still runnable.
            if schedule_status != TaskScheduleStatus::Runnable {
                info!("Task {} not runnable, skipping", handle);
                unsafe { self.task_manager.enqueue(task) };
                continue;
            }

            // check that the requested time has passed.
            if time < resume_time {
                info!(
                    "Task resume time {:?} not reached, time {:?}, skipping.",
                    resume_time, time
                );
                unsafe { self.task_manager.enqueue(task) };
                continue;
            }

            // allocate stack
            if scheduler_data.context.is_none() && !empty_task {
                // try to allocate the task or reinsert.
                let (slot, stack) = match self.stacks.allocate() {
                    Ok(r) => r,
                    Err(_) => {
                        error!("Unable to schedule task {}, retrying later", handle);
                        unsafe { self.task_manager.enqueue(task) };
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
            self.worker_pool.schedule_task(task);
        }

        stale
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

impl Deref for TaskScheduler {
    type Target = IScheduler;

    fn deref(&self) -> &Self::Target {
        IScheduler::from_object(self.coerce_obj())
    }
}

impl DerefMut for TaskScheduler {
    fn deref_mut(&mut self) -> &mut Self::Target {
        IScheduler::from_object_mut(self.coerce_obj_mut())
    }
}

is_object! { #![uuid(0x7f2cb683, 0x26b4, 0x46cb, 0xa91d, 0x3cbecf295ad8)] TaskScheduler }

impl_vtable! {
    impl mut ISchedulerVTable => TaskScheduler {
        unsafe extern "C" fn worker_ids(
            ptr: *const ()
        ) -> SpanInner<WorkerId, false> {
            let this = &*(ptr as *const TaskScheduler);
            this.worker_ids().into()
        }

        #[allow(improper_ctypes_definitions)]
        unsafe extern "C" fn find_task(
            ptr: *const (),
            handle: TaskHandle
        ) -> ISchedulerResult<NonNull<IRawTask>> {
            let this = &*(ptr as *const TaskScheduler);
            this.find_task(handle).map(|t| t.into()).into()
        }

        #[allow(improper_ctypes_definitions)]
        unsafe extern "C" fn register_task(
            ptr: *mut (),
            task: NonNull<IRawTask>,
            wait_on: SpanInner<TaskHandle, false>,
        ) -> ISchedulerResult<()> {
            let this = &mut *(ptr as *mut TaskScheduler);
            this.register_task(task.as_ref(), wait_on.into()).into()
        }

        #[allow(improper_ctypes_definitions)]
        unsafe extern "C" fn unregister_task(
            ptr: *mut (),
            task: NonNull<IRawTask>,
        ) -> ISchedulerResult<()> {
            let this = &mut *(ptr as *mut TaskScheduler);
            this.unregister_task(task.as_ref()).into()
        }

        #[allow(improper_ctypes_definitions)]
        unsafe extern "C" fn wait_task_on(
            ptr: *mut (),
            task: NonNull<IRawTask>,
            wait_on: NonNull<IRawTask>
        ) -> ISchedulerResult<()> {
            let this = &mut *(ptr as *mut TaskScheduler);
            this.wait_task_on(task.as_ref(), wait_on.as_ref()).into()
        }

        #[allow(improper_ctypes_definitions)]
        unsafe extern "C" fn notify_one(
            ptr: *mut (),
            task: NonNull<IRawTask>
        ) -> ISchedulerResult<Optional<usize>> {
            let this = &mut *(ptr as *mut TaskScheduler);
            this.notify_one(task.as_ref()).map(|w| w.into()).into()
        }

        #[allow(improper_ctypes_definitions)]
        unsafe extern "C" fn notify_all(
            ptr: *mut (),
            task: NonNull<IRawTask>
        ) -> ISchedulerResult<usize> {
            let this = &mut *(ptr as *mut TaskScheduler);
            this.notify_all(task.as_ref()).into()
        }

        #[allow(improper_ctypes_definitions)]
        unsafe extern "C" fn unblock_task(
            ptr: *mut (),
            task: NonNull<IRawTask>
        ) -> ISchedulerResult<()> {
            let this = &mut *(ptr as *mut TaskScheduler);
            this.unblock_task(task.as_ref()).into()
        }
    }
}
