use crate::rust::{NotifyFn, WaitOnFn};
use std::any::Any;
use std::fmt::Debug;
use std::mem::forget;
use std::pin::Pin;
use std::sync::Arc;

/// A task.
#[derive(Debug)]
pub struct RawTask {
    inner: Pin<Arc<dyn TaskInner>>,
}

/// Handle to a task.
#[derive(Debug, Copy, Clone, Hash, Ord, PartialOrd, PartialEq, Eq)]
pub struct TaskHandle {
    /// Task identifier.
    pub id: usize,
    /// Generation of the task.
    pub generation: usize,
}

/// Status of a task.
#[derive(Debug, Copy, Clone, Hash, Ord, PartialOrd, PartialEq, Eq)]
pub enum TaskStatus {
    /// The task is blocked and can not be resumed.
    Blocked,
    /// The task is waiting on another task.
    Waiting,
    /// The task is runnable.
    Runnable,
    /// The task has been aborted.
    Aborted,
    /// The task has finished.
    Finished,
}

/// A specialized [`Result`] type for tasks.
pub type Result<T> = std::result::Result<T, Option<Box<dyn Any + Send + 'static>>>;

/// Definition of a task.
pub trait TaskInner: Debug + Send + Sync {
    /// Polls whether the task is, or was, in the process
    /// of unwinding it's stack.
    fn panicking(&self) -> bool;

    /// Takes the panic error of the task.
    ///
    /// # Safety
    ///
    /// Calling this before the task has completed can lead to race conditions.
    unsafe fn take_panic_error(&self) -> Option<Box<dyn Any + Send + 'static>>;

    /// Polls whether the task has finished or was aborted.
    fn is_completed(&self) -> bool;

    /// Polls whether the task is blocked.
    fn is_blocked(&self) -> bool;

    /// Polls whether the task is aborted.
    fn is_aborted(&self) -> bool;

    /// Polls whether the task is finished.
    fn is_finished(&self) -> bool;

    /// Polls the status of the task.
    fn poll_status(&self) -> TaskStatus;

    /// Fetches the internal handle.
    fn get_handle(&self) -> TaskHandle;

    /// Sends a termination signal to the task.
    ///
    /// Returns whether the task could be signaled.
    ///
    /// # Panics
    ///
    /// **Must** be run from within a task.
    ///
    /// # Safety
    ///
    /// Aborting a task can lead to uninitialized values.
    unsafe fn abort(&self) -> bool;

    /// Unblocks the task.
    ///
    /// # Panics
    ///
    /// **Must** be run from within a task.
    ///
    /// # Safety
    ///
    /// See [`TaskRuntimeInner::unblock_task`](super::TaskRuntimeInner::unblock_task).
    unsafe fn unblock(&self);

    /// Blocks the current task until the task has completed.
    ///
    /// # Panics
    ///
    /// **Must** be run from within a task.
    fn wait_on(&self) {
        self.wait_on_if(None)
    }

    /// Blocks the current task until the task has completed.
    ///
    /// See [`TaskRuntimeInner::wait_on_if`](super::TaskRuntimeInner::wait_on_if).
    ///
    /// # Panics
    ///
    /// **Must** be run from within a task.
    fn wait_on_if(&self, predicate: Option<WaitOnFn>);

    /// Resumes the operation of one waiter.
    ///
    /// # Panics
    ///
    /// **Must** be run from within a task.
    ///
    /// # Safety
    ///
    /// See [`TaskRuntimeInner::notify_finished_one`](super::TaskRuntimeInner::notify_finished_one).
    unsafe fn notify_finished_one(&self) {
        self.notify_finished_one_and_then(None)
    }

    /// Resumes the operation of one waiter.
    ///
    /// # Panics
    ///
    /// **Must** be run from within a task.
    ///
    /// # Safety
    ///
    /// See [`TaskRuntimeInner::notify_finished_one_and_then`](super::TaskRuntimeInner::notify_finished_one_and_then).
    unsafe fn notify_finished_one_and_then(&self, and_then: Option<NotifyFn>);

    /// Resumes the operation of all waiters.
    ///
    /// # Panics
    ///
    /// **Must** be run from within a task.
    ///
    /// # Safety
    ///
    /// See [`TaskRuntimeInner::broadcast_finished`](super::TaskRuntimeInner::broadcast_finished).
    unsafe fn broadcast_finished(&self) {
        self.broadcast_finished_and_then(None)
    }

    /// Resumes the operation of all waiters.
    ///
    /// # Panics
    ///
    /// **Must** be run from within a task.
    ///
    /// # Safety
    ///
    /// See [`TaskRuntimeInner::broadcast_finished_and_then`](super::TaskRuntimeInner::broadcast_finished_and_then).
    unsafe fn broadcast_finished_and_then(&self, and_then: Option<NotifyFn>);
}

impl RawTask {
    /// Resumes the execution of the current task after
    /// the task has been completed.
    ///
    /// # Panics
    ///
    /// **Must** be run from within a task.
    ///
    /// # Note
    ///
    /// Joining won't unblock the task.
    pub fn join(self) -> Result<()> {
        // wait for task to finish.
        self.wait_on();
        let result = if self.is_aborted() {
            unsafe { Err(self.inner.take_panic_error()) }
        } else {
            Ok(())
        };

        // drop the raw value instead of dropping the `RawTask`
        // to avoid waiting again.
        let raw = self.into_raw();
        drop(raw);

        result
    }

    /// Constructs a `RawTask` from a raw task.
    ///
    /// # Safety
    ///
    /// The raw task must have previously been returned by a call to
    /// [`RawTask::into_raw`].
    pub unsafe fn from_raw(raw: Pin<Arc<dyn TaskInner>>) -> Self {
        Self { inner: raw }
    }

    /// Extracts the raw task.
    ///
    /// Dropping the raw value won't join the task.
    pub fn into_raw(self) -> Pin<Arc<dyn TaskInner>> {
        // bypass the dropping of the value.
        let copy = unsafe { (&self.inner as *const Pin<Arc<dyn TaskInner>>).read() };
        forget(self);
        copy
    }

    /// Polls whether the task is, or was, in the process
    /// of unwinding it's stack.
    pub fn panicking(&self) -> bool {
        self.inner.panicking()
    }

    /// Polls whether the task has finished or was aborted.
    pub fn is_completed(&self) -> bool {
        self.inner.is_completed()
    }

    /// Polls whether the task is blocked.
    pub fn is_blocked(&self) -> bool {
        self.inner.is_blocked()
    }

    /// Polls whether the task is aborted.
    pub fn is_aborted(&self) -> bool {
        self.inner.is_aborted()
    }

    /// Polls whether the task is finished.
    pub fn is_finished(&self) -> bool {
        self.inner.is_finished()
    }

    /// Polls the status of the task.
    pub fn poll_status(&self) -> TaskStatus {
        self.inner.poll_status()
    }

    /// Fetches the internal handle.
    pub fn get_handle(&self) -> TaskHandle {
        self.inner.get_handle()
    }

    /// Sends a termination signal to the task.
    ///
    /// Returns whether the task could be signaled.
    ///
    /// # Panics
    ///
    /// **Must** be run from within a task.
    ///
    /// # Safety
    ///
    /// Aborting a task can lead to uninitialized values.
    pub unsafe fn abort(&mut self) -> bool {
        self.inner.abort()
    }

    /// Unblocks the task.
    ///
    /// # Panics
    ///
    /// **Must** be run from within a task.
    ///
    /// # Safety
    ///
    /// See [`TaskRuntimeInner::unblock_task`](super::TaskRuntimeInner::unblock_task).
    pub unsafe fn unblock(&mut self) {
        if self.is_blocked() {
            self.inner.unblock()
        }
    }

    /// Blocks the current task until the task has completed.
    ///
    /// # Panics
    ///
    /// **Must** be run from within a task.
    pub fn wait_on(&self) {
        self.wait_on_if(None)
    }

    /// Blocks the current task until the task has completed.
    ///
    /// See [`TaskRuntimeInner::wait_on_if`](super::TaskRuntimeInner::wait_on_if).
    ///
    /// # Panics
    ///
    /// **Must** be run from within a task.
    pub fn wait_on_if(&self, predicate: Option<WaitOnFn>) {
        if !self.is_completed() {
            self.inner.wait_on_if(predicate)
        }
    }

    /// Resumes the operation of one waiter.
    ///
    /// # Panics
    ///
    /// **Must** be run from within a task.
    ///
    /// # Safety
    ///
    /// See [`TaskRuntimeInner::notify_finished_one`](super::TaskRuntimeInner::notify_finished_one).
    pub unsafe fn notify_finished_one(&self) {
        self.notify_finished_one_and_then(None)
    }

    /// Resumes the operation of one waiter.
    ///
    /// # Panics
    ///
    /// **Must** be run from within a task.
    ///
    /// # Safety
    ///
    /// See [`TaskRuntimeInner::notify_finished_one_and_then`](super::TaskRuntimeInner::notify_finished_one_and_then).
    pub unsafe fn notify_finished_one_and_then(&self, after_wake: Option<NotifyFn>) {
        if self.is_blocked() {
            self.inner.notify_finished_one_and_then(after_wake)
        }
    }

    /// Resumes the operation of all waiters.
    ///
    /// # Panics
    ///
    /// **Must** be run from within a task.
    ///
    /// # Safety
    ///
    /// See [`TaskRuntimeInner::broadcast_finished`](super::TaskRuntimeInner::broadcast_finished).
    pub unsafe fn broadcast_finished(&self) {
        self.broadcast_finished_and_then(None)
    }

    /// Resumes the operation of all waiters.
    ///
    /// # Panics
    ///
    /// **Must** be run from within a task.
    ///
    /// # Safety
    ///
    /// See [`TaskRuntimeInner::broadcast_finished_and_then`](super::TaskRuntimeInner::broadcast_finished_and_then).
    pub unsafe fn broadcast_finished_and_then(&self, after_wake: Option<NotifyFn>) {
        if self.is_blocked() {
            self.inner.broadcast_finished_and_then(after_wake)
        }
    }
}

impl Drop for RawTask {
    fn drop(&mut self) {
        // unblock the task if it is blocked.
        unsafe { self.unblock() };

        // wait for task to finish.
        self.wait_on()
    }
}
