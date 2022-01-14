use crate::runtime::enter_runtime_call;
use fimo_tasks_int::rust::sync::{Condvar, Mutex};
use fimo_tasks_int::rust::{get_runtime, Task};
use std::sync::Arc;

#[test]
fn smoke() {
    enter_runtime_call(|| {
        let c = Condvar::new();
        c.notify_one();
        c.notify_all();
    })
}

#[test]
fn notify_one() {
    enter_runtime_call(|| {
        let m = Arc::new(Mutex::new(()));
        let m2 = m.clone();
        let c = Arc::new(Condvar::new());
        let c2 = c.clone();

        let g = m.lock();
        let t = Task::new(
            move || {
                let _g = m2.lock();
                c2.notify_one();
            },
            &[],
        );
        let g = c.wait(g);
        drop(g);

        t.join().unwrap();
    })
}

#[test]
fn notify_all() {
    enter_runtime_call(|| {
        const N: usize = 10;

        let m = Arc::new(Mutex::new(0));
        let c = Arc::new(Condvar::new());

        let tasks: Vec<_> = (0..N)
            .map(|_| {
                let m2 = m.clone();
                let c2 = c.clone();
                Task::new(
                    move || {
                        let mut g = m2.lock();
                        *g += 1;
                        let mut g = c2.wait(g);
                        *g -= 1;
                    },
                    &[],
                )
            })
            .collect();

        loop {
            let g = m.lock();
            if *g != N {
                drop(g);
                get_runtime().yield_now();
            } else {
                break;
            }
        }

        c.notify_all();
        drop(tasks);
        assert_eq!(*m.lock(), 0);
    })
}

#[test]
#[should_panic]
fn two_mutexes() {
    enter_runtime_call(|| {
        let m = Arc::new(Mutex::new(()));
        let m2 = m.clone();
        let c = Arc::new(Condvar::new());
        let c2 = c.clone();

        let mut g = m.lock();
        let t = Task::new(
            move || {
                let _g = m2.lock();

                let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    let o = Mutex::new(());
                    let mut g = o.lock();
                    g = c2.wait(g);
                    drop(g);
                }));
                c2.notify_one();

                if let Err(err) = r {
                    std::panic::resume_unwind(err);
                }
            },
            &[],
        );
        g = c.wait(g);
        drop(g);

        if let Err(Some(err)) = t.join() {
            std::panic::resume_unwind(err)
        }
    })
}
