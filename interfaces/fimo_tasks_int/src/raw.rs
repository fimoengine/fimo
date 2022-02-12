//! Raw tasks primitives.

use atomic::Atomic;
use fimo_ffi::fn_wrapper::RawFnOnce;
use fimo_ffi::marker::{SendMarker, SendSyncMarker};
use fimo_ffi::object::{CoerceObjectMut, ObjectWrapper};
use fimo_ffi::vtable::IBase;
use fimo_ffi::{fimo_object, fimo_vtable, impl_vtable, is_object, Optional, StrInner};
use fimo_ffi::{ConstStr, ObjBox, Object};
use std::cell::UnsafeCell;
use std::marker::PhantomPinned;
use std::mem::MaybeUninit;
use std::ptr::NonNull;
use std::sync::atomic::Ordering;
use std::time::{Duration, SystemTime};

/// Handle to a task.
#[repr(C)]
#[derive(Debug, Copy, Clone, Hash, Ord, PartialOrd, PartialEq, Eq)]
pub struct TaskHandle {
    /// Id of the task.
    pub id: usize,
    /// Generation of the id.
    pub generation: usize,
}

/// Priority of a task.
#[repr(transparent)]
#[derive(Debug, Copy, Clone, Hash, Ord, PartialOrd, PartialEq, Eq)]
pub struct TaskPriority(pub isize);

/// Id of a worker.
#[repr(transparent)]
#[derive(Debug, Copy, Clone, Hash, Ord, PartialOrd, PartialEq, Eq)]
pub struct WorkerId(usize);

impl WorkerId {
    /// Id of an invalid worker.
    pub const INVALID_WORKER_ID: usize = usize::MAX;

    /// Creates a new worker id.
    ///
    /// Accepts all possible id's except [`WorkerId::INVALID_WORKER_ID`].
    pub fn new(id: usize) -> Option<WorkerId> {
        if id == Self::INVALID_WORKER_ID {
            None
        } else {
            Some(WorkerId(id))
        }
    }
}

/// Run status of a task.
#[repr(u32)]
#[derive(Debug, Copy, Clone, Hash, Ord, PartialOrd, PartialEq, Eq)]
pub enum TaskRunStatus {
    /// The task is not running.
    Idle,
    /// The task is running.
    Running,
    /// The task has been completed.
    Completed,
}

/// Scheduler status of a task.
#[repr(u32)]
#[derive(Debug, Copy, Clone, Hash, Ord, PartialOrd, PartialEq, Eq)]
pub enum TaskScheduleStatus {
    /// The task is blocked and can not be resumed.
    Blocked,
    /// The task is waiting on another task.
    Waiting,
    /// The task is runnable.
    Runnable,
    /// The task is scheduled.
    Scheduled,
    /// The task is waiting for processing from the scheduler.
    Processing,
    /// The task has been aborted.
    Aborted,
    /// The task has finished.
    Finished,
}

#[derive(Debug)]
pub(crate) struct RawTaskInner {
    info: TaskInfo,
    data: UnsafeCell<SchedulerContextInner>,
}

unsafe impl Sync for RawTaskInner where SchedulerContextInner: Sync {}

is_object! { #![uuid(0xeb91ee4a, 0x22d2, 0x4b91, 0x9e06, 0x0994f0d79b0f)] RawTaskInner }

impl_vtable! {
    impl IRawTaskVTable => RawTaskInner {
        unsafe extern "C" fn name(this: *const ()) -> Optional<StrInner<false>> {
            let this = &*(this as *const RawTaskInner);
            this.info.name.as_ref().map(|v| (v as &str).into())
        }

        unsafe extern "C" fn priority(this: *const ()) -> TaskPriority {
            let this = &*(this as *const RawTaskInner);
            this.info.priority
        }

        unsafe extern "C" fn spawn_location(this: *const ()) -> Optional<Location<'static>> {
            let this = &*(this as *const RawTaskInner);
            this.info.spawn_location
        }

        #[allow(improper_ctypes_definitions)]
        unsafe extern "C" fn scheduler_context(this: *const ()) -> *const ISchedulerContext {
            let this = &*(this as *const RawTaskInner);
            ISchedulerContext::from_object((*this.data.get()).coerce_obj())
        }

        #[allow(improper_ctypes_definitions)]
        unsafe extern "C" fn scheduler_context_mut(this: *const ()) -> *mut ISchedulerContext {
            let this = &*(this as *const RawTaskInner);
            ISchedulerContext::from_object_mut((*this.data.get()).coerce_obj_mut())
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct TaskInfo {
    priority: TaskPriority,
    name: Optional<fimo_ffi::String>,
    spawn_location: Optional<Location<'static>>,
}

fimo_object! {
    /// Interface of a raw task.
    #![vtable = IRawTaskVTable]
    pub struct IRawTask;
}

impl IRawTask {
    /// Extracts the name of the task.
    #[inline]
    pub fn name(&self) -> Option<&str> {
        let (ptr, vtable) = self.into_raw_parts();
        unsafe { (vtable.name)(ptr).into_rust().map(|n| n.into()) }
    }

    /// Extracts the starting priority of the task.
    #[inline]
    pub fn priority(&self) -> TaskPriority {
        let (ptr, vtable) = self.into_raw_parts();
        unsafe { (vtable.priority)(ptr) }
    }

    /// Extracts the spawn location of the task.
    #[inline]
    pub fn spawn_location(&self) -> Option<Location<'static>> {
        let (ptr, vtable) = self.into_raw_parts();
        unsafe { (vtable.spawn_location)(ptr).into_rust() }
    }

    /// Fetches a pointer to the internal scheduler context.
    #[inline]
    pub fn scheduler_context(&self) -> &ISchedulerContext {
        let (ptr, vtable) = self.into_raw_parts();
        unsafe { &*(vtable.scheduler_context)(ptr) }
    }

    /// Fetches a mutable pointer to the internal scheduler context.
    ///
    /// # Safety
    ///
    /// Behavior is undefined if not called from a task scheduler.
    #[inline]
    #[allow(clippy::mut_from_ref)]
    pub unsafe fn scheduler_context_mut(&self) -> &mut ISchedulerContext {
        let (ptr, vtable) = self.into_raw_parts();
        &mut *(vtable.scheduler_context_mut)(ptr)
    }
}

fimo_vtable! {
    /// VTable of a [`IRawTask`].
    #[derive(Copy, Clone, Debug, Ord, PartialOrd, Eq, PartialEq, Hash)]
    #![marker = SendSyncMarker]
    #![uuid(0xfa8ec56f, 0x9c02, 0x4ad1, 0x9845, 0x814310169d73)]
    pub struct IRawTaskVTable {
        /// Extracts the name of the task.
        pub name: unsafe extern "C" fn(*const ()) -> Optional<StrInner<false>>,
        /// Extracts the starting priority of the task.
        pub priority: unsafe extern "C" fn(*const ()) -> TaskPriority,
        /// Extracts the spawn location of the task.
        pub spawn_location: unsafe extern "C" fn(*const ()) -> Optional<Location<'static>>,
        /// Fetches a pointer to the internal scheduler context.
        pub scheduler_context: unsafe extern "C" fn(*const ()) -> *const ISchedulerContext,
        /// Fetches a mutable pointer to the internal scheduler context.
        ///
        /// # Safety
        ///
        /// Behavior is undefined if not called from a task scheduler.
        pub scheduler_context_mut: unsafe extern "C" fn(*const ()) -> *mut ISchedulerContext,
    }
}

/// A timestamp in nanoseconds from the unix epoch.
#[repr(C)]
#[derive(Copy, Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct Timestamp {
    high: u64,
    low: u64,
}

impl From<u128> for Timestamp {
    fn from(v: u128) -> Self {
        let high = (v >> 64) as u64;
        let low = (v & (u64::MAX as u128)) as u64;
        Self { high, low }
    }
}

impl From<Timestamp> for u128 {
    fn from(v: Timestamp) -> Self {
        let high: u128 = (v.high as u128) << 64;
        let low: u128 = v.low as _;
        high | low
    }
}

/// Location in a file.
#[repr(C)]
#[derive(Copy, Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct Location<'a> {
    file: ConstStr<'a>,
    line: u32,
    col: u32,
}

impl<'a> Location<'a> {
    /// Returns the source location of the caller of this function.
    #[must_use]
    #[track_caller]
    pub fn caller() -> Location<'static> {
        let l = std::panic::Location::caller();
        Location {
            file: l.file().into(),
            line: l.line(),
            col: l.column(),
        }
    }

    /// Returns the file from which the panic originated.
    #[must_use]
    pub fn file(&self) -> &str {
        self.file.into()
    }

    /// Returns the line from which the panic originated.
    #[must_use]
    pub fn line(&self) -> u32 {
        self.line
    }

    /// Returns the column from which the panic originated.
    #[must_use]
    pub fn column(&self) -> u32 {
        self.col
    }
}

/// Request for a status change.
#[repr(u32)]
#[derive(Debug, Copy, Clone, Hash, Ord, PartialOrd, PartialEq, Eq)]
pub enum StatusRequest {
    /// No request.
    None,
    /// Block request.
    Block,
    /// Abort request.
    Abort,
}

type CleanupFunc = RawFnOnce<(NonNull<Optional<UserData>>,), (), SendSyncMarker>;
type UserData = ObjBox<Object<IBase<SendSyncMarker>>>;
type PanicData = ObjBox<Object<IBase<SendMarker>>>;
type SchedulerData = ObjBox<Object<IBase<SendSyncMarker>>>;

pub(crate) struct SchedulerContextInner {
    panicking: Atomic<bool>,
    registered: Atomic<bool>,
    handle: MaybeUninit<TaskHandle>,
    resume_time: u128,
    worker: Atomic<usize>,
    request: Atomic<StatusRequest>,
    run_status: Atomic<TaskRunStatus>,
    schedule_status: Atomic<TaskScheduleStatus>,
    cleanup_func: Optional<CleanupFunc>,
    entry_func: Optional<RawFnOnce<(), (), SendSyncMarker>>,
    user_data: Optional<UserData>,
    panic_data: Optional<PanicData>,
    scheduler_data: Optional<SchedulerData>,
    _pinned: PhantomPinned,
}

unsafe impl Sync for SchedulerContextInner {}

#[repr(C)]
#[doc(hidden)]
#[derive(Debug)]
pub struct UnregisterResult {
    pub handle: TaskHandle,
    pub scheduler_data: Optional<SchedulerData>,
}

is_object! { #![uuid(0x9424aef3, 0xbc9a, 0x4b0d, 0xa877, 0x68a6c76f08ae)] SchedulerContextInner }

impl_vtable! {
    impl mut ISchedulerContextVTable => SchedulerContextInner {
        unsafe extern "C" fn handle(this: *const ()) -> MaybeUninit<TaskHandle> {
            let this = &*(this as *const SchedulerContextInner);
            this.handle
        }

        unsafe extern "C" fn is_registered(this: *const ()) -> bool {
            let this = &*(this as *const SchedulerContextInner);
            this.registered.load(Ordering::Acquire)
        }

        #[allow(improper_ctypes_definitions)]
        unsafe extern "C" fn register(
            this: *mut (),
            handle: TaskHandle,
            sched_data: Optional<SchedulerData>,
        ) {
            assert!(!is_registered(this));

            let this = &mut *(this as *mut SchedulerContextInner);
            this.handle.write(handle);
            this.scheduler_data = sched_data;
            this.registered.store(true, Ordering::Release);
        }

        #[allow(improper_ctypes_definitions)]
        unsafe extern "C" fn unregister(
            this: *mut (),
        ) -> UnregisterResult {
            assert!(is_registered(this));

            let this = &mut *(this as *mut SchedulerContextInner);
            let handle = this.handle.assume_init();
            let data = this.scheduler_data.take();
            this.registered.store(false, Ordering::Release);
            UnregisterResult {
                handle,
                scheduler_data: data
            }
        }

        unsafe extern "C" fn resume_timestamp(this: *const ()) -> Timestamp {
            let this = &*(this as *const SchedulerContextInner);
            this.resume_time.into()
        }

        unsafe extern "C" fn set_resume_timestamp(this: *mut (), timestamp: Timestamp) {
            let this = &mut *(this as *mut SchedulerContextInner);
            let timestamp = timestamp.into();
            if this.resume_time <= timestamp {
                this.resume_time = timestamp;
            }
        }

        unsafe extern "C" fn worker(this: *const ()) -> Optional<WorkerId> {
            let this = &*(this as *const SchedulerContextInner);
            WorkerId::new(this.worker.load(Ordering::Acquire)).into()
        }

        unsafe extern "C" fn set_worker(this: *const (), worker: Optional<WorkerId>) {
            let this = &*(this as *const SchedulerContextInner);
            let worker = worker.map_or(WorkerId::INVALID_WORKER_ID, |w| w.0);
            this.worker.store(worker, Ordering::Release);
        }

        unsafe extern "C" fn request_block(this: *const ()) {
            let this = &*(this as *const SchedulerContextInner);
            let _ = this.request.compare_exchange(
                StatusRequest::None,
                StatusRequest::Block,
                Ordering::AcqRel,
                Ordering::Relaxed,
            );
        }

        unsafe extern "C" fn request_abort(this: *const ()) {
            let this = &*(this as *const SchedulerContextInner);
            // Abort overwrites all other requests.
            this.request.store(StatusRequest::Abort, Ordering::Release);
        }

        unsafe extern "C" fn clear_requests(this: *const ()) -> StatusRequest {
            let this = &*(this as *const SchedulerContextInner);
            this.request.swap(StatusRequest::None, Ordering::AcqRel)
        }

        unsafe extern "C" fn run_status(this: *const ()) -> TaskRunStatus {
            let this = &*(this as *const SchedulerContextInner);
            this.run_status.load(Ordering::Acquire)
        }

        unsafe extern "C" fn set_run_status(this: *const (), status: TaskRunStatus) {
            let this = &*(this as *const SchedulerContextInner);
            this.run_status.store(status, Ordering::Release)
        }

        unsafe extern "C" fn schedule_status(this: *const ()) -> TaskScheduleStatus {
            let this = &*(this as *const SchedulerContextInner);
            this.schedule_status.load(Ordering::Acquire)
        }

        unsafe extern "C" fn set_schedule_status(this: *const (), status: TaskScheduleStatus) {
            let this = &*(this as *const SchedulerContextInner);
            this.schedule_status.store(status, Ordering::Release)
        }

        unsafe extern "C" fn is_empty_task(this: *const ()) -> bool {
            let this = &*(this as *const SchedulerContextInner);
            this.entry_func.is_none()
        }

        unsafe extern "C" fn take_entry_function(
            this: *mut (),
        ) -> Optional<RawFnOnce<(), (), SendSyncMarker>> {
            let this = &mut *(this as *mut SchedulerContextInner);
            this.entry_func.take()
        }

        unsafe extern "C" fn is_panicking(this: *const ()) -> bool {
            let this = &*(this as *const SchedulerContextInner);
            this.panicking.load(Ordering::Acquire)
        }

        #[allow(improper_ctypes_definitions)]
        unsafe extern "C" fn set_panic(this: *mut (), panic: Optional<PanicData>) {
            assert!(!is_panicking(this));

            let this = &mut *(this as *mut SchedulerContextInner);
            this.panic_data = panic;
            this.panicking.store(true, Ordering::Release)
        }

        #[allow(improper_ctypes_definitions)]
        unsafe extern "C" fn take_panic_data(this: *mut ()) -> Optional<PanicData> {
            let this = &mut *(this as *mut SchedulerContextInner);
            this.panic_data.take()
        }

        unsafe extern "C" fn cleanup(this: *mut ()) {
            let this = &mut *(this as *mut SchedulerContextInner);
            if let Some(f) = this.cleanup_func.take().into_rust() {
                f.assume_valid()(NonNull::from(&mut this.user_data))
            }
        }

        #[allow(improper_ctypes_definitions)]
        unsafe extern "C" fn user_data(this: *const ()) -> Optional<*const Object<IBase<SendSyncMarker>>> {
            let this = &*(this as *const SchedulerContextInner);
            this.user_data.as_ref().map(|d| &**d as _)
        }

        #[allow(improper_ctypes_definitions)]
        unsafe extern "C" fn user_data_mut(this: *mut ()) -> Optional<*mut Object<IBase<SendSyncMarker>>> {
            let this = &mut *(this as *mut SchedulerContextInner);
            this.user_data.as_mut().map(|d| &mut **d as _)
        }

        #[allow(improper_ctypes_definitions)]
        unsafe extern "C" fn scheduler_data(
            this: *const (),
        ) -> Optional<*const Object<IBase<SendSyncMarker>>> {
            let this = &*(this as *const SchedulerContextInner);
            this.scheduler_data.as_ref().map(|d| &**d as _)
        }

        #[allow(improper_ctypes_definitions)]
        unsafe extern "C" fn scheduler_data_mut(
            this: *mut (),
        ) -> Optional<*mut Object<IBase<SendSyncMarker>>> {
            let this = &mut *(this as *mut SchedulerContextInner);
            this.scheduler_data.as_mut().map(|d| &mut **d as _)
        }
    }
}

fimo_object! {
    /// Interface of a scheduler context.
    #![vtable = ISchedulerContextVTable]
    pub struct ISchedulerContext;
}

impl ISchedulerContext {
    /// Extracts the handle to the task.
    #[inline]
    pub fn handle(&self) -> MaybeUninit<TaskHandle> {
        let (ptr, vtable) = self.into_raw_parts();
        unsafe { (vtable.handle)(ptr) }
    }

    /// Checks whether the context has been marked as registered.
    #[inline]
    pub fn is_registered(&self) -> bool {
        let (ptr, vtable) = self.into_raw_parts();
        unsafe { (vtable.is_registered)(ptr) }
    }

    /// Marks the context as registered.
    ///
    /// # Panics
    ///
    /// May panic if the task is already registered.
    ///
    /// # Safety
    ///
    /// Behavior is undefined if not called from a task scheduler.
    #[inline]
    pub unsafe fn register(&mut self, handle: TaskHandle, scheduler_data: Option<SchedulerData>) {
        let (ptr, vtable) = self.into_raw_parts_mut();
        (vtable.register)(ptr, handle, scheduler_data.into())
    }

    /// Marks the context as unregistered.
    ///
    /// # Panics
    ///
    /// May panic if the task is not registered.
    ///
    /// # Safety
    ///
    /// Behavior is undefined if not called from a task scheduler.
    #[inline]
    pub unsafe fn unregister(&mut self) -> (TaskHandle, Option<SchedulerData>) {
        let (ptr, vtable) = self.into_raw_parts_mut();
        let res = (vtable.unregister)(ptr);
        (res.handle, res.scheduler_data.into_rust())
    }

    /// Extracts the resume time.
    #[inline]
    pub fn resume_time(&self) -> SystemTime {
        let (ptr, vtable) = self.into_raw_parts();
        let timestamp: u128 = unsafe { (vtable.resume_timestamp)(ptr).into() };
        let duration = Duration::from_nanos(timestamp as _);
        SystemTime::UNIX_EPOCH + duration
    }

    /// Advances the internal timer to the provided time.
    #[inline]
    pub fn set_resume_time(&mut self, time: SystemTime) {
        let time = time
            .duration_since(SystemTime::UNIX_EPOCH)
            .expect("Invalid time")
            .as_nanos()
            .into();

        let (ptr, vtable) = self.into_raw_parts_mut();
        unsafe { (vtable.set_resume_timestamp)(ptr, time) }
    }

    /// Extracts the assigned worker.
    #[inline]
    pub fn worker(&self) -> Option<WorkerId> {
        let (ptr, vtable) = self.into_raw_parts();
        unsafe { (vtable.worker)(ptr).into_rust() }
    }

    /// Sets a new worker.
    ///
    /// # Safety
    ///
    /// Behavior is undefined if any of the following conditions are violated:
    ///
    /// * A worker associated with the provided [`WorkerId`] does not exist.
    /// * The task has yielded it's execution and has cached thread-local variables.
    #[inline]
    pub unsafe fn set_worker(&self, worker: Option<WorkerId>) {
        let (ptr, vtable) = self.into_raw_parts();
        (vtable.set_worker)(ptr, worker.into())
    }

    /// Requests for the task to be blocked.
    #[inline]
    pub fn request_block(&self) {
        let (ptr, vtable) = self.into_raw_parts();
        unsafe { (vtable.request_block)(ptr) }
    }

    /// Requests for the task to be aborted.
    ///
    /// # Safety
    ///
    /// Aborting a task may lead to uninitialized data.
    #[inline]
    pub unsafe fn request_abort(&self) {
        let (ptr, vtable) = self.into_raw_parts();
        (vtable.request_abort)(ptr)
    }

    /// Clears the requests and returns it.
    #[inline]
    pub fn clear_request(&self) -> StatusRequest {
        let (ptr, vtable) = self.into_raw_parts();
        unsafe { (vtable.clear_requests)(ptr) }
    }

    /// Extracts the current run status.
    #[inline]
    pub fn run_status(&self) -> TaskRunStatus {
        let (ptr, vtable) = self.into_raw_parts();
        unsafe { (vtable.run_status)(ptr) }
    }

    /// Sets a new run status.
    ///
    /// # Safety
    ///
    /// Behavior is undefined if not called from a task scheduler.
    #[inline]
    pub unsafe fn set_run_status(&self, status: TaskRunStatus) {
        let (ptr, vtable) = self.into_raw_parts();
        (vtable.set_run_status)(ptr, status)
    }

    /// Extracts the current schedule status.
    #[inline]
    pub fn schedule_status(&self) -> TaskScheduleStatus {
        let (ptr, vtable) = self.into_raw_parts();
        unsafe { (vtable.schedule_status)(ptr) }
    }

    /// Sets a new schedule status.
    ///
    /// # Safety
    ///
    /// Behavior is undefined if not called from a task scheduler.
    #[inline]
    pub unsafe fn set_schedule_status(&self, status: TaskScheduleStatus) {
        let (ptr, vtable) = self.into_raw_parts();
        (vtable.set_schedule_status)(ptr, status)
    }

    /// Checks whether the task is empty.
    ///
    /// # Note
    ///
    /// May change after the task is registered with a runtime.
    #[inline]
    pub fn is_empty_task(&self) -> bool {
        let (ptr, vtable) = self.into_raw_parts();
        unsafe { (vtable.is_empty_task)(ptr) }
    }

    /// Takes the entry function of the task.
    ///
    /// # Safety
    ///
    /// Behavior is undefined if not called from a task scheduler.
    #[inline]
    pub unsafe fn take_entry_function(&mut self) -> Option<RawFnOnce<(), (), SendSyncMarker>> {
        let (ptr, vtable) = self.into_raw_parts_mut();
        (vtable.take_entry_function)(ptr).into_rust()
    }

    /// Checks whether the task is panicking.
    #[inline]
    pub fn is_panicking(&self) -> bool {
        let (ptr, vtable) = self.into_raw_parts();
        unsafe { (vtable.is_panicking)(ptr) }
    }

    /// Sets the panicking flag.
    ///
    /// # Panics
    ///
    /// May panic if the flag is already set.
    ///
    /// # Safety
    ///
    /// Behavior is undefined if not called from a task scheduler.
    #[inline]
    pub unsafe fn set_panic(&mut self, panic: Option<PanicData>) {
        let (ptr, vtable) = self.into_raw_parts_mut();
        (vtable.set_panic)(ptr, panic.into())
    }

    /// Takes the panic data from the task.
    ///
    /// # Panics
    ///
    /// May panic if the task is registered.
    ///
    /// # Safety
    ///
    /// Behavior is undefined if the task has not completed or aborted it's execution.
    #[inline]
    pub unsafe fn take_panic_data(&mut self) -> Option<PanicData> {
        let (ptr, vtable) = self.into_raw_parts_mut();
        (vtable.take_panic_data)(ptr).into_rust()
    }

    /// Calls the cleanup function.
    ///
    /// # Safety
    ///
    /// Behavior is undefined if not called from a task scheduler.
    #[inline]
    pub unsafe fn cleanup(&mut self) {
        let (ptr, vtable) = self.into_raw_parts_mut();
        (vtable.cleanup)(ptr)
    }

    /// Fetches a reference to the user data.
    #[inline]
    pub fn user_data(&self) -> Option<&Object<IBase<SendSyncMarker>>> {
        let (ptr, vtable) = self.into_raw_parts();
        unsafe { (vtable.user_data)(ptr).into_rust().map(|o| &*o) }
    }

    /// Fetches a mutable reference to the user data.
    #[inline]
    pub fn user_data_mut(&mut self) -> Option<&mut Object<IBase<SendSyncMarker>>> {
        let (ptr, vtable) = self.into_raw_parts_mut();
        unsafe { (vtable.user_data_mut)(ptr).into_rust().map(|o| &mut *o) }
    }

    /// Fetches a reference to the scheduler data.
    ///
    /// # Safety
    ///
    /// Behavior is undefined if not called from a task scheduler.
    #[inline]
    pub unsafe fn scheduler_data(&self) -> Option<&Object<IBase<SendSyncMarker>>> {
        let (ptr, vtable) = self.into_raw_parts();
        (vtable.scheduler_data)(ptr).into_rust().map(|o| &*o)
    }

    /// Fetches a mutable reference to the scheduler data.
    ///
    /// # Safety
    ///
    /// Behavior is undefined if not called from a task scheduler.
    #[inline]
    pub unsafe fn scheduler_data_mut(&mut self) -> Option<&mut Object<IBase<SendSyncMarker>>> {
        let (ptr, vtable) = self.into_raw_parts_mut();
        (vtable.scheduler_data_mut)(ptr)
            .into_rust()
            .map(|o| &mut *o)
    }
}

fimo_vtable! {
    /// VTable of a [`ISchedulerContext`].
    #[derive(Copy, Clone, Debug, Ord, PartialOrd, Eq, PartialEq, Hash)]
    #![marker = SendSyncMarker]
    #![uuid(0xf0e48d5e, 0xd826, 0x4122, 0xae14, 0xd8430ad3e796)]
    pub struct ISchedulerContextVTable {
        /// Extracts the handle to the task.
        pub handle: unsafe extern "C" fn(*const ()) -> MaybeUninit<TaskHandle>,
        /// Checks whether the context has been marked as registered.
        pub is_registered: unsafe extern "C" fn(*const ()) -> bool,
        /// Marks the context as registered.
        ///
        /// # Panics
        ///
        /// May panic if the task is already registered.
        ///
        /// # Safety
        ///
        /// Behavior is undefined if not called from a task scheduler.
        pub register: unsafe extern "C" fn(*mut (), TaskHandle, Optional<SchedulerData>),
        /// Marks the context as unregistered.
        ///
        /// # Panics
        ///
        /// May panic if the task is not registered.
        ///
        /// # Safety
        ///
        /// Behavior is undefined if not called from a task scheduler.
        pub unregister: unsafe extern "C" fn(*mut ()) -> UnregisterResult,
        /// Extracts a timestamp as nanoseconds.
        pub resume_timestamp: unsafe extern "C" fn(*const ()) -> Timestamp,
        /// Advances the internal timer to the provided timestamp.
        pub set_resume_timestamp: unsafe extern "C" fn(*mut (), Timestamp),
        /// Extracts the assigned worker.
        pub worker: unsafe extern "C" fn(*const ()) -> Optional<WorkerId>,
        /// Sets a new worker.
        ///
        /// # Safety
        ///
        /// Behavior is undefined if any of the following conditions are violated:
        ///
        /// * A worker associated with the provided [`WorkerId`] does not exist.
        /// * The task has yielded it's execution and has cached thread-local variables.
        pub set_worker: unsafe extern "C" fn(*const (), Optional<WorkerId>),
        /// Requests for the task to be blocked.
        pub request_block: unsafe extern "C" fn(*const ()),
        /// Requests for the task to be aborted.
        ///
        /// # Safety
        ///
        /// Aborting a task may lead to uninitialized data.
        pub request_abort: unsafe extern "C" fn(*const ()),
        /// Clears the requests and returns it.
        pub clear_requests: unsafe extern "C" fn(*const ()) -> StatusRequest,
        /// Extracts the current run status.
        pub run_status: unsafe extern "C" fn(*const ()) -> TaskRunStatus,
        /// Sets a new run status.
        ///
        /// # Safety
        ///
        /// Behavior is undefined if not called from a task scheduler.
        pub set_run_status: unsafe extern "C" fn(*const (), TaskRunStatus),
        /// Extracts the current schedule status.
        pub schedule_status: unsafe extern "C" fn(*const ()) -> TaskScheduleStatus,
        /// Sets a new schedule status.
        ///
        /// # Safety
        ///
        /// Behavior is undefined if not called from a task scheduler.
        pub set_schedule_status: unsafe extern "C" fn(*const (), TaskScheduleStatus),
        /// Checks whether the task is empty.
        ///
        /// # Note
        ///
        /// May change after the task is registered with a runtime.
        pub is_empty_task: unsafe extern "C" fn(*const ()) -> bool,
        /// Takes the entry function of the task.
        ///
        /// # Safety
        ///
        /// Behavior is undefined if not called from a task scheduler.
        pub take_entry_function: unsafe extern "C" fn(*mut ()) -> Optional<RawFnOnce<(), (), SendSyncMarker>>,
        /// Checks whether the task is panicking.
        pub is_panicking: unsafe extern "C" fn(*const ()) -> bool,
        /// Sets the panicking flag.
        ///
        /// # Panics
        ///
        /// May panic if the flag is already set.
        ///
        /// # Safety
        ///
        /// Behavior is undefined if not called from a task scheduler.
        pub set_panic: unsafe extern "C" fn(*mut (), Optional<PanicData>),
        /// Takes the panic data from the task.
        ///
        /// # Panics
        ///
        /// May panic if the task is registered.
        ///
        /// # Safety
        ///
        /// Behavior is undefined if the task has not completed or aborted it's execution.
        pub take_panic_data: unsafe extern "C" fn(*mut ()) -> Optional<PanicData>,
        /// Calls the cleanup function.
        ///
        /// # Safety
        ///
        /// Behavior is undefined if not called from a task scheduler.
        pub cleanup: unsafe extern "C" fn(*mut ()),
        /// Fetches a reference to the user data.
        pub user_data: unsafe extern "C" fn(*const ()) -> Optional<*const Object<IBase<SendSyncMarker>>>,
        /// Fetches a mutable reference to the user data.
        pub user_data_mut: unsafe extern "C" fn(*mut ()) -> Optional<*mut Object<IBase<SendSyncMarker>>>,
        /// Fetches a reference to the scheduler data.
        ///
        /// # Safety
        ///
        /// Behavior is undefined if not called from a task scheduler.
        pub scheduler_data: unsafe extern "C" fn(*const ()) -> Optional<*const Object<IBase<SendSyncMarker>>>,
        /// Fetches a mutable reference to the scheduler data.
        ///
        /// # Safety
        ///
        /// Behavior is undefined if not called from a task scheduler.
        pub scheduler_data_mut: unsafe extern "C" fn(*mut ()) -> Optional<*mut Object<IBase<SendSyncMarker>>>,
    }
}
