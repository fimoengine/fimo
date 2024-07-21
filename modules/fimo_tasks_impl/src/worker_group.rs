use crate::{context::ContextImpl, worker_group::worker_thread::with_worker_context_lock};
use fimo_std::{
    error::Error,
    ffi::{FFISharable, FFITransferable},
};
use fimo_tasks::WorkerGroupId;
use std::{
    ffi::{CStr, CString},
    fmt::{Debug, Formatter},
    sync::{Arc, Weak},
};

pub mod command_buffer;
pub mod event_loop;
mod task;
pub mod worker_thread;

pub struct WorkerGroupImpl {
    id: WorkerGroupId,
    name: CString,
    event_loop: event_loop::EventLoopHandle,
    _ctx: Weak<ContextImpl>,
}

impl WorkerGroupImpl {
    pub fn id(&self) -> WorkerGroupId {
        self.id
    }

    pub fn name(&self) -> &CStr {
        &self.name
    }

    pub fn is_open(&self) -> bool {
        self.event_loop.is_open()
    }

    pub fn is_worker(&self) -> bool {
        with_worker_context_lock(|worker| {
            let group = Arc::as_ptr(&worker.group);
            std::ptr::eq(group, self)
        })
        .unwrap_or(false)
    }

    pub fn request_close(&self) -> Result<(), Error> {
        self.event_loop.request_close()?;
        todo!("unregister from context");
        Ok(())
    }
}

impl Debug for WorkerGroupImpl {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WorkerGroupImpl")
            .field("id", &self.id)
            .field("name", &self.name)
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
                workers: None,
                stack_sizes: None,
                enqueue_buffer: None,
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
