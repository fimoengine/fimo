use crate::base_api::{BaseAPI, INTERFACE_VERSION};
use emf_core_base_rs::ffi::collections::NonNullConst;
use emf_core_base_rs::ffi::module::InterfaceExtension;
use emf_core_base_rs::ffi::version::Version;
use emf_core_base_rs::ffi::{CBaseBinding, CBaseInterface, CBaseInterfaceVTable, TypeWrapper};
use lazy_static::lazy_static;
use std::mem::MaybeUninit;
use std::ptr::NonNull;

lazy_static! {
    static ref EXTENSIONS: Vec<InterfaceExtension> =
        vec![InterfaceExtension::from("unwind_internal")];
}

union Uninit<T: Copy + Sized> {
    t: T,
    u: (),
}

const unsafe fn uninit<T: Copy + Sized>() -> T {
    Uninit { u: () }.t
}

#[allow(invalid_value)]
const INTERFACE_VTABLE: MaybeUninit<CBaseInterfaceVTable> =
    MaybeUninit::new(CBaseInterfaceVTable {
        version: INTERFACE_VERSION,
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
        library_register_loader_fn: TypeWrapper(library_bindings::register_loader),
        library_unregister_loader_fn: TypeWrapper(library_bindings::unregister_loader),
        library_get_loader_interface_fn: TypeWrapper(library_bindings::get_loader_interface),
        library_get_loader_handle_from_type_fn: TypeWrapper(
            library_bindings::get_loader_handle_from_type,
        ),
        library_get_loader_handle_from_library_fn: TypeWrapper(
            library_bindings::get_loader_handle_from_library,
        ),
        library_get_num_loaders_fn: TypeWrapper(library_bindings::get_num_loaders),
        library_library_exists_fn: TypeWrapper(library_bindings::library_exists),
        library_type_exists_fn: TypeWrapper(library_bindings::type_exists),
        library_get_library_types_fn: TypeWrapper(library_bindings::get_library_types),
        library_create_library_handle_fn: TypeWrapper(library_bindings::create_library_handle),
        library_remove_library_handle_fn: TypeWrapper(library_bindings::remove_library_handle),
        library_link_library_fn: TypeWrapper(library_bindings::link_library),
        library_get_internal_library_handle_fn: TypeWrapper(
            library_bindings::get_internal_library_handle,
        ),
        library_load_fn: TypeWrapper(library_bindings::load),
        library_unload_fn: TypeWrapper(library_bindings::unload),
        library_get_data_symbol_fn: TypeWrapper(library_bindings::get_data_symbol),
        library_get_function_symbol_fn: TypeWrapper(library_bindings::get_function_symbol),
        module_register_loader_fn: unsafe { uninit() },
        module_unregister_loader_fn: unsafe { uninit() },
        module_get_loader_interface_fn: unsafe { uninit() },
        module_get_loader_handle_from_type_fn: unsafe { uninit() },
        module_get_loader_handle_from_module_fn: unsafe { uninit() },
        module_get_num_modules_fn: unsafe { uninit() },
        module_get_num_loaders_fn: unsafe { uninit() },
        module_get_num_exported_interfaces_fn: unsafe { uninit() },
        module_module_exists_fn: unsafe { uninit() },
        module_type_exists_fn: unsafe { uninit() },
        module_exported_interface_exists_fn: unsafe { uninit() },
        module_get_modules_fn: unsafe { uninit() },
        module_get_module_types_fn: unsafe { uninit() },
        module_get_exported_interfaces_fn: unsafe { uninit() },
        module_get_exported_interface_handle_fn: unsafe { uninit() },
        module_create_module_handle_fn: unsafe { uninit() },
        module_remove_module_handle_fn: unsafe { uninit() },
        module_link_module_fn: unsafe { uninit() },
        module_get_internal_module_handle_fn: unsafe { uninit() },
        module_add_module_fn: unsafe { uninit() },
        module_remove_module_fn: unsafe { uninit() },
        module_load_fn: unsafe { uninit() },
        module_unload_fn: unsafe { uninit() },
        module_initialize_fn: unsafe { uninit() },
        module_terminate_fn: unsafe { uninit() },
        module_add_dependency_fn: unsafe { uninit() },
        module_remove_dependency_fn: unsafe { uninit() },
        module_export_interface_fn: unsafe { uninit() },
        module_get_load_dependencies_fn: unsafe { uninit() },
        module_get_runtime_dependencies_fn: unsafe { uninit() },
        module_get_exportable_interfaces_fn: unsafe { uninit() },
        module_fetch_status_fn: unsafe { uninit() },
        module_get_module_path_fn: unsafe { uninit() },
        module_get_module_info_fn: unsafe { uninit() },
        module_get_interface_fn: unsafe { uninit() },
    });

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
        let base = NonNull::from(Box::leak(base)).cast();

        Self {
            interface: CBaseInterface {
                base_module: Some(base),
                vtable: NonNullConst::from(unsafe { INTERFACE_VTABLE.assume_init_ref() }),
            },
        }
    }

    /// Fetches the initialized version.
    #[inline]
    pub fn version(&self) -> Version {
        self.interface.interface_version()
    }

    /// Fetches the implemented extensions.
    #[inline]
    pub fn extensions(&self) -> &[InterfaceExtension] {
        EXTENSIONS.as_slice()
    }
}

pub(crate) mod utilities {
    use emf_core_base_rs::ffi::library::OSPathString;
    use std::path::PathBuf;

    #[cfg(unix)]
    pub unsafe fn os_path_to_path_buf(path: OSPathString) -> PathBuf {
        use std::ffi::OsStr;
        use std::os::unix::ffi::OsStrExt;
        PathBuf::from(OsStr::from_bytes(path.as_ref()).to_os_string())
    }

    #[cfg(windows)]
    pub unsafe fn os_path_to_path_buf(path: OSPathString) -> PathBuf {
        use std::ffi::OsString;
        use std::os::windows::ffi::OsStringExt;
        PathBuf::from(OsString::from_wide(path.as_ref()))
    }
}

pub(crate) mod sys_bindings {
    use crate::base_api::BaseAPI;
    use emf_core_base_rs::ffi::collections::Optional;
    use emf_core_base_rs::ffi::errors::Error;
    use emf_core_base_rs::ffi::sys::sync_handler::SyncHandlerInterface;
    use emf_core_base_rs::ffi::{Bool, CBase, CBaseFn, FnId};
    use emf_core_base_rs::sys::sync_handler::{SyncHandler, SyncHandlerAPI};
    use std::ptr::NonNull;

    pub unsafe extern "C-unwind" fn shutdown(base_module: Option<NonNull<CBase>>) -> ! {
        BaseAPI::from_raw_unlocked(base_module.unwrap())
            .get_sys_api()
            .shutdown()
    }

    pub unsafe extern "C-unwind" fn panic(
        base_module: Option<NonNull<CBase>>,
        error: Optional<Error>,
    ) -> ! {
        BaseAPI::from_raw_unlocked(base_module.unwrap())
            .get_sys_api()
            .panic(error.into_rust().map(From::from))
    }

    pub unsafe extern "C-unwind" fn has_fn(base_module: Option<NonNull<CBase>>, id: FnId) -> Bool {
        BaseAPI::from_raw_locked(base_module.unwrap())
            .get_sys_api()
            .has_fn(id)
            .into()
    }

    #[allow(improper_ctypes_definitions)]
    pub unsafe extern "C-unwind" fn get_fn(
        base_module: Option<NonNull<CBase>>,
        id: FnId,
    ) -> Optional<CBaseFn> {
        BaseAPI::from_raw_locked(base_module.unwrap())
            .get_sys_api()
            .get_fn(id)
            .map_or(Optional::None, Optional::Some)
    }

    pub unsafe extern "C-unwind" fn lock(base_module: Option<NonNull<CBase>>) {
        BaseAPI::from_raw_unlocked(base_module.unwrap())
            .get_sys_api()
            .lock();
    }

    pub unsafe extern "C-unwind" fn try_lock(base_module: Option<NonNull<CBase>>) -> Bool {
        BaseAPI::from_raw_unlocked(base_module.unwrap())
            .get_sys_api()
            .try_lock()
            .is_ok()
            .into()
    }

    pub unsafe extern "C-unwind" fn unlock(base_module: Option<NonNull<CBase>>) {
        BaseAPI::from_raw_locked(base_module.unwrap())
            .get_sys_api()
            .unlock();
    }

    pub unsafe extern "C-unwind" fn get_sync_handler(
        base_module: Option<NonNull<CBase>>,
    ) -> SyncHandlerInterface {
        BaseAPI::from_raw_locked(base_module.unwrap())
            .get_sys_api()
            .get_sync_handler()
            .to_raw()
    }

    pub unsafe extern "C-unwind" fn set_sync_handler(
        base_module: Option<NonNull<CBase>>,
        handler: Optional<SyncHandlerInterface>,
    ) {
        BaseAPI::from_raw_locked(base_module.unwrap())
            .get_sys_api()
            .set_sync_handler(From::from(handler.map(|h| SyncHandler::from_raw(h))))
    }
}

pub(crate) mod version_bindings {
    use crate::base_api::BaseAPI;
    use emf_core_base_rs::ffi::collections::{ConstSpan, MutSpan, NonNullConst, Result};
    use emf_core_base_rs::ffi::errors::Error;
    use emf_core_base_rs::ffi::version::{ReleaseType, Version};
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
            .setup_unwind(move |base| base.get_version_api().new_short(major, minor, patch))
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
        BaseAPI::from_raw_unlocked(base_module.unwrap()).setup_unwind(move |base| {
            base.get_version_api()
                .new_long(major, minor, patch, release_type, release_number)
        })
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
        BaseAPI::from_raw_unlocked(base_module.unwrap()).setup_unwind(move |base| {
            base.get_version_api().new_full(
                major,
                minor,
                patch,
                release_type,
                release_number,
                build,
            )
        })
    }

    #[allow(improper_ctypes_definitions)]
    pub unsafe extern "C-unwind" fn from_string(
        base_module: Option<NonNull<CBase>>,
        buffer: ConstSpan<u8>,
    ) -> Result<Version, Error> {
        BaseAPI::from_raw_unlocked(base_module.unwrap()).setup_unwind(move |base| {
            base.get_version_api()
                .from_string(std::str::from_utf8_unchecked(buffer.as_ref().as_ref()))
                .map_or_else(|e| Result::Err(e.into_inner()), Result::Ok)
        })
    }

    pub unsafe extern "C-unwind" fn string_length_short(
        base_module: Option<NonNull<CBase>>,
        version: NonNullConst<Version>,
    ) -> usize {
        BaseAPI::from_raw_unlocked(base_module.unwrap())
            .setup_unwind(move |base| base.get_version_api().string_length_short(version.as_ref()))
    }

    pub unsafe extern "C-unwind" fn string_length_long(
        base_module: Option<NonNull<CBase>>,
        version: NonNullConst<Version>,
    ) -> usize {
        BaseAPI::from_raw_unlocked(base_module.unwrap())
            .setup_unwind(move |base| base.get_version_api().string_length_long(version.as_ref()))
    }

    pub unsafe extern "C-unwind" fn string_length_full(
        base_module: Option<NonNull<CBase>>,
        version: NonNullConst<Version>,
    ) -> usize {
        BaseAPI::from_raw_unlocked(base_module.unwrap())
            .setup_unwind(move |base| base.get_version_api().string_length_full(version.as_ref()))
    }

    pub unsafe extern "C-unwind" fn as_string_short(
        base_module: Option<NonNull<CBase>>,
        version: NonNullConst<Version>,
        mut buffer: MutSpan<u8>,
    ) -> Result<usize, Error> {
        BaseAPI::from_raw_unlocked(base_module.unwrap()).setup_unwind(move |base| {
            base.get_version_api()
                .as_string_short(
                    version.as_ref(),
                    std::str::from_utf8_mut(buffer.as_mut()).unwrap(),
                )
                .map_or_else(|e| Result::Err(e.into_inner()), Result::Ok)
        })
    }

    pub unsafe extern "C-unwind" fn as_string_long(
        base_module: Option<NonNull<CBase>>,
        version: NonNullConst<Version>,
        mut buffer: MutSpan<u8>,
    ) -> Result<usize, Error> {
        BaseAPI::from_raw_unlocked(base_module.unwrap()).setup_unwind(move |base| {
            base.get_version_api()
                .as_string_long(
                    version.as_ref(),
                    std::str::from_utf8_mut(buffer.as_mut()).unwrap(),
                )
                .map_or_else(|e| Result::Err(e.into_inner()), Result::Ok)
        })
    }

    pub unsafe extern "C-unwind" fn as_string_full(
        base_module: Option<NonNull<CBase>>,
        version: NonNullConst<Version>,
        mut buffer: MutSpan<u8>,
    ) -> Result<usize, Error> {
        BaseAPI::from_raw_unlocked(base_module.unwrap()).setup_unwind(move |base| {
            base.get_version_api()
                .as_string_full(
                    version.as_ref(),
                    std::str::from_utf8_mut(buffer.as_mut()).unwrap(),
                )
                .map_or_else(|e| Result::Err(e.into_inner()), Result::Ok)
        })
    }

    pub unsafe extern "C-unwind" fn string_is_valid(
        base_module: Option<NonNull<CBase>>,
        version_string: ConstSpan<u8>,
    ) -> Bool {
        BaseAPI::from_raw_unlocked(base_module.unwrap()).setup_unwind(move |base| {
            base.get_version_api()
                .string_is_valid(std::str::from_utf8_unchecked(
                    version_string.as_ref().as_ref(),
                ))
                .into()
        })
    }

    pub unsafe extern "C-unwind" fn compare(
        base_module: Option<NonNull<CBase>>,
        lhs: NonNullConst<Version>,
        rhs: NonNullConst<Version>,
    ) -> i32 {
        BaseAPI::from_raw_unlocked(base_module.unwrap()).setup_unwind(move |base| {
            match base.get_version_api().compare(lhs.as_ref(), rhs.as_ref()) {
                Ordering::Less => -1,
                Ordering::Equal => 0,
                Ordering::Greater => 1,
            }
        })
    }

    pub unsafe extern "C-unwind" fn compare_weak(
        base_module: Option<NonNull<CBase>>,
        lhs: NonNullConst<Version>,
        rhs: NonNullConst<Version>,
    ) -> i32 {
        BaseAPI::from_raw_unlocked(base_module.unwrap()).setup_unwind(move |base| {
            match base
                .get_version_api()
                .compare_weak(lhs.as_ref(), rhs.as_ref())
            {
                Ordering::Less => -1,
                Ordering::Equal => 0,
                Ordering::Greater => 1,
            }
        })
    }

    pub unsafe extern "C-unwind" fn compare_strong(
        base_module: Option<NonNull<CBase>>,
        lhs: NonNullConst<Version>,
        rhs: NonNullConst<Version>,
    ) -> i32 {
        BaseAPI::from_raw_unlocked(base_module.unwrap()).setup_unwind(move |base| {
            match base
                .get_version_api()
                .compare_strong(lhs.as_ref(), rhs.as_ref())
            {
                Ordering::Less => -1,
                Ordering::Equal => 0,
                Ordering::Greater => 1,
            }
        })
    }

    pub unsafe extern "C-unwind" fn is_compatible(
        base_module: Option<NonNull<CBase>>,
        lhs: NonNullConst<Version>,
        rhs: NonNullConst<Version>,
    ) -> Bool {
        BaseAPI::from_raw_unlocked(base_module.unwrap()).setup_unwind(move |base| {
            base.get_version_api()
                .is_compatible(lhs.as_ref(), rhs.as_ref())
                .into()
        })
    }
}

pub(crate) mod library_bindings {
    use crate::base_api::BaseAPI;
    use emf_core_base_rs::ffi::collections::{MutSpan, NonNullConst, Result};
    use emf_core_base_rs::ffi::errors::Error;
    use emf_core_base_rs::ffi::library::library_loader::LibraryLoaderInterface;
    use emf_core_base_rs::ffi::library::{
        InternalHandle, LibraryHandle, LibraryType, LoaderHandle, OSPathString, Symbol,
    };
    use emf_core_base_rs::ffi::{Bool, CBase, CBaseFn};
    use emf_core_base_rs::library::library_loader::{LibraryLoader, UnknownLoader};
    use emf_core_base_rs::library::{InternalLibrary, Library, Loader};
    use emf_core_base_rs::ownership::Owned;
    use std::ffi::{c_void, CStr};
    use std::ptr::NonNull;

    struct LibraryLoaderWrapper(LibraryLoaderInterface);

    impl From<&LibraryLoaderWrapper> for LibraryLoader<UnknownLoader<'static>, Owned> {
        fn from(val: &LibraryLoaderWrapper) -> Self {
            unsafe { Self::from_raw(val.0) }
        }
    }

    pub unsafe extern "C-unwind" fn register_loader(
        base_module: Option<NonNull<CBase>>,
        loader: LibraryLoaderInterface,
        lib_type: NonNullConst<LibraryType>,
    ) -> Result<LoaderHandle, Error> {
        BaseAPI::from_raw_locked(base_module.unwrap()).setup_unwind(move |base| {
            base.get_library_api()
                .register_loader(
                    &LibraryLoaderWrapper(loader),
                    std::str::from_utf8(lib_type.as_ref().as_ref()).unwrap(),
                )
                .map_or_else(
                    |e| Result::Err(e.into_inner()),
                    |v| Result::Ok(v.as_handle()),
                )
        })
    }

    pub unsafe extern "C-unwind" fn unregister_loader(
        base_module: Option<NonNull<CBase>>,
        handle: LoaderHandle,
    ) -> Result<i8, Error> {
        BaseAPI::from_raw_locked(base_module.unwrap()).setup_unwind(move |base| {
            base.get_library_api()
                .unregister_loader(Loader::new(handle))
                .map_or_else(|e| Result::Err(e.into_inner()), |_v| Result::Ok(0))
        })
    }

    pub unsafe extern "C-unwind" fn get_loader_interface(
        base_module: Option<NonNull<CBase>>,
        handle: LoaderHandle,
    ) -> Result<LibraryLoaderInterface, Error> {
        BaseAPI::from_raw_locked(base_module.unwrap()).setup_unwind(move |base| {
            base.get_library_api()
                .get_loader_interface(&Loader::new(handle))
                .map_or_else(
                    |e| Result::Err(e.into_inner()),
                    |v: LibraryLoader<UnknownLoader<'_>, Owned>| Result::Ok(v.to_raw()),
                )
        })
    }

    pub unsafe extern "C-unwind" fn get_loader_handle_from_type(
        base_module: Option<NonNull<CBase>>,
        lib_type: NonNullConst<LibraryType>,
    ) -> Result<LoaderHandle, Error> {
        BaseAPI::from_raw_locked(base_module.unwrap()).setup_unwind(move |base| {
            base.get_library_api()
                .get_loader_handle_from_type(
                    std::str::from_utf8(lib_type.as_ref().as_ref()).unwrap(),
                )
                .map_or_else(
                    |e| Result::Err(e.into_inner()),
                    |v| Result::Ok(v.as_handle()),
                )
        })
    }

    pub unsafe extern "C-unwind" fn get_loader_handle_from_library(
        base_module: Option<NonNull<CBase>>,
        handle: LibraryHandle,
    ) -> Result<LoaderHandle, Error> {
        BaseAPI::from_raw_locked(base_module.unwrap()).setup_unwind(move |base| {
            base.get_library_api()
                .get_loader_handle_from_library(&Library::<Owned>::new(handle))
                .map_or_else(
                    |e| Result::Err(e.into_inner()),
                    |v| Result::Ok(v.as_handle()),
                )
        })
    }

    pub unsafe extern "C-unwind" fn get_num_loaders(base_module: Option<NonNull<CBase>>) -> usize {
        BaseAPI::from_raw_locked(base_module.unwrap())
            .setup_unwind(move |base| base.get_library_api().get_num_loaders())
    }

    pub unsafe extern "C-unwind" fn library_exists(
        base_module: Option<NonNull<CBase>>,
        handle: LibraryHandle,
    ) -> Bool {
        BaseAPI::from_raw_locked(base_module.unwrap()).setup_unwind(move |base| {
            base.get_library_api()
                .library_exists(&Library::<Owned>::new(handle))
                .into()
        })
    }

    pub unsafe extern "C-unwind" fn type_exists(
        base_module: Option<NonNull<CBase>>,
        lib_type: NonNullConst<LibraryType>,
    ) -> Bool {
        BaseAPI::from_raw_locked(base_module.unwrap()).setup_unwind(move |base| {
            base.get_library_api()
                .type_exists(std::str::from_utf8(lib_type.as_ref().as_ref()).unwrap())
                .into()
        })
    }

    pub unsafe extern "C-unwind" fn get_library_types(
        base_module: Option<NonNull<CBase>>,
        buffer: MutSpan<LibraryType>,
    ) -> Result<usize, Error> {
        BaseAPI::from_raw_locked(base_module.unwrap()).setup_unwind(move |base| {
            base.get_library_api()
                .get_library_types(buffer)
                .map_or_else(|e| Result::Err(e.into_inner()), Result::Ok)
        })
    }

    pub unsafe extern "C-unwind" fn create_library_handle(
        base_module: Option<NonNull<CBase>>,
    ) -> LibraryHandle {
        BaseAPI::from_raw_locked(base_module.unwrap())
            .setup_unwind(move |base| base.get_library_api().create_library_handle().as_handle())
    }

    pub unsafe extern "C-unwind" fn remove_library_handle(
        base_module: Option<NonNull<CBase>>,
        handle: LibraryHandle,
    ) -> Result<i8, Error> {
        BaseAPI::from_raw_locked(base_module.unwrap()).setup_unwind(move |base| {
            base.get_library_api()
                .remove_library_handle(Library::new(handle))
                .map_or_else(|e| Result::Err(e.into_inner()), |_v| Result::Ok(0))
        })
    }

    pub unsafe extern "C-unwind" fn link_library(
        base_module: Option<NonNull<CBase>>,
        handle: LibraryHandle,
        loader: LoaderHandle,
        internal: InternalHandle,
    ) -> Result<i8, Error> {
        BaseAPI::from_raw_locked(base_module.unwrap()).setup_unwind(move |base| {
            base.get_library_api()
                .link_library(
                    &Library::<Owned>::new(handle),
                    &Loader::<Owned>::new(loader),
                    &InternalLibrary::<Owned>::new(internal),
                )
                .map_or_else(|e| Result::Err(e.into_inner()), |_v| Result::Ok(0))
        })
    }

    pub unsafe extern "C-unwind" fn get_internal_library_handle(
        base_module: Option<NonNull<CBase>>,
        handle: LibraryHandle,
    ) -> Result<InternalHandle, Error> {
        BaseAPI::from_raw_locked(base_module.unwrap()).setup_unwind(move |base| {
            base.get_library_api()
                .get_internal_library_handle(&Library::<Owned>::new(handle))
                .map_or_else(
                    |e| Result::Err(e.into_inner()),
                    |v| Result::Ok(v.as_handle()),
                )
        })
    }

    pub unsafe extern "C-unwind" fn load(
        base_module: Option<NonNull<CBase>>,
        loader: LoaderHandle,
        path: OSPathString,
    ) -> Result<LibraryHandle, Error> {
        let path = super::utilities::os_path_to_path_buf(path);
        BaseAPI::from_raw_locked(base_module.unwrap()).setup_unwind(move |base| {
            base.get_library_api()
                .load_library(&Loader::<Owned>::new(loader), path)
                .map_or_else(
                    |e| Result::Err(e.into_inner()),
                    |v| Result::Ok(v.as_handle()),
                )
        })
    }

    pub unsafe extern "C-unwind" fn unload(
        base_module: Option<NonNull<CBase>>,
        handle: LibraryHandle,
    ) -> Result<i8, Error> {
        BaseAPI::from_raw_locked(base_module.unwrap()).setup_unwind(move |base| {
            base.get_library_api()
                .unload_library(Library::new(handle))
                .map_or_else(|e| Result::Err(e.into_inner()), |_v| Result::Ok(0))
        })
    }

    pub unsafe extern "C-unwind" fn get_data_symbol(
        base_module: Option<NonNull<CBase>>,
        handle: LibraryHandle,
        symbol: NonNullConst<u8>,
    ) -> Result<Symbol<NonNullConst<c_void>>, Error> {
        BaseAPI::from_raw_locked(base_module.unwrap()).setup_unwind(move |base| {
            base.get_library_api()
                .get_data_symbol(
                    &Library::<Owned>::new(handle),
                    CStr::from_ptr(symbol.cast().as_ptr()),
                    |v| &*v.as_ptr(),
                )
                .map_or_else(
                    |e| Result::Err(e.into_inner()),
                    |v| {
                        Result::Ok(Symbol {
                            symbol: NonNullConst::from(AsRef::<c_void>::as_ref(&v)),
                        })
                    },
                )
        })
    }

    #[allow(improper_ctypes_definitions)]
    pub unsafe extern "C-unwind" fn get_function_symbol(
        base_module: Option<NonNull<CBase>>,
        handle: LibraryHandle,
        symbol: NonNullConst<u8>,
    ) -> Result<Symbol<CBaseFn>, Error> {
        BaseAPI::from_raw_locked(base_module.unwrap()).setup_unwind(move |base| {
            base.get_library_api()
                .get_function_symbol(
                    &Library::<Owned>::new(handle),
                    CStr::from_ptr(symbol.cast().as_ptr()),
                    |v| v,
                )
                .map_or_else(
                    |e| Result::Err(e.into_inner()),
                    |v| {
                        Result::Ok(Symbol {
                            symbol: *v.as_ref(),
                        })
                    },
                )
        })
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
