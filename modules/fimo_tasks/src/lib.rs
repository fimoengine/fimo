//! Implementation of the `fimo-tasks` interface.
#![feature(c_unwind)]
#![warn(
    missing_docs,
    rust_2018_idioms,
    missing_debug_implementations,
    rustdoc::broken_intra_doc_links
)]
use fimo_tasks_int::rust::{NotifyFn, RawTask, Result, SpawnAllFn, TaskHandle, WaitOnFn, WorkerId};
use std::fmt::Debug;
use std::pin::Pin;
use std::time::Instant;

mod raw_task;
mod task_runtime_inner;
mod task_scheduler;
mod task_worker;

#[cfg(feature = "module")]
pub mod module;

use task_runtime_inner::TaskRuntimeInner;
use task_scheduler::NewTaskStatus;
use task_worker::{TaskWorker, WORKER};

#[cfg(feature = "module")]
pub use module::MODULE_NAME;

/// The runtime that manages the execution of tasks.
#[derive(Debug)]
pub struct TaskRuntime {
    inner: Pin<Box<TaskRuntimeInner>>,
}

impl TaskRuntime {
    /// Constructs a new `TaskRuntime`.
    pub fn new(workers: usize, max_tasks: usize, allocated_tasks: usize) -> Self {
        Self {
            inner: TaskRuntimeInner::new(workers, max_tasks, allocated_tasks),
        }
    }

    /// Enters the runtime with a function.
    ///
    /// The current thread is blocked until the task has run.
    /// Like [`TaskRuntime::execute_task`] but panics if the task was
    /// aborted with a panic.
    ///
    /// # Note
    ///
    /// Can **must** called from outside the runtime.
    pub fn enter_runtime(&self, f: impl FnOnce() + Send) {
        if let Err(Some(e)) = self.execute_task(f, &[]) {
            std::panic::resume_unwind(e)
        }
    }

    /// Spawns and waits on a new task,
    ///
    /// Returns the status and whether the task panicked.
    ///
    /// # Note
    ///
    /// Can **must** called from outside the runtime.
    pub fn execute_task(&self, f: impl FnOnce() + Send, dependencies: &[TaskHandle]) -> Result<()> {
        // we can elide the lifetime because we're going to wait for the task to finish.
        let boxed = unsafe {
            std::mem::transmute::<Box<dyn FnOnce() + Send>, Box<dyn FnOnce() + Send + 'static>>(
                Box::new(f),
            )
        };
        self.inner.as_ref().execute_task(Some(boxed), dependencies)
    }

    /// Spawns a new task.
    ///
    /// # Panics
    ///
    /// **Must** be run from within a task.
    pub fn spawn_task(f: impl FnOnce() + Send + 'static, dependencies: &[TaskHandle]) -> RawTask {
        let worker = WORKER.with(|w| w.get().unwrap());
        worker.runtime.spawn_task(
            Some(Box::new(f)),
            dependencies,
            NewTaskStatus::Runnable,
            None,
        )
    }

    /// Spawns a new task for each worker thread.
    ///
    /// # Panics
    ///
    /// **Must** be run from within a task.
    pub fn spawn_all(
        f: impl FnOnce() + Send + Clone + 'static,
        dependencies: &[TaskHandle],
    ) -> RawTask {
        let worker = WORKER.with(|w| w.get().unwrap());

        let num_tasks = worker.runtime.num_workers();
        let mut tasks: Vec<_> = (0..num_tasks - 1)
            .map(|i| {
                let worker_id = WorkerId(i);
                worker.runtime.spawn_task(
                    Some(Box::new(f.clone())),
                    dependencies,
                    NewTaskStatus::Runnable,
                    Some(worker_id),
                )
            })
            .collect();
        tasks.push(worker.runtime.spawn_task(
            Some(Box::new(f)),
            dependencies,
            NewTaskStatus::Runnable,
            Some(WorkerId(num_tasks - 1)),
        ));

        let dependencies: Vec<_> = tasks.iter().map(|t| t.get_handle()).collect();
        TaskRuntime::spawn_task(
            move || {
                if tasks.iter().any(|task| task.is_aborted()) {
                    drop(tasks);
                    unsafe { TaskRuntime::abort() };
                }
            },
            dependencies.as_slice(),
        )
    }

    /// Spawns a new empty task.
    ///
    /// # Panics
    ///
    /// **Must** be run from within a task.
    pub fn spawn_empty(dependencies: &[TaskHandle]) -> RawTask {
        let worker = WORKER.with(|w| w.get().unwrap());
        worker
            .runtime
            .spawn_task(None, dependencies, NewTaskStatus::Runnable, None)
    }

    /// Spawns a new blocked task.
    ///
    /// # Note
    ///
    /// The task must be unblocked before dropping or
    /// else will wait forever.
    ///
    /// # Panics
    ///
    /// **Must** be run from within a task.
    pub fn spawn_task_blocked(
        f: impl FnOnce() + Send + 'static,
        dependencies: &[TaskHandle],
    ) -> RawTask {
        let worker = WORKER.with(|w| w.get().unwrap());
        worker.runtime.spawn_task(
            Some(Box::new(f)),
            dependencies,
            NewTaskStatus::Blocked,
            None,
        )
    }

    /// Spawns a new blocked empty task.
    ///
    /// # Note
    ///
    /// The task must be unblocked before dropping or
    /// else will wait forever.
    ///
    /// # Panics
    ///
    /// **Must** be run from within a task.
    pub fn spawn_empty_blocked(dependencies: &[TaskHandle]) -> RawTask {
        let worker = WORKER.with(|w| w.get().unwrap());
        worker
            .runtime
            .spawn_task(None, dependencies, NewTaskStatus::Blocked, None)
    }

    /// Yields the current task.
    ///
    /// # Panics
    ///
    /// **Must** be run from within a task.
    pub fn yield_now() {
        TaskWorker::yield_now();
    }

    /// Yields the current task until a minimum time has reached.
    ///
    /// # Panics
    ///
    /// **Must** be run from within a task.
    pub fn yield_until(instant: Instant) {
        TaskWorker::yield_until(instant);
    }

    /// Blocks the current task indefinitely.
    ///
    /// # Panics
    ///
    /// **Must** be run from within a task.
    ///
    /// # Safety
    ///
    /// The task must be unblocked before it is dropped.
    pub unsafe fn block() {
        TaskWorker::block();
    }

    /// Blocks the current task until the other task has completed.
    ///
    /// # Panics
    ///
    /// **Must** be run from within a task.
    pub fn wait_on(task: TaskHandle) {
        TaskRuntime::wait_on_if(task, None)
    }

    /// Blocks the current task until the other task has completed.
    ///
    /// See [`WaitOnFn`] for more information on the properties of the predicate.
    ///
    /// # Panics
    ///
    /// **Must** be run from within a task.
    pub fn wait_on_if(task: TaskHandle, predicate: Option<WaitOnFn>) {
        TaskWorker::wait_on(task, predicate);
    }

    /// Aborts the current task.
    ///
    /// # Panics
    ///
    /// **Must** be run from within a task.
    ///
    /// # Safety
    ///
    /// Aborting can lead to uninitialized values.
    pub unsafe fn abort() -> ! {
        // abort and cleanup.
        TaskWorker::abort(true);
    }

    /// Notifies all waiters that the task has finished
    /// without changing the status.
    ///
    /// # Panics
    ///
    /// **Must** be run from within a task.
    ///
    /// # Safety
    ///
    /// See [`TaskRuntime::notify_finished_one_and_then()`].
    pub unsafe fn broadcast_finished(task: TaskHandle) {
        TaskRuntime::broadcast_finished_and_then(task, None);
    }

    /// Notifies all waiters that the task has finished
    /// without changing the status.
    ///
    /// # Panics
    ///
    /// **Must** be run from within a task.
    ///
    /// # Safety
    ///
    /// See [`TaskRuntime::notify_finished_one_and_then()`].
    pub unsafe fn broadcast_finished_and_then(task: TaskHandle, after_wake: Option<NotifyFn>) {
        TaskRuntimeInner::broadcast_finished(task, after_wake);
    }

    /// Notifies one waiter that the task has finished
    /// without changing the status.
    ///
    /// # Panics
    ///
    /// **Must** be run from within a task.
    ///
    /// # Safety
    ///
    /// See [`TaskRuntime::notify_finished_one_and_then()`].
    pub unsafe fn notify_finished_one(task: TaskHandle) {
        TaskRuntime::notify_finished_one_and_then(task, None);
    }

    /// Notifies one waiter that the task has finished
    /// without changing the status.
    ///
    /// # Panics
    ///
    /// **Must** be run from within a task.
    ///
    /// # Safety
    ///
    /// A waiting task may require that the task fully finishes before
    /// resuming execution. This function is mainly intended to be
    /// used for the implementation of condition variables.
    ///
    /// The `after_wake` function will be called while the runtime is locked
    /// and is not allowed to call into the runtime or panic.
    pub unsafe fn notify_finished_one_and_then(task: TaskHandle, after_wake: Option<NotifyFn>) {
        TaskRuntimeInner::notify_finished_one(task, after_wake);
    }

    /// Unblocks a task.
    ///
    /// # Panics
    ///
    /// **Must** be run from within a task.
    ///
    /// # Safety
    ///
    /// Some tasks are meant to remain blocked until they are dropped.
    pub unsafe fn unblock_task(task: TaskHandle) {
        TaskRuntimeInner::unblock_task(task);
    }
}

impl fimo_tasks_int::rust::TaskRuntimeInner for TaskRuntime {
    fn execute_task(&self, f: Box<dyn FnOnce() + Send>, dependencies: &[TaskHandle]) -> Result<()> {
        self.inner.as_ref().execute_task(Some(f), dependencies)
    }

    fn spawn_task(&self, f: Box<dyn FnOnce() + Send>, dependencies: &[TaskHandle]) -> RawTask {
        let worker = WORKER.with(|w| w.get().unwrap());
        worker
            .runtime
            .spawn_task(Some(f), dependencies, NewTaskStatus::Runnable, None)
    }

    fn spawn_all(&self, f: Box<dyn SpawnAllFn>, dependencies: &[TaskHandle]) -> RawTask {
        let worker = WORKER.with(|w| w.get().unwrap());

        let num_tasks = worker.runtime.num_workers();
        let mut tasks: Vec<_> = (0..num_tasks - 1)
            .map(|i| {
                let worker_id = WorkerId(i);
                worker.runtime.spawn_task(
                    Some(f.boxed_clone()),
                    dependencies,
                    NewTaskStatus::Runnable,
                    Some(worker_id),
                )
            })
            .collect();

        let f = Box::leak(f).as_fn_once();
        let f = unsafe { Box::from_raw(f) };

        tasks.push(worker.runtime.spawn_task(
            Some(f),
            dependencies,
            NewTaskStatus::Runnable,
            Some(WorkerId(num_tasks - 1)),
        ));

        let dependencies: Vec<_> = tasks.iter().map(|t| t.get_handle()).collect();
        TaskRuntime::spawn_task(
            move || {
                if tasks.iter().any(|task| task.is_aborted()) {
                    drop(tasks);
                    unsafe { TaskRuntime::abort() };
                }
            },
            dependencies.as_slice(),
        )
    }

    fn spawn_empty(&self, dependencies: &[TaskHandle]) -> RawTask {
        TaskRuntime::spawn_empty(dependencies)
    }

    fn spawn_task_blocked(
        &self,
        f: Box<dyn FnOnce() + Send>,
        dependencies: &[TaskHandle],
    ) -> RawTask {
        let worker = WORKER.with(|w| w.get().unwrap());
        worker
            .runtime
            .spawn_task(Some(f), dependencies, NewTaskStatus::Blocked, None)
    }

    fn spawn_empty_blocked(&self, dependencies: &[TaskHandle]) -> RawTask {
        TaskRuntime::spawn_empty_blocked(dependencies)
    }

    fn yield_now(&self) {
        TaskRuntime::yield_now()
    }

    fn yield_until(&self, instant: Instant) {
        TaskRuntime::yield_until(instant)
    }

    unsafe fn block(&self) {
        TaskRuntime::block()
    }

    unsafe fn unblock_task(&self, task: TaskHandle) {
        TaskRuntime::unblock_task(task)
    }

    fn wait_on_if(&self, task: TaskHandle, predicate: Option<WaitOnFn>) {
        TaskRuntime::wait_on_if(task, predicate)
    }

    unsafe fn abort(&self) -> ! {
        TaskRuntime::abort()
    }

    unsafe fn broadcast_finished_and_then(&self, task: TaskHandle, and_then: Option<NotifyFn>) {
        TaskRuntime::broadcast_finished_and_then(task, and_then)
    }

    unsafe fn notify_finished_one_and_then(&self, task: TaskHandle, and_then: Option<NotifyFn>) {
        TaskRuntime::notify_finished_one_and_then(task, and_then)
    }

    fn get_worker_id(&self) -> WorkerId {
        let worker = WORKER.with(|w| w.get().unwrap());
        worker.id
    }
}

#[cfg(test)]
mod tests {
    use crate::TaskRuntime;
    use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
    use std::sync::{Arc, Barrier};
    use std::time::{Duration, Instant};

    #[test]
    fn spawning() {
        let cpus = num_cpus::get();
        let tasks = 1024;
        let allocated_tasks = 128;

        let runtime = TaskRuntime::new(cpus, tasks, allocated_tasks);
        runtime.enter_runtime(|| println!("Hello task!"));
        drop(runtime);
    }

    #[test]
    #[should_panic]
    fn panic() {
        let cpus = num_cpus::get();
        let tasks = 1024;
        let allocated_tasks = 128;

        let runtime = TaskRuntime::new(cpus, tasks, allocated_tasks);
        runtime.enter_runtime(|| panic!("Hello panic!"));
        drop(runtime);
    }

    #[test]
    fn yield_until() {
        let cpus = num_cpus::get();
        let tasks = 1024;
        let allocated_tasks = 128;

        let runtime = TaskRuntime::new(cpus, tasks, allocated_tasks);
        runtime.enter_runtime(|| {
            let current = Instant::now();
            let until = current + Duration::from_millis(500);

            TaskRuntime::yield_until(until);

            assert!(until <= Instant::now())
        });
        drop(runtime);
    }

    #[test]
    fn signal_tasks() {
        let cpus = num_cpus::get();
        let tasks = 1024;
        let allocated_tasks = 128;

        let runtime = TaskRuntime::new(cpus, tasks, allocated_tasks);
        runtime.enter_runtime(move || {
            println!("Worker entered.");

            let t1_run = Arc::new(AtomicBool::new(false));
            let t2_run = Arc::new(AtomicBool::new(false));

            let wait_barrier = Arc::new(AtomicBool::new(true));
            let wait_barrier_t1 = Arc::clone(&wait_barrier);
            let wait_barrier_t2 = Arc::clone(&wait_barrier);

            let barrier = Arc::new(Barrier::new(2));
            let barrier_t1 = Arc::clone(&barrier);
            let barrier_t2 = Arc::clone(&barrier);

            let t1_r = Arc::clone(&t1_run);
            let t2_r = Arc::clone(&t2_run);

            // condition variable
            let cond_var = TaskRuntime::spawn_empty_blocked(&[]);

            let t_1 = TaskRuntime::spawn_task(
                move || {
                    println!("Task 1 started");
                    t1_r.store(true, Ordering::Release);

                    if wait_barrier_t1.load(Ordering::Acquire) {
                        barrier_t1.wait();
                    }
                },
                &[cond_var.get_handle()],
            );

            let t_2 = TaskRuntime::spawn_task(
                move || {
                    println!("Task 2 started");
                    t2_r.store(true, Ordering::Release);

                    if wait_barrier_t2.load(Ordering::Acquire) {
                        barrier_t2.wait();
                    }
                },
                &[cond_var.get_handle()],
            );

            // start one.
            unsafe { TaskRuntime::notify_finished_one(cond_var.get_handle()) };

            // start one.
            barrier.wait();
            wait_barrier.store(false, Ordering::Release);

            let t1_res = t1_run.load(Ordering::Acquire);
            let t2_res = t2_run.load(Ordering::Acquire);

            // only one has completed.
            assert!((t1_res || t2_res) && (t1_res != t2_res));

            // start other.
            unsafe { cond_var.notify_finished_one() };

            // ensure they are finished.
            t_1.join().unwrap();
            t_2.join().unwrap();

            let t1_res = t1_run.load(Ordering::Acquire);
            let t2_res = t2_run.load(Ordering::Acquire);
            assert_eq!(t1_res, t2_res);
        });

        drop(runtime);
        println!("Runtime exited!")
    }

    #[test]
    fn notify_all() {
        let cpus = num_cpus::get();
        let tasks = 1024;
        let allocated_tasks = 128;

        let runtime = TaskRuntime::new(cpus, tasks, allocated_tasks);
        runtime.enter_runtime(move || {
            // condition variable
            let cond_var = TaskRuntime::spawn_empty_blocked(&[]);

            let tasks: Vec<_> = (0..100)
                .map(|i| {
                    TaskRuntime::spawn_task(
                        move || {
                            println!("Hello task {}", i);
                        },
                        &[cond_var.get_handle()],
                    )
                })
                .collect();

            unsafe { cond_var.broadcast_finished() };

            for task in tasks {
                task.join().unwrap();
            }
        });

        drop(runtime);
    }

    #[test]
    fn spawn_all_workers() {
        let cpus = num_cpus::get();
        let tasks = 1024;
        let allocated_tasks = 128;

        let runtime = TaskRuntime::new(cpus, tasks, allocated_tasks);
        runtime.enter_runtime(move || {
            let counter = Arc::new(AtomicUsize::new(0));
            let counter_clone = Arc::clone(&counter);

            TaskRuntime::spawn_all(
                move || {
                    counter_clone.fetch_add(1, Ordering::AcqRel);
                },
                &[],
            );

            assert_eq!(counter.load(Ordering::Acquire), cpus);
        });

        drop(runtime);
    }
}
