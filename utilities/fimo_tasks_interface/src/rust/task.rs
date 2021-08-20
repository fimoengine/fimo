use crate::rust::{get_runtime, RawTask, Result, TaskHandle, TaskStatus};
use std::cell::UnsafeCell;
use std::marker::PhantomPinned;
use std::mem::MaybeUninit;
use std::pin::Pin;

/// A task.
#[derive(Debug)]
pub struct Task<T: 'static + Send> {
    data: UnsafeCell<MaybeUninit<T>>,
    raw: MaybeUninit<RawTask>,
    pin: PhantomPinned,
}

/// Status of a task.
#[derive(Debug, Copy, Clone, Hash, Ord, PartialOrd, PartialEq, Eq)]
pub enum TaskCompletionStatus<T> {
    /// Task is pending for completion.
    Pending,
    /// Task has finished successfully.
    Completed(T),
    /// Task has been aborted.
    Aborted,
}

impl<T: 'static + Send> Task<T> {
    /// Spawns a new task.
    ///
    /// # Panics
    ///
    /// **Must** be run from within a task.
    pub fn new(
        f: impl FnOnce() -> T + Send + 'static,
        dependencies: impl AsRef<[TaskHandle]>,
    ) -> Pin<Box<Self>> {
        // create the pin.
        let mut res = Box::pin(Self {
            data: UnsafeCell::new(MaybeUninit::uninit()),
            raw: MaybeUninit::uninit(),
            pin: Default::default(),
        });

        let task_func = {
            let res_ref = Pin::as_ref(&res);
            let data_ptr = unsafe { Pin::into_inner_unchecked(res_ref).data.get() as usize };
            move || {
                // casting is save, because `T` and `MaybeUninit<T>` have  the same layout.
                let data_ptr = data_ptr as *mut T;

                // write out result into the pointer.
                // we guarantee that we are the only one accessing the data.
                unsafe { data_ptr.write(f()) }
            }
        };

        // spawn task and write into the pin.
        let raw = get_runtime().spawn_task(task_func, dependencies);
        unsafe { Pin::get_unchecked_mut(res.as_mut()).raw.write(raw) };

        res
    }

    /// Polls whether the task is, or was, in the process
    /// of unwinding it's stack.
    pub fn panicking(&self) -> bool {
        let raw_ref = unsafe { &*self.raw.as_ptr() };
        raw_ref.panicking()
    }

    /// Polls the status of the task.
    ///
    /// Returns a reference to the task result if it has finished.
    pub fn poll(&self) -> TaskCompletionStatus<&T> {
        let raw_ref = unsafe { &*self.raw.as_ptr() };

        // check if the data has been written.
        match raw_ref.poll_status() {
            TaskStatus::Aborted => TaskCompletionStatus::Aborted,
            TaskStatus::Finished => {
                // the task has finished, fetch the data.
                let data_ptr = unsafe { (&*self.data.get()).as_ptr() };
                unsafe { TaskCompletionStatus::Completed(&*data_ptr) }
            }
            _ => TaskCompletionStatus::Pending,
        }
    }

    /// Polls the status of the task.
    ///
    /// Returns a mutable reference to the task result if it has finished.
    pub fn poll_mut(self: Pin<&mut Self>) -> TaskCompletionStatus<&mut T> {
        let raw_ref = unsafe { &*self.raw.as_ptr() };

        // check if the data has been written.
        match raw_ref.poll_status() {
            TaskStatus::Aborted => TaskCompletionStatus::Aborted,
            TaskStatus::Finished => {
                // the task has finished, fetch the data.
                let data_ptr = unsafe { (&mut *self.data.get()).as_mut_ptr() };
                unsafe { TaskCompletionStatus::Completed(&mut *data_ptr) }
            }
            _ => TaskCompletionStatus::Pending,
        }
    }

    /// Waits on the task to be completed.
    ///
    /// Returns a reference to the task result if the task could finish.
    ///
    /// # Panics
    ///
    /// **Must** be run from within a task.
    pub fn wait_on(&self) -> TaskCompletionStatus<&T> {
        // wait on the task and check its status.
        let raw_ref = unsafe { &*self.raw.as_ptr() };
        raw_ref.wait_on();
        self.poll()
    }

    /// Waits on the task to be completed.
    ///
    /// Returns a mutable reference to the task result if the task could finish.
    ///
    /// # Panics
    ///
    /// **Must** be run from within a task.
    pub fn wait_on_mut(self: Pin<&mut Self>) -> TaskCompletionStatus<&mut T> {
        // we won't move the data.
        let inner = unsafe { Pin::into_inner_unchecked(self) };

        // wait on the task and check its status.
        let raw_ref = unsafe { &*inner.raw.as_ptr() };
        raw_ref.wait_on();
        unsafe { Pin::new_unchecked(inner).poll_mut() }
    }

    /// Consumes the task and waits for it to finish.
    ///
    /// Returns the result of the task.
    ///
    /// # Panics
    ///
    /// **Must** be run from within a task.
    pub fn join(self: Pin<Box<Self>>) -> Result<T> {
        // we won't move the data.
        let mut inner = unsafe { Pin::into_inner_unchecked(self) };

        // join the raw to get the result.
        let raw = unsafe { inner.raw.as_mut_ptr().read() };
        let res = raw.join();

        // take a copy of the data, which may have been initialized.
        let data = unsafe { inner.data.get().read() };

        // transmute the box so that the `Task<T>` drop is not called.
        let uninit =
            unsafe { std::mem::transmute::<Box<Task<T>>, Box<MaybeUninit<Task<T>>>>(inner) };
        drop(uninit);

        match res {
            Ok(_) => {
                // take the data because we know it is initialized.
                let data = unsafe { data.assume_init() };
                Ok(data)
            }
            Err(e) => Err(e),
        }
    }

    /// Sends a termination signal to the task.
    ///
    /// Returns whether the task could be signaled.
    ///
    /// # Panics
    ///
    /// **Must** be run from within a task.
    ///
    /// # Safety
    ///
    /// Aborting a task can lead to uninitialized values.
    pub unsafe fn abort(self: Pin<&mut Self>) {
        // we won't move the data.
        let inner = Pin::into_inner_unchecked(self);

        let raw_ref = &mut *inner.raw.as_mut_ptr();
        raw_ref.abort();
    }
}

impl<T: 'static + Send> Drop for Task<T> {
    fn drop(&mut self) {
        // wait on the data to finish.
        // the data only needs to be dropped if the task completed successfully,
        // otherwise nothing has been written.
        if matches!(self.wait_on(), TaskCompletionStatus::Completed(_)) {
            // drop written data.
            let data = unsafe { self.data.get().read().assume_init() };
            drop(data);
        }

        // drop raw task.
        let raw = unsafe { self.raw.as_mut_ptr().read() };
        drop(raw);
    }
}

// SAFETY: if `T` is `Sync` it should be save to pass a reference
// to a `Task<T>` because a `Task<()>` is always `Sync`.
unsafe impl<T: 'static + Send + Sync> Sync for Task<T> {}
