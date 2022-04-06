use crate::enter_and_init_runtime;
use fimo_module::Error;
use fimo_tasks_int::{sync::Barrier, task::ParallelBuilder};
use std::sync::Arc;

#[test]
fn test_barrier() -> Result<(), Error> {
    enter_and_init_runtime(|| {
        const N: usize = 10;

        let barrier = Arc::new(Barrier::new(N));
        let b = barrier.clone();

        let tasks = ParallelBuilder::new()
            .num_tasks(Some(N - 1))
            .spawn(move || b.wait().is_leader(), &[])?;

        // all tasks should be waiting for the barrier.
        let mut leader_found = barrier.wait().is_leader();

        // one task should be the leader.
        for task in tasks {
            if let Some(leader) = task.wait() {
                if *leader {
                    assert!(!leader_found);
                    leader_found = true
                }
            }
        }

        assert!(leader_found);

        Ok(())
    })
}
