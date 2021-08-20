use crate::rust::{RawTask, Result, TaskHandle};
use std::fmt::{Debug, Formatter};
use std::ops::Deref;
use std::time::{Duration, Instant};

/// The task runtime.
#[repr(transparent)]
pub struct TaskRuntime {
    inner: dyn TaskRuntimeInner,
}

/// Id of a worker thread.
#[derive(Debug, Copy, Clone, Hash, Ord, PartialOrd, PartialEq, Eq)]
pub struct WorkerId(pub usize);

/// Function passed to the [`TaskRuntimeInner::wait_on_if`] function.
#[derive(Copy, Clone)]
pub struct WaitOnFn {
    /// Data passed to the function.
    pub data: usize,
    /// Callback
    ///
    /// Is only called if the runtime intends to put the task to sleep.
    /// Can be used to control whether to make the task wait.
    ///
    /// Is called while the runtime is locked and
    /// can not call into the runtime or panic.
    pub validate: fn(data: usize) -> bool,
    /// The function that is called ofter the task has been put to sleep.
    ///
    /// Is called while the runtime is locked and
    /// can not call into the runtime or panic.
    pub after_sleep: fn(notify_fn: &mut dyn FnMut(TaskHandle), data: usize),
}

/// Function passed to the [`TaskRuntimeInner::notify_finished_one_and_then`] and
/// [`TaskRuntimeInner::broadcast_finished_and_then`] functions.
#[derive(Debug, Copy, Clone, Hash, Ord, PartialOrd, PartialEq, Eq)]
pub struct NotifyFn {
    /// Data passed to the function.
    pub data: usize,
    /// Callback
    ///
    /// The function will be called while the runtime is locked
    /// and is not allowed to call into the runtime or panic.
    pub function: fn(waiters: usize, data: usize),
}

/// Definition of the task runtime.
pub trait TaskRuntimeInner: Send + Sync {
    /// Enters the runtime with a function.
    ///
    /// The current thread is blocked until the task has run.
    /// Like [`TaskRuntimeInner::execute_task`] but panics if the task was
    /// aborted with a panic.
    ///
    /// # Note
    ///
    /// Can **must** called from outside the runtime.
    fn enter_runtime(&self, f: Box<dyn FnOnce() + Send>) {
        if let Err(Some(e)) = self.execute_task(f, &[]) {
            std::panic::resume_unwind(e)
        }
    }

    /// Spawns and waits on the new task,
    ///
    /// Returns the status and whether the task panicked.
    ///
    /// # Note
    ///
    /// Can **must** called from outside the runtime.
    fn execute_task(&self, f: Box<dyn FnOnce() + Send>, dependencies: &[TaskHandle]) -> Result<()>;

    /// Spawns a new task.
    ///
    /// # Panics
    ///
    /// **Must** be run from within a task.
    fn spawn_task(&self, f: Box<dyn FnOnce() + Send>, dependencies: &[TaskHandle]) -> RawTask;

    /// Spawns the same task on each worker thread.
    ///
    /// # Panics
    ///
    /// **Must** be run from within a task.
    fn spawn_all(&self, f: Box<dyn SpawnAllFn>, dependencies: &[TaskHandle]) -> RawTask;

    /// Spawns a new empty task.
    ///
    /// # Panics
    ///
    /// **Must** be run from within a task.
    fn spawn_empty(&self, dependencies: &[TaskHandle]) -> RawTask;

    /// Spawns a new blocked task.
    ///
    /// # Note
    ///
    /// The task must be unblocked before dropping or
    /// else will wait forever.
    ///
    /// # Panics
    ///
    /// **Must** be run from within a task.
    fn spawn_task_blocked(
        &self,
        f: Box<dyn FnOnce() + Send>,
        dependencies: &[TaskHandle],
    ) -> RawTask;

    /// Spawns a new blocked empty task.
    ///
    /// # Note
    ///
    /// The task must be unblocked before dropping or
    /// else will wait forever.
    ///
    /// # Panics
    ///
    /// **Must** be run from within a task.
    fn spawn_empty_blocked(&self, dependencies: &[TaskHandle]) -> RawTask;

    /// Yields the current task.
    ///
    /// # Panics
    ///
    /// **Must** be run from within a task.
    fn yield_now(&self);

    /// Yields the current task until a minimum time has reached.
    ///
    /// # Panics
    ///
    /// **Must** be run from within a task.
    fn yield_until(&self, instant: Instant);

    /// Yields the current task for a certain duration.
    ///
    /// # Panics
    ///
    /// **Must** be run from within a task.
    fn yield_for(&self, duration: Duration) {
        self.yield_until(Instant::now() + duration)
    }

    /// Blocks the current task indefinitely.
    ///
    /// # Panics
    ///
    /// **Must** be run from within a task.
    ///
    /// # Safety
    ///
    /// The task must be unblocked before it is dropped.
    unsafe fn block(&self);

    /// Unblocks a task.
    ///
    /// # Panics
    ///
    /// **Must** be run from within a task.
    ///
    /// # Safety
    ///
    /// Some tasks are meant to remain blocked until they are dropped.
    unsafe fn unblock_task(&self, task: TaskHandle);

    /// Blocks the current task until the other task has completed.
    ///
    /// # Panics
    ///
    /// **Must** be run from within a task.
    fn wait_on(&self, task: TaskHandle) {
        self.wait_on_if(task, None)
    }

    /// Blocks the current task until the other task has completed.
    ///
    /// See [`WaitOnFn`] for more information on the properties of the predicate.
    ///
    /// # Panics
    ///
    /// **Must** be run from within a task.
    fn wait_on_if(&self, task: TaskHandle, predicate: Option<WaitOnFn>);

    /// Aborts the current task.
    ///
    /// # Panics
    ///
    /// **Must** be run from within a task.
    ///
    /// # Safety
    ///
    /// Aborting can lead to uninitialized values.
    unsafe fn abort(&self) -> !;

    /// Notifies all waiters that the task has finished
    /// without changing the status.
    ///
    /// # Panics
    ///
    /// **Must** be run from within a task.
    ///
    /// # Safety
    ///
    /// See [`TaskRuntimeInner::notify_finished_one()`].
    unsafe fn broadcast_finished(&self, task: TaskHandle) {
        self.broadcast_finished_and_then(task, None)
    }

    /// Notifies all waiters that the task has finished
    /// without changing the status.
    ///
    /// # Panics
    ///
    /// **Must** be run from within a task.
    ///
    /// # Safety
    ///
    /// See [`TaskRuntimeInner::notify_finished_one()`].
    unsafe fn broadcast_finished_and_then(&self, task: TaskHandle, after_wait: Option<NotifyFn>);

    /// Notifies one waiter that the task has finished
    /// without changing the status.
    ///
    /// # Panics
    ///
    /// **Must** be run from within a task.
    ///
    /// # Safety
    ///
    /// A waiting task may require that the task fully finishes before
    /// resuming execution. This function is mainly intended to be
    /// used for the implementation of condition variables.
    unsafe fn notify_finished_one(&self, task: TaskHandle) {
        self.notify_finished_one_and_then(task, None)
    }

    /// Notifies one waiter that the task has finished
    /// without changing the status.
    ///
    /// # Panics
    ///
    /// **Must** be run from within a task.
    ///
    /// # Safety
    ///
    /// A waiting task may require that the task fully finishes before
    /// resuming execution. This function is mainly intended to be
    /// used for the implementation of condition variables.
    ///
    /// The `after_wake` function will be called while the runtime is locked
    /// and is not allowed to call into the runtime or panic.
    unsafe fn notify_finished_one_and_then(&self, task: TaskHandle, after_wait: Option<NotifyFn>);

    /// Fetches the id of the current worker thread.
    ///
    /// # Panics
    ///
    /// **Must** be run from within a task.
    fn get_worker_id(&self) -> WorkerId;
}

/// A function that can be passed to the
/// [`TaskRuntimeInner::spawn_all`] function.
pub trait SpawnAllFn: FnOnce() + Send {
    /// Clones the function into a box.
    fn boxed_clone(&self) -> Box<dyn FnOnce() + Send>;

    /// Fetches a mutable reference to the function.
    fn as_fn_once(&mut self) -> &mut (dyn FnOnce() + Send);
}

impl<T> SpawnAllFn for T
where
    T: FnOnce() + Clone + Send + 'static,
{
    fn boxed_clone(&self) -> Box<dyn FnOnce() + Send> {
        Box::new(Clone::clone(self))
    }

    fn as_fn_once(&mut self) -> &mut (dyn FnOnce() + Send) {
        self
    }
}

impl TaskRuntime {
    /// Enters the runtime with a function.
    ///
    /// The current thread is blocked until the task has run.
    ///
    /// # Note
    ///
    /// Can **must** called from outside the runtime.
    pub fn enter_runtime(&self, f: impl FnOnce() + Send) {
        if let Err(Some(e)) = self.execute_task(f, &[]) {
            std::panic::resume_unwind(e)
        }
    }

    /// Spawns and waits on a new task,
    ///
    /// Returns the status and whether the task panicked.
    ///
    /// # Note
    ///
    /// Can **must** called from outside the runtime.
    pub fn execute_task(
        &self,
        f: impl FnOnce() + Send,
        dependencies: impl AsRef<[TaskHandle]>,
    ) -> Result<()> {
        let boxed = unsafe {
            std::mem::transmute::<Box<dyn FnOnce() + Send>, Box<dyn FnOnce() + Send + 'static>>(
                Box::new(f),
            )
        };

        (**self).execute_task(boxed, dependencies.as_ref())
    }

    /// Spawns a new task for each worker thread.
    ///
    /// # Panics
    ///
    /// **Must** be run from within a task.
    pub fn spawn_all(
        &self,
        f: impl FnOnce() + Send + Clone + 'static,
        dependencies: impl AsRef<[TaskHandle]>,
    ) -> RawTask {
        (**self).spawn_all(Box::new(f), dependencies.as_ref())
    }

    /// Spawns a new task.
    ///
    /// # Panics
    ///
    /// **Must** be run from within a task.
    pub fn spawn_task(
        &self,
        f: impl FnOnce() + Send + 'static,
        dependencies: impl AsRef<[TaskHandle]>,
    ) -> RawTask {
        (**self).spawn_task(Box::new(f), dependencies.as_ref())
    }

    /// Spawns a new blocked task.
    ///
    /// # Note
    ///
    /// The task must be unblocked before dropping or
    /// else will wait forever.
    ///
    /// # Panics
    ///
    /// **Must** be run from within a task.
    pub fn spawn_task_blocked(
        &self,
        f: impl FnOnce() + Send + 'static,
        dependencies: impl AsRef<[TaskHandle]>,
    ) -> RawTask {
        (**self).spawn_task_blocked(Box::new(f), dependencies.as_ref())
    }

    /// Spawns a new empty task.
    ///
    /// # Panics
    ///
    /// **Must** be run from within a task.
    pub fn spawn_empty(&self, dependencies: impl AsRef<[TaskHandle]>) -> RawTask {
        (**self).spawn_empty(dependencies.as_ref())
    }

    /// Spawns a new blocked empty task.
    ///
    /// # Note
    ///
    /// The task must be unblocked before dropping or
    /// else will wait forever.
    ///
    /// # Panics
    ///
    /// **Must** be run from within a task.
    pub fn spawn_empty_blocked(&self, dependencies: impl AsRef<[TaskHandle]>) -> RawTask {
        (**self).spawn_empty_blocked(dependencies.as_ref())
    }
}

impl Deref for TaskRuntime {
    type Target = dyn TaskRuntimeInner;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl Debug for TaskRuntime {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "(TaskRuntime)")
    }
}

impl Debug for WaitOnFn {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "(WaitOnFn)")
    }
}
