use crate::task_scheduler::TaskSlot;
use crate::TaskRuntime;
use atomic::Atomic;
use context::Context;
use fimo_tasks_interface::rust::{NotifyFn, TaskHandle, TaskInner, TaskStatus, WaitOnFn, WorkerId};
use std::any::Any;
use std::cell::{Cell, RefCell};
use std::fmt::{Debug, Formatter};
use std::marker::PhantomPinned;
use std::mem::ManuallyDrop;
use std::ops::Deref;
use std::pin::Pin;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::Instant;

#[derive(Debug)]
pub(crate) struct RawTaskInner {
    pub handle: TaskHandle,
    pub panicking: Atomic<bool>,
    pub run_flag: Atomic<RunFlag>,
    pub wait_until: Cell<Instant>,
    pub task_status: Atomic<TaskStatus>,
    pub task_context: RefCell<Option<RawTaskInnerContext>>,
    pub panic_error: RefCell<Option<Box<dyn Any + Send + 'static>>>,
    pub _pinned: PhantomPinned,
}

unsafe impl Send for RawTaskInner {}
unsafe impl Sync for RawTaskInner {}

pub(crate) struct RawTaskInnerContext {
    pub task_slot: Option<TaskSlot>,
    pub context: Option<Context>,
    pub worker_id: Option<WorkerId>,
    pub function: Option<ManuallyDrop<Box<dyn FnOnce() + Send>>>,
}

#[derive(Debug, Copy, Clone)]
pub(crate) struct RawTaskInnerRef {
    pub task: *const RawTaskInner,
}

#[derive(Debug, Copy, Clone, Hash, Ord, PartialOrd, PartialEq, Eq)]
pub(crate) enum RunFlag {
    Run,
    Stop,
}

unsafe impl Send for RawTaskInnerRef {}

impl RawTaskInner {
    pub fn new(handle: TaskHandle) -> Self {
        Self {
            handle,
            panicking: Atomic::new(false),
            run_flag: Atomic::new(RunFlag::Stop),
            wait_until: Cell::new(Instant::now()),
            task_status: Atomic::new(TaskStatus::Blocked),
            task_context: RefCell::new(None),
            panic_error: RefCell::new(None),
            _pinned: Default::default(),
        }
    }

    pub fn pinned_box(handle: TaskHandle) -> Pin<Box<Self>> {
        Box::pin(Self::new(handle))
    }

    pub fn pinned_arc(handle: TaskHandle) -> Pin<Arc<Self>> {
        Arc::pin(Self::new(handle))
    }

    pub fn panicking(self: Pin<&Self>) -> bool {
        self.panicking.load(Ordering::Acquire)
    }

    pub fn take_panic_error(self: Pin<&Self>) -> Option<Box<dyn Any + Send + 'static>> {
        if let Ok(mut error) = self.panic_error.try_borrow_mut() {
            error.take()
        } else {
            None
        }
    }

    pub fn poll_status(self: Pin<&Self>) -> TaskStatus {
        self.task_status.load(Ordering::Acquire)
    }

    pub unsafe fn abort(self: Pin<&Self>) -> bool {
        self.run_flag
            .compare_exchange(
                RunFlag::Run,
                RunFlag::Stop,
                Ordering::AcqRel,
                Ordering::Acquire,
            )
            .is_ok()
    }

    pub fn poll_run_flag(self: Pin<&Self>) -> RunFlag {
        self.run_flag.load(Ordering::Acquire)
    }

    pub unsafe fn set_run_flag(self: Pin<&Self>, flag: RunFlag) {
        self.run_flag.store(flag, Ordering::Release)
    }

    pub unsafe fn set_status(self: Pin<&Self>, status: TaskStatus) {
        self.task_status.store(status, Ordering::Release)
    }

    pub unsafe fn as_raw_ref(self: Pin<&Self>) -> Pin<RawTaskInnerRef> {
        Pin::new_unchecked(RawTaskInnerRef {
            task: Pin::into_inner_unchecked(self) as *const _,
        })
    }
}

impl TaskInner for RawTaskInner {
    fn panicking(&self) -> bool {
        unsafe { RawTaskInner::panicking(Pin::new_unchecked(self)) }
    }

    unsafe fn take_panic_error(&self) -> Option<Box<dyn Any + Send + 'static>> {
        RawTaskInner::take_panic_error(Pin::new_unchecked(self))
    }

    fn is_completed(&self) -> bool {
        matches!(
            self.poll_status(),
            TaskStatus::Aborted | TaskStatus::Finished
        )
    }

    fn is_blocked(&self) -> bool {
        self.poll_status() == TaskStatus::Blocked
    }

    fn is_aborted(&self) -> bool {
        self.poll_status() == TaskStatus::Aborted
    }

    fn is_finished(&self) -> bool {
        self.poll_status() == TaskStatus::Finished
    }

    fn poll_status(&self) -> TaskStatus {
        unsafe { RawTaskInner::poll_status(Pin::new_unchecked(self)) }
    }

    fn get_handle(&self) -> TaskHandle {
        self.handle
    }

    unsafe fn abort(&self) -> bool {
        RawTaskInner::abort(Pin::new_unchecked(self))
    }

    unsafe fn unblock(&self) {
        if self.is_blocked() {
            TaskRuntime::unblock_task(self.get_handle())
        }
    }

    fn wait_on_if(&self, predicate: Option<WaitOnFn>) {
        if !self.is_completed() {
            TaskRuntime::wait_on_if(self.get_handle(), predicate)
        }
    }

    unsafe fn notify_finished_one_and_then(&self, and_then: Option<NotifyFn>) {
        if self.is_blocked() {
            TaskRuntime::notify_finished_one_and_then(self.get_handle(), and_then)
        }
    }

    unsafe fn broadcast_finished_and_then(&self, and_then: Option<NotifyFn>) {
        if self.is_blocked() {
            TaskRuntime::broadcast_finished_and_then(self.get_handle(), and_then)
        }
    }
}

impl Debug for RawTaskInnerContext {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RawTaskContext")
            .field("task_slot", &self.task_slot)
            .field("context", &self.context)
            .field("worker_id", &self.worker_id)
            .finish_non_exhaustive()
    }
}

impl Deref for RawTaskInnerRef {
    type Target = RawTaskInner;

    fn deref(&self) -> &Self::Target {
        unsafe { &*self.task }
    }
}
