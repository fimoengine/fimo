use std::{ffi::CStr, num::NonZeroUsize, sync::Arc};

use crate::{
    module_export::{TasksModule, TasksModuleToken},
    worker_group::{
        self, worker_thread::with_worker_context_lock, WorkerGroupFFI, WorkerGroupImpl,
    },
    WorkerGroupQuery,
};
use fimo_std::{bindings as std_bindings, error::Error, ffi::FFITransferable, module::Module};
use fimo_tasks::{bindings, TaskId, WorkerGroupId, WorkerId};

#[derive(Debug)]
pub struct ContextImpl;

impl ContextImpl {
    pub fn is_worker_(&self, module: TasksModule<'_>) -> bool {
        let _span = fimo_std::span_trace!(module.context(), "self: {self:?}");
        with_worker_context_lock(|_| {}).is_ok()
    }

    pub fn task_id(&self, module: TasksModule<'_>) -> Result<TaskId, Error> {
        let _span = fimo_std::span_trace!(module.context(), "self: {self:?}");
        with_worker_context_lock(|worker| match &worker.current_task {
            None => {
                fimo_std::emit_error!(module.context(), "no task registered for current worker");
                Err(Error::EUNKNOWN)
            }
            Some(t) => Ok(t.id()),
        })
        .flatten()
    }

    pub fn worker_id(&self, module: TasksModule<'_>) -> Result<WorkerId, Error> {
        let _span = fimo_std::span_trace!(module.context(), "self: {self:?}");
        with_worker_context_lock(|worker| worker.id)
    }

    pub fn worker_group(&self, module: TasksModule<'_>) -> Result<Arc<WorkerGroupImpl>, Error> {
        let _span = fimo_std::span_trace!(module.context(), "self: {self:?}");
        with_worker_context_lock(|worker| worker.group.clone())
    }

    pub fn worker_group_by_id(
        &self,
        module: TasksModule<'_>,
        id: WorkerGroupId,
    ) -> Result<Arc<WorkerGroupImpl>, Error> {
        let _span = fimo_std::span_trace!(module.context(), "self: {self:?}, id: {id:?}");
        let runtime = module.data().shared_runtime();
        match runtime.worker_group_by_id(id) {
            None => {
                fimo_std::emit_error!(module.context(), "no group found");
                Err(Error::EINVAL)
            }
            Some(group) => {
                fimo_std::emit_trace!(module.context(), "found group: {group:?}");
                Ok(group)
            }
        }
    }

    pub fn query_worker_groups(&self, module: TasksModule<'_>) -> Result<WorkerGroupQuery, Error> {
        let _span = fimo_std::span_trace!(module.context(), "self: {self:?}");
        let runtime = module.data().shared_runtime();
        let groups = runtime.query_worker_groups();
        fimo_std::emit_trace!(module.context(), "found groups: {groups:?}");
        Ok(groups)
    }

    pub fn create_worker_group(
        &self,
        module: TasksModule<'_>,
        name: &CStr,
        stacks: &[bindings::FiTasksWorkerGroupConfigStack],
        default_stack_index: usize,
        number_of_workers: Option<NonZeroUsize>,
        is_queryable: bool,
    ) -> Result<Arc<WorkerGroupImpl>, Error> {
        let _span = fimo_std::span_trace!(
            module.context(),
            "self: {self:?}, name: {name:?}, number of workers: {number_of_workers:?}, \
            is queryable: {is_queryable:?}"
        );
        let runtime = module.data().shared_runtime();
        runtime.spawn_worker_group(
            name,
            stacks,
            default_stack_index,
            number_of_workers,
            is_queryable,
        )
    }

    pub fn yield_now(&self, module: TasksModule<'_>) -> Result<(), Error> {
        let _span = fimo_std::span_trace!(module.context(), "self: {self:?}");
        fimo_std::emit_trace!(module.context(), "yielding task");
        worker_group::worker_thread::yield_now()
    }

    /// # Safety
    ///
    /// Aborting a task does not unwind the stack, possibly resulting in broken invariants. This
    /// function should only be called as a last resort.
    pub unsafe fn abort(
        &self,
        module: TasksModule<'_>,
        error: *mut std::ffi::c_void,
    ) -> Result<std::convert::Infallible, Error> {
        let _span = fimo_std::span_trace!(module.context(), "self: {self:?}, error: {error:?}");
        fimo_std::emit_trace!(module.context(), "aborting task");

        // Safety: The caller ensures that it is safe.
        unsafe { worker_group::worker_thread::abort_task(error) }
    }

    pub fn sleep_for(
        &self,
        module: TasksModule<'_>,
        duration: std::time::Duration,
    ) -> Result<(), Error> {
        let _span =
            fimo_std::span_trace!(module.context(), "self: {self:?}, duration: {duration:?}");
        let now = std::time::Instant::now();
        let until = now + duration;
        fimo_std::emit_trace!(module.context(), "sleeping task until {until:?}");
        worker_group::worker_thread::wait_until(until)
    }

    pub fn tss_set(
        &self,
        module: TasksModule<'_>,
        key: bindings::FiTasksTssKey,
        value: *mut std::ffi::c_void,
        dtor: bindings::FiTasksTssDtor,
    ) -> Result<(), Error> {
        let _span = fimo_std::span_trace!(
            module.context(),
            "self: {self:?}, key: {key:?}, value: {value:?}, dtor: {dtor:?}"
        );
        fimo_std::emit_trace!(module.context(), "writing tss");
        with_worker_context_lock(|worker| {
            let task = worker
                .current_task
                .as_mut()
                .expect("no task is being executed by the worker");

            // Safety: The task is owned by the current worker, since it is being executed by it.
            unsafe { task.write_tss_value(key, value, dtor) };
        })
    }

    pub fn tss_get(
        &self,
        module: TasksModule<'_>,
        key: bindings::FiTasksTssKey,
    ) -> Result<*mut std::ffi::c_void, Error> {
        let _span = fimo_std::span_trace!(module.context(), "self: {self:?}, key: {key:?}");
        fimo_std::emit_trace!(module.context(), "reading tss");
        with_worker_context_lock(|worker| {
            let task = worker
                .current_task
                .as_mut()
                .expect("no task is being executed by the worker");

            // Safety: The task is being executed by the current worker, which therefore owns it.
            if let Some(value) = unsafe { task.read_tss_value(key) } {
                fimo_std::emit_trace!(module.context(), "found value {value:?}");
                Ok(value)
            } else {
                fimo_std::emit_error!(module.context(), "tss key not set");
                Err(Error::EINVAL)
            }
        })
        .flatten()
    }

    pub fn tss_clear(
        &self,
        module: TasksModule<'_>,
        key: bindings::FiTasksTssKey,
    ) -> Result<(), Error> {
        let _span = fimo_std::span_trace!(module.context(), "self: {self:?}, key: {key:?}");
        fimo_std::emit_trace!(module.context(), "clearing tss");
        with_worker_context_lock(|worker| {
            let task = worker
                .current_task
                .as_mut()
                .expect("no task is being executed by the worker");

            // Safety: The task is being executed by the current worker, which therefore owns it.
            unsafe { task.clear_tss_value(key) }
        })
        .flatten()
    }
}

impl ContextImpl {
    pub(crate) const fn ffi_context() -> fimo_tasks::Context {
        const VTABLE: &bindings::FiTasksVTable = &bindings::FiTasksVTable {
            v0: bindings::FiTasksVTableV0 {
                is_worker: Some(ContextImpl::is_worker_ffi),
                task_id: Some(ContextImpl::task_id_ffi),
                worker_id: Some(ContextImpl::worker_id_ffi),
                worker_group: Some(ContextImpl::worker_group_ffi),
                worker_group_by_id: Some(ContextImpl::worker_group_by_id_ffi),
                query_worker_groups: Some(ContextImpl::query_worker_groups_ffi),
                release_worker_group_query: Some(ContextImpl::release_worker_group_query_ffi),
                create_worker_group: Some(ContextImpl::create_worker_group_ffi),
                yield_: Some(ContextImpl::yield_ffi),
                abort: Some(ContextImpl::abort_ffi),
                sleep: Some(ContextImpl::sleep_ffi),
                tss_set: Some(ContextImpl::tss_set_ffi),
                tss_get: Some(ContextImpl::tss_get_ffi),
                tss_clear: Some(ContextImpl::tss_clear_ffi),
                park_conditionally: Some(ContextImpl::park_conditionally_ffi),
                unpark_one: Some(ContextImpl::unpark_one_ffi),
                unpark_all: Some(ContextImpl::unpark_all_ffi),
                unpark_requeue: Some(ContextImpl::unpark_requeue_ffi),
                unpark_filter: Some(ContextImpl::unpark_filter_ffi),
            },
        };

        let context = bindings::FiTasksContext {
            data: std::ptr::null_mut(),
            vtable: VTABLE,
        };

        // Safety:
        unsafe { std::mem::transmute(context) }
    }

    unsafe extern "C" fn is_worker_ffi(_this: *mut std::ffi::c_void) -> bool {
        fimo_std::panic::abort_on_panic(|| {
            // Safety: Is safe since we are calling it from an exported symbol.
            unsafe { TasksModuleToken::with_current_unlocked(|module| Self.is_worker_(module)) }
        })
    }

    unsafe extern "C" fn task_id_ffi(
        _this: *mut std::ffi::c_void,
        id: *mut usize,
    ) -> std_bindings::FimoError {
        fimo_std::panic::catch_unwind(|| {
            // Safety: Is safe since we are calling it from an exported symbol.
            unsafe {
                TasksModuleToken::with_current_unlocked(|module| {
                    let _span = fimo_std::span_trace!(module.context(), "id: {id:?}");
                    if id.is_null() {
                        fimo_std::emit_error!(module.context(), "`id` is null");
                        return Err(Error::EINVAL);
                    }
                    id.write(Self.task_id(module)?.0);
                    Ok(())
                })
            }
        })
        .flatten()
        .map_or_else(|e| e.into_error(), |_| Error::EOK.into_error())
    }

    unsafe extern "C" fn worker_id_ffi(
        _this: *mut std::ffi::c_void,
        id: *mut usize,
    ) -> std_bindings::FimoError {
        fimo_std::panic::catch_unwind(|| {
            // Safety: Is safe since we are calling it from an exported symbol.
            unsafe {
                TasksModuleToken::with_current_unlocked(|module| {
                    let _span = fimo_std::span_trace!(module.context(), "id: {id:?}");
                    if id.is_null() {
                        fimo_std::emit_error!(module.context(), "`id` is null");
                        return Err(Error::EINVAL);
                    }
                    id.write(Self.worker_id(module)?.0);
                    Ok(())
                })
            }
        })
        .flatten()
        .map_or_else(|e| e.into_error(), |_| Error::EOK.into_error())
    }

    unsafe extern "C" fn worker_group_ffi(
        _this: *mut std::ffi::c_void,
        group: *mut bindings::FiTasksWorkerGroup,
    ) -> std_bindings::FimoError {
        fimo_std::panic::catch_unwind(|| {
            // Safety: Is safe since we are calling it from an exported symbol.
            unsafe {
                TasksModuleToken::with_current_unlocked(|module| {
                    let _span = fimo_std::span_trace!(module.context(), "group: {group:?}");
                    if group.is_null() {
                        fimo_std::emit_error!(module.context(), "`group` is null");
                        return Err(Error::EINVAL);
                    }
                    group.write(
                        Self.worker_group(module)
                            .map(|g| WorkerGroupFFI(g).into_ffi())?,
                    );
                    Ok(())
                })
            }
        })
        .flatten()
        .map_or_else(|e| e.into_error(), |_| Error::EOK.into_error())
    }

    unsafe extern "C" fn worker_group_by_id_ffi(
        _this: *mut std::ffi::c_void,
        id: usize,
        group: *mut bindings::FiTasksWorkerGroup,
    ) -> std_bindings::FimoError {
        fimo_std::panic::catch_unwind(|| {
            // Safety: Is safe since we are calling it from an exported symbol.
            unsafe {
                TasksModuleToken::with_current_unlocked(|module| {
                    let _span =
                        fimo_std::span_trace!(module.context(), "id: {id:?}, group: {group:?}");
                    if group.is_null() {
                        fimo_std::emit_error!(module.context(), "`group` is null");
                        return Err(Error::EINVAL);
                    }
                    group.write(
                        Self.worker_group_by_id(module, WorkerGroupId(id))
                            .map(|g| WorkerGroupFFI(g).into_ffi())?,
                    );
                    Ok(())
                })
            }
        })
        .flatten()
        .map_or_else(|e| e.into_error(), |_| Error::EOK.into_error())
    }

    unsafe extern "C" fn query_worker_groups_ffi(
        _this: *mut std::ffi::c_void,
        query: *mut *mut bindings::FiTasksWorkerGroupQuery,
    ) -> std_bindings::FimoError {
        fimo_std::panic::catch_unwind(|| {
            // Safety: Is safe since we are calling it from an exported symbol.
            unsafe {
                TasksModuleToken::with_current_unlocked(|module| {
                    let _span = fimo_std::span_trace!(module.context(), "query: {query:?}");
                    if query.is_null() {
                        fimo_std::emit_error!(module.context(), "`query` is null");
                        return Err(Error::EINVAL);
                    }
                    query.write(Self.query_worker_groups(module)?.into_ffi());
                    Ok(())
                })
            }
        })
        .flatten()
        .map_or_else(|e| e.into_error(), |_| Error::EOK.into_error())
    }

    unsafe extern "C" fn release_worker_group_query_ffi(
        _this: *mut std::ffi::c_void,
        query: *mut bindings::FiTasksWorkerGroupQuery,
    ) -> std_bindings::FimoError {
        fimo_std::panic::catch_unwind(|| {
            // Safety: Is safe since we are calling it from an exported symbol.
            unsafe {
                TasksModuleToken::with_current_unlocked(|module| {
                    let _span = fimo_std::span_trace!(module.context(), "query: {query:?}");
                    if query.is_null() {
                        fimo_std::emit_error!(module.context(), "`query` is null");
                        return Err(Error::EINVAL);
                    }

                    let query = WorkerGroupQuery::from_ffi(query);
                    fimo_std::emit_trace!(module.context(), "dropping query: {query:?}");
                    drop(query);
                    Ok(())
                })
            }
        })
        .flatten()
        .map_or_else(|e| e.into_error(), |_| Error::EOK.into_error())
    }

    unsafe extern "C" fn create_worker_group_ffi(
        _this: *mut std::ffi::c_void,
        cfg: bindings::FiTasksWorkerGroupConfig,
        group: *mut bindings::FiTasksWorkerGroup,
    ) -> std_bindings::FimoError {
        fimo_std::panic::catch_unwind(|| {
            // Safety: Is safe since we are calling it from an exported symbol.
            unsafe {
                TasksModuleToken::with_current_unlocked(|module| {
                    let _span = fimo_std::span_trace!(module.context(), "group: {group:?}");
                    if !cfg.next.is_null() {
                        fimo_std::emit_error!(module.context(), "`cfg.next` is not null");
                        return Err(Error::EINVAL);
                    }

                    let name = {
                        if cfg.name.is_null() {
                            fimo_std::emit_error!(module.context(), "`cfg.name` is null");
                            return Err(Error::EINVAL);
                        }
                        CStr::from_ptr(cfg.name)
                    };
                    let stacks = {
                        if cfg.stacks.is_null() || cfg.num_stacks == 0 {
                            fimo_std::emit_error!(module.context(), "`cfg` specifies no stacks");
                            return Err(Error::EINVAL);
                        }
                        std::slice::from_raw_parts(cfg.stacks, cfg.num_stacks)
                    };
                    let default_stack_index = {
                        if cfg.default_stack_index >= stacks.len() {
                            fimo_std::emit_error!(
                                module.context(),
                                "`cfg.default_stack_index` is out of bounds"
                            );
                            return Err(Error::EINVAL);
                        }
                        cfg.default_stack_index
                    };
                    let number_of_workers = NonZeroUsize::new(cfg.number_of_workers);
                    let is_queryable = cfg.is_queryable;

                    if cfg.name.is_null() {
                        fimo_std::emit_error!(module.context(), "`cfg.next` is not null");
                        return Err(Error::EINVAL);
                    }

                    if group.is_null() {
                        fimo_std::emit_error!(module.context(), "`query` is null");
                        return Err(Error::EINVAL);
                    }

                    group.write(
                        Self.create_worker_group(
                            module,
                            name,
                            stacks,
                            default_stack_index,
                            number_of_workers,
                            is_queryable,
                        )
                        .map(|g| WorkerGroupFFI(g).into_ffi())?,
                    );
                    Ok(())
                })
            }
        })
        .flatten()
        .map_or_else(|e| e.into_error(), |_| Error::EOK.into_error())
    }

    unsafe extern "C" fn yield_ffi(_this: *mut std::ffi::c_void) -> std_bindings::FimoError {
        fimo_std::panic::catch_unwind(|| {
            // Safety: Is safe since we are calling it from an exported symbol.
            unsafe {
                TasksModuleToken::with_current_unlocked(|module| {
                    let _span = fimo_std::span_trace!(module.context(), "");
                    Self.yield_now(module)
                })
            }
        })
        .flatten()
        .map_or_else(|e| e.into_error(), |_| Error::EOK.into_error())
    }

    unsafe extern "C" fn abort_ffi(
        _this: *mut std::ffi::c_void,
        error: *mut std::ffi::c_void,
    ) -> std_bindings::FimoError {
        fimo_std::panic::catch_unwind(|| {
            // Safety: Is safe since we are calling it from an exported symbol.
            unsafe {
                TasksModuleToken::with_current_unlocked(|module| {
                    let _span = fimo_std::span_trace!(module.context(), "error: {error:?}");
                    Self.abort(module, error)
                })
            }
        })
        .flatten()
        .map_or_else(|e| e.into_error(), |_| Error::EOK.into_error())
    }

    unsafe extern "C" fn sleep_ffi(
        _this: *mut std::ffi::c_void,
        duration: std_bindings::FimoDuration,
    ) -> std_bindings::FimoError {
        fimo_std::panic::catch_unwind(|| {
            // Safety: Is safe since we are calling it from an exported symbol.
            unsafe {
                TasksModuleToken::with_current_unlocked(|module| {
                    let _span = fimo_std::span_trace!(module.context(), "duration: {duration:?}");
                    let duration = std::time::Duration::new(duration.secs, duration.nanos);
                    Self.sleep_for(module, duration)
                })
            }
        })
        .flatten()
        .map_or_else(|e| e.into_error(), |_| Error::EOK.into_error())
    }

    unsafe extern "C" fn tss_set_ffi(
        _this: *mut std::ffi::c_void,
        key: bindings::FiTasksTssKey,
        value: *mut std::ffi::c_void,
        dtor: bindings::FiTasksTssDtor,
    ) -> std_bindings::FimoError {
        fimo_std::panic::catch_unwind(|| {
            // Safety: Is safe since we are calling it from an exported symbol.
            unsafe {
                TasksModuleToken::with_current_unlocked(|module| {
                    let _span = fimo_std::span_trace!(
                        module.context(),
                        "key: {key:?}, value: {value:?}, dtor: {dtor:?}"
                    );
                    Self.tss_set(module, key, value, dtor)
                })
            }
        })
        .flatten()
        .map_or_else(|e| e.into_error(), |_| Error::EOK.into_error())
    }

    unsafe extern "C" fn tss_get_ffi(
        _this: *mut std::ffi::c_void,
        key: bindings::FiTasksTssKey,
        value: *mut *mut std::ffi::c_void,
    ) -> std_bindings::FimoError {
        fimo_std::panic::catch_unwind(|| {
            // Safety: Is safe since we are calling it from an exported symbol.
            unsafe {
                TasksModuleToken::with_current_unlocked(|module| {
                    let _span =
                        fimo_std::span_trace!(module.context(), "key: {key:?}, value: {value:?}");
                    if value.is_null() {
                        fimo_std::emit_error!(module.context(), "`value` is null");
                        return Err(Error::EINVAL);
                    }
                    value.write(Self.tss_get(module, key)?);
                    Ok(())
                })
            }
        })
        .flatten()
        .map_or_else(|e| e.into_error(), |_| Error::EOK.into_error())
    }

    unsafe extern "C" fn tss_clear_ffi(
        _this: *mut std::ffi::c_void,
        key: bindings::FiTasksTssKey,
    ) -> std_bindings::FimoError {
        fimo_std::panic::catch_unwind(|| {
            // Safety: Is safe since we are calling it from an exported symbol.
            unsafe {
                TasksModuleToken::with_current_unlocked(|module| {
                    let _span = fimo_std::span_trace!(module.context(), "key: {key:?}");
                    Self.tss_clear(module, key)
                })
            }
        })
        .flatten()
        .map_or_else(|e| e.into_error(), |_| Error::EOK.into_error())
    }

    unsafe extern "C" fn park_conditionally_ffi(
        _this: *mut std::ffi::c_void,
        _key: *const std::ffi::c_void,
        _validate: Option<unsafe extern "C" fn(*mut std::ffi::c_void) -> bool>,
        _validate_data: *mut std::ffi::c_void,
        _before_sleep: Option<unsafe extern "C" fn(*mut std::ffi::c_void)>,
        _before_sleep_data: *mut std::ffi::c_void,
        _timed_out: Option<
            unsafe extern "C" fn(*mut std::ffi::c_void, *const std::ffi::c_void, bool),
        >,
        _timed_out_data: *mut std::ffi::c_void,
        _park_token: *const std::ffi::c_void,
        _timeout: *const std_bindings::FimoDuration,
        _result: *mut bindings::FiTasksParkResult,
    ) -> std_bindings::FimoError {
        Error::ENOSYS.into_error()
    }

    unsafe extern "C" fn unpark_one_ffi(
        _this: *mut std::ffi::c_void,
        _key: *const std::ffi::c_void,
        _callback: Option<
            unsafe extern "C" fn(
                *mut std::ffi::c_void,
                bindings::FiTasksUnparkResult,
            ) -> *const std::ffi::c_void,
        >,
        _callback_data: *mut std::ffi::c_void,
        _result: *mut bindings::FiTasksUnparkResult,
    ) -> std_bindings::FimoError {
        Error::ENOSYS.into_error()
    }

    unsafe extern "C" fn unpark_all_ffi(
        _this: *mut std::ffi::c_void,
        _key: *const std::ffi::c_void,
        _unpark_token: *const std::ffi::c_void,
        _unparked_tasks: *mut usize,
    ) -> std_bindings::FimoError {
        Error::ENOSYS.into_error()
    }

    unsafe extern "C" fn unpark_requeue_ffi(
        _this: *mut std::ffi::c_void,
        _key_from: *const std::ffi::c_void,
        _key_to: *const std::ffi::c_void,
        _validate: Option<
            unsafe extern "C" fn(*mut std::ffi::c_void) -> bindings::FiTasksRequeueOp,
        >,
        _validate_data: *mut std::ffi::c_void,
        _callback: Option<
            unsafe extern "C" fn(
                *mut std::ffi::c_void,
                bindings::FiTasksRequeueOp,
                bindings::FiTasksUnparkResult,
            ) -> *const std::ffi::c_void,
        >,
        _callback_data: *mut std::ffi::c_void,
        _result: *mut bindings::FiTasksUnparkResult,
    ) -> std_bindings::FimoError {
        Error::ENOSYS.into_error()
    }

    unsafe extern "C" fn unpark_filter_ffi(
        _this: *mut std::ffi::c_void,
        _key: *const std::ffi::c_void,
        _filter: Option<
            unsafe extern "C" fn(
                *mut std::ffi::c_void,
                *const std::ffi::c_void,
            ) -> bindings::FiTasksUnparkFilterOp,
        >,
        _filter_data: *mut std::ffi::c_void,
        _callback: Option<
            unsafe extern "C" fn(
                *mut std::ffi::c_void,
                bindings::FiTasksUnparkResult,
            ) -> *const std::ffi::c_void,
        >,
        _callback_data: *mut std::ffi::c_void,
        _result: *mut bindings::FiTasksUnparkResult,
    ) -> std_bindings::FimoError {
        Error::ENOSYS.into_error()
    }
}
