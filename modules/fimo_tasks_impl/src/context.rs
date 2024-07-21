use crate::{
    module_export::TasksModuleToken,
    worker_group::{worker_thread::with_worker_context_lock, WorkerGroupFFI},
};
use fimo_std::{bindings as std_bindings, error::Error, ffi::FFITransferable, module::Module};
use fimo_tasks::bindings;

#[derive(Debug)]
pub struct ContextImpl {}

impl ContextImpl {
    pub(crate) const fn ffi_context() -> fimo_tasks::Context {
        const VTABLE: &bindings::FiTasksVTable = &bindings::FiTasksVTable {
            v0: bindings::FiTasksVTableV0 {
                is_worker: Some(ContextImpl::is_worker),
                task_id: Some(ContextImpl::task_id),
                worker_id: Some(ContextImpl::worker_id),
                worker_group: Some(ContextImpl::worker_group),
                worker_group_by_id: Some(ContextImpl::worker_group_by_id),
                query_worker_groups: Some(ContextImpl::query_worker_groups),
                release_worker_group_query: Some(ContextImpl::release_worker_group_query),
                create_worker_group: Some(ContextImpl::create_worker_group),
                yield_: Some(ContextImpl::yield_),
                abort: Some(ContextImpl::abort),
                sleep: Some(ContextImpl::sleep),
                tss_set: Some(ContextImpl::tss_set),
                tss_get: Some(ContextImpl::tss_get),
                tss_clear: Some(ContextImpl::tss_clear),
                park_conditionally: Some(ContextImpl::park_conditionally),
                unpark_one: Some(ContextImpl::unpark_one),
                unpark_all: Some(ContextImpl::unpark_all),
                unpark_requeue: Some(ContextImpl::unpark_requeue),
                unpark_filter: Some(ContextImpl::unpark_filter),
            },
        };

        let context = bindings::FiTasksContext {
            data: std::ptr::null_mut(),
            vtable: VTABLE,
        };

        // Safety:
        unsafe { std::mem::transmute(context) }
    }

    unsafe extern "C" fn is_worker(_this: *mut std::ffi::c_void) -> bool {
        fimo_std::panic::abort_on_panic(|| {
            // Safety: Is safe since we are calling it from an exported symbol.
            unsafe {
                TasksModuleToken::with_current_unlocked(|_module| {
                    with_worker_context_lock(|_| {}).is_ok()
                })
            }
        })
    }

    unsafe extern "C" fn task_id(
        _this: *mut std::ffi::c_void,
        id: *mut usize,
    ) -> std_bindings::FimoError {
        fimo_std::panic::catch_unwind(|| {
            // Safety: Is safe since we are calling it from an exported symbol.
            unsafe {
                TasksModuleToken::with_current_unlocked(|module| {
                    if id.is_null() {
                        fimo_std::emit_error!(module.context(), "`id` is null");
                        return Err(Error::EINVAL);
                    }

                    with_worker_context_lock(|worker| match &worker.current_task {
                        None => {
                            fimo_std::emit_error!(
                                module.context(),
                                "no task registered for current worker"
                            );
                            Err(Error::EUNKNOWN)
                        }
                        Some(t) => {
                            id.write(t.id().0);
                            Ok(())
                        }
                    })
                    .flatten()
                })
            }
        })
        .flatten()
        .map_or_else(|e| e.into_error(), |_| Error::EOK.into_error())
    }

    unsafe extern "C" fn worker_id(
        _this: *mut std::ffi::c_void,
        id: *mut usize,
    ) -> std_bindings::FimoError {
        fimo_std::panic::catch_unwind(|| {
            // Safety: Is safe since we are calling it from an exported symbol.
            unsafe {
                TasksModuleToken::with_current_unlocked(|module| {
                    if id.is_null() {
                        fimo_std::emit_error!(module.context(), "`id` is null");
                        return Err(Error::EINVAL);
                    }

                    with_worker_context_lock(|worker| {
                        id.write(worker.id.0);
                    })
                })
            }
        })
        .flatten()
        .map_or_else(|e| e.into_error(), |_| Error::EOK.into_error())
    }

    unsafe extern "C" fn worker_group(
        _this: *mut std::ffi::c_void,
        group: *mut bindings::FiTasksWorkerGroup,
    ) -> std_bindings::FimoError {
        fimo_std::panic::catch_unwind(|| {
            // Safety: Is safe since we are calling it from an exported symbol.
            unsafe {
                TasksModuleToken::with_current_unlocked(|module| {
                    if group.is_null() {
                        fimo_std::emit_error!(module.context(), "`group` is null");
                        return Err(Error::EINVAL);
                    }

                    with_worker_context_lock(|worker| {
                        let grp = worker.group.clone();
                        let grp = WorkerGroupFFI(grp).into_ffi();
                        group.write(grp);
                    })
                })
            }
        })
        .flatten()
        .map_or_else(|e| e.into_error(), |_| Error::EOK.into_error())
    }

    unsafe extern "C" fn worker_group_by_id(
        _this: *mut std::ffi::c_void,
        _id: usize,
        _group: *mut bindings::FiTasksWorkerGroup,
    ) -> std_bindings::FimoError {
        Error::ENOSYS.into_error()
    }

    unsafe extern "C" fn query_worker_groups(
        _this: *mut std::ffi::c_void,
        _query: *mut *mut bindings::FiTasksWorkerGroupQuery,
    ) -> std_bindings::FimoError {
        Error::ENOSYS.into_error()
    }

    unsafe extern "C" fn release_worker_group_query(
        _this: *mut std::ffi::c_void,
        _query: *mut bindings::FiTasksWorkerGroupQuery,
    ) -> std_bindings::FimoError {
        Error::ENOSYS.into_error()
    }

    unsafe extern "C" fn create_worker_group(
        _this: *mut std::ffi::c_void,
        _cfg: bindings::FiTasksWorkerGroupConfig,
        _group: *mut bindings::FiTasksWorkerGroup,
    ) -> std_bindings::FimoError {
        Error::ENOSYS.into_error()
    }

    unsafe extern "C" fn yield_(_this: *mut std::ffi::c_void) -> std_bindings::FimoError {
        Error::ENOSYS.into_error()
    }

    unsafe extern "C" fn abort(
        _this: *mut std::ffi::c_void,
        _error: *mut std::ffi::c_void,
    ) -> std_bindings::FimoError {
        Error::ENOSYS.into_error()
    }

    unsafe extern "C" fn sleep(
        _this: *mut std::ffi::c_void,
        _duration: std_bindings::FimoDuration,
    ) -> std_bindings::FimoError {
        Error::ENOSYS.into_error()
    }

    unsafe extern "C" fn tss_set(
        _this: *mut std::ffi::c_void,
        _key: bindings::FiTasksTssKey,
        _value: *mut std::ffi::c_void,
        _dtor: bindings::FiTasksTssDtor,
    ) -> std_bindings::FimoError {
        Error::ENOSYS.into_error()
    }

    unsafe extern "C" fn tss_get(
        _this: *mut std::ffi::c_void,
        _key: bindings::FiTasksTssKey,
        _value: *mut *mut std::ffi::c_void,
    ) -> std_bindings::FimoError {
        Error::ENOSYS.into_error()
    }

    unsafe extern "C" fn tss_clear(
        _this: *mut std::ffi::c_void,
        _key: bindings::FiTasksTssKey,
    ) -> std_bindings::FimoError {
        Error::ENOSYS.into_error()
    }

    unsafe extern "C" fn park_conditionally(
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

    unsafe extern "C" fn unpark_one(
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

    unsafe extern "C" fn unpark_all(
        _this: *mut std::ffi::c_void,
        _key: *const std::ffi::c_void,
        _unpark_token: *const std::ffi::c_void,
        _unparked_tasks: *mut usize,
    ) -> std_bindings::FimoError {
        Error::ENOSYS.into_error()
    }

    unsafe extern "C" fn unpark_requeue(
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

    unsafe extern "C" fn unpark_filter(
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
