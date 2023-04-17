use crate::worker_pool::WorkerPool;
use crate::Runtime;
use context::Context;
use fimo_ffi::type_id::StableTypeId;
use fimo_ffi::{DynObj, FfiFn, Object};
use fimo_logging_int::{debug, error, info, trace};
use fimo_module::Error;
use fimo_tasks_int::raw::{
    IRawTask, PseudoTask, StatusRequest, TaskHandle, TaskRunStatus, TaskScheduleStatus, WorkerId,
};
use fimo_tasks_int::runtime::{IScheduler, NotifyResult, WaitToken, WakeupToken};
use std::cmp::Reverse;
use std::fmt::Debug;
use std::mem::{ManuallyDrop, MaybeUninit};
use std::sync::mpsc::{Receiver, Sender};
use std::sync::Weak;
use std::time::SystemTime;

pub(crate) mod stack_allocator;
pub(crate) mod task_manager;

use stack_allocator::StackAllocator;
use task_manager::{AssertValidTask, Msg, MsgData, TaskManager};

/// Task scheduler.
#[derive(Object, StableTypeId)]
#[name("TaskScheduler")]
#[uuid("7f2cb683-26b4-46cb-a91d-3cbecf295ad8")]
#[interfaces(IScheduler)]
pub struct TaskScheduler {
    _unique_addr: Box<u8>,
    scheduler_task: PseudoTask,
    stacks: ManuallyDrop<StackAllocator>,
    worker_pool: ManuallyDrop<WorkerPool>,
    task_manager: ManuallyDrop<TaskManager>,
}

// SAFETY: The API of the scheduler ensures that the type can
// be shared with other threads. This sharing does not happen
// in practice, as the scheduler will usually lie behind a
// mutex.
unsafe impl Sync for TaskScheduler {}

impl TaskScheduler {
    pub(crate) fn new(
        stack_size: usize,
        pre_allocated: usize,
        preferred_num_allocations: usize,
        msg_receiver: Receiver<Msg<'static>>,
    ) -> Result<Self, Error> {
        trace!("Constructing scheduler");

        // Allocate a box to generate a new unique address.
        let addr = Box::new(0);
        let addr_ptr = &*addr as *const u8 as *const ();

        let mut this = Self {
            _unique_addr: addr,
            scheduler_task: PseudoTask(addr_ptr),
            stacks: ManuallyDrop::new(StackAllocator::new(
                stack_size,
                pre_allocated,
                preferred_num_allocations,
            )?),
            worker_pool: ManuallyDrop::new(WorkerPool::new()),
            task_manager: ManuallyDrop::new(TaskManager::new(msg_receiver)),
        };

        // SAFETY: Checking of the conditions
        // 1. We controll the address.
        // 2. Task is yet unregistered.
        unsafe { this.register_or_fetch_pseudo_task(addr_ptr)? };

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

    /// Blocks a task until the scheduler has been unlocked.
    ///
    /// # Safety
    ///
    /// The task must be registered with our runtime.
    #[inline]
    pub(crate) unsafe fn wait_on_scheduler(
        &mut self,
        task: &DynObj<dyn IRawTask + '_>,
    ) -> Result<(), Error> {
        self.wait_task_on(task, self.scheduler_task, None, WaitToken::INVALID)
    }

    #[inline]
    pub(crate) fn notify_scheduler_unlocked(&mut self) -> Result<(), Error> {
        let mut f = MaybeUninit::new(|_| WakeupToken::None);

        // SAFETY: We transfer the ownership of `f` to the wrapper and `discard` it.
        let f = unsafe { FfiFn::new_value(&mut f) };

        // SAFETY: There are three parts of the safety that must be checked:
        // 1. `scheduler_task` is registered (is trivial).
        // 2. The callback must not panic or use the scheduler (trivial).
        // 3. Waking one task does not cause any undesired effects,
        // like uninitialized memory accesses.
        //
        // We can assert that the second point is true, as `scheduler_task` represents
        // an always blocked task, used to synchronize the access to the scheduler.
        // In other words, an equivalent of a mutex. Being woken up by that mutex
        // does not equal acquiring the lock, but is rather a hint to retry locking it.
        // Under these semantics, the mutex is only a performance improvement over
        // calling an equivalent `try_lock` in a loop and has therefore no side effects.
        unsafe { self.notify_one(self.scheduler_task, f)? };
        Ok(())
    }

    // Handles the case that a task has finished it's execution.
    fn process_finished(&mut self, task: AssertValidTask) {
        let context = task.context().borrow();

        trace!("Cleaning up task {}", context.handle());
        debug!("Task run status: {:?}", context.run_status());
        debug!("Task schedule status: {:?}", context.schedule_status());
        debug!("Task data: {:?}", context.scheduler_data());

        // task should not be running or have any dependencies at this point
        debug_assert_eq!(context.run_status(), TaskRunStatus::Idle);
        debug_assert!(matches!(
            context.schedule_status(),
            TaskScheduleStatus::Aborted | TaskScheduleStatus::Finished
        ));
        debug_assert!(context
            .scheduler_data()
            .private_data()
            .dependencies
            .is_empty());

        trace!("Notify waiters of {}", context.handle());

        // `notify_all` requires the context.
        drop(context);

        let pseudo_task = self
            .task_manager
            .fetch_pseudo_task(task.as_raw() as *const _ as *const ())
            .expect("registered task must have a corresponding pseudo task");

        // SAFETY: The task has been completed, therefore notifying the waiters
        // of the task is a safe (and required) operation.
        unsafe {
            self.task_manager
                .notify_all(pseudo_task, WakeupToken::None)
                .expect("Invalid task")
        };

        let mut context = task.context().borrow_mut();
        let data = context.scheduler_data();

        // If the task panicked, we need to retrieve the panic data from the
        // shared context and place it in the public context.
        let panic = {
            let mut shared = data.shared_data_mut();
            let mut private = data.private_data_mut();

            trace!("Free task slot of {}", context.handle());
            if let Some(slot) = private.slot.take() {
                if let Err(e) = self.stacks.deallocate(slot) {
                    error!("Could not free task slot, error: {}", e)
                }
            }

            shared.take_panic()
        };

        if let Some(panic) = panic {
            context.set_panic(Some(panic));
            debug_assert!(context.is_panicking());
        }

        trace!("Internal task cleanup");
        context.cleanup();

        trace!("Marking task {} as completed", context.handle());
        context.set_run_status(TaskRunStatus::Completed);
    }

    fn process_msg(&mut self) -> bool {
        trace!("Processing scheduler messages");

        let msgs = self.task_manager.take_messages();
        let stale = msgs.is_empty();

        trace!("Processing {} scheduler messages", msgs.len());
        for Msg { task, data } in msgs {
            let context = task.context().borrow();
            let scheduler_data = context.scheduler_data();

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
            debug!("Task data: {:?}", scheduler_data);

            // task should not be running
            debug_assert_eq!(run_status, TaskRunStatus::Idle);
            debug_assert_eq!(schedule_status, TaskScheduleStatus::Processing);

            // Start by processing the message
            let mut private = scheduler_data.private_data_mut();
            // SAFETY: We are toggling and untoggling the flag according to the documentation.
            unsafe { private.toggle_processing(true) };

            trace!("Processing {} message", data.msg_type());

            // The callback may require the context.
            drop(private);
            drop(context);

            let apply_requests = match data {
                MsgData::Yield { f } => {
                    let scheduler = fimo_ffi::ptr::coerce_obj_mut(self);

                    // SAFETY: The api ensures that the task remains blocked until
                    // the message is processed. We know that the callback was valid
                    // at the time the task yielded its execution, therefore it must
                    // have remained valid and we can invoke it.
                    unsafe { f.assume_valid()(scheduler, task.as_raw()) };
                    true
                }
                MsgData::Completed { aborted } => {
                    let context = task.context().borrow();

                    // SAFETY: As the implementation of the scheduler we ensure that
                    // the operation is well behaved, as no one else is allowed to
                    // modify the status.
                    unsafe {
                        if aborted {
                            trace!("Marking task {} as aborted", context.handle());
                            context.set_schedule_status(TaskScheduleStatus::Aborted)
                        } else {
                            trace!("Marking task {} as finished", context.handle());
                            context.set_schedule_status(TaskScheduleStatus::Finished)
                        }
                    }
                    false
                }
            };

            let context = task.context().borrow();
            let scheduler_data = context.scheduler_data();

            // the status may have changed.
            trace!("Task {} processed", context.handle());
            debug!("Task run status: {:?}", context.run_status());
            debug!("Task schedule status: {:?}", context.schedule_status());
            debug!("Task data: {:?}", scheduler_data);

            let mut private = scheduler_data.private_data_mut();

            // SAFETY: See above.
            // Remove the processing flag.
            unsafe { private.toggle_processing(false) };

            if apply_requests {
                // Clear and apply the status-change request now that they may be set.
                // SAFETY: We are the scheduler and are therefore allowed to clear the requests
                // and apply them.
                unsafe {
                    let request = context.clear_request();
                    match request {
                        StatusRequest::None => {
                            if private.dependencies.is_empty() {
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
            }

            // The status must be queried again.
            match context.schedule_status() {
                TaskScheduleStatus::Runnable => {
                    trace!("Inserting task {} into processing queue", context.handle());
                    drop(private);
                    drop(context);
                    self.task_manager.enqueue(task);
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
                    drop(private);
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

    /// Shuts down the worker pool by dropping it.
    ///
    /// After being called it is impossible for it to be restarted.
    ///
    /// # Note
    ///
    /// Calling this method practically make the entire runtime unusable.
    #[inline]
    pub(crate) fn shutdown_worker_pool(&mut self) {
        // SAFETY: The contract of this method allows this.
        unsafe { ManuallyDrop::drop(&mut self.worker_pool) };
    }

    pub(crate) fn schedule_tasks(&mut self) -> (bool, Option<SystemTime>) {
        trace!("Running scheduler");
        let no_msgs = self.process_msg();

        // SAFETY: We are currently scheduling.
        let queue = unsafe { self.task_manager.clear_queue() };

        trace!("Scheduling {} tasks", queue.len());

        let mut stale = no_msgs;
        let time = SystemTime::now();
        let mut min_time = None;
        for Reverse(task) in queue.into_sorted_vec() {
            let context = task.context().borrow();
            let scheduler_data = context.scheduler_data();

            let run_status = context.run_status();
            let resume_time = context.resume_time();
            let schedule_status = context.schedule_status();

            trace!(
                "Scheduling task {}, name {:?}",
                context.handle(),
                task.resolved_name()
            );
            debug!("Task run status: {:?}", run_status);
            debug!("Task schedule status: {:?}", schedule_status);
            debug!("Task data: {:?}", scheduler_data);
            debug_assert_eq!(run_status, TaskRunStatus::Idle);

            let mut private = scheduler_data.private_data_mut();
            let mut shared = scheduler_data.shared_data_mut();

            // The `wait_task_on` method can change the status of a runnable
            // task from `Runnable` to `Waiting` even when it is already enqueued.
            // If that is the case we must remove the task from the queue and
            // mark it as not in the queue.
            if schedule_status != TaskScheduleStatus::Runnable {
                info!("Task {} not runnable, skipping", context.handle());

                // SAFETY: The task was removed from the queue.
                unsafe { private.assert_not_in_queue() };
                continue;
            }

            // check that the requested time has passed.
            if time < resume_time {
                info!(
                    "Task resume time {:?} not reached, time {:?}, skipping.",
                    resume_time, time
                );
                drop(private);
                drop(shared);
                drop(context);

                min_time = Some(min_time.map_or(resume_time, |t| std::cmp::min(t, resume_time)));

                // SAFETY: The task only appeared once in the queue before it was cleared
                // so moving it to the new queue won't cause any duplicates.
                unsafe { self.task_manager.enqueue_unchecked(task) };
                continue;
            }

            // we found a runnable task so we are not stale
            stale = false;

            // allocate stack
            if shared.is_empty_context() && !context.is_empty_task() {
                // try to allocate the task or reinsert.
                let (slot, stack) = match self.stacks.allocate() {
                    Ok(r) => r,
                    Err(_) => {
                        error!(
                            "Unable to schedule task {}, retrying later",
                            context.handle()
                        );

                        // make the borrow checker happy.
                        drop(private);
                        drop(shared);
                        drop(context);

                        // SAFETY: The task was contained only once in the queue and
                        // therefore will continue to be as such.
                        unsafe { self.task_manager.enqueue_unchecked(task) };

                        if min_time.is_none() {
                            min_time = Some(time)
                        }

                        continue;
                    }
                };

                // SAFETY: We know that the stack will outlive the context, because the context
                // lives until the completion of the task and only then will the stack be deallocated.
                unsafe {
                    let context = Context::new(&stack, crate::worker_pool::task_main);
                    private.slot = Some(slot);
                    shared.set_context(context);
                }
            }

            // schedule task on worker
            // SAFETY: The task was removed from the queue.
            unsafe { private.assert_not_in_queue() };

            // SAFETY: As the scheduler we ensure correctness.
            unsafe { context.set_schedule_status(TaskScheduleStatus::Scheduled) };

            drop(private);
            drop(shared);
            drop(context);

            self.worker_pool.assign_task_to_worker(task);
        }

        self.notify_scheduler_unlocked()
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

        // SAFETY: We are dropping the scheduler so further drops are safe.
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

    fn get_task_from_handle(&self, handle: TaskHandle) -> Option<&DynObj<dyn IRawTask + '_>> {
        self.task_manager.get_task_from_handle(handle)
    }

    unsafe fn get_pseudo_task_from_handle(
        &self,
        handle: TaskHandle,
    ) -> Option<fimo_tasks_int::raw::PseudoTask> {
        self.task_manager.get_pseudo_task_from_handle(handle)
    }

    unsafe fn register_task(
        &mut self,
        task: &DynObj<dyn IRawTask + '_>,
        wait_on: &[TaskHandle],
    ) -> fimo_module::Result<()> {
        self.task_manager.register_task(task, wait_on)
    }

    unsafe fn unregister_task(
        &mut self,
        task: &DynObj<dyn IRawTask + '_>,
    ) -> fimo_module::Result<()> {
        // SAFETY: We assume that the task is registered with our runtime.
        let task = AssertValidTask::from_raw(task);
        self.task_manager.unregister_task(task)
    }

    unsafe fn unblock_task(&mut self, task: &DynObj<dyn IRawTask + '_>) -> fimo_module::Result<()> {
        // SAFETY: We assume that the task is registered with our runtime.
        let task = AssertValidTask::from_raw(task);
        self.task_manager.unblock_task(task)
    }

    unsafe fn register_or_fetch_pseudo_task(
        &mut self,
        addr: *const (),
    ) -> fimo_module::Result<fimo_tasks_int::raw::PseudoTask> {
        self.task_manager.register_or_fetch_pseudo_task(addr, None)
    }

    unsafe fn unregister_pseudo_task(
        &mut self,
        task: fimo_tasks_int::raw::PseudoTask,
    ) -> fimo_module::Result<()> {
        self.task_manager.unregister_pseudo_task(task, false)
    }

    unsafe fn unregister_pseudo_task_if_empty(
        &mut self,
        task: fimo_tasks_int::raw::PseudoTask,
    ) -> fimo_module::Result<bool> {
        self.task_manager
            .unregister_pseudo_task_if_empty(task, false)
    }

    unsafe fn wait_task_on(
        &mut self,
        task: &DynObj<dyn IRawTask + '_>,
        on: fimo_tasks_int::raw::PseudoTask,
        data_addr: Option<&mut MaybeUninit<WakeupToken>>,
        token: WaitToken,
    ) -> fimo_module::Result<()> {
        // SAFETY: We assume that the task is registered with our runtime.
        let task = AssertValidTask::from_raw(task);
        self.task_manager.wait_task_on(task, on, data_addr, token)
    }

    unsafe fn notify_one(
        &mut self,
        task: fimo_tasks_int::raw::PseudoTask,
        data_callback: FfiFn<'_, dyn FnOnce(NotifyResult) -> WakeupToken + '_, u8>,
    ) -> fimo_module::Result<NotifyResult> {
        self.task_manager.notify_one(task, data_callback)
    }

    unsafe fn notify_all(
        &mut self,
        task: fimo_tasks_int::raw::PseudoTask,
        data: WakeupToken,
    ) -> fimo_module::Result<usize> {
        self.task_manager.notify_all(task, data)
    }

    unsafe fn notify_filter(
        &mut self,
        task: fimo_tasks_int::raw::PseudoTask,
        filter: FfiFn<'_, dyn FnMut(WaitToken) -> fimo_tasks_int::runtime::NotifyFilterOp + '_, u8>,
        data_callback: FfiFn<'_, dyn FnOnce(NotifyResult) -> WakeupToken + '_, u8>,
    ) -> fimo_module::Result<NotifyResult> {
        self.task_manager.notify_filter(task, filter, data_callback)
    }
}
