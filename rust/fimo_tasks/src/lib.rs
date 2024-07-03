//! Bindings to the fimo tasks symbols.
#![feature(allocator_api)]
#![feature(new_uninit)]

use std::{marker::PhantomData, time::Duration};

use fimo_std::{
    error::{to_result, to_result_indirect_in_place, Error},
    ffi::FFITransferable,
};

pub mod bindings;
pub mod symbols;

mod command_buffer;
mod local;
mod task;
mod worker_group;

pub use command_buffer::*;
use fimo_std::ffi::FFISharable;
pub use local::*;
pub use task::*;
pub use worker_group::*;

/// Context of runtime.
#[derive(Debug)]
#[repr(transparent)]
pub struct Context(bindings::FiTasksContext);

impl Context {
    /// Returns whether the current thread is a worker thread managed by some worker group of the
    /// context.
    ///
    /// Some operations can only be performed by worker threads.
    pub fn is_worker(&self) -> bool {
        // Safety: FFI call is safe
        unsafe { (self.vtable().v0.is_worker.unwrap_unchecked())(self.data()) }
    }

    /// Returns the unique id of the current task.
    ///
    /// The id will be unique for as long as the task is being executed, but may be reused by other
    /// tasks upon completion.
    ///
    /// Can only be called successfully from a task.
    pub fn task_id(&self) -> Result<TaskId, Error> {
        // Safety: FFI call is safe
        let id = unsafe {
            to_result_indirect_in_place(|err, id| {
                *err = (self.vtable().v0.task_id.unwrap_unchecked())(self.data(), id.as_mut_ptr());
            })?
        };

        Ok(TaskId(id))
    }

    /// Returns the unique id of the current worker.
    ///
    /// Can only be called successfully from a task.
    pub fn worker_id(&self) -> Result<WorkerId, Error> {
        // Safety: FFI call is safe
        let id = unsafe {
            to_result_indirect_in_place(|err, id| {
                *err =
                    (self.vtable().v0.worker_id.unwrap_unchecked())(self.data(), id.as_mut_ptr());
            })?
        };

        Ok(WorkerId(id))
    }

    /// Returns a handle to the current [`WorkerGroup`].
    ///
    /// Can only be called successfully from a task.
    pub fn worker_group(&self) -> Result<WorkerGroup<'_>, Error> {
        // Safety: FFI call is safe
        let group = unsafe {
            to_result_indirect_in_place(|err, group| {
                *err = (self.vtable().v0.worker_group.unwrap_unchecked())(
                    self.data(),
                    group.as_mut_ptr(),
                );
            })?
        };

        Ok(WorkerGroup(group, PhantomData))
    }

    /// Acquires a handle to a [`WorkerGroup`] assigned to the identifier `id`.
    pub fn worker_group_by_id(&self, id: WorkerGroupId) -> Result<WorkerGroup<'_>, Error> {
        // Safety: FFI call is safe
        let group = unsafe {
            to_result_indirect_in_place(|err, group| {
                *err = (self.vtable().v0.worker_group_by_id.unwrap_unchecked())(
                    self.data(),
                    id.0,
                    group.as_mut_ptr(),
                );
            })?
        };

        Ok(WorkerGroup(group, PhantomData))
    }

    /// Queries a list of [`WorkerGroup`]s available in the context.
    pub fn query_worker_groups(&self) -> Result<WorkerGroupQuery<'_>, Error> {
        // Safety: FFI call is safe
        let query = unsafe {
            to_result_indirect_in_place(|err, query| {
                *err = (self.vtable().v0.query_worker_groups.unwrap_unchecked())(
                    self.data(),
                    query.as_mut_ptr(),
                );
            })?
        };

        Ok(WorkerGroupQuery { query, ctx: self })
    }

    /// Yields the execution of the current task back to the scheduler.
    ///
    /// Yielding a task may allow other tasks to be scheduled. May only be called in a task.
    pub fn yield_now(&self) -> Result<(), Error> {
        // Safety: FFI call is safe
        unsafe { to_result((self.vtable().v0.yield_.unwrap_unchecked())(self.data())) }
    }

    /// Pauses the execution of the current task for the specified duration.
    pub fn sleep(&self, duration: Duration) -> Result<(), Error> {
        let secs = duration.as_secs();
        let nanos = duration.subsec_nanos();
        let duration = fimo_std::time::Duration::new(secs, nanos);

        // Safety: FFI call is safe
        unsafe {
            to_result((self.vtable().v0.sleep.unwrap_unchecked())(
                self.data(),
                duration.into_ffi(),
            ))
        }
    }

    #[inline(always)]
    fn data(&self) -> *mut std::ffi::c_void {
        self.0.data
    }

    #[inline(always)]
    fn vtable(&self) -> &bindings::FiTasksVTable {
        // Safety: The VTable is always initialized
        unsafe { &*self.0.vtable }
    }
}

// Safety: Sound by invariant
unsafe impl Send for Context {}

// Safety: Sound by invariant
unsafe impl Sync for Context {}

impl FFISharable<bindings::FiTasksContext> for Context {
    type BorrowedView<'a> = std::convert::Infallible;

    fn share_to_ffi(&self) -> bindings::FiTasksContext {
        self.0
    }

    unsafe fn borrow_from_ffi<'a>(_ffi: bindings::FiTasksContext) -> Self::BorrowedView<'a> {
        unreachable!("can not borrow a ffi context")
    }
}
