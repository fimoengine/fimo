use fimo_tasks::WorkerId;
use std::{
    ffi::CString,
    fmt::{Debug, Formatter},
    sync::Weak,
};

use crate::{context::ContextImpl};

mod command_buffer;
mod event_loop;
mod task;
mod worker_thread;

pub struct WorkerGroupImpl {
    id: WorkerId,
    name: CString,
    event_loop: event_loop::EventLoopHandle,
    _ctx: Weak<ContextImpl>,
}

impl WorkerGroupImpl {}

impl Debug for WorkerGroupImpl {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WorkerGroupImpl")
            .field("id", &self.id)
            .field("name", &self.name)
            .field("event_loop", &self.event_loop)
            .finish_non_exhaustive()
    }
}
