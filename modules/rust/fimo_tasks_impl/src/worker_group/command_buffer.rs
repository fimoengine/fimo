use crate::worker_group::{worker_thread::wait_on_command_buffer, WorkerGroupImpl};
use fimo_std::error::Error;
use std::{
    fmt::{Debug, Formatter},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Weak,
    },
};

#[derive(Debug, Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Hash)]
pub struct CommandBufferHandleId(pub usize);

pub struct CommandBufferHandleImpl {
    id: CommandBufferHandleId,
    status: AtomicBool,
    completed: AtomicBool,
    group: Weak<WorkerGroupImpl>,
}

impl CommandBufferHandleImpl {
    pub fn completion_status(&self) -> Option<bool> {
        if self.completed.load(Ordering::Acquire) {
            Some(self.completed.load(Ordering::Relaxed))
        } else {
            None
        }
    }

    /// # Safety
    ///
    /// May only be called once after the completion/abortion of the command buffer.
    pub unsafe fn mark_completed(&self, aborted: bool) {
        self.status.store(aborted, Ordering::Relaxed);
        let completed = self.completed.swap(true, Ordering::Release);
        debug_assert!(!completed);
    }

    pub fn worker_group(&self) -> Result<Arc<WorkerGroupImpl>, Error> {
        self.group.upgrade().ok_or(Error::ECANCELED)
    }

    pub fn worker_group_weak(&self) -> &Weak<WorkerGroupImpl> {
        &self.group
    }

    pub fn wait_on(self: Arc<Self>) -> Result<bool, (Error, Arc<Self>)> {
        // If the handle was already marked as completed we can return early.
        if let Some(aborted) = self.completion_status() {
            return Ok(aborted);
        }

        // Request that the worker wait on the completion of the buffer.
        wait_on_command_buffer(self)
    }
}

impl Debug for CommandBufferHandleImpl {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CommandBufferHandleImpl")
            .field("completion_status", &self.completion_status())
            .finish_non_exhaustive()
    }
}

#[derive(Debug)]
pub struct RawCommandBuffer(*mut fimo_tasks::bindings::FiTasksCommandBuffer);

impl RawCommandBuffer {
    fn buffer(&self) -> &fimo_tasks::bindings::FiTasksCommandBuffer {
        // Safety: A `RawCommandBuffer` works like a `Box`. We own the buffer.
        unsafe { &*self.0 }
    }

    fn buffer_mut(&mut self) -> &mut fimo_tasks::bindings::FiTasksCommandBuffer {
        // Safety: A `RawCommandBuffer` works like a `Box`. We own the buffer.
        unsafe { &mut *self.0 }
    }

    /// # Safety
    ///
    /// May only be called once when the execution of the command buffer was successful.
    pub unsafe fn run_completion_handler(&mut self) {
        let buffer = self.buffer_mut();

        if let Some(on_complete) = buffer.on_complete {
            // Safety: The caller ensures that this is sound.
            unsafe { on_complete(buffer.status_callback_data, buffer) };
        }
    }

    /// # Safety
    ///
    /// May only be called once when the execution of the command buffer was aborted.
    pub unsafe fn run_abortion_handler(&mut self, index: usize) {
        let buffer = self.buffer_mut();

        if let Some(on_abort) = buffer.on_abort {
            // Safety: The caller ensures that this is sound.
            unsafe { on_abort(buffer.status_callback_data, buffer, index) };
        }
    }
}

// Safety: A `FiTasksCommandBuffer` is `Send`.
unsafe impl Send for RawCommandBuffer {}

impl Drop for RawCommandBuffer {
    fn drop(&mut self) {
        let buffer = self.buffer_mut();

        if let Some(on_cleanup) = buffer.on_cleanup {
            // Safety: We only call cleanup once.
            unsafe { on_cleanup(buffer.status_callback_data, buffer) };
        }
    }
}
