use crate::scheduler::task_manager::{AssertValidTask, Msg, MsgData, PanicData};
use crate::spin_wait::SpinWait;
use crate::{Runtime, TaskScheduler};
use context::stack::ProtectedFixedSizeStack;
use context::{Context, Transfer};
use crossbeam_deque::{Injector, Steal, Stealer, Worker};
use fimo_ffi::error::wrap_error;
use fimo_ffi::DynObj;
use fimo_module::{Error, ErrorKind};
use fimo_tasks_int::raw::{IRawTask, TaskScheduleStatus, WorkerId};
use fimo_tasks_int::runtime::init_runtime;
use log::{debug, error, info, trace, warn};
use parking_lot::{Condvar, Mutex, MutexGuard};
use std::cell::Cell;
use std::collections::BTreeMap;
use std::mem::{ManuallyDrop, MaybeUninit};
use std::ptr::addr_of_mut;
use std::sync::atomic::{AtomicBool, AtomicUsize};
use std::sync::mpsc::{channel, Sender};
use std::sync::{Arc, Weak};
use std::thread::JoinHandle;
use std::time::SystemTime;

#[derive(Debug)]
pub(crate) struct WorkerPool {
    runtime: Weak<Runtime>,
    worker_ids: Vec<WorkerId>,
    running_workers: Arc<AtomicUsize>,
    global_queue: Arc<Injector<AssertValidTask>>,
    workers: BTreeMap<WorkerId, (Arc<TaskWorker>, Stealer<AssertValidTask>)>,
}

impl WorkerPool {
    pub fn new() -> Self {
        Self {
            runtime: Weak::new(),
            worker_ids: vec![],
            workers: Default::default(),
            running_workers: Arc::new(AtomicUsize::new(0)),
            global_queue: Arc::new(Injector::new()),
        }
    }

    pub fn start_workers(
        &mut self,
        runtime: Weak<Runtime>,
        msg_sender: Sender<Msg<'static>>,
        workers: Option<usize>,
    ) -> Result<(), Error> {
        trace!("Starting worker threads");

        // Use provided number or fetch the number of available cpu cores.
        // Set the number to at least 1.
        let workers = workers.unwrap_or_else(num_cpus::get);
        let workers = workers.max(1);

        debug!("Number of worker threads {}", workers);

        // the runtime must be valid.
        debug_assert!(runtime.strong_count() > 0);

        trace!("Spawning workers");
        self.runtime = runtime.clone();
        for id in 0..workers {
            let id = WorkerId::new(id).map_or(
                Err(Error::new(
                    ErrorKind::ResourceExhausted,
                    "Too many workers spawned",
                )),
                Ok,
            )?;
            let (worker, stealer) = TaskWorker::new(
                id,
                msg_sender.clone(),
                runtime.clone(),
                self.running_workers.clone(),
                self.global_queue.clone(),
            )?;
            self.worker_ids.push(id);
            self.workers.insert(id, (worker, stealer));
        }

        trace!("Initializing workers");
        // initialize the task stealer of each task.
        for id in &self.worker_ids {
            let (worker, _) = self.workers.get(id).unwrap();
            let mut stealer = worker.task_stealer.lock();

            // push the stealer of each worker except it's own onto the vector.
            for (worker, s) in self.workers.values() {
                if worker.id != *id {
                    stealer.push(s.clone());
                }
            }
        }

        Ok(())
    }

    #[inline]
    pub fn workers(&self) -> &[WorkerId] {
        &self.worker_ids
    }

    /// Assigns a task to a worker queue.
    #[inline]
    pub fn assign_task_to_worker(&mut self, task: AssertValidTask) {
        let (handle, worker) = {
            let context = task.context().borrow();
            (context.handle(), context.worker())
        };

        if let Some(id) = worker {
            // If the task was assigned to a worker we push it to its private
            // queue and ensure that only the worker can run it.

            trace!("Assigning task {handle} to worker {id}");
            let (worker, _) = self.workers.get(&id).unwrap_or_else(|| {
                warn!("Worker id {id} not found, assigning {handle} to random worker");
                self.workers.values().next().unwrap()
            });
            worker.owned_queue.push(task);

            trace!("Notifying worker {}", worker.id);
            worker.tasks_available.notify_all();
        } else {
            // If the task is unassigned we push it to the global queue and dynamically
            // assign it to the first free worker.

            trace!("Assigning task {handle} to the global queue");
            self.global_queue.push(task);
            self.wake_all_workers();
        }
    }

    /// Notifies all workers that there are scheduled tasks.
    #[inline]
    pub fn wake_all_workers(&mut self) {
        trace!("Waking all workers");
        for (worker, _) in self.workers.values() {
            worker.tasks_available.notify_all();
        }
    }
}

#[derive(Debug)]
pub(crate) struct TaskWorker {
    id: WorkerId,
    runtime: Weak<Runtime>,
    should_run: AtomicBool,
    tasks_available: Condvar,
    running_workers: Arc<AtomicUsize>,
    thread: ManuallyDrop<JoinHandle<()>>,
    owned_queue: Injector<AssertValidTask>,
    global_queue: Arc<Injector<AssertValidTask>>,
    task_stealer: Mutex<Vec<Stealer<AssertValidTask>>>,
}

pub(crate) struct WorkerInner {
    sender: Sender<Msg<'static>>,
    worker: Arc<TaskWorker>,
    local_queue: Worker<AssertValidTask>,
    current_task: Cell<Option<CopyTask>>,
}

#[derive(Clone, Copy)]
struct CopyTask(&'static DynObj<dyn IRawTask + 'static>);

impl CopyTask {
    #[inline]
    fn from_task(t: AssertValidTask) -> Self {
        // SAFETY: The types have the same layout.
        unsafe { std::mem::transmute(t) }
    }

    #[inline]
    fn into_task(self) -> AssertValidTask {
        // SAFETY: The task remained valid.
        unsafe { AssertValidTask::from_raw(self.0) }
    }
}

impl WorkerInner {
    #[inline]
    pub fn shared_data(&self) -> &TaskWorker {
        &self.worker
    }

    #[inline]
    pub fn current_task(&self) -> Option<AssertValidTask> {
        self.current_task.get().map(|t| t.into_task())
    }

    /// Puts a worker to sleep if it can't run any tasks.
    #[inline]
    pub fn wait_on_tasks(
        &self,
        mut lock: MutexGuard<'_, TaskScheduler>,
        until: Option<SystemTime>,
    ) {
        // wait only if there are no more tasks available.
        if self.local_queue.is_empty()
            && self.shared_data().owned_queue.is_empty()
            && self.shared_data().global_queue.is_empty()
            && self.shared_data().should_run()
        {
            if self.shared_data().mark_waiting() {
                info!(
                    "No more tasks. Worker {} going to deep sleep",
                    self.worker.id
                );
                self.worker.tasks_available.wait(&mut lock);
                info!("Woke up worker {} from sleep", self.worker.id);
            } else {
                info!(
                    "Putting last worker {} to sleep until a task becomes runnable",
                    self.worker.id
                );

                if let Some(until) = until {
                    let now = SystemTime::now();
                    let until = std::cmp::max(now, until);
                    let duration = until.duration_since(now).expect("time went backwards");
                    self.worker.tasks_available.wait_for(&mut lock, duration);
                } else {
                    self.worker.tasks_available.wait(&mut lock);
                }
            }

            self.shared_data().mark_running();
        }
    }
}

impl Drop for WorkerPool {
    fn drop(&mut self) {
        info!("Shutting down worker pool");

        // signal the shutdown
        for (worker, _) in self.workers.values() {
            worker.signal_shutdown();
        }

        // wake the workers.
        self.wake_all_workers();

        // wait for the tasks to finish
        for (worker, _) in self.workers.values_mut() {
            // SAFETY: We are the WorkerPool
            unsafe { worker.join() };
        }
    }
}

#[thread_local]
pub(crate) static WORKER: Cell<Option<&'static WorkerInner>> = Cell::new(None);

impl TaskWorker {
    pub fn new(
        id: WorkerId,
        sender: Sender<Msg<'static>>,
        runtime: Weak<Runtime>,
        running_workers: Arc<AtomicUsize>,
        global_queue: Arc<Injector<AssertValidTask>>,
    ) -> Result<(Arc<Self>, Stealer<AssertValidTask>), Error> {
        let (sen, rec) = channel();
        let (work_sen, work_rec) = channel();

        let thread = {
            std::thread::Builder::new()
                .name(format!("Worker {}", id))
                .spawn(move || {
                    info!("Spawned new worker {id}");

                    let local_queue = Worker::new_fifo();
                    sen.send(local_queue.stealer()).unwrap();

                    let worker = match work_rec.recv() {
                        Ok(w) => w,
                        Err(e) => {
                            error!("Error: {e}");
                            return;
                        }
                    };

                    // allocate and publish the inner data.
                    let inner = Box::new(WorkerInner {
                        sender,
                        worker,
                        local_queue,
                        current_task: Cell::new(None),
                    });

                    let runtime = inner.worker.runtime.as_ptr();
                    let runtime = fimo_ffi::ptr::coerce_obj_raw(runtime);

                    // SAFETY: The thread was just created so it can't have been called.
                    unsafe { init_runtime(runtime) };

                    inner.shared_data().mark_running();
                    WORKER.set(Some(Box::leak(inner)));

                    let stack = ProtectedFixedSizeStack::default();

                    // SAFETY: The stack will outlive the context.
                    unsafe {
                        Context::new(&*stack, worker_main).resume(0);
                    }

                    // remove and deallocate published worker.
                    // SAFETY: The data was allocated in a box and now that the worker is cleaning up
                    // no one has access to the data.
                    let worker = unsafe {
                        Box::from_raw(WORKER.take().unwrap_unchecked() as *const WorkerInner
                            as *mut WorkerInner)
                    };

                    info!("Shutting down worker {id}");
                    worker.shared_data().mark_waiting();
                })
                .map_err(|e| Error::new(ErrorKind::Unknown, wrap_error(e)))?
        };

        let mut worker: Arc<MaybeUninit<Self>> = Arc::new(MaybeUninit::uninit());

        // get addresses of all fields manually.
        let uninit_worker = Arc::get_mut(&mut worker).unwrap().as_mut_ptr();

        // SAFETY: We aren't dereferencing the pointer, just initializing the data manually.
        unsafe {
            let id_ptr = addr_of_mut!((*uninit_worker).id);
            let runtime_ptr = addr_of_mut!((*uninit_worker).runtime);
            let thread_ptr = addr_of_mut!((*uninit_worker).thread);
            let should_run_ptr = addr_of_mut!((*uninit_worker).should_run);
            let running_workers_ptr = addr_of_mut!((*uninit_worker).running_workers);
            let tasks_available_ptr = addr_of_mut!((*uninit_worker).tasks_available);
            let owned_queue_ptr = addr_of_mut!((*uninit_worker).owned_queue);
            let global_queue_ptr = addr_of_mut!((*uninit_worker).global_queue);
            let task_stealer_ptr = addr_of_mut!((*uninit_worker).task_stealer);

            // initialize fields.
            id_ptr.write(id);
            runtime_ptr.write(runtime);
            thread_ptr.write(ManuallyDrop::new(thread));
            should_run_ptr.write(AtomicBool::new(false));
            running_workers_ptr.write(running_workers);
            tasks_available_ptr.write(Condvar::new());
            owned_queue_ptr.write(Injector::new());
            global_queue_ptr.write(global_queue);
            task_stealer_ptr.write(Mutex::new(Vec::new()));
        }

        // safety: we have initialized all fields.
        let worker = unsafe { Arc::from_raw(Arc::into_raw(worker) as *const Self) };

        // receive stealer from worker.
        let stealer = rec
            .recv()
            .map_err(|e| Error::new(ErrorKind::Unknown, wrap_error(e)));
        worker
            .should_run
            .store(stealer.is_ok(), std::sync::atomic::Ordering::Release);

        // send worker data to worker.
        work_sen.send(worker.clone()).unwrap();

        let stealer = stealer?;
        Ok((worker, stealer))
    }

    #[inline]
    pub fn id(&self) -> WorkerId {
        self.id
    }

    /// # Safety
    ///
    /// Called only from the worker pool.
    #[inline]
    unsafe fn join(&self) {
        let mut thread = std::ptr::read(&self.thread);
        ManuallyDrop::drop(&mut thread)
    }

    #[inline]
    pub fn signal_shutdown(&self) {
        trace!("Notifying worker {} for shutdown", self.id);
        self.should_run
            .store(false, std::sync::atomic::Ordering::Release);
    }

    #[inline]
    pub fn should_run(&self) -> bool {
        self.should_run.load(std::sync::atomic::Ordering::Acquire)
    }

    #[inline]
    pub fn mark_running(&self) {
        self.running_workers
            .fetch_add(1, std::sync::atomic::Ordering::AcqRel);
    }

    #[inline]
    pub fn mark_waiting(&self) -> bool {
        // if we are not the last running worker we should be put to sleep
        self.running_workers
            .fetch_sub(1, std::sync::atomic::Ordering::AcqRel)
            != 1
    }
}

pub(crate) extern "C" fn worker_main(thread_context: Transfer) -> ! {
    let worker = WORKER.get().unwrap();

    let mut spin_wait = SpinWait::new();

    while worker.shared_data().should_run() {
        // steal some tasks from the global queue.
        let _ = worker.worker.global_queue.steal_batch(&worker.local_queue);

        // first try the owned queue.
        let task = worker.worker.owned_queue.steal().success().or_else(|| {
            // then try the local queue.
            worker.local_queue.pop().or_else(|| {
                // then try stealing from other tasks.
                std::iter::repeat_with(|| {
                    let s = worker.worker.task_stealer.lock();
                    let s: Steal<_> = s
                        .iter()
                        .map(|s| s.steal_batch_and_pop(&worker.local_queue))
                        .collect();
                    s
                })
                .find(|s| !s.is_retry())
                .and_then(|s| s.success())
            })
        });

        if task.is_none() {
            trace!(
                "Worker {} found no tasks, trying to enter scheduler",
                worker.worker.id
            );
            if let Some(runtime) = worker.worker.runtime.upgrade() {
                // schedule available tasks or wait.
                match Runtime::schedule_tasks(runtime, &mut spin_wait) {
                    None => {}
                    Some(_) => {
                        spin_wait.reset();
                    }
                }
            } else {
                // the runtime is being dropped, wait.
                std::thread::yield_now();
            }

            // retry.
            continue;
        }

        // publish the current task
        let task = task.unwrap();

        let context = {
            let context = task.context().borrow();
            trace!(
                "Worker {} set to run task {}",
                worker.worker.id,
                context.handle()
            );

            // The `task_main` requires that the current task is privided
            // in `current_task`.
            worker
                .current_task
                .set(Some(CopyTask::from_task(task.clone())));

            // register task with current worker.
            // SAFETY: The operation is valid for the following reasons:
            // 1. We know that the id exists, as that is the id of the current worker.
            // 2. Currently there are only two paths a path can take -- it was registered with
            // the worker set to `None` or `Some(_)`. This is the only instance of a call to
            // `set_worker` in the runtime implementation, and according to the safety doc
            // we can assume that no one will modify it until the task is unregistered.
            // Starting with the case that the worker was `None`, we know that the task has not
            // jet started it's execution. Therefore we are allowed to assign it to any available
            // worker. The other case it that the worker is `Some`. In that case the task has
            // already started and we must make shure that we don't invalidate any data cached
            // by the task. With the current implementation such an invalidation is impossible,
            // as the task will be reassigned to the same worker each time it is scheduled.
            // Therefore we aren't actually modifying the worker but just setting it to the same
            // value.
            unsafe { context.set_worker(Some(worker.worker.id)) };

            let data = context.scheduler_data();
            let mut shared = data.shared_data_mut();
            shared.take_context()
        };

        let msg_data = if let Some(context) = context {
            // jump into task.
            // SAFETY: We assume that the tasks are well behaved.
            let tr = unsafe { context.resume(0) };

            // remove task
            worker.current_task.set(None);

            // write back task context.
            let context = task.context().borrow();
            context
                .scheduler_data()
                .shared_data_mut()
                .set_context(tr.context);

            // read msg data.
            // SAFETY: `yield_to_worker` writes the pointer of the message
            // in the `Transfer` and ensures that it won't drop it.
            unsafe { (tr.data as *const MsgData<'_>).read() }
        } else {
            // remove task
            worker.current_task.set(None);

            MsgData::Completed { aborted: false }
        };

        // Mark the status as processing.
        // SAFETY: While we are outside of the scheduler, this operation
        // is implementation defined and the runtime knows how to handle it.
        unsafe {
            task.context()
                .borrow()
                .set_schedule_status(TaskScheduleStatus::Processing)
        };

        // send back message.
        let msg = Msg {
            task,
            data: msg_data,
        };
        worker.sender.send(msg).unwrap();
    }

    // resume main function.
    // SAFETY: We assume that the tasks are well behaved.
    unsafe { thread_context.context.resume(0) };
    unreachable!()
}

/// Yields the current task to the worker.
///
/// # Safety
///
/// May only be run from a task on a worker thread. Requirements:
///
/// - The thread local `WORKER` is initialized to `Some(_)`
/// - The handle to the current task is saved in `WORKER`
/// - The context of the current task is provided in the handle of the task.
pub(crate) unsafe fn yield_to_worker(msg_data: MsgData<'_>) {
    let Transfer { context, .. } = {
        // take the context of the current task.
        let worker = WORKER.get().unwrap_unchecked();
        let task = worker.current_task.get().unwrap_unchecked().into_task();

        let worker_context = {
            let scheduler_context = task.context().borrow();
            trace!(
                "Yielding task {} to worker {}",
                scheduler_context.handle(),
                worker.worker.id
            );

            let data = scheduler_context.scheduler_data();
            let mut shared = data.shared_data_mut();
            shared.take_context().unwrap_unchecked()
        };

        // pass the pointer to the data back to the worker function.
        // the worker will read the data, so we must make sure that it won't be dropped by us.
        let msg_data = MaybeUninit::new(msg_data);
        worker_context.resume(msg_data.as_ptr() as usize)
    };

    // at this point we may be on a different thread than we started
    // with and we must ensure that the access to the thread_local WORKER isn't
    // elided with the old one.
    let worker = std::ptr::read_volatile(&WORKER).get().unwrap_unchecked();
    let task = worker.current_task.get().unwrap_unchecked().into_task();
    let scheduler_context = task.context().borrow();
    scheduler_context
        .scheduler_data()
        .shared_data_mut()
        .set_context(context);

    trace!(
        "Resumed task {} to worker {}",
        scheduler_context.handle(),
        worker.worker.id
    );
}

pub(crate) extern "C" fn task_main(thread_context: Transfer) -> ! {
    loop {
        // Fetch the worker, write back the thread context and take the entry function.
        // SAFETY: `WORKER` is always initialized during the execution of a worker and
        // the current task was initialized prior to calling this function.
        let task = unsafe {
            let worker = WORKER.get().unwrap_unchecked();
            worker.current_task.get().unwrap_unchecked().into_task()
        };

        let f = {
            let context = task.context().borrow();
            let data = context.scheduler_data();
            let mut shared = data.shared_data_mut();
            shared.set_context(thread_context.context);
            shared.take_entry_func()
        };

        if let Some(f) = f {
            let f = std::panic::AssertUnwindSafe(f);
            if let Err(e) = std::panic::catch_unwind(f) {
                // If there was an error we wrap it up in our custom wrapper and write it
                // into the shared context portion.
                let e = PanicData::new(e);
                {
                    let context = task.context().borrow();
                    let data = context.scheduler_data();
                    let mut shared = data.shared_data_mut();
                    shared.set_panic(e);
                }

                // SAFETY: The preconditions are satisfied by the worker loop.
                unsafe { yield_to_worker(MsgData::Completed { aborted: true }) }
            }
        }

        // SAFETY: The preconditions are satisfied by the worker loop.
        unsafe { yield_to_worker(MsgData::Completed { aborted: false }) }

        unreachable!()
    }
}
