use std::sync::Arc;

use fimo_module::Error;
use fimo_tasks_int::{sync::Mutex, task::ParallelBuilder};

use crate::enter_and_init_runtime;

#[test]
fn new() -> Result<(), Error> {
    enter_and_init_runtime(|| {
        let _m = Mutex::new(());

        Ok(())
    })
}

#[test]
fn inner() -> Result<(), Error> {
    enter_and_init_runtime(|| {
        let m = Mutex::new(5);
        assert_eq!(m.into_inner(), 5);

        Ok(())
    })
}

#[test]
fn get_mut() -> Result<(), Error> {
    enter_and_init_runtime(|| {
        let mut m = Mutex::new(5);
        *m.get_mut() = 10;

        let l = m.lock();
        assert_eq!(*l, 10);

        Ok(())
    })
}

#[test]
fn lock() -> Result<(), Error> {
    enter_and_init_runtime(|| {
        const NUM_TASKS: usize = 100;
        let m: Arc<Mutex<usize>> = Arc::new(Mutex::new(0));
        let m2 = m.clone();

        let tasks = ParallelBuilder::new().num_tasks(Some(NUM_TASKS)).spawn(
            move || {
                let mut l = m2.lock();
                *l += 1;
            },
            &[],
        )?;

        for t in tasks {
            let _ = t.join();
        }

        let l = m.lock();
        assert_eq!(*l, NUM_TASKS);

        Ok(())
    })
}

#[test]
fn try_lock() -> Result<(), Error> {
    enter_and_init_runtime(|| {
        let m = Mutex::new(5);
        let l = m.try_lock();
        assert!(l.is_some());

        let l2 = m.try_lock();
        assert!(l2.is_none());

        drop(l);
        let l = m.try_lock();
        assert!(l.is_some());

        Ok(())
    })
}
