use crate::{bindings, Context};
use std::{
    alloc::Allocator,
    any::Any,
    cell::UnsafeCell,
    ffi::CString,
    marker::PhantomData,
    mem::MaybeUninit,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};

/// A unique identifier for a task.
///
/// The id is guaranteed to be unique for as long as the task has not finished its execution.
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TaskId(pub(super) usize);

/// Status of a task that has finished being executed by the runtime.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum TaskStatus {
    /// Task has run to completion successfully.
    Completed,
    /// Task has been aborted.
    Aborted,
}

#[repr(C)]
#[derive(Debug)]
pub(super) struct RawTask<'a, A> {
    raw: bindings::FiTasksTask,
    allocator: A,
    label: Option<CString>,
    drop_raw: unsafe fn(*mut bindings::FiTasksTask),
    _phantom: PhantomData<dyn FnOnce() + 'a>,
}

impl<'a, A> RawTask<'a, A>
where
    A: Allocator + Clone + Send + 'a,
{
    pub(super) fn new_in<F, S>(label: Option<CString>, f: F, s: S, allocator: A) -> Box<Self, A>
    where
        F: FnOnce(&Context) -> Result<(), ()> + Send + 'a,
        S: FnOnce(TaskStatus) + Send + 'a,
    {
        unsafe extern "C" fn start<'a, F, A>(
            data: *mut std::ffi::c_void,
            task: *mut bindings::FiTasksTask,
            context: bindings::FiTasksContext,
        ) where
            F: FnOnce(&Context) -> Result<(), ()> + 'a,
            A: Allocator + Clone,
        {
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(move || {
                // Safety: We are the only ones with a reference to the task.
                let task = unsafe { &mut *task.cast::<RawTask<'_, A>>() };
                let allocator = task.allocator.clone();

                // Safety: We know that the closure is contained in a `Box`.
                let f = unsafe { Box::from_raw_in(data.cast::<F>(), allocator) };
                task.raw.user_data = std::ptr::null_mut();

                f(&Context(context))
            }));

            let abort = !matches!(result, Ok(Ok(_)));
            drop(result);

            if abort {
                let context = Context(context);
                // Safety: FFI call is safe
                unsafe {
                    (context.vtable().v0.abort.unwrap_unchecked())(
                        context.data(),
                        std::ptr::null_mut(),
                    )
                };

                // In case of an error from the abort operation we abort the entire process.
                std::process::abort();
            }
        }

        unsafe extern "C" fn on_complete<'a, S, A>(
            data: *mut std::ffi::c_void,
            task: *mut bindings::FiTasksTask,
        ) where
            S: FnOnce(TaskStatus) + 'a,
            A: Allocator + Clone,
        {
            fimo_std::panic::abort_on_panic(|| {
                // Safety: We are the only ones with a reference to the task.
                let task = unsafe { &mut *task.cast::<RawTask<'_, A>>() };
                let allocator = task.allocator.clone();

                // Safety: We know that the closure is contained in a `Box`.
                let f = unsafe { Box::from_raw_in(data.cast::<S>(), allocator) };
                task.raw.status_callback_data = std::ptr::null_mut();

                f(TaskStatus::Completed);
            });
        }

        unsafe extern "C" fn on_abort<'a, S, A>(
            data: *mut std::ffi::c_void,
            task: *mut bindings::FiTasksTask,
            _error: *mut std::ffi::c_void,
        ) where
            S: FnOnce(TaskStatus) + 'a,
            A: Allocator + Clone,
        {
            fimo_std::panic::abort_on_panic(|| {
                // Safety: We are the only ones with a reference to the task.
                let task = unsafe { &mut *task.cast::<RawTask<'_, A>>() };
                let allocator = task.allocator.clone();

                // Safety: We know that the closure is contained in a `Box`.
                let f = unsafe { Box::from_raw_in(data.cast::<S>(), allocator) };
                task.raw.status_callback_data = std::ptr::null_mut();

                f(TaskStatus::Aborted);
            });
        }

        unsafe extern "C" fn on_cleanup<A>(
            _data: *mut std::ffi::c_void,
            task: *mut bindings::FiTasksTask,
        ) where
            A: Allocator + Clone,
        {
            fimo_std::panic::abort_on_panic(|| {
                // Safety: We are the only ones with a reference to the task.
                let task = unsafe { &mut *task.cast::<RawTask<'_, A>>() };
                let allocator = task.allocator.clone();

                // Safety: We know that the task is contained in a `Box`.
                unsafe { drop(Box::from_raw_in(task, allocator)) };
            });
        }

        unsafe fn on_drop<'a, F, S, A>(task: *mut bindings::FiTasksTask)
        where
            F: FnOnce(&Context) -> Result<(), ()> + 'a,
            S: FnOnce(TaskStatus) + 'a,
            A: Allocator + Clone,
        {
            // Safety: We are the only ones with a reference to the task.
            let task = unsafe { &mut *task.cast::<RawTask<'_, A>>() };

            if !task.raw.user_data.is_null() {
                let allocator = task.allocator.clone();
                // Safety: We know that the closure is contained in a `Box`.
                unsafe {
                    drop(Box::from_raw_in(task.raw.user_data.cast::<F>(), allocator));
                }
                task.raw.user_data = std::ptr::null_mut();
            }

            if !task.raw.status_callback_data.is_null() {
                let allocator = task.allocator.clone();
                // Safety: We know that the closure is contained in a `Box`.
                unsafe {
                    drop(Box::from_raw_in(
                        task.raw.status_callback_data.cast::<S>(),
                        allocator,
                    ));
                }
                task.raw.status_callback_data = std::ptr::null_mut();
            }
        }

        // First allocate the box, so that an out of memory panic does not cause
        // the destructor of the task to be called.
        let mut x = Box::new_uninit_in(allocator.clone());

        let f = Box::new_in(f, allocator.clone());
        let s = Box::new_in(s, allocator.clone());

        let label_ffi = match label.as_ref() {
            None => std::ptr::null(),
            Some(label) => label.as_ptr(),
        };

        x.write(Self {
            raw: bindings::FiTasksTask {
                label: label_ffi,
                start: Some(start::<F, A>),
                user_data: Box::into_raw(f).cast(),
                on_complete: Some(on_complete::<S, A>),
                on_abort: Some(on_abort::<S, A>),
                status_callback_data: Box::into_raw(s).cast(),
                on_cleanup: Some(on_cleanup::<A>),
                cleanup_data: std::ptr::null_mut(),
            },
            allocator,
            label,
            drop_raw: on_drop::<F, S, A>,
            _phantom: PhantomData,
        });

        // Safety: We initialized the box
        unsafe { x.assume_init() }
    }
}

impl<A> Drop for RawTask<'_, A> {
    fn drop(&mut self) {
        // Safety: The `drop_raw` function is safe to call since the task is being dropped.
        unsafe {
            (self.drop_raw)(&mut self.raw);
        }
    }
}

// Safety: A `RawTask` is composed of only `Send` closures.
unsafe impl<A: Send> Send for RawTask<'_, A> {}

/// Handle to an enqueued task.
pub struct TaskHandle<T, A: Allocator> {
    pub(super) inner: Arc<TaskHandleInner<T>, A>,
}

pub(super) struct TaskHandleInner<T> {
    pub(super) completed: AtomicBool,
    pub(super) value: UnsafeCell<MaybeUninit<Result<T, Box<dyn Any + Send + 'static>>>>,
}

// Safety: Is sound as a `TaskHandleInner` essentially works like a `Mutex` on `T`.
unsafe impl<T> Send for TaskHandleInner<T> where T: Send {}

// Safety: See above.
unsafe impl<T> Sync for TaskHandleInner<T> where T: Send {}

impl<T, A> TaskHandle<T, A>
where
    A: Allocator,
{
    /// Returns whether the task has been completed.
    pub fn is_completed(&self) -> bool {
        self.inner.completed.load(Ordering::Acquire)
    }

    /// Returns the completion status of the task, if it has finished executing.
    pub fn completion_status(&self) -> Option<TaskStatus> {
        if !self.is_completed() {
            None
        } else {
            // Safety: The task has been completed, therefore we are allowed to read from value.
            let value = unsafe { (*self.inner.value.get()).assume_init_ref() };

            match value {
                Ok(_) => Some(TaskStatus::Completed),
                Err(_) => Some(TaskStatus::Aborted),
            }
        }
    }

    /// Extracts the result of the task.
    ///
    /// # Panics
    ///
    /// The result can only be read after the task has been completed. Trying to do it otherwise
    /// will result in a panic.
    pub fn unwrap(self) -> Result<T, Box<dyn Any + Send + 'static>> {
        if !self.is_completed() {
            panic!("the task has not been run to completion.")
        }

        // Safety: We know that the task has been completed, i.e., it has initialized
        // the value.
        unsafe {
            let value_ref = &*self.inner.value.get();
            value_ref.assume_init_read()
        }
    }

    /// Extracts a reference to the result of the task.
    ///
    /// # Panics
    ///
    /// The result can only be read after the task has been completed. Trying to do it otherwise
    /// will result in a panic.
    pub fn unwrap_ref(&self) -> &Result<T, Box<dyn Any + Send + 'static>> {
        if !self.is_completed() {
            panic!("the task has not been run to completion.")
        }

        // Safety: We know that the task has been completed, i.e., it has initialized
        // the value.
        unsafe {
            let value_ref = &*self.inner.value.get();
            value_ref.assume_init_ref()
        }
    }

    /// Extracts a mutable reference to the result of the task.
    ///
    /// # Panics
    ///
    /// The result can only be read after the task has been completed. Trying to do it otherwise
    /// will result in a panic.
    pub fn unwrap_mut(&mut self) -> &mut Result<T, Box<dyn Any + Send + 'static>> {
        if !self.is_completed() {
            panic!("the task has not been run to completion.")
        }

        // Safety: We know that the task has been completed, i.e., it has initialized
        // the value.
        unsafe {
            let value_ref = &mut *self.inner.value.get();
            value_ref.assume_init_mut()
        }
    }
}
