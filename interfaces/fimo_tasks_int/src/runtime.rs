//! Task runtime interface.

use crate::raw::{IRawTask, RawTaskInner, TaskHandle, TaskPriority, TaskScheduleStatus, WorkerId};
use fimo_ffi::fn_wrapper::RawFnOnce;
use fimo_ffi::marker::{SendMarker, SendSyncMarker};
use fimo_ffi::object::{CoerceObject, ObjectWrapper};
use fimo_ffi::vtable::{IBase, VTableUpcast};
use fimo_ffi::{fimo_object, fimo_vtable, ObjArc, ObjBox, Object, Optional, SpanInner};
use fimo_module::{impl_vtable, is_object, Error};
use log::Level::Trace;
use log::{log_enabled, trace};
use std::cell::UnsafeCell;
use std::mem::{ManuallyDrop, MaybeUninit};
use std::ops::Deref;
use std::pin::Pin;
use std::ptr::NonNull;
use std::sync::{Arc, Condvar, Mutex};
use std::time::SystemTime;

#[thread_local]
static RUNTIME: std::cell::Cell<Option<*const IRuntime>> = std::cell::Cell::new(None);

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

        // wait for the task to complete and unregister.
        runtime.yield_and_enter(|s, curr| {
            assert!(matches!(s.wait_task_on(curr, raw), Ok(_)));
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
    pub unsafe fn with_worker(mut self, worker: WorkerId) -> Self {
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

/// Result type for the [`ISchedulerVTable`].
pub type ISchedulerResult<T> = fimo_ffi::Result<T, Error>;

fimo_object! {
    /// Interface of a task runtime.
    #![vtable = IRuntimeVTable]
    pub struct IRuntime;
}

/// Result of a wait operation
#[derive(Copy, Clone, Debug, Ord, PartialOrd, Eq, PartialEq, Hash)]
pub enum WaitStatus {
    /// The operation was skipped because the dependency does not exist or refers to itself.
    Skipped,
    /// The wait was successful.
    Completed,
}

impl IRuntime {
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
        &self,
        f: F,
        wait_on: &[TaskHandle],
    ) -> Result<R, Error> {
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
    pub fn block_on_and_enter<F: FnOnce(&IRuntime) -> R + Send, R: Send>(
        &self,
        f: F,
        wait_on: &[TaskHandle],
    ) -> Result<R, Error> {
        // if we are already owned by the runtime we can reuse the existing implementation.
        // otherwise we must implement the join functionality.
        if is_worker() {
            self.block_on(move || f(self), wait_on)
        } else {
            trace!("Entering the runtime");

            // task synchronisation is implemented with condition variables.
            struct CleanupData {
                condvar: Condvar,
                completed: Mutex<bool>,
            }
            is_object! { #![uuid(0x47c0e60c, 0x8cd9, 0x4dd1, 0x8b21, 0x79037a93278c)] CleanupData }
            fimo_vtable! {
                #[derive(Copy, Clone, Debug, Ord, PartialOrd, Eq, PartialEq, Hash)]
                #![marker = SendSyncMarker]
                #![uuid(0x90e5fb0b, 0x8186, 0x4593, 0xb79d, 0x262d145238f2)]
                struct CleanupVTable;
            }
            impl_vtable! { impl CleanupVTable => CleanupData {} }

            // initialize the condvar and hold the mutex until we try to join.
            let data = CleanupData {
                condvar: Default::default(),
                completed: Mutex::new(false),
            };
            let mut completed = data.completed.lock().unwrap();

            let f = move || f(self);
            let cleanup = move |data: Option<NonNull<CleanupData>>| unsafe {
                trace!("Notify owner thread");

                // after locking the mutex we are guaranteed that the owner
                // thread is waiting on the condvar, so we set the flag and notify it.
                let data: &CleanupData = data.unwrap().as_ref();
                let mut completed = data.completed.lock().unwrap();
                *completed = true;
                data.condvar.notify_all();
            };
            let join = |task: &IRawTask, value: &UnsafeCell<MaybeUninit<R>>| {
                trace!("Joining task on owner thread");

                // check if the task has been completed...
                while !*completed {
                    // ... if it isn't the case, wait.
                    completed = data.condvar.wait(completed).unwrap();
                }

                // by this point the task has finished so we can unregister it.
                self.enter_scheduler(|s, _| {
                    assert!(matches!(s.unregister_task(task), Ok(_)));
                });

                // safety: the task is unowned.
                unsafe {
                    let context = task.scheduler_context_mut();
                    match context.schedule_status() {
                        TaskScheduleStatus::Aborted => Err(context.take_panic_data()),
                        TaskScheduleStatus::Finished => Ok(value.get().read().assume_init()),
                        _ => unreachable!(),
                    }
                }
            };

            Builder::new().block_on_complex(f, cleanup, NonNull::from(&data), wait_on, join, self)
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
        &self,
        f: F,
        wait_on: &[TaskHandle],
    ) -> Result<JoinHandle<R, impl RawTaskWrapper<Output = R> + 'static>, Error> {
        Builder::new().spawn(f, wait_on)
    }

    /// Retrieves the id of the current worker.
    #[inline]
    pub fn worker_id(&self) -> Option<WorkerId> {
        trace!("Retrieving id of current worker");

        let (ptr, vtable) = self.into_raw_parts();
        let id = unsafe { (vtable.worker_id)(ptr).into_rust() };

        trace!("Current worker id: {:?}", id);

        id
    }

    /// Acquires a reference to the scheduler.
    ///
    /// The task will block, until the scheduler can be acquired.
    ///
    /// # Deadlock
    ///
    /// Trying to access the scheduler by other means than
    /// the provided reference may result in a deadlock.
    #[inline]
    pub fn enter_scheduler<F: FnOnce(&mut IScheduler, Option<&IRawTask>) -> R, R>(
        &self,
        f: F,
    ) -> R {
        trace!("Entering scheduler");
        let mut res = MaybeUninit::uninit();

        {
            let res = &mut res;
            let wrapper = move |s: NonNull<IScheduler>, t: Optional<NonNull<IRawTask>>| unsafe {
                trace!("Scheduler entered");
                res.write(f(&mut *s.as_ptr(), t.into_rust().map(|t| t.as_ref())));
                trace!("Exiting scheduler");
            };
            let mut wrapper = MaybeUninit::new(wrapper);

            unsafe {
                let wrapper = RawFnOnce::new(&mut wrapper);
                let (ptr, vtable) = self.into_raw_parts();
                (vtable.enter_scheduler)(ptr, wrapper);
            }
        }

        trace!("Scheduler exited");
        unsafe { res.assume_init() }
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
    pub fn yield_now(&self) {
        self.yield_and_enter(|_, _| {})
    }

    /// Yields the current task to the runtime.
    ///
    /// Yields the current task to the runtime, allowing other tasks to be
    /// run on the current worker. The task won't resume until the instant
    /// `until` has passed.
    ///
    /// # Panics
    ///
    /// Can only be called from a worker thread.
    #[inline]
    pub fn yield_until(&self, until: SystemTime) {
        self.yield_and_enter(move |_, curr| {
            trace!("Yielding task {:?} until {:?}", curr.resolved_name(), until);
            // safety: we are inside the scheduler so it is safe.
            unsafe { curr.scheduler_context_mut().set_resume_time(until) }
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
    /// Can only be called from a worker thread.
    #[inline]
    pub fn yield_and_enter<F: FnOnce(&mut IScheduler, &IRawTask) -> R + Send, R: Send>(
        &self,
        f: F,
    ) -> R {
        assert!(is_worker());
        trace!("Yielding to scheduler");
        let mut res = MaybeUninit::uninit();

        {
            let res = &mut res;
            let wrapper = move |s: NonNull<IScheduler>, t: NonNull<IRawTask>| unsafe {
                trace!("Yielded to scheduler");
                res.write(f(&mut *s.as_ptr(), t.as_ref()));
                trace!("Resuming task");
            };
            let mut wrapper = MaybeUninit::new(wrapper);

            unsafe {
                let wrapper = RawFnOnce::new(&mut wrapper);
                let (ptr, vtable) = self.into_raw_parts();
                (vtable.yield_and_enter)(ptr, wrapper);
            }
        }

        trace!("Task resumed");
        unsafe { res.assume_init() }
    }

    /// Blocks the current task indefinitely.
    ///
    /// # Panics
    ///
    /// Can only be called from a worker thread.
    #[inline]
    pub fn block_now(&self) {
        trace!("Requesting block of current task");

        self.yield_and_enter(|_, curr| {
            curr.scheduler_context().request_block();
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
    pub unsafe fn abort_now(&self) -> ! {
        trace!("Aborting current task");
        struct Abort {}
        std::panic::resume_unwind(Box::new(Abort {}))
    }

    /// Blocks the current task, until another task completes.
    ///
    /// The current task will yield it's execution, until the other task completes.
    /// Returns whether the task waited on the other task.
    /// Passing the handle to the current task will always return `Ok(WaitStatus::Skipped)`.
    /// Otherwise this function always guarantees, that the other task has completed.
    ///
    /// # Panics
    ///
    /// Can only be called from a worker thread.
    #[inline]
    pub fn wait_on(&self, handle: TaskHandle) -> Result<WaitStatus, Error> {
        trace!("Wait until task-id {} completes", handle);

        self.yield_and_enter(move |s, curr| {
            // safety: the task is provided from the scheduler, so it is registered.
            if unsafe { curr.scheduler_context().handle().assume_init() } != handle {
                // safety: a wait operation does not invalidate the task
                // or cause race-conditions.
                if let Ok(wait_on) = unsafe { s.find_task_unbound(handle) } {
                    s.wait_task_on(curr, wait_on).map(|_| WaitStatus::Completed)
                } else {
                    trace!("The task-id {} was not found, skipping wait", handle);
                    Ok(WaitStatus::Skipped)
                }
            } else {
                trace!("The task-id {} refers to itself, skipping wait", handle);
                Ok(WaitStatus::Skipped)
            }
        })
    }
}

fimo_vtable! {
    /// VTable of a [`IRuntime`].
    #[derive(Copy, Clone, Debug, Ord, PartialOrd, Eq, PartialEq, Hash)]
    #![marker = SendSyncMarker]
    #![uuid(0x095a88ff, 0xf45a, 0x4cf8, 0xa8f2, 0xe18eb028a7de)]
    pub struct IRuntimeVTable {
        /// Retrieves the id of the current worker.
        pub worker_id: unsafe extern "C" fn(*const ()) -> Optional<WorkerId>,
        /// Acquires a reference to the scheduler.
        ///
        /// The task will block, until the scheduler can be acquired.
        ///
        /// # Deadlock
        ///
        /// Trying to access the scheduler by other means than
        /// the provided reference may result in a deadlock.
        #[allow(clippy::type_complexity)]
        pub enter_scheduler: unsafe extern "C" fn(
            *const (),
            RawFnOnce<(NonNull<IScheduler>, Optional<NonNull<IRawTask>>), ()>
        ),
        /// Yields the current task to the runtime.
        ///
        /// Yields the current task to the runtime, allowing other tasks to be
        /// run on the current worker. On the next run of the scheduler it will call
        /// the provided function.
        pub yield_and_enter: unsafe extern "C" fn(
            *const (),
            RawFnOnce<(NonNull<IScheduler>, NonNull<IRawTask>), (), SendMarker>
        )
    }
}

fimo_object! {
    /// Interface of a scheduler.
    #![vtable = ISchedulerVTable]
    pub struct IScheduler;
}

impl IScheduler {
    /// Fetches the id's of all available workers.
    #[inline]
    pub fn worker_ids(&self) -> &[WorkerId] {
        trace!("Enumerating task runtime worker id's");

        let (ptr, vtable) = self.into_raw_parts();
        let workers: &[WorkerId] = unsafe { (vtable.worker_ids)(ptr).into() };

        trace!("Found {} workers: {:?}", workers.len(), workers);

        workers
    }

    /// Searches for a registered task.
    #[inline]
    pub fn find_task(&self, handle: TaskHandle) -> Result<&IRawTask, Error> {
        unsafe { self.find_task_unbound(handle) }
    }

    /// Searches for a registered task.
    ///
    /// # Safety
    ///
    /// The reference may outlive the task, if it is stored or the scheduler is modified.
    #[inline]
    pub unsafe fn find_task_unbound<'a>(&self, handle: TaskHandle) -> Result<&'a IRawTask, Error> {
        trace!("Searching for task: {}", handle);

        let (ptr, vtable) = self.into_raw_parts();
        let res = (vtable.find_task)(ptr, handle)
            .into_rust()
            .map(|t| t.as_ref());

        if log_enabled!(Trace) {
            match &res {
                Ok(task) => trace!(
                    "Found task name: {:?}, id: {}",
                    task.resolved_name(),
                    handle
                ),
                Err(err) => trace!("No task found with error: {}", err),
            }
        }

        res
    }

    /// Registers a task with the runtime for execution.
    ///
    /// # Safety
    ///
    /// The pointed to value must be kept alive until the runtime releases it.
    #[inline]
    pub unsafe fn register_task(
        &mut self,
        task: &IRawTask,
        deps: &[TaskHandle],
    ) -> Result<(), Error> {
        trace!("Registering task: {:?}", task.resolved_name());

        let (ptr, vtable) = self.into_raw_parts_mut();
        let res = (vtable.register_task)(ptr, task.into(), deps.into()).into_rust();

        if log_enabled!(Trace) {
            let name = task.resolved_name();

            match &res {
                Ok(_) => {
                    trace!(
                        "Task {:?} assigned to id: {}",
                        name,
                        task.scheduler_context().handle().assume_init()
                    )
                }
                Err(err) => trace!("Task not registered, error: {}", err),
            }
        }

        res
    }

    /// Unregisters a task from the task runtime.
    pub fn unregister_task(&mut self, task: &IRawTask) -> Result<(), Error> {
        trace!("Unregistering task: {:?}", task.resolved_name());

        let (ptr, vtable) = self.into_raw_parts_mut();
        let res = unsafe { (vtable.unregister_task)(ptr, task.into()).into_rust() };

        if log_enabled!(Trace) {
            let name = task.resolved_name();

            match &res {
                Ok(_) => trace!("Task {:?} unregistered", name),
                Err(err) => trace!("Task not unregistered, error: {}", err),
            }
        }

        res
    }

    /// Registers a block for a task until the dependency completes.
    ///
    /// A task may not wait on itself.
    ///
    /// # Note
    ///
    /// Does not guarantee that the task will wait immediately if it is already scheduled.
    #[inline]
    pub fn wait_task_on(&mut self, task: &IRawTask, wait_on: &IRawTask) -> Result<(), Error> {
        trace!(
            "Task {:?} registering wait on {:?}",
            task.resolved_name(),
            wait_on.resolved_name()
        );

        let (ptr, vtable) = self.into_raw_parts_mut();
        let res = unsafe { (vtable.wait_task_on)(ptr, task.into(), wait_on.into()).into_rust() };

        if log_enabled!(Trace) {
            match &res {
                Ok(_) => {
                    trace!(
                        "Task {:?} waiting on {:?}",
                        task.resolved_name(),
                        wait_on.resolved_name()
                    );
                }
                Err(err) => trace!("Waiting not successful, error: {}", err),
            }
        }

        res
    }

    /// Wakes up one task.
    ///
    /// Wakes up the task with the highest priority, that is waiting on provided task to finish.
    /// Returns the number of remaining waiters, if the operation was successful and `Ok(None)`
    /// if there were no waiters.
    ///
    /// # Safety
    ///
    /// A waiting task may require that the task fully finishes before
    /// resuming execution. This function is mainly intended to be
    /// used for the implementation of condition variables.
    #[inline]
    pub unsafe fn notify_one(&mut self, task: &IRawTask) -> Result<Option<usize>, Error> {
        trace!("Notifying one waiter of task: {:?}", task.resolved_name());

        let (ptr, vtable) = self.into_raw_parts_mut();
        let res = (vtable.notify_one)(ptr, task.into())
            .into_rust()
            .map(|n| n.into_rust());

        if log_enabled!(Trace) {
            match &res {
                Ok(None) => trace!("No task is waiting on task: {:?}", task.resolved_name()),
                Ok(Some(n)) => {
                    trace!(
                        "Notified one task from {:?}, {} waiters remaining",
                        task.resolved_name(),
                        n
                    )
                }
                Err(err) => trace!("Notify failed, error: {}", err),
            }
        }

        res
    }

    /// Wakes up all tasks.
    ///
    /// Wakes up all waiting tasks. Returns the number of tasks that were woken up.
    ///
    /// # Safety
    ///
    /// A waiting task may require that the task fully finishes before
    /// resuming execution. This function is mainly intended to be
    /// used for the implementation of condition variables.
    #[inline]
    pub unsafe fn notify_all(&mut self, task: &IRawTask) -> Result<usize, Error> {
        trace!("Notifying all waiters of task: {:?}", task.resolved_name());

        let (ptr, vtable) = self.into_raw_parts_mut();
        let res = (vtable.notify_all)(ptr, task.into()).into_rust();

        if log_enabled!(Trace) {
            match &res {
                Ok(n) => trace!("Notified {} tasks from {:?}", n, task.resolved_name()),
                Err(err) => trace!("Notify failed, error: {}", err),
            }
        }

        res
    }

    /// Unblocks a blocked task.
    #[inline]
    pub fn unblock_task(&mut self, task: &IRawTask) -> Result<(), Error> {
        trace!("Unblocking task {:?}", task.resolved_name());

        let (ptr, vtable) = self.into_raw_parts_mut();
        let res = unsafe { (vtable.unblock_task)(ptr, task.into()).into_rust() };

        if log_enabled!(Trace) {
            match &res {
                Ok(_) => trace!("Task {:?} unblocked", task.resolved_name()),
                Err(err) => trace!(
                    "Unblock of {:?} failed, error: {}",
                    task.resolved_name(),
                    err
                ),
            }
        }

        res
    }
}

fimo_vtable! {
    /// VTable of a [`IScheduler`].
    #[derive(Copy, Clone, Debug, Ord, PartialOrd, Eq, PartialEq, Hash)]
    #![marker = SendMarker]
    #![uuid(0x095a88ff, 0xf45a, 0x4cf8, 0xa8f2, 0xe18eb028a7de)]
    pub struct ISchedulerVTable {
        /// Fetches the id's of all available workers.
        pub worker_ids: unsafe extern "C" fn(*const ()) -> SpanInner<WorkerId, false>,
        /// Searches for a registered task.
        ///
        /// # Warning
        ///
        /// The task may be destroyed upon exiting the scheduler.
        pub find_task:
            unsafe extern "C" fn(*const (), TaskHandle) -> ISchedulerResult<NonNull<IRawTask>>,
        /// Registers a task with the runtime for execution.
        ///
        /// # Safety
        ///
        /// The pointed to value must be kept alive until the runtime releases it.
        pub register_task: unsafe extern "C" fn(
            *mut (),
            NonNull<IRawTask>,
            SpanInner<TaskHandle, false>,
        ) -> ISchedulerResult<()>,
        /// Unregisters a task from the task runtime.
        pub unregister_task: unsafe extern "C" fn(
            *mut (),
            NonNull<IRawTask>,
        ) -> ISchedulerResult<()>,
        /// Blocks a task until the dependency completes.
        ///
        /// A task may not wait on itself.
        ///
        /// # Note
        ///
        /// Does not guarantee that the task will wait immediately if it is already scheduled.
        pub wait_task_on:
            unsafe extern "C" fn(*mut (), NonNull<IRawTask>, NonNull<IRawTask>) -> ISchedulerResult<()>,
        /// Wakes up one task.
        ///
        /// Wakes up the task with the highest priority, that is waiting on provided task to finish.
        /// Returns the number of remaining waiters, if the operation was successful and `Ok(None)`
        /// if there were no waiters.
        ///
        /// # Safety
        ///
        /// A waiting task may require that the task fully finishes before
        /// resuming execution. This function is mainly intended to be
        /// used for the implementation of condition variables.
        pub notify_one:
            unsafe extern "C" fn(*mut (), NonNull<IRawTask>) -> ISchedulerResult<Optional<usize>>,
        /// Wakes up all tasks.
        ///
        /// Wakes up all waiting tasks. Returns the number of tasks that were woken up.
        ///
        /// # Safety
        ///
        /// A waiting task may require that the task fully finishes before
        /// resuming execution. This function is mainly intended to be
        /// used for the implementation of condition variables.
        pub notify_all:
            unsafe extern "C" fn(*mut(), NonNull<IRawTask>) -> ISchedulerResult<usize>,
        /// Unblocks a blocked task.
        pub unblock_task:
            unsafe extern "C" fn(*mut (), NonNull<IRawTask>) -> ISchedulerResult<()>,
    }
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
pub unsafe fn get_runtime() -> &'static IRuntime {
    &*RUNTIME.get().unwrap_unchecked()
}

/// Provides the runtime to the current worker.
///
/// # Safety
///
/// May not be called with multiple runtimes.
pub unsafe fn init_runtime(runtime: *const IRuntime) {
    RUNTIME.set(Some(runtime))
}
