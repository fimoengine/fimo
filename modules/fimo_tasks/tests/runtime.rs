use fimo_ffi::cell::AtomicRefCell;
use fimo_ffi::DynObj;
use fimo_module::Error;
use fimo_tasks::Builder;
use fimo_tasks_int::raw::{IRawTask, ISchedulerContext, TaskScheduleStatus, WakeupData};
use fimo_tasks_int::runtime::{
    current_runtime, get_runtime, init_runtime, is_worker, IRuntime, IRuntimeExt, IScheduler,
    WaitStatus,
};
use fimo_tasks_int::task::ParallelBuilder;
use std::sync::Once;
use std::time::{Duration, SystemTime};

static INIT: Once = Once::new();

mod sync;

fn new_runtime<R>(f: impl FnOnce(&DynObj<dyn IRuntime>) -> Result<R, Error>) -> Result<R, Error> {
    INIT.call_once(pretty_env_logger::init);

    let runtime = Builder::new().build()?;
    let runtime = fimo_ffi::ptr::coerce_obj(&*runtime);
    f(runtime)
}

pub fn enter_and_init_runtime<R: Send>(
    f: impl FnOnce() -> Result<R, Error> + Send,
) -> Result<R, Error> {
    new_runtime(move |runtime| {
        assert!(!is_worker());
        assert!(runtime.worker_id().is_none());

        let res = runtime.block_on_and_enter(
            move |runtime| {
                unsafe { init_runtime(runtime) };
                f()
            },
            &[],
        )?;

        assert!(!is_worker());
        assert!(runtime.worker_id().is_none());

        res
    })
}

#[test]
fn eq() {
    let res = enter_and_init_runtime(|| Ok(5));
    assert_eq!(res.unwrap(), 5)
}

#[test]
#[should_panic]
fn error() {
    #[allow(unreachable_code)]
    let _ = enter_and_init_runtime(|| {
        panic!("Hey");
        Ok(())
    });
}

#[test]
fn enter_scheduler() {
    new_runtime(|r| {
        r.enter_scheduler(|_s, c| {
            assert!(c.is_none());
        });

        r.block_on_and_enter(
            |r| {
                unsafe { init_runtime(r) };

                r.enter_scheduler(|_, c| assert!(c.is_some()))
            },
            &[],
        )
        .unwrap();

        Ok(())
    })
    .unwrap()
}

#[test]
fn worker_id() -> Result<(), Error> {
    enter_and_init_runtime(|| {
        assert!(is_worker());

        let r = unsafe { get_runtime() };
        assert!(r.worker_id().is_some());

        Ok(())
    })
}

#[test]
fn unique_worker_ids() -> Result<(), Error> {
    enter_and_init_runtime(|| {
        let r = unsafe { get_runtime() };
        r.enter_scheduler(|s, _| {
            let worker_ids = s.worker_ids();
            assert!(worker_ids
                .iter()
                .all(|id| worker_ids.iter().filter(|x| **x == *id).count() == 1));
            Ok(())
        })
    })
}

#[test]
fn block_on_multiple_unique() -> Result<(), Error> {
    enter_and_init_runtime(|| {
        let r = unsafe { get_runtime() };
        let ids = ParallelBuilder::new()
            .with_name("Parallel block_on".into())
            .unique_workers(true)
            .block_on(
                || {
                    unsafe { init_runtime(r) };
                    r.worker_id().unwrap()
                },
                &[],
            )
            .unwrap();

        assert!(r.enter_scheduler(|s, _| s.worker_ids().iter().all(|id| ids.contains(id))));
        Ok(())
    })
}

#[test]
fn block_on() -> Result<(), Error> {
    enter_and_init_runtime(|| {
        let r = unsafe { get_runtime() };

        let n = AtomicRefCell::new(0);
        let t = r.spawn(
            || {
                let mut n = n.borrow_mut();
                *n = 5;
            },
            &[],
        )?;

        r.block_on(
            || {
                let mut n = n.borrow_mut();
                assert_eq!(*n, 5);

                *n = 10;
            },
            &[t.handle()],
        )?;

        let n = n.borrow();
        assert_eq!(*n, 10);

        Ok(())
    })
}

#[test]
fn spawn() -> Result<(), Error> {
    enter_and_init_runtime(|| {
        let n = AtomicRefCell::new(0);
        let r = unsafe { get_runtime() };

        const NUM_TASKS: usize = 100;

        let mut tasks = vec![r.spawn(|| {}, &[])?];
        for _ in 0..NUM_TASKS {
            let handle = tasks.last().unwrap().handle();
            let t = r.spawn(
                || {
                    let mut n = n.borrow_mut();
                    *n += 1;
                },
                &[handle],
            )?;
            tasks.push(t);
        }

        drop(tasks);
        let n = n.borrow();
        assert_eq!(*n, NUM_TASKS);

        Ok(())
    })
}

#[test]
fn sleep_for() -> Result<(), Error> {
    enter_and_init_runtime(|| {
        let before_sleep = SystemTime::now();
        let duration = Duration::from_millis(100);
        let r = unsafe { get_runtime() };

        r.yield_for(duration);
        let after_sleep = SystemTime::now();
        let sleep_duration = after_sleep.duration_since(before_sleep).unwrap();
        assert!(duration <= sleep_duration);

        Ok(())
    })
}

#[test]
fn sleep_until() -> Result<(), Error> {
    enter_and_init_runtime(|| {
        let before_sleep = SystemTime::now();
        let duration = Duration::from_millis(100);
        let sleep_until = before_sleep + duration;
        let r = unsafe { get_runtime() };

        r.yield_until(sleep_until);
        let after_sleep = SystemTime::now();
        let sleep_duration = after_sleep.duration_since(before_sleep).unwrap();
        assert!(duration <= sleep_duration);

        Ok(())
    })
}

#[test]
fn block() -> Result<(), Error> {
    enter_and_init_runtime(|| {
        let r = current_runtime().unwrap();

        let (sx, rx) = std::sync::mpsc::channel();

        let mut t = r.spawn(
            || {
                let r = current_runtime().unwrap();
                // reimplementation of `block_now` to also send a message.
                r.yield_and_enter(move |_, curr| {
                    sx.send(()).expect("unable to send signal");
                    curr.context().borrow().request_block();
                })
            },
            &[],
        )?;

        // wait until the task blocks
        rx.recv().unwrap();

        // synchronize the current task with the scheduler to make shure
        // that the task has been blocked.
        r.yield_now();

        let status = t.as_raw().context_atomic().schedule_status();
        assert_eq!(status, TaskScheduleStatus::Blocked);

        t.unblock()?;

        let res = t.join();
        assert!(matches!(res, Ok(_)));

        Ok(())
    })
}

#[test]
fn abort() -> Result<(), Error> {
    enter_and_init_runtime(|| {
        let r = current_runtime().unwrap();

        let t = r.spawn(
            || {
                let r = current_runtime().unwrap();
                unsafe { r.abort_now() }
            },
            &[],
        )?;

        let res = t.wait();
        assert_eq!(res, None);

        let status = t.as_raw().context_atomic().schedule_status();
        assert_eq!(status, TaskScheduleStatus::Aborted);

        let res = t.join();
        assert!(res.is_err());

        Ok(())
    })
}

#[test]
fn wait() -> Result<(), Error> {
    enter_and_init_runtime(|| {
        let r = current_runtime().unwrap();

        let t = r.spawn(|| {}, &[])?;
        let t_handle = t.handle();

        assert!(matches!(
            r.wait_on(t_handle),
            Ok(WaitStatus::Completed(WakeupData::None))
        ));
        let status = t.as_raw().context_atomic().schedule_status();
        assert_eq!(status, TaskScheduleStatus::Finished);
        let _ = t.join();

        // task already finished.
        assert!(matches!(r.wait_on(t_handle), Ok(WaitStatus::Skipped)));

        // waiting on itself is simply skipped.
        let self_handle =
            r.yield_and_enter(|_, cur| unsafe { cur.context().borrow().handle().assume_init() });
        assert!(matches!(r.wait_on(self_handle), Ok(WaitStatus::Skipped)));

        Ok(())
    })
}
