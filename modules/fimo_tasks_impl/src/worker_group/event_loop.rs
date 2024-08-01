use crate::{
    module_export::{TasksModule, TasksModuleToken},
    worker_group::{
        command_buffer::{
            CommandBufferEventLoopCommand, CommandBufferHandleImpl, CommandBufferId,
            CommandBufferImpl, Waiter,
        },
        event_loop::stack_manager::StackDescriptor,
        task::EnqueuedTask,
        worker_thread::{
            TaskRequest, TaskResponse, WorkerBootstrapper, WorkerHandle, WorkerRequest,
            WorkerResponse, WorkerSyncInfo,
        },
        WorkerGroupImpl,
    },
};
use crossbeam_channel::{select, Receiver, Sender, TrySendError};
use fimo_std::{error::Error, module::Module};
use fimo_tasks::{TaskId, WorkerId};
use rustc_hash::FxHashMap;
use std::{
    fmt::{Debug, Formatter},
    num::NonZeroUsize,
    sync::{Arc, Mutex, RwLock},
    thread::JoinHandle,
    time::{Duration, Instant},
};

pub mod stack_manager;
pub mod time_out;

#[derive(Debug)]
pub enum OuterRequest {
    Close,
}

#[derive(Debug)]
pub enum InnerRequest {
    UnblockTask(TaskId),
    UnblockCommandBuffer(Arc<CommandBufferHandleImpl>),
    WorkerRequest(WorkerRequest),
}

pub struct EventLoopHandle {
    connection_status: RwLock<ConnectionStatus>,
    outer_requests: Sender<OuterRequest>,
    _inner_requests: Sender<InnerRequest>,
    handle: Mutex<Option<JoinHandle<()>>>,
}

#[derive(Debug, Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Hash)]
enum ConnectionStatus {
    Open,
    Closed,
}

impl EventLoopHandle {
    pub fn new(
        ctx: fimo_std::context::ContextView<'_>,
        group: Arc<WorkerGroupImpl>,
        num_workers: usize,
        default_stack_size: usize,
        stacks: Vec<StackDescriptor>,
    ) -> Self {
        let _span = fimo_std::span_trace!(
            ctx,
            "group: {group:?}, num_workers: {num_workers:?}, \
            default_stack_size: {default_stack_size:?}, stacks: {stacks:?}"
        );
        fimo_std::emit_trace!(ctx, "spawning event loop");

        let connection_status = RwLock::new(ConnectionStatus::Open);
        let (outer_sx, outer_rx) = crossbeam_channel::unbounded();
        let (inner_sx, inner_rx) = crossbeam_channel::unbounded();

        // Synchronize the initialization of the event loop.
        let (error_sx, error_rx) = crossbeam_channel::bounded::<std::thread::Result<()>>(1);
        let handle = std::thread::spawn({
            let group = group.clone();
            let inner_sx = inner_sx.clone();
            move || {
                // Safety: The module can not be unloaded until the event loop finished.
                unsafe {
                    TasksModuleToken::with_current_unlocked(|module| {
                        // Initialize the tracing for the event loop thread.
                        use fimo_std::module::Module;
                        let _tracing = fimo_std::tracing::ThreadAccess::new(&module.context());
                        let _span = fimo_std::span_trace!(
                            module.context(),
                            "event loop, name: {:?}, id: {:?}",
                            group.name,
                            group.id,
                        );

                        let event_loop =
                            match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                                EventLoop::new(
                                    &module,
                                    group,
                                    num_workers,
                                    default_stack_size,
                                    stacks,
                                    outer_rx,
                                    inner_sx,
                                    inner_rx,
                                )
                            })) {
                                Ok(event_loop) => {
                                    fimo_std::emit_trace!(
                                        module.context(),
                                        "event_loop: {event_loop:?}"
                                    );
                                    error_sx.send(Ok(())).expect("could not send status");
                                    event_loop
                                }
                                Err(e) => {
                                    fimo_std::emit_error!(
                                        module.context(),
                                        "event loop creation failed"
                                    );
                                    error_sx.send(Err(e)).expect("could not send status");
                                    return;
                                }
                            };
                        event_loop.enter_event_loop(&module);
                    });
                }
            }
        });

        // Panic if we could not create the event loop.
        if let Err(e) = error_rx.recv().expect("could not receive status") {
            fimo_std::emit_error!(ctx, "could not spawn event loop");
            std::panic::resume_unwind(e);
        }

        Self {
            connection_status,
            outer_requests: outer_sx,
            _inner_requests: inner_sx,
            handle: Mutex::new(Some(handle)),
        }
    }

    pub fn is_open(&self) -> bool {
        self.connection_status
            .read()
            .map(|s| *s == ConnectionStatus::Open)
            .unwrap_or(false)
    }

    pub fn request_close(&self) -> Result<(), Error> {
        // Acquire the `RwLock` with `write` permissions, such that no messages are sent while we
        // try to send the close message.
        let mut status = self
            .connection_status
            .write()
            .map_err(|_e| Error::ECANCELED)?;

        // If the channel is already closed we can return.
        if *status == ConnectionStatus::Closed {
            return Ok(());
        }

        // Send the message.
        self.outer_requests
            .try_send(OuterRequest::Close)
            .map_err(|e| match e {
                TrySendError::Full(_) => Error::ECOMM,
                TrySendError::Disconnected(_) => Error::ECONNABORTED,
            })?;

        // Change the status, so that other threads don't keep sending new messages while the
        // channel is still open.
        *status = ConnectionStatus::Closed;

        Ok(())
    }

    pub fn enqueue_command_buffer(&self) -> Result<Arc<CommandBufferHandleImpl>, Error> {
        // Acquire the lock, such that it can not be closed in the meantime.
        let status = self
            .connection_status
            .read()
            .map_err(|_e| Error::ECANCELED)?;

        // If the channel is already closed we can return.
        if *status == ConnectionStatus::Closed {
            return Err(Error::ECANCELED);
        }

        todo!()
    }

    pub fn wait_for_close(&self) {
        let handle = {
            let mut guard = self.handle.lock().expect("could not lock thread handle");
            guard.take()
        };

        if let Some(handle) = handle {
            let _ = handle.join();
        }
    }
}

impl Debug for EventLoopHandle {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EventLoopHandle")
            .field("connection_status", &self.connection_status)
            .finish_non_exhaustive()
    }
}

impl Drop for EventLoopHandle {
    fn drop(&mut self) {
        if let Some(handle) = self.handle.get_mut().expect("could not get handle").take() {
            let _ = self.request_close();
            let _ = handle.join();
        }
    }
}

#[derive(Debug)]
struct EventLoop {
    is_closed: bool,
    next_timeout: Instant,
    group: Arc<WorkerGroupImpl>,
    stack_manager: stack_manager::StackManager,
    public_messages: Receiver<OuterRequest>,
    private_messages: Receiver<InnerRequest>,
    private_messages_sender: Sender<InnerRequest>,
    worker_shared: Arc<WorkerSyncInfo>,
    workers: FxHashMap<WorkerId, WorkerHandle>,
    blocked_tasks: FxHashMap<TaskId, BlockedTask>,
    handles: FxHashMap<CommandBufferId, CommandBufferImpl>,
    timeouts: Vec<time_out::TimeOut>,
}

#[derive(Debug)]
enum BlockedTask {
    WaitTimeout {
        task: EnqueuedTask,
    },
    WaitCommandBuffer {
        task: EnqueuedTask,
        buffer: Arc<CommandBufferHandleImpl>,
    },
    External {
        task: EnqueuedTask,
    },
}

// Outer requests.
impl EventLoop {
    fn on_close(&mut self, module: &TasksModule<'_>) {
        fimo_std::emit_trace!(module.context(), "closing queue");
        self.is_closed = true;
    }
}

// Inner requests.
impl EventLoop {
    fn on_worker_request(&mut self, module: &TasksModule<'_>, request: WorkerRequest) {
        fimo_std::emit_trace!(module.context(), "worker_request: {request:?}");

        let WorkerRequest { task, request } = request;
        match request {
            TaskRequest::Complete => {
                self.finish_task(module, task, false);
            }
            TaskRequest::Abort(_) => {
                self.finish_task(module, task, true);
            }
            TaskRequest::Yield => {
                unreachable!("yields should not return to the event loop")
            }
            TaskRequest::WaitUntil(time) => {
                // Insert the timeout into our timeout queue.
                let timeout =
                    time_out::TimeOut::new(time, time_out::TimeOutHandle::Internal(task.id()));
                self.add_timeout(module, timeout);
                self.blocked_tasks
                    .insert(task.id(), BlockedTask::WaitTimeout { task });
            }
            TaskRequest::WaitOnCommandBuffer(handle) => {
                match handle.completion_status() {
                    None => {
                        // Add the task to the block list.
                        let state = self
                            .handles
                            .get_mut(&handle.id())
                            .expect("command buffer does not exist");
                        state.register_waiter(Waiter::Task(task.id()));
                        self.blocked_tasks.insert(
                            task.id(),
                            BlockedTask::WaitCommandBuffer {
                                task,
                                buffer: handle,
                            },
                        );
                    }
                    Some(aborted) => {
                        // Unblock the call stack.
                        let mut task = task;
                        let call_stack = task.peek_call_stack();
                        call_stack
                            .unblock()
                            .expect("could not unblock task call stack");

                        // Enqueue the task.
                        let worker_id = task.worker();
                        let worker = &self.workers[&worker_id];
                        worker.push_local_response(WorkerResponse {
                            task,
                            response: TaskResponse::WaitOnCommandBuffer(aborted),
                        });
                    }
                }
            }
        }
    }

    fn on_unblock_task(&mut self, module: &TasksModule<'_>, task: TaskId, time_out: bool) {
        fimo_std::emit_trace!(module.context(), "unblocking task: {task:?}",);

        let task = self.blocked_tasks.remove(&task).expect("task not found");
        match task {
            BlockedTask::WaitTimeout { mut task } => {
                if !time_out {
                    panic!("tried to manually wake sleeping task, task: {task:?}");
                }

                // Unblock the call stack.
                let call_stack = task.peek_call_stack();
                call_stack
                    .unblock()
                    .expect("could not unblock task call stack");

                let worker_id = task.worker();
                let worker = &self.workers[&worker_id];
                worker.push_local_response(WorkerResponse {
                    task,
                    response: TaskResponse::WaitUntil,
                });
            }
            BlockedTask::WaitCommandBuffer { mut task, buffer } => {
                if time_out {
                    panic!("time outs are not supported while waiting on command buffer, task: {task:?}, command buffer: {buffer:?}");
                }

                // Unblock the call stack.
                let call_stack = task.peek_call_stack();
                call_stack
                    .unblock()
                    .expect("could not unblock task call stack");

                let aborted = buffer
                    .completion_status()
                    .expect("command buffer is not completed");
                let worker_id = task.worker();
                let worker = &self.workers[&worker_id];
                worker.push_local_response(WorkerResponse {
                    task,
                    response: TaskResponse::WaitOnCommandBuffer(aborted),
                });
            }
            #[allow(clippy::unimplemented)]
            BlockedTask::External { .. } => unimplemented!(),
        }
    }

    fn on_unblock_command_buffer(
        &mut self,
        module: &TasksModule<'_>,
        command_buffer: Arc<CommandBufferHandleImpl>,
    ) {
        fimo_std::emit_trace!(
            module.context(),
            "unblocking command buffer: {command_buffer:?}",
        );

        // Check if it was aborted.
        if command_buffer.is_completed() {
            return;
        }

        // Continue processing the commands.
        self.process_command_buffer_commands(module, command_buffer.id());
    }
}

// Task management.
impl EventLoop {
    fn enqueue_task(
        &mut self,
        module: &TasksModule<'_>,
        task: EnqueuedTask,
        worker: Option<WorkerId>,
    ) {
        fimo_std::emit_trace!(
            module.context(),
            "enqueueing task: {task:?}, worker: {worker:?}"
        );

        if let Some(worker) = worker {
            let worker = self.workers.get(&worker).expect("worker not found");
            worker.push_local_response(WorkerResponse {
                task,
                response: TaskResponse::Start,
            });
        } else {
            self.worker_shared.push_global_response(WorkerResponse {
                task,
                response: TaskResponse::Start,
            });
        }
    }

    fn finish_task(&mut self, module: &TasksModule<'_>, task: EnqueuedTask, aborted: bool) {
        fimo_std::emit_trace!(
            module.context(),
            "finishing task: {task:?}, aborted: {aborted:?}"
        );

        let (_, buffer_id, index, task, stack) = task.into_raw_parts();

        // Release the stack.
        let allocator = self
            .stack_manager
            .allocator_by_id_mut(stack.id())
            .expect("stack allocator not found");
        allocator.release_stack(stack);
        if let Some((buffer_handle, task_id, stack)) = allocator.pop_waiter() {
            let command_buffer = self
                .handles
                .get_mut(&buffer_handle.id())
                .expect("command buffer not found");
            let (index, worker, task) = command_buffer.mark_task_as_unblocked(task_id);
            let task = EnqueuedTask::new(module, task_id, buffer_id, index, task, stack);
            self.enqueue_task(module, task, worker);
        }

        // Mark the command buffer as completed.
        let command_buffer = self
            .handles
            .get_mut(&buffer_id)
            .expect("command buffer not found");

        if aborted {
            command_buffer.mark_task_as_aborted(module, index, task);
        } else {
            command_buffer.mark_task_as_completed(module, index, task);
        }
        self.process_command_buffer_commands(module, buffer_id);
    }

    fn process_command_buffer_commands(
        &mut self,
        module: &TasksModule<'_>,
        command_buffer_id: CommandBufferId,
    ) {
        fimo_std::emit_trace!(
            module.context(),
            "processing command buffer: {command_buffer_id:?}"
        );

        let command_buffer = self
            .handles
            .get_mut(&command_buffer_id)
            .expect("command buffer not found");

        let check_command_buffer = |handle: &CommandBufferHandleImpl| {
            if handle.id() == command_buffer_id {
                fimo_std::emit_error!(module.context(), "a command buffer can not wait on itself");
                return false;
            }

            let group = handle.worker_group_weak().as_ptr();
            if !std::ptr::eq(group, Arc::as_ptr(&self.group)) {
                fimo_std::emit_error!(
                    module.context(),
                    "command buffer handle does not belong to the same worker group"
                );
                return false;
            }

            true
        };
        let check_worker = |worker| {
            if !self.workers.contains_key(&worker) {
                fimo_std::emit_error!(
                    module.context(),
                    "specified worker {worker:?} does not exist"
                );
                return false;
            }

            true
        };
        let check_stack_size = |stack_size: Option<NonZeroUsize>| {
            let stack_size =
                stack_size.map_or(self.stack_manager.default_stack_size(), |x| x.get());
            if !self.stack_manager.has_allocator(stack_size) {
                fimo_std::emit_error!(
                    module.context(),
                    "required stack size of {stack_size} can not be satisfied"
                );
                return false;
            }

            true
        };

        match command_buffer.process_commands(
            module,
            check_command_buffer,
            check_worker,
            check_stack_size,
        ) {
            CommandBufferEventLoopCommand::Waiting | CommandBufferEventLoopCommand::Processed => {}
            CommandBufferEventLoopCommand::Completed => {
                fimo_std::emit_trace!(
                    module.context(),
                    "command buffer completed: {command_buffer:?}"
                );

                // Wake all waiters of the command buffer and clean up.
                let waiters = command_buffer.take_waiters();
                for waiter in waiters {
                    match waiter {
                        Waiter::Task(task) => {
                            self.on_unblock_task(module, task, false);
                        }
                        // Avoid recursive call by sending a message back to ourselves.
                        Waiter::CommandBuffer(handle) => {
                            if !handle.is_completed() {
                                self.private_messages_sender
                                    .send(InnerRequest::UnblockCommandBuffer(handle))
                                    .unwrap();
                            }
                        }
                    }
                }

                self.handles.remove(&command_buffer_id);
                self.worker_shared.notify_command_buffer_completed();
            }
            CommandBufferEventLoopCommand::SpawnTask(index, task) => {
                fimo_std::emit_trace!(
                    module.context(),
                    "spawning task, command buffer: {command_buffer:?}, task: {task:?}"
                );

                // Try to allocate a stack that is large enough to execute the task.
                let stack_size = command_buffer.stack_size();
                let stack_size =
                    stack_size.map_or(self.stack_manager.default_stack_size(), |x| x.get());
                let allocator = self
                    .stack_manager
                    .allocator_by_size_mut(stack_size)
                    .expect("allocator not found");

                let stack = match allocator.acquire_stack() {
                    Ok(stack) => stack,
                    Err(Error::EBUSY) => {
                        // If we aren't successful due to reaching the maximum number of
                        // allowed stacks we block the task.
                        fimo_std::emit_info!(
                            module.context(),
                            "maximum number of allowed stacks reached for size {stack_size}"
                        );
                        allocator.register_waiter(command_buffer.handle().clone(), task.id());
                        return;
                    }
                    Err(e) => {
                        fimo_std::emit_info!(
                            module.context(),
                            "could not allocate stack, error: {e}"
                        );
                        panic!("unknown error")
                    }
                };

                let task_id = task.id();
                let buffer_id = command_buffer.handle().id();
                let worker = command_buffer.worker();
                let task = EnqueuedTask::new(module, task_id, buffer_id, index, task, stack);
                self.enqueue_task(module, task, worker);
            }
            CommandBufferEventLoopCommand::WaitCommandBuffer(buffer_id) => {
                fimo_std::emit_trace!(
                    module.context(),
                    "waiting on command buffer, command buffer: {command_buffer:?}, wait on: {buffer_id:?}"
                );

                let buffer = self
                    .handles
                    .get_mut(&buffer_id)
                    .expect("command buffer not found");
                debug_assert!(
                    !buffer.handle().is_completed(),
                    "buffer has already finished executing"
                );

                buffer.register_waiter(Waiter::CommandBuffer(buffer.handle().clone()));
            }
        }
    }
}

// Event loop implementation.
impl EventLoop {
    #[allow(clippy::too_many_arguments)]
    fn new(
        _module: &TasksModule<'_>,
        group: Arc<WorkerGroupImpl>,
        num_workers: usize,
        default_stack_size: usize,
        stacks: Vec<StackDescriptor>,
        outer_receiver: Receiver<OuterRequest>,
        inner_sender: Sender<InnerRequest>,
        inner_receiver: Receiver<InnerRequest>,
    ) -> Self {
        let is_closed = false;
        let next_timeout = Instant::now();
        let stack_manager = stack_manager::StackManager::new(default_stack_size, stacks);
        let public_messages = outer_receiver;
        let private_messages = inner_receiver;
        let private_messages_sender = inner_sender;
        let blocked_tasks = FxHashMap::default();
        let handles = FxHashMap::default();
        let timeouts = Vec::default();

        // Bootstrap for the worker threads.
        let worker_bootstrappers = (0..num_workers)
            .map(|id| {
                let id = WorkerId(id);
                WorkerBootstrapper::new(id, group.clone(), private_messages_sender.clone())
            })
            .collect::<Vec<_>>();
        let (worker_threads, queue_stealers): (Vec<_>, Vec<_>) = worker_bootstrappers
            .iter()
            .map(|w| w.bootstrap_data())
            .unzip();
        let queue_stealers = queue_stealers.into_boxed_slice();
        let worker_threads = worker_threads.into_boxed_slice();
        let worker_shared = Arc::new(WorkerSyncInfo::new(queue_stealers, worker_threads));

        // Start the worker threads.
        let workers = worker_bootstrappers
            .into_iter()
            .map(|w| w.start(worker_shared.clone()))
            .collect();

        Self {
            is_closed,
            next_timeout,
            group,
            stack_manager,
            public_messages,
            private_messages,
            private_messages_sender,
            worker_shared,
            workers,
            blocked_tasks,
            handles,
            timeouts,
        }
    }

    fn can_join(&self) -> bool {
        self.is_closed && self.handles.is_empty()
    }

    fn handle_outer_request(&mut self, module: &TasksModule<'_>, msg: OuterRequest) {
        match msg {
            OuterRequest::Close => self.on_close(module),
        }
    }

    fn handle_inner_request(&mut self, module: &TasksModule<'_>, msg: InnerRequest) {
        match msg {
            InnerRequest::WorkerRequest(request) => self.on_worker_request(module, request),
            InnerRequest::UnblockCommandBuffer(command_buffer) => {
                self.on_unblock_command_buffer(module, command_buffer);
            }
            InnerRequest::UnblockTask(task) => self.on_unblock_task(module, task, false),
        }
    }

    fn handle_timeouts(&mut self, module: &TasksModule<'_>) {
        let now = Instant::now();

        // Loop over all timeout instances.
        // The array is sorted such that `t[i].time >= t[i+1].time`.
        loop {
            // If there are no more instances we are done.
            let timeout = self.timeouts.last();
            if timeout.is_none() {
                break;
            }
            let time = timeout.unwrap().peek_time();

            // Stop if the timeout has not passed yet.
            if now < time {
                self.next_timeout = time.min(self.next_timeout);
                break;
            }

            // Remove the instance now that we know that the timeout time has passed.
            let handle = self.timeouts.pop().unwrap().consume();

            // Some handles are shared outside the event loop, e.g. synchronization operations
            // between multiple event loops. In those cases we have to ensure that the task is not
            // enqueued multiple times due to race conditions.
            if let Some(task) = handle.try_consume() {
                // Now that consuming the handle was successful, we can wake the task back up.
                self.on_unblock_task(module, task, true);
            }
        }
    }

    fn add_timeout(&mut self, module: &TasksModule<'_>, timeout: time_out::TimeOut) {
        fimo_std::emit_trace!(module.context(), "adding time out, time out: {timeout:?}");

        let insertion_position = self
            .timeouts
            .binary_search_by_key(&timeout.peek_time(), |t| t.peek_time())
            .unwrap_or_else(|pos| pos);
        self.timeouts.insert(insertion_position, timeout);
    }

    fn handle_request(&mut self, module: &TasksModule<'_>) {
        const MIN_TIMEOUT: Duration = Duration::ZERO;
        const MAX_TIMEOUT: Duration = Duration::from_millis(5);

        // Compute the maximum timeout depending on the next requested timeout.
        let now = Instant::now();
        let timeout = self
            .next_timeout
            .checked_duration_since(now)
            .unwrap_or(MIN_TIMEOUT)
            .min(MAX_TIMEOUT);

        enum Request {
            Outer(OuterRequest),
            Inner(InnerRequest),
            None,
        }

        // Read a message from the two channels.
        let request;
        select! {
            recv(self.public_messages) -> msg => request = msg.map_or(Request::None, Request::Outer),
            recv(self.private_messages) -> msg => request = msg.map_or(Request::None, Request::Inner),
            default(timeout) => request = Request::None,
        }

        // Handle the messages.
        match request {
            Request::Outer(msg) => self.handle_outer_request(module, msg),
            Request::Inner(msg) => self.handle_inner_request(module, msg),
            Request::None => {}
        }

        // Check whether some operation timed out.
        self.handle_timeouts(module);
    }

    fn enter_event_loop(mut self, module: &TasksModule<'_>) {
        fimo_std::panic::abort_on_panic(|| {
            fimo_std::emit_trace!(module.context(), "starting event loop");
            while !self.can_join() {
                self.handle_request(module);
            }

            fimo_std::emit_trace!(module.context(), "joining worker threads");
            for worker in self.workers.values_mut() {
                worker.join();
            }
            fimo_std::emit_trace!(module.context(), "worker threads joined");
        });
    }
}
