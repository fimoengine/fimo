use std::sync::Arc;

use fimo_module::Error;
use fimo_tasks_int::{
    runtime::{current_runtime, IRuntimeExt},
    sync::{Condvar, Mutex},
    task::ParallelBuilder,
};

use crate::enter_and_init_runtime;

#[test]
fn smoke() -> Result<(), Error> {
    enter_and_init_runtime(|| {
        let c = Condvar::new();
        c.notify_one();
        c.notify_all();

        Ok(())
    })
}

#[test]
fn notify_one() -> Result<(), Error> {
    enter_and_init_runtime(|| {
        let m = Arc::new(Mutex::new(()));
        let m2 = m.clone();
        let c = Arc::new(Condvar::new());
        let c2 = c.clone();

        let runtime = current_runtime().unwrap();
        let mut g = m.lock();
        let _t = runtime.spawn(
            move || {
                let _g = m2.lock();
                c2.notify_one();
            },
            &[],
        );
        c.wait(&mut g);

        Ok(())
    })
}

#[test]
fn notify_all() -> Result<(), Error> {
    enter_and_init_runtime(|| {
        const N: usize = 10;

        let data = Arc::new((Mutex::new(0), Condvar::new()));
        let (tx, rx) = std::sync::mpsc::channel();

        let _t = {
            let data = data.clone();
            let tx = tx.clone();
            ParallelBuilder::new().num_tasks(Some(N)).spawn(
                move || {
                    let &(ref lock, ref cond) = &*data;
                    let mut cnt = lock.lock();
                    *cnt += 1;
                    if *cnt == N {
                        tx.send(()).unwrap();
                    }
                    while *cnt != 0 {
                        cond.wait(&mut cnt);
                    }
                    tx.send(()).unwrap();
                },
                &[],
            )?
        };
        drop(tx);

        let &(ref lock, ref cond) = &*data;
        rx.recv().unwrap();
        let mut cnt = lock.lock();
        *cnt = 0;
        cond.notify_all();
        drop(cnt);

        for _ in 0..N {
            rx.recv().unwrap();
        }

        Ok(())
    })
}

#[test]
fn notify_one_return_true() -> Result<(), Error> {
    enter_and_init_runtime(|| {
        let m = Arc::new(Mutex::new(()));
        let m2 = m.clone();
        let c = Arc::new(Condvar::new());
        let c2 = c.clone();

        let mut g = m.lock();
        let runtime = current_runtime().unwrap();
        let _t = runtime.spawn(
            move || {
                let _g = m2.lock();
                assert!(c2.notify_one());
            },
            &[],
        );
        c.wait(&mut g);

        Ok(())
    })
}

#[test]
fn notify_one_return_false() -> Result<(), Error> {
    enter_and_init_runtime(|| {
        let m = Arc::new(Mutex::new(()));
        let c = Arc::new(Condvar::new());

        let runtime = current_runtime().unwrap();
        let _t = runtime.spawn(
            move || {
                let _g = m.lock();
                assert!(!c.notify_one());
            },
            &[],
        );

        Ok(())
    })
}

#[test]
fn notify_all_return() -> Result<(), Error> {
    enter_and_init_runtime(|| {
        const N: usize = 10;

        let data = Arc::new((Mutex::new(0), Condvar::new()));
        let (tx, rx) = std::sync::mpsc::channel();

        let t = {
            let data = data.clone();
            let tx = tx.clone();
            ParallelBuilder::new().num_tasks(Some(N)).spawn(
                move || {
                    let &(ref lock, ref cond) = &*data;
                    let mut cnt = lock.lock();
                    *cnt += 1;
                    if *cnt == N {
                        tx.send(()).unwrap();
                    }
                    while *cnt != 0 {
                        cond.wait(&mut cnt);
                    }
                    tx.send(()).unwrap();
                },
                &[],
            )?
        };
        drop(tx);

        let &(ref lock, ref cond) = &*data;
        rx.recv().unwrap();
        let mut cnt = lock.lock();
        *cnt = 0;
        assert_eq!(cond.notify_all(), N);
        drop(cnt);

        for _ in 0..N {
            rx.recv().unwrap();
        }

        drop(t);

        assert_eq!(cond.notify_all(), 0);

        Ok(())
    })
}

#[test]
#[should_panic]
fn two_mutexes() {
    let _ = enter_and_init_runtime(|| {
        let m = Arc::new(Mutex::new(()));
        let m2 = m.clone();
        let m3 = Arc::new(Mutex::new(()));
        let c = Arc::new(Condvar::new());
        let c2 = c.clone();

        // Make sure we don't leave the child thread dangling
        struct PanicGuard<'a>(&'a Condvar);
        impl<'a> Drop for PanicGuard<'a> {
            fn drop(&mut self) {
                self.0.notify_one();
            }
        }

        let (tx, rx) = std::sync::mpsc::channel();
        let g = m.lock();

        let runtime = current_runtime().unwrap();
        let _t = runtime.spawn(
            move || {
                let mut g = m2.lock();
                tx.send(()).unwrap();
                c2.wait(&mut g);
            },
            &[],
        )?;

        drop(g);
        rx.recv().unwrap();
        let _g = m.lock();
        let _guard = PanicGuard(&*c);
        c.wait(&mut m3.lock());

        Ok(())
    });
}
#[test]
fn two_mutexes_disjoint() {
    let _ = enter_and_init_runtime(|| {
        let m = Arc::new(Mutex::new(()));
        let m2 = m.clone();
        let m3 = Arc::new(Mutex::new(()));
        let m4 = m3.clone();
        let c = Arc::new(Condvar::new());
        let c2 = c.clone();
        let c3 = c.clone();

        let mut g = m.lock();
        let runtime = current_runtime().unwrap();
        let _t = runtime.spawn(
            move || {
                let _g = m2.lock();
                c2.notify_one();
            },
            &[],
        )?;
        c.wait(&mut g);
        drop(g);

        let mut g = m3.lock();
        let _t = runtime.spawn(
            move || {
                let _g = m4.lock();
                c3.notify_one();
            },
            &[],
        )?;
        let _ = c.wait(&mut g);
        drop(g);

        Ok(())
    });
}
