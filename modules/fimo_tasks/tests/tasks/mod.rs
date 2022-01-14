use crate::runtime::enter_runtime_call;
use fimo_tasks_int::rust as ft;
use ft::{Task, TaskCompletionStatus};

#[test]
fn spawn_task() {
    enter_runtime_call(|| {
        let task = Task::new(|| 10, &[]);
        assert_eq!(task.join().unwrap(), 10)
    })
}

#[test]
fn abort() {
    enter_runtime_call(|| {
        let runtime = ft::get_runtime();
        let signal = runtime.spawn_empty_blocked(&[]);
        let mut task = Task::new(|| (), &[signal.get_handle()]);

        assert!(matches!(task.poll(), TaskCompletionStatus::Pending));
        unsafe { task.as_mut().abort() };
        drop(signal);

        let status = task.wait_on();
        match status {
            TaskCompletionStatus::Aborted => {}
            _ => unreachable!(),
        }

        assert!(matches!(task.join(), Err(None)))
    })
}

#[test]
fn wait_on() {
    enter_runtime_call(|| {
        let runtime = ft::get_runtime();
        let signal = runtime.spawn_empty_blocked(&[]);
        let task = Task::new(|| 99, &[signal.get_handle()]);

        assert!(matches!(task.poll(), TaskCompletionStatus::Pending));
        drop(signal);

        let status = task.wait_on();
        match status {
            TaskCompletionStatus::Completed(val) => {
                assert_eq!(*val, 99)
            }
            _ => unreachable!(),
        }
    })
}

#[test]
fn mutable_polling() {
    enter_runtime_call(|| {
        let mut task = Task::new(|| 99, &[]);

        let status = task.as_mut().wait_on_mut();
        match status {
            TaskCompletionStatus::Completed(val) => {
                assert_eq!(*val, 99);
                *val = 101;
            }
            _ => unreachable!(),
        }

        match task.as_mut().poll_mut() {
            TaskCompletionStatus::Completed(val) => {
                assert_eq!(*val, 101);
                *val = 15;
            }
            _ => unreachable!(),
        }

        assert!(matches!(task.join(), Ok(15)))
    })
}

#[test]
fn dropping() {
    enter_runtime_call(|| {
        use std::sync::Arc;
        let task = Task::new(|| Arc::new(0), &[]);

        let ptr = match task.wait_on() {
            TaskCompletionStatus::Completed(ptr) => Arc::clone(ptr),
            _ => unreachable!(),
        };

        assert_eq!(Arc::strong_count(&ptr), 2);
        drop(task);
        assert_eq!(Arc::strong_count(&ptr), 1);
    })
}
