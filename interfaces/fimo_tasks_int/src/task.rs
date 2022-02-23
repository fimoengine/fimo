//! Utilities for handling tasks.
//!
use crate::raw::{IRawTask, RawTaskInner, TaskPriority, TaskScheduleStatus, WorkerId};
use crate::runtime::{get_runtime, is_worker};
use crate::{IRuntime, TaskHandle};
use fimo_ffi::fn_wrapper::RawFnOnce;
use fimo_ffi::marker::{SendMarker, SendSyncMarker};
use fimo_ffi::object::{CoerceObject, ObjectWrapper};
use fimo_ffi::vtable::{IBase, VTableUpcast};
use fimo_ffi::{ObjArc, ObjBox, Object, Optional};
use fimo_module::{Error, ErrorKind};
use log::{error, trace};
use std::cell::UnsafeCell;
use std::mem::{ManuallyDrop, MaybeUninit};
use std::ops::Deref;
use std::pin::Pin;
use std::ptr::NonNull;
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
pub struct JoinHandle<T, R: RawTaskWrapper<Output = T>> {
    handle: R,
}

impl<T, R: RawTaskWrapper<Output = T>> JoinHandle<T, R> {
    /// Fetches the handle of the task.
    #[inline]
    pub fn handle(&self) -> TaskHandle {
        // safety: we own the task and know that it is registered.
        unsafe {
            self.handle
                .as_raw()
                .scheduler_context()
                .handle()
                .assume_init()
        }
    }

    /// Fetches a reference to the contained raw task.
    #[inline]
    pub fn as_raw(&self) -> &IRawTask {
        self.handle.as_raw()
    }

    /// Returns a reference to the output of the task.
    ///
    /// # Safety
    ///
    /// This function is only safe, if the task has been completed successfully.
    #[inline]
    pub unsafe fn assume_completed_ref(&self) -> &T {
        self.handle.peek_output()
    }

    /// Returns a mutable reference to the output of the task.
    ///
    /// # Safety
    ///
    /// This function is only safe, if the task has been completed successfully.
    #[inline]
    pub unsafe fn assume_completed_mut(&mut self) -> &mut T {
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
    pub unsafe fn assume_completed_read(&self) -> T {
        self.handle.read_output()
    }

    /// Waits for the associated task to finish.
    ///
    /// # Panics
    ///
    /// This function may panic if a task tries to join itself.
    pub fn join(mut self) -> Result<T, Option<ObjBox<Object<IBase<SendMarker>>>>> {
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
    unsafe fn join_ref(&mut self) -> Result<T, Option<ObjBox<Object<IBase<SendMarker>>>>> {
        let runtime = get_runtime();
        let raw = self.handle.as_raw();

        // wait for the task to complete.
        assert!(matches!(
            runtime.wait_on(raw.scheduler_context().handle().assume_init()),
            Ok(_)
        ));

        // unregister the completed task.
        runtime.enter_scheduler(|s, _| {
            assert!(matches!(s.unregister_task(raw), Ok(_)));
        });

        // safety: the task is unowned.
        let context = raw.scheduler_context_mut();
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
    pub fn wait(&self) -> Option<&T> {
        debug_assert!(is_worker());
        trace!(
            "Waiting on task-id {}, name {:?}",
            self.handle(),
            self.as_raw().resolved_name()
        );

        let runtime = unsafe { get_runtime() };
        assert!(matches!(runtime.wait_on(self.handle()), Ok(_)));

        let context = self.as_raw().scheduler_context();
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
    pub fn wait_mut(&mut self) -> Option<&mut T> {
        debug_assert!(is_worker());
        trace!(
            "Waiting on task-id {}, name {:?}",
            self.handle(),
            self.as_raw().resolved_name()
        );

        let runtime = unsafe { get_runtime() };
        assert!(matches!(runtime.wait_on(self.handle()), Ok(_)));

        let context = self.as_raw().scheduler_context();
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

        self.as_raw().scheduler_context().request_block();
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

        self.as_raw().scheduler_context().request_abort();
    }
}

impl<T, R: RawTaskWrapper<Output = T>> Drop for JoinHandle<T, R> {
    fn drop(&mut self) {
        trace!(
            "Dropping task-id {}, name {:?}",
            self.handle(),
            self.as_raw().resolved_name()
        );
        unsafe { assert!(matches!(self.join_ref(), Ok(_))) };
    }
}

#[derive(Debug)]
struct Task<T> {
    raw: RawTaskInner,
    res: UnsafeCell<MaybeUninit<T>>,
}

/// A wrapper around a raw task.
pub trait RawTaskWrapper: Deref {
    /// Output type of the task.
    type Output;

    /// Extracts a reference to the task.
    fn as_raw(&self) -> &IRawTask;

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
        $(impl<T> RawTaskWrapper for $type {
            type Output = T;

            #[inline]
            fn as_raw(&self) -> &IRawTask {
                IRawTask::from_object(self.raw.coerce_obj())
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

as_raw_impl! {&Task<T>}
as_raw_impl! {Pin<Box<Task<T>>>, Pin<ObjBox<Task<T>>>}
as_raw_impl! {Pin<Arc<Task<T>>>, Pin<ObjArc<Task<T>>>}

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
    pub fn block_on<F: FnOnce() -> R + Send, R: Send>(
        self,
        f: F,
        wait_on: &[TaskHandle],
    ) -> Result<R, Error> {
        assert!(is_worker());
        trace!("Blocking on new task");

        let mut task: MaybeUninit<Task<R>> = MaybeUninit::uninit();

        // fetch the addresses to the inner members, so we can initialize them.
        let raw_ptr = unsafe { std::ptr::addr_of_mut!((*task.as_mut_ptr()).raw) };
        let res_ptr = unsafe { std::ptr::addr_of_mut!((*task.as_mut_ptr()).res) };

        struct AssertSync<T>(*mut MaybeUninit<T>);
        // we know that `T` won't be shared only written, so we can mark it as `Send`.
        unsafe impl<T: Send> Send for AssertSync<T> {}

        let res = unsafe {
            // initialize res and fetch a pointer to the inner result.
            res_ptr.write(UnsafeCell::new(MaybeUninit::uninit()));
            AssertSync(&mut *(*res_ptr).get())
        };

        let f = move || {
            // write the result directly into the address, knowing that it will live
            // at least as long as the task itself.
            unsafe { res.0.write(MaybeUninit::new(f())) };
        };
        let mut f = MaybeUninit::new(f);
        // safety: we know that f is valid until the raw fn is called.
        let f = unsafe { RawFnOnce::new(&mut f) };

        // initialize the raw field.
        let raw_task = self.inner.build(Some(f), None, None);
        unsafe { raw_ptr.write(raw_task) };

        // safety: all fields have been initialized.
        let task = unsafe { task.assume_init() };
        let handle = &task;

        let runtime = unsafe { get_runtime() };
        let handle = runtime.enter_scheduler(move |s, _| unsafe {
            trace!("Register task with the runtime");
            s.register_task(handle.as_raw(), wait_on)
                .map(|_| JoinHandle { handle })
        })?;

        match handle.join() {
            Ok(v) => Ok(v),
            Err(err) => {
                // empty errors indicate an aborted task.
                if let Some(err) = err {
                    use crate::raw::IRustPanicData;

                    // a runtime written in rust can choose to wrap the native
                    // panic into a `IRustPanicData`.
                    if let Ok(err) = ObjBox::try_cast::<IRustPanicData>(err) {
                        let err = IRustPanicData::take_rust_panic(err);
                        std::panic::resume_unwind(err)
                    }
                }

                panic!("Unknown panic!")
            }
        }
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
    pub fn block_on_complex<
        F: FnOnce() -> R + Send,
        R: Send,
        C: FnOnce(Option<NonNull<O>>) + Send + 'static,
        O: CoerceObject<V> + 'static,
        V: VTableUpcast<IBase<SendSyncMarker>>,
        J: FnOnce(
            &IRawTask,
            &UnsafeCell<MaybeUninit<R>>,
        ) -> Result<R, Option<ObjBox<Object<IBase<SendMarker>>>>>,
    >(
        self,
        f: F,
        cleanup: C,
        data: NonNull<O>,
        wait_on: &[TaskHandle],
        join: J,
        runtime: &IRuntime,
    ) -> Result<R, Error> {
        trace!("Blocking on new task");

        let mut task: MaybeUninit<Task<R>> = MaybeUninit::uninit();

        // fetch the addresses to the inner members, so we can initialize them.
        let raw_ptr = unsafe { std::ptr::addr_of_mut!((*task.as_mut_ptr()).raw) };
        let res_ptr = unsafe { std::ptr::addr_of_mut!((*task.as_mut_ptr()).res) };

        struct AssertSync<T>(*mut MaybeUninit<T>);
        // we know that `T` won't be shared only written, so we can mark it as `Send`.
        unsafe impl<T: Send> Send for AssertSync<T> {}

        let res = unsafe {
            // initialize res and fetch a pointer to the inner result.
            res_ptr.write(UnsafeCell::new(MaybeUninit::uninit()));
            AssertSync(&mut *(*res_ptr).get())
        };

        let f = move || {
            // write the result directly into the address, knowing that it will live
            // at least as long as the task itself.
            unsafe { res.0.write(MaybeUninit::new(f())) };
        };
        let mut f = MaybeUninit::new(f);
        // safety: we know that f is valid until the raw fn is called.
        let f = unsafe { RawFnOnce::new(&mut f) };

        let cleanup = move |data: Optional<NonNull<Object<IBase<SendSyncMarker>>>>| {
            if let Some(data) = data.into_rust() {
                let data: *const O = Object::try_cast_obj_raw(data.as_ptr()).unwrap();
                let data: NonNull<O> = unsafe { NonNull::new_unchecked(data as *mut _) };
                cleanup(Some(data));
            } else {
                cleanup(None);
            }
        };
        let mut cleanup = MaybeUninit::new(cleanup);
        let cleanup = unsafe { RawFnOnce::new(&mut cleanup) };

        let data = O::coerce_obj_raw(data.as_ptr());
        let data = Object::cast_super_raw(data);
        let data = unsafe { NonNull::new_unchecked(data as *mut _) };

        // initialize the raw field.
        let raw_task = self.inner.build(Some(f), Some(cleanup), Some(data));
        unsafe { raw_ptr.write(raw_task) };

        // safety: all fields have been initialized.
        let task = unsafe { task.assume_init() };
        let handle = &task;

        let handle = runtime.enter_scheduler(move |s, _| unsafe {
            trace!("Register task with the runtime");
            s.register_task(handle.as_raw(), wait_on)
                .map(|_| JoinHandle { handle })
        })?;
        let handle = ManuallyDrop::new(handle);

        match join(handle.as_raw(), &handle.handle.res) {
            Ok(v) => Ok(v),
            Err(err) => {
                // empty errors indicate an aborted task.
                if let Some(err) = err {
                    use crate::raw::IRustPanicData;

                    // a runtime written in rust can choose to wrap the native
                    // panic into a `IRustPanicData`.
                    if let Ok(err) = ObjBox::try_cast::<IRustPanicData>(err) {
                        let err = IRustPanicData::take_rust_panic(err);
                        std::panic::resume_unwind(err)
                    }
                }

                panic!("Unknown panic!")
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
    pub fn spawn<F: FnOnce() -> R + Send + 'static, R: Send + 'static>(
        self,
        f: F,
        wait_on: &[TaskHandle],
    ) -> Result<JoinHandle<R, impl RawTaskWrapper<Output = R> + 'static>, Error> {
        assert!(is_worker());
        trace!("Spawning new task");

        let mut task: ObjBox<MaybeUninit<Task<R>>> = ObjBox::new_uninit();

        // fetch the addresses to the inner members, so we can initialize them.
        let raw_ptr = unsafe { std::ptr::addr_of_mut!((*task.as_mut_ptr()).raw) };
        let res_ptr = unsafe { std::ptr::addr_of_mut!((*task.as_mut_ptr()).res) };

        struct AssertSync<T>(*mut MaybeUninit<T>);
        // we know that `T` won't be shared only written, so we can mark it as `Send`.
        unsafe impl<T: Send> Send for AssertSync<T> {}

        let res = unsafe {
            // initialize res and fetch a pointer to the inner result.
            res_ptr.write(UnsafeCell::new(MaybeUninit::uninit()));
            AssertSync(&mut *(*res_ptr).get())
        };

        let f = move || {
            // write the result directly into the address, knowing that it will live
            // at least as long as the task itself.
            unsafe { res.0.write(MaybeUninit::new(f())) };
        };
        let f = RawFnOnce::new_boxed(Box::new(f));

        // initialize the raw field.
        let raw_task = self.inner.build(Some(f), None, None);
        unsafe { raw_ptr.write(raw_task) };

        // safety: all fields have been initialized.
        let task = unsafe { Pin::new_unchecked(task.assume_init()) };

        let runtime = unsafe { get_runtime() };
        runtime.enter_scheduler(move |s, _| unsafe {
            trace!("Register task with the runtime");
            s.register_task(task.as_raw(), wait_on)
                .map(|_| JoinHandle { handle: task })
        })
    }

    /// Spawns a task onto the task runtime.
    ///
    /// Spawns a task on any of the available workers, where it will run to completion.
    #[inline]
    #[track_caller]
    pub fn spawn_complex<
        F: FnOnce() -> R + Send + 'static,
        R: Send + 'static,
        C: FnOnce(Option<NonNull<O>>) + Send + 'static,
        O: CoerceObject<V> + 'static,
        V: VTableUpcast<IBase<SendSyncMarker>>,
    >(
        self,
        runtime: &IRuntime,
        f: F,
        cleanup: C,
        data: NonNull<O>,
        wait_on: &[TaskHandle],
    ) -> Result<JoinHandle<R, impl RawTaskWrapper<Output = R> + 'static>, Error> {
        trace!("Spawning new task");

        let mut task: ObjBox<MaybeUninit<Task<R>>> = ObjBox::new_uninit();

        // fetch the addresses to the inner members, so we can initialize them.
        let raw_ptr = unsafe { std::ptr::addr_of_mut!((*task.as_mut_ptr()).raw) };
        let res_ptr = unsafe { std::ptr::addr_of_mut!((*task.as_mut_ptr()).res) };

        struct AssertSync<T>(*mut MaybeUninit<T>);
        // we know that `T` won't be shared only written, so we can mark it as `Send`.
        unsafe impl<T: Send> Send for AssertSync<T> {}

        let res = unsafe {
            // initialize res and fetch a pointer to the inner result.
            res_ptr.write(UnsafeCell::new(MaybeUninit::uninit()));
            AssertSync(&mut *(*res_ptr).get())
        };

        let f = move || {
            // write the result directly into the address, knowing that it will live
            // at least as long as the task itself.
            unsafe { res.0.write(MaybeUninit::new(f())) };
        };
        let f = RawFnOnce::new_boxed(Box::new(f));

        let cleanup = move |data: Optional<NonNull<Object<IBase<SendSyncMarker>>>>| {
            if let Some(data) = data.into_rust() {
                let data: *const O = Object::try_cast_obj_raw(data.as_ptr()).unwrap();
                let data: NonNull<O> = unsafe { NonNull::new_unchecked(data as *mut _) };
                cleanup(Some(data));
            } else {
                cleanup(None);
            }
        };
        let cleanup = RawFnOnce::new_boxed(Box::new(cleanup));

        let data = O::coerce_obj_raw(data.as_ptr());
        let data = Object::cast_super_raw(data);
        let data = unsafe { NonNull::new_unchecked(data as *mut _) };

        // initialize the raw field.
        let raw_task = self.inner.build(Some(f), Some(cleanup), Some(data));
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

        let mut funcs = self.num_tasks.map_or(Vec::new(), Vec::with_capacity);
        let mut tasks = self.num_tasks.map_or(Vec::new(), Vec::with_capacity);
        let mut task_workers = self.num_tasks.map_or(Vec::new(), Vec::with_capacity);

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

            if self.unique_workers {
                funcs.reserve(num_tasks);
                tasks.reserve(num_tasks);
                task_workers.reserve(num_tasks);
                task_workers.extend(workers[..num_tasks].iter().map(|w| Some(*w)));
            } else {
                task_workers.resize(num_tasks, None);
            }

            Ok((0..num_tasks)
                .map(|i| {
                    tasks.push(MaybeUninit::uninit());
                    let task: &mut MaybeUninit<Task<R>> = &mut tasks[i];
                    let worker = task_workers[i];

                    // fetch the addresses to the inner members, so we can initialize them.
                    let raw_ptr = unsafe { std::ptr::addr_of_mut!((*task.as_mut_ptr()).raw) };
                    let res_ptr = unsafe { std::ptr::addr_of_mut!((*task.as_mut_ptr()).res) };

                    struct AssertSync<T>(*mut MaybeUninit<T>);
                    // we know that `T` won't be shared only written, so we can mark it as `Send`.
                    unsafe impl<T: Send> Send for AssertSync<T> {}

                    let res = unsafe {
                        // initialize res and fetch a pointer to the inner result.
                        res_ptr.write(UnsafeCell::new(MaybeUninit::uninit()));
                        AssertSync(&mut *(*res_ptr).get())
                    };

                    let f = f.clone();
                    let f = move || {
                        // write the result directly into the address, knowing that it will live
                        // at least as long as the task itself.
                        unsafe { res.0.write(MaybeUninit::new(f())) };
                    };
                    let f = MaybeUninit::new(f);
                    funcs.push(f);

                    // safety: we know that f is valid until the raw fn is called.
                    let f = unsafe { RawFnOnce::new(&mut funcs[i]) };

                    // initialize the raw field.
                    let raw_task = unsafe {
                        self.inner
                            .clone()
                            .with_worker(worker)
                            .extend_name_index(i)
                            .build(Some(f), None, None)
                    };
                    unsafe { raw_ptr.write(raw_task) };

                    // safety: all fields have been initialized.
                    let task = unsafe { task.assume_init_ref() };

                    trace!("Register task with the runtime");
                    unsafe { s.register_task(task.as_raw(), wait_on) }
                })
                .collect())
        })?;

        handles
            .into_iter()
            .enumerate()
            .map(|(i, handle)| {
                let handle = handle.map(|_| unsafe {
                    JoinHandle {
                        handle: tasks[i].assume_init_ref(),
                    }
                })?;

                match handle.join() {
                    Ok(v) => Ok(v),
                    Err(err) => {
                        // empty errors indicate an aborted task.
                        if let Some(err) = err {
                            use crate::raw::IRustPanicData;

                            // a runtime written in rust can choose to wrap the native
                            // panic into a `IRustPanicData`.
                            if let Ok(err) = ObjBox::try_cast::<IRustPanicData>(err) {
                                let err = IRustPanicData::take_rust_panic(err);
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
