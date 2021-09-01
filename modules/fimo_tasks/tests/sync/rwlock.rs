use crate::runtime::enter_runtime_call;
use fimo_tasks_interface::rust::sync::RwLock;
use fimo_tasks_interface::rust::Task;
use std::mem::forget;
use std::sync::Arc;

#[test]
fn smoke() {
    enter_runtime_call(|| {
        let l = RwLock::new(());
        drop(l.read());
        drop(l.write());
        drop((l.read(), l.read()));
        drop(l.write());
    })
}

#[test]
fn try_read() {
    enter_runtime_call(|| {
        let l = RwLock::new(());
        println!("{:?}", *l.try_read().unwrap())
    })
}

#[test]
fn try_write() {
    enter_runtime_call(|| {
        let l = RwLock::new(());
        *l.try_write().unwrap() = ();
    })
}

#[test]
fn multiple_tasks() {
    enter_runtime_call(|| {
        const T: u32 = 1000;
        const N: u32 = 3;
        let l = Arc::new(RwLock::new(0));

        let tasks: Vec<_> = (0..T)
            .map(|_| {
                let m_clone = Arc::clone(&l);
                Task::new(
                    move || {
                        for _ in 0..N {
                            *m_clone.write() += 1;
                        }
                    },
                    &[],
                )
            })
            .collect();

        drop(tasks);
        assert_eq!(*l.write(), T * N);
    })
}

#[test]
fn force_unlock_read() {
    enter_runtime_call(|| {
        let l = Arc::new(RwLock::new(()));
        let g = l.read();
        forget(g);

        assert!(matches!(l.try_write(), None));
        unsafe { l.force_unlock_read() };
        assert!(matches!(l.try_write(), Some(_)));
    })
}

#[test]
fn force_unlock_write() {
    enter_runtime_call(|| {
        let l = Arc::new(RwLock::new(()));
        let g = l.write();
        forget(g);

        assert!(matches!(l.try_write(), None));
        unsafe { l.force_unlock_write() };
        assert!(matches!(l.try_write(), Some(_)));
    })
}

#[test]
fn data_ptr() {
    enter_runtime_call(|| {
        let l = Arc::new(RwLock::new(0));
        unsafe { *l.data_ptr() = 5 };
        assert_eq!(*l.read(), 5);
    })
}
