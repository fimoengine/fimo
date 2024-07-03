use crate::{
    module_export::Module,
    worker_group::{
        command_buffer::{CommandBufferHandleId, CommandBufferHandleImpl},
        task::EnqueuedTask,
        worker_thread::{TaskRequest, TaskResponse, WorkerHandle, WorkerRequest, WorkerResponse},
        WorkerGroupImpl,
    },
};
use crossbeam_channel::{select, Receiver, Sender, TrySendError};
use fimo_std::error::Error;
use fimo_tasks::{TaskId, WorkerId};
use rustc_hash::FxHashMap;
use std::{
    cell::Cell,
    collections::VecDeque,
    fmt::{Debug, Formatter},
    sync::{Arc, RwLock, Weak},
    thread::JoinHandle,
    time::{Duration, Instant},
};

#[derive(Debug)]
pub enum OuterRequest {
    Close,
}

#[derive(Debug)]
pub enum InnerRequest {
    WorkerRequest(WorkerRequest),
}

pub struct EventLoopHandle {
    group: Weak<WorkerGroupImpl>,
    connection_status: RwLock<Cell<ConnectionStatus>>,
    outer_requests: Sender<OuterRequest>,
    inner_requests: Sender<InnerRequest>,
    handle: Option<JoinHandle<()>>,
}

#[derive(Debug, Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Hash)]
enum ConnectionStatus {
    Open,
    Closed,
}

impl EventLoopHandle {
    pub fn is_open(&self) -> bool {
        self.connection_status
            .read()
            .map(|s| s.get() == ConnectionStatus::Open)
            .unwrap_or(false)
    }

    pub fn request_close(&self) -> Result<(), Error> {
        // Acquire the `RwLock` with `write` permissions, such that no messages are sent while we
        // try to send the close message.
        let status = self
            .connection_status
            .write()
            .map_err(|_e| Error::ECANCELED)?;

        // If the channel is already closed we can return.
        if status.get() == ConnectionStatus::Closed {
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
        status.set(ConnectionStatus::Closed);

        Ok(())
    }
}

impl Debug for EventLoopHandle {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MainThreadDataPublic")
            .field("connection_status", &self.connection_status)
            .field("public_messages", &self.outer_requests)
            .field("handle", &self.handle)
            .finish_non_exhaustive()
    }
}

#[derive(Debug)]
struct EventLoop {
    is_closed: bool,
    next_timeout: Instant,
    group: Arc<WorkerGroupImpl>,
    public_messages: Receiver<OuterRequest>,
    private_messages: Receiver<InnerRequest>,
    workers: FxHashMap<WorkerId, WorkerHandle>,
    handles: FxHashMap<CommandBufferHandleId, CommandBufferHandleData>,
    tasks: FxHashMap<TaskId, usize>,
    timeouts: Vec<TimeOut>,
    module: Module<'static>,
}

// Outer requests.
impl EventLoop {
    fn on_close(&mut self) {
        self.is_closed = true;
    }
}

// Inner requests.
impl EventLoop {
    fn on_worker_request(&mut self, request: WorkerRequest) {
        let WorkerRequest { task, request } = request;
        match request {
            TaskRequest::Complete => {
                todo!("Run complete of the command buffer");
            }
            TaskRequest::Abort(_) => {
                todo!("Run abort of the command buffer");
            }
            TaskRequest::Yield => {
                unreachable!("yields should not return to the event loop")
            }
            TaskRequest::WaitUntil(time) => {
                // Insert the timeout into our timeout queue.
                let timeout = TimeOut {
                    time,
                    handle: TimeOutHandle::TaskWaitUntil(task),
                };
                self.add_timeout(timeout);
            }
            TaskRequest::WaitOnCommandBuffer(_) => {
                todo!("Check if the handle is complete");
            }
        }
    }
}

// Event loop implementation.
impl EventLoop {
    fn can_join(&self) -> bool {
        self.is_closed && self.handles.is_empty()
    }

    fn handle_outer_request(&mut self, msg: OuterRequest) {
        match msg {
            OuterRequest::Close => self.on_close(),
        }
    }

    fn handle_inner_request(&mut self, msg: InnerRequest) {
        match msg {
            InnerRequest::WorkerRequest(request) => self.on_worker_request(request),
        }
    }

    fn handle_timeouts(&mut self) {
        let now = Instant::now();

        // Loop over all timeout instances.
        // The array is sorted such that `t[i].time >= t[i+1].time`.
        loop {
            // If there are no more instances we are done.
            let timeout = self.timeouts.last();
            if timeout.is_none() {
                break;
            }
            let TimeOut { time, .. } = timeout.unwrap();

            // Stop if the timeout has not passed yet.
            if now < *time {
                self.next_timeout = (*time).min(self.next_timeout);
                break;
            }

            // Remove the instance now that we know that the timeout time has passed.
            let TimeOut { handle, .. } = self.timeouts.pop().unwrap();
            match handle {
                // Wake the task back up now that the timer has passed.
                TimeOutHandle::TaskWaitUntil(task) => {
                    let worker_id = task.worker();
                    let worker = &self.workers[&worker_id];
                    worker.push_local_response(WorkerResponse {
                        task,
                        response: TaskResponse::WaitUntil,
                    });
                }
            }
        }
    }

    fn add_timeout(&mut self, timeout: TimeOut) {
        let insertion_position = self
            .timeouts
            .binary_search_by_key(&timeout.time, |t| t.time)
            .unwrap_or_else(|pos| pos);
        self.timeouts.insert(insertion_position, timeout);
    }

    fn handle_request(&mut self) {
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
            Request::Outer(msg) => self.handle_outer_request(msg),
            Request::Inner(msg) => self.handle_inner_request(msg),
            Request::None => {}
        }

        // Check whether some operation timed out.
        self.handle_timeouts();
    }

    fn enter_event_loop(mut self) {
        while !self.can_join() {
            self.handle_request();
        }
    }
}

#[derive(Debug)]
struct CommandBufferHandleData {
    handle: Arc<CommandBufferHandleImpl>,
    waiters: VecDeque<Waiter>,
}
#[derive(Debug, Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Hash)]
enum Waiter {
    Task(TaskId),
    CommandBuffer(CommandBufferHandleId),
}

#[derive(Debug)]
struct TimeOut {
    time: Instant,
    handle: TimeOutHandle,
}

#[derive(Debug)]
enum TimeOutHandle {
    TaskWaitUntil(EnqueuedTask),
}
