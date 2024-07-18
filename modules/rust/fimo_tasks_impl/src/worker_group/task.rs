#![allow(dead_code)]

use crate::{
    module_export::ModuleToken,
    worker_group::worker_thread::{abort_task, complete_task, with_worker_context_lock},
};
use fimo_std::{error::Error, ffi::FFISharable, module::Module};
use fimo_tasks::{TaskId, WorkerId};
use rustc_hash::FxHashMap;
use std::{mem::ManuallyDrop, ops::Deref};

#[derive(Debug)]
pub struct EnqueuedTask {
    id: TaskId,
    index: usize,
    task: RawTask,
    stack: AcquiredStack,
    worker: Option<WorkerId>,
    local_data: Option<LocalData>,
    resume_context: Option<context::Context>,
}

impl EnqueuedTask {
    pub fn new(
        id: TaskId,
        index: usize,
        task: *mut fimo_tasks::bindings::FiTasksTask,
        stack: AcquiredStack,
    ) -> Self {
        debug_assert!(!task.is_null());

        extern "C" fn task_start(t: context::Transfer) -> ! {
            let context::Transfer { context, .. } = t;

            let _ = std::panic::catch_unwind(|| {
                let (context, mut task) = with_worker_context_lock(|worker| {
                    // Reset the context of the event loop.
                    worker.resume_context = Some(context);

                    // Extract the context.
                    // Safety: The module can not be unloaded until all commands have been executed.
                    let context = unsafe {
                        ModuleToken::with_current_unlocked(|module| {
                            module.exports().context().share_to_ffi()
                        })
                    };

                    // Extract the `RawTask`.
                    let task = worker.current_task.as_ref().unwrap();
                    (context, task.task)
                })
                .unwrap();

                match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    // Safety: The task is bound and has not started yet.
                    unsafe { task.start_task(context) };
                })) {
                    // Safety: The task is complete.
                    Ok(_) => unsafe { complete_task().unwrap() },
                    // Safety: The task was aborted.
                    Err(_) => unsafe { abort_task(std::ptr::null_mut()).unwrap() },
                }
            });

            std::process::abort();
        }

        let local_data = LocalData::new();
        // Safety: The stack will outlive the task.
        let resume_context = unsafe { context::Context::new(stack.memory(), task_start) };

        Self {
            id,
            index,
            task: RawTask(task),
            stack,
            worker: None,
            local_data: Some(local_data),
            resume_context: Some(resume_context),
        }
    }

    pub fn id(&self) -> TaskId {
        self.id
    }

    pub fn worker(&self) -> WorkerId {
        self.worker.expect("task not bound to a worker")
    }

    pub fn bind_to_worker(&mut self, worker: WorkerId) {
        if self.worker.is_some() {
            panic!("task already bound to a worker");
        }
        self.worker = Some(worker);
    }

    pub fn take_resume_context(&mut self) -> context::Context {
        self.resume_context.take().expect("task context missing")
    }

    pub fn set_resume_context(&mut self, context: context::Context) {
        if self.resume_context.is_some() {
            panic!("task context already present");
        }
        self.resume_context = Some(context);
    }

    /// # Safety
    ///
    /// Must be called by the thread that executed the task.
    /// The task must have been completed.
    /// The context must be locked.
    pub unsafe fn run_cleanup(&mut self) {
        // Safety: The caller ensures that it is sound.
        unsafe {
            self.cleanup_local_data();

            self.task.run_completion_handler();
            self.task.run_cleanup_handler();
        }
    }

    /// # Safety
    ///
    /// Must be called by the thread that executed the task.
    /// The task must have been aborted.
    /// The context must be locked.
    pub unsafe fn run_abort(&mut self, error: *mut std::ffi::c_void) {
        // Safety: The caller ensures that it is sound.
        unsafe {
            self.cleanup_local_data();

            self.task.run_abortion_handler(error);
            self.task.run_cleanup_handler();
        }
    }

    /// # Safety
    ///
    /// Must be called on the same thread that initialized all the values.
    /// The context must be locked.
    unsafe fn cleanup_local_data(&mut self) {
        let local_data = self
            .local_data
            .take()
            .expect("local data already cleaned up");
        // Safety: The caller ensures that it is sound.
        unsafe { local_data.clear_all_values() };
    }
}

impl Drop for EnqueuedTask {
    fn drop(&mut self) {
        // To ensure the safety constraints we don't allow dropping.
        std::process::abort();
    }
}

#[derive(Debug)]
pub struct AcquiredStack {
    size: usize,
    memory: StackMemory,
}

impl AcquiredStack {
    pub fn new(size: usize, memory: StackMemory) -> Self {
        Self { size, memory }
    }

    pub fn into_raw_parts(self) -> (usize, StackMemory) {
        (self.size, self.memory)
    }

    pub fn memory(&self) -> &StackMemory {
        &self.memory
    }
}

#[derive(Debug)]
pub enum StackMemory {
    Protected(context::stack::ProtectedFixedSizeStack),
    Unprotected(context::stack::FixedSizeStack),
}

impl Deref for StackMemory {
    type Target = context::stack::Stack;

    fn deref(&self) -> &Self::Target {
        match self {
            StackMemory::Protected(stack) => stack,
            StackMemory::Unprotected(stack) => stack,
        }
    }
}

#[derive(Debug)]
struct LocalData {
    values: FxHashMap<usize, LocalDataValue>,
}

impl LocalData {
    fn new() -> Self {
        Self {
            values: Default::default(),
        }
    }

    pub fn get_value(&self, key: usize) -> Option<*mut std::ffi::c_void> {
        self.values.get(&key).map(|v| v.value)
    }

    pub fn set_value(&mut self, key: usize, value: LocalDataValue) {
        self.values.insert(key, value);
    }

    /// # Safety
    ///
    /// Must be called on the same thread that created the value.
    /// The context must be locked.
    unsafe fn clear_value(&mut self, key: usize) -> Result<(), Error> {
        match self.values.remove(&key) {
            None => Err(Error::EINVAL),
            Some(value) => {
                // Safety: Ensured by the caller.
                unsafe {
                    value.cleanup();
                }
                Ok(())
            }
        }
    }

    /// # Safety
    ///
    /// Must be called on the same thread that initialized all the values.
    /// The context must be locked.
    unsafe fn clear_all_values(self) {
        let mut this = ManuallyDrop::new(self);
        for (_, value) in this.values.drain() {
            // Safety: Ensured by the caller.
            unsafe {
                value.cleanup();
            }
        }
    }
}

impl Drop for LocalData {
    fn drop(&mut self) {
        // To ensure the safety constraints we don't allow dropping.
        std::process::abort();
    }
}

#[derive(Debug)]
pub struct LocalDataValue {
    value: *mut std::ffi::c_void,
    on_cleanup: fimo_tasks::bindings::FiTasksTssDtor,
}

impl LocalDataValue {
    pub fn new(
        value: *mut std::ffi::c_void,
        on_cleanup: fimo_tasks::bindings::FiTasksTssDtor,
    ) -> Self {
        Self { value, on_cleanup }
    }

    /// # Safety
    ///
    /// Must be called from the same thread that created the instance.
    /// The context must be locked.
    unsafe fn cleanup(self) {
        if let Some(on_cleanup) = self.on_cleanup {
            // Safety: Ensured by the caller.
            unsafe {
                on_cleanup(self.value);
            }
        }
    }
}

// Safety: We ensure that the data is only accessed from the same thread that created it.
unsafe impl Send for LocalDataValue {}

#[derive(Debug, Copy, Clone)]
struct RawTask(*mut fimo_tasks::bindings::FiTasksTask);

impl RawTask {
    fn task_mut(&mut self) -> &mut fimo_tasks::bindings::FiTasksTask {
        // Safety: A `RawTask` works like a `Box`. We own the buffer.
        unsafe { &mut *self.0 }
    }

    /// # Safety
    ///
    /// The task must be bound.
    /// May only be called once.
    unsafe fn start_task(&mut self, context: fimo_tasks::bindings::FiTasksContext) {
        let task = self.task_mut();
        let start = task.start.unwrap();

        // Safety: The caller ensures that this is sound.
        unsafe { start(task.user_data, task, context) };
    }

    /// # Safety
    ///
    /// May only be called once when the execution of the task was successful.
    unsafe fn run_completion_handler(&mut self) {
        let task = self.task_mut();

        if let Some(on_complete) = task.on_complete {
            // Safety: The caller ensures that this is sound.
            unsafe {
                on_complete(task.status_callback_data, task);
            }
        }
    }

    /// # Safety
    ///
    /// May only be called once when the task was aborted.
    unsafe fn run_abortion_handler(&mut self, error: *mut std::ffi::c_void) {
        let task = self.task_mut();

        if let Some(on_abort) = task.on_abort {
            // Safety: The caller ensures that this is sound.
            unsafe {
                on_abort(task.status_callback_data, task, error);
            }
        }
    }

    /// # Safety
    ///
    /// May only be called once when the task is dropped.
    unsafe fn run_cleanup_handler(&mut self) {
        let task = self.task_mut();

        if let Some(on_cleanup) = task.on_cleanup {
            // Safety: The caller ensures that this is sound.
            unsafe {
                on_cleanup(task.cleanup_data, task);
            }
        }
    }
}

// Safety: A `FiTasksTask` is `Send`.
unsafe impl Send for RawTask {}
