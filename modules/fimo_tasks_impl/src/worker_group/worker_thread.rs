use crate::{
    module_export::{TasksModule, TasksModuleToken},
    worker_group::{
        command_buffer::CommandBufferHandleImpl, event_loop::InnerRequest, task::EnqueuedTask,
        WorkerGroupImpl,
    },
};
use crossbeam_channel::{Receiver, Sender};
use crossbeam_deque::{Injector, Stealer, Worker};
use fimo_std::{error::Error, module::Module, tracing, tracing::ThreadAccess};
use fimo_tasks_meta::WorkerId;
use std::{
    cell::{RefCell, RefMut},
    fmt::Debug,
    mem::MaybeUninit,
    sync::{
        atomic::{AtomicBool, AtomicUsize, Ordering},
        Arc,
    },
    thread::{JoinHandle, Thread},
    time::Instant,
};

#[thread_local]
static WORKER_THREAD: WorkerContextLock = WorkerContextLock::new();

#[derive(Debug)]
pub struct WorkerBootstrapper {
    id: WorkerId,
    latch: Sender<Arc<WorkerSyncInfo>>,
    stealer: Stealer<WorkerResponse>,
    join_handle: JoinHandle<()>,
    bound_tasks_sender: Sender<WorkerResponse>,
}

impl WorkerBootstrapper {
    pub fn new(
        id: WorkerId,
        group: Arc<WorkerGroupImpl>,
        event_loop_sender: Sender<InnerRequest>,
    ) -> Self {
        let worker = Worker::new_fifo();
        let stealer = worker.stealer();
        let (sx, rx) = crossbeam_channel::unbounded();
        let (latch_sx, latch_rx) = crossbeam_channel::bounded(1);

        let name = format!("{:?} Worker: {id:?}", group.name);
        let join_handle = std::thread::Builder::new()
            .name(name)
            .spawn({
                let sx = sx.clone();
                move || {
                    // Wait for the sync object.
                    let sync = latch_rx.recv().expect("no signal received");

                    let worker = WorkerThread {
                        id,
                        sync,
                        group,
                        event_loop_sender,
                        bound_tasks_sender: sx,
                        bound_tasks: rx,
                        local_queue: worker,
                    };
                    worker_event_loop(worker);
                }
            })
            .expect("could not create worker thread");

        Self {
            id,
            latch: latch_sx,
            stealer,
            join_handle,
            bound_tasks_sender: sx,
        }
    }

    pub fn bootstrap_data(&self) -> (Thread, Stealer<WorkerResponse>) {
        let thread = self.join_handle.thread().clone();
        let stealer = self.stealer.clone();
        (thread, stealer)
    }

    pub fn start(self, sync: Arc<WorkerSyncInfo>) -> (WorkerId, WorkerHandle) {
        self.latch.send(sync.clone()).expect("can not send signal");

        (
            self.id,
            WorkerHandle {
                sync,
                bound_tasks_sender: self.bound_tasks_sender,
                join_handle: Some(self.join_handle),
            },
        )
    }
}

#[derive(Debug)]
pub struct WorkerHandle {
    sync: Arc<WorkerSyncInfo>,
    bound_tasks_sender: Sender<WorkerResponse>,
    join_handle: Option<JoinHandle<()>>,
}

impl WorkerHandle {
    pub fn push_local_response(&self, worker_response: WorkerResponse) {
        self.bound_tasks_sender
            .send(worker_response)
            .expect("local queue closed");

        // Wake the worker thread.
        if let Some(handle) = &self.join_handle {
            handle.thread().unpark();
        }
    }

    pub fn join(&mut self) {
        // Notify all workers to stop executing tasks.
        self.sync.request_join();
        let handle = self.join_handle.take().expect("handle already joined");

        // Wake the worker so that we don't deadlock.
        handle.thread().unpark();

        handle.join().expect("worker did not complete successfully");
    }
}

impl Drop for WorkerHandle {
    fn drop(&mut self) {
        if self.join_handle.is_some() {
            self.join();
        }
    }
}

#[derive(Debug)]
pub struct WorkerContext {
    pub id: WorkerId,
    pub group: Arc<WorkerGroupImpl>,
    pub current_task: Option<EnqueuedTask>,
    pub resume_context: Option<context::Context>,
}

#[derive(Debug)]
struct WorkerThread {
    id: WorkerId,
    sync: Arc<WorkerSyncInfo>,
    group: Arc<WorkerGroupImpl>,
    event_loop_sender: Sender<InnerRequest>,
    bound_tasks_sender: Sender<WorkerResponse>,
    bound_tasks: Receiver<WorkerResponse>,
    local_queue: Worker<WorkerResponse>,
}

#[derive(Debug)]
pub struct WorkerRequest {
    pub task: EnqueuedTask,
    pub request: TaskRequest,
}

#[derive(Debug)]
pub struct WorkerResponse {
    pub task: EnqueuedTask,
    pub response: TaskResponse,
}

#[derive(Debug, Clone, Copy, Ord, PartialOrd, Eq, PartialEq, Hash)]
pub struct AssertSend<T>(pub T);

// Safety: The user must ensure that it is sound.
unsafe impl<T> Send for AssertSend<T> {}

#[derive(Debug)]
pub enum TaskRequest {
    Complete,
    Abort(AssertSend<*mut std::ffi::c_void>),
    Yield,
    WaitUntil(Instant),
    WaitOnCommandBuffer(Arc<CommandBufferHandleImpl>),
}

#[derive(Debug)]
pub enum TaskResponse {
    Start,
    #[allow(dead_code)]
    Complete(std::convert::Infallible),
    #[allow(dead_code)]
    Abort(std::convert::Infallible),
    Yield,
    WaitUntil,
    WaitOnCommandBuffer(bool),
}

#[derive(Debug)]
pub struct WorkerSyncInfo {
    join_requested: AtomicBool,
    enqueued_command_buffers: AtomicUsize,
    global_queue: Injector<WorkerResponse>,
    queue_stealers: Box<[Stealer<WorkerResponse>]>,
    worker_threads: Box<[Thread]>,
}

impl WorkerSyncInfo {
    pub fn new(
        queue_stealers: Box<[Stealer<WorkerResponse>]>,
        worker_threads: Box<[Thread]>,
    ) -> Self {
        Self {
            join_requested: AtomicBool::new(false),
            enqueued_command_buffers: AtomicUsize::new(0),
            global_queue: Injector::new(),
            queue_stealers,
            worker_threads,
        }
    }

    pub fn push_global_response(&self, worker_response: WorkerResponse) {
        self.global_queue.push(worker_response);

        // Wake all worker threads.
        for thread in &self.worker_threads {
            thread.unpark();
        }
    }

    pub fn request_join(&self) {
        self.join_requested.store(true, Ordering::Release);
    }

    pub fn notify_command_buffer_enqueued(&self) {
        self.enqueued_command_buffers
            .fetch_add(1, Ordering::Release);
    }

    pub fn notify_command_buffer_completed(&self) {
        self.enqueued_command_buffers
            .fetch_sub(1, Ordering::Release);
    }

    fn can_join(&self) -> bool {
        self.join_requested.load(Ordering::Acquire)
            && self.enqueued_command_buffers.load(Ordering::Acquire) == 0
    }

    fn dequeue_task(&self, local: &Worker<WorkerResponse>) -> Option<WorkerResponse> {
        // Pop a task from the local queue, if not empty.
        let task = local.pop().or_else(|| {
            // Otherwise, we need to look for a task elsewhere.
            std::iter::repeat_with(|| {
                // Try stealing a batch of tasks from the global queue.
                self.global_queue
                    .steal_batch_and_pop(local)
                    // Or try stealing a task from one of the other threads.
                    .or_else(|| self.queue_stealers.iter().map(|s| s.steal()).collect())
            })
            // Loop while no task was stolen and any steal operation needs to be retried.
            .find(|s| !s.is_retry())
            // Extract the stolen task, if there is one.
            .and_then(|s| s.success())
        });

        // Park the thread if there were no tasks.
        if let Some(x) = task {
            Some(x)
        } else {
            if !self.can_join() {
                std::thread::park();
            }
            None
        }
    }
}

impl Drop for WorkerSyncInfo {
    fn drop(&mut self) {
        self.request_join();
    }
}

#[derive(Debug)]
pub struct WorkerContextLock(RefCell<Option<WorkerContext>>);

impl WorkerContextLock {
    const fn new() -> Self {
        Self(RefCell::new(None))
    }

    /// # Safety
    ///
    /// May only be called by the event loop, which must also uninitialize it.
    unsafe fn init(&self, worker: WorkerContext) {
        let mut guard = self.0.borrow_mut();
        if guard.is_some() {
            panic!("tried to initialize a `WorkerContextLock` twice");
        }
        *guard = Some(worker);
    }

    /// # Safety
    ///
    /// May only be called by the event loop.
    unsafe fn uninit(&self) -> WorkerContext {
        let mut guard = self.0.borrow_mut();
        match guard.take() {
            None => panic!("tried to uninitialize an uninitialized `WorkerContextLock`"),
            Some(worker) => worker,
        }
    }
}

pub fn with_worker_context_lock<R>(f: impl FnOnce(&mut WorkerContext) -> R) -> Result<R, Error> {
    let guard = WORKER_THREAD.0.try_borrow_mut().map_err(Error::new)?;
    let mut worker =
        RefMut::filter_map(guard, |worker| worker.as_mut()).map_err(|_e| <Error>::EPERM)?;
    Ok(f(&mut worker))
}

/// # Safety
///
/// Should not be used directly.
unsafe fn send_worker_request(request: TaskRequest) -> Result<TaskResponse, Error> {
    // Take the context of the event loop.
    let context = with_worker_context_lock(|worker| worker.resume_context.take().unwrap())?;

    // Switch to the event loop.
    let request = MaybeUninit::new(request);
    // Safety: We ensure that everything is set up properly.
    let context::Transfer { context, data } =
        unsafe { context.resume(request.as_ptr().expose_provenance()) };

    // Restore the context of the event loop.
    with_worker_context_lock(|worker| worker.resume_context = Some(context))?;

    // Safety: We are passed ownership to a `TaskResponse` instance.
    let response = unsafe { std::ptr::with_exposed_provenance::<TaskResponse>(data).read() };
    Ok(response)
}

/// # Safety
///
/// May only be called upon completion of a task.
pub unsafe fn complete_task() -> Result<std::convert::Infallible, Error> {
    // Safety: Ensured by the caller.
    let response = unsafe { send_worker_request(TaskRequest::Complete)? };
    match response {
        #[allow(unreachable_patterns)]
        TaskResponse::Complete(x) => Ok(x),
        _ => unreachable!("should not happen"),
    }
}

/// # Safety
///
/// May only be called upon abortion of a task.
pub unsafe fn abort_task(error: *mut std::ffi::c_void) -> Result<std::convert::Infallible, Error> {
    // Safety: Ensured by the caller.
    let response = unsafe { send_worker_request(TaskRequest::Abort(AssertSend(error)))? };
    match response {
        #[allow(unreachable_patterns)]
        TaskResponse::Abort(x) => Ok(x),
        _ => unreachable!("should not happen"),
    }
}

pub fn yield_now() -> Result<(), Error> {
    // Safety: Is always safe.
    let response = unsafe { send_worker_request(TaskRequest::Yield)? };
    match response {
        TaskResponse::Yield => Ok(()),
        _ => unreachable!("should not happen"),
    }
}

pub fn wait_until(instant: Instant) -> Result<(), Error> {
    // Safety: Is always safe.
    let response = unsafe { send_worker_request(TaskRequest::WaitUntil(instant))? };
    match response {
        TaskResponse::WaitUntil => Ok(()),
        _ => unreachable!("should not happen"),
    }
}

pub fn wait_on_command_buffer(
    handle: Arc<CommandBufferHandleImpl>,
) -> Result<bool, (Error, Arc<CommandBufferHandleImpl>)> {
    // Check that the handle belongs to the same worker group.
    match with_worker_context_lock(|worker| {
        let group = handle.worker_group_weak();
        if !std::ptr::eq(Arc::as_ptr(&worker.group), group.as_ptr()) {
            Err(Error::EPERM)
        } else {
            Ok(())
        }
    })
    .flatten()
    {
        Ok(_) => {}
        Err(e) => return Err((e, handle)),
    }

    // Safety: Is always safe.
    let response = unsafe { send_worker_request(TaskRequest::WaitOnCommandBuffer(handle.clone())) };
    match response {
        Ok(TaskResponse::WaitOnCommandBuffer(aborted)) => Ok(aborted),
        Err(e) => Err((e, handle)),
        _ => unreachable!("should not happen"),
    }
}

fn worker_event_loop(data: WorkerThread) {
    // Safety: While the event loop is running, the task can not be unloaded.
    unsafe {
        let WorkerThread {
            id,
            sync,
            group,
            event_loop_sender,
            bound_tasks_sender,
            bound_tasks,
            local_queue,
        } = data;

        TasksModuleToken::with_current(move |module| {
            let module = **module;

            // Initialize the tracing for the worker thread.
            use fimo_std::module::Module;
            let _tracing = ThreadAccess::new(&module.context());
            let _span =
                fimo_std::span_trace!(module.context(), "worker event loop, worker: {id:?}");

            // Initialize the shared worker data.
            let shared = WorkerContext {
                id,
                group: group.clone(),
                current_task: None,
                resume_context: None,
            };
            // Safety: We are the event loop and are going to uninitialize it.
            WORKER_THREAD.init(shared);

            // Loop until we must join.
            while !sync.can_join() {
                // First handle the bound tasks.
                let WorkerResponse { mut task, response } = match bound_tasks.try_recv() {
                    Ok(task) => task,
                    Err(_) => {
                        // If we don't own any tasks we try to dequeue one.
                        match sync.dequeue_task(&local_queue) {
                            None => continue,
                            Some(task) => task,
                        }
                    }
                };

                // Retrieve the context of the task.
                let context = task.take_resume_context();

                // Switch the call stack.
                tracing::CallStack::suspend_current(&module.context(), false)
                    .expect("could not suspend current event loop call stack");
                let call_stack = task.take_call_stack();
                let call_stack = call_stack
                    .switch()
                    .expect("could not switch to task call stack");
                tracing::CallStack::resume_current(&module.context())
                    .expect("could not resume task call stack");

                // Set the task as active.
                with_worker_context_lock(|worker| worker.current_task = Some(task)).unwrap();

                // Jump into the task.
                let response = MaybeUninit::new(response);
                // Safety: We ensure that everything is set up properly.
                let context::Transfer { context, data } =
                    context.resume(response.as_ptr().expose_provenance());

                // Safety: We are passed ownership to a `TaskRequest` instance.
                let request = std::ptr::with_exposed_provenance::<TaskRequest>(data).read();

                // Set the task as inactive.
                let mut task =
                    with_worker_context_lock(|worker| worker.current_task.take().unwrap()).unwrap();
                task.set_resume_context(context);

                // Process the request.
                match request {
                    TaskRequest::Complete => {
                        // Switch back to the event loop call stack.
                        swap_call_stack(module, &mut task, call_stack, false);

                        // Lock the context so that the callbacks can not call into the context.
                        with_worker_context_lock(|_| {
                            // Safety: The task has been completed and the context is locked.
                            task.run_cleanup();
                        })
                        .unwrap();

                        // Notify the main event loop.
                        event_loop_sender
                            .send(InnerRequest::WorkerRequest(WorkerRequest { task, request }))
                            .expect("event loop queue should be open");
                    }
                    TaskRequest::Abort(AssertSend(error)) => {
                        // Switch back to the event loop call stack.
                        swap_call_stack(module, &mut task, call_stack, false);

                        // Lock the context so that the callbacks can not call into the context.
                        with_worker_context_lock(|_| {
                            // Safety: The task has been aborted and the context is locked.
                            task.run_abort(error);
                        })
                        .unwrap();

                        // Notify the main event loop.
                        event_loop_sender
                            .send(InnerRequest::WorkerRequest(WorkerRequest { task, request }))
                            .expect("event loop queue should be open");
                    }
                    TaskRequest::Yield => {
                        // Switch back to the event loop call stack.
                        swap_call_stack(module, &mut task, call_stack, false);

                        // Push the task onto our task queue.
                        bound_tasks_sender
                            .send(WorkerResponse {
                                task,
                                response: TaskResponse::Yield,
                            })
                            .unwrap();
                    }
                    TaskRequest::WaitUntil(timeout) => {
                        // If the timeout has passed we can enqueue the task.
                        if Instant::now() >= timeout {
                            // Switch back to the event loop call stack.
                            swap_call_stack(module, &mut task, call_stack, false);

                            // Push the task onto our task queue.
                            bound_tasks_sender
                                .send(WorkerResponse {
                                    task,
                                    response: TaskResponse::WaitUntil,
                                })
                                .unwrap();
                            continue;
                        }

                        // Switch back to the event loop call stack.
                        swap_call_stack(module, &mut task, call_stack, true);

                        // Otherwise we notify the event loop.
                        event_loop_sender
                            .send(InnerRequest::WorkerRequest(WorkerRequest {
                                task,
                                request: TaskRequest::WaitUntil(timeout),
                            }))
                            .expect("event loop queue should be open");
                    }
                    TaskRequest::WaitOnCommandBuffer(handle) => {
                        // If the command buffer is already completed we can enqueue the task.
                        if let Some(aborted) = handle.completion_status() {
                            // Switch back to the event loop call stack.
                            swap_call_stack(module, &mut task, call_stack, false);

                            // Push the task onto our task queue.
                            bound_tasks_sender
                                .send(WorkerResponse {
                                    task,
                                    response: TaskResponse::WaitOnCommandBuffer(aborted),
                                })
                                .unwrap();
                            continue;
                        }

                        // Switch back to the event loop call stack.
                        swap_call_stack(module, &mut task, call_stack, true);

                        // Otherwise we notify the event loop.
                        event_loop_sender
                            .send(InnerRequest::WorkerRequest(WorkerRequest {
                                task,
                                request: TaskRequest::WaitOnCommandBuffer(handle),
                            }))
                            .expect("event loop queue should be open");
                    }
                }
            }

            // Drop the shared worker data.
            // Safety: We are the event loop.
            drop(WORKER_THREAD.uninit());
        });
    }
}

fn swap_call_stack(
    module: TasksModule<'_>,
    task: &mut EnqueuedTask,
    call_stack: tracing::CallStack,
    block: bool,
) {
    // Switch back to the event loop call stack.
    tracing::CallStack::suspend_current(&module.context(), block)
        .expect("could not suspend task call stack");
    task.set_call_stack(call_stack.switch().expect("could not switch to event loop"));
    tracing::CallStack::resume_current(&module.context())
        .expect("could not resume event loop call stack");
}
