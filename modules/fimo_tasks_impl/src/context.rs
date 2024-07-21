#[derive(Debug)]
pub struct ContextImpl {}

impl ContextImpl {
    pub(crate) const fn ffi_context() -> fimo_tasks::Context {
        const VTABLE: &fimo_tasks::bindings::FiTasksVTable = &fimo_tasks::bindings::FiTasksVTable {
            v0: fimo_tasks::bindings::FiTasksVTableV0 {
                is_worker: None,
                task_id: None,
                worker_id: None,
                worker_group: None,
                worker_group_by_id: None,
                query_worker_groups: None,
                release_worker_group_query: None,
                create_worker_group: None,
                yield_: None,
                abort: None,
                sleep: None,
                tss_set: None,
                tss_get: None,
                tss_clear: None,
                park_conditionally: None,
                unpark_one: None,
                unpark_all: None,
                unpark_requeue: None,
                unpark_filter: None,
            },
        };

        let context = fimo_tasks::bindings::FiTasksContext {
            data: std::ptr::null_mut(),
            vtable: VTABLE,
        };

        // Safety:
        unsafe { std::mem::transmute(context) }
    }
}
