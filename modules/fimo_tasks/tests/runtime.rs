use fimo_ffi::object::{CoerceObject, ObjectWrapper};
use fimo_module::Error;
use fimo_tasks::Runtime;
use fimo_tasks_int::runtime::{init_runtime, is_worker, IRuntime};
use std::sync::Once;

static INIT: Once = Once::new();

fn new_runtime(f: impl FnOnce(&IRuntime) -> Result<(), Error>) -> Result<(), Error> {
    INIT.call_once(pretty_env_logger::init);

    let stack_size = 1024 * 1024 * 4; // 4 MiB
    let allocated_tasks = 1024;
    let preferred_num_tasks = 1024;
    let workers = None;

    let runtime = Runtime::new(stack_size, allocated_tasks, preferred_num_tasks, workers)?;
    f(IRuntime::from_object(runtime.coerce_obj()))
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
fn worker_id() {
    new_runtime(|r| {
        assert!(!is_worker());
        assert!(r.worker_id().is_none());

        r.block_on_and_enter(
            |r| {
                unsafe { init_runtime(r) };
                assert!(is_worker());
                assert!(r.worker_id().is_some());
            },
            &[],
        )
        .unwrap();

        assert!(!is_worker());
        assert!(r.worker_id().is_none());

        Ok(())
    })
    .unwrap()
}
