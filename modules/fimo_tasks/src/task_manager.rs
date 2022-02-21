use crate::stack_allocator::TaskSlot;
use context::Context;
use fimo_ffi::fn_wrapper::RawFnOnce;
use fimo_ffi::marker::{SendMarker, SendSyncMarker};
use fimo_ffi::vtable::IBase;
use fimo_ffi::ObjBox;
use fimo_module::{impl_vtable, is_object, Error, ErrorKind};
use fimo_tasks_int::raw::{
    IRawTask, IRustPanicData, IRustPanicDataVTable, ISchedulerContext, StatusRequest, TaskHandle,
    TaskPriority, TaskRunStatus, TaskScheduleStatus, WorkerId,
};
use fimo_tasks_int::runtime::IScheduler;
use log::{debug, error, trace};
use std::any::Any;
use std::cmp::{Ordering, Reverse};
use std::collections::{BTreeMap, BTreeSet, BinaryHeap, VecDeque};
use std::ops::RangeFrom;
use std::ptr::NonNull;
use std::sync::mpsc::Receiver;
use std::time::SystemTime;

#[derive(Debug)]
pub(crate) struct TaskManager {
    msg_receiver: Receiver<Msg>,
    handle_iter: RangeFrom<usize>,
    free_handles: VecDeque<TaskHandle>,
    tasks: BTreeMap<TaskHandle, &'static RawTask>,
    processing_queue: BinaryHeap<Reverse<&'static RawTask>>,
}

impl TaskManager {
    pub fn new(msg_receiver: Receiver<Msg>) -> Self {
        trace!("Initializing the task manager");
        Self {
            msg_receiver,
            handle_iter: 0..,
            free_handles: Default::default(),
            tasks: Default::default(),
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

            unsafe { self.free_handle_reuse(handle) };
        }
    }

    #[inline]
    pub fn take_messages(&mut self) -> Vec<Msg> {
        self.msg_receiver.try_iter().collect()
    }

    /// # Safety
    ///
    /// A task may appear only once in the queue.
    #[inline]
    pub unsafe fn enqueue(&mut self, task: &'static RawTask) {
        self.processing_queue.push(Reverse(task))
    }

    #[inline]
    pub fn enqueue_checked(&mut self, task: &'static RawTask) {
        unsafe {
            let data = task.scheduler_context().scheduler_data();
            if !data.processing {
                self.enqueue(task)
            }
        }
    }

    #[inline]
    pub fn clear_queue(&mut self) -> BinaryHeap<Reverse<&'static RawTask>> {
        std::mem::take(&mut self.processing_queue)
    }

    pub fn find_task(&self, handle: TaskHandle) -> Result<&'static IRawTask, Error> {
        trace!("Searching for task {}", handle);
        if let Some(&task) = self.tasks.get(&handle) {
            trace!("Found task");
            Ok(&task.0)
        } else {
            error!("Task {} not found", handle);
            let err = format!("Task {} is not registered", handle);
            Err(Error::new(ErrorKind::InvalidArgument, err))
        }
    }

    pub fn register_task(
        &mut self,
        task: &'static IRawTask,
        wait_on: &[TaskHandle],
    ) -> Result<(), Error> {
        trace!("Registering new task {:?}", task.resolved_name());

        let context = unsafe { task.scheduler_context_mut() };
        if context.is_registered() {
            error!("Task {:?} already registered", task.resolved_name());
            let err = format!("The task {:?} is already registered", task.resolved_name());
            return Err(Error::new(ErrorKind::InvalidArgument, err));
        }

        // register the handle and data internally.
        let handle = self.allocate_handle()?;
        let data = ObjBox::new(ContextData::new());
        unsafe { context.register(handle, Some(ObjBox::coerce_object(data))) }

        let task: &'static RawTask = unsafe { &*(task as *const _ as *const RawTask) };

        // clear the task.
        let context = unsafe { task.scheduler_context_mut() };
        unsafe { context.set_run_status(TaskRunStatus::Idle) };
        unsafe { context.set_schedule_status(TaskScheduleStatus::Processing) };
        unsafe { context.take_panic_data() };

        // wait on all dependencies
        for dep in wait_on {
            if let Some(&dep) = self.tasks.get(dep) {
                if let Err(e) = self.wait_task_on(task, dep) {
                    error!("Aborting registration, error: {}", e);
                    let (handle, _) = unsafe { context.unregister() };
                    unsafe { self.free_handle_reuse(handle) };
                    return Err(e);
                }
            }
        }

        let request = context.clear_request();

        match request {
            StatusRequest::None => {
                let data = unsafe { context.scheduler_data() };
                if data.dependencies.is_empty() {
                    trace!(
                        "Registered task {:?} with id {} as runnable",
                        task.resolved_name(),
                        context.handle()
                    );
                    unsafe { context.set_schedule_status(TaskScheduleStatus::Runnable) };
                    unsafe { self.enqueue(task) };
                } else {
                    trace!(
                        "Registered task {:?} with id {} as waiting",
                        task.resolved_name(),
                        context.handle()
                    );
                    unsafe { context.set_schedule_status(TaskScheduleStatus::Waiting) };
                }
            }
            StatusRequest::Block => {
                trace!(
                    "Registered task {:?} with id {} as blocked",
                    task.resolved_name(),
                    context.handle()
                );
                unsafe { context.set_schedule_status(TaskScheduleStatus::Blocked) };
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
                let (handle, _) = unsafe { context.unregister() };
                unsafe { self.free_handle_reuse(handle) };
                return Err(Error::new(ErrorKind::InvalidArgument, err));
            }
        }

        // register the task to the runtime.
        self.tasks.insert(handle, task);

        Ok(())
    }

    pub fn unregister_task(&mut self, task: &RawTask) -> Result<(), Error> {
        // safety: we are inside the scheduler.
        let context = unsafe { task.scheduler_context_mut() };

        trace!("Unregistering task {}", context.handle());
        debug!("Task run status: {:?}", context.run_status());
        debug!("Task schedule status: {:?}", context.schedule_status());
        debug!("Number of waiters: {}", unsafe {
            context.scheduler_data().waiters.len()
        });
        debug!("Number of dependencies: {}", unsafe {
            context.scheduler_data().dependencies.len()
        });

        if context.run_status() != TaskRunStatus::Completed {
            error!("Task {} has not been completed", context.handle());
            let err = format!("Task {} has not been completed", context.handle());
            return Err(Error::new(ErrorKind::InvalidArgument, err));
        }

        // the task has been completed, we only need to unregister it and free the handle.
        let (handle, data) = unsafe { context.unregister() };

        debug_assert!(matches!(self.tasks.remove(&handle), Some(_)));
        debug_assert!(data.dependencies.is_empty());
        debug_assert!(data.waiters.is_empty());

        self.free_handle(handle);
        Ok(())
    }

    pub fn wait_task_on(
        &mut self,
        task: &'static RawTask,
        wait_on: &'static RawTask,
    ) -> Result<(), Error> {
        // safety: we are inside the scheduler.
        let context = unsafe { task.scheduler_context_mut() };
        let wait_on_context = unsafe { wait_on.scheduler_context_mut() };

        trace!(
            "Setting task {} to wait for task {}",
            context.handle(),
            wait_on_context.handle()
        );
        debug!("Task run status: {:?}", context.run_status());
        debug!("Task schedule status: {:?}", context.schedule_status());
        debug!("Number of waiters: {}", unsafe {
            context.scheduler_data().waiters.len()
        });
        debug!("Number of dependencies: {}", unsafe {
            context.scheduler_data().dependencies.len()
        });

        if task == wait_on {
            error!("Task {} waiting on itself", context.handle());
            let err = format!(
                "A task may not wait on itself, handle: {}",
                context.handle()
            );
            return Err(Error::new(ErrorKind::InvalidArgument, err));
        }

        // check that the task has not been completed.
        if context.run_status() == TaskRunStatus::Completed {
            error!("Can not wait, task {} already completed", context.handle());
            let err = format!("The task {} has already been completed", context.handle());
            return Err(Error::new(ErrorKind::InvalidArgument, err));
        }

        // skip if the dependency has already been completed.
        if wait_on_context.run_status() == TaskRunStatus::Completed {
            trace!(
                "Skipping wait for task {}, task {} already completed",
                context.handle(),
                wait_on_context.handle()
            );
            return Ok(());
        }

        // a task may not wait on another task multiple times.
        if unsafe { context.scheduler_data().dependencies.contains(wait_on) } {
            error!(
                "Task {} waiting on {} multiple times",
                context.handle(),
                context.handle()
            );
            let err = format!(
                "The task {} is already waiting on {}",
                context.handle(),
                wait_on_context.handle()
            );
            return Err(Error::new(ErrorKind::InvalidArgument, err));
        }

        let scheduler_data = unsafe { context.scheduler_data_mut() };
        let wait_on_scheduler_data = unsafe { wait_on_context.scheduler_data_mut() };

        // register the dependency
        scheduler_data.dependencies.insert(wait_on);
        wait_on_scheduler_data.waiters.push(Reverse(task));

        // if the task is marked as runnable (i.e. is inserted into the processing queue),
        // we set it to waiting.
        if context.schedule_status() == TaskScheduleStatus::Runnable {
            trace!("Set task {} to waiting", context.handle());
            unsafe { context.set_schedule_status(TaskScheduleStatus::Waiting) }
        }

        Ok(())
    }

    pub unsafe fn notify_one(&mut self, task: &RawTask) -> Result<Option<usize>, Error> {
        // safety: we are inside the scheduler.
        let context = task.scheduler_context_mut();

        trace!("Notifying one waiter of task {}", context.handle());
        debug!("Task run status: {:?}", context.run_status());
        debug!("Task schedule status: {:?}", context.schedule_status());
        debug!(
            "Number of waiters: {}",
            context.scheduler_data().waiters.len()
        );
        debug!(
            "Number of dependencies: {}",
            context.scheduler_data().dependencies.len()
        );

        if !context.is_registered() {
            error!("Task {} is not registered", context.handle());
            let err = format!("Task {} is not registered", context.handle());
            return Err(Error::new(ErrorKind::InvalidArgument, err));
        }

        // safety: the scheduler data is only available from
        // the scheduler and can therefore be modified.
        let scheduler_data = context.scheduler_data_mut();
        if let Some(Reverse(waiter)) = scheduler_data.waiters.pop() {
            self.notify_waiter(task, waiter);
            Ok(Some(scheduler_data.waiters.len()))
        } else {
            trace!("No waiters skipping");
            Ok(None)
        }
    }

    pub unsafe fn notify_all(&mut self, task: &RawTask) -> Result<usize, Error> {
        // safety: we are inside the scheduler.
        let context = task.scheduler_context_mut();

        trace!("Notifying all waiters of task {}", context.handle());
        debug!("Task run status: {:?}", context.run_status());
        debug!("Task schedule status: {:?}", context.schedule_status());
        debug!(
            "Number of waiters: {}",
            context.scheduler_data().waiters.len()
        );
        debug!(
            "Number of dependencies: {}",
            context.scheduler_data().dependencies.len()
        );

        if !context.is_registered() {
            error!("Task {} is not registered", context.handle());
            let err = format!("Task {} is not registered", context.handle());
            return Err(Error::new(ErrorKind::InvalidArgument, err));
        }

        // safety: the scheduler data is only available from
        // the scheduler and can therefore be modified.
        let scheduler_data = context.scheduler_data_mut();

        let num_waiters = scheduler_data.waiters.len();
        while let Some(Reverse(waiter)) = scheduler_data.waiters.pop() {
            self.notify_waiter(task, waiter)
        }

        Ok(num_waiters)
    }

    pub unsafe fn notify_waiter(&mut self, task: &RawTask, waiter: &'static RawTask) {
        let task_context = task.scheduler_context_mut();
        let waiter_context = waiter.scheduler_context_mut();

        trace!(
            "Notifying waiter {} that {} finished",
            waiter_context.handle(),
            task_context.handle()
        );
        debug!("Waiter run status: {:?}", waiter_context.run_status());
        debug!(
            "Waiter schedule status: {:?}",
            waiter_context.schedule_status()
        );
        debug!(
            "Number of waiters: {}",
            waiter_context.scheduler_data().waiters.len()
        );
        debug!(
            "Number of dependencies: {}",
            waiter_context.scheduler_data().dependencies.len()
        );

        // the waiter must be either blocked or waiting.
        debug_assert!(matches!(
            waiter_context.schedule_status(),
            TaskScheduleStatus::Blocked | TaskScheduleStatus::Waiting
        ));

        // cache schedule status.
        let handle = waiter_context.handle();
        let schedule_status = waiter_context.schedule_status();

        let waiter_data = waiter_context.scheduler_data_mut();
        debug_assert!(waiter_data.dependencies.remove(task));

        // make the task runnable if nothing prevents it.
        if waiter_data.dependencies.is_empty() && schedule_status == TaskScheduleStatus::Waiting {
            trace!("Waking up task {}", handle);
            waiter_context.set_schedule_status(TaskScheduleStatus::Runnable);
            self.enqueue_checked(waiter);
        }
    }

    pub fn unblock_task(&mut self, task: &'static RawTask) -> Result<(), Error> {
        let context = task.scheduler_context();

        trace!("Unblocking task {}", context.handle());
        debug!("Task run status: {:?}", context.run_status());
        debug!("Task schedule status: {:?}", context.schedule_status());
        debug!("Number of waiters: {}", unsafe {
            context.scheduler_data().waiters.len()
        });
        debug!("Number of dependencies: {}", unsafe {
            context.scheduler_data().dependencies.len()
        });

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

        let scheduler_data = unsafe { context.scheduler_data() };
        if scheduler_data.dependencies.is_empty() {
            trace!("Marking task {} as runnable", context.handle());
            unsafe { context.set_schedule_status(TaskScheduleStatus::Runnable) };
            self.enqueue_checked(task);
        } else {
            trace!("Marking task {} as waiting", context.handle());
            unsafe { context.set_schedule_status(TaskScheduleStatus::Waiting) };
        }

        Ok(())
    }
}

#[derive(Debug)]
pub(crate) struct Msg {
    pub task: &'static RawTask,
    pub data: MsgData,
}

#[derive(Debug)]
pub(crate) enum MsgData {
    Completed {
        aborted: bool,
    },
    Yield {
        f: RawFnOnce<(NonNull<IScheduler>, NonNull<IRawTask>), (), SendMarker>,
    },
}

impl MsgData {
    #[inline]
    pub fn msg_type(&self) -> &str {
        match &self {
            MsgData::Completed { .. } => "Completed",
            MsgData::Yield { .. } => "Yield",
        }
    }
}

#[derive(Debug)]
#[repr(transparent)]
pub(crate) struct RawTask(IRawTask);

impl RawTask {
    /// Constructs a `RawTask` from an [`IRawTask`].
    ///
    /// # Safety
    ///
    /// The caller must ensure that the task is registered
    /// with the current runtime.
    #[inline]
    pub unsafe fn from_i_raw(task: &IRawTask) -> &Self {
        &*(task as *const _ as *const Self)
    }

    /// Extracts the contained [`IRawTask`].
    #[inline]
    pub fn as_i_raw(&self) -> &IRawTask {
        &self.0
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

    /// Fetches a pointer to the internal scheduler context.
    #[inline]
    pub fn scheduler_context(&self) -> &SchedulerContext {
        // safety: SchedulerContext has a transparent repr and we know that
        // the task is registered with our runtime.
        unsafe {
            std::mem::transmute::<&ISchedulerContext, &SchedulerContext>(self.0.scheduler_context())
        }
    }

    /// Fetches a mutable pointer to the internal scheduler context.
    ///
    /// # Safety
    ///
    /// Behavior is undefined if not called from a task scheduler.
    #[inline]
    #[allow(clippy::mut_from_ref)]
    pub unsafe fn scheduler_context_mut(&self) -> &mut SchedulerContext {
        // safety: SchedulerContext has a transparent repr and we know that
        // the task is registered with our runtime.
        std::mem::transmute::<&mut ISchedulerContext, &mut SchedulerContext>(
            self.0.scheduler_context_mut(),
        )
    }
}

impl PartialEq for RawTask {
    fn eq(&self, other: &Self) -> bool {
        self.scheduler_context().handle() == other.scheduler_context().handle()
    }
}

impl PartialOrd for RawTask {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Eq for RawTask {}

impl Ord for RawTask {
    fn cmp(&self, other: &Self) -> Ordering {
        self.priority().cmp(&other.priority())
    }
}

#[derive(Debug)]
#[repr(transparent)]
pub(crate) struct SchedulerContext(ISchedulerContext);

impl SchedulerContext {
    /// Extracts the handle to the task.
    #[inline]
    pub fn handle(&self) -> TaskHandle {
        debug_assert!(self.is_registered());
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
    ///
    /// # Safety
    ///
    /// Behavior is undefined if not called from a task scheduler.
    #[inline]
    pub unsafe fn unregister(&mut self) -> (TaskHandle, ObjBox<ContextData>) {
        let (handle, data) = self.0.unregister();
        let data = data.expect("Scheduler data taken from registered task");
        let data = ObjBox::try_object_cast(data).expect("Invalid scheduler data");
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
    /// # Safety
    ///
    /// Behavior is undefined if any of the following conditions are violated:
    ///
    /// * A worker associated with the provided [`WorkerId`] does not exist.
    /// * The task has yielded it's execution and has cached thread-local variables.
    #[inline]
    pub unsafe fn set_worker(&self, worker: Option<WorkerId>) {
        self.0.set_worker(worker)
    }

    /// Clears the requests and returns it.
    #[inline]
    pub fn clear_request(&self) -> StatusRequest {
        self.0.clear_request()
    }

    /// Extracts the current run status.
    #[inline]
    pub fn run_status(&self) -> TaskRunStatus {
        self.0.run_status()
    }

    /// Sets a new run status.
    ///
    /// # Safety
    ///
    /// Behavior is undefined if not called from a task scheduler.
    #[inline]
    pub unsafe fn set_run_status(&self, status: TaskRunStatus) {
        self.0.set_run_status(status)
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

    /// Takes the entry function of the task.
    ///
    /// # Safety
    ///
    /// Behavior is undefined if not called from a task scheduler.
    #[inline]
    pub unsafe fn take_entry_function(&mut self) -> Option<RawFnOnce<(), (), SendMarker>> {
        self.0.take_entry_function()
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
    pub unsafe fn set_panic(&mut self, panic: Option<ObjBox<PanicData>>) {
        let panic = panic.map(|p| {
            let p: ObjBox<IRustPanicData> = ObjBox::coerce_object(p);
            ObjBox::cast_super(p)
        });
        self.0.set_panic(panic)
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
    pub unsafe fn take_panic_data(&mut self) -> Option<ObjBox<PanicData>> {
        self.0
            .take_panic_data()
            .map(|p| ObjBox::try_object_cast(p).expect("Invalid panic data"))
    }

    /// Calls the cleanup function.
    #[inline]
    pub fn cleanup(&mut self) {
        self.0.cleanup()
    }

    /// Fetches a reference to the scheduler data.
    ///
    /// # Safety
    ///
    /// Behavior is undefined if not called from a task scheduler.
    #[inline]
    pub unsafe fn scheduler_data(&self) -> &ContextData {
        self.0
            .scheduler_data()
            .expect("Invalid scheduler data")
            .try_cast_obj()
            .expect("Invalid scheduler data")
    }

    /// Fetches a mutable reference to the scheduler data.
    ///
    /// # Safety
    ///
    /// Behavior is undefined if not called from a task scheduler.
    #[inline]
    pub unsafe fn scheduler_data_mut(&mut self) -> &mut ContextData {
        self.0
            .scheduler_data_mut()
            .expect("Invalid scheduler data")
            .try_cast_obj_mut()
            .expect("Invalid scheduler data")
    }
}

#[derive(Debug)]
pub(crate) struct ContextData {
    pub processing: bool,
    pub slot: Option<TaskSlot>,
    pub context: Option<Context>,
    pub dependencies: BTreeSet<&'static RawTask>,
    pub waiters: BinaryHeap<Reverse<&'static RawTask>>,
}

impl ContextData {
    fn new() -> Self {
        Self {
            processing: false,
            slot: None,
            context: None,
            dependencies: Default::default(),
            waiters: Default::default(),
        }
    }
}

is_object! { #![uuid(0xc68fe659, 0xbeef, 0x4341, 0x9b75, 0xf54b0ef387ff)] ContextData }

impl_vtable! {
    impl mut IBase<SendSyncMarker> => ContextData {}
}

pub(crate) struct PanicData {
    data: Option<Box<dyn Any + Send + 'static>>,
}

impl PanicData {
    pub fn new(e: Box<dyn Any + Send + 'static>) -> ObjBox<Self> {
        ObjBox::new(Self { data: Some(e) })
    }
}

is_object! { #![uuid(0xd2e5a6f3, 0xd5a0, 0x41f0, 0xa6b1, 0x62d543c5c46b)] PanicData }

impl_vtable! {
    impl inline mut IRustPanicDataVTable => PanicData {
        |ptr| unsafe {
            let this = &mut *(ptr as *mut PanicData);
            this.data.take().expect("Invalid panic data")
        }
    }
}
