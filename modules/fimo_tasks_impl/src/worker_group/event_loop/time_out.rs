use fimo_tasks_meta::TaskId;
use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::Instant,
};

#[derive(Debug)]
pub(super) struct TimeOut {
    time: Instant,
    handle: TimeOutHandle,
}

impl TimeOut {
    pub fn new(time: Instant, handle: TimeOutHandle) -> Self {
        Self { time, handle }
    }

    pub fn peek_time(&self) -> Instant {
        self.time
    }

    pub fn consume(self) -> TimeOutHandle {
        self.handle
    }
}

#[derive(Debug)]
pub(super) enum TimeOutHandle {
    Internal(TaskId),
    #[allow(dead_code)]
    External(Arc<ExternalTimeOutHandle>),
}

impl TimeOutHandle {
    pub fn try_consume(self) -> Option<TaskId> {
        match self {
            TimeOutHandle::Internal(task) => Some(task),
            TimeOutHandle::External(handle) => handle.try_consume(),
        }
    }
}

#[derive(Debug)]
pub struct ExternalTimeOutHandle {
    task: TaskId,
    consumed: AtomicBool,
}

impl ExternalTimeOutHandle {
    #[allow(dead_code)]
    fn new(task: TaskId) -> Arc<Self> {
        Arc::new(Self {
            task,
            consumed: AtomicBool::new(false),
        })
    }

    pub fn try_consume(self: Arc<Self>) -> Option<TaskId> {
        let is_consumed = self.consumed.swap(true, Ordering::Acquire);
        if is_consumed {
            None
        } else {
            Some(self.task)
        }
    }
}
