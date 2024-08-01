use crate::{
    module_export::TasksModule,
    worker_group::{
        task::RawTask, worker_thread::wait_on_command_buffer, WorkerGroupFFI, WorkerGroupImpl,
    },
};
use fimo_std::{
    bindings as std_bindings,
    error::Error,
    ffi::{FFISharable, FFITransferable},
    module::Module,
};
use fimo_tasks::{
    bindings::{self, FiTasksCommandBufferEntryType},
    TaskId, WorkerId,
};
use rustc_hash::FxHashMap;
use std::{
    collections::VecDeque,
    ffi::CStr,
    fmt::{Debug, Formatter},
    num::NonZeroUsize,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Weak,
    },
};

#[derive(Debug, Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Hash)]
pub struct CommandBufferId(pub usize);

pub struct CommandBufferHandleImpl {
    id: CommandBufferId,
    status: AtomicBool,
    completed: AtomicBool,
    group: Weak<WorkerGroupImpl>,
}

impl CommandBufferHandleImpl {
    /// # Safety
    ///
    /// - `buffer` must be dereferencable.
    pub unsafe fn new(
        group: Arc<WorkerGroupImpl>,
        buffer: *mut bindings::FiTasksCommandBuffer,
    ) -> Result<Arc<Self>, Error> {
        // Safety: Ensured by the caller.
        let buffer = unsafe { CommandBufferImpl::new(&group, buffer) };

        let guard = group
            .event_loop
            .read()
            .expect("failed to lock event loop handle");
        match guard.as_ref() {
            Some(handle) => handle.enqueue_command_buffer(buffer),
            None => Err(Error::EINVAL),
        }
    }

    pub fn id(&self) -> CommandBufferId {
        self.id
    }

    pub fn is_completed(&self) -> bool {
        self.completion_status().is_some()
    }

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
#[repr(transparent)]
pub struct CommandBufferHandleFFI(pub Arc<CommandBufferHandleImpl>);

impl CommandBufferHandleFFI {
    const VTABLE: &bindings::FiTasksCommandBufferHandleVTable =
        &bindings::FiTasksCommandBufferHandleVTable {
            v0: bindings::FiTasksCommandBufferHandleVTableV0 {
                acquire: Some(Self::acquire),
                release: Some(Self::release),
                worker_group: Some(Self::worker_group),
                wait_on: Some(Self::wait_on),
            },
        };

    unsafe extern "C" fn acquire(this: *mut std::ffi::c_void) {
        fimo_std::panic::abort_on_panic(|| {
            // Safety: Must be ensured by the caller.
            let this = unsafe { Self::borrow_from_ffi(this) };

            // Safety: Is always in an Arc.
            unsafe { Arc::increment_strong_count(this) };
        });
    }

    unsafe extern "C" fn release(this: *mut std::ffi::c_void) {
        fimo_std::panic::abort_on_panic(|| {
            // Safety: Must be ensured by the caller.
            let this = unsafe { Self::borrow_from_ffi(this) };

            // Safety: Is always in an Arc.
            unsafe { Arc::decrement_strong_count(this) };
        });
    }

    unsafe extern "C" fn worker_group(
        this: *mut std::ffi::c_void,
        group: *mut bindings::FiTasksWorkerGroup,
    ) -> std_bindings::FimoError {
        fimo_std::panic::catch_unwind(|| {
            // Safety: Must be ensured by the caller.
            let this = unsafe { Self::borrow_from_ffi(this) };
            if group.is_null() {
                return Err(Error::EINVAL);
            }

            // Safety: We checked that the caller provided a valid pointer.
            unsafe { group.write(WorkerGroupFFI(this.worker_group()?).into_ffi()) }
            Ok(())
        })
        .flatten()
        .map_or_else(|e| e.into_error(), |_| Error::EOK.into_error())
    }

    unsafe extern "C" fn wait_on(
        this: *mut std::ffi::c_void,
        aborted: *mut bool,
    ) -> std_bindings::FimoError {
        fimo_std::panic::catch_unwind(|| {
            // Safety: Is always in an `Arc`.
            let handle = unsafe { Self(Arc::from_raw(this.cast_const().cast())).0 };
            match handle.wait_on() {
                Ok(x) => {
                    if !aborted.is_null() {
                        // Safety: We checked that the caller provided a valid pointer.
                        unsafe { aborted.write(x) };
                    }
                    Ok(())
                }
                Err((e, handle)) => {
                    // Don't decrease the reference count.
                    #[allow(clippy::mem_forget)]
                    std::mem::forget(handle);
                    Err(e)
                }
            }
        })
        .flatten()
        .map_or_else(|e| e.into_error(), |_| Error::EOK.into_error())
    }
}

impl FFISharable<*mut std::ffi::c_void> for CommandBufferHandleFFI {
    type BorrowedView<'a> = &'a CommandBufferHandleImpl;

    fn share_to_ffi(&self) -> *mut std::ffi::c_void {
        Arc::as_ptr(&self.0).cast_mut().cast()
    }

    unsafe fn borrow_from_ffi<'a>(ffi: *mut std::ffi::c_void) -> Self::BorrowedView<'a> {
        // Safety: Is sound if `ffi` is the result of `Self::share_to_ffi`.
        unsafe { &*ffi.cast_const().cast() }
    }
}

impl FFITransferable<bindings::FiTasksCommandBufferHandle> for CommandBufferHandleFFI {
    fn into_ffi(self) -> bindings::FiTasksCommandBufferHandle {
        bindings::FiTasksCommandBufferHandle {
            data: Arc::into_raw(self.0).cast_mut().cast(),
            vtable: Self::VTABLE,
        }
    }

    unsafe fn from_ffi(ffi: bindings::FiTasksCommandBufferHandle) -> Self {
        // Safety: Is always in an `Arc`.
        unsafe { Self(Arc::from_raw(ffi.data.cast_const().cast())) }
    }
}

#[derive(Debug)]
pub struct CommandBufferImpl {
    num_enqueued_tasks: usize,
    handle: Arc<CommandBufferHandleImpl>,
    buffer: CommandBufferIterator,
    wait_reason: WaitReason,
    waiters: VecDeque<Waiter>,
    blocked_tasks: FxHashMap<TaskId, (usize, Option<WorkerId>, RawTask)>,
    worker: Option<WorkerId>,
    stack_size: Option<NonZeroUsize>,
}

impl CommandBufferImpl {
    /// # Safety
    ///
    /// - `buffer` must be dereferencable.
    unsafe fn new(
        group: &Arc<WorkerGroupImpl>,
        buffer: *mut bindings::FiTasksCommandBuffer,
    ) -> Self {
        let mut handle = Arc::new(CommandBufferHandleImpl {
            id: CommandBufferId(0),
            status: AtomicBool::new(false),
            completed: AtomicBool::new(false),
            group: Arc::downgrade(&group),
        });
        let id = CommandBufferId(Arc::as_ptr(&handle).addr());
        Arc::get_mut(&mut handle).unwrap().id = id;

        let this = Self {
            num_enqueued_tasks: 0,
            handle,
            buffer: CommandBufferIterator::new(RawCommandBuffer(buffer)),
            wait_reason: WaitReason::None,
            waiters: Default::default(),
            blocked_tasks: Default::default(),
            worker: None,
            stack_size: None,
        };

        this
    }

    pub fn handle(&self) -> &Arc<CommandBufferHandleImpl> {
        &self.handle
    }

    pub fn worker(&self) -> Option<WorkerId> {
        self.worker
    }

    pub fn stack_size(&self) -> Option<NonZeroUsize> {
        self.stack_size
    }

    pub fn mark_task_as_blocked(&mut self, index: usize, worker: Option<WorkerId>, task: RawTask) {
        let id = task.id();
        let old = self.blocked_tasks.insert(id, (index, worker, task));
        assert!(old.is_none(), "task marked as blocked multiple times");
    }

    pub fn mark_task_as_unblocked(
        &mut self,
        task_id: TaskId,
    ) -> (usize, Option<WorkerId>, RawTask) {
        self.blocked_tasks.remove(&task_id).expect("task not found")
    }

    pub fn mark_task_as_completed(
        &mut self,
        _module: &TasksModule<'_>,
        _index: usize,
        _task: RawTask,
    ) {
        self.num_enqueued_tasks -= 1;
        if self.num_enqueued_tasks == 0 && self.buffer.is_done() {
            // Safety: Is only called once.
            unsafe {
                self.buffer.mark_completed();
                self.handle.mark_completed(false);
            }
        }
    }

    pub fn mark_task_as_aborted(&mut self, module: &TasksModule<'_>, index: usize, _task: RawTask) {
        self.num_enqueued_tasks -= 1;
        self.abort(module, index);
    }

    pub fn register_waiter(&mut self, waiter: Waiter) {
        self.waiters.push_back(waiter);
    }

    pub fn take_waiters(&mut self) -> VecDeque<Waiter> {
        assert_eq!(self.num_enqueued_tasks, 0, "there are still running tasks");
        assert!(
            self.buffer.is_done(),
            "not all commands have been processed"
        );
        std::mem::take(&mut self.waiters)
    }

    pub fn process_commands(
        &mut self,
        module: &TasksModule<'_>,
        check_command_buffer: impl Fn(&CommandBufferHandleImpl) -> bool,
        check_worker: impl Fn(WorkerId) -> bool,
        check_stack_size: impl Fn(Option<NonZeroUsize>) -> bool,
    ) -> CommandBufferEventLoopCommand {
        // Skip if we are completed.
        if self.handle.completion_status().is_some() {
            return match self.num_enqueued_tasks {
                0 => CommandBufferEventLoopCommand::Completed,
                _ => CommandBufferEventLoopCommand::Processed,
            };
        }

        // Skip if we are still waiting.
        match &self.wait_reason {
            WaitReason::None => {}
            WaitReason::Barrier => {
                if self.num_enqueued_tasks != 0 {
                    return CommandBufferEventLoopCommand::Waiting;
                }
                self.wait_reason = WaitReason::None;
            }
            WaitReason::CommandBuffer {
                index,
                command_buffer,
            } => {
                match command_buffer.completion_status() {
                    // Wait for the command buffer.
                    None => {
                        return CommandBufferEventLoopCommand::Waiting;
                    }
                    // Propagate the abort to the current buffer.
                    Some(false) => {
                        let index = *index;
                        self.wait_reason = WaitReason::None;
                        return self.abort(module, index);
                    }
                    // The command buffer is done.
                    Some(true) => {
                        self.wait_reason = WaitReason::None;
                    }
                }
            }
        }

        // Process all commands we can.
        for (idx, command) in &mut self.buffer {
            match command {
                Command::SpawnTask(task) => {
                    self.num_enqueued_tasks += 1;
                    return CommandBufferEventLoopCommand::SpawnTask(idx, task);
                }
                Command::WaitBarrier => {
                    // Wait until all tasks have been completed.
                    if self.num_enqueued_tasks != 0 {
                        self.wait_reason = WaitReason::Barrier;
                        return CommandBufferEventLoopCommand::Waiting;
                    }
                }
                Command::WaitCommandBuffer(command_buffer) => {
                    if !check_command_buffer(&command_buffer) {
                        drop(command_buffer);
                        return self.abort(module, idx);
                    }
                    match command_buffer.completion_status() {
                        // Wait for the command buffer.
                        None => {
                            let id = command_buffer.id();
                            self.wait_reason = WaitReason::CommandBuffer {
                                index: idx,
                                command_buffer,
                            };
                            return CommandBufferEventLoopCommand::WaitCommandBuffer(id);
                        }
                        // Propagate the abort to the current buffer.
                        Some(false) => {
                            drop(command_buffer);
                            return self.abort(module, idx);
                        }
                        // The command buffer is done.
                        Some(true) => {}
                    }
                }
                Command::SetWorker(worker) => {
                    if !check_worker(worker) {
                        return self.abort(module, idx);
                    }
                    self.worker = Some(worker);
                }
                Command::EnableAllWorkers => {
                    self.worker = None;
                }
                Command::SetStackSize(stack_size) => {
                    if !check_stack_size(stack_size) {
                        return self.abort(module, idx);
                    }
                    self.stack_size = stack_size;
                }
                Command::Unknown => {
                    fimo_std::emit_error!(
                        module.context(),
                        "Unknown command at index {idx} for command buffer {:?}",
                        self.buffer.buffer.label()
                    );
                    return self.abort(module, idx);
                }
            }
        }

        if self.num_enqueued_tasks == 0 {
            // Safety: Is only called once.
            unsafe {
                self.buffer.mark_completed();
                self.handle.mark_completed(false);
            };
        }

        match self.num_enqueued_tasks {
            0 => CommandBufferEventLoopCommand::Completed,
            _ => CommandBufferEventLoopCommand::Processed,
        }
    }

    pub fn abort(
        &mut self,
        module: &TasksModule<'_>,
        cause: usize,
    ) -> CommandBufferEventLoopCommand {
        // Do nothing if it is already complete.
        if self.handle.completion_status().is_some() {
            return match self.num_enqueued_tasks {
                0 => CommandBufferEventLoopCommand::Completed,
                _ => CommandBufferEventLoopCommand::Processed,
            };
        }

        fimo_std::emit_error!(
            module.context(),
            "Aborting command buffer {:?} due to an error while processing command {cause}",
            self.buffer.buffer.label()
        );

        for (_, (_, _, mut task)) in self.blocked_tasks.drain() {
            self.num_enqueued_tasks -= 1;
            // Safety: The task is being aborted.
            unsafe {
                task.run_abortion_handler(std::ptr::null_mut());
                task.run_cleanup_handler();
            }
        }

        self.buffer.abort(cause);

        // Safety: Is only called once.
        unsafe { self.handle.mark_completed(true) };
        match self.num_enqueued_tasks {
            0 => CommandBufferEventLoopCommand::Completed,
            _ => CommandBufferEventLoopCommand::Processed,
        }
    }
}

impl Drop for CommandBufferImpl {
    fn drop(&mut self) {
        assert_eq!(self.num_enqueued_tasks, 0, "not all task are finished");
        assert!(
            self.blocked_tasks.is_empty(),
            "blocked tasks have not been cleaned up"
        );
        assert!(self.waiters.is_empty(), "waiters have not been woken up");
        assert!(
            self.buffer.is_done(),
            "not all commands have been processed"
        );
    }
}

#[derive(Debug)]
enum WaitReason {
    None,
    Barrier,
    CommandBuffer {
        index: usize,
        command_buffer: Arc<CommandBufferHandleImpl>,
    },
}

#[derive(Debug)]
pub enum CommandBufferEventLoopCommand {
    Waiting,
    Processed,
    Completed,
    SpawnTask(usize, RawTask),
    WaitCommandBuffer(CommandBufferId),
}

#[derive(Debug, Clone)]
pub enum Waiter {
    Task(TaskId),
    CommandBuffer(Arc<CommandBufferHandleImpl>),
}

#[derive(Debug)]
struct CommandBufferIterator {
    index: usize,
    num_commands: usize,
    buffer: RawCommandBuffer,
    state: CommandBufferState,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
enum CommandBufferState {
    Running,
    Aborted,
    Completed,
}

impl CommandBufferIterator {
    fn new(buffer: RawCommandBuffer) -> Self {
        let num_commands = buffer.buffer().num_entries;
        Self {
            index: 0,
            num_commands,
            buffer,
            state: CommandBufferState::Running,
        }
    }

    fn is_done(&self) -> bool {
        self.index == self.num_commands
    }

    fn abort(&mut self, cause: usize) {
        debug_assert_eq!(self.state, CommandBufferState::Running);
        debug_assert!(cause <= self.num_commands);
        for (_, command) in self.by_ref() {
            if let Command::SpawnTask(mut t) = command {
                // Safety:
                unsafe {
                    t.run_abortion_handler(std::ptr::null_mut());
                    t.run_cleanup_handler();
                }
            }
        }
        self.state = CommandBufferState::Aborted;

        // Safety: Is only called once.
        unsafe {
            self.buffer.run_abortion_handler(cause);
        }
    }

    /// # Safety
    ///
    /// All commands must have finished executing.
    unsafe fn mark_completed(&mut self) {
        debug_assert_eq!(self.state, CommandBufferState::Running);
        debug_assert_eq!(self.index, self.num_commands);
        self.state = CommandBufferState::Completed;

        // Safety: Is only called once.
        unsafe {
            self.buffer.run_completion_handler();
        }
    }
}

impl Iterator for CommandBufferIterator {
    type Item = (usize, Command);

    fn next(&mut self) -> Option<Self::Item> {
        if self.index == self.num_commands {
            return None;
        }
        let current = self.index;
        self.index += 1;

        // Safety: The pointer is a valid slice.
        let command = unsafe { &*self.buffer.buffer().entries.add(current) };
        let command = match command.type_ {
            FiTasksCommandBufferEntryType::FI_TASKS_COMMAND_BUFFER_ENTRY_TYPE_SPAWN_TASK => {
                // Safety: We checked the tag of the union.
                // Since we only iterate each buffer once, the task is also unaliased.
                let task = unsafe {
                    RawTask::new(*command.data.spawn_task)
                };
                Command::SpawnTask(task)
            }
            FiTasksCommandBufferEntryType::FI_TASKS_COMMAND_BUFFER_ENTRY_TYPE_WAIT_BARRIER => {
                Command::WaitBarrier
            }
            FiTasksCommandBufferEntryType::FI_TASKS_COMMAND_BUFFER_ENTRY_TYPE_WAIT_COMMAND_BUFFER => {
                // Safety: We checked the tag of the union.
                let command_buffer = unsafe {
                    Arc::from_raw(command.data.wait_command_buffer.data.cast_const().cast::<CommandBufferHandleImpl>())
                };
                Command::WaitCommandBuffer(command_buffer)
            }
            FiTasksCommandBufferEntryType::FI_TASKS_COMMAND_BUFFER_ENTRY_TYPE_SET_WORKER => {
                // Safety: We checked the tag of the union.
                let worker = unsafe {
                    WorkerId(*command.data.set_worker)
                };
                Command::SetWorker(worker)
            }
            FiTasksCommandBufferEntryType::FI_TASKS_COMMAND_BUFFER_ENTRY_TYPE_ENABLE_ALL_WORKERS => {
                Command::EnableAllWorkers
            }
            FiTasksCommandBufferEntryType::FI_TASKS_COMMAND_BUFFER_ENTRY_TYPE_SET_STACK_SIZE => {
                // Safety: We checked the tag of the union.
                let stack_size = unsafe {
                    *command.data.set_stack_size
                };
                Command::SetStackSize(NonZeroUsize::new(stack_size))
            }
            _ => Command::Unknown,
        };

        Some((current, command))
    }
}

impl Drop for CommandBufferIterator {
    fn drop(&mut self) {
        assert_eq!(
            self.index, self.num_commands,
            "not all commands have been processed"
        );
        assert_ne!(
            self.state,
            CommandBufferState::Running,
            "not all commands have finished executing"
        );
    }
}

enum Command {
    SpawnTask(RawTask),
    WaitBarrier,
    WaitCommandBuffer(Arc<CommandBufferHandleImpl>),
    SetWorker(WorkerId),
    EnableAllWorkers,
    SetStackSize(Option<NonZeroUsize>),
    Unknown,
}

#[derive(Debug)]
struct RawCommandBuffer(*mut bindings::FiTasksCommandBuffer);

impl RawCommandBuffer {
    fn buffer(&self) -> &bindings::FiTasksCommandBuffer {
        // Safety: A `RawCommandBuffer` works like a `Box`. We own the buffer.
        unsafe { &*self.0 }
    }

    fn buffer_mut(&mut self) -> &mut bindings::FiTasksCommandBuffer {
        // Safety: A `RawCommandBuffer` works like a `Box`. We own the buffer.
        unsafe { &mut *self.0 }
    }

    pub fn label(&self) -> &CStr {
        let buffer = self.buffer();
        if buffer.label.is_null() {
            c"unlabeled"
        } else {
            // Safety: The string is guaranteed to be valid.
            unsafe { CStr::from_ptr(buffer.label) }
        }
    }

    /// # Safety
    ///
    /// May only be called once when the execution of the command buffer was successful.
    unsafe fn run_completion_handler(&mut self) {
        let buffer = self.buffer_mut();

        if let Some(on_complete) = buffer.on_complete {
            // Safety: The caller ensures that this is sound.
            unsafe { on_complete(buffer.status_callback_data, buffer) };
        }
    }

    /// # Safety
    ///
    /// May only be called once when the execution of the command buffer was aborted.
    unsafe fn run_abortion_handler(&mut self, index: usize) {
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
