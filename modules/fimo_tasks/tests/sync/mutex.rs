use crate::runtime::enter_runtime_call;
use fimo_tasks_interface::rust::sync::Mutex;
use fimo_tasks_interface::rust::Task;
use std::mem::forget;
use std::sync::Arc;

#[test]
fn smoke() {
    enter_runtime_call(|| {
        let m = Mutex::new(());
        drop(m.lock());
        drop(m.lock());
    })
}

#[test]
fn try_lock() {
    enter_runtime_call(|| {
        let m = Mutex::new(());
        *m.try_lock().unwrap() = ();
    })
}

#[test]
fn multiple_tasks() {
    enter_runtime_call(|| {
        const T: u32 = 1000;
        const N: u32 = 3;
        let m = Arc::new(Mutex::new(0));

        let tasks: Vec<_> = (0..T)
            .map(|_| {
                let m_clone = Arc::clone(&m);
                Task::new(
                    move || {
                        for _ in 0..N {
                            *m_clone.lock() += 1;
                        }
                    },
                    &[],
                )
            })
            .collect();

        drop(tasks);
        assert_eq!(*m.lock(), T * N);
    })
}

#[test]
fn force_unlock() {
    enter_runtime_call(|| {
        let m = Arc::new(Mutex::new(()));
        let g = m.lock();
        forget(g);

        assert!(matches!(m.try_lock(), None));
        unsafe { m.force_unlock() };
        assert!(matches!(m.try_lock(), Some(_)));
    })
}

#[test]
fn data_ptr() {
    enter_runtime_call(|| {
        let m = Arc::new(Mutex::new(0));
        unsafe { *m.data_ptr() = 5 };
        assert_eq!(*m.lock(), 5);
    })
}
