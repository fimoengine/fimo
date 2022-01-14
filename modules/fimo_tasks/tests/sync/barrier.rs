use crate::runtime::enter_runtime_call;
use fimo_tasks_int::rust::sync::Barrier;
use fimo_tasks_int::rust::{Task, TaskCompletionStatus};
use std::sync::Arc;

#[test]
fn test_barrier() {
    enter_runtime_call(|| {
        const N: usize = 10;

        let barrier = Arc::new(Barrier::new(N));

        let tasks: Vec<_> = (0..N - 1)
            .map(|_| {
                let b = barrier.clone();
                Task::new(move || b.wait().is_leader(), &[])
            })
            .collect();

        // all tasks should be waiting for the barrier.
        assert!(tasks
            .iter()
            .all(|t| t.poll() == TaskCompletionStatus::Pending));

        let mut leader_found = barrier.wait().is_leader();

        // one task should be the leader.
        for task in tasks {
            if task.join().unwrap() {
                assert!(!leader_found);
                leader_found = true
            }
        }

        assert!(leader_found)
    })
}
