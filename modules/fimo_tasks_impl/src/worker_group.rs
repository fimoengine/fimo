use crate::{worker_group::worker_thread::with_worker_context_lock, RuntimeShared};
use event_loop::{stack_manager::StackDescriptor, EventLoopHandle};
use fimo_std::{
    error::Error,
    ffi::{FFISharable, FFITransferable},
};
use fimo_tasks::{bindings, WorkerGroupId};
use std::{
    ffi::{CStr, CString},
    fmt::Debug,
    sync::{Arc, RwLock},
};

pub mod command_buffer;
pub mod event_loop;
mod task;
pub mod worker_thread;

pub struct WorkerGroupImpl {
    id: WorkerGroupId,
    name: CString,
    visible: bool,
    event_loop: RwLock<Option<event_loop::EventLoopHandle>>,
    runtime: Arc<RuntimeShared>,
}

impl WorkerGroupImpl {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        ctx: fimo_std::context::ContextView<'_>,
        id: WorkerGroupId,
        name: CString,
        visible: bool,
        num_workers: usize,
        default_stack_size: usize,
        stacks: Vec<StackDescriptor>,
        runtime: Arc<RuntimeShared>,
    ) -> Arc<Self> {
        let _span = fimo_std::span_trace!(
            ctx,
            "id: {id:?}, name: {name:?}, visible: {visible:?}, num_workers: {num_workers:?}, \
            default_stack_size: {default_stack_size:?}, stacks: {stacks:?}, runtime: {runtime:?}"
        );
        fimo_std::emit_trace!(ctx, "constructing worker group");
        let this = Arc::new(Self {
            id,
            name,
            visible,
            event_loop: RwLock::new(None),
            runtime,
        });

        {
            let mut guard = this
                .event_loop
                .write()
                .expect("could not lock event loop handle");
            *guard = Some(EventLoopHandle::new(
                ctx,
                this.clone(),
                num_workers,
                default_stack_size,
                stacks,
            ));
        }

        this
    }

    pub fn id(&self) -> WorkerGroupId {
        self.id
    }

    pub fn name(&self) -> &CStr {
        &self.name
    }

    pub fn is_visible(&self) -> bool {
        self.visible
    }

    pub fn is_open(&self) -> bool {
        let guard = self
            .event_loop
            .read()
            .expect("failed to lock event loop handle");
        guard.as_ref().map_or(false, |h| h.is_open())
    }

    pub fn is_worker(&self) -> bool {
        with_worker_context_lock(|worker| {
            let group = Arc::as_ptr(&worker.group);
            std::ptr::eq(group, self)
        })
        .unwrap_or(false)
    }

    pub fn request_close(&self) -> Result<(), Error> {
        let guard = self
            .event_loop
            .read()
            .expect("failed to lock event loop handle");
        if let Some(handle) = guard.as_ref() {
            handle.request_close()?;
            self.runtime.shutdown_worker_group(self.id());
        }
        Ok(())
    }

    pub fn wait_for_close(&self) {
        self.request_close()
            .expect("could not request to close the event loop");

        let guard = self
            .event_loop
            .read()
            .expect("failed to lock event loop handle");
        if let Some(handle) = guard.as_ref() {
            handle.wait_for_close();
        }
    }
}

impl Debug for WorkerGroupImpl {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WorkerGroupImpl")
            .field("id", &self.id)
            .field("name", &self.name)
            .field("visible", &self.visible)
            .field("event_loop", &self.event_loop)
            .finish_non_exhaustive()
    }
}

#[derive(Debug)]
#[repr(transparent)]
pub struct WorkerGroupFFI(pub Arc<WorkerGroupImpl>);

impl WorkerGroupFFI {
    const VTABLE: &'static fimo_tasks::bindings::FiTasksWorkerGroupVTable =
        &fimo_tasks::bindings::FiTasksWorkerGroupVTable {
            v0: fimo_tasks::bindings::FiTasksWorkerGroupVTableV0 {
                id: Some(Self::id),
                acquire: Some(Self::acquire),
                release: Some(Self::release),
                is_open: Some(Self::is_open),
                is_worker: Some(Self::is_worker),
                name: Some(Self::name),
                request_close: Some(Self::request_close),
                workers: Some(Self::workers),
                stack_sizes: Some(Self::stack_sizes),
                enqueue_buffer: Some(Self::enqueue_buffer),
            },
        };

    unsafe extern "C" fn id(this: *mut std::ffi::c_void) -> usize {
        fimo_std::panic::abort_on_panic(|| {
            // Safety: Must be ensured by the caller.
            let this = unsafe { Self::borrow_from_ffi(this) };
            this.id().0
        })
    }

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

    unsafe extern "C" fn is_open(this: *mut std::ffi::c_void) -> bool {
        fimo_std::panic::abort_on_panic(|| {
            // Safety: Must be ensured by the caller.
            let this = unsafe { Self::borrow_from_ffi(this) };
            this.is_open()
        })
    }

    unsafe extern "C" fn is_worker(this: *mut std::ffi::c_void) -> bool {
        fimo_std::panic::abort_on_panic(|| {
            // Safety: Must be ensured by the caller.
            let this = unsafe { Self::borrow_from_ffi(this) };
            this.is_worker()
        })
    }

    unsafe extern "C" fn name(this: *mut std::ffi::c_void) -> *const std::ffi::c_char {
        fimo_std::panic::abort_on_panic(|| {
            // Safety: Must be ensured by the caller.
            let this = unsafe { Self::borrow_from_ffi(this) };
            this.name().as_ptr()
        })
    }

    unsafe extern "C" fn request_close(
        this: *mut std::ffi::c_void,
    ) -> fimo_std::bindings::FimoError {
        fimo_std::panic::catch_unwind(|| {
            // Safety: Must be ensured by the caller.
            let this = unsafe { Self::borrow_from_ffi(this) };
            this.request_close()
        })
        .flatten()
        .map_or_else(|e| e.into_error(), |_| Error::EOK.into_error())
    }

    unsafe extern "C" fn workers(
        _this: *mut std::ffi::c_void,
        _workers: *mut *mut usize,
        _count: *mut usize,
    ) -> fimo_std::bindings::FimoError {
        Error::ENOSYS.into_error()
    }

    unsafe extern "C" fn stack_sizes(
        _this: *mut std::ffi::c_void,
        _stack_sizes: *mut *mut usize,
        _count: *mut usize,
    ) -> fimo_std::bindings::FimoError {
        Error::ENOSYS.into_error()
    }

    unsafe extern "C" fn enqueue_buffer(
        _this: *mut std::ffi::c_void,
        _buffer: *const bindings::FiTasksCommandBuffer,
        _detached: bool,
        _handle: *mut bindings::FiTasksCommandBufferHandle,
    ) -> fimo_std::bindings::FimoError {
        Error::ENOSYS.into_error()
    }
}

impl FFISharable<*mut std::ffi::c_void> for WorkerGroupFFI {
    type BorrowedView<'a> = &'a WorkerGroupImpl;

    fn share_to_ffi(&self) -> *mut std::ffi::c_void {
        Arc::as_ptr(&self.0).cast_mut().cast()
    }

    unsafe fn borrow_from_ffi<'a>(ffi: *mut std::ffi::c_void) -> Self::BorrowedView<'a> {
        // Safety:
        unsafe { &*ffi.cast_const().cast() }
    }
}

impl FFITransferable<fimo_tasks::bindings::FiTasksWorkerGroup> for WorkerGroupFFI {
    fn into_ffi(self) -> fimo_tasks::bindings::FiTasksWorkerGroup {
        fimo_tasks::bindings::FiTasksWorkerGroup {
            data: Arc::into_raw(self.0).cast_mut().cast(),
            vtable: Self::VTABLE,
        }
    }

    unsafe fn from_ffi(ffi: fimo_tasks::bindings::FiTasksWorkerGroup) -> Self {
        // Safety:
        unsafe { WorkerGroupFFI(Arc::from_raw(ffi.data.cast_const().cast())) }
    }
}
