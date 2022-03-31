use fimo_ffi::DynObj;
use fimo_module::Error;
use fimo_tasks::Builder;
use fimo_tasks_int::runtime::{
    get_runtime, init_runtime, is_worker, IRuntime, IRuntimeExt, IScheduler,
};
use fimo_tasks_int::task::ParallelBuilder;
use std::sync::Once;

static INIT: Once = Once::new();

fn new_runtime(f: impl FnOnce(&DynObj<dyn IRuntime>) -> Result<(), Error>) -> Result<(), Error> {
    INIT.call_once(pretty_env_logger::init);

    let runtime = Builder::new().build()?;
    let runtime = fimo_ffi::ptr::coerce_obj(&*runtime);
    f(runtime)
}

pub fn enter_and_init_runtime(f: impl FnOnce() -> Result<(), Error> + Send) -> Result<(), Error> {
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
