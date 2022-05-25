use super::stack_allocator::TaskSlot;
use context::Context;
use fimo_ffi::cell::{AtomicRef, AtomicRefCell, AtomicRefMut};
use fimo_ffi::ffi_fn::RawFfiFn;
use fimo_ffi::ptr::IBaseExt;
use fimo_ffi::{DynObj, FfiFn, ObjBox, ObjectId};
use fimo_logging_int::{debug, error, info, trace, SpanStackId};
use fimo_module::{Error, ErrorKind};
use fimo_tasks_int::raw::{
    AtomicISchedulerContext, IRawTask, IRustPanicData, ISchedulerContext, PseudoTask,
    StatusRequest, TaskHandle, TaskPriority, TaskRunStatus, TaskScheduleStatus, WorkerId,
};
use fimo_tasks_int::runtime::{IScheduler, NotifyResult, WaitToken, WakeupToken};
use std::any::Any;
use std::cmp::{Ordering, Reverse};
use std::collections::{BTreeMap, BTreeSet, BinaryHeap, HashMap, VecDeque};
use std::fmt::Debug;
use std::mem::MaybeUninit;
use std::ops::RangeFrom;
use std::sync::atomic::AtomicPtr;
use std::sync::mpsc::Receiver;
use std::time::SystemTime;

#[derive(Debug)]
pub(crate) struct TaskManager {
    msg_receiver: Receiver<Msg<'static>>,
    handle_iter: RangeFrom<usize>,
    free_handles: VecDeque<TaskHandle>,
    tasks: HashMap<TaskHandle, AssertValidTask>,
    pseudo_tasks: HashMap<usize, PseudoTaskData>,
    processing_queue: BinaryHeap<Reverse<AssertValidTask>>,
}

impl TaskManager {
    pub fn new(msg_receiver: Receiver<Msg<'static>>) -> Self {
        trace!("Initializing the task manager");
        Self {
            msg_receiver,
            handle_iter: 0..,
            free_handles: Default::default(),
            tasks: Default::default(),
            pseudo_tasks: Default::default(),
            processing_queue: Default::default(),
        }
    }

    fn allocate_handle(&mut self) -> Result<TaskHandle, Error> {
        trace!("Allocating a task handle");

        if let Some(handle) = self.free_handles.pop_front() {
            debug!("Reusing existing handle {}", handle);
            Ok(handle)
        } else {
            trace!("Allocating new handle");
            if let Some(id) = self.handle_iter.next() {
                let handle = TaskHandle { id, generation: 0 };
                debug!("Allocated handle {}", handle);
                Ok(handle)
            } else {
                error!("Handles exhausted");
                let err = "Exhausted all possible handles";
                Err(Error::new(ErrorKind::ResourceExhausted, err))
            }
        }
    }

    /// Frees the handle without increasing the generation allowing it's reallocation.
    ///
    /// # Safety
    ///
    /// The caller must ensure that the provided handle has not been leaked and is
    /// currently allocated. Mainly intended to be used when an error occurs during
    /// a task registration, allowing us to support more tasks before an overflow occurs.
    unsafe fn free_handle_reuse(&mut self, handle: TaskHandle) {
        trace!("Reusing handle {}", handle);

        // pushing it to the back may improve debugging by
        // maximizing the time until the same id is reused.
        self.free_handles.push_back(handle);
    }

    fn free_handle(&mut self, handle: TaskHandle) {
        trace!("Freeing handle {}", handle);

        // discard a handle if the generation reaches the
        // maximum possible value, otherwise push it onto the list.
        if handle.generation == usize::MAX {
            trace!("Discarding handle {}", handle);
        } else {
            trace!("Mark handle {} for reuse", handle);
            let handle = TaskHandle {
                id: handle.id,
                generation: handle.generation + 1,
            };

            // SAFETY: We created a new handle by increasing the generation so
            // we know that it s unique.
            unsafe { self.free_handle_reuse(handle) };
        }
    }

    #[inline]
    pub fn take_messages(&mut self) -> Vec<Msg<'static>> {
        self.msg_receiver.try_iter().collect()
    }

    /// Enqueues a task without changing or checking any flags.
    ///
    /// # Safety
    ///
    /// A task may appear only once in the queue.
    #[inline]
    pub unsafe fn enqueue_unchecked(&mut self, task: AssertValidTask) {
        self.processing_queue.push(Reverse(task))
    }

    /// Enqueues a task if it is able to.
    ///
    /// # Note
    ///
    /// Does nothing if the `processing` or `in_queue` flags in the private
    /// context are set. Sets `in_queue` to `true` in case of success.
    #[inline]
    pub fn enqueue(&mut self, task: AssertValidTask) {
        let context = task.context().borrow();
        debug_assert_eq!(context.schedule_status(), TaskScheduleStatus::Runnable);

        let data = context.scheduler_data();
        let mut private_data = data.private_data_mut();
        if !(private_data.is_processing() || private_data.is_in_queue()) {
            // SAFETY: We insert the task immediately afterwards.
            unsafe { private_data.assert_in_queue() };

            drop(private_data);
            drop(context);

            // SAFETY: this method is behind a mutable reference, so we know
            // that the scheduler lock was acquired and we have unique access
            // to the processing queue. Now we have to ensure that we aren't
            // trying to enqueue a task multiple times. This is done by
            // dynamically checking that the `in_queue` flag is not set. This
            // flag can only be unset during the processing of the queue
            // during scheduling. To remove the need of removing a prematurely
            // inserted task, we disable the operation entirely if the task is
            // being processed.
            unsafe { self.enqueue_unchecked(task) }
        }
    }

    /// Clears the queue and returns it.
    ///
    /// # Safety
    ///
    /// Only callable during scheduling.
    #[inline]
    pub unsafe fn clear_queue(&mut self) -> BinaryHeap<Reverse<AssertValidTask>> {
        std::mem::take(&mut self.processing_queue)
    }

    pub fn get_task_from_handle(&self, handle: TaskHandle) -> Option<&DynObj<dyn IRawTask + '_>> {
        trace!("Searching for task {}", handle);
        if let Some(task) = self.tasks.get(&handle) {
            trace!("Found task {handle}");
            Some(task.as_raw())
        } else {
            info!("Task {handle} not found");
            None
        }
    }

    /// # Safety
    ///
    /// See [`get_pseudo_task_from_handle`](fimo_tasks_int::runtime::IScheduler::get_pseudo_task_from_handle)
    pub unsafe fn get_pseudo_task_from_handle(&self, handle: TaskHandle) -> Option<PseudoTask> {
        trace!("Searching for pseudo task of {handle}");
        if let Some(task) = self.tasks.get(&handle) {
            trace!("Found task {handle}");
            let pseudo_task = self
                .fetch_pseudo_task(task.0 as *const _ as *const ())
                .expect("registered task must have a corresponding pseudo task");
            Some(pseudo_task)
        } else {
            info!("Task {handle} not found");
            None
        }
    }

    /// # Safety
    ///
    /// See [`register_task`](fimo_tasks_int::runtime::IScheduler::register_task)
    pub unsafe fn register_task(
        &mut self,
        task: &DynObj<dyn IRawTask + '_>,
        wait_on: &[TaskHandle],
    ) -> Result<(), Error> {
        trace!("Registering new task {:?}", task.resolved_name());

        let task_addr = task as *const _ as *const ();
        let handle;
        {
            let mut context = task.context().borrow_mut();
            if context.is_registered() {
                error!("Task {:?} already registered", task.resolved_name());
                let err = format!("The task {:?} is already registered", task.resolved_name());
                return Err(Error::new(ErrorKind::InvalidArgument, err));
            }

            // register the handle and data internally.
            handle = match self.allocate_handle() {
                Ok(x) => x,
                Err(e) => {
                    return Err(e);
                }
            };

            // SAFETY: As the implementation of the runtime we are allowed to freely modify the task
            // as by this point we already own it.
            let entry_func = context.take_entry_function();
            let data = ObjBox::new(ContextData::new(entry_func));
            context.register(handle, Some(ObjBox::coerce_obj(data)));
        }

        // SAFETY: We assert that this method is called only with valid tasks.
        let task = AssertValidTask::from_raw(task);

        // allocate a pseudo task for each task.
        let pseudo = match self.register_or_fetch_pseudo_task(task_addr, Some(task.clone())) {
            Ok(p) => p,
            Err(e) => {
                let mut context = task.context().borrow_mut();
                let (handle, _) = context.unregister();

                // SAFETY: The handle wasn't leaked to the outside world so it is safe
                // to be reused.
                self.free_handle_reuse(handle);
                return Err(e);
            }
        };

        // clear the task.
        // SAFETY: See above.
        let mut context = task.context().borrow_mut();
        context.set_run_status(TaskRunStatus::Idle);
        context.set_schedule_status(TaskScheduleStatus::Processing);
        context.take_panic_data();

        // `wait_task_on` requires a borrow of the context
        drop(context);

        // wait on all dependencies
        for dep in wait_on {
            if let Some(dep) = self.tasks.get(dep) {
                let dep = self
                    .fetch_pseudo_task(dep.as_raw() as *const _ as *const ())
                    .expect("registered task must have a corresponding pseudo task");
                if let Err(e) = self.wait_task_on(task.clone(), dep, None, WaitToken::INVALID) {
                    error!("Aborting registration, error: {}", e);
                    let mut context = task.context().borrow_mut();
                    let (handle, _) = context.unregister();
                    self.unregister_pseudo_task(pseudo, true)
                        .expect("error unregistering task");

                    // SAFETY: The handle wasn't leaked to the outside world so it is safe
                    // to be reused.
                    self.free_handle_reuse(handle);
                    return Err(e);
                }
            }
        }

        let mut context = task.context().borrow_mut();

        // SAFETY: See above.
        let request = context.clear_request();

        match request {
            StatusRequest::None => {
                let data = context.scheduler_data();
                let private = data.private_data();
                if private.dependencies.is_empty() {
                    trace!(
                        "Registered task {:?} with id {} as runnable",
                        task.resolved_name(),
                        context.handle()
                    );

                    // SAFETY: The rest of the runtime wasn't informed of the task, so we
                    // are allowed to set an initial status.
                    context.set_schedule_status(TaskScheduleStatus::Runnable);

                    // `enqueue` will borrow the private context mutably, so we must relinquish our
                    // borrow to prevent a panic.
                    drop(private);
                    drop(context);

                    self.enqueue(task.clone());
                } else {
                    trace!(
                        "Registered task {:?} with id {} as waiting",
                        task.resolved_name(),
                        context.handle()
                    );

                    // SAFETY: The rest of the runtime wasn't informed of the task, so we
                    // are allowed to set an initial status.
                    context.set_schedule_status(TaskScheduleStatus::Waiting);

                    // makes the borrow checker happy.
                    drop(private);
                    drop(context);
                }
            }
            StatusRequest::Block => {
                trace!(
                    "Registered task {:?} with id {} as blocked",
                    task.resolved_name(),
                    context.handle()
                );

                // SAFETY: The rest of the runtime wasn't informed of the task, so we
                // are allowed to set an initial status.
                context.set_schedule_status(TaskScheduleStatus::Blocked);

                // makes the borrow checker happy.
                drop(context);
            }
            StatusRequest::Abort => {
                error!(
                    "Tries to register the task {:?} as aborted",
                    task.resolved_name()
                );
                let err = format!(
                    "A task may not request an abort upon registration, name {:?}",
                    task.resolved_name()
                );

                // Having a task aborted before registration is nonsensical,
                // so we can simply invalidate it and reuse it's handle.
                let (handle, _) = context.unregister();
                self.unregister_pseudo_task(pseudo, true)
                    .expect("error unregistering task");
                self.free_handle_reuse(handle);
                return Err(Error::new(ErrorKind::InvalidArgument, err));
            }
        }

        // register the task to the runtime.
        self.tasks.insert(handle, task);

        Ok(())
    }

    /// # Safety
    ///
    /// See [`unregister_task`](fimo_tasks_int::runtime::IScheduler::unregister_task)
    pub unsafe fn unregister_task(&mut self, task: AssertValidTask) -> Result<(), Error> {
        let mut context = task.context().borrow_mut();

        trace!("Unregistering task {}", context.handle());
        debug!("Task run status: {:?}", context.run_status());
        debug!("Task schedule status: {:?}", context.schedule_status());
        debug!("Task data: {:?}", context.scheduler_data());

        if context.run_status() != TaskRunStatus::Completed {
            error!("Task {} has not been completed", context.handle());
            let err = format!("Task {} has not been completed", context.handle());
            return Err(Error::new(ErrorKind::InvalidArgument, err));
        }

        let pseudo = self
            .fetch_pseudo_task(task.0 as *const _ as *const ())
            .expect("task must be registered");
        self.unregister_pseudo_task(pseudo, true)
            .expect("error unregistering task");

        // the task has been completed, we only need to unregister it and free the handle.
        let (handle, data) = context.unregister();
        let private = data.private_data();

        assert!(matches!(self.tasks.remove(&handle), Some(_)));
        debug_assert!(private.dependencies.is_empty());

        self.free_handle(handle);
        Ok(())
    }

    /// # Safety
    ///
    /// See [`unblock_task`](fimo_tasks_int::runtime::IScheduler::unblock_task)
    pub unsafe fn unblock_task(&mut self, task: AssertValidTask) -> Result<(), Error> {
        let context = task.context().borrow();
        let scheduler_data = context.scheduler_data();
        let private = scheduler_data.private_data();

        trace!("Unblocking task {}", context.handle());
        debug!("Task run status: {:?}", context.run_status());
        debug!("Task schedule status: {:?}", context.schedule_status());
        debug!("Task data: {:?}", context.scheduler_data());

        // The status of a task may only be changed by the runtime, which
        // we have unique access to. So we can assume that they won't change
        // sporadically. In case the task is not blocked a runtime may decide
        // to simply do nothing. On the otherhand this may signal a logic error
        // from the part of a caller, so we decide to make it explicit by
        // returning an error.
        if context.schedule_status() != TaskScheduleStatus::Blocked {
            error!(
                "Invalid status for task {}: {:?}",
                context.handle(),
                context.schedule_status()
            );
            let err = format!(
                "Task {} is not blocked, status: {:?}",
                context.handle(),
                context.schedule_status()
            );
            return Err(Error::new(ErrorKind::InvalidArgument, err));
        }

        // Once unblocked we must decide if the task is runnable or if it has
        // other dependencies. If `dependencies` is empty, we know that nothing
        // is preventing the task from running and we simply enqueue it for further
        // processing.
        if private.dependencies.is_empty() {
            trace!("Marking task {} as runnable", context.handle());
            context.set_schedule_status(TaskScheduleStatus::Runnable);

            drop(private);
            drop(context);
            self.enqueue(task);
        } else {
            trace!("Marking task {} as waiting", context.handle());
            context.set_schedule_status(TaskScheduleStatus::Waiting)
        }

        Ok(())
    }

    /// # Safety
    ///
    /// See [`register_or_fetch_pseudo_task`](fimo_tasks_int::runtime::IScheduler::register_or_fetch_pseudo_task)
    pub unsafe fn register_or_fetch_pseudo_task(
        &mut self,
        addr: *const (),
        task: Option<AssertValidTask>,
    ) -> fimo_module::Result<PseudoTask> {
        let addr_key = addr.addr();
        let entry = self.pseudo_tasks.entry(addr_key);

        // SAFETY: We are extending the lifetime of the provided task.
        // We know that it is sound because of the contract of the register function,
        // which states that the task may not be moved for the entire duration
        // for which it is registered with the runtime.
        let task = std::mem::transmute(task);

        // initialize the entry if it does not exit.
        let _ = entry.or_insert(PseudoTaskData {
            waiters: Default::default(),
            task,
        });

        Ok(PseudoTask(addr))
    }

    pub fn fetch_pseudo_task(&self, addr: *const ()) -> Option<PseudoTask> {
        let addr_key = addr.addr();
        if self.pseudo_tasks.get(&addr_key).is_some() {
            Some(PseudoTask(addr))
        } else {
            None
        }
    }

    /// # Safety
    ///
    /// See [`unregister_pseudo_task`](fimo_tasks_int::runtime::IScheduler::unregister_pseudo_task)
    pub unsafe fn unregister_pseudo_task(
        &mut self,
        task: PseudoTask,
        force: bool,
    ) -> fimo_module::Result<()> {
        let addr_key = task.0.addr();
        let data = self
            .pseudo_tasks
            .get(&addr_key)
            .expect("pseudo task must have been registered");

        trace!("Unregistering pseudo task {:?}", task.0);
        debug!("Pseudo task data: {:?}", data);

        if !force && data.task.is_some() {
            let t = data.task.clone().unwrap();
            error!("Can not unregister pseudo task {task:?} associated with task {t:?}");
            let err = format!("can not unregister pseudo task {task:?} associated with task {t:?}");
            return Err(Error::new(ErrorKind::InvalidArgument, err));
        }

        // check that there are no waiters
        if !data.waiters.is_empty() {
            error!("Pseudo task {:?} has waiting tasks", task.0);
            let err = format!("pseudo task {:?} has waiting tasks", task.0);
            return Err(Error::new(ErrorKind::InvalidArgument, err));
        }

        self.pseudo_tasks.remove(&addr_key);

        Ok(())
    }

    /// # Safety
    ///
    /// See [`unregister_pseudo_task_if_empty`](fimo_tasks_int::runtime::IScheduler::unregister_pseudo_task_if_empty)
    pub unsafe fn unregister_pseudo_task_if_empty(
        &mut self,
        task: PseudoTask,
        force: bool,
    ) -> fimo_module::Result<bool> {
        let addr_key = task.0.addr();
        let data = self
            .pseudo_tasks
            .get(&addr_key)
            .expect("pseudo task must have been registered");

        trace!("Unregistering pseudo task {:?}", task.0);
        debug!("Pseudo task data: {:?}", data);

        if !force && data.task.is_some() {
            let t = data.task.clone().unwrap();
            error!("Can not unregister pseudo task {task:?} associated with task {t:?}");
            let err = format!("can not unregister pseudo task {task:?} associated with task {t:?}");
            return Err(Error::new(ErrorKind::InvalidArgument, err));
        }

        // check that there are no waiters
        if !data.waiters.is_empty() {
            Ok(false)
        } else {
            debug!("Pseudo task {:?} is not empty -- Skipping", task.0);
            self.pseudo_tasks.remove(&addr_key);
            Ok(true)
        }
    }

    /// # Safety
    ///
    /// See [`wait_task_on`](fimo_tasks_int::runtime::IScheduler::wait_task_on)
    pub unsafe fn wait_task_on(
        &mut self,
        task: AssertValidTask,
        on: PseudoTask,
        data_addr: Option<&mut MaybeUninit<WakeupToken>>,
        token: WaitToken,
    ) -> fimo_module::Result<()> {
        let on_addr = on.0.addr();

        let context = task.context().borrow();
        let on_data = self
            .pseudo_tasks
            .get_mut(&on_addr)
            .expect("pseudo task must be registered");

        let scheduler_data = context.scheduler_data();

        trace!("Setting task {} to wait for task {on:?}", context.handle(),);
        debug!("Task run status: {:?}", context.run_status());
        debug!("Task schedule status: {:?}", context.schedule_status());
        debug!("Task data: {:?}", context.scheduler_data());

        let mut private = scheduler_data.private_data_mut();

        // check that the task has not been completed.
        if context.run_status() == TaskRunStatus::Completed {
            error!("Can not wait, task {} already completed", context.handle());
            let err = format!("task {} already complete", context.handle());
            return Err(Error::new(ErrorKind::InvalidArgument, err));
        }

        // a task may not wait on another task multiple times.
        if private.dependencies.contains(&on) {
            error!("Task {} waiting on {on:?} multiple times", context.handle());
            let err = format!("task {} is already waiting on {on:?}", context.handle());
            return Err(Error::new(ErrorKind::InvalidArgument, err));
        }

        if let Some(on) = on_data.task.clone() {
            let handle = context.handle();

            if task == on {
                error!("Task {handle} waiting on itself",);
                let err = format!("a task may not wait on itself, handle: {handle}",);
                return Err(Error::new(ErrorKind::InvalidArgument, err));
            }

            let on_context = on.context().borrow();
            let on_handle = on_context.handle();

            // skip if the dependency has already been completed.
            if on_context.run_status() == TaskRunStatus::Completed {
                trace!("Skipping wait for task {handle}, task {on_handle} already completed",);

                // The caller relies on the fact, that the data is initialized once the task
                // has been woken up. Normally that would be implemented in the wake routine,
                // but as we are skipping the wait entirely it is now our responsibility.
                if let Some(data_addr) = data_addr {
                    data_addr.write(WakeupToken::Skipped);
                }

                return Ok(());
            }
        }

        // Insert the address of the data in the map of the task.
        debug_assert!(!private.dependency_data_addr.contains_key(&on));
        if let Some(addr) = data_addr {
            private
                .dependency_data_addr
                .insert(on, AtomicPtr::new(addr.as_mut_ptr()));
        }

        // register the dependency
        private.dependencies.insert(on);
        on_data.waiters.push(Waiter(Reverse(task.clone()), token));

        // This method accepts arbitrary tasks, so it is possible that the task
        // was already inserted into the queue. In that case we simply mark it
        // as `Waiting` and let the scheduling logic handle the rest.
        //
        // In the future we may disallow this entirely by allowing only the own
        // task to wait on an arbitrary task.
        if context.schedule_status() == TaskScheduleStatus::Runnable {
            trace!("Set task {} to waiting", context.handle());

            // SAFETY: We know that only the runtime is allowed to modify the status
            // of a task. By that logic we can assume that status won't change unless
            // we are the ones changing it.
            context.set_schedule_status(TaskScheduleStatus::Waiting)
        }

        Ok(())
    }

    /// # Safety
    ///
    /// See [`notify_one`](fimo_tasks_int::runtime::IScheduler::notify_one)
    pub unsafe fn notify_one(
        &mut self,
        task: PseudoTask,
        data_callback: FfiFn<'_, dyn FnOnce(NotifyResult) -> WakeupToken + '_, u8>,
    ) -> fimo_module::Result<NotifyResult> {
        let task_data = self
            .pseudo_tasks
            .get_mut(&task.0.addr())
            .expect("pseudo task must be registered");

        trace!("Notifying one waiter of pseudo task {task:?}");
        debug!("Pseudo task data: {:?}", task_data);

        if let Some(task) = task_data.task.clone() {
            let context = task.context().borrow();
            let scheduler_data = context.scheduler_data();

            debug!("Associated task run status: {:?}", context.run_status());
            debug!(
                "Associated task schedule status: {:?}",
                context.schedule_status()
            );
            debug!("Associated task data: {:?}", scheduler_data);
        }

        let result = if task_data.waiters.is_empty() {
            NotifyResult {
                notified_tasks: 0,
                remaining_tasks: 0,
            }
        } else {
            NotifyResult {
                notified_tasks: 1,
                remaining_tasks: task_data.waiters.len() - 1,
            }
        };
        let data = data_callback(result);

        // The actual logic of waking a task is implemented in the `notify_waiter`
        // method. We must simply select a suitable waiter. This is done by removing
        // the first waiter from the `waiters` heap, which returns the waiter with the
        // highest priority.
        if let Some(Waiter(Reverse(waiter), _)) = task_data.waiters.pop() {
            self.notify_waiter(task, waiter, data);
        }

        Ok(result)
    }

    /// # Safety
    ///
    /// See [`notify_all`](fimo_tasks_int::runtime::IScheduler::notify_all)
    pub unsafe fn notify_all(
        &mut self,
        task: PseudoTask,
        data: WakeupToken,
    ) -> fimo_module::Result<usize> {
        let task_data = self
            .pseudo_tasks
            .get_mut(&task.0.addr())
            .expect("pseudo task must be registered");

        trace!("Notifying all waiters of pseudo task {task:?}");
        debug!("Pseudo task data: {:?}", task_data);

        if let Some(task) = task_data.task.clone() {
            let context = task.context().borrow();
            let scheduler_data = context.scheduler_data();

            debug!("Associated task run status: {:?}", context.run_status());
            debug!(
                "Associated task schedule status: {:?}",
                context.schedule_status()
            );
            debug!("Associated task data: {:?}", scheduler_data);
        }

        let mut waiters = std::mem::take(&mut task_data.waiters);
        let num_waiters = waiters.len();

        // The actual logic of waking a task is implemented in the `notify_waiter`
        // method. We simply call that method with every waiter in the heap.
        while let Some(Waiter(Reverse(waiter), _)) = waiters.pop() {
            self.notify_waiter(task, waiter, data)
        }

        Ok(num_waiters)
    }

    /// # Safety
    ///
    /// See [`notify_filter`](fimo_tasks_int::runtime::IScheduler::notify_filter)
    pub unsafe fn notify_filter(
        &mut self,
        task: fimo_tasks_int::raw::PseudoTask,
        mut filter: FfiFn<
            '_,
            dyn FnMut(WaitToken) -> fimo_tasks_int::runtime::NotifyFilterOp + '_,
            u8,
        >,
        data_callback: FfiFn<'_, dyn FnOnce(NotifyResult) -> WakeupToken + '_, u8>,
    ) -> fimo_module::Result<NotifyResult> {
        let task_data = self
            .pseudo_tasks
            .get_mut(&task.0.addr())
            .expect("pseudo task must be registered");

        trace!("Notifying waiters of pseudo task {task:?}");
        debug!("Pseudo task data: {:?}", task_data);

        if let Some(task) = task_data.task.clone() {
            let context = task.context().borrow();
            let scheduler_data = context.scheduler_data();

            debug!("Associated task run status: {:?}", context.run_status());
            debug!(
                "Associated task schedule status: {:?}",
                context.schedule_status()
            );
            debug!("Associated task data: {:?}", scheduler_data);
        }

        let mut retain_queue = Vec::new();
        let mut notify_queue = Vec::new();

        // Filter the waiter queue.
        while let Some(Waiter(t, token)) = task_data.waiters.pop() {
            let op = filter(token);
            match op {
                // Push the task to the notify queue for further processing.
                fimo_tasks_int::runtime::NotifyFilterOp::Notify => {
                    notify_queue.push(t);
                }

                // Reinsert the task into the waiters queue.
                fimo_tasks_int::runtime::NotifyFilterOp::Stop => {
                    task_data.waiters.push(Waiter(t, token));
                    break;
                }

                // Push the task to the retain queue for reinsertion.
                fimo_tasks_int::runtime::NotifyFilterOp::Skip => {
                    retain_queue.push(Waiter(t, token));
                }
            }
        }

        // Reinsert the skipped waiters into the queue.
        task_data.waiters.extend(retain_queue);

        let result = NotifyResult {
            notified_tasks: notify_queue.len(),
            remaining_tasks: task_data.waiters.len(),
        };
        let data = data_callback(result);

        // The actual logic of waking a task is implemented in the `notify_waiter`
        // method. Wake all the selected waiters.
        for Reverse(waiter) in notify_queue {
            self.notify_waiter(task, waiter, data);
        }

        Ok(result)
    }

    /// # Safety
    ///
    /// May only be called by [`notify_one`](#method.notify_one),
    /// [`notify_all`](#method.notify_all) and
    /// [`notify_filter`](#method.notify_filter).
    pub unsafe fn notify_waiter(
        &mut self,
        task: PseudoTask,
        waiter: AssertValidTask,
        data: WakeupToken,
    ) {
        let waiter_context = waiter.context().borrow();
        let waiter_data = waiter_context.scheduler_data();

        trace!("Notifying waiter {} of {task:?}", waiter_context.handle());
        debug!("Waiter run status: {:?}", waiter_context.run_status());
        debug!(
            "Waiter schedule status: {:?}",
            waiter_context.schedule_status()
        );
        debug!("Task data: {:?}", waiter_context.scheduler_data());

        let mut waiter_private = waiter_data.private_data_mut();
        assert!(waiter_private.dependencies.remove(&task));

        // Write the message into the data if it points to a valid address.
        if let Some(mut data_addr) = waiter_private.dependency_data_addr.remove(&task) {
            // SAFETY: We know by the contract of the `wait_on` method, that the data_addr
            // must be either valid or null. In the former case, we also know that the task
            // did not continue its execution, as it was put to wait. Therefore the address
            // must have remained valid.
            data_addr.get_mut().write(data);
        }

        // make the task runnable if nothing prevents it.
        let schedule_status = waiter_context.schedule_status();
        if waiter_private.dependencies.is_empty() && schedule_status == TaskScheduleStatus::Waiting
        {
            trace!("Waking up task {}", waiter_context.handle());
            waiter_context.set_schedule_status(TaskScheduleStatus::Runnable);
            drop(waiter_private);
            drop(waiter_context);
            self.enqueue(waiter);
        }
    }
}

pub(crate) struct Msg<'a> {
    pub task: AssertValidTask,
    pub data: MsgData<'a>,
}

#[derive(Debug)]
pub(crate) enum MsgData<'a> {
    Completed {
        aborted: bool,
    },
    #[allow(clippy::type_complexity)]
    Yield {
        f: RawFfiFn<
            dyn FnOnce(&mut DynObj<dyn IScheduler + '_>, &DynObj<dyn IRawTask + '_>) + Send + 'a,
        >,
    },
}

impl MsgData<'_> {
    #[inline]
    pub fn msg_type(&self) -> &str {
        match &self {
            MsgData::Completed { .. } => "Completed",
            MsgData::Yield { .. } => "Yield",
        }
    }
}

#[repr(transparent)]
#[derive(Clone)]
pub(crate) struct AssertValidTask(&'static DynObj<dyn IRawTask + 'static>);

impl AssertValidTask {
    /// Constructs a `AssertValidTask` from an [`IRawTask`].
    ///
    /// # Safety
    ///
    /// The caller must ensure that the task is registered
    /// with the current runtime.
    #[inline]
    pub unsafe fn from_raw(task: &DynObj<dyn IRawTask + '_>) -> Self {
        let task = std::mem::transmute(task);
        Self(task)
    }

    /// Extracts the contained [`IRawTask`].
    #[inline]
    pub fn as_raw<'t>(&self) -> &DynObj<dyn IRawTask + 't> {
        self.0
    }

    /// Shorthand for `self.name().unwrap_or("unnamed")`.
    #[inline]
    pub fn name(&self) -> Option<&str> {
        self.0.name()
    }

    /// Shorthand for `self.name().unwrap_or("unnamed")`.
    #[inline]
    pub fn resolved_name(&self) -> &str {
        self.0.resolved_name()
    }

    /// Extracts the starting priority of the task.
    #[inline]
    pub fn priority(&self) -> TaskPriority {
        self.0.priority()
    }

    /// Returns a reference to the context.
    #[inline]
    pub fn context(&self) -> &AtomicRefCell<SchedulerContext<'_>> {
        let context = self.0.context();
        // SAFETY: SchedulerContext has a  transparent repr so it should be safe
        unsafe { std::mem::transmute(context) }
    }

    #[inline]
    pub fn context_atomic(&self) -> AtomicISchedulerContext<'_> {
        self.0.context_atomic()
    }
}

impl PartialEq for AssertValidTask {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        // SAFETY: The invariant was checked at construction time.
        unsafe {
            self.context_atomic().handle().assume_init()
                == other.context_atomic().handle().assume_init()
        }
    }
}

impl PartialOrd for AssertValidTask {
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Eq for AssertValidTask {}

impl Ord for AssertValidTask {
    #[inline]
    fn cmp(&self, other: &Self) -> Ordering {
        self.priority().cmp(&other.priority())
    }
}

impl Debug for AssertValidTask {
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("AssertValidTask")
            .field(&self.name())
            .field(&self.priority())
            .field(&self.context_atomic())
            .finish()
    }
}

#[repr(transparent)]
pub(crate) struct SchedulerContext<'a>(DynObj<dyn ISchedulerContext + 'a>);

impl<'a> SchedulerContext<'a> {
    /// Extracts the handle to the task.
    #[inline]
    pub fn handle(&self) -> TaskHandle {
        debug_assert!(self.is_registered());

        // SAFETY: Being registered is an invariant of an `SchedulerContext`.
        // Every registered task has a valid handle.
        unsafe { self.0.handle().assume_init() }
    }

    /// Checks whether the context has been marked as registered.
    #[inline]
    pub fn is_registered(&self) -> bool {
        self.0.is_registered()
    }

    /// Marks the context as unregistered.
    ///
    /// # Panics
    ///
    /// May panic if the task is not registered.
    #[inline]
    fn unregister(&mut self) -> (TaskHandle, ObjBox<ContextData>) {
        // SAFETY: Can only be called from a scheduler.
        let (handle, data) = unsafe { self.0.unregister() };
        let data = data.expect("Scheduler data taken from registered task");
        let data = ObjBox::downcast(data).expect("Invalid scheduler data");
        (handle, data)
    }

    /// Extracts the resume time.
    #[inline]
    pub fn resume_time(&self) -> SystemTime {
        self.0.resume_time()
    }

    /// Extracts the assigned worker.
    #[inline]
    pub fn worker(&self) -> Option<WorkerId> {
        self.0.worker()
    }

    /// Sets a new worker.
    ///
    /// Passing in `None` will automatically select an appropriate worker.
    ///
    /// # Note
    ///
    /// Must be implemented atomically.
    ///
    /// # Safety
    ///
    /// Behavior is undefined if any of the following conditions are violated:
    ///
    /// * A worker associated with the provided [`WorkerId`] does not exist.
    /// * The task has yielded it's execution and has cached thread-local variables.
    /// * Is used by someone other than the runtime implementation and the task is registered.
    #[inline]
    pub unsafe fn set_worker(&self, worker: Option<WorkerId>) {
        self.0.set_worker(worker)
    }

    /// Clears the requests and returns it.
    #[inline]
    pub unsafe fn clear_request(&self) -> StatusRequest {
        self.0.clear_request()
    }

    /// Extracts the current run status.
    #[inline]
    pub fn run_status(&self) -> TaskRunStatus {
        self.0.run_status()
    }

    /// Sets a new run status.
    #[inline]
    pub(super) fn set_run_status(&self, status: TaskRunStatus) {
        // SAFETY: Can only be called from a scheduler.
        unsafe { self.0.set_run_status(status) }
    }

    /// Extracts the current schedule status.
    #[inline]
    pub fn schedule_status(&self) -> TaskScheduleStatus {
        self.0.schedule_status()
    }

    /// Sets a new schedule status.
    ///
    /// # Safety
    ///
    /// Behavior is undefined if not called from a task scheduler.
    #[inline]
    pub unsafe fn set_schedule_status(&self, status: TaskScheduleStatus) {
        self.0.set_schedule_status(status)
    }

    /// Checks whether the task is empty.
    ///
    /// # Note
    ///
    /// May change after the task is registered with a runtime.
    #[inline]
    pub fn is_empty_task(&self) -> bool {
        self.0.is_empty_task()
    }

    /// Checks whether the task is panicking.
    #[inline]
    pub fn is_panicking(&self) -> bool {
        self.0.is_panicking()
    }

    /// Sets the panicking flag.
    ///
    /// # Panics
    ///
    /// May panic if the flag is already set.
    ///
    /// # Safety
    ///
    /// Behavior is undefined if not called from a task scheduler.
    #[inline]
    pub(super) fn set_panic(&mut self, panic: Option<ObjBox<PanicData>>) {
        let panic = panic.map(|p| {
            let p: ObjBox<DynObj<dyn IRustPanicData + Send>> = ObjBox::coerce_obj(p);
            ObjBox::cast_super(p)
        });

        // SAFETY: We are part of the scheduler.
        unsafe { self.0.set_panic(panic) }
    }

    /// Takes the panic data from the task.
    ///
    /// # Panics
    ///
    /// May panic if the task is registered.
    ///
    /// # Safety
    ///
    /// Behavior is undefined if the task has not completed or aborted it's execution.
    #[inline]
    unsafe fn take_panic_data(&mut self) -> Option<ObjBox<PanicData>> {
        self.0
            .take_panic_data()
            .map(|p| ObjBox::downcast(p).expect("Invalid panic data"))
    }

    /// Calls the cleanup function.
    #[inline]
    pub fn cleanup(&mut self) {
        self.0.cleanup()
    }

    /// Fetches a reference to the scheduler data.
    #[inline]
    pub fn scheduler_data(&self) -> &ContextData {
        self.0
            .scheduler_data()
            .expect("Invalid scheduler data")
            .downcast()
            .expect("Invalid scheduler data")
    }
}

#[derive(Debug, ObjectId)]
#[fetch_vtable(uuid = "c68fe659-beef-4341-9b75-f54b0ef387ff")]
pub(crate) struct ContextData {
    shared: AtomicRefCell<SharedContext>,
    private: AtomicRefCell<PrivateContext>,
}

/// Data used by the Scheduler and worker pool.
pub(crate) struct SharedContext {
    context: Option<Context>,
    branch: Option<SpanStackId>,
    panic: Option<ObjBox<PanicData>>,
    entry_func: Option<FfiFn<'static, dyn FnOnce() + Send + 'static>>,
}

// SAFETY: The api ensures that the type can be shared with other threads.
unsafe impl Sync for SharedContext {}

/// Data used only by the Scheduler.
///
///
#[derive(Debug)]
pub(super) struct PrivateContext {
    in_queue: bool,
    processing: bool,
    /// Slot a task was assigned to.
    pub slot: Option<TaskSlot>,
    /// Dependencies on which the task must wait for.
    pub dependencies: BTreeSet<PseudoTask>,
    /// Address of the WakeupData for each dependency.
    dependency_data_addr: BTreeMap<PseudoTask, AtomicPtr<WakeupToken>>,
}

impl ContextData {
    #[inline]
    fn new(f: Option<FfiFn<'_, dyn FnOnce() + Send + '_>>) -> Self {
        Self {
            shared: AtomicRefCell::new(SharedContext::new(f)),
            private: AtomicRefCell::new(PrivateContext::new()),
        }
    }

    /// Borrows the shared portion of the context mutably.
    #[inline]
    pub fn shared_data_mut(&self) -> AtomicRefMut<'_, SharedContext> {
        self.shared.borrow_mut()
    }

    /// Borrows the private portion of the context.
    #[inline]
    pub(super) fn private_data(&self) -> AtomicRef<'_, PrivateContext> {
        self.private.borrow()
    }

    /// Borrows the private portion of the context mutably.
    #[inline]
    pub(super) fn private_data_mut(&self) -> AtomicRefMut<'_, PrivateContext> {
        self.private.borrow_mut()
    }
}

impl SharedContext {
    #[inline]
    fn new(f: Option<FfiFn<'_, dyn FnOnce() + Send + '_>>) -> Self {
        Self {
            context: None,
            branch: None,
            panic: None,
            // SAFETY: Ideally we would like to have a way to specify the concept of a
            // minimal lifetime. As that is currently not possible, we choose the
            // `'static` lifetime as a placeholder. As long as we ensure that the
            // task outlives the function it should be sound.
            entry_func: unsafe { std::mem::transmute(f) },
        }
    }

    #[inline]
    pub fn is_empty_context(&self) -> bool {
        self.context.is_none()
    }

    #[inline]
    pub fn take_context(&mut self) -> Option<Context> {
        self.context.take()
    }

    #[inline]
    pub fn set_context(&mut self, c: Context) {
        self.context = Some(c)
    }

    #[inline]
    pub fn branch(&mut self) -> SpanStackId {
        self.branch.expect("The task has no registered branch")
    }

    #[inline]
    pub fn take_branch(&mut self) -> Option<SpanStackId> {
        self.branch.take()
    }

    #[inline]
    pub fn set_branch(&mut self, branch: SpanStackId) {
        debug_assert!(self.branch.is_none());
        self.branch = Some(branch)
    }

    #[inline]
    pub fn take_panic(&mut self) -> Option<ObjBox<PanicData>> {
        self.panic.take()
    }

    #[inline]
    pub fn set_panic(&mut self, panic: ObjBox<PanicData>) {
        self.panic = Some(panic)
    }

    #[inline]
    pub fn take_entry_func(&mut self) -> Option<FfiFn<'static, dyn FnOnce() + Send + 'static>> {
        self.entry_func.take()
    }
}

impl PrivateContext {
    #[inline]
    fn new() -> Self {
        Self {
            in_queue: false,
            processing: false,
            slot: None,
            dependencies: Default::default(),
            dependency_data_addr: Default::default(),
        }
    }

    /// Indicates whether the task is being processed.
    #[inline]
    pub fn is_processing(&self) -> bool {
        self.processing
    }

    /// Sets the processing flag to `p`.
    ///
    /// # Safety
    ///
    /// May only be used by the [`process_msg`](super::TaskScheduler::process_msg)
    /// function before and after processing the message.
    #[inline]
    pub unsafe fn toggle_processing(&mut self, p: bool) {
        debug_assert_ne!(self.processing, p);
        self.processing = p;
    }

    /// Indicates whether the task is already in the task queue.
    #[inline]
    pub fn is_in_queue(&self) -> bool {
        self.in_queue
    }

    /// Indicates that the task is present in the task queue.
    ///
    /// # Safety
    ///
    /// The task must be in the task queue.
    #[inline]
    unsafe fn assert_in_queue(&mut self) {
        self.in_queue = true;
    }

    /// Indicates that the task is not present in the task queue.
    ///
    /// # Safety
    ///
    /// The task must not be in the task queue.
    #[inline]
    pub unsafe fn assert_not_in_queue(&mut self) {
        debug_assert!(self.in_queue);
        self.in_queue = false;
    }
}

impl Debug for SharedContext {
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SharedContext")
            .field("context", &self.context)
            .finish_non_exhaustive()
    }
}

#[derive(Default, Debug)]
struct PseudoTaskData {
    waiters: BinaryHeap<Waiter>,
    task: Option<AssertValidTask>,
}

#[derive(Debug)]
struct Waiter(pub Reverse<AssertValidTask>, pub WaitToken);

impl PartialEq for Waiter {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.0.eq(&other.0)
    }
}

impl Eq for Waiter {}

impl PartialOrd for Waiter {
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.0.partial_cmp(&other.0)
    }
}

impl Ord for Waiter {
    #[inline]
    fn cmp(&self, other: &Self) -> Ordering {
        self.0.cmp(&other.0)
    }
}

#[derive(ObjectId)]
#[fetch_vtable(
    uuid = "d2e5a6f3-d5a0-41f0-a6b1-62d543c5c46b",
    interfaces(IRustPanicData)
)]
pub(crate) struct PanicData {
    data: Option<Box<dyn Any + Send + 'static>>,
}

impl PanicData {
    #[inline]
    pub fn new(e: Box<dyn Any + Send + 'static>) -> ObjBox<Self> {
        ObjBox::new(Self { data: Some(e) })
    }
}

impl IRustPanicData for PanicData {
    #[inline]
    unsafe fn take_rust_panic_impl(&mut self) -> Box<dyn Any + Send + 'static> {
        // safety: the function is called only once
        self.data.take().unwrap_unchecked()
    }
}
