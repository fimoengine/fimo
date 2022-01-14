use crate::initialize;
use fimo_tasks_int::rust as ft;

pub fn enter_runtime_call(f: impl FnOnce() + Send) {
    let tasks_interface = initialize().unwrap();
    tasks_interface.as_task_runtime().enter_runtime(|| {
        ft::initialize_local_bindings(tasks_interface.as_task_runtime());
        f()
    });
}

#[test]
fn enter_runtime() {
    let tasks_interface = initialize().unwrap();
    tasks_interface
        .as_task_runtime()
        .enter_runtime(move || println!("Hello tasks!"));
}

#[test]
fn initialize_bindings() {
    let tasks_interface = initialize().unwrap();
    tasks_interface.as_task_runtime().enter_runtime(|| {
        assert!(!ft::is_worker());
        ft::initialize_local_bindings(tasks_interface.as_task_runtime());
        assert!(ft::is_worker())
    });
}

#[test]
#[should_panic]
fn panic() {
    enter_runtime_call(|| panic!("my panic msg."))
}

#[test]
fn spawn_tasks() {
    enter_runtime_call(|| {
        use std::sync::atomic::{AtomicUsize, Ordering};
        use std::sync::Arc;

        const NUM_TASKS: usize = 100;
        let counter = Arc::new(AtomicUsize::new(0));
        let runtime = ft::get_runtime();

        let tasks: Vec<_> = (0..NUM_TASKS)
            .map(|_| {
                let counter_their = Arc::clone(&counter);
                runtime.spawn_task(
                    move || {
                        counter_their.fetch_add(1, Ordering::AcqRel);
                    },
                    &[],
                )
            })
            .collect();

        let task_handles: Vec<_> = tasks.iter().map(|t| t.get_handle()).collect();
        let finished = runtime.spawn_empty(task_handles);
        finished.join().unwrap();

        assert_eq!(counter.load(Ordering::Acquire), NUM_TASKS)
    })
}
