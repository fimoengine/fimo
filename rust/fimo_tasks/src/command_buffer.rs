use crate::{
    bindings,
    task::{RawTask, TaskHandleInner},
    Context, TaskHandle, TaskStatus, WorkerGroup, WorkerId,
};
use fimo_std::{
    allocator::FimoAllocator,
    error::{to_result_indirect_in_place, Error},
    ffi::FFITransferable,
};
use std::{
    alloc::Allocator,
    cell::UnsafeCell,
    ffi::CString,
    marker::PhantomData,
    mem::{ManuallyDrop, MaybeUninit},
    num::NonZeroUsize,
    sync::{
        atomic::{AtomicBool, AtomicUsize, Ordering},
        Arc, Condvar, Mutex,
    },
};

/// A list of commands to be executed by a [`WorkerGroup`].
#[derive(Debug)]
pub struct CommandBuffer<'ctx, A: Allocator = FimoAllocator> {
    inner: RawCommandBuffer<'static, 'ctx, A>,
}

impl CommandBuffer<'_> {
    /// Builds a new empty command buffer.
    pub fn new() -> Self {
        Self::new_in(FimoAllocator)
    }

    /// Create a scope for enqueuing scoped command buffers.
    ///
    /// See [`CommandBuffer::scope_in`] for additional info.
    pub fn scope<'env, F, T>(group: &WorkerGroup<'env>, f: F) -> T
    where
        F: for<'scope> FnOnce(&'scope Scope<'scope, 'env>) -> T,
    {
        Self::scope_in(group, f, FimoAllocator)
    }
}

impl<'ctx, A> CommandBuffer<'ctx, A>
where
    A: Allocator + Clone + Send + 'static,
{
    /// Builds a new empty command buffer with a custom allocator.
    pub fn new_in(alloc: A) -> Self {
        Self {
            inner: RawCommandBuffer::new_in(None, alloc),
        }
    }

    /// Create a scope for enqueuing scoped command buffers.
    ///
    /// The function passed to `scope_in` will be provided a [`Scope`] object, through which scoped
    /// command buffers can be enqueue.
    ///
    /// Unlike non-scoped command buffers, scoped command buffers can borrow non-`'static` data, as
    /// the scope guaranteed all command buffers will be joined at the end of the scope.
    ///
    /// All command buffers enqueued within the scope that haven't been manually joined will be
    /// automatically joined before this function returns.
    ///
    /// # Panics
    ///
    /// If any of the automatically joined command buffers is aborted, this function will panic.
    ///
    /// If you want to handle panics from enqueued command buffers,
    /// [`join`](CommandBufferHandle::join) them before the end of the scope.
    ///
    /// # Aborts
    ///
    /// If it is not possible to automatically join the command buffers at the end of the scope,
    /// this function will abort the process, ensuring that no undefined behavior can occur.
    ///
    /// # Lifetimes
    ///
    /// Scoped command buffers involve two lifetimes: `'scope` and `'env`.
    ///
    /// The `'scope` lifetime represents the lifetime of the scope itself. That is: the time during
    /// which new scoped command buffers may be enqueued, and also the time during which they might
    /// still be running. Once this lifetime ends, all scoped command buffers are joined. This
    /// lifetime starts within the `scope_in` function, before `f` (the argument to `scope_in`)
    /// starts. It ends after `f` returns and all scoped command buffers have been joined, buf
    /// before `scope_in` returns.
    ///
    /// The `'env` lifetime represents the lifetime of whatever is borrowed by the scoped command
    /// buffers. This lifetime must outlast the call to `scope_in`, and thus cannot be smaller than
    /// `'scope`. It can be as small as the call to `scope_in`, meaning that anything that outlives
    /// this call, such as local variables defined right before the scope, can be borrowed by the
    /// scoped command buffers.
    ///
    /// The `'env: 'scope` bound is part of the definition of the `Scope` type.
    pub fn scope_in<'env, F, T>(group: &WorkerGroup<'env>, f: F, alloc: A) -> T
    where
        F: for<'scope> FnOnce(&'scope Scope<'scope, 'env, A>) -> T,
    {
        let mut command_buffer = CommandBuffer::new_in(alloc);
        let scope = Scope {
            command_buffer: &mut command_buffer,
            worker_group: group,
            scope: PhantomData,
            env: PhantomData,
        };

        // Run `f`, but catch panics so we can make sure to wait for all the threads to join.
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| f(&scope)));

        // Block until all tasks could be completed.
        let has_aborted = match std::panic::catch_unwind(std::panic::AssertUnwindSafe(move || {
            if command_buffer.is_empty() {
                drop(command_buffer);
                false
            } else {
                match command_buffer.block_on(group) {
                    Ok(CommandBufferStatus::Completed) => false,
                    Ok(CommandBufferStatus::Aborted(_)) => true,
                    Err(e) => {
                        panic!("could not join the scoped command buffers, error=`{e:?}`")
                    }
                }
            }
        })) {
            Ok(x) => x,
            Err(_) => {
                // There was an error when waiting for the completion of the tasks.
                // At this time some tasks may still be running. To ensure that no
                // undefined behavior occurs we abort the process.
                std::process::abort();
            }
        };

        // Throw any panic from `f`, or the return value of `f` if no thread panicked.
        match result {
            Err(e) => std::panic::resume_unwind(e),
            Ok(_) if has_aborted => panic!("a scoped command buffer was aborted"),
            Ok(result) => result,
        }
    }

    /// Returns a reference to the underlying allocator.
    pub fn allocator(&self) -> &A {
        self.inner.allocator()
    }

    /// Returns whether the command buffer is empty.
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    /// Returns the number of commands contained in the command buffer.
    pub fn len(&self) -> usize {
        self.inner.len()
    }

    /// Reserves capacity for at least `additional` more commands to be inserted in the given
    /// `CommandBuffer`. The `CommandBuffer` may reserve more space to speculatively avoid frequent
    /// reallocations. After calling `reserve`, capacity will be greater than or equal to
    /// `self.len() + additional`. Does nothing if capacity is already sufficient.
    ///
    /// # Panics
    ///
    /// Panics if the new capacity exceeds `isize::MAX` _bytes_.
    pub fn reserve(&mut self, additional: usize) {
        self.inner.reserve(additional);
    }

    /// Spawns a new task, returning a [`TaskHandle`] to it.
    ///
    /// The handle provides some methods to query the completion status of the task and extracting
    /// its result.
    pub fn spawn_task<T: Send + 'static>(
        &mut self,
        f: impl FnOnce(&Context) -> T + Send + 'static,
    ) -> TaskHandle<T, A> {
        // Safety: Is safe, as `f` is `Send`.
        unsafe { self.inner.spawn_task(f) }
    }

    /// Inserts a barrier to synchronize the execution of the commands in the buffer.
    ///
    /// A barrier ensures that all previous commands have been completed before the following
    /// commands are started to be executed.
    pub fn wait_barrier(&mut self) {
        self.inner.wait_barrier();
    }

    /// Inserts a dependency to another command buffer.
    ///
    /// This call ensures that the other command buffer is completed before the following commands
    /// are started to be executed.
    ///
    /// # Note
    ///
    /// The command buffer must have been enqueued on the same [`WorkerGroup`] as the current
    /// command buffer. Synchronization between multiple [`WorkerGroup`]s currently requires the
    /// use of external synchronization mechanisms.
    pub fn wait_command_buffer<T: Allocator>(&mut self, handle: CommandBufferHandle<'ctx, T>) {
        self.inner.wait_command_buffer(handle.handle);
    }

    /// Specifies the single worker that is allowed to execute the following commands.
    pub fn set_worker(&mut self, worker: WorkerId) {
        self.inner.set_worker(worker);
    }

    /// Allows all workers of the [`WorkerGroup`] to execute the following commands.
    pub fn enable_all_workers(&mut self) {
        self.inner.enable_all_workers();
    }

    /// Specifies the minimum stack size for the following commands.
    pub fn set_stack_size(&mut self, size: Option<NonZeroUsize>) {
        self.inner.set_stack_size(size);
    }

    /// Enqueues the command buffer into the [`WorkerGroup`].
    ///
    /// Enqueues the command buffer and returns a handle, which may be used to query the completion
    /// status of the buffer and various other operations.
    ///
    /// # Completion
    ///
    /// The caller must provide a closure that will be called upon completion, whether successful or
    /// not, of the command buffer. A panic in the completion procedure may cause the abortion of
    /// the process.
    pub fn enqueue(
        self,
        group: &WorkerGroup<'ctx>,
        on_complete: impl FnOnce(CommandBufferStatus) + Send + 'static,
    ) -> Result<CommandBufferHandle<'ctx, A>, Error> {
        self.inner.enqueue(group, on_complete)
    }

    /// Enqueues the command buffer into the [`WorkerGroup`].
    ///
    /// Unlike [`CommandBuffer::enqueue`], this function does not return a handle.
    ///
    /// # Completion
    ///
    /// The caller must provide a closure that will be called upon completion, whether successful or
    /// not, of the command buffer. A panic in the completion procedure may cause the abortion of
    /// the process.
    pub fn enqueue_detached(
        self,
        group: &WorkerGroup<'_>,
        on_complete: impl FnOnce(CommandBufferStatus) + Send + 'static,
    ) -> Result<(), Error> {
        self.inner.enqueue_detached(group, on_complete)
    }

    /// Enqueues the command buffer into the [`WorkerGroup`] and waits until it is completed.
    ///
    /// Upon completion, the current thread is resumed, and the status of the finished command
    /// buffer is returned to the caller.
    ///
    /// # Thread blocking
    ///
    /// This method can serve as an entry point onto a [`WorkerGroup`] from foreign threads, e.g.
    /// threads not managed by the [`WorkerGroup`]. In those cases the thread will be suspended
    /// entirely and may therefore not be the most efficient synchronization method, in case the
    /// thread is managed by another [`WorkerGroup`].
    ///
    /// If the current thread is managed by the same [`WorkerGroup`], then this method will only
    /// block the current task instead of the entire thread.
    pub fn block_on(self, group: &WorkerGroup<'_>) -> Result<CommandBufferStatus, Error> {
        self.inner.block_on(group)
    }
}

impl<A> Default for CommandBuffer<'_, A>
where
    A: Allocator + Clone + Send + Default + 'static,
{
    fn default() -> Self {
        CommandBuffer::new_in(Default::default())
    }
}

/// A list of commands to be executed by a [`WorkerGroup`].
#[derive(Debug)]
pub struct ScopedCommandBuffer<'scope, 'env, A: Allocator = FimoAllocator> {
    scope: &'scope Scope<'scope, 'env, A>,
    inner: RawCommandBuffer<'scope, 'env, A>,
}

impl<'scope, 'env, A> ScopedCommandBuffer<'scope, 'env, A>
where
    A: Allocator + Clone + Send + 'static,
{
    /// Builds a new empty command buffer with a custom allocator.
    pub fn new(scope: &'scope Scope<'scope, 'env, A>) -> Self {
        let alloc = scope.allocator().clone();
        Self {
            scope,
            inner: RawCommandBuffer::new_in(None, alloc),
        }
    }

    /// Returns a reference to the underlying allocator.
    pub fn allocator(&self) -> &A {
        self.inner.allocator()
    }

    /// Returns whether the `ScopedCommandBuffer` is empty.
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    /// Returns the number of commands contained in the command buffer.
    pub fn len(&self) -> usize {
        self.inner.len()
    }

    /// Reserves capacity for at least `additional` more commands to be inserted in the given
    /// `ScopedCommandBuffer`. The `ScopedCommandBuffer` may reserve more space to speculatively
    /// avoid frequent reallocations. After calling `reserve`, capacity will be greater than or
    /// equal to `self.len() + additional`. Does nothing if capacity is already sufficient.
    ///
    /// # Panics
    ///
    /// Panics if the new capacity exceeds `isize::MAX` _bytes_.
    pub fn reserve(&mut self, additional: usize) {
        self.inner.reserve(additional);
    }

    /// Spawns a new task, returning a [`TaskHandle`] to it.
    ///
    /// The handle provides some methods to query the completion status of the task and extracting
    /// its result.
    pub fn spawn_task<T: Send + 'scope>(
        &mut self,
        f: impl FnOnce(&Context) -> T + Send + 'scope,
    ) -> TaskHandle<T, A> {
        // Safety: Is safe, as `f` is `Send`.
        unsafe { self.inner.spawn_task(f) }
    }

    /// Inserts a barrier to synchronize the execution of the commands in the buffer.
    ///
    /// A barrier ensures that all previous commands have been completed before the following
    /// commands are started to be executed.
    pub fn wait_barrier(&mut self) {
        self.inner.wait_barrier();
    }

    /// Inserts a dependency to another command buffer.
    ///
    /// This call ensures that the other command buffer is completed before the following commands
    /// are started to be executed.
    ///
    /// # Note
    ///
    /// The command buffer must have been enqueued on the same [`WorkerGroup`] as the current
    /// command buffer. Synchronization between multiple [`WorkerGroup`]s currently requires the
    /// use of external synchronization mechanisms.
    pub fn wait_command_buffer<T: Allocator>(&mut self, handle: CommandBufferHandle<'env, T>) {
        self.inner.wait_command_buffer(handle.handle);
    }

    /// Specifies the single worker that is allowed to execute the following commands.
    pub fn set_worker(&mut self, worker: WorkerId) {
        self.inner.set_worker(worker);
    }

    /// Allows all workers of the [`WorkerGroup`] to execute the following commands.
    pub fn enable_all_workers(&mut self) {
        self.inner.enable_all_workers();
    }

    /// Specifies the minimum stack size for the following commands.
    pub fn set_stack_size(&mut self, size: Option<NonZeroUsize>) {
        self.inner.set_stack_size(size);
    }

    /// Enqueues the command buffer into the [`WorkerGroup`].
    ///
    /// Enqueues the command buffer and returns a handle, which may be used to query the completion
    /// status of the buffer and various other operations.
    ///
    /// # Completion
    ///
    /// The caller must provide a closure that will be called upon completion, whether successful or
    /// not, of the command buffer. A panic in the completion procedure may cause the abortion of
    /// the process.
    pub fn enqueue(
        self,
        on_complete: impl FnOnce(CommandBufferStatus) + Send + 'scope,
    ) -> Result<CommandBufferHandle<'env, A>, Error> {
        // Reserve additional space beforehand. Doing so ensures that no panic can
        // occur after enqueuing the buffer.
        // Safety: There can only be one reference at any time.
        unsafe {
            let buffer = &mut *self.scope.command_buffer;
            buffer.reserve(1);
        }

        let group = self.scope.worker_group;
        let handle = self.inner.enqueue(group, on_complete)?;

        // We reserved the additional memory, therefore this operation can not panic.
        // Safety: There can only be one reference at any time.
        unsafe {
            let buffer = &mut *self.scope.command_buffer;
            buffer.wait_command_buffer(handle.clone());
        }

        Ok(handle)
    }

    /// Enqueues the command buffer into the [`WorkerGroup`].
    ///
    /// Unlike [`ScopedCommandBuffer::enqueue`], this function does not return a handle.
    ///
    /// # Completion
    ///
    /// The caller must provide a closure that will be called upon completion, whether successful or
    /// not, of the command buffer. A panic in the completion procedure may cause the abortion of
    /// the process.
    pub fn enqueue_detached(
        self,
        on_complete: impl FnOnce(CommandBufferStatus) + Send + 'scope,
    ) -> Result<(), Error> {
        // We can not use `enqueue_detached` since it would not allow us to register
        // the handle to the inner command buffer. Instead, we let `enqueue` register
        // the handle for us and drop it after.
        let _ = self.enqueue(on_complete)?;
        Ok(())
    }

    /// Enqueues the command buffer into the [`WorkerGroup`] and waits until it is completed.
    ///
    /// Upon completion, the current thread is resumed, and the status of the finished command
    /// buffer is returned to the caller.
    ///
    /// # Thread blocking
    ///
    /// This method can serve as an entry point onto a [`WorkerGroup`] from foreign threads, e.g.
    /// threads not managed by the [`WorkerGroup`]. In those cases the thread will be suspended
    /// entirely and may therefore not be the most efficient synchronization method, in case the
    /// thread is managed by another [`WorkerGroup`].
    ///
    /// If the current thread is managed by the same [`WorkerGroup`], then this method will only
    /// block the current task instead of the entire thread.
    pub fn block_on(self) -> Result<CommandBufferStatus, Error> {
        let group = self.scope.worker_group;
        self.inner.block_on(group)
    }
}

/// A scope to enqueue scoped command buffers in.
///
/// See [`CommandBuffer::scope_in`] for details.
#[derive(Debug)]
pub struct Scope<'scope, 'env, A: Allocator = FimoAllocator> {
    command_buffer: *mut CommandBuffer<'scope, A>,
    // Implicitly requires 'env: 'scope.
    worker_group: &'scope WorkerGroup<'env>,
    // See `Scope` in the standard library why invariance is required.
    scope: PhantomData<&'scope mut &'scope ()>,
    env: PhantomData<&'env mut &'env ()>,
}

impl<A> Scope<'_, '_, A>
where
    A: Allocator + Clone + Send + 'static,
{
    /// Returns a reference to the underlying allocator.
    pub fn allocator(&self) -> &A {
        // Safety:
        unsafe { (*self.command_buffer).allocator() }
    }
}

#[derive(Debug)]
enum Command<'scope, 'ctx, A: Allocator> {
    Task(Box<RawTask<'scope, A>, A>),
    Barrier,
    Handle(CommandBufferHandleInner<'ctx>),
    SetWorker(WorkerId),
    EnableAllWorkers,
    SetStackSize(usize),
}

/// Completion status of a [`CommandBuffer`] or [`ScopedCommandBuffer`].
#[derive(Debug, Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Hash)]
pub enum CommandBufferStatus {
    /// The command buffer has been completed successfully.
    Completed,
    /// The command buffer was aborted.
    ///
    /// Contains the index of the command that caused the abort.
    Aborted(usize),
}

#[derive(Debug)]
struct RawCommandBuffer<'scope, 'ctx, A: Allocator = FimoAllocator> {
    label: Option<CString>,
    commands: Vec<Command<'scope, 'ctx, A>, A>,
}

impl<'scope, 'ctx, A> RawCommandBuffer<'scope, 'ctx, A>
where
    A: Allocator + Clone + Send + 'scope,
{
    fn new_in(label: Option<CString>, alloc: A) -> Self {
        Self {
            label,
            commands: Vec::new_in(alloc),
        }
    }

    fn allocator(&self) -> &A {
        self.commands.allocator()
    }

    fn is_empty(&self) -> bool {
        self.commands.is_empty()
    }

    fn len(&self) -> usize {
        self.commands.len()
    }

    fn reserve(&mut self, additional: usize) {
        self.commands.reserve(additional);
    }

    unsafe fn spawn_task<T: Send + 'scope>(
        &mut self,
        f: impl FnOnce(&Context) -> T + Send + 'scope,
    ) -> TaskHandle<T, A> {
        let alloc = self.commands.allocator().clone();
        let handle = Arc::new_in(
            TaskHandleInner {
                completed: AtomicBool::new(false),
                value: UnsafeCell::new(MaybeUninit::uninit()),
            },
            alloc.clone(),
        );

        let f = {
            let handle = handle.clone();
            move |context: &Context| {
                let result =
                    std::panic::catch_unwind(std::panic::AssertUnwindSafe(move || f(context)));
                let success = result.is_ok();

                // Safety: We are the only ones that have access to the value at this point in time.
                unsafe {
                    let value_ref = &mut *handle.value.get();
                    value_ref.write(result);
                }

                if success {
                    Ok(())
                } else {
                    Err(())
                }
            }
        };

        let s = {
            let handle = handle.clone();
            move |_status: TaskStatus| {
                handle.completed.store(true, Ordering::Release);
            }
        };

        let task = RawTask::new_in(None, f, s, alloc);
        self.commands.push(Command::Task(task));

        TaskHandle { inner: handle }
    }

    fn wait_barrier(&mut self) {
        self.commands.push(Command::Barrier);
    }

    fn wait_command_buffer(&mut self, handle: CommandBufferHandleInner<'ctx>) {
        self.commands.push(Command::Handle(handle));
    }

    fn set_worker(&mut self, worker: WorkerId) {
        self.commands.push(Command::SetWorker(worker));
    }

    fn enable_all_workers(&mut self) {
        self.commands.push(Command::EnableAllWorkers);
    }

    fn set_stack_size(&mut self, size: Option<NonZeroUsize>) {
        self.commands
            .push(Command::SetStackSize(size.map_or(0, |x| x.get())));
    }

    fn into_raw_command_buffer<F>(
        self,
        f: F,
    ) -> impl FFITransferable<*mut bindings::FiTasksCommandBuffer>
    where
        F: FnOnce(CommandBufferStatus) + Send,
    {
        #[repr(C)]
        struct FfiBuffer<A, F>
        where
            F: FnOnce(CommandBufferStatus) + Send,
            A: Allocator + Clone,
        {
            buffer: bindings::FiTasksCommandBuffer,
            label: Option<CString>,
            entries: Option<Box<[bindings::FiTasksCommandBufferEntry], A>>,
            on_finish: Option<F>,
        }

        unsafe extern "C" fn on_complete<A, F>(
            _data: *mut std::ffi::c_void,
            buffer: *mut bindings::FiTasksCommandBuffer,
        ) where
            F: FnOnce(CommandBufferStatus) + Send,
            A: Allocator + Clone,
        {
            fimo_std::panic::abort_on_panic(|| {
                // Safety: We are the only ones with a reference to the buffer.
                let buffer = unsafe { &mut *buffer.cast::<FfiBuffer<A, F>>() };
                let f = buffer.on_finish.take().unwrap();
                f(CommandBufferStatus::Completed);
            });
        }

        unsafe extern "C" fn on_abort<A, F>(
            _data: *mut std::ffi::c_void,
            buffer: *mut bindings::FiTasksCommandBuffer,
            entry: usize,
        ) where
            F: FnOnce(CommandBufferStatus) + Send,
            A: Allocator + Clone,
        {
            fimo_std::panic::abort_on_panic(|| {
                // Safety: We are the only ones with a reference to the buffer.
                let buffer = unsafe { &mut *buffer.cast::<FfiBuffer<A, F>>() };
                let f = buffer.on_finish.take().unwrap();
                f(CommandBufferStatus::Aborted(entry));
            });
        }

        unsafe extern "C" fn on_cleanup<A, F>(
            _data: *mut std::ffi::c_void,
            buffer: *mut bindings::FiTasksCommandBuffer,
        ) where
            F: FnOnce(CommandBufferStatus) + Send,
            A: Allocator + Clone,
        {
            fimo_std::panic::abort_on_panic(|| {
                // Safety: We know that the type must match and that we own the data.
                let mut buffer = unsafe { FfiBufferBox::<A, F>::from_ffi(buffer) };

                // Remove the entries so that the destructor does not try to drop them again.
                drop(buffer.0.entries.take());
                drop(buffer);
            });
        }

        impl<A, F> Drop for FfiBuffer<A, F>
        where
            F: FnOnce(CommandBufferStatus) + Send,
            A: Allocator + Clone,
        {
            fn drop(&mut self) {
                fimo_std::panic::abort_on_panic(|| {
                    // Drop the array if it has not been processed yet.
                    if let Some(entries) = self.entries.take() {
                        for entry in entries.iter() {
                            match entry.type_ {
                                bindings::FiTasksCommandBufferEntryType::FI_TASKS_COMMAND_BUFFER_ENTRY_TYPE_SPAWN_TASK => {
                                    // Safety:
                                    unsafe {
                                        let task = *entry.data.spawn_task;
                                        if let Some(on_cleanup) = (*task).on_cleanup {
                                            on_cleanup((*task).cleanup_data, task);
                                        }
                                    }
                                }
                                bindings::FiTasksCommandBufferEntryType::FI_TASKS_COMMAND_BUFFER_ENTRY_TYPE_WAIT_COMMAND_BUFFER => {
                                    // Safety:
                                    unsafe {
                                        let handle = *entry.data.wait_command_buffer;
                                        if let Some(release)  = (*handle.vtable).v0.release {
                                            release(handle.data);
                                        }
                                    }
                                }
                                _ => {}
                            }
                        }
                    }
                    drop(self.on_finish.take());
                });
            }
        }

        #[repr(transparent)]
        struct FfiBufferBox<A, F>(Box<FfiBuffer<A, F>, A>)
        where
            F: FnOnce(CommandBufferStatus) + Send,
            A: Allocator + Clone;

        impl<A, F> FFITransferable<*mut bindings::FiTasksCommandBuffer> for FfiBufferBox<A, F>
        where
            F: FnOnce(CommandBufferStatus) + Send,
            A: Allocator + Clone,
        {
            fn into_ffi(self) -> *mut bindings::FiTasksCommandBuffer {
                let raw = Box::into_raw(self.0);
                raw.cast()
            }

            unsafe fn from_ffi(ffi: *mut bindings::FiTasksCommandBuffer) -> Self {
                let raw = ffi.cast::<FfiBuffer<A, F>>();

                // Safety: We know that the types match and that the value must still be alive.
                unsafe {
                    let alloc = Box::allocator((*raw).entries.as_ref().unwrap_unchecked()).clone();
                    Self(Box::from_raw_in(raw, alloc))
                }
            }
        }

        let alloc = self.allocator().clone();
        let mut entries = Box::new_uninit_slice_in(self.commands.len(), alloc.clone());

        // Safety: We can safely destructure `self`
        let (label, commands) = unsafe {
            let this = ManuallyDrop::new(self);
            let label = std::ptr::from_ref(&this.label).read();
            let commands = std::ptr::from_ref(&this.commands).read();
            (label, commands)
        };

        // Can not panic since we pre-allocated all entries
        for (entry, command) in entries.iter_mut().zip(commands) {
            match command {
                Command::Task(task) => entry.write(bindings::FiTasksCommandBufferEntry {
                    type_: bindings::FiTasksCommandBufferEntryType::FI_TASKS_COMMAND_BUFFER_ENTRY_TYPE_SPAWN_TASK,
                    data: bindings::FiTasksCommandBufferEntryData { spawn_task: ManuallyDrop::new(Box::into_raw(task).cast())},
                }),
                Command::Barrier => entry.write(bindings::FiTasksCommandBufferEntry {
                    type_: bindings::FiTasksCommandBufferEntryType::FI_TASKS_COMMAND_BUFFER_ENTRY_TYPE_WAIT_BARRIER,
                    data: bindings::FiTasksCommandBufferEntryData { wait_barrier: ManuallyDrop::new(0) },
                }),
                Command::Handle(handle) => entry.write(bindings::FiTasksCommandBufferEntry {
                    type_: bindings::FiTasksCommandBufferEntryType::FI_TASKS_COMMAND_BUFFER_ENTRY_TYPE_WAIT_COMMAND_BUFFER,
                    data: bindings::FiTasksCommandBufferEntryData { wait_command_buffer: ManuallyDrop::new(handle.into_raw_handle()) },
                }),
                Command::SetWorker(worker) => entry.write(bindings::FiTasksCommandBufferEntry {
                    type_: bindings::FiTasksCommandBufferEntryType::FI_TASKS_COMMAND_BUFFER_ENTRY_TYPE_SET_WORKER,
                    data: bindings::FiTasksCommandBufferEntryData {set_worker: ManuallyDrop::new(worker.0)},
                }),
                Command::EnableAllWorkers => entry.write(bindings::FiTasksCommandBufferEntry {
                    type_: bindings::FiTasksCommandBufferEntryType::FI_TASKS_COMMAND_BUFFER_ENTRY_TYPE_ENABLE_ALL_WORKERS,
                    data: bindings::FiTasksCommandBufferEntryData {enable_all_workers: ManuallyDrop::new(0)},
                }),
                Command::SetStackSize(size) => entry.write(bindings::FiTasksCommandBufferEntry {
                    type_: bindings::FiTasksCommandBufferEntryType::FI_TASKS_COMMAND_BUFFER_ENTRY_TYPE_SET_STACK_SIZE,
                    data: bindings::FiTasksCommandBufferEntryData {set_stack_size: ManuallyDrop::new(size)},
                }),
            };
        }

        // Safety: We initialized all entries.
        let mut entries = unsafe { entries.assume_init() };
        FfiBufferBox(Box::new_in(
            FfiBuffer {
                buffer: bindings::FiTasksCommandBuffer {
                    label: label.as_ref().map_or(std::ptr::null(), |x| x.as_ptr()),
                    entries: entries.as_mut_ptr(),
                    num_entries: entries.len(),
                    on_complete: Some(on_complete::<A, F>),
                    on_abort: Some(on_abort::<A, F>),
                    status_callback_data: std::ptr::null_mut(),
                    on_cleanup: Some(on_cleanup::<A, F>),
                    cleanup_data: std::ptr::null_mut(),
                },
                label,
                entries: Some(entries),
                on_finish: Some(f),
            },
            alloc,
        ))
    }

    pub fn enqueue(
        self,
        group: &WorkerGroup<'ctx>,
        on_complete: impl FnOnce(CommandBufferStatus) + Send,
    ) -> Result<CommandBufferHandle<'ctx, A>, Error> {
        let status = Arc::new_in(AtomicUsize::new(0), self.allocator().clone());
        let raw = self.into_raw_command_buffer({
            let status = status.clone();
            move |x| {
                match x {
                    CommandBufferStatus::Completed => status.store(1, Ordering::Release),
                    CommandBufferStatus::Aborted(idx) => status.store(idx + 2, Ordering::Release),
                };
                on_complete(x);
            }
        });

        // Safety: FFI call is safe.
        let handle = unsafe {
            to_result_indirect_in_place(|error, handle| {
                *error = (group.vtable().v0.enqueue_buffer.unwrap_unchecked())(
                    group.data(),
                    raw.into_ffi(),
                    false,
                    handle.as_mut_ptr(),
                );
            })?
        };

        Ok(CommandBufferHandle {
            handle: CommandBufferHandleInner {
                handle,
                _phantom: PhantomData,
            },
            status,
        })
    }

    pub fn enqueue_detached(
        self,
        group: &WorkerGroup<'ctx>,
        on_complete: impl FnOnce(CommandBufferStatus) + Send,
    ) -> Result<(), Error> {
        let raw = self.into_raw_command_buffer(on_complete);

        // Safety: FFI call is safe.
        let handle = unsafe {
            to_result_indirect_in_place(|error, handle| {
                *error = (group.vtable().v0.enqueue_buffer.unwrap_unchecked())(
                    group.data(),
                    raw.into_ffi(),
                    true,
                    handle.as_mut_ptr(),
                );
            })?
        };

        debug_assert!(handle.data.is_null());
        Ok(())
    }

    fn block_on(self, group: &WorkerGroup<'ctx>) -> Result<CommandBufferStatus, Error> {
        if group.is_worker() {
            let handle = self.enqueue(group, |_| {})?;
            handle.join().map_err(|err| err.into_error())
        } else {
            let sync_arc =
                Arc::new_in((Mutex::new(None), Condvar::new()), self.allocator().clone());

            self.enqueue_detached(group, {
                let sync_arc = sync_arc.clone();
                move |status| {
                    let (lock, cvar) = &*sync_arc;
                    let mut stat = lock.lock().unwrap();
                    *stat = Some(status);
                    cvar.notify_one();
                }
            })?;

            let (lock, cvar) = &*sync_arc;
            let guard = cvar
                .wait_while(lock.lock().unwrap(), |status| status.is_none())
                .unwrap();

            Ok(guard.unwrap())
        }
    }
}

/// A handle to an enqueued [`CommandBuffer`] or [`ScopedCommandBuffer`].
#[derive(Debug, Clone)]
pub struct CommandBufferHandle<'ctx, A: Allocator> {
    handle: CommandBufferHandleInner<'ctx>,
    status: Arc<AtomicUsize, A>,
}

impl<'ctx, A: Allocator> CommandBufferHandle<'ctx, A> {
    /// Returns whether the command buffer has been completed.
    pub fn is_completed(&self) -> bool {
        self.completion_status().is_some()
    }

    /// Returns the completion status of the command buffer, if it has finished executing.
    pub fn completion_status(&self) -> Option<CommandBufferStatus> {
        let status = self.status.load(Ordering::Acquire);
        match status {
            0 => None,
            1 => Some(CommandBufferStatus::Completed),
            x => Some(CommandBufferStatus::Aborted(x - 2)),
        }
    }

    /// Returns the [`WorkerGroup`] which executes the command buffer.
    pub fn worker_group(&self) -> Result<WorkerGroup<'ctx>, Error> {
        self.handle.worker_group()
    }

    /// Blocks the current task until the command buffer has been completed.
    ///
    /// Can only be called from a task running in the same [`WorkerGroup`].
    pub fn join(self) -> Result<CommandBufferStatus, CommandBufferHandleError<'ctx, A>> {
        let this = ManuallyDrop::new(self);

        // Safety: FFI call is safe
        let result = unsafe {
            to_result_indirect_in_place(|err, aborted| {
                *err = (this.handle.vtable().v0.wait_on.unwrap_unchecked())(
                    this.handle.data(),
                    aborted.as_mut_ptr(),
                );
            })
        };

        match result {
            Ok(_) => Ok(this.completion_status().unwrap()),
            Err(err) => {
                let this = ManuallyDrop::into_inner(this);
                Err(CommandBufferHandleError(this, err))
            }
        }
    }
}

#[derive(Debug)]
struct CommandBufferHandleInner<'ctx> {
    handle: bindings::FiTasksCommandBufferHandle,
    _phantom: PhantomData<&'ctx ()>,
}

impl<'ctx> CommandBufferHandleInner<'ctx> {
    fn worker_group(&self) -> Result<WorkerGroup<'ctx>, Error> {
        // Safety: FFI call is safe
        let group = unsafe {
            to_result_indirect_in_place(|err, group| {
                *err = self.vtable().v0.worker_group.unwrap_unchecked()(
                    self.data(),
                    group.as_mut_ptr(),
                );
            })?
        };

        Ok(WorkerGroup(group, PhantomData))
    }

    fn into_raw_handle(self) -> bindings::FiTasksCommandBufferHandle {
        let this = ManuallyDrop::new(self);
        this.handle
    }

    #[inline(always)]
    fn data(&self) -> *mut std::ffi::c_void {
        self.handle.data
    }

    #[inline(always)]
    fn vtable(&self) -> &bindings::FiTasksCommandBufferHandleVTable {
        // Safety: The VTable is always initialized
        unsafe { &*self.handle.vtable }
    }
}

impl Clone for CommandBufferHandleInner<'_> {
    fn clone(&self) -> Self {
        // Safety: We own the reference and are therefore allowed to acquire another one.
        unsafe {
            self.vtable().v0.acquire.unwrap_unchecked()(self.data());
        }

        Self {
            handle: self.handle,
            _phantom: PhantomData,
        }
    }
}

impl Drop for CommandBufferHandleInner<'_> {
    fn drop(&mut self) {
        // Safety: We own the reference and are therefore allowed to release it.
        unsafe {
            self.vtable().v0.release.unwrap_unchecked()(self.data());
        }
    }
}

// Safety: Sound by invariant
unsafe impl Send for CommandBufferHandleInner<'_> {}

// Safety: Sound by invariant
unsafe impl Sync for CommandBufferHandleInner<'_> {}

/// Error from the [`CommandBufferHandle::join`] operation.
#[derive(Debug)]
pub struct CommandBufferHandleError<'ctx, A: Allocator>(CommandBufferHandle<'ctx, A>, Error);

impl<'ctx, A: Allocator> CommandBufferHandleError<'ctx, A> {
    /// Returns the contained error.
    pub fn error(&self) -> &Error {
        &self.1
    }

    /// Extracts the contained error.
    pub fn into_error(self) -> Error {
        self.1
    }

    /// Returns a reference to the [`CommandBufferHandle`] that caused the error.
    pub fn handle(&self) -> &CommandBufferHandle<'ctx, A> {
        &self.0
    }

    /// Extracts the [`CommandBufferHandle`] that caused the error.
    pub fn into_handle(self) -> CommandBufferHandle<'ctx, A> {
        self.0
    }
}

impl<A: Allocator> std::fmt::Display for CommandBufferHandleError<'_, A> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        <Error as std::fmt::Display>::fmt(&self.1, f)
    }
}

impl<A: Allocator> std::error::Error for CommandBufferHandleError<'_, A> where Self: std::fmt::Debug {}
