use fimo_module::Error;
use fimo_tasks::Builder;
use fimo_tasks_int::runtime::{init_runtime, is_worker, IRuntime};
use std::sync::Once;

static INIT: Once = Once::new();

fn new_runtime(f: impl FnOnce(&IRuntime) -> Result<(), Error>) -> Result<(), Error> {
    INIT.call_once(pretty_env_logger::init);

    let runtime = Builder::new().build()?;
    f(&*runtime)
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
