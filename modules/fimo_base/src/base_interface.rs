use crate::base_api::BaseAPI;
use emf_core_base_rs::ffi::module::InterfaceExtension;
use emf_core_base_rs::ffi::version::Version;
use emf_core_base_rs::ffi::{CBaseInterface, TypeWrapper};
use lazy_static::lazy_static;
use std::ptr::NonNull;

lazy_static! {
    static ref EXTENSIONS: Vec<InterfaceExtension> =
        vec![InterfaceExtension::from("unwind_internal")];
}

/// Wrapper of the base interface.
#[derive(Debug)]
#[repr(transparent)]
pub struct BaseInterfaceWrapper {
    interface: CBaseInterface,
}

impl Default for BaseInterfaceWrapper {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for BaseInterfaceWrapper {
    fn drop(&mut self) {
        // Safety: Guaranteed by the fact that the interface is immutable.
        unsafe {
            Box::<BaseAPI>::from_raw(self.interface.base_module.take().unwrap().cast().as_ptr())
        };
    }
}

impl BaseInterfaceWrapper {
    /// Initialize the interface.
    #[inline]
    pub fn new() -> Self {
        let base = Box::new(BaseAPI::new());
        let base_version = base.version();
        let base = NonNull::from(Box::leak(base)).cast();

        #[allow(invalid_value)]
        Self {
            interface: CBaseInterface {
                version: base_version,
                base_module: Some(base),
                sys_shutdown_fn: TypeWrapper(sys_bindings::shutdown),
                sys_panic_fn: TypeWrapper(sys_bindings::panic),
                sys_has_function_fn: TypeWrapper(sys_bindings::has_fn),
                sys_get_function_fn: TypeWrapper(sys_bindings::get_fn),
                sys_lock_fn: TypeWrapper(sys_bindings::lock),
                sys_try_lock_fn: TypeWrapper(sys_bindings::try_lock),
                sys_unlock_fn: TypeWrapper(sys_bindings::unlock),
                sys_get_sync_handler_fn: TypeWrapper(sys_bindings::get_sync_handler),
                sys_set_sync_handler_fn: TypeWrapper(sys_bindings::set_sync_handler),
                version_new_short_fn: TypeWrapper(version_bindings::new_short),
                version_new_long_fn: TypeWrapper(version_bindings::new_long),
                version_new_full_fn: TypeWrapper(version_bindings::new_full),
                version_from_string_fn: TypeWrapper(version_bindings::from_string),
                version_string_length_short_fn: TypeWrapper(version_bindings::string_length_short),
                version_string_length_long_fn: TypeWrapper(version_bindings::string_length_long),
                version_string_length_full_fn: TypeWrapper(version_bindings::string_length_full),
                version_as_string_short_fn: TypeWrapper(version_bindings::as_string_short),
                version_as_string_long_fn: TypeWrapper(version_bindings::as_string_long),
                version_as_string_full_fn: TypeWrapper(version_bindings::as_string_full),
                version_string_is_valid_fn: TypeWrapper(version_bindings::string_is_valid),
                version_compare_fn: TypeWrapper(version_bindings::compare),
                version_compare_weak_fn: TypeWrapper(version_bindings::compare_weak),
                version_compare_strong_fn: TypeWrapper(version_bindings::compare_strong),
                version_is_compatible_fn: TypeWrapper(version_bindings::is_compatible),
                library_register_loader_fn: unsafe { std::mem::zeroed() },
                library_unregister_loader_fn: unsafe { std::mem::zeroed() },
                library_get_loader_interface_fn: unsafe { std::mem::zeroed() },
                library_get_loader_handle_from_type_fn: unsafe { std::mem::zeroed() },
                library_get_loader_handle_from_library_fn: unsafe { std::mem::zeroed() },
                library_get_num_loaders_fn: unsafe { std::mem::zeroed() },
                library_library_exists_fn: unsafe { std::mem::zeroed() },
                library_type_exists_fn: unsafe { std::mem::zeroed() },
                library_get_library_types_fn: unsafe { std::mem::zeroed() },
                library_create_library_handle_fn: unsafe { std::mem::zeroed() },
                library_remove_library_handle_fn: unsafe { std::mem::zeroed() },
                library_link_library_fn: unsafe { std::mem::zeroed() },
                library_get_internal_library_handle_fn: unsafe { std::mem::zeroed() },
                library_load_fn: unsafe { std::mem::zeroed() },
                library_unload_fn: unsafe { std::mem::zeroed() },
                library_get_data_symbol_fn: unsafe { std::mem::zeroed() },
                library_get_function_symbol_fn: unsafe { std::mem::zeroed() },
                module_register_loader_fn: unsafe { std::mem::zeroed() },
                module_unregister_loader_fn: unsafe { std::mem::zeroed() },
                module_get_loader_interface_fn: unsafe { std::mem::zeroed() },
                module_get_loader_handle_from_type_fn: unsafe { std::mem::zeroed() },
                module_get_loader_handle_from_module_fn: unsafe { std::mem::zeroed() },
                module_get_num_modules_fn: unsafe { std::mem::zeroed() },
                module_get_num_loaders_fn: unsafe { std::mem::zeroed() },
                module_get_num_exported_interfaces_fn: unsafe { std::mem::zeroed() },
                module_module_exists_fn: unsafe { std::mem::zeroed() },
                module_type_exists_fn: unsafe { std::mem::zeroed() },
                module_exported_interface_exists_fn: unsafe { std::mem::zeroed() },
                module_get_modules_fn: unsafe { std::mem::zeroed() },
                module_get_module_types_fn: unsafe { std::mem::zeroed() },
                module_get_exported_interfaces_fn: unsafe { std::mem::zeroed() },
                module_get_exported_interface_handle_fn: unsafe { std::mem::zeroed() },
                module_create_module_handle_fn: unsafe { std::mem::zeroed() },
                module_remove_module_handle_fn: unsafe { std::mem::zeroed() },
                module_link_module_fn: unsafe { std::mem::zeroed() },
                module_get_internal_module_handle_fn: unsafe { std::mem::zeroed() },
                module_add_module_fn: unsafe { std::mem::zeroed() },
                module_remove_module_fn: unsafe { std::mem::zeroed() },
                module_load_fn: unsafe { std::mem::zeroed() },
                module_unload_fn: unsafe { std::mem::zeroed() },
                module_initialize_fn: unsafe { std::mem::zeroed() },
                module_terminate_fn: unsafe { std::mem::zeroed() },
                module_add_dependency_fn: unsafe { std::mem::zeroed() },
                module_remove_dependency_fn: unsafe { std::mem::zeroed() },
                module_export_interface_fn: unsafe { std::mem::zeroed() },
                module_get_load_dependencies_fn: unsafe { std::mem::zeroed() },
                module_get_runtime_dependencies_fn: unsafe { std::mem::zeroed() },
                module_get_exportable_interfaces_fn: unsafe { std::mem::zeroed() },
                module_fetch_status_fn: unsafe { std::mem::zeroed() },
                module_get_module_path_fn: unsafe { std::mem::zeroed() },
                module_get_module_info_fn: unsafe { std::mem::zeroed() },
                module_get_interface_fn: unsafe { std::mem::zeroed() },
            },
        }
    }

    /// Fetches the initialized version.
    #[inline]
    pub fn version(&self) -> Version {
        self.interface.version
    }

    /// Fetches the implemented extensions.
    #[inline]
    pub fn extensions(&self) -> &[InterfaceExtension] {
        EXTENSIONS.as_slice()
    }
}

pub(crate) mod sys_bindings {
    use crate::base_api::BaseAPI;
    use emf_core_base_rs::ffi::collections::{NonNullConst, Optional};
    use emf_core_base_rs::ffi::sys::sync_handler::SyncHandlerInterface;
    use emf_core_base_rs::ffi::{Bool, CBase, CBaseFn, FnId};
    use emf_core_base_rs::sys::sync_handler::{SyncHandler, SyncHandlerAPI};
    use std::ffi::CStr;
    use std::ptr::NonNull;

    pub unsafe extern "C-unwind" fn shutdown(base_module: Option<NonNull<CBase>>) -> ! {
        BaseAPI::from_raw_unlocked(base_module.unwrap())
            .get_sys_api()
            .shutdown()
    }

    pub unsafe extern "C-unwind" fn panic(
        base_module: Option<NonNull<CBase>>,
        error: Option<NonNullConst<u8>>,
    ) -> ! {
        match error {
            None => BaseAPI::from_raw_unlocked(base_module.unwrap())
                .get_sys_api()
                .panic(None),
            Some(error) => BaseAPI::from_raw_unlocked(base_module.unwrap())
                .get_sys_api()
                .panic(Some(CStr::from_ptr(error.cast().as_ptr()))),
        }
    }

    pub unsafe extern "C-unwind" fn has_fn(base_module: Option<NonNull<CBase>>, id: FnId) -> Bool {
        if BaseAPI::from_raw_locked(base_module.unwrap())
            .get_sys_api()
            .has_fn(id)
        {
            Bool::True
        } else {
            Bool::False
        }
    }

    #[allow(improper_ctypes_definitions)]
    pub unsafe extern "C-unwind" fn get_fn(
        base_module: Option<NonNull<CBase>>,
        id: FnId,
    ) -> Optional<CBaseFn> {
        BaseAPI::from_raw_locked(base_module.unwrap())
            .get_sys_api()
            .get_fn(id)
            .map_or(Optional::none(), Optional::some)
    }

    pub unsafe extern "C-unwind" fn lock(base_module: Option<NonNull<CBase>>) {
        BaseAPI::from_raw_unlocked(base_module.unwrap())
            .get_sys_api()
            .lock();
    }

    pub unsafe extern "C-unwind" fn try_lock(base_module: Option<NonNull<CBase>>) -> Bool {
        if BaseAPI::from_raw_unlocked(base_module.unwrap())
            .get_sys_api()
            .try_lock()
            .is_ok()
        {
            Bool::True
        } else {
            Bool::False
        }
    }

    pub unsafe extern "C-unwind" fn unlock(base_module: Option<NonNull<CBase>>) {
        BaseAPI::from_raw_locked(base_module.unwrap())
            .get_sys_api()
            .unlock();
    }

    pub unsafe extern "C-unwind" fn get_sync_handler(
        base_module: Option<NonNull<CBase>>,
    ) -> NonNullConst<SyncHandlerInterface> {
        BaseAPI::from_raw_locked(base_module.unwrap())
            .get_sys_api()
            .get_sync_handler()
            .to_interface()
    }

    pub unsafe extern "C-unwind" fn set_sync_handler(
        base_module: Option<NonNull<CBase>>,
        handler: Option<NonNullConst<SyncHandlerInterface>>,
    ) {
        BaseAPI::from_raw_locked(base_module.unwrap())
            .get_sys_api()
            .set_sync_handler(handler.map(|h| SyncHandler::from_interface(h)))
    }
}

pub(crate) mod version_bindings {
    use crate::base_api::BaseAPI;
    use emf_core_base_rs::ffi::collections::{ConstSpan, MutSpan, NonNullConst, Result};
    use emf_core_base_rs::ffi::version::{Error, ReleaseType, Version};
    use emf_core_base_rs::ffi::{Bool, CBase};
    use std::cmp::Ordering;
    use std::ptr::NonNull;

    #[allow(improper_ctypes_definitions)]
    pub unsafe extern "C-unwind" fn new_short(
        base_module: Option<NonNull<CBase>>,
        major: i32,
        minor: i32,
        patch: i32,
    ) -> Version {
        BaseAPI::from_raw_unlocked(base_module.unwrap())
            .get_version_api()
            .new_short(major, minor, patch)
    }

    #[allow(improper_ctypes_definitions)]
    pub unsafe extern "C-unwind" fn new_long(
        base_module: Option<NonNull<CBase>>,
        major: i32,
        minor: i32,
        patch: i32,
        release_type: ReleaseType,
        release_number: i8,
    ) -> Version {
        BaseAPI::from_raw_unlocked(base_module.unwrap())
            .get_version_api()
            .new_long(major, minor, patch, release_type, release_number)
    }

    #[allow(improper_ctypes_definitions)]
    pub unsafe extern "C-unwind" fn new_full(
        base_module: Option<NonNull<CBase>>,
        major: i32,
        minor: i32,
        patch: i32,
        release_type: ReleaseType,
        release_number: i8,
        build: i64,
    ) -> Version {
        BaseAPI::from_raw_unlocked(base_module.unwrap())
            .get_version_api()
            .new_full(major, minor, patch, release_type, release_number, build)
    }

    #[allow(improper_ctypes_definitions)]
    pub unsafe extern "C-unwind" fn from_string(
        base_module: Option<NonNull<CBase>>,
        buffer: NonNullConst<ConstSpan<u8>>,
    ) -> Result<Version, Error> {
        BaseAPI::from_raw_unlocked(base_module.unwrap())
            .get_version_api()
            .from_string(std::str::from_utf8_unchecked(buffer.as_ref().as_ref()))
            .map_or_else(Result::new_err, Result::new_ok)
    }

    pub unsafe extern "C-unwind" fn string_length_short(
        base_module: Option<NonNull<CBase>>,
        version: NonNullConst<Version>,
    ) -> usize {
        BaseAPI::from_raw_unlocked(base_module.unwrap())
            .get_version_api()
            .string_length_short(version.as_ref())
    }

    pub unsafe extern "C-unwind" fn string_length_long(
        base_module: Option<NonNull<CBase>>,
        version: NonNullConst<Version>,
    ) -> usize {
        BaseAPI::from_raw_unlocked(base_module.unwrap())
            .get_version_api()
            .string_length_long(version.as_ref())
    }

    pub unsafe extern "C-unwind" fn string_length_full(
        base_module: Option<NonNull<CBase>>,
        version: NonNullConst<Version>,
    ) -> usize {
        BaseAPI::from_raw_unlocked(base_module.unwrap())
            .get_version_api()
            .string_length_full(version.as_ref())
    }

    #[allow(improper_ctypes_definitions)]
    pub unsafe extern "C-unwind" fn as_string_short(
        base_module: Option<NonNull<CBase>>,
        version: NonNullConst<Version>,
        buffer: NonNull<MutSpan<u8>>,
    ) -> Result<usize, Error> {
        BaseAPI::from_raw_unlocked(base_module.unwrap())
            .get_version_api()
            .as_string_short(
                version.as_ref(),
                std::str::from_utf8_unchecked_mut((&mut *buffer.as_ptr()).as_mut()),
            )
            .map_or_else(Result::new_err, Result::new_ok)
    }

    #[allow(improper_ctypes_definitions)]
    pub unsafe extern "C-unwind" fn as_string_long(
        base_module: Option<NonNull<CBase>>,
        version: NonNullConst<Version>,
        buffer: NonNull<MutSpan<u8>>,
    ) -> Result<usize, Error> {
        BaseAPI::from_raw_unlocked(base_module.unwrap())
            .get_version_api()
            .as_string_long(
                version.as_ref(),
                std::str::from_utf8_unchecked_mut((&mut *buffer.as_ptr()).as_mut()),
            )
            .map_or_else(Result::new_err, Result::new_ok)
    }

    #[allow(improper_ctypes_definitions)]
    pub unsafe extern "C-unwind" fn as_string_full(
        base_module: Option<NonNull<CBase>>,
        version: NonNullConst<Version>,
        buffer: NonNull<MutSpan<u8>>,
    ) -> Result<usize, Error> {
        BaseAPI::from_raw_unlocked(base_module.unwrap())
            .get_version_api()
            .as_string_full(
                version.as_ref(),
                std::str::from_utf8_unchecked_mut((&mut *buffer.as_ptr()).as_mut()),
            )
            .map_or_else(Result::new_err, Result::new_ok)
    }

    pub unsafe extern "C-unwind" fn string_is_valid(
        base_module: Option<NonNull<CBase>>,
        version_string: NonNullConst<ConstSpan<u8>>,
    ) -> Bool {
        if BaseAPI::from_raw_unlocked(base_module.unwrap())
            .get_version_api()
            .string_is_valid(std::str::from_utf8_unchecked(
                version_string.as_ref().as_ref(),
            ))
        {
            Bool::True
        } else {
            Bool::False
        }
    }

    pub unsafe extern "C-unwind" fn compare(
        base_module: Option<NonNull<CBase>>,
        lhs: NonNullConst<Version>,
        rhs: NonNullConst<Version>,
    ) -> i32 {
        match BaseAPI::from_raw_unlocked(base_module.unwrap())
            .get_version_api()
            .compare(lhs.as_ref(), rhs.as_ref())
        {
            Ordering::Less => -1,
            Ordering::Equal => 0,
            Ordering::Greater => 1,
        }
    }

    pub unsafe extern "C-unwind" fn compare_weak(
        base_module: Option<NonNull<CBase>>,
        lhs: NonNullConst<Version>,
        rhs: NonNullConst<Version>,
    ) -> i32 {
        match BaseAPI::from_raw_unlocked(base_module.unwrap())
            .get_version_api()
            .compare_weak(lhs.as_ref(), rhs.as_ref())
        {
            Ordering::Less => -1,
            Ordering::Equal => 0,
            Ordering::Greater => 1,
        }
    }

    pub unsafe extern "C-unwind" fn compare_strong(
        base_module: Option<NonNull<CBase>>,
        lhs: NonNullConst<Version>,
        rhs: NonNullConst<Version>,
    ) -> i32 {
        match BaseAPI::from_raw_unlocked(base_module.unwrap())
            .get_version_api()
            .compare_strong(lhs.as_ref(), rhs.as_ref())
        {
            Ordering::Less => -1,
            Ordering::Equal => 0,
            Ordering::Greater => 1,
        }
    }

    pub unsafe extern "C-unwind" fn is_compatible(
        base_module: Option<NonNull<CBase>>,
        lhs: NonNullConst<Version>,
        rhs: NonNullConst<Version>,
    ) -> Bool {
        match BaseAPI::from_raw_unlocked(base_module.unwrap())
            .get_version_api()
            .is_compatible(lhs.as_ref(), rhs.as_ref())
        {
            true => Bool::True,
            false => Bool::False,
        }
    }
}

pub(crate) mod extensions_bindings {
    pub(crate) mod unwind_internal {
        use crate::base_api::BaseAPI;
        use emf_core_base_rs::ffi::collections::NonNullConst;
        use emf_core_base_rs::ffi::extensions::unwind_internal::{
            Context, PanicFn, ShutdownFn, UnwindInternalInterface,
        };
        use emf_core_base_rs::ffi::{CBase, TypeWrapper};
        use std::ptr::NonNull;

        const INTERFACE: UnwindInternalInterface = UnwindInternalInterface {
            set_context_fn: TypeWrapper(set_context),
            get_context_fn: TypeWrapper(get_context),
            set_shutdown_fn_fn: TypeWrapper(set_shutdown),
            get_shutdown_fn_fn: TypeWrapper(get_shutdown),
            set_panic_fn_fn: TypeWrapper(set_panic),
            get_panic_fn_fn: TypeWrapper(get_panic),
        };

        pub unsafe extern "C-unwind" fn get_unwind_internal_interface(
            _base_module: Option<NonNull<CBase>>,
        ) -> NonNullConst<UnwindInternalInterface> {
            NonNullConst::from(&INTERFACE)
        }

        unsafe extern "C-unwind" fn set_context(
            base_module: Option<NonNull<CBase>>,
            context: Option<NonNull<Context>>,
        ) {
            BaseAPI::from_raw_locked(base_module.unwrap())
                .get_sys_api()
                .set_unwind_context(context)
        }

        unsafe extern "C-unwind" fn get_context(
            base_module: Option<NonNull<CBase>>,
        ) -> Option<NonNull<Context>> {
            BaseAPI::from_raw_locked(base_module.unwrap())
                .get_sys_api()
                .get_unwind_context()
        }

        unsafe extern "C-unwind" fn set_shutdown(
            base_module: Option<NonNull<CBase>>,
            shutdown_fn: Option<ShutdownFn>,
        ) {
            BaseAPI::from_raw_locked(base_module.unwrap())
                .get_sys_api()
                .set_unwind_shutdown(shutdown_fn)
        }

        unsafe extern "C-unwind" fn get_shutdown(
            base_module: Option<NonNull<CBase>>,
        ) -> Option<ShutdownFn> {
            BaseAPI::from_raw_locked(base_module.unwrap())
                .get_sys_api()
                .get_unwind_shutdown()
        }

        unsafe extern "C-unwind" fn set_panic(
            base_module: Option<NonNull<CBase>>,
            panic_fn: Option<PanicFn>,
        ) {
            BaseAPI::from_raw_locked(base_module.unwrap())
                .get_sys_api()
                .set_unwind_panic(panic_fn)
        }

        unsafe extern "C-unwind" fn get_panic(
            base_module: Option<NonNull<CBase>>,
        ) -> Option<PanicFn> {
            BaseAPI::from_raw_locked(base_module.unwrap())
                .get_sys_api()
                .get_unwind_panic()
        }
    }
}
