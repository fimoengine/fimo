//! Utilities for handling tasks.
//!
use crate::raw::{
    IRawTask, ISchedulerContext, RawTaskInner, TaskPriority, TaskScheduleStatus, WorkerId,
};
use crate::runtime::{get_runtime, is_worker, IRuntimeExt, IScheduler};
use crate::TaskHandle;
use fimo_ffi::ptr::IBase;
use fimo_ffi::{DynObj, FfiFn, ObjArc, ObjBox};
use fimo_module::{Error, ErrorKind};
use log::{error, trace};
use std::cell::UnsafeCell;
use std::mem::MaybeUninit;
use std::ops::Deref;
use std::pin::Pin;
use std::sync::Arc;
use std::time::SystemTime;

/// An owned permission to join on a task (block on its termination).
///
/// A `JoinHandle` blocks until the task is terminated when dropped.
///
/// # Safety
///
/// The handle it tied to the runtime that owns the task and may not
/// be moved outside of it.
#[derive(Debug)]
pub struct JoinHandle<T: RawTaskWrapper> {
    handle: T,
}

impl<T: RawTaskWrapper<Output = R>, R> JoinHandle<T> {
    /// Fetches the handle of the task.
    #[inline]
    pub fn handle(&self) -> TaskHandle {
        // safety: we own the task and know that it is registered.
        unsafe { self.handle.as_raw().context_atomic().handle().assume_init() }
    }

    /// Fetches a reference to the contained raw task.
    #[inline]
    pub fn as_raw(&self) -> &DynObj<dyn IRawTask + '_> {
        self.handle.as_raw()
    }

    /// Returns a reference to the output of the task.
    ///
    /// # Safety
    ///
    /// This function is only safe, if the task has been completed successfully.
    #[inline]
    pub unsafe fn assume_completed_ref(&self) -> &R {
        self.handle.peek_output()
    }

    /// Returns a mutable reference to the output of the task.
    ///
    /// # Safety
    ///
    /// This function is only safe, if the task has been completed successfully.
    #[inline]
    pub unsafe fn assume_completed_mut(&mut self) -> &mut R {
        self.handle.peek_output_mut()
    }

    /// Reads the output of the task without moving it.
    ///
    /// Equivalent of using [`std::ptr::read()`] on the return value of the task.
    ///
    /// # Safety
    ///
    /// This function is only safe, if the task has been completed successfully.
    #[inline]
    pub unsafe fn assume_completed_read(&self) -> R {
        self.handle.read_output()
    }

    /// Returns the inner handle consuming the `JoinHandle` without joining the task.
    pub fn into_inner(self) -> T {
        let handle = unsafe { std::ptr::read(&self.handle) };
        std::mem::forget(self);
        handle
    }

    /// Waits for the associated task to finish.
    ///
    /// # Panics
    ///
    /// This function may panic if a task tries to join itself.
    pub fn join(mut self) -> Result<R, Option<ObjBox<DynObj<dyn IBase + Send>>>> {
        trace!(
            "Joining task-id {}, name {:?}",
            self.handle(),
            self.as_raw().resolved_name()
        );

        // join the task.
        let res = unsafe { self.join_ref() };

        // at this point the task has already been consumed, so we must forget it.
        std::mem::forget(self);
        res
    }

    /// Joins the task by reference.
    ///
    /// # Panics
    ///
    /// This function may panic if a task tries to join itself.
    ///
    /// # Safety
    ///
    /// This call consumes the `JoinHandle`.
    #[inline]
    unsafe fn join_ref(&mut self) -> Result<R, Option<ObjBox<DynObj<dyn IBase + Send>>>> {
        let runtime = get_runtime();
        let handle = self.handle();
        let raw = self.handle.as_raw();

        // wait for the task to complete.
        assert!(matches!(runtime.wait_on(handle), Ok(_)));

        // unregister the completed task.
        runtime.enter_scheduler(|s, _| {
            assert!(matches!(s.unregister_task(raw), Ok(_)));
        });

        // safety: the task is unowned.
        let context = &mut *raw.context().borrow_mut();
        match context.schedule_status() {
            TaskScheduleStatus::Aborted => Err(context.take_panic_data()),
            TaskScheduleStatus::Finished => Ok(self.assume_completed_read()),
            _ => unreachable!(),
        }
    }

    /// Waits for the associated task to finish.
    ///
    /// Once finished, extracts a reference to the result.
    /// If the task was aborted this function returns [`None`].
    pub fn wait(&self) -> Option<&R> {
        debug_assert!(is_worker());
        trace!(
            "Waiting on task-id {}, name {:?}",
            self.handle(),
            self.as_raw().resolved_name()
        );

        let runtime = unsafe { get_runtime() };
        assert!(matches!(runtime.wait_on(self.handle()), Ok(_)));

        let context = self.as_raw().context_atomic();
        match context.schedule_status() {
            TaskScheduleStatus::Aborted => None,
            TaskScheduleStatus::Finished => unsafe { Some(self.assume_completed_ref()) },
            _ => unreachable!(),
        }
    }

    /// Waits for the associated task to finish.
    ///
    /// Once finished, extracts a mutable reference to the result.
    /// If the task was aborted this function returns [`None`].
    pub fn wait_mut(&mut self) -> Option<&mut R> {
        debug_assert!(is_worker());
        trace!(
            "Waiting on task-id {}, name {:?}",
            self.handle(),
            self.as_raw().resolved_name()
        );

        let runtime = unsafe { get_runtime() };
        assert!(matches!(runtime.wait_on(self.handle()), Ok(_)));

        let context = self.as_raw().context_atomic();
        match context.schedule_status() {
            TaskScheduleStatus::Aborted => None,
            TaskScheduleStatus::Finished => unsafe { Some(self.assume_completed_mut()) },
            _ => unreachable!(),
        }
    }

    /// Unblocks the task.
    pub fn unblock(&mut self) -> Result<(), Error> {
        debug_assert!(is_worker());
        trace!(
            "Unblocking task-id {}, name {:?}",
            self.handle(),
            self.as_raw().resolved_name()
        );

        let runtime = unsafe { get_runtime() };
        runtime.enter_scheduler(|s, _| s.unblock_task(self.as_raw()))
    }

    /// Requests for a task to be blocked.
    ///
    /// # Note
    ///
    /// Does not actually guarantee, that the task will be blocked.
    #[inline]
    pub fn request_block(&mut self) {
        trace!(
            "Requesting block for task-id {}, name {:?}",
            self.handle(),
            self.as_raw().resolved_name()
        );

        self.as_raw().context_atomic().request_block();
    }

    /// Requests for a task to be aborted.
    ///
    /// # Note
    ///
    /// Does not actually guarantee, that the task will be aborted.
    ///
    /// # Safety
    ///
    /// Aborting a task may lead to broken invariants.
    #[inline]
    pub unsafe fn request_abort(&mut self) {
        trace!(
            "Requesting abort for task-id {}, name {:?}",
            self.handle(),
            self.as_raw().resolved_name()
        );

        self.as_raw().context_atomic().request_abort();
    }
}

impl<T: RawTaskWrapper> Drop for JoinHandle<T> {
    fn drop(&mut self) {
        trace!(
            "Dropping task-id {}, name {:?}",
            self.handle(),
            self.as_raw().resolved_name()
        );
        unsafe { assert!(matches!(self.join_ref(), Ok(_))) };
    }
}

/// Definition of a task.
#[derive(Debug)]
pub struct Task<'a, T> {
    raw: RawTaskInner<'a>,
    res: UnsafeCell<MaybeUninit<T>>,
}

/// A wrapper around a raw task.
pub trait RawTaskWrapper: Deref {
    /// Output type of the task.
    type Output;

    /// Extracts a reference to the task.
    fn as_raw(&self) -> &DynObj<dyn IRawTask + '_>;

    /// Reads the output of the task without moving it.
    ///
    /// Equivalent of using [`std::ptr::read()`] on the return value of the task.
    ///
    /// # Safety
    ///
    /// This function is only safe, if the task has been completed successfully.
    unsafe fn read_output(&self) -> Self::Output;

    /// Returns a reference to the output of the task.
    ///
    /// # Safety
    ///
    /// This function is only safe, if the task has been completed successfully.
    unsafe fn peek_output(&self) -> &Self::Output;

    /// Returns a mutable reference to the output of the task.
    ///
    /// # Safety
    ///
    /// This function is only safe, if the task has been completed successfully.
    unsafe fn peek_output_mut(&mut self) -> &mut Self::Output;
}

macro_rules! as_raw_impl {
    ($($type: ty),*) => {
        $(impl<'a, T: 'a> RawTaskWrapper for $type {
            type Output = T;

            #[inline]
            fn as_raw(&self) -> &DynObj<dyn IRawTask + '_> {
                fimo_ffi::ptr::coerce_obj(&self.raw)
            }

            #[inline]
            unsafe fn read_output(&self) -> T {
                std::ptr::read(self.res.get()).assume_init()
            }

            #[inline]
            unsafe fn peek_output(&self) -> &T {
                (*self.res.get()).assume_init_ref()
            }

            #[inline]
            unsafe fn peek_output_mut(&mut self) -> &mut T {
                (*self.res.get()).assume_init_mut()
            }
        })*
    };
}

as_raw_impl! {&Task<'a, T>}
as_raw_impl! {Pin<Box<Task<'a, T>>>, Pin<ObjBox<Task<'a, T>>>}
as_raw_impl! {Pin<Arc<Task<'a, T>>>, Pin<ObjArc<Task<'a, T>>>}

/// A task builder.
#[derive(Debug)]
pub struct Builder {
    inner: crate::raw::Builder,
}

impl Builder {
    /// Constructs a new `Builder`.
    #[inline]
    #[track_caller]
    pub fn new() -> Self {
        Self {
            inner: Default::default(),
        }
    }

    /// Names the task.
    #[inline]
    pub fn with_name(mut self, name: String) -> Self {
        self.inner = self.inner.with_name(name);
        self
    }

    /// Assigns a priority to the task.
    ///
    /// A lower [`TaskPriority`] value will lead to a higher priority.
    /// The default priority is `TaskPriority(0)`.
    #[inline]
    pub fn with_priority(mut self, priority: TaskPriority) -> Self {
        self.inner = self.inner.with_priority(priority);
        self
    }

    /// Assigns a start time to the task.
    #[inline]
    pub fn with_start_time(mut self, start_time: SystemTime) -> Self {
        self.inner = self.inner.with_start_time(start_time);
        self
    }

    /// Assigns a worker to the task.
    ///
    /// # Safety
    ///
    /// Behavior is undefined if the worker does not exist.
    #[inline]
    pub unsafe fn with_worker(mut self, worker: Option<WorkerId>) -> Self {
        self.inner = self.inner.with_worker(worker);
        self
    }

    /// Marks the task as blocked.
    #[inline]
    pub fn blocked(mut self) -> Self {
        self.inner = self.inner.blocked();
        self
    }

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
    pub fn block_on<'a, 'b, F, R>(self, f: F, wait_on: &'b [TaskHandle]) -> fimo_module::Result<R>
    where
        F: FnOnce() -> R + Send + 'a,
        R: Send + 'a,
    {
        if !is_worker() {
            return Err(Error::new(
                ErrorKind::FailedPrecondition,
                "spawn() can only be called from an initialized worker thread",
            ));
        }

        let runtime = unsafe { get_runtime() };
        let join = |handle: JoinHandle<&'_ Task<'a, R>>| -> fimo_module::Result<R> {
            match handle.join() {
                Ok(v) => Ok(v),
                Err(err) => {
                    // empty errors indicate an aborted task.
                    if let Some(err) = err {
                        use crate::raw::{IRustPanicData, IRustPanicDataExt};

                        // a runtime written in rust can choose to wrap the native
                        // panic into a `IRustPanicData`.
                        let err = ObjBox::cast_super::<dyn IBase>(err);
                        if let Some(err) = ObjBox::downcast_interface::<dyn IRustPanicData>(err) {
                            let err = IRustPanicDataExt::take_rust_panic(err);
                            std::panic::resume_unwind(err)
                        }
                    }

                    panic!("Unknown panic!")
                }
            }
        };

        self.block_on_complex::<_, _, _, fn(), _>(runtime, Some(f), None, wait_on, join)
    }

    /// Runs a task to completion on the task runtime.
    ///
    /// Blocks the current task until the new task has been completed.
    #[inline]
    #[track_caller]
    pub fn block_on_complex<'a, 'b, 'c, Run, F, R, Cleanup, Join>(
        self,
        runtime: &'b Run,
        f: Option<F>,
        cleanup: Option<Cleanup>,
        wait_on: &'c [TaskHandle],
        join: Join,
    ) -> fimo_module::Result<R>
    where
        Run: IRuntimeExt + ?Sized,
        F: FnOnce() -> R + Send + 'a,
        Cleanup: FnOnce() + Send + 'a,
        R: Send + 'a,
        Join: FnOnce(JoinHandle<&'_ Task<'a, R>>) -> fimo_module::Result<R>,
    {
        trace!("Blocking on new task");

        if f.is_none() && std::mem::size_of::<R>() != 0 {
            error!("Tried to spawn an empty task with an invalid return type");
            return Err(Error::new(
                ErrorKind::InvalidArgument,
                "empty tasks with non zst return types are not allowed",
            ));
        }

        let mut task: MaybeUninit<UnsafeCell<Task<'a, R>>> = MaybeUninit::uninit();

        // fetch the addresses to the inner members, so we can initialize them.
        let task_ptr = UnsafeCell::raw_get(task.as_ptr());
        let raw_ptr = unsafe { std::ptr::addr_of_mut!((*task_ptr).raw) };
        let res_ptr = unsafe { std::ptr::addr_of_mut!((*task_ptr).res) };

        struct AssertSend<T>(*mut T);

        // SAFETY: we know that T is Send
        unsafe impl<T: Send> Send for AssertSend<T> {}

        let res = unsafe {
            // initialize res and fetch a pointer to the inner result.
            res_ptr.write(UnsafeCell::new(MaybeUninit::uninit()));
            AssertSend((*(*res_ptr).get()).as_mut_ptr())
        };

        let f = if let Some(f) = f {
            let f = move || {
                // write the result directly into the address, knowing that it will live
                // at least as long as the task itself.
                unsafe { res.0.write(f()) };
            };

            Some(FfiFn::r#box(Box::new(f)))
        } else {
            None
        };

        let cleanup = cleanup.map(|cleanup| FfiFn::r#box(Box::new(cleanup)));

        // initialize the raw field.
        let raw_task = self.inner.build(f, cleanup);
        unsafe { raw_ptr.write(raw_task) };

        // SAFETY: all fields have been initialized so we can fetch a reference to it
        // but we aren't allowed to move the task as it contains a self referencing pointer
        // to the result member.
        let handle = unsafe { &*task.assume_init_ref().get() };

        match runtime.enter_scheduler(move |s, _| unsafe {
            trace!("Register task with the runtime");
            s.register_task(handle.as_raw(), wait_on)
                .map(|_| JoinHandle { handle })
        }) {
            Ok(handle) => join(handle),
            Err(e) => {
                // The task could not be registered so it hasn't begun its execution.
                // Therefore the task can be dropped.
                unsafe { task.assume_init_drop() }
                Err(e)
            }
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
    pub fn spawn<'a, F, R>(
        self,
        f: F,
        wait_on: &[TaskHandle],
    ) -> fimo_module::Result<JoinHandle<Pin<ObjBox<Task<'a, R>>>>>
    where
        F: FnOnce() -> R + Send + 'a,
        R: Send + 'a,
    {
        if !is_worker() {
            return Err(Error::new(
                ErrorKind::FailedPrecondition,
                "spawn() can only be called from an initialized worker thread",
            ));
        }

        let runtime = unsafe { get_runtime() };
        self.spawn_complex::<_, _, _, fn()>(runtime, Some(f), None, wait_on)
    }

    /// Spawns a task onto the task runtime.
    ///
    /// Spawns a task on any of the available workers, where it will run to completion.
    #[inline]
    #[track_caller]
    pub fn spawn_complex<'a, 'b, 'c, Run, F, R, Cleanup>(
        self,
        runtime: &'b Run,
        f: Option<F>,
        cleanup: Option<Cleanup>,
        wait_on: &'c [TaskHandle],
    ) -> fimo_module::Result<JoinHandle<Pin<ObjBox<Task<'a, R>>>>>
    where
        Run: IRuntimeExt + ?Sized,
        F: FnOnce() -> R + Send + 'a,
        Cleanup: FnOnce() + Send + 'a,
        R: Send + 'a,
    {
        trace!("Spawning new task");

        if f.is_none() && std::mem::size_of::<R>() != 0 {
            error!("Tried to spawn an empty task with an invalid return type");
            return Err(Error::new(
                ErrorKind::InvalidArgument,
                "empty tasks with non zst return types are not allowed",
            ));
        }

        let mut task: ObjBox<MaybeUninit<Task<'_, R>>> = ObjBox::new_uninit();

        // fetch the addresses to the inner members, so we can initialize them.
        let raw_ptr = unsafe { std::ptr::addr_of_mut!((*task.as_mut_ptr()).raw) };
        let res_ptr = unsafe { std::ptr::addr_of_mut!((*task.as_mut_ptr()).res) };

        struct AssertSync<T>(*mut MaybeUninit<T>);
        // we know that `T` won't be shared only written, so we can mark it as `Send`.
        unsafe impl<T: Send> Send for AssertSync<T> {}

        let res = unsafe {
            // initialize res and fetch a pointer to the inner result.
            res_ptr.write(UnsafeCell::new(MaybeUninit::uninit()));
            AssertSync((*res_ptr).get())
        };

        let f = if let Some(f) = f {
            let f = move || {
                // write the result directly into the address, knowing that it will live
                // at least as long as the task itself.
                unsafe { res.0.write(MaybeUninit::new(f())) };
            };

            Some(FfiFn::r#box(Box::new(f)))
        } else {
            None
        };

        let cleanup = cleanup.map(|cleanup| FfiFn::r#box(Box::new(cleanup)));

        // initialize the raw field.
        let raw_task = self.inner.build(f, cleanup);
        unsafe { raw_ptr.write(raw_task) };

        // safety: all fields have been initialized.
        let task = unsafe { Pin::new_unchecked(task.assume_init()) };

        runtime.enter_scheduler(move |s, _| unsafe {
            trace!("Register task with the runtime");
            s.register_task(task.as_raw(), wait_on)
                .map(|_| JoinHandle { handle: task })
        })
    }
}

impl Default for Builder {
    #[inline]
    #[track_caller]
    fn default() -> Self {
        Self::new()
    }
}

/// A builder for tasks distributed among multiple workers.
#[derive(Debug)]
pub struct ParallelBuilder {
    unique_workers: bool,
    num_tasks: Option<usize>,
    inner: crate::raw::Builder,
}

impl ParallelBuilder {
    /// Constructs a new `ParallelBuilder`.
    #[inline]
    #[track_caller]
    pub fn new() -> Self {
        Self {
            unique_workers: false,
            num_tasks: None,
            inner: Default::default(),
        }
    }

    /// Names the task.
    #[inline]
    pub fn with_name(mut self, name: String) -> Self {
        self.inner = self.inner.with_name(name);
        self
    }

    /// Assigns a priority to the task.
    ///
    /// A lower [`TaskPriority`] value will lead to a higher priority.
    /// The default priority is `TaskPriority(0)`.
    #[inline]
    pub fn with_priority(mut self, priority: TaskPriority) -> Self {
        self.inner = self.inner.with_priority(priority);
        self
    }

    /// Assigns a start time to the task.
    #[inline]
    pub fn with_start_time(mut self, start_time: SystemTime) -> Self {
        self.inner = self.inner.with_start_time(start_time);
        self
    }

    /// Specifies whether each task is assigned to one unique worker.
    #[inline]
    pub fn unique_workers(mut self, unique: bool) -> Self {
        self.unique_workers = unique;
        self
    }

    /// Specifies how many tasks should be spawned.
    ///
    /// Passing in `None` spawns one task per worker thread.
    #[inline]
    pub fn num_tasks(mut self, tasks: Option<usize>) -> Self {
        self.num_tasks = tasks;
        self
    }

    /// Marks the task as blocked.
    #[inline]
    pub fn blocked(mut self) -> Self {
        self.inner = self.inner.blocked();
        self
    }

    /// Runs a task to completion on the task runtime.
    ///
    /// Blocks the current task until the new task has been completed.
    ///
    /// # Panics
    ///
    /// This function panics if any of the provided function panics.
    /// Can only be called from a worker thread.
    #[inline]
    #[track_caller]
    pub fn block_on<F: FnOnce() -> R + Send + Clone, R: Send>(
        self,
        f: F,
        wait_on: &[TaskHandle],
    ) -> Result<Vec<R>, Error> {
        assert!(is_worker());
        trace!("Blocking on multiple new tasks");

        let runtime = unsafe { get_runtime() };
        let handles: Vec<_> = runtime.enter_scheduler(|s, _| {
            let workers = s.worker_ids();
            let num_tasks = self.num_tasks.unwrap_or(workers.len());

            if num_tasks > workers.len() && self.unique_workers {
                error!(
                    "Can not spawn {} tasks on {} unique workers",
                    num_tasks,
                    workers.len()
                );
                let err = format!(
                    "Can not spawn {} tasks on {} unique workers",
                    num_tasks,
                    workers.len()
                );
                return Err(Error::new(ErrorKind::OutOfRange, err));
            }

            let workers: Vec<_> = if self.unique_workers {
                workers[..num_tasks].iter().map(|w| Some(*w)).collect()
            } else {
                vec![None; num_tasks]
            };

            let mut handles = Vec::with_capacity(num_tasks);
            for (i, worker) in workers.into_iter().enumerate() {
                let mut task: ObjBox<MaybeUninit<Task<'_, R>>> = ObjBox::new_uninit();

                // fetch the addresses to the inner members, so we can initialize them.
                let raw_ptr = unsafe { std::ptr::addr_of_mut!((*task.as_mut_ptr()).raw) };
                let res_ptr = unsafe { std::ptr::addr_of_mut!((*task.as_mut_ptr()).res) };

                struct AssertSync<T>(*mut MaybeUninit<T>);
                // we know that `T` won't be shared only written, so we can mark it as `Send`.
                unsafe impl<T: Send> Send for AssertSync<T> {}

                let res = unsafe {
                    // initialize res and fetch a pointer to the inner result.
                    res_ptr.write(UnsafeCell::new(MaybeUninit::uninit()));
                    AssertSync((*res_ptr).get())
                };

                let f = f.clone();
                let f = move || {
                    // write the result directly into the address, knowing that it will live
                    // at least as long as the task itself.
                    unsafe { res.0.write(MaybeUninit::new(f())) };
                };
                let f = FfiFn::r#box(Box::new(f));

                // initialize the raw field.
                let raw_task = unsafe {
                    self.inner
                        .clone()
                        .with_worker(worker)
                        .extend_name_index(i)
                        .build(Some(f), None)
                };
                unsafe { raw_ptr.write(raw_task) };

                // safety: all fields have been initialized.
                let task = unsafe { Pin::new_unchecked(task.assume_init()) };

                if let Err(e) = unsafe { s.register_task(task.as_raw(), wait_on) } {
                    handles.push(Err(e));
                } else {
                    handles.push(Ok(JoinHandle { handle: task }))
                }
            }
            Ok(handles)
        })?;

        handles
            .into_iter()
            .map(|handle| {
                let handle = handle?;

                match handle.join() {
                    Ok(v) => Ok(v),
                    Err(err) => {
                        // empty errors indicate an aborted task.
                        if let Some(err) = err {
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
                }
            })
            .collect()
    }
}

impl Default for ParallelBuilder {
    #[inline]
    #[track_caller]
    fn default() -> Self {
        Self::new()
    }
}
