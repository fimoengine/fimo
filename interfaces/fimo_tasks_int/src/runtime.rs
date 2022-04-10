//! Task runtime interface.

use crate::raw::{
    IRawTask, ISchedulerContext, PseudoTask, TaskHandle, TaskScheduleStatus, WorkerId,
};
use crate::task::{Builder, JoinHandle, RawTaskWrapper, Task};
use fimo_ffi::ptr::{IBase, RawObj};
use fimo_ffi::span::ConstSpanPtr;
use fimo_ffi::{interface, ConstSpan, DynObj, FfiFn, ObjBox, ObjectId, Optional, ReprC};
use log::trace;
use std::mem::MaybeUninit;
use std::pin::Pin;
use std::sync::{Arc, Condvar, Mutex};
use std::time::{Duration, SystemTime};

#[thread_local]
static RUNTIME: std::cell::Cell<Option<*const DynObj<dyn IRuntime>>> = std::cell::Cell::new(None);

/// Data passed to a task upon wakeup.
#[repr(C, u8)]
#[derive(Debug, Copy, Clone, Hash, Ord, PartialOrd, PartialEq, Eq)]
pub enum WakeupToken {
    /// Wake up without any data.
    None,
    /// The wait operation was skipped by the runtime.
    Skipped,
    /// The wait operation timed out.
    ///
    /// # Note
    ///
    /// Reserved for future use.
    TimedOut,
    /// Custom data passed to the task.
    Custom(*const ()),
}

/// The passed data transfers ownership of the contents.
unsafe impl Send for WakeupToken {}
unsafe impl Sync for WakeupToken {}

/// Data provided by a task during a wait operation.
#[repr(transparent)]
#[derive(Debug, Copy, Clone, Hash, Ord, PartialOrd, PartialEq, Eq)]
pub struct WaitToken(pub *const ());

impl WaitToken {
    /// Invalid default token.
    pub const INVALID: WaitToken = WaitToken(std::ptr::null());
}

unsafe impl Send for WaitToken {}
unsafe impl Sync for WaitToken {}

impl Default for WaitToken {
    #[inline]
    fn default() -> Self {
        Self::INVALID
    }
}

/// Operation that `notify_filter` and `pseudo_notify_filter` should perform for each task.
#[repr(u8)]
#[derive(Debug, Copy, Clone, Hash, Ord, PartialOrd, PartialEq, Eq)]
pub enum NotifyFilterOp {
    /// Notifies the task and continues the filter operation.
    Notify,
    /// Stops the filter operation without notifying the task.
    Stop,
    /// Continues the filter operation without notifying the task.
    Skip,
}

/// Result of some `notify_*` and `pseudo_notify_*` functions.
#[repr(C)]
#[derive(Debug, Copy, Clone, Hash, Ord, PartialOrd, PartialEq, Eq)]
pub struct NotifyResult {
    /// Number of tasks notified by the operation.
    pub notified_tasks: usize,
    /// Number of remaining tasks still waiting for a notification.
    pub remaining_tasks: usize,
}

impl NotifyResult {
    /// Returns whether at least one task was notified.
    #[inline]
    pub fn has_notified_tasks(&self) -> bool {
        self.notified_tasks != 0
    }

    /// Returns whether there are remaining tasks.
    #[inline]
    pub fn has_tasks_remaining(&self) -> bool {
        self.remaining_tasks != 0
    }
}

/// Interface of a task runtime.
#[interface(
    uuid = "095a88ff-f45a-4cf8-a8f2-e18eb028a7de",
    vtable = "IRuntimeVTable",
    generate()
)]
pub trait IRuntime: Send + Sync {
    /// Retrieves the id of the current worker.
    fn worker_id(&self) -> Option<WorkerId>;

    /// Acquires a reference to the scheduler.
    ///
    /// The task will block, until the scheduler can be acquired.
    ///
    /// # Deadlock
    ///
    /// Trying to access the scheduler by other means than
    /// the provided reference may result in a deadlock.
    ///
    /// # Notes
    ///
    /// The closure is not allowed to panic.
    #[allow(clippy::type_complexity)]
    fn enter_scheduler_impl(
        &self,
        f: FfiFn<
            '_,
            dyn FnOnce(&mut DynObj<dyn IScheduler + '_>, Option<&DynObj<dyn IRawTask + '_>>) + '_,
        >,
    );

    /// Yields the current task to the runtime.
    ///
    /// Yields the current task to the runtime, allowing other tasks to be
    /// run on the current worker. On the next run of the scheduler it will call
    /// the provided function.
    ///
    /// # Notes
    ///
    /// The closure is not allowed to panic.
    #[allow(clippy::type_complexity)]
    fn yield_and_enter_impl(
        &self,
        f: FfiFn<
            '_,
            dyn FnOnce(&mut DynObj<dyn IScheduler + '_>, &DynObj<dyn IRawTask + '_>) + Send + '_,
        >,
    );
}

/// Extension trait for implementations of [`IRuntime`].
pub trait IRuntimeExt: IRuntime {
    /// Runs a task to completion on the task runtime.
    ///
    /// Blocks the current task until the new task has been completed.
    ///
    /// # Panics
    ///
    /// This function panics if the provided function panics.
    /// Can only be called from a worker thread.
    #[inline]
    #[track_caller]
    fn block_on<F: FnOnce() -> R + Send, R: Send>(
        &self,
        f: F,
        wait_on: &[TaskHandle],
    ) -> fimo_module::Result<R> {
        Builder::new().block_on(f, wait_on)
    }

    /// Runs a task to completion on the task runtime.
    ///
    /// Blocks the current task until the new task has been completed.
    ///
    /// # Panics
    ///
    /// This function panics if the provided function panics.
    #[inline]
    #[track_caller]
    fn block_on_and_enter<'th: 'b, 'a, 'b, F, R>(
        &'th self,
        f: F,
        wait_on: &'a [TaskHandle],
    ) -> fimo_module::Result<R>
    where
        F: FnOnce(&Self) -> R + Send + 'b,
        R: Send + 'b,
    {
        // if we are already owned by the runtime we can reuse the existing implementation.
        // otherwise we must implement the join functionality.
        if is_worker() {
            self.block_on(move || f(self), wait_on)
        } else {
            trace!("Entering the runtime");

            // task synchronisation is implemented with condition variables.
            #[derive(ObjectId)]
            #[fetch_vtable(uuid = "47c0e60c-8cd9-4dd1-8b21-79037a93278c")]
            struct CleanupData {
                condvar: Condvar,
                completed: Mutex<bool>,
            }

            // initialize the condvar and hold the mutex until we try to join.
            let data = Arc::new(CleanupData {
                condvar: Default::default(),
                completed: Mutex::new(false),
            });
            let data_cleanup = data.clone();

            let mut completed = data.completed.lock().unwrap();

            let f = move || f(self);
            let cleanup = move || {
                trace!("Notify owner thread");

                // after locking the mutex we are guaranteed that the owner
                // thread is waiting on the condvar, so we set the flag and notify it.
                let data = data_cleanup;
                let mut completed = data.completed.lock().unwrap();
                *completed = true;
                data.condvar.notify_all();
            };
            let join = |handle: JoinHandle<&'_ Task<'b, R>>| {
                trace!("Joining task on owner thread");

                // check if the task has been completed...
                while !*completed {
                    // ... if it isn't the case, wait.
                    completed = data.condvar.wait(completed).unwrap();
                }

                // decompose the handle as we can not rely on the `JoinHandle` outside of tasks.
                let handle = handle.into_inner();
                let task = handle.as_raw();

                // By this point the task has finished so we can unregister it.
                // SAFETY: The handle was the original owner of the task and now it has been
                // transferred to us, so we are allowed to unregister it.
                self.enter_scheduler(|s, _| unsafe {
                    assert!(matches!(s.unregister_task(task), Ok(_)));
                });

                let mut context = task.context().borrow_mut();
                match context.schedule_status() {
                    TaskScheduleStatus::Aborted => {
                        // empty errors indicate an aborted task.
                        // SAFETY: the task is aborted.
                        if let Some(err) = unsafe { context.take_panic_data() } {
                            use crate::raw::{IRustPanicData, IRustPanicDataExt};

                            // a runtime written in rust can choose to wrap the native
                            // panic into a `IRustPanicData`.
                            let err = ObjBox::cast_super::<dyn IBase>(err);
                            if let Some(err) = ObjBox::downcast_interface::<dyn IRustPanicData>(err)
                            {
                                let err = IRustPanicDataExt::take_rust_panic(err);
                                std::panic::resume_unwind(err)
                            }
                        }

                        panic!("Unknown panic!")
                    }
                    // SAFETY: the task finished so it has already initialized the result
                    TaskScheduleStatus::Finished => unsafe { Ok(handle.read_output()) },
                    _ => unreachable!(),
                }
            };

            Builder::new().block_on_complex(self, Some(f), Some(cleanup), wait_on, join)
        }
    }

    /// Spawns a task onto the task runtime.
    ///
    /// Spawns a task on any of the available workers, where it will run to completion.
    ///
    /// # Panics
    ///
    /// Can only be called from a worker thread.
    #[inline]
    #[track_caller]
    fn spawn<F, R>(
        &self,
        f: F,
        wait_on: &[TaskHandle],
    ) -> fimo_module::Result<JoinHandle<Pin<ObjBox<Task<'static, R>>>>>
    where
        F: FnOnce() -> R + Send + 'static,
        R: Send + 'static,
    {
        Builder::new().spawn(f, wait_on)
    }

    /// Acquires a reference to the scheduler.
    ///
    /// The task will block, until the scheduler can be acquired.
    ///
    /// # Deadlock
    ///
    /// Trying to access the scheduler by other means than
    /// the provided reference may result in a deadlock.
    ///
    /// # Panics
    ///
    /// Any panic inside the closure will be catched and resumed once the task resumes.
    #[inline]
    fn enter_scheduler<
        F: FnOnce(&mut DynObj<dyn IScheduler + '_>, Option<&DynObj<dyn IRawTask + '_>>) -> R,
        R,
    >(
        &self,
        f: F,
    ) -> R {
        use std::panic::{catch_unwind, resume_unwind, AssertUnwindSafe};

        trace!("Entering scheduler");
        let mut res = MaybeUninit::uninit();

        {
            let res = &mut res;
            let f = move |s: &mut DynObj<dyn IScheduler + '_>,
                          t: Option<&DynObj<dyn IRawTask + '_>>| {
                trace!("Scheduler entered");

                let x = catch_unwind(AssertUnwindSafe(|| f(s, t)));
                res.write(x);

                trace!("Exiting scheduler");
            };
            let mut f = MaybeUninit::new(f);

            unsafe {
                let f = FfiFn::new_value(&mut f);
                self.enter_scheduler_impl(f)
            }
        }

        trace!("Scheduler exited");
        let result = unsafe { res.assume_init() };
        match result {
            Ok(x) => x,
            Err(p) => resume_unwind(p),
        }
    }

    /// Yields the current task to the runtime.
    ///
    /// Yields the current task to the runtime, allowing other tasks to be
    /// run on the current worker.
    ///
    /// # Panics
    ///
    /// Can only be called from a worker thread.
    #[inline]
    fn yield_now(&self) {
        self.yield_and_enter(|_, _| {})
    }

    /// Yields the current task to the runtime for at least the specified amount of time.
    ///
    /// Yields the current task to the runtime, allowing other tasks to be
    /// run on the current worker. The task won't resume until the duration
    /// `dur`
    ///
    /// # Panics
    ///
    /// Can only be called from a worker thread.
    #[inline]
    fn yield_for(&self, dur: Duration) {
        let until = SystemTime::now() + dur;
        self.yield_until(until)
    }

    /// Yields the current task to the runtime until at least the specified time has passed.
    ///
    /// Yields the current task to the runtime, allowing other tasks to be
    /// run on the current worker. The task won't resume until the instant
    /// `until` has passed.
    ///
    /// # Panics
    ///
    /// Can only be called from a worker thread.
    #[inline]
    fn yield_until(&self, until: SystemTime) {
        self.yield_and_enter(move |_, curr| {
            trace!("Yielding task {:?} until {:?}", curr.resolved_name(), until);
            // we are inside the scheduler so the call to `borrow` is guaranteed to succeed.
            curr.context().borrow().set_resume_time(until)
        })
    }

    /// Yields the current task to the runtime.
    ///
    /// Yields the current task to the runtime, allowing other tasks to be
    /// run on the current worker. On the next run of the scheduler it will call
    /// the provided function.
    ///
    /// # Panics
    ///
    /// Can only be called from a worker thread. Any panic inside the closure
    /// will be catched and resumed once the task resumes.
    #[inline]
    fn yield_and_enter<
        F: FnOnce(&mut DynObj<dyn IScheduler + '_>, &DynObj<dyn IRawTask + '_>) -> R + Send,
        R: Send,
    >(
        &self,
        f: F,
    ) -> R {
        use std::panic::{catch_unwind, resume_unwind, AssertUnwindSafe};

        assert!(is_worker());
        trace!("Yielding to scheduler");

        let mut res = MaybeUninit::uninit();

        {
            let res = &mut res;
            let f = move |s: &mut DynObj<dyn IScheduler + '_>, t: &DynObj<dyn IRawTask + '_>| {
                trace!("Yielded to scheduler");

                let x = catch_unwind(AssertUnwindSafe(|| f(s, t)));
                res.write(x);
                trace!("Resuming task");
            };
            let mut f = MaybeUninit::new(f);

            unsafe {
                let f = FfiFn::new_value(&mut f);
                self.yield_and_enter_impl(f)
            }
        }

        trace!("Task resumed");

        let result = unsafe { res.assume_init() };
        match result {
            Ok(x) => x,
            Err(p) => resume_unwind(p),
        }
    }

    /// Blocks the current task indefinitely.
    ///
    /// # Panics
    ///
    /// Can only be called from a worker thread.
    #[inline]
    fn block_now(&self) {
        trace!("Requesting block of current task");

        self.yield_and_enter(|_, curr| {
            // we are inside the scheduler so the call to `borrow` is guaranteed to succeed.
            curr.context().borrow().request_block();
            trace!("Requested block for task {:?}", curr.resolved_name())
        })
    }

    /// Aborts the current task.
    ///
    /// The current task will be aborted immediately.
    ///
    /// # Safety
    ///
    /// Abortion of tasks is currently implemented by unwinding the stack.
    /// Future implementations my implemented by other means than unwinding.
    ///
    /// # Panics
    ///
    /// Can only be called from a worker thread.
    #[inline]
    unsafe fn abort_now(&self) -> ! {
        trace!("Aborting current task");
        struct Abort {}
        std::panic::resume_unwind(Box::new(Abort {}))
    }

    /// Blocks the current task, until another task completes.
    ///
    /// The current task will yield it's execution, until the other task completes.
    /// Returns whether the task waited on the other task.
    ///
    /// Passing the handle to the current or non existant task will always
    /// succeed with `WakeupToken::Skipped`. May return `WakeupToken::Skipped` if
    /// the runtime determines that waiting is not necessary.
    ///
    /// On success the specified task will be completed.
    ///
    /// # Panics
    ///
    /// Can only be called from a worker thread.
    #[inline]
    fn wait_on(&self, handle: TaskHandle) -> fimo_module::Result<WakeupToken> {
        trace!("Wait until task-id {} completes", handle);

        let mut data = MaybeUninit::uninit();
        let data_ref = &mut data;
        let res = self.yield_and_enter(move |s, curr| {
            // SAFETY: The task is provided from the scheduler, so it is registered.
            // we are inside the scheduler so the call to `borrow` is guaranteed to succeed.
            if unsafe { curr.context().borrow().handle().assume_init() } != handle {
                // SAFETY: A wait operation does not invalidate the task or cause race-conditions.
                if let Ok(wait_on) = unsafe { s.find_task_unbound(handle) } {
                    // SAFETY: Both tasks are provided by the scheduler and can't have been
                    // unregistered in the meantime, as the runtime is locked.
                    unsafe {
                        s.wait_task_on(curr, wait_on, Some(data_ref), WaitToken::INVALID)
                            .map(|_| WakeupToken::None)
                    }
                } else {
                    trace!("The task-id {} was not found, skipping wait", handle);
                    Ok(WakeupToken::Skipped)
                }
            } else {
                trace!("The task-id {} refers to itself, skipping wait", handle);
                Ok(WakeupToken::Skipped)
            }
        });

        match res {
            Ok(WakeupToken::None) => {
                // SAFETY: By the contract of the `wait_task_on` we know that
                // if the task was put to sleep, it will have some data passed to
                // once it is woken up. We just checked that the wait operation
                // was successful, therefore the data was written in `data`.
                unsafe { Ok(data.assume_init()) }
            }
            Ok(WakeupToken::Skipped) => Ok(WakeupToken::Skipped),
            Ok(_) => unreachable!(),
            Err(e) => Err(e),
        }
    }
}

impl<T: IRuntime + ?Sized> IRuntimeExt for T {}

/// Interface of a scheduler.
#[interface(
    uuid = "095a88ff-f45a-4cf8-a8f2-e18eb028a7de",
    vtable = "ISchedulerVTable",
    generate()
)]
pub trait IScheduler: Sync {
    /// Fetches the id's of all available workers.
    #[vtable_info(
        unsafe,
        abi = r#"extern "C-unwind""#,
        return_type = "ConstSpanPtr<WorkerId>",
        into = "Into::into",
        from_expr = "unsafe { res.deref().into() }"
    )]
    fn worker_ids(&self) -> &[WorkerId];

    /// Searches for a registered task.
    #[inline]
    #[vtable_info(ignore)]
    fn find_task(&self, handle: TaskHandle) -> fimo_module::Result<&DynObj<dyn IRawTask + '_>> {
        unsafe { self.find_task_unbound(handle) }
    }

    /// Searches for a registered task.
    ///
    /// # Safety
    ///
    /// The reference may outlive the task, if it is stored or the scheduler is modified.
    #[vtable_info(
        unsafe,
        abi = r#"extern "C-unwind""#,
        return_type = "fimo_module::FFIResult<RawObj<dyn IRawTask + 'static>>",
        into_expr = "let res = fimo_module::FFIResult::from_rust(res)?; fimo_module::FFIResult::Ok(fimo_ffi::ptr::into_raw(res))",
        from_expr = "let res = res.into_rust()?; Ok(&*(std::mem::transmute::<_, *const DynObj<dyn IRawTask + 'a>>(fimo_ffi::ptr::from_raw(res))))"
    )]
    unsafe fn find_task_unbound<'a>(
        &self,
        handle: TaskHandle,
    ) -> fimo_module::Result<&'a DynObj<dyn IRawTask + 'a>>;

    /// Registers a task with the runtime for execution.
    ///
    /// This function effectively tries to transfer the ownership of the task
    /// to the runtime. On success, the caller may request the runtime to give
    /// up the ownership of the task by calling the [`unregister_task`](#method.unregister_task)
    /// method.
    ///
    /// The task may be modified by the method even in the case that it doen't succeed.
    /// In case of failure, the task should be seen as invalid and be dropped.
    ///
    /// # Safety
    ///
    /// Behavior is undefined if any of the following conditions are violated:
    ///
    /// * The task must be valid.
    /// * The same task may not be registered multiple times.
    /// * The task may not be moved while owned by the runtime.
    /// * The task must be kept alive until the runtime relinquishes the ownership.
    #[vtable_info(
        unsafe,
        abi = r#"extern "C-unwind""#,
        return_type = "fimo_module::FFIResult<u8>",
        into_expr = "let _ = fimo_module::FFIResult::from_rust(res)?; fimo_module::FFIResult::Ok(0)",
        from_expr = "let _ = res.into_rust()?; Ok(())"
    )]
    unsafe fn register_task(
        &mut self,
        #[vtable_info(
            type = "RawObj<dyn IRawTask + '_>",
            into = "fimo_ffi::ptr::into_raw",
            from_expr = "let p_1 = &*fimo_ffi::ptr::from_raw(p_1);"
        )]
        task: &DynObj<dyn IRawTask + '_>,
        #[vtable_info(
            type = "ConstSpan<'_, TaskHandle>",
            into = "Into::into",
            from = "Into::into"
        )]
        wait_on: &[TaskHandle],
    ) -> fimo_module::Result<()>;

    /// Unregisters a task from the task runtime.
    ///
    /// Requests for the runtime to give up its ownership of the task.
    /// For this method to complete, the task must have run to completion
    /// (i. e. is finished or aborted). On success the task is invalidated
    /// and the ownership returned to the caller.
    ///
    /// # Safety
    ///
    /// This method can be thought of the task equivalent of the `free` function
    /// which deallocates a memory allocation. Following this analogy the task
    /// must have been registered with the runtime with a call to
    /// [`register_task`](#method.register_task). Further, a caller must ensure
    /// that they are the original owners of the task and are not merely borrowing
    /// it by the likes of [`find_task`](#method.find_task) or any other means.
    #[vtable_info(
        unsafe,
        abi = r#"extern "C-unwind""#,
        return_type = "fimo_module::FFIResult<u8>",
        into_expr = "let _ = fimo_module::FFIResult::from_rust(res)?; fimo_module::FFIResult::Ok(0)",
        from_expr = "let _ = res.into_rust()?; Ok(())"
    )]
    unsafe fn unregister_task(
        &mut self,
        #[vtable_info(
            type = "RawObj<dyn IRawTask + '_>",
            into = "fimo_ffi::ptr::into_raw",
            from_expr = "let p_1 = &*fimo_ffi::ptr::from_raw(p_1);"
        )]
        task: &DynObj<dyn IRawTask + '_>,
    ) -> fimo_module::Result<()>;

    /// Allocates or fetches a [`PseudoTask`] bound to the given address.
    ///
    ///
    /// On success, the caller may request the runtime to unbind to provided address
    /// by calling the method.
    ///
    /// # Safety
    ///
    /// Condition:
    ///
    /// * The address must be controlled by the caller until it is unbound.
    #[vtable_info(
        unsafe,
        abi = r#"extern "C-unwind""#,
        return_type = "fimo_module::FFIResult<PseudoTask>",
        into_expr = "let t = fimo_module::FFIResult::from_rust(res)?; fimo_module::FFIResult::Ok(t)",
        from_expr = "let t = res.into_rust()?; Ok(t)"
    )]
    unsafe fn register_or_fetch_pseudo_task(
        &mut self,
        addr: *const (),
    ) -> fimo_module::Result<PseudoTask>;

    /// Unregisters a pseudo task from the task runtime.
    ///
    /// Invalidates the pseudo task and unbinds the bound address.
    ///
    /// # Safety
    ///
    /// This method can be thought of the task equivalent of the `free` function
    /// which deallocates a memory allocation. Following this analogy the task
    /// must have been registered with the runtime with a call to
    /// [`register_or_fetch_pseudo_task`](#method.register_or_fetch_pseudo_task).
    /// Further, a caller must ensure that they are the original owners of the task
    /// and are not merely borrowing it.
    #[vtable_info(
        unsafe,
        abi = r#"extern "C-unwind""#,
        return_type = "fimo_module::FFIResult<u8>",
        into_expr = "let _ = fimo_module::FFIResult::from_rust(res)?; fimo_module::FFIResult::Ok(0)",
        from_expr = "let _ = res.into_rust()?; Ok(())"
    )]
    unsafe fn unregister_pseudo_task(&mut self, task: PseudoTask) -> fimo_module::Result<()>;

    /// Unregisters a pseudo task from the task runtime if it is empty.
    ///
    /// Invalidates the pseudo task and unbinds the bound address if it is in
    /// an empty state, like after the first call to [`register_or_fetch_pseudo_task`].
    /// Returns whether the pseudo task was unregistered.
    ///
    /// # Safety
    ///
    /// This method can be thought of the task equivalent of the `free` function
    /// which deallocates a memory allocation. Following this analogy the task
    /// must have been registered with the runtime with a call to
    /// [`register_or_fetch_pseudo_task`](#method.register_or_fetch_pseudo_task).
    /// Further, a caller must ensure that they are the original owners of the task
    /// and are not merely borrowing it.
    ///
    /// [`register_or_fetch_pseudo_task`]: #method.register_or_fetch_pseudo_task
    #[vtable_info(
        unsafe,
        abi = r#"extern "C-unwind""#,
        return_type = "fimo_module::FFIResult<bool>",
        into_expr = "let v = fimo_module::FFIResult::from_rust(res)?; fimo_module::FFIResult::Ok(v)",
        from_expr = "let v = res.into_rust()?; Ok(v)"
    )]
    unsafe fn unregister_pseudo_task_if_empty(
        &mut self,
        task: PseudoTask,
    ) -> fimo_module::Result<bool>;

    /// Registers a block for a task until the dependency completes.
    ///
    /// A task may not wait on itself or wait on another task multiple times.
    /// After being woken up `data_addr` is initialized with a message from the task that woke it up.
    ///
    /// # Note
    ///
    /// Does not guarantee that the task will wait immediately if it is already scheduled.
    ///
    /// # Safety
    ///
    /// Both tasks must be registered with the runtime.
    #[vtable_info(
        unsafe,
        abi = r#"extern "C-unwind""#,
        return_type = "fimo_module::FFIResult<u8>",
        into_expr = "let _ = fimo_module::FFIResult::from_rust(res)?; fimo_module::FFIResult::Ok(0)",
        from_expr = "let _ = res.into_rust()?; Ok(())"
    )]
    unsafe fn wait_task_on(
        &mut self,
        #[vtable_info(
            type = "RawObj<dyn IRawTask + '_>",
            into = "fimo_ffi::ptr::into_raw",
            from_expr = "let p_1 = &*fimo_ffi::ptr::from_raw(p_1);"
        )]
        task: &DynObj<dyn IRawTask + '_>,
        #[vtable_info(
            type = "RawObj<dyn IRawTask + '_>",
            into = "fimo_ffi::ptr::into_raw",
            from_expr = "let p_2 = &*fimo_ffi::ptr::from_raw(p_2);"
        )]
        on: &DynObj<dyn IRawTask + '_>,
        #[vtable_info(
            type = "Optional<&mut MaybeUninit<WakeupToken>>",
            into = "Into::into",
            from = "Into::into"
        )]
        data_addr: Option<&mut MaybeUninit<WakeupToken>>,
        token: WaitToken,
    ) -> fimo_module::Result<()>;

    /// Registers a block for a task until the pseudo task releases it.
    ///
    /// A task may not wait on a pseudo task multiple times.
    /// After being woken up `data_addr` is initialized with a message from the task that woke it up.
    ///
    /// # Note
    ///
    /// Does not guarantee that the task will wait immediately if it is already scheduled.
    ///
    /// # Safety
    ///
    /// Both tasks must be registered with the runtime.
    #[vtable_info(
        unsafe,
        abi = r#"extern "C-unwind""#,
        return_type = "fimo_module::FFIResult<u8>",
        into_expr = "let _ = fimo_module::FFIResult::from_rust(res)?; fimo_module::FFIResult::Ok(0)",
        from_expr = "let _ = res.into_rust()?; Ok(())"
    )]
    unsafe fn pseudo_wait_task_on(
        &mut self,
        #[vtable_info(
            type = "RawObj<dyn IRawTask + '_>",
            into = "fimo_ffi::ptr::into_raw",
            from_expr = "let p_1 = &*fimo_ffi::ptr::from_raw(p_1);"
        )]
        task: &DynObj<dyn IRawTask + '_>,
        on: PseudoTask,
        #[vtable_info(
            type = "Optional<&mut MaybeUninit<WakeupToken>>",
            into = "Into::into",
            from = "Into::into"
        )]
        data_addr: Option<&mut MaybeUninit<WakeupToken>>,
        token: WaitToken,
    ) -> fimo_module::Result<()>;

    /// Wakes up one task.
    ///
    /// Wakes up the task with the highest priority, that is waiting on provided the task to finish.
    ///
    /// # Safety
    ///
    /// A waiting task may require that the task fully finishes before
    /// resuming execution. This function is mainly intended to be
    /// used for the implementation of condition variables.
    ///
    /// The closure `data_callback` may not panic or call into the scheduler.
    /// Further, the behavior is undefined if called with an unregistered task.
    #[vtable_info(
        unsafe,
        abi = r#"extern "C-unwind""#,
        return_type = "fimo_module::FFIResult<NotifyResult>",
        into = "Into::into",
        from = "Into::into"
    )]
    unsafe fn notify_one(
        &mut self,
        #[vtable_info(
            type = "RawObj<dyn IRawTask + '_>",
            into = "fimo_ffi::ptr::into_raw",
            from_expr = "let p_1 = &*fimo_ffi::ptr::from_raw(p_1);"
        )]
        task: &DynObj<dyn IRawTask + '_>,
        data_callback: FfiFn<'_, dyn FnOnce(NotifyResult) -> WakeupToken + '_, u8>,
    ) -> fimo_module::Result<NotifyResult>;

    /// Wakes up one task.
    ///
    /// Wakes up the task with the highest priority, that is waiting on the provided pseudo task.
    ///
    /// # Safety
    ///
    /// The pseudo task must be registered with the runtime.
    /// Further, the closure `data_callback` may not panic or call into the scheduler.
    #[vtable_info(
        unsafe,
        abi = r#"extern "C-unwind""#,
        return_type = "fimo_module::FFIResult<NotifyResult>",
        into = "Into::into",
        from = "Into::into"
    )]
    unsafe fn pseudo_notify_one(
        &mut self,
        task: PseudoTask,
        data_callback: FfiFn<'_, dyn FnOnce(NotifyResult) -> WakeupToken + '_, u8>,
    ) -> fimo_module::Result<NotifyResult>;

    /// Wakes up all tasks.
    ///
    /// Wakes up all waiting tasks. Returns the number of tasks that were woken up.
    ///
    /// # Safety
    ///
    /// A waiting task may require that the task fully finishes before
    /// resuming execution. This function is mainly intended to be
    /// used for the implementation of condition variables.
    ///
    /// Further, the behavior is undefined if called with an unregistered task.
    #[vtable_info(
        unsafe,
        abi = r#"extern "C-unwind""#,
        return_type = "fimo_module::FFIResult<usize>",
        into = "Into::into",
        from = "Into::into"
    )]
    unsafe fn notify_all(
        &mut self,
        #[vtable_info(
            type = "RawObj<dyn IRawTask + '_>",
            into = "fimo_ffi::ptr::into_raw",
            from_expr = "let p_1 = &*fimo_ffi::ptr::from_raw(p_1);"
        )]
        task: &DynObj<dyn IRawTask + '_>,
        data: WakeupToken,
    ) -> fimo_module::Result<usize>;

    /// Wakes up all tasks.
    ///
    /// Wakes up all waiting tasks. Returns the number of tasks that were woken up.
    ///
    /// # Safety
    ///
    /// The pseudo task must be registered with the runtime.
    #[vtable_info(
        unsafe,
        abi = r#"extern "C-unwind""#,
        return_type = "fimo_module::FFIResult<usize>",
        into = "Into::into",
        from = "Into::into"
    )]
    unsafe fn pseudo_notify_all(
        &mut self,
        task: PseudoTask,
        data: WakeupToken,
    ) -> fimo_module::Result<usize>;

    /// Notifies a number of tasks depending on the result of a filter function.
    ///
    /// The `filter` function is called for each task in the queue or until
    /// [`NotifyFilterOp::Stop`] is returned. This function is passed the
    /// [`WaitToken`] associated with a particular task, which is notified if
    /// [`NotifyFilterOp::Notify`] is returned.
    ///
    /// The `data_callback` function is passed an [`NotifyResult`] indicating the
    /// number of tasks that were notified and whether there are still waiting
    /// tasks in the queue. This [`NotifyResult`] value is also returned by
    /// `pseudo_notify_filter`.
    ///
    /// The `data_callback` function should return an UnparkToken value which will
    /// be passed to all tasks that are notified. If no task is notified then the
    /// returned value is ignored.
    ///
    /// # Safety
    ///
    /// The pseudo task must be registered with the runtime.
    /// Further, `filter` and `data_callback` may not panic or call into the scheduler.
    #[vtable_info(
        unsafe,
        abi = r#"extern "C-unwind""#,
        return_type = "fimo_module::FFIResult<NotifyResult>",
        into = "Into::into",
        from = "Into::into"
    )]
    unsafe fn pseudo_notify_filter(
        &mut self,
        task: PseudoTask,
        filter: FfiFn<'_, dyn FnMut(WaitToken) -> NotifyFilterOp + '_, u8>,
        data_callback: FfiFn<'_, dyn FnOnce(NotifyResult) -> WakeupToken + '_, u8>,
    ) -> fimo_module::Result<NotifyResult>;

    /// Unblocks a blocked task.
    ///
    /// Once unblocked, the task may resume it's execution.
    /// May error if the task is not blocked.
    ///
    /// # Safety
    ///
    /// The task must be registered with the runtime.
    #[vtable_info(
        unsafe,
        abi = r#"extern "C-unwind""#,
        return_type = "fimo_module::FFIResult<u8>",
        into_expr = "let _ = fimo_module::FFIResult::from_rust(res)?; fimo_module::FFIResult::Ok(0)",
        from_expr = "let _ = res.into_rust()?; Ok(())"
    )]
    unsafe fn unblock_task(
        &mut self,
        #[vtable_info(
            type = "RawObj<dyn IRawTask + '_>",
            into = "fimo_ffi::ptr::into_raw",
            from_expr = "let p_1 = &*fimo_ffi::ptr::from_raw(p_1);"
        )]
        task: &DynObj<dyn IRawTask + '_>,
    ) -> fimo_module::Result<()>;
}

/// Returns whether a thread is a managed by a runtime.
#[inline]
pub fn is_worker() -> bool {
    RUNTIME.get().is_some()
}

/// Returns a reference to the [`IRuntime`] that owns the worker.
///
/// The reference remains valid as long as the worker thread is kept alive.
///
/// # Safety
///
/// **Must** be run from within a worker after a call to [`init_runtime()`].
#[inline]
pub unsafe fn get_runtime() -> &'static DynObj<dyn IRuntime> {
    &*RUNTIME.get().unwrap_unchecked()
}

/// Returns a reference to the [`IRuntime`] that owns the worker.
///
/// The reference remains valid as long as the worker thread is kept alive.
#[inline]
pub fn current_runtime() -> Option<&'static DynObj<dyn IRuntime>> {
    if is_worker() {
        unsafe { Some(get_runtime()) }
    } else {
        None
    }
}

/// Provides the runtime to the current worker.
///
/// # Safety
///
/// May not be called with multiple runtimes from the same worker.
pub unsafe fn init_runtime(runtime: *const DynObj<dyn IRuntime>) {
    RUNTIME.set(Some(runtime))
}
