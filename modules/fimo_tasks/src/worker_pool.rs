use crate::spin_wait::SpinWait;
use crate::task_manager::{Msg, MsgData, PanicData, RawTask};
use crate::{Runtime, TaskScheduler};
use context::stack::ProtectedFixedSizeStack;
use context::{Context, Transfer};
use crossbeam_deque::{Injector, Stealer, Worker};
use fimo_ffi::object::{CoerceObject, ObjectWrapper};
use fimo_module::{Error, ErrorKind};
use fimo_tasks_int::raw::{TaskScheduleStatus, WorkerId};
use fimo_tasks_int::runtime::{init_runtime, IRuntime};
use log::{debug, error, info, trace};
use parking_lot::{Condvar, Mutex, MutexGuard};
use std::cell::Cell;
use std::collections::BTreeMap;
use std::mem::MaybeUninit;
use std::ptr::addr_of_mut;
use std::sync::atomic::AtomicBool;
use std::sync::mpsc::{channel, Sender};
use std::sync::{Arc, Weak};
use std::thread::JoinHandle;

#[derive(Debug)]
pub(crate) struct WorkerPool {
    runtime: Weak<Runtime>,
    worker_ids: Vec<WorkerId>,
    tasks_available: Arc<Condvar>,
    global_queue: Arc<Injector<&'static RawTask>>,
    workers: BTreeMap<WorkerId, (Arc<TaskWorker>, Stealer<&'static RawTask>)>,
}

impl WorkerPool {
    pub fn new() -> Self {
        Self {
            runtime: Weak::new(),
            worker_ids: vec![],
            tasks_available: Arc::new(Condvar::new()),
            workers: Default::default(),
            global_queue: Arc::new(Injector::new()),
        }
    }

    pub fn start_workers(
        &mut self,
        runtime: Weak<Runtime>,
        msg_sender: Sender<Msg>,
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
                self.tasks_available.clone(),
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
    pub fn schedule_task(&mut self, task: &'static RawTask) {
        let context = task.scheduler_context();

        let id = context.worker().unwrap_or(self.worker_ids[0]);
        let (worker, _) = self
            .workers
            .get(&id)
            .unwrap_or_else(|| self.workers.values().next().unwrap());

        unsafe { context.set_schedule_status(TaskScheduleStatus::Scheduled) };
        worker.owned_queue.push(task);
        self.tasks_available.notify_all();
    }
}

#[derive(Debug)]
pub(crate) struct TaskWorker {
    id: WorkerId,
    thread: JoinHandle<()>,
    runtime: Weak<Runtime>,
    should_run: AtomicBool,
    owned_queue: Injector<&'static RawTask>,
    global_queue: Arc<Injector<&'static RawTask>>,
    task_stealer: Mutex<Vec<Stealer<&'static RawTask>>>,
}

pub(crate) struct WorkerInner {
    sender: Sender<Msg>,
    worker: Arc<TaskWorker>,
    tasks_available: Arc<Condvar>,
    local_queue: Worker<&'static RawTask>,
    current_task: Cell<Option<&'static RawTask>>,
}

impl WorkerInner {
    #[inline]
    pub fn shared_data(&self) -> &TaskWorker {
        &self.worker
    }

    #[inline]
    pub fn current_task(&self) -> Option<&'static RawTask> {
        self.current_task.get()
    }

    #[inline]
    pub fn wait_on_tasks(&self, lock: &mut MutexGuard<'_, TaskScheduler>) {
        self.tasks_available.wait(lock)
    }
}

#[thread_local]
pub(crate) static WORKER: Cell<Option<&'static WorkerInner>> = Cell::new(None);

impl TaskWorker {
    pub fn new(
        id: WorkerId,
        sender: Sender<Msg>,
        runtime: Weak<Runtime>,
        tasks_available: Arc<Condvar>,
        global_queue: Arc<Injector<&'static RawTask>>,
    ) -> Result<(Arc<Self>, Stealer<&'static RawTask>), Error> {
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
                        tasks_available,
                        local_queue,
                        current_task: Cell::new(None),
                    });

                    let runtime = inner.worker.runtime.as_ptr();
                    let runtime = Runtime::coerce_obj_raw(runtime);
                    let runtime = IRuntime::from_object_raw(runtime);

                    unsafe { init_runtime(runtime) };
                    WORKER.set(Some(Box::leak(inner)));

                    let stack = ProtectedFixedSizeStack::default();
                    unsafe {
                        Context::new(&*stack, worker_main).resume(0);
                    }

                    // remove and deallocate published worker.
                    unsafe {
                        Box::from_raw(
                            WORKER.take().unwrap() as *const WorkerInner as *mut WorkerInner
                        )
                    };

                    info!("Shutting down worker {id}");
                })
                .map_err(|e| Error::new(ErrorKind::Unknown, e))?
        };

        let mut worker: Arc<MaybeUninit<Self>> = Arc::new(MaybeUninit::uninit());

        // get addresses of all fields manually.
        let uninit_worker = Arc::get_mut(&mut worker).unwrap().as_mut_ptr();
        let id_ptr = unsafe { addr_of_mut!((*uninit_worker).id) };
        let runtime_ptr = unsafe { addr_of_mut!((*uninit_worker).runtime) };
        let thread_ptr = unsafe { addr_of_mut!((*uninit_worker).thread) };
        let should_run_ptr = unsafe { addr_of_mut!((*uninit_worker).should_run) };
        let owned_queue_ptr = unsafe { addr_of_mut!((*uninit_worker).owned_queue) };
        let global_queue_ptr = unsafe { addr_of_mut!((*uninit_worker).global_queue) };
        let task_stealer_ptr = unsafe { addr_of_mut!((*uninit_worker).task_stealer) };

        // initialize fields.
        unsafe {
            id_ptr.write(id);
            runtime_ptr.write(runtime);
            thread_ptr.write(thread);
            should_run_ptr.write(AtomicBool::new(false));
            owned_queue_ptr.write(Injector::new());
            global_queue_ptr.write(global_queue);
            task_stealer_ptr.write(Mutex::new(Vec::new()));
        }

        // safety: we have initialized all fields.
        let worker = unsafe { Arc::from_raw(Arc::into_raw(worker) as *const Self) };

        // receive stealer from worker.
        let stealer = rec.recv().map_err(|e| Error::new(ErrorKind::Unknown, e));
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
}

impl Drop for TaskWorker {
    fn drop(&mut self) {
        self.should_run
            .store(false, std::sync::atomic::Ordering::Release);
    }
}

pub(crate) extern "C" fn worker_main(thread_context: Transfer) -> ! {
    let worker = WORKER.get().unwrap();

    let mut sleep = false;
    let mut spin_wait = SpinWait::new();

    while worker
        .worker
        .should_run
        .load(std::sync::atomic::Ordering::Acquire)
    {
        // first try the owned queue.
        let task = worker.worker.owned_queue.steal().success().or_else(|| {
            // then try the local queue.
            worker.local_queue.pop().or_else(|| {
                std::iter::repeat_with(|| {
                    // if both are empty steal from global queue.
                    worker
                        .worker
                        .global_queue
                        .steal_batch_and_pop(&worker.local_queue)
                        .or_else(|| {
                            // otherwise steal from other tasks.
                            let s = worker.worker.task_stealer.lock();
                            s.iter()
                                .map(|s| s.steal_batch_and_pop(&worker.local_queue))
                                .collect()
                        })
                })
                .find(|s| !s.is_retry())
                .and_then(|s| s.success())
            })
        });

        if task.is_none() {
            if let Some(runtime) = worker.worker.runtime.upgrade() {
                // schedule available tasks or wait.
                match runtime.schedule_tasks(&mut spin_wait, sleep) {
                    None => {}
                    Some(false) => {
                        sleep = true;
                    }
                    Some(true) => {
                        spin_wait.reset();
                        sleep = false;
                    }
                }

                runtime.schedule_tasks(&mut spin_wait, false);
            } else {
                // the runtime is being dropped, wait.
                std::thread::yield_now();
            }

            // retry.
            continue;
        }

        // publish the current task
        let task = task.unwrap();
        worker.current_task.set(Some(task));

        // safety: the task is managed by us.
        let scheduler_data = unsafe { task.scheduler_context_mut() };

        // register task with current worker.
        unsafe { scheduler_data.set_worker(Some(worker.worker.id)) };

        let msg_data =
            if let Some(context) = unsafe { scheduler_data.scheduler_data_mut().context.take() } {
                // jump into task.
                let tr = unsafe { context.resume(0) };

                // remove task
                worker.current_task.set(None);

                // write back task context.
                let context = unsafe { scheduler_data.scheduler_data_mut() };
                context.context = Some(tr.context);

                // read msg data.
                unsafe { (tr.data as *const MsgData).read() }
            } else {
                // remove task
                worker.current_task.set(None);

                MsgData::Completed { aborted: false }
            };

        // send back message.
        let msg = Msg {
            task,
            data: msg_data,
        };
        worker.sender.send(msg).unwrap()
    }

    // resume main function.
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
pub(crate) unsafe fn yield_to_worker(msg_data: MsgData) {
    // take the context of the current task.
    let worker = WORKER.get().unwrap_unchecked();
    let task = worker.current_task.get().unwrap_unchecked();
    let worker_context = task
        .scheduler_context_mut()
        .scheduler_data_mut()
        .context
        .take()
        .unwrap_unchecked();

    // pass the pointer to the data back to the worker function.
    // the worker will read the data, so we must make sure that it won't be dropped by us.
    let msg_data = MaybeUninit::new(msg_data);
    let Transfer { context, .. } = worker_context.resume(msg_data.as_ptr() as usize);

    // at this point we may be on a different thread than we started
    // with and we must ensure that the access to the thread_local WORKER isn't
    // elided with the old one.
    let worker = std::ptr::read_volatile(&WORKER).get().unwrap_unchecked();
    let task = worker.current_task.get().unwrap_unchecked();
    let _ = task
        .scheduler_context_mut()
        .scheduler_data_mut()
        .context
        .insert(context);
}

pub(crate) extern "C" fn task_main(thread_context: Transfer) -> ! {
    loop {
        // fetch the worker and write back the thread context.
        let worker = unsafe { WORKER.get().unwrap_unchecked() };
        let task = unsafe { worker.current_task.get().unwrap_unchecked() };
        let context = unsafe { task.scheduler_context_mut() };
        let _ = unsafe {
            context
                .scheduler_data_mut()
                .context
                .insert(thread_context.context)
        };

        if let Some(f) = unsafe { context.take_entry_function() } {
            let f = unsafe { f.assume_valid() };
            let f = std::panic::AssertUnwindSafe(f);
            if let Err(e) = std::panic::catch_unwind(f) {
                let e = PanicData::new(e);
                unsafe { context.set_panic(Some(e)) };
                debug_assert!(context.is_panicking());
                unsafe { yield_to_worker(MsgData::Completed { aborted: true }) }
            }
        }

        unsafe { yield_to_worker(MsgData::Completed { aborted: false }) }

        unreachable!()
    }
}
