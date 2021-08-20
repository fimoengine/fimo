use crate::raw_task::{RawTaskInner, RawTaskInnerContext, RawTaskInnerRef, RunFlag};
use crate::task_scheduler::{TaskSchedulerMsg, TaskSchedulerMsgType};
use crate::TaskRuntimeInner;
use context::{Context, Transfer};
use crossbeam_deque::{Injector, Stealer, Worker};
use fimo_tasks_interface::rust::{TaskHandle, TaskStatus, WaitOnFn, WorkerId};
use std::cell::Cell;
use std::mem::ManuallyDrop;
use std::pin::Pin;
use std::sync::atomic::Ordering;
use std::sync::mpsc::Sender;
use std::sync::{Arc, Barrier};
use std::thread::JoinHandle;
use std::time::Instant;

thread_local! { pub(crate) static WORKER: Cell<Option<Pin<&'static TaskWorker>>> = Cell::new(None) }

#[derive(Debug)]
pub(crate) struct TaskWorker {
    pub id: WorkerId,
    pub thread: Option<JoinHandle<()>>,
    pub worker_task: ManuallyDrop<Pin<Box<RawTaskInner>>>,
    pub local_queue: Worker<Pin<RawTaskInnerRef>>,
    pub owned_queue: Injector<Pin<RawTaskInnerRef>>,
    pub scheduler_sender: Sender<TaskSchedulerMsg>,
    pub runtime: Pin<&'static TaskRuntimeInner>,
    pub current_task: Cell<Option<Pin<RawTaskInnerRef>>>,
}

unsafe impl Sync for TaskWorker {}

impl TaskWorker {
    pub fn new(
        id: WorkerId,
        thread_sync: Arc<Barrier>,
        sender: Sender<TaskSchedulerMsg>,
        mut runtime: Pin<&mut TaskRuntimeInner>,
    ) -> Self {
        // casting the reference is save, as it outlives the worker.
        let runtime_ptr =
            unsafe { Pin::into_inner_unchecked(runtime.as_ref()) as *const TaskRuntimeInner };
        let runtime_ref = unsafe { Pin::new_unchecked(&*runtime_ptr) };

        let thread = {
            std::thread::Builder::new()
                .name(format!("Worker {}", id.0))
                .spawn(move || {
                    thread_sync.wait();

                    // set worker ref and wait until ready
                    let worker =
                        Pin::new(unsafe { &*(&runtime_ref.task_workers[id.0] as *const _) });
                    WORKER.with(|w| w.set(Some(worker)));

                    while worker.worker_task.as_ref().poll_status() == TaskStatus::Blocked {
                        std::thread::yield_now();
                    }

                    let context = worker
                        .worker_task
                        .task_context
                        .borrow_mut()
                        .as_mut()
                        .unwrap()
                        .context
                        .take()
                        .unwrap();

                    // jump into managed context.
                    unsafe { context.resume(0) };
                })
                .unwrap()
        };

        let runtime_ref_mut = unsafe { Pin::into_inner_unchecked(runtime.as_mut()) };
        let worker_task =
            RawTaskInner::pinned_box(runtime_ref_mut.task_scheduler.get_mut().create_handle());
        // we control the task so it is save to modify.
        unsafe {
            worker_task.as_ref().set_run_flag(RunFlag::Run);
            let (task_slot, stack) = runtime_ref_mut.task_scheduler.get_mut().reserve_stack();

            let mut context = worker_task.task_context.borrow_mut();
            *context = Some(RawTaskInnerContext {
                task_slot: Some(task_slot),
                context: Some(Context::new(stack, Self::worker_main)),
                worker_id: Some(id),
                function: None,
            })
        };

        Self {
            id,
            thread: Some(thread),
            worker_task: ManuallyDrop::new(worker_task),
            local_queue: Worker::new_fifo(),
            owned_queue: Default::default(),
            scheduler_sender: sender,
            runtime: runtime_ref,
            current_task: Cell::new(None),
        }
    }

    pub fn start_worker(&mut self) -> Stealer<Pin<RawTaskInnerRef>> {
        // set the runnable status
        unsafe { self.worker_task.as_ref().set_status(TaskStatus::Runnable) };
        self.local_queue.stealer()
    }

    extern "C" fn worker_main(thread_context: Transfer) -> ! {
        let worker = WORKER.with(|w| w.get().unwrap());

        // the runtime won't be moved.
        let runtime = unsafe { Pin::into_inner_unchecked(worker.runtime) };

        // increment reference count.
        runtime.running_workers.fetch_add(1, Ordering::AcqRel);

        // execute until stop signal is received.
        while worker.worker_task.as_ref().poll_run_flag() == RunFlag::Run {
            // fetch one task.
            let task = worker.owned_queue.steal().success().or_else(|| {
                worker.local_queue.pop().or_else(|| {
                    std::iter::repeat_with(|| {
                        runtime
                            .global_queue
                            .steal_batch_and_pop(&worker.local_queue)
                            .or_else(|| runtime.task_stealer.iter().map(|s| s.steal()).collect())
                    })
                    .find(|s| !s.is_retry())
                    .and_then(|s| s.success())
                })
            });

            if task.is_none() {
                // try scheduling new tasks.
                if let Some(mut guard) = runtime.task_scheduler.try_lock() {
                    // update runtime and schedule tasks.
                    guard.schedule_tasks(worker.runtime);
                } else {
                    // otherwise wait
                    std::thread::yield_now();
                }

                continue;
            }

            // make task visible.
            let task = task.unwrap();
            worker.current_task.set(Some(task));

            // take ownership of the context and bind task to worker.
            let context = {
                let mut context = task.task_context.borrow_mut();
                let raw_context = context.as_mut().unwrap();

                // bind to current worker.
                raw_context.worker_id = Some(worker.id);

                // take ownership of the context.
                raw_context.context.take()
            };

            // execute task.
            let msg_type = match context {
                None => {
                    // mark as finished.
                    unsafe { task.as_ref().set_status(TaskStatus::Finished) };
                    TaskSchedulerMsgType::Remove
                }
                Some(context) => {
                    // jump into task.
                    let Transfer { context, data } = unsafe { context.resume(0) };

                    // write back context.
                    let mut ctx = task.task_context.borrow_mut();
                    let raw_context = ctx.as_mut().unwrap();
                    raw_context.context = Some(context);

                    unsafe { (data as *const TaskSchedulerMsgType).read() }
                }
            };

            // send back message.
            worker
                .scheduler_sender
                .send(TaskSchedulerMsg {
                    task: task.handle,
                    msg_type,
                })
                .unwrap();
        }

        // decrement reference count.
        runtime.running_workers.fetch_sub(1, Ordering::AcqRel);

        // set finished status and return to thread function.
        unsafe {
            worker.worker_task.as_ref().set_status(TaskStatus::Finished);
            thread_context.context.resume(0);
        }
        unreachable!()
    }

    pub extern "C" fn task_main(thread_context: Transfer) -> ! {
        loop {
            // fetch the worker and write back the thread context.
            let worker = WORKER.with(|w| w.get().unwrap());
            worker
                .worker_task
                .as_ref()
                .task_context
                .borrow_mut()
                .as_mut()
                .unwrap()
                .context = Some(thread_context.context);

            // get current task.
            let task = worker.current_task.get().unwrap();

            // return to worker if it should not run.
            if task.as_ref().poll_run_flag() == RunFlag::Stop {
                // abort and cleanup.
                unsafe { Self::abort(true) };
            }

            {
                let mut context = task.task_context.borrow_mut();
                let raw_context = context.as_mut().unwrap();
                if let Some(f) = &mut raw_context.function {
                    let f = unsafe { ManuallyDrop::take(f) };
                    drop(context);

                    if let Err(e) = std::panic::catch_unwind(std::panic::AssertUnwindSafe(f)) {
                        // store panic in task.
                        *task.panic_error.borrow_mut() = Some(e);
                        task.panicking.store(true, Ordering::Release);
                        unsafe { Self::abort(false) };
                    }
                }
            }

            // the task has finished.
            unsafe { Self::finish() };
        }
    }

    unsafe fn resume_worker(msg_type: TaskSchedulerMsgType) {
        let worker = WORKER.with(|w| w.get().unwrap());
        let worker_task = worker.worker_task.as_ref();
        let mut context = worker_task.task_context.borrow_mut();
        let worker_context = context.as_mut().unwrap().context.take().unwrap();
        drop(context);

        // pass pointer to the msg type to the worker.
        let Transfer { context, .. } = worker_context.resume(&msg_type as *const _ as usize);

        // reset context
        let mut ctx = worker_task.task_context.borrow_mut();
        let raw_ctx = ctx.as_mut().unwrap();
        raw_ctx.context = Some(context);
        drop(ctx);
    }

    pub unsafe fn abort(cleanup: bool) -> ! {
        let worker = WORKER.with(|w| w.get().unwrap());
        let task = worker.current_task.get().unwrap();

        if cleanup {
            // drop the contained function.
            let mut context = task.task_context.borrow_mut();
            let raw_context = context.as_mut().unwrap();
            ManuallyDrop::drop(raw_context.function.as_mut().unwrap());
        }

        // change to finished and return.
        task.as_ref().set_status(TaskStatus::Aborted);
        Self::resume_worker(TaskSchedulerMsgType::Remove);
        unreachable!()
    }

    unsafe fn finish() -> ! {
        let worker = WORKER.with(|w| w.get().unwrap());
        let task = worker.current_task.get().unwrap();

        // change to finished and return.
        task.as_ref().set_status(TaskStatus::Finished);
        Self::resume_worker(TaskSchedulerMsgType::Remove);
        unreachable!()
    }

    pub unsafe fn block() {
        Self::resume_worker(TaskSchedulerMsgType::Block)
    }

    pub fn wait_on(task: TaskHandle, predicate: Option<WaitOnFn>) {
        unsafe {
            Self::resume_worker(TaskSchedulerMsgType::Wait {
                dependency: task,
                predicate,
            })
        }
    }

    pub fn yield_now() {
        unsafe { Self::resume_worker(TaskSchedulerMsgType::Yield { until: None }) }
    }

    pub fn yield_until(instant: Instant) {
        unsafe {
            Self::resume_worker(TaskSchedulerMsgType::Yield {
                until: Some(instant),
            })
        }
    }
}

impl Drop for TaskWorker {
    fn drop(&mut self) {
        unsafe { self.worker_task.as_ref().abort() };
    }
}
