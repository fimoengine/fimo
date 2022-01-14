use crate::raw_task::{RawTaskInner, RawTaskInnerRef};
use crate::task_scheduler::{NewTaskStatus, TaskScheduler};
use crate::task_worker::{TaskWorker, WORKER};
use crate::TaskRuntime;
use atomic::Atomic;
use crossbeam_deque::{Injector, Stealer};
use fimo_tasks_int::rust::{NotifyFn, RawTask, Result, TaskHandle, TaskInner, WorkerId};
use parking_lot::{Condvar, Mutex};
use std::marker::PhantomPinned;
use std::pin::Pin;
use std::sync::atomic::Ordering;
use std::sync::{Arc, Barrier};

#[derive(Debug)]
pub(crate) struct TaskRuntimeInner {
    pub task_workers: Vec<TaskWorker>,
    pub running_workers: Atomic<usize>,
    pub global_queue: Injector<Pin<RawTaskInnerRef>>,
    pub task_stealer: Vec<Stealer<Pin<RawTaskInnerRef>>>,
    pub task_scheduler: Mutex<TaskScheduler>,
    _pinned: PhantomPinned,
}

unsafe impl Sync for TaskRuntimeInner {}

impl TaskRuntimeInner {
    pub fn new(workers: usize, max_tasks: usize, allocated_tasks: usize) -> Pin<Box<Self>> {
        let (sx, rx) = std::sync::mpsc::channel();

        let mut runtime = Box::pin(Self {
            task_workers: Vec::with_capacity(workers),
            running_workers: Atomic::new(0),
            global_queue: Default::default(),
            task_stealer: Vec::with_capacity(workers),
            task_scheduler: parking_lot::Mutex::new(TaskScheduler::new(
                max_tasks + workers,
                allocated_tasks + workers,
                rx,
            )),
            _pinned: Default::default(),
        });

        // barrier for synchronising all workers with the main thread.
        let sync = Arc::new(Barrier::new(workers + 1));

        for i in 0..workers {
            let worker_id = WorkerId(i);
            let runtime_ref = unsafe {
                Pin::new_unchecked(&mut *(Pin::into_inner_unchecked(runtime.as_mut()) as *mut _))
            };

            let worker = TaskWorker::new(worker_id, Arc::clone(&sync), sx.clone(), runtime_ref);

            // create and start the new worker.
            unsafe {
                let runtime = Pin::into_inner_unchecked(runtime.as_mut());
                runtime.task_workers.push(worker);

                let stealer = runtime.task_workers[worker_id.0].start_worker();
                runtime.task_stealer.push(stealer);
            }
        }

        sync.wait();

        runtime
    }

    pub fn spawn_task(
        self: Pin<&Self>,
        function: Option<Box<dyn FnOnce() + Send>>,
        dependencies: &[TaskHandle],
        blocked: NewTaskStatus,
        worker_id: Option<WorkerId>,
    ) -> RawTask {
        // we won't move the runtime.
        let runtime = unsafe { Pin::into_inner_unchecked(self) };

        if WORKER.with(|w| w.get()).is_some() {
            let mut guard = runtime.task_scheduler.try_lock();
            while guard.is_none() {
                TaskRuntime::yield_now();
                guard = runtime.task_scheduler.try_lock();
            }

            guard
                .unwrap()
                .spawn_task(function, dependencies, blocked, worker_id)
        } else {
            let mut guard = runtime.task_scheduler.lock();
            guard.spawn_task(function, dependencies, blocked, worker_id)
        }
    }

    pub fn execute_task(
        self: Pin<&Self>,
        function: Option<Box<dyn FnOnce() + Send>>,
        dependencies: &[TaskHandle],
    ) -> Result<()> {
        // we won't move the runtime.
        let runtime = unsafe { Pin::into_inner_unchecked(self) };

        if WORKER.with(|w| w.get()).is_some() {
            panic!("execute_task can not be called from a worker thread.")
        } else {
            let mut guard = runtime.task_scheduler.lock();
            let cond_var = Arc::new(Condvar::new());
            let cond_var_clone = Arc::clone(&cond_var);

            let task = RawTaskInner::new(guard.create_handle());
            let pinned = unsafe { Pin::new_unchecked(&task) };
            unsafe {
                guard.initialize_task(
                    pinned,
                    function,
                    dependencies,
                    NewTaskStatus::Runnable,
                    None,
                    Some(cond_var_clone),
                    None,
                )
            };

            // wait until the task has finished.
            cond_var.wait(&mut guard);
            if pinned.is_aborted() {
                Err(pinned.take_panic_error())
            } else {
                Ok(())
            }
        }
    }

    pub unsafe fn broadcast_finished(task: TaskHandle, and_then: Option<NotifyFn>) {
        let worker = WORKER.with(|w| w.get().unwrap());

        // we won't move the runtime.
        let runtime = Pin::into_inner_unchecked(worker.runtime);

        let mut guard = runtime.task_scheduler.try_lock();
        while guard.is_none() {
            TaskRuntime::yield_now();
            guard = runtime.task_scheduler.try_lock();
        }
        let mut guard = guard.unwrap();
        guard.broadcast_finished(task, and_then);
    }

    pub unsafe fn notify_finished_one(task: TaskHandle, and_then: Option<NotifyFn>) {
        let worker = WORKER.with(|w| w.get().unwrap());

        // we won't move the runtime.
        let runtime = Pin::into_inner_unchecked(worker.runtime);

        let mut guard = runtime.task_scheduler.try_lock();
        while guard.is_none() {
            TaskRuntime::yield_now();
            guard = runtime.task_scheduler.try_lock();
        }
        let mut guard = guard.unwrap();
        guard.notify_finished_one(task, and_then);
    }

    pub unsafe fn unblock_task(task: TaskHandle) {
        let worker = WORKER.with(|w| w.get().unwrap());

        // we won't move the runtime.
        let runtime = Pin::into_inner_unchecked(worker.runtime);

        let mut guard = runtime.task_scheduler.try_lock();
        while guard.is_none() {
            TaskRuntime::yield_now();
            guard = runtime.task_scheduler.try_lock();
        }
        let mut guard = guard.unwrap();
        guard.unblock_task(task);
    }

    pub fn num_workers(&self) -> usize {
        self.task_workers.len()
    }
}

impl Drop for TaskRuntimeInner {
    fn drop(&mut self) {
        for worker in &mut self.task_workers {
            unsafe { worker.worker_task.as_ref().abort() };
        }

        while self.running_workers.load(Ordering::Acquire) != 0 {
            std::thread::yield_now();
        }

        for worker in &mut self.task_workers {
            let thread = worker.thread.take().unwrap();
            thread.join().unwrap();
        }
    }
}
