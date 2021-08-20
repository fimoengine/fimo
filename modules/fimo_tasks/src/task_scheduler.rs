use crate::raw_task::{RawTaskInner, RawTaskInnerContext, RawTaskInnerRef, RunFlag};
use crate::task_worker::TaskWorker;
use crate::TaskRuntimeInner;
use atomic::Atomic;
use context::stack::{ProtectedFixedSizeStack, Stack};
use context::Context;
use fimo_tasks_interface::rust::{NotifyFn, RawTask, TaskHandle, TaskStatus, WaitOnFn, WorkerId};
use parking_lot::Condvar;
use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::fmt::{Debug, Formatter};
use std::mem::ManuallyDrop;
use std::ops::RangeFrom;
use std::pin::Pin;
use std::sync::atomic::Ordering;
use std::sync::mpsc::Receiver;
use std::sync::Arc;
use std::time::Instant;

#[derive(Debug)]
pub(crate) struct TaskScheduler {
    stack_size: usize,
    stack_allocations: usize,
    desired_allocations: usize,
    free_stacks: VecDeque<TaskSlot>,
    runnable_tasks: VecDeque<TaskHandle>,
    tasks: BTreeMap<TaskHandle, TaskInfo>,
    task_handle_iterator: RangeFrom<usize>,
    free_task_handles: VecDeque<TaskHandle>,
    msg_receiver: Receiver<TaskSchedulerMsg>,
    task_stacks: Vec<RefCell<Option<ProtectedFixedSizeStack>>>,
}

struct TaskInfo {
    raw: Pin<RawTaskInnerRef>,
    scheduled: Atomic<bool>,
    worker_id: Option<WorkerId>,
    waiters: BTreeSet<TaskHandle>,
    dependencies: BTreeSet<TaskHandle>,
    condition_var: Option<Arc<Condvar>>,
    strong_handle: Option<Pin<Arc<RawTaskInner>>>,
    function: Option<Box<dyn FnOnce() + Send>>,
}

#[derive(Debug)]
pub(crate) struct TaskSchedulerMsg {
    pub task: TaskHandle,
    pub msg_type: TaskSchedulerMsgType,
}

#[derive(Debug, Copy, Clone)]
pub(crate) enum TaskSchedulerMsgType {
    Yield {
        until: Option<Instant>,
    },
    Wait {
        dependency: TaskHandle,
        predicate: Option<WaitOnFn>,
    },
    Block,
    Remove,
}

#[derive(Debug, Copy, Clone, Hash, Ord, PartialOrd, PartialEq, Eq)]
pub(crate) struct TaskSlot(usize);

#[derive(Debug, Copy, Clone, Hash, Ord, PartialOrd, PartialEq, Eq)]
pub(crate) enum NewTaskStatus {
    Runnable,
    Blocked,
}

impl TaskScheduler {
    pub fn new(
        max_tasks: usize,
        allocated_tasks: usize,
        receiver: Receiver<TaskSchedulerMsg>,
    ) -> Self {
        let mut scheduler = Self {
            stack_size: 2 * 1024 * 1024, // 2 mb
            stack_allocations: allocated_tasks,
            desired_allocations: allocated_tasks,
            free_stacks: VecDeque::with_capacity(max_tasks),
            runnable_tasks: Default::default(),
            tasks: Default::default(),
            task_handle_iterator: 1..,
            free_task_handles: Default::default(),
            msg_receiver: receiver,
            task_stacks: Vec::with_capacity(max_tasks),
        };

        for i in 0..max_tasks {
            scheduler.free_stacks.push_back(TaskSlot(i));
            if i < allocated_tasks {
                scheduler.task_stacks.push(RefCell::new(Some(
                    ProtectedFixedSizeStack::new(scheduler.stack_size).unwrap(),
                )));
            } else {
                scheduler.task_stacks.push(RefCell::new(None));
            }
        }

        scheduler
    }

    pub unsafe fn reserve_stack(&mut self) -> (TaskSlot, &Stack) {
        // get a new slot ...
        let slot = self
            .free_stacks
            .pop_front()
            .expect("Maximum number of concurrent tasks reached.");

        // and allocate the stack if it was deallocated.
        let mut stack = self.task_stacks[slot.0].borrow_mut();
        if stack.is_none() {
            self.stack_allocations += 1;
            *stack = Some(ProtectedFixedSizeStack::new(self.stack_size).unwrap());
        }

        // extend lifetime of the stack.
        (
            slot,
            &*(stack.as_ref().unwrap() as *const ProtectedFixedSizeStack),
        )
    }

    unsafe fn free_stack(&mut self, slot: TaskSlot) {
        self.free_stacks.push_front(slot);

        // deallocate the stack if there are too many.
        if self.stack_allocations > self.desired_allocations {
            self.stack_allocations -= 1;
            *self.task_stacks[slot.0].borrow_mut() = None;
        }
    }

    pub fn create_handle(&mut self) -> TaskHandle {
        if let Some(handle) = self.free_task_handles.pop_front() {
            handle
        } else {
            TaskHandle {
                id: self.task_handle_iterator.next().unwrap(),
                generation: 0,
            }
        }
    }

    pub fn erase_handle(&mut self, handle: TaskHandle) {
        self.tasks.remove(&handle).unwrap();

        self.free_task_handles.push_back(TaskHandle {
            id: handle.id,
            generation: handle.generation + 1,
        })
    }

    pub fn spawn_task(
        &mut self,
        function: Option<Box<dyn FnOnce() + Send>>,
        dependencies: &[TaskHandle],
        blocked: NewTaskStatus,
        worker_id: Option<WorkerId>,
    ) -> RawTask {
        let handle = self.create_handle();
        let inner = RawTaskInner::pinned_arc(handle);
        let strong_handle = inner.clone();

        unsafe {
            self.initialize_task(
                inner.as_ref(),
                function,
                dependencies,
                blocked,
                worker_id,
                None,
                Some(strong_handle),
            )
        };

        unsafe { RawTask::from_raw(inner) }
    }

    #[allow(clippy::too_many_arguments)]
    pub unsafe fn initialize_task(
        &mut self,
        task: Pin<&RawTaskInner>,
        function: Option<Box<dyn FnOnce() + Send>>,
        dependencies: &[TaskHandle],
        blocked: NewTaskStatus,
        worker_id: Option<WorkerId>,
        cond_var: Option<Arc<Condvar>>,
        strong_handle: Option<Pin<Arc<RawTaskInner>>>,
    ) {
        task.set_run_flag(RunFlag::Run);

        debug_assert!(!self.tasks.contains_key(&task.handle));

        let mut task_info = TaskInfo {
            raw: task.as_raw_ref(),
            scheduled: Atomic::new(false),
            worker_id,
            waiters: Default::default(),
            dependencies: Default::default(),
            function,
            condition_var: cond_var,
            strong_handle,
        };

        for dependency in dependencies {
            if let Some(dependency) = self.tasks.get_mut(dependency) {
                let dependency_task = dependency.raw.as_ref();
                if !matches!(
                    dependency_task.poll_status(),
                    TaskStatus::Finished | TaskStatus::Aborted
                ) {
                    // dependency has not finished yet.
                    // insert dependency.
                    dependency.waiters.insert(task.handle);
                    task_info.dependencies.insert(dependency_task.handle);

                    // set to waiting.
                    task.set_status(TaskStatus::Waiting);
                }
            }
        }

        match blocked {
            NewTaskStatus::Runnable => {
                // enqueue if it is runnable.
                if task_info.dependencies.is_empty() {
                    // set the task to runnable and enqueue.
                    task.set_status(TaskStatus::Runnable);
                    self.runnable_tasks.push_back(task.handle);
                }
            }
            NewTaskStatus::Blocked => {
                // set the task to blocked and return.
                task.set_status(TaskStatus::Blocked);
            }
        }

        // insert task.
        self.tasks.insert(task.handle, task_info);
    }

    fn process_msgs(&mut self) {
        let msgs: Vec<_> = self.msg_receiver.try_iter().collect();
        for TaskSchedulerMsg { task, msg_type } in msgs {
            let task_info = self
                .tasks
                .get_mut(&task)
                .unwrap_or_else(|| panic!("could not find task {:?}", task));
            let task = task_info.raw;

            debug_assert!(task_info.is_scheduled());
            debug_assert!(task_info.dependencies.is_empty());
            debug_assert!(matches!(
                task.as_ref().poll_status(),
                TaskStatus::Runnable | TaskStatus::Finished | TaskStatus::Aborted
            ));

            unsafe { task_info.set_scheduled(false) };

            match msg_type {
                TaskSchedulerMsgType::Yield { until } => {
                    // task must be runnable.
                    debug_assert!(matches!(task.as_ref().poll_status(), TaskStatus::Runnable));

                    if let Some(wait_time) = until {
                        if task.wait_until.get() < wait_time {
                            task.wait_until.set(wait_time);
                        }
                    }

                    self.runnable_tasks.push_back(task.handle);
                }
                TaskSchedulerMsgType::Wait {
                    dependency,
                    predicate,
                } => {
                    // wait on the dependency.
                    self.wait_on(task.handle, dependency, predicate)
                }
                TaskSchedulerMsgType::Block => {
                    // task must be runnable.
                    debug_assert!(matches!(task.as_ref().poll_status(), TaskStatus::Runnable));

                    unsafe { task.as_ref().set_status(TaskStatus::Blocked) };
                }
                TaskSchedulerMsgType::Remove => {
                    // task must have completed.
                    debug_assert!(matches!(
                        task.as_ref().poll_status(),
                        TaskStatus::Finished | TaskStatus::Aborted
                    ));

                    // take ownership of the condvar and handle so that the memory remains initialized.
                    let condvar = task_info.condition_var.take();
                    let strong_handle = task_info.strong_handle.take();

                    // notify all waiters.
                    unsafe { self.broadcast_finished(task.handle, None) };

                    // remove the task.
                    self.erase_handle(task.handle);

                    // cleanup context.
                    let context = task.task_context.borrow_mut().take().unwrap();
                    if let Some(task_slot) = context.task_slot {
                        unsafe { self.free_stack(task_slot) };
                    }
                    drop(context);

                    // we can free the memory as we have already removed the task from the runtime.
                    drop(strong_handle);

                    // notify condvar.
                    if let Some(cond) = condvar {
                        cond.notify_all();
                    }
                }
            }
        }
    }

    pub fn schedule_tasks(&mut self, runtime: Pin<&TaskRuntimeInner>) {
        debug_assert!(runtime.task_scheduler.is_locked());

        // process new messages before scheduling.
        self.process_msgs();

        let current_instant = Instant::now();
        let count = self.runnable_tasks.len();
        for _ in 0..count {
            let task = self.runnable_tasks.pop_front().unwrap();
            // elide lifetime.
            let task_info = unsafe { &mut *(self.tasks.get_mut(&task).unwrap() as *mut TaskInfo) };
            let task = task_info.raw.as_ref();

            debug_assert!(!task_info.is_scheduled());
            debug_assert!(task_info.dependencies.is_empty());
            debug_assert!(task.poll_status() == TaskStatus::Runnable);

            // the yield time must have passed.
            if current_instant < task.wait_until.get() {
                self.runnable_tasks.push_back(task.handle);
                continue;
            }

            let mut context = task.task_context.borrow_mut();
            // create a new context if it does not exist.
            if context.is_none() {
                let mut task_context = RawTaskInnerContext {
                    task_slot: None,
                    context: None,
                    worker_id: task_info.worker_id,
                    function: task_info.function.take().map(ManuallyDrop::new),
                };

                if task_context.function.is_some() {
                    let (task_slot, stack) = unsafe { self.reserve_stack() };
                    task_context.task_slot = Some(task_slot);
                    task_context.context =
                        unsafe { Some(Context::new(stack, TaskWorker::task_main)) };
                }

                *context = Some(task_context);
            }

            let worker = { context.as_ref().unwrap().worker_id };

            // free the refcell.
            drop(context);

            // set the task as scheduled.
            unsafe { task_info.set_scheduled(true) };

            // we won't move the runtime.
            let runtime_ref = unsafe { Pin::into_inner_unchecked(runtime) };

            // insert into a queue.
            match worker {
                None => {
                    // insert into main queue.
                    runtime_ref.global_queue.push(task_info.raw);
                }
                Some(worker) => {
                    // insert into owned queue.
                    runtime_ref.task_workers[worker.0]
                        .owned_queue
                        .push(task_info.raw);
                }
            }
        }
    }

    pub fn wait_on(
        &mut self,
        task: TaskHandle,
        dependency: TaskHandle,
        predicate: Option<WaitOnFn>,
    ) {
        if task == dependency {
            panic!("Trying to wait on itself {:?}", dependency)
        }

        // allow mutable references to multiple values.
        // we won't acquire two references to the same value.
        let task_info = match self.tasks.get_mut(&task) {
            Some(i) => i,
            None => return,
        };
        let task_info = unsafe { &mut *(task_info as *mut TaskInfo) };
        let task = task_info.raw.as_ref();

        // task must be runnable.
        debug_assert!(matches!(task.poll_status(), TaskStatus::Runnable));

        // check if dependency exists.
        if let Some(dependency) = self.tasks.get_mut(&dependency) {
            let dependency_task = dependency.raw.as_ref();
            if !matches!(
                dependency_task.poll_status(),
                TaskStatus::Finished | TaskStatus::Aborted
            ) {
                // validate the wait operation
                let wait = predicate
                    .as_ref()
                    .map_or_else(|| true, |w| (w.validate)(w.data));

                if wait {
                    // dependency has not finished yet.
                    // insert dependency.
                    dependency.waiters.insert(task.handle);
                    task_info.dependencies.insert(dependency_task.handle);

                    // set to waiting.
                    unsafe { task.set_status(TaskStatus::Waiting) };

                    let mut notify_fn =
                        |task: TaskHandle| unsafe { self.notify_finished_one(task, None) };

                    // call the `after_sleep` callback.
                    if let Some(w) = predicate {
                        (w.after_sleep)(&mut notify_fn, w.data)
                    }

                    // don't insert task into runnable queue.
                    return;
                }
            }
        }

        // otherwise just reschedule and call predicate.
        self.runnable_tasks.push_back(task.handle);
    }

    pub unsafe fn broadcast_finished(&mut self, task: TaskHandle, after_wake: Option<NotifyFn>) {
        // allow mutable references to multiple values.
        // we won't acquire two references to the same value.
        let task_info = match self.tasks.get_mut(&task) {
            Some(i) => i,
            None => return,
        };
        let task_info = &mut *(task_info as *mut TaskInfo);

        // task must be blocked or completed.
        debug_assert!(matches!(
            task_info.raw.as_ref().poll_status(),
            TaskStatus::Blocked | TaskStatus::Aborted | TaskStatus::Finished
        ));

        let mut modified = false;

        for waiter in &task_info.waiters {
            let waiter_task_info = self.tasks.get_mut(waiter).unwrap();
            let waiter_task = waiter_task_info.raw.as_ref();

            // waiter task must be blocked or waiting.
            debug_assert!(matches!(
                waiter_task.poll_status(),
                TaskStatus::Blocked | TaskStatus::Waiting
            ));
            debug_assert!(!waiter_task_info.is_scheduled());

            waiter_task_info.dependencies.remove(&task);

            // make task runnable.
            if waiter_task_info.dependencies.is_empty()
                && waiter_task.poll_status() == TaskStatus::Waiting
            {
                waiter_task.set_status(TaskStatus::Runnable);
                self.runnable_tasks.push_back(waiter_task.handle);
            }

            modified = true;
        }

        // clear waiters.
        task_info.waiters.clear();

        // call callback.
        if modified {
            if let Some(n) = after_wake {
                (n.function)(0, n.data)
            }
        }
    }

    pub unsafe fn notify_finished_one(&mut self, task: TaskHandle, after_wake: Option<NotifyFn>) {
        // allow mutable references to multiple values.
        // we won't acquire two references to the same value.
        let task_info = match self.tasks.get_mut(&task) {
            Some(i) => i,
            None => return,
        };
        let task_info = &mut *(task_info as *mut TaskInfo);

        // task must be blocked or completed.
        debug_assert!(matches!(
            task_info.raw.as_ref().poll_status(),
            TaskStatus::Blocked | TaskStatus::Aborted | TaskStatus::Finished
        ));

        // fetch one waiter and make it runnable.
        if let Some(waiter) = task_info.waiters.iter().next().copied() {
            let waiter_task_info = self.tasks.get_mut(&waiter).unwrap();
            let waiter_task = waiter_task_info.raw.as_ref();

            // waiter task must be blocked or waiting.
            debug_assert!(matches!(
                waiter_task.poll_status(),
                TaskStatus::Blocked | TaskStatus::Waiting
            ));
            debug_assert!(!waiter_task_info.is_scheduled());

            waiter_task_info.dependencies.remove(&task);

            // make task runnable.
            if waiter_task_info.dependencies.is_empty()
                && waiter_task.poll_status() == TaskStatus::Waiting
            {
                waiter_task.set_status(TaskStatus::Runnable);
                self.runnable_tasks.push_back(waiter_task.handle);
            }

            // remove from list.
            task_info.waiters.remove(&waiter);

            // call callback.
            if let Some(n) = after_wake {
                (n.function)(task_info.waiters.len(), n.data)
            }
        }
    }

    pub unsafe fn unblock_task(&mut self, task: TaskHandle) {
        let task_info = match self.tasks.get_mut(&task) {
            Some(i) => i,
            None => return,
        };

        // task must be blocked.
        debug_assert!(matches!(
            task_info.raw.as_ref().poll_status(),
            TaskStatus::Blocked
        ));

        if task_info.dependencies.is_empty() {
            // task has no dependencies and can be scheduled.
            task_info.raw.as_ref().set_status(TaskStatus::Runnable);
            self.runnable_tasks.push_back(task);
        } else {
            // task must wait.
            task_info.raw.as_ref().set_status(TaskStatus::Waiting);
        }
    }
}

impl TaskInfo {
    fn is_scheduled(&self) -> bool {
        self.scheduled.load(Ordering::Acquire)
    }

    unsafe fn set_scheduled(&self, scheduled: bool) {
        self.scheduled.store(scheduled, Ordering::Release)
    }
}

impl Debug for TaskInfo {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TaskInfo")
            .field("raw", &self.raw)
            .field("scheduled", &self.scheduled)
            .field("dependencies", &self.dependencies)
            .field("waiters", &self.waiters)
            .finish_non_exhaustive()
    }
}
