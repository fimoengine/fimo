// Copyright 2016 Amanieu d'Antras
//
// Licensed under the Apache License, Version 2.0, <LICENSE-APACHE or
// http://apache.org/licenses/LICENSE-2.0> or the MIT license <LICENSE-MIT or
// http://opensource.org/licenses/MIT>, at your option. This file may not be
// copied, modified, or distributed except according to those terms.

use std::sync::{
    atomic::{AtomicUsize, Ordering},
    mpsc::channel,
    Arc,
};

use fimo_module::Error;
use fimo_tasks_int::{
    runtime::{current_runtime, IRuntimeExt},
    sync::RwLock,
    task::ParallelBuilder,
};
use rand::Rng;

use crate::enter_and_init_runtime;

#[derive(Eq, PartialEq, Debug)]
struct NonCopy(i32);

#[test]
fn smoke() -> Result<(), Error> {
    enter_and_init_runtime(|| {
        let l = RwLock::new(());
        drop(l.read());
        drop(l.write());
        drop((l.read(), l.read()));
        drop(l.write());

        Ok(())
    })
}

#[test]
fn frob() -> Result<(), Error> {
    enter_and_init_runtime(|| {
        const N: u32 = 10;
        const M: u32 = 1000;

        let r = Arc::new(RwLock::new(()));
        let (tx, rx) = channel::<()>();
        {
            let tx = tx.clone();
            ParallelBuilder::new().num_tasks(Some(N as usize)).spawn(
                move || {
                    let mut rng = rand::thread_rng();
                    for _ in 0..M {
                        if rng.gen_bool(1.0 / N as f64) {
                            drop(r.write());
                        } else {
                            drop(r.read());
                        }
                    }
                    drop(tx);
                },
                &[],
            )?;
        }
        drop(tx);
        let _ = rx.recv();

        Ok(())
    })
}

#[test]
fn test_rw_arc_no_poison_wr() -> Result<(), Error> {
    enter_and_init_runtime(|| {
        let runtime = current_runtime().unwrap();

        let arc = Arc::new(RwLock::new(1));
        let arc2 = arc.clone();
        let _: Result<(), _> = runtime
            .spawn(
                move || {
                    let _lock = arc2.write();
                    panic!();
                },
                &[],
            )?
            .join();
        let lock = arc.read();
        assert_eq!(*lock, 1);

        Ok(())
    })
}

#[test]
fn test_rw_arc_no_poison_ww() -> Result<(), Error> {
    enter_and_init_runtime(|| {
        let runtime = current_runtime().unwrap();

        let arc = Arc::new(RwLock::new(1));
        let arc2 = arc.clone();
        let _: Result<(), _> = runtime
            .spawn(
                move || {
                    let _lock = arc2.write();
                    panic!();
                },
                &[],
            )?
            .join();
        let lock = arc.write();
        assert_eq!(*lock, 1);

        Ok(())
    })
}

#[test]
fn test_rw_arc_no_poison_rr() -> Result<(), Error> {
    enter_and_init_runtime(|| {
        let runtime = current_runtime().unwrap();

        let arc = Arc::new(RwLock::new(1));
        let arc2 = arc.clone();
        let _: Result<(), _> = runtime
            .spawn(
                move || {
                    let _lock = arc2.read();
                    panic!();
                },
                &[],
            )?
            .join();
        let lock = arc.read();
        assert_eq!(*lock, 1);

        Ok(())
    })
}

#[test]
fn test_rw_arc_no_poison_rw() -> Result<(), Error> {
    enter_and_init_runtime(|| {
        let runtime = current_runtime().unwrap();

        let arc = Arc::new(RwLock::new(1));
        let arc2 = arc.clone();
        let _: Result<(), _> = runtime
            .spawn(
                move || {
                    let _lock = arc2.read();
                    panic!();
                },
                &[],
            )?
            .join();
        let lock = arc.write();
        assert_eq!(*lock, 1);

        Ok(())
    })
}

#[test]
fn test_rw_arc() -> Result<(), Error> {
    enter_and_init_runtime(|| {
        let runtime = current_runtime().unwrap();

        let arc = Arc::new(RwLock::new(0));
        let arc2 = arc.clone();
        let (tx, rx) = channel();

        runtime.spawn(
            move || {
                let runtime = current_runtime().unwrap();
                let mut lock = arc2.write();
                for _ in 0..10 {
                    let tmp = *lock;
                    *lock = -1;
                    runtime.yield_now();
                    *lock = tmp + 1;
                }
                tx.send(()).unwrap();
            },
            &[],
        )?;

        let children = {
            let arc3 = arc.clone();
            ParallelBuilder::new().num_tasks(Some(5)).spawn(
                move || {
                    let lock = arc3.read();
                    assert!(*lock >= 0);
                },
                &[],
            )?
        };

        // Wait for children to pass their asserts
        for r in children {
            assert!(r.join().is_ok());
        }

        // Wait for writer to finish
        rx.recv().unwrap();
        let lock = arc.read();
        assert_eq!(*lock, 10);

        Ok(())
    })
}

#[test]
fn test_rw_arc_access_in_unwind() -> Result<(), Error> {
    enter_and_init_runtime(|| {
        let runtime = current_runtime().unwrap();

        let arc = Arc::new(RwLock::new(1));
        let arc2 = arc.clone();
        let _: Result<(), _> = runtime
            .spawn(
                move || {
                    struct Unwinder {
                        i: Arc<RwLock<isize>>,
                    }
                    impl Drop for Unwinder {
                        fn drop(&mut self) {
                            let mut lock = self.i.write();
                            *lock += 1;
                        }
                    }
                    let _u = Unwinder { i: arc2 };
                    panic!();
                },
                &[],
            )?
            .join();
        let lock = arc.read();
        assert_eq!(*lock, 2);

        Ok(())
    })
}

#[test]
fn test_rwlock_unsized() -> Result<(), Error> {
    enter_and_init_runtime(|| {
        let rw: &RwLock<[i32]> = &RwLock::new([1, 2, 3]) as _;
        {
            let b = &mut *rw.write();
            b[0] = 4;
            b[2] = 5;
        }
        let comp: &[i32] = &[4, 2, 5];
        assert_eq!(&*rw.read(), comp);

        Ok(())
    })
}

#[test]
fn test_rwlock_try_read() -> Result<(), Error> {
    enter_and_init_runtime(|| {
        let lock = RwLock::new(0isize);
        {
            let read_guard = lock.read();

            let read_result = lock.try_read();
            assert!(
                read_result.is_some(),
                "try_read should succeed while read_guard is in scope"
            );

            drop(read_guard);
        }
        {
            let write_guard = lock.write();

            let read_result = lock.try_read();
            assert!(
                read_result.is_none(),
                "try_read should fail while write_guard is in scope"
            );

            drop(write_guard);
        }

        Ok(())
    })
}

#[test]
fn test_rwlock_try_write() -> Result<(), Error> {
    enter_and_init_runtime(|| {
        let lock = RwLock::new(0isize);
        {
            let read_guard = lock.read();

            let write_result = lock.try_write();
            assert!(
                write_result.is_none(),
                "try_write should fail while read_guard is in scope"
            );
            assert!(lock.is_locked());
            assert!(!lock.is_locked_exclusive());

            drop(read_guard);
        }
        {
            let write_guard = lock.write();

            let write_result = lock.try_write();
            assert!(
                write_result.is_none(),
                "try_write should fail while write_guard is in scope"
            );
            assert!(lock.is_locked());
            assert!(lock.is_locked_exclusive());

            drop(write_guard);
        }

        Ok(())
    })
}

#[test]
fn test_into_inner() -> Result<(), Error> {
    enter_and_init_runtime(|| {
        let m = RwLock::new(NonCopy(10));
        assert_eq!(m.into_inner(), NonCopy(10));

        Ok(())
    })
}

#[test]
fn test_into_inner_drop() -> Result<(), Error> {
    enter_and_init_runtime(|| {
        struct Foo(Arc<AtomicUsize>);
        impl Drop for Foo {
            fn drop(&mut self) {
                self.0.fetch_add(1, Ordering::SeqCst);
            }
        }
        let num_drops = Arc::new(AtomicUsize::new(0));
        let m = RwLock::new(Foo(num_drops.clone()));
        assert_eq!(num_drops.load(Ordering::SeqCst), 0);
        {
            let _inner = m.into_inner();
            assert_eq!(num_drops.load(Ordering::SeqCst), 0);
        }
        assert_eq!(num_drops.load(Ordering::SeqCst), 1);

        Ok(())
    })
}

#[test]
fn test_get_mut() -> Result<(), Error> {
    enter_and_init_runtime(|| {
        let mut m = RwLock::new(NonCopy(10));
        *m.get_mut() = NonCopy(20);
        assert_eq!(m.into_inner(), NonCopy(20));

        Ok(())
    })
}

#[test]
fn test_rwlockguard_sync() -> Result<(), Error> {
    enter_and_init_runtime(|| {
        fn sync<T: Sync>(_: T) {}

        let rwlock = RwLock::new(());
        sync(rwlock.read());
        sync(rwlock.write());

        Ok(())
    })
}

#[test]
fn test_rwlock_debug() -> Result<(), Error> {
    enter_and_init_runtime(|| {
        let x = RwLock::new(vec![0u8, 10]);

        assert_eq!(format!("{:?}", x), "RwLock { data: [0, 10] }");
        let _lock = x.write();
        assert_eq!(format!("{:?}", x), "RwLock { data: <locked> }");

        Ok(())
    })
}

#[test]
fn test_clone() -> Result<(), Error> {
    enter_and_init_runtime(|| {
        let rwlock = RwLock::new(Arc::new(1));
        let a = rwlock.read();
        let b = a.clone();
        assert_eq!(Arc::strong_count(&b), 2);

        Ok(())
    })
}

#[test]
fn test_rw_write_is_locked() -> Result<(), Error> {
    enter_and_init_runtime(|| {
        let lock = RwLock::new(0isize);
        {
            let _read_guard = lock.read();

            assert!(lock.is_locked());
            assert!(!lock.is_locked_exclusive());
        }

        {
            let _write_guard = lock.write();

            assert!(lock.is_locked());
            assert!(lock.is_locked_exclusive());
        }

        Ok(())
    })
}
