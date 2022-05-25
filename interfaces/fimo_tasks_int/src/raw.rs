//! Raw tasks primitives.

use atomic::Atomic;
use fimo_ffi::cell::AtomicRefCell;
use fimo_ffi::ffi_fn::RawFfiFn;
use fimo_ffi::obj_box::RawObjBox;
use fimo_ffi::ptr::{IBase, RawObj, RawObjMut};
use fimo_ffi::str::ConstStrPtr;
use fimo_ffi::tuple::Tuple2;
use fimo_ffi::{interface, ConstStr, DynObj, FfiFn, ObjBox, ObjectId, Optional, ReprC};
use std::any::Any;
use std::fmt::Write;
use std::fmt::{Debug, Display, Formatter};
use std::marker::PhantomPinned;
use std::mem::MaybeUninit;
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

impl Display for TaskHandle {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "({}, {})", self.id, self.generation)
    }
}

/// Priority of a task.
#[repr(transparent)]
#[derive(Debug, Copy, Clone, Hash, Ord, PartialOrd, PartialEq, Eq)]
pub struct TaskPriority(pub isize);

impl Display for TaskPriority {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        Display::fmt(&self.0, f)
    }
}

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

impl Display for WorkerId {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        Display::fmt(&self.0, f)
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

/// Representation of a raw task.
#[derive(ObjectId)]
#[fetch_vtable(uuid = "eb91ee4a-22d2-4b91-9e06-0994f0d79b0f", interfaces(IRawTask))]
pub struct RawTaskInner<'a> {
    info: TaskInfo,
    data: AtomicRefCell<SchedulerContextInner<'a>>,
}

impl<'a> IRawTask for RawTaskInner<'a> {
    #[inline]
    fn name(&self) -> Option<&str> {
        self.info.name.as_deref()
    }

    #[inline]
    fn priority(&self) -> TaskPriority {
        self.info.priority
    }

    #[inline]
    fn spawn_location(&self) -> Option<Location<'static>> {
        self.info.spawn_location.map(Location::from_std)
    }

    #[inline]
    fn context(&self) -> &AtomicRefCell<DynObj<dyn ISchedulerContext + '_>> {
        self.data.coerce_obj()
    }
}

impl Debug for RawTaskInner<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RawTaskInner")
            .field("info", &self.info)
            .field("data", &self.context_atomic())
            .finish()
    }
}

#[derive(Debug, Clone)]
pub(crate) struct TaskInfo {
    priority: TaskPriority,
    name: Option<String>,
    spawn_location: Option<&'static std::panic::Location<'static>>,
}

/// A builder for raw tasks.
#[derive(Debug, Clone)]
pub struct Builder {
    info: TaskInfo,
    start_time: SystemTime,
    worker: Option<WorkerId>,
    status: StatusRequest,
}

impl Builder {
    /// Constructs a new `Builder`.
    #[inline]
    #[track_caller]
    pub fn new() -> Self {
        Self {
            info: TaskInfo {
                priority: TaskPriority(0),
                name: Default::default(),
                spawn_location: Some(std::panic::Location::caller()),
            },
            start_time: SystemTime::now(),
            worker: None,
            status: StatusRequest::None,
        }
    }

    /// Names the task.
    #[inline]
    pub fn with_name(mut self, name: String) -> Self {
        self.info.name = Some(name);
        self
    }

    /// Extends the name of the task with an index.
    #[inline]
    pub fn extend_name_index(mut self, index: usize) -> Self {
        if let Some(n) = &mut self.info.name {
            write!(n, ": {index}").expect("could not extend name");
        }
        self
    }

    /// Assigns a priority to the task.
    ///
    /// A lower [`TaskPriority`] value will lead to a higher priority.
    /// The default priority is `TaskPriority(0)`.
    #[inline]
    pub fn with_priority(mut self, priority: TaskPriority) -> Self {
        self.info.priority = priority;
        self
    }

    /// Assigns a start time to the task.
    #[inline]
    pub fn with_start_time(mut self, start_time: SystemTime) -> Self {
        if self.start_time < start_time {
            self.start_time = start_time;
        }
        self
    }

    /// Assigns a worker to the task.
    ///
    /// # Safety
    ///
    /// Behavior is undefined if the worker does not exist.
    #[inline]
    pub unsafe fn with_worker(mut self, worker: Option<WorkerId>) -> Self {
        self.worker = worker;
        self
    }

    /// Marks the task as blocked.
    #[inline]
    pub fn blocked(mut self) -> Self {
        self.status = StatusRequest::Block;
        self
    }

    /// Builds the [`RawTaskInner`].
    #[inline]
    pub fn build<'a>(
        self,
        f: Option<FfiFn<'a, dyn FnOnce() + Send + 'a>>,
        cleanup: Option<FfiFn<'a, dyn FnOnce() + Send + 'a>>,
    ) -> RawTaskInner<'a> {
        RawTaskInner {
            info: self.info,
            data: AtomicRefCell::new(SchedulerContextInner {
                is_empty: f.is_none(),
                panicking: Atomic::new(false),
                registered: Atomic::new(false),
                handle: Atomic::new(MaybeUninit::uninit()),
                resume_time: Atomic::new(
                    self.start_time
                        .duration_since(SystemTime::UNIX_EPOCH)
                        .expect("Time went backwards")
                        .as_nanos(),
                ),
                worker: Atomic::new(
                    self.worker
                        .map(|w| w.0)
                        .unwrap_or(WorkerId::INVALID_WORKER_ID),
                ),
                request: Atomic::new(self.status),
                run_status: Atomic::new(TaskRunStatus::Idle),
                schedule_status: Atomic::new(TaskScheduleStatus::Runnable),
                cleanup_func: SyncWrapper::new(cleanup),
                entry_func: SyncWrapper::new(f),
                panic_data: SyncWrapper::new(None),
                scheduler_data: None,
                _pinned: Default::default(),
            }),
        }
    }
}

impl Default for Builder {
    #[inline]
    #[track_caller]
    fn default() -> Self {
        Builder::new()
    }
}

/// A task that is always blocked and bound to an address.
///
/// Useful for the implementation of synchronization primitives.
#[repr(transparent)]
#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct PseudoTask(pub *const ());

unsafe impl Send for PseudoTask {}
unsafe impl Sync for PseudoTask {}

/// Interface of a raw task.
#[interface(
    uuid = "fa8ec56f-9c02-4ad1-9845-814310169d73",
    vtable = "IRawTaskVTable",
    generate()
)]
pub trait IRawTask: IBase + Send + Sync {
    /// Returns the optional name of the task.
    #[vtable_info(
        unsafe,
        abi = r#"extern "C-unwind""#,
        return_type = "Optional<ConstStrPtr>",
        into_expr = "Optional::from_rust(res.map(|d| d.into()))",
        from_expr = "unsafe { res.into_rust().map(|d| d.deref().into()) }"
    )]
    fn name(&self) -> Option<&str>;

    /// Shorthand for `self.name().unwrap_or("unnamed")`.
    #[inline]
    #[vtable_info(ignore)]
    fn resolved_name(&self) -> &str {
        self.name().unwrap_or("unnamed")
    }

    /// Returns the starting priority of the task.
    #[vtable_info(unsafe, abi = r#"extern "C-unwind""#)]
    fn priority(&self) -> TaskPriority;

    /// Returns the spawn location of the task.
    #[vtable_info(
        unsafe,
        abi = r#"extern "C-unwind""#,
        return_type = "Optional<Location<'static>>",
        into = "Into::into",
        from = "Into::into"
    )]
    fn spawn_location(&self) -> Option<Location<'static>>;

    /// Returns a reference to the context.
    ///
    /// # Intended usage
    ///
    /// This method is intended to be used primarely by owners of unregistered tasks
    /// or while holding a lock to the scheduler. Under these circumstances the runtime
    /// guarantees that a call to [`try_borrow`][try_borrow] succeeds.
    ///
    /// Calling the [`try_borrow_mut`][try_borrow_mut] method may cause a panic as multiple
    /// workers try to access the context. Therefore the [`try_borrow_mut`][try_borrow_mut]
    /// method is reserved for use by the scheduler and owners of unregistered tasks.
    ///
    /// Using these functions without, even indirectly, without the proper preconditions may
    /// cause undefined behavior.
    ///
    /// [try_borrow]: AtomicRefCell::try_borrow
    /// [try_borrow_mut]: AtomicRefCell::try_borrow_mut
    #[vtable_info(
        unsafe,
        abi = r#"extern "C-unwind""#,
        return_type = "*const AtomicRefCell<DynObj<dyn ISchedulerContext + 'static>>",
        into_expr = "
            std::mem::transmute
                ::<*const AtomicRefCell<DynObj<dyn ISchedulerContext + '_>>, 
                    *const AtomicRefCell<DynObj<dyn ISchedulerContext + 'static>>>(res)",
        from_expr = "
            unsafe {
                let res = std::mem::transmute
                    ::<*const AtomicRefCell<DynObj<dyn ISchedulerContext + 'static>>, 
                        *const AtomicRefCell<DynObj<dyn ISchedulerContext + '_>>>(res);
                &*res
            }"
    )]
    fn context(&self) -> &AtomicRefCell<DynObj<dyn ISchedulerContext + '_>>;

    /// Returns an atomic view to the context.
    ///
    /// Should be preferred over [`context`](#method.context) as it allows accessing
    /// parts of the [`ISchedulerContext`] without borrowing it.
    #[inline]
    #[vtable_info(ignore)]
    fn context_atomic(&self) -> AtomicISchedulerContext<'_> {
        let context = self.context();
        AtomicISchedulerContext { context }
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
    #[inline]
    fn from(v: u128) -> Self {
        let high = (v >> 64) as u64;
        let low = (v & (u64::MAX as u128)) as u64;
        Self { high, low }
    }
}

impl From<SystemTime> for Timestamp {
    #[inline]
    fn from(t: SystemTime) -> Self {
        let duration = t
            .duration_since(SystemTime::UNIX_EPOCH)
            .expect("time went backwards");
        let nanos = duration.as_nanos();
        nanos.into()
    }
}

impl From<Timestamp> for u128 {
    #[inline]
    fn from(v: Timestamp) -> Self {
        let high: u128 = (v.high as u128) << 64;
        let low: u128 = v.low as _;
        high | low
    }
}

impl From<Timestamp> for SystemTime {
    fn from(t: Timestamp) -> Self {
        const NANOS_PER_SEC: u32 = 1_000_000_000;
        let nanos: u128 = t.into();

        // split the timestamp into seconds and nanoseconds
        let secs: u64 = (nanos / NANOS_PER_SEC as u128) as u64;
        let subsec_nanos: u64 = (nanos - (secs * NANOS_PER_SEC as u64) as u128) as u64;

        let duration = Duration::from_secs(secs) + Duration::from_nanos(subsec_nanos);
        SystemTime::UNIX_EPOCH + duration
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
        Location::from_std(l)
    }

    /// Constructs a location from a [`std::panic::Location`].
    #[must_use]
    pub fn from_std(loc: &'a std::panic::Location<'a>) -> Location<'a> {
        Location {
            file: loc.file().into(),
            line: loc.line(),
            col: loc.column(),
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

#[derive(ObjectId)]
#[fetch_vtable(
    uuid = "9424aef3-bc9a-4b0d-a877-68a6c76f08ae",
    interfaces(ISchedulerContext)
)]
pub(crate) struct SchedulerContextInner<'a> {
    is_empty: bool,
    panicking: Atomic<bool>,
    registered: Atomic<bool>,
    // may use locks on some architectures
    handle: Atomic<MaybeUninit<TaskHandle>>,
    resume_time: Atomic<u128>,
    worker: Atomic<usize>,
    request: Atomic<StatusRequest>,
    run_status: Atomic<TaskRunStatus>,
    schedule_status: Atomic<TaskScheduleStatus>,
    cleanup_func: SyncWrapper<Option<FfiFn<'a, dyn FnOnce() + Send + 'a>>>,
    entry_func: SyncWrapper<Option<FfiFn<'a, dyn FnOnce() + Send + 'a>>>,
    panic_data: SyncWrapper<Option<ObjBox<DynObj<dyn IBase + Send + 'static>>>>,
    scheduler_data: Option<ObjBox<DynObj<dyn IBase + Send + Sync + 'static>>>,
    _pinned: PhantomPinned,
}

struct SendWrapper<T>(T);
unsafe impl<T> Send for SendWrapper<T> {}

struct SyncWrapper<T> {
    init: bool,
    val: MaybeUninit<T>,
}

impl<T> SyncWrapper<T> {
    #[inline]
    fn new(val: T) -> Self {
        Self {
            init: true,
            val: MaybeUninit::new(val),
        }
    }

    #[inline]
    fn is_init(&self) -> bool {
        self.init
    }

    #[inline]
    fn take(&mut self) -> T {
        assert!(self.init, "value already taken");
        self.init = false;
        unsafe { self.val.assume_init_read() }
    }

    #[inline]
    fn swap(&mut self, val: T) -> T {
        assert!(self.init, "value already taken");
        let mut val = MaybeUninit::new(val);
        std::mem::swap(&mut self.val, &mut val);
        unsafe { val.assume_init() }
    }

    #[inline]
    fn write(&mut self, val: T) -> Option<T> {
        let mut val = MaybeUninit::new(val);
        std::mem::swap(&mut self.val, &mut val);

        let mut init = true;
        std::mem::swap(&mut self.init, &mut init);
        if init {
            unsafe { Some(val.assume_init()) }
        } else {
            None
        }
    }
}

unsafe impl<T: Send> Sync for SyncWrapper<T> {}

impl<T> Drop for SyncWrapper<T> {
    fn drop(&mut self) {
        if self.init {
            unsafe { self.val.assume_init_drop() }
        }
    }
}

impl<'a> ISchedulerContext for SchedulerContextInner<'a> {
    #[inline]
    fn handle(&self) -> MaybeUninit<TaskHandle> {
        self.handle.load(Ordering::Acquire)
    }

    #[inline]
    fn is_registered(&self) -> bool {
        self.registered.load(Ordering::Acquire)
    }

    #[inline]
    unsafe fn register(
        &mut self,
        handle: TaskHandle,
        scheduler_data: Option<ObjBox<DynObj<dyn IBase + Send + Sync + 'static>>>,
    ) {
        assert!(!self.is_registered(), "task already registered");

        self.handle
            .store(MaybeUninit::new(handle), Ordering::Release);
        self.scheduler_data = scheduler_data;
        self.registered.store(true, Ordering::Release);
    }

    #[inline]
    unsafe fn unregister(
        &mut self,
    ) -> (
        TaskHandle,
        Option<ObjBox<DynObj<dyn IBase + Send + Sync + 'static>>>,
    ) {
        assert!(self.is_registered(), "task is not registered");

        let handle = self.handle().assume_init();
        let data = self.scheduler_data.take();
        self.registered.store(false, Ordering::Release);
        (handle, data)
    }

    #[inline]
    fn resume_time(&self) -> SystemTime {
        let timestamp = Timestamp::from(self.resume_time.load(Ordering::Acquire));
        timestamp.into()
    }

    #[inline]
    fn set_resume_time(&self, time: SystemTime) {
        let time: u128 = time
            .duration_since(SystemTime::UNIX_EPOCH)
            .expect("time went backwards")
            .as_nanos();
        let mut current = self.resume_time.load(Ordering::Relaxed);
        while current <= time {
            match self.resume_time.compare_exchange_weak(
                current,
                time,
                Ordering::AcqRel,
                Ordering::Relaxed,
            ) {
                Ok(_) => return,
                Err(t) => current = t,
            }
        }
    }

    #[inline]
    fn worker(&self) -> Option<WorkerId> {
        let id = self.worker.load(Ordering::Acquire);
        WorkerId::new(id)
    }

    #[inline]
    unsafe fn set_worker(&self, worker: Option<WorkerId>) {
        let id = worker.map_or(WorkerId::INVALID_WORKER_ID, |w| w.0);
        self.worker.store(id, Ordering::Release);
    }

    #[inline]
    fn request_block(&self) {
        let _ = self.request.compare_exchange(
            StatusRequest::None,
            StatusRequest::Block,
            Ordering::AcqRel,
            Ordering::Relaxed,
        );
    }

    #[inline]
    unsafe fn request_abort(&self) {
        // Abort overwrites all other requests.
        self.request.store(StatusRequest::Abort, Ordering::Release);
    }

    #[inline]
    fn peek_request(&self) -> StatusRequest {
        self.request.load(Ordering::Acquire)
    }

    #[inline]
    unsafe fn clear_request(&self) -> StatusRequest {
        self.request.swap(StatusRequest::None, Ordering::AcqRel)
    }

    #[inline]
    fn run_status(&self) -> TaskRunStatus {
        self.run_status.load(Ordering::Acquire)
    }

    #[inline]
    unsafe fn set_run_status(&self, status: TaskRunStatus) {
        self.run_status.store(status, Ordering::Release)
    }

    #[inline]
    fn schedule_status(&self) -> TaskScheduleStatus {
        self.schedule_status.load(Ordering::Acquire)
    }

    #[inline]
    unsafe fn set_schedule_status(&self, status: TaskScheduleStatus) {
        self.schedule_status.store(status, Ordering::Release)
    }

    #[inline]
    fn is_empty_task(&self) -> bool {
        self.is_empty
    }

    #[inline]
    unsafe fn take_entry_function(&mut self) -> Option<FfiFn<'_, dyn FnOnce() + Send + '_>> {
        // FfiFn is invariant in `T` but RawFfiFn is not.
        self.entry_func
            .take()
            .take()
            .map(|f| f.into_raw().assume_valid())
    }

    #[inline]
    fn is_panicking(&self) -> bool {
        self.panicking.load(Ordering::Acquire)
    }

    #[inline]
    unsafe fn set_panic(&mut self, panic: Option<ObjBox<DynObj<dyn IBase + Send + 'static>>>) {
        assert!(!self.is_panicking(), "panic may not be overwritten");

        self.panic_data.write(panic);
        self.panicking.store(true, Ordering::Release)
    }

    #[inline]
    unsafe fn take_panic_data(&mut self) -> Option<ObjBox<DynObj<dyn IBase + Send + 'static>>> {
        self.panic_data.swap(None)
    }

    #[inline]
    fn cleanup(&mut self) {
        if let Some(f) = self.cleanup_func.take() {
            f()
        }
    }

    #[inline]
    fn scheduler_data(&self) -> Option<&DynObj<dyn IBase + Send + Sync + 'static>> {
        self.scheduler_data.as_deref()
    }
}

impl Drop for SchedulerContextInner<'_> {
    fn drop(&mut self) {
        if self.cleanup_func.is_init() {
            self.cleanup()
        }
    }
}

/// Interface of a scheduler context.
#[interface(
    uuid = "f0e48d5e-d826-4122-ae14-d8430ad3e796",
    vtable = "ISchedulerContextVTable",
    generate()
)]
pub trait ISchedulerContext: IBase + Send + Sync {
    /// Extracts the handle to the task.
    ///
    /// # Note
    ///
    /// Must be implemented atomically.
    #[vtable_info(unsafe, abi = r#"extern "C-unwind""#)]
    fn handle(&self) -> MaybeUninit<TaskHandle>;

    /// Checks whether the context has been marked as registered.
    ///
    /// # Note
    ///
    /// Must be implemented atomically.
    #[vtable_info(unsafe, abi = r#"extern "C-unwind""#)]
    fn is_registered(&self) -> bool;

    /// Marks the context as registered.
    ///
    /// # Panics
    ///
    /// May panic if the task is already registered.
    ///
    /// # Safety
    ///
    /// Behavior is undefined if not called from a task scheduler.
    #[vtable_info(unsafe, abi = r#"extern "C-unwind""#)]
    unsafe fn register(
        &mut self,
        handle: TaskHandle,
        #[vtable_info(
            type = "Optional<RawObjBox<RawObjMut<dyn IBase + Send + Sync + 'static>>>",
            into_expr = "let p_2 = Optional::from_rust(p_2.map(|d| d.into()));",
            from_expr = "let p_2 = p_2.into_rust().map(|d| d.into());"
        )]
        scheduler_data: Option<ObjBox<DynObj<dyn IBase + Send + Sync + 'static>>>,
    );

    /// Marks the context as unregistered.
    ///
    /// # Panics
    ///
    /// May panic if the task is not registered.
    ///
    /// # Safety
    ///
    /// Behavior is undefined if not called from a task scheduler.
    #[vtable_info(
        unsafe,
        abi = r#"extern "C-unwind""#,
        return_type = "Tuple2<TaskHandle, Optional<RawObjBox<RawObjMut<dyn IBase + Send + Sync + 'static>>>>",
        into_expr = "Tuple2(res.0, Optional::from_rust(res.1.map(|d| d.into())))",
        from_expr = "(res.0, res.1.into_rust().map(|d| d.into()))"
    )]
    unsafe fn unregister(
        &mut self,
    ) -> (
        TaskHandle,
        Option<ObjBox<DynObj<dyn IBase + Send + Sync + 'static>>>,
    );

    /// Extracts the resume time.
    ///
    /// # Note
    ///
    /// Must be implemented atomically.
    #[vtable_info(
        unsafe,
        abi = r#"extern "C-unwind""#,
        return_type = "Timestamp",
        into = "Into::into",
        from = "Into::into"
    )]
    fn resume_time(&self) -> SystemTime;

    /// Advances the internal timer to the provided time.
    ///
    /// # Note
    ///
    /// Must be implemented atomically.
    /// The runtime may not be notified if this is called outside of the scheduler.
    #[vtable_info(unsafe, abi = r#"extern "C-unwind""#)]
    fn set_resume_time(
        &self,
        #[vtable_info(type = "Timestamp", into = "Into::into", from = "Into::into")]
        time: SystemTime,
    );

    /// Extracts the assigned worker.
    ///
    /// # Note
    ///
    /// Must be implemented atomically.
    #[vtable_info(
        unsafe,
        abi = r#"extern "C-unwind""#,
        return_type = "Optional<WorkerId>",
        into = "Into::into",
        from = "Into::into"
    )]
    fn worker(&self) -> Option<WorkerId>;

    /// Sets a new worker.
    ///
    /// Passing in `None` will automatically select an appropriate worker.
    ///
    /// # Note
    ///
    /// Must be implemented atomically.
    ///
    /// # Safety
    ///
    /// Behavior is undefined if any of the following conditions are violated:
    ///
    /// * A worker associated with the provided [`WorkerId`] does not exist.
    /// * The task has yielded it's execution and has cached thread-local variables.
    /// * Is used by someone other than the runtime implementation and the task is registered.
    #[vtable_info(unsafe, abi = r#"extern "C-unwind""#)]
    unsafe fn set_worker(
        &self,
        #[vtable_info(type = "Optional<WorkerId>", into = "Into::into", from = "Into::into")]
        worker: Option<WorkerId>,
    );

    /// Requests for the task to be blocked.
    ///
    /// # Note
    ///
    /// Must be implemented atomically.
    #[vtable_info(unsafe, abi = r#"extern "C-unwind""#)]
    fn request_block(&self);

    /// Requests for the task to be aborted.
    ///
    /// # Note
    ///
    /// Must be implemented atomically.
    ///
    /// # Safety
    ///
    /// Aborting a task may lead to uninitialized data.
    #[vtable_info(unsafe, abi = r#"extern "C-unwind""#)]
    unsafe fn request_abort(&self);

    /// Shows the requests.
    ///
    /// # Note
    ///
    /// Must be implemented atomically.
    #[vtable_info(unsafe, abi = r#"extern "C-unwind""#)]
    fn peek_request(&self) -> StatusRequest;

    /// Clears the requests and returns it.
    ///
    /// # Note
    ///
    /// Must be implemented atomically.
    ///
    /// # Safety
    ///
    /// Behavior is undefined if not called from a task scheduler.
    #[vtable_info(unsafe, abi = r#"extern "C-unwind""#)]
    unsafe fn clear_request(&self) -> StatusRequest;

    /// Extracts the current run status.
    ///
    /// # Note
    ///
    /// Must be implemented atomically.
    #[vtable_info(unsafe, abi = r#"extern "C-unwind""#)]
    fn run_status(&self) -> TaskRunStatus;

    /// Sets a new run status.
    ///
    /// # Note
    ///
    /// Must be implemented atomically.
    ///
    /// # Safety
    ///
    /// Behavior is undefined if not called from a task scheduler.
    #[vtable_info(unsafe, abi = r#"extern "C-unwind""#)]
    unsafe fn set_run_status(&self, status: TaskRunStatus);

    /// Extracts the current schedule status.
    ///
    /// # Note
    ///
    /// Must be implemented atomically.
    #[vtable_info(unsafe, abi = r#"extern "C-unwind""#)]
    fn schedule_status(&self) -> TaskScheduleStatus;

    /// Sets a new schedule status.
    ///
    /// # Note
    ///
    /// Must be implemented atomically.
    ///
    /// # Safety
    ///
    /// Behavior is undefined if not called from a task scheduler.
    #[vtable_info(unsafe, abi = r#"extern "C-unwind""#)]
    unsafe fn set_schedule_status(&self, status: TaskScheduleStatus);

    /// Checks whether the task is empty.
    ///
    /// # Note
    ///
    /// May change after the task is registered with a runtime.
    #[vtable_info(unsafe, abi = r#"extern "C-unwind""#)]
    fn is_empty_task(&self) -> bool;

    /// Takes the entry function of the task.
    ///
    /// # Safety
    ///
    /// Behavior is undefined if not called from a task scheduler.
    #[vtable_info(
        unsafe,
        abi = r#"extern "C-unwind""#,
        return_type = "Optional<RawFfiFn<dyn FnOnce() + Send + 'static>>",
        into_expr = "let res = Optional::from_rust(res)?; 
            Optional::Some(std::mem::transmute(res.into_raw()))",
        from_expr = "let res = res.into_rust()?; Some(res.assume_valid())"
    )]
    unsafe fn take_entry_function(&mut self) -> Option<FfiFn<'_, dyn FnOnce() + Send + '_>>;

    /// Checks whether the task is panicking.
    ///
    /// # Note
    ///
    /// Must be implemented atomically.
    #[vtable_info(unsafe, abi = r#"extern "C-unwind""#)]
    fn is_panicking(&self) -> bool;

    /// Sets the panicking flag.
    ///
    /// # Panics
    ///
    /// May panic if the flag is already set.
    ///
    /// # Safety
    ///
    /// Behavior is undefined if not called from a task scheduler.
    #[vtable_info(unsafe, abi = r#"extern "C-unwind""#)]
    unsafe fn set_panic(
        &mut self,
        #[vtable_info(
            type = "Optional<RawObjBox<RawObjMut<dyn IBase + Send + 'static>>>",
            into_expr = "let p_1 = Optional::from_rust(p_1.map(|v| v.into()));",
            from_expr = "let p_1 = p_1.into_rust().map(|v| v.into())"
        )]
        panic: Option<ObjBox<DynObj<dyn IBase + Send + 'static>>>,
    );

    /// Takes the panic data from the task.
    ///
    /// # Panics
    ///
    /// May panic if the task is registered.
    ///
    /// # Safety
    ///
    /// Behavior is undefined if the task has not completed or aborted it's execution.
    #[vtable_info(
        unsafe,
        abi = r#"extern "C-unwind""#,
        return_type = "Optional<RawObjBox<RawObjMut<dyn IBase + Send + 'static>>>",
        into_expr = "let res = Optional::from_rust(res)?; Optional::Some(res.into())",
        from_expr = "let res = res.into_rust()?; Some(res.into())"
    )]
    unsafe fn take_panic_data(&mut self) -> Option<ObjBox<DynObj<dyn IBase + Send + 'static>>>;

    /// Calls the cleanup function.
    #[vtable_info(unsafe, abi = r#"extern "C-unwind""#)]
    fn cleanup(&mut self);

    /// Fetches a reference to the scheduler data.
    #[vtable_info(
        unsafe,
        abi = r#"extern "C-unwind""#,
        return_type = "Optional<RawObj<dyn IBase + Send + Sync + 'static>>",
        into_expr = "let res = Optional::from_rust(res)?; Optional::Some(fimo_ffi::ptr::into_raw(res))",
        from_expr = "let res = res.into_rust()?; unsafe { Some(&*fimo_ffi::ptr::from_raw(res)) }"
    )]
    fn scheduler_data(&self) -> Option<&DynObj<dyn IBase + Send + Sync + 'static>>;
}

/// Access to the atomic member functions of [`ISchedulerContext`].
///
/// Bypasses the [`AtomicRefCell`] allowing access while the context is borrowed.
pub struct AtomicISchedulerContext<'a> {
    context: &'a AtomicRefCell<DynObj<dyn ISchedulerContext + 'a>>,
}

impl AtomicISchedulerContext<'_> {
    /// Extracts the handle to the task.
    #[inline]
    pub fn handle(&self) -> MaybeUninit<TaskHandle> {
        let ptr = self.context.as_ptr();
        // SAFETY: the function is atomic and therefore causes no data races.
        unsafe { (*ptr).handle() }
    }

    /// Checks whether the context has been marked as registered.
    #[inline]
    pub fn is_registered(&self) -> bool {
        let ptr = self.context.as_ptr();
        // SAFETY: the function is atomic and therefore causes no data races.
        unsafe { (*ptr).is_registered() }
    }

    /// Extracts the resume time.
    #[inline]
    pub fn resume_time(&self) -> SystemTime {
        let ptr = self.context.as_ptr();
        // SAFETY: the function is atomic and therefore causes no data races.
        unsafe { (*ptr).resume_time() }
    }

    /// Extracts the assigned worker.
    #[inline]
    pub fn worker(&self) -> Option<WorkerId> {
        let ptr = self.context.as_ptr();
        // SAFETY: the function is atomic and therefore causes no data races.
        unsafe { (*ptr).worker() }
    }

    /// Requests for the task to be blocked.
    #[inline]
    pub fn request_block(&self) {
        let ptr = self.context.as_ptr();
        // SAFETY: the function is atomic and therefore causes no data races.
        unsafe { (*ptr).request_block() }
    }

    /// Requests for the task to be aborted.
    ///
    /// # Safety
    ///
    /// Aborting a task may lead to uninitialized data.
    #[inline]
    pub unsafe fn request_abort(&self) {
        let ptr = self.context.as_ptr();
        // SAFETY: the function is atomic and therefore causes no data races.
        (*ptr).request_abort()
    }

    /// Shows the requests.
    #[inline]
    pub fn peek_request(&self) -> StatusRequest {
        let ptr = self.context.as_ptr();
        // SAFETY: the function is atomic and therefore causes no data races.
        unsafe { (*ptr).peek_request() }
    }

    /// Extracts the current run status.
    #[inline]
    pub fn run_status(&self) -> TaskRunStatus {
        let ptr = self.context.as_ptr();
        // SAFETY: the function is atomic and therefore causes no data races.
        unsafe { (*ptr).run_status() }
    }

    /// Extracts the current schedule status.
    #[inline]
    pub fn schedule_status(&self) -> TaskScheduleStatus {
        let ptr = self.context.as_ptr();
        // SAFETY: the function is atomic and therefore causes no data races.
        unsafe { (*ptr).schedule_status() }
    }

    /// Checks whether the task is panicking.
    pub fn is_panicking(&self) -> bool {
        let ptr = self.context.as_ptr();
        // SAFETY: the function is atomic and therefore causes no data races.
        unsafe { (*ptr).is_panicking() }
    }
}

impl Debug for AtomicISchedulerContext<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AtomicISchedulerContext")
            .field("panicking", &self.is_panicking())
            .field("registered", &self.is_registered())
            .field("handle", &self.handle())
            .field("resume_time", &self.resume_time())
            .field("worker", &self.worker())
            .field("request", &self.peek_request())
            .field("run_status", &self.run_status())
            .field("schedule_status", &self.schedule_status())
            .finish()
    }
}

/// Interface of rust panics.
#[interface(
    uuid = "c16e06dc-1558-47a2-912e-ee59a3d1375e",
    vtable = "IRustPanicDataVTable",
    generate()
)]
pub trait IRustPanicData: IBase + Send {
    /// Takes the rust panic data.
    ///
    /// # Safety
    ///
    /// May only be called once.
    unsafe fn take_rust_panic_impl(&mut self) -> Box<dyn Any + Send + 'static>;
}

/// Extension trait for implementations of [`IRustPanicData`].
pub trait IRustPanicDataExt: IRustPanicData {
    /// Takes the rust panic data.
    fn take_rust_panic(mut p: ObjBox<Self>) -> Box<dyn Any + Send + 'static> {
        // Safety: we assume that the data has not been taken yet.
        unsafe { p.take_rust_panic_impl() }
    }
}

impl<T: IRustPanicData + ?Sized> IRustPanicDataExt for T {}
