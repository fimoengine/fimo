//! Implementation of the `emf-core-base` interface.
use crate::{DataGuard, Locked, Unlocked};
use emf_core_base_rs::ffi::version::Version;
use emf_core_base_rs::ffi::CBase;
use emf_core_base_rs::ownership::Owned;
use emf_core_base_rs::version::ReleaseType;
use emf_core_base_rs::Error;
use fimo_version_rs::new_long;
use std::cell::UnsafeCell;
use std::panic::UnwindSafe;
use std::pin::Pin;
use std::ptr::NonNull;

mod api;
mod native_loader;

pub mod base_interface;

use api::ExitStatus;
pub use api::LibraryAPI;
pub use api::ModuleAPI;
pub use api::SysAPI;
pub use api::VersionAPI;
use native_loader::NativeLoader;

/// Implemented interface version.
pub const INTERFACE_VERSION: Version = new_long(0, 2, 0, ReleaseType::Unstable, 0);

/// Interface implementation.
#[derive(Debug)]
pub struct BaseAPI<'i> {
    version: Version,
    native_loader: Pin<&'i NativeLoader>,
    library_api: UnsafeCell<LibraryAPI<'i>>,
    module_api: UnsafeCell<ModuleAPI<'i>>,
    sys_api: UnsafeCell<SysAPI<'i>>,
    version_api: UnsafeCell<VersionAPI<'i>>,
}

impl Default for BaseAPI<'_> {
    fn default() -> Self {
        Self::new().unwrap()
    }
}

impl Drop for BaseAPI<'_> {
    fn drop(&mut self) {
        // Reset the apis.
        self.library_api.get_mut().reset();

        // Drop the loader.
        // The loader originates from a Box and is mutable.
        let loader = unsafe {
            Box::<NativeLoader>::from_raw(self.native_loader.get_ref() as *const _ as *mut _)
        };
        drop(loader);
    }
}

impl<'i> BaseAPI<'i> {
    /// Constructs a new interface.
    #[inline]
    pub fn new() -> Result<Self, Error<Owned>> {
        let mut api = Self {
            version: INTERFACE_VERSION,
            native_loader: Pin::new(Box::leak(Box::new(NativeLoader::new()))).into_ref(),
            library_api: UnsafeCell::new(LibraryAPI::new()),
            module_api: UnsafeCell::new(ModuleAPI::new()),
            sys_api: UnsafeCell::new(SysAPI::new()),
            version_api: UnsafeCell::new(VersionAPI::new()),
        };

        // Register the native loader.
        use emf_core_base_rs::library::NATIVE_LIBRARY_TYPE_NAME;
        api.library_api
            .get_mut()
            .register_loader(api.native_loader, NATIVE_LIBRARY_TYPE_NAME)?;

        Ok(api)
    }

    /// Interprets the pointer as an unlocked instance.
    ///
    /// # Safety
    ///
    /// The validity of the pointer is not checked.
    /// It is assumed that the instance is unlocked.
    pub unsafe fn from_raw_unlocked(
        ptr: NonNull<CBase>,
    ) -> DataGuard<'static, BaseAPI<'static>, Unlocked> {
        <DataGuard<'_, BaseAPI<'static>>>::new(ptr.cast::<BaseAPI<'static>>().as_mut())
    }

    /// Interprets the pointer as an unlocked instance.
    ///
    /// # Safety
    ///
    /// The validity of the pointer is not checked.
    /// It is assumed that the instance is locked.
    pub unsafe fn from_raw_locked(
        ptr: NonNull<CBase>,
    ) -> DataGuard<'static, BaseAPI<'static>, Locked> {
        Self::from_raw_unlocked(ptr).assume_locked()
    }

    /// Fetches the implemented version.
    #[inline]
    pub fn version(&self) -> Version {
        self.version
    }

    /// Fetches the library api.
    ///
    /// # Safety
    ///
    /// This gives direct access to the api, bypassing the locking mechanism.
    #[inline]
    pub unsafe fn get_library_api(&self) -> *mut LibraryAPI<'i> {
        self.library_api.get()
    }

    /// Fetches the module api.
    ///
    /// # Safety
    ///
    /// This gives direct access to the api, bypassing the locking mechanism.
    #[inline]
    pub unsafe fn get_module_api(&self) -> *mut ModuleAPI<'i> {
        self.module_api.get()
    }

    /// Fetches the sys api.
    ///
    /// # Safety
    ///
    /// This gives direct access to the api, bypassing the locking mechanism.
    #[inline]
    pub unsafe fn get_sys_api(&self) -> *mut SysAPI<'i> {
        self.sys_api.get()
    }

    /// Fetches the version api.
    ///
    /// # Safety
    ///
    /// This gives direct access to the api, bypassing the locking mechanism.
    #[inline]
    pub unsafe fn get_version_api(&self) -> *mut VersionAPI<'i> {
        self.version_api.get()
    }
}

impl<'a, 'i> DataGuard<'a, BaseAPI<'i>, Unlocked> {
    /// Calls a closure, propagating any panic that occurs.
    #[inline]
    pub fn setup_unwind<F: FnOnce(&Self) -> R + UnwindSafe, R>(&self, f: F) -> R {
        self.get_sys_api().setup_unwind(move |_| f(self))
    }

    /// Calls a closure, catching any panic that might occur.
    #[inline]
    pub fn catch_unwind<F: FnOnce(&Self) -> R + UnwindSafe, R>(&self, f: F) -> ExitStatus<R> {
        self.get_sys_api().catch_unwind(move |_| f(self))
    }

    /// Fetches the library api.
    pub fn get_library_api(&self) -> DataGuard<'a, LibraryAPI<'i>, Unlocked> {
        <DataGuard<'a, _>>::new(unsafe { &mut *self.data.get_library_api() })
    }

    /// Fetches the module api.
    pub fn get_module_api(&self) -> DataGuard<'a, ModuleAPI<'i>, Unlocked> {
        <DataGuard<'a, _>>::new(unsafe { &mut *self.data.get_module_api() })
    }

    /// Fetches the sys api.
    pub fn get_sys_api(&self) -> DataGuard<'a, SysAPI<'i>, Unlocked> {
        <DataGuard<'a, _>>::new(unsafe { &mut *self.data.get_sys_api() })
    }

    /// Fetches the version api.
    pub fn get_version_api(&self) -> DataGuard<'a, VersionAPI<'i>, Unlocked> {
        <DataGuard<'a, _>>::new(unsafe { &mut *self.data.get_version_api() })
    }
}

impl<'a, 'i> DataGuard<'a, BaseAPI<'i>, Locked> {
    /// Calls a closure, propagating any panic that occurs.
    #[inline]
    pub fn setup_unwind<F: FnOnce(&Self) -> R + UnwindSafe, R>(&self, f: F) -> R {
        self.get_sys_api().setup_unwind(move |_| f(self))
    }

    /// Calls a closure, catching any panic that might occur.
    #[inline]
    pub fn catch_unwind<F: FnOnce(&Self) -> R + UnwindSafe, R>(&self, f: F) -> ExitStatus<R> {
        self.get_sys_api().catch_unwind(move |_| f(self))
    }

    /// Fetches the library api.
    pub fn get_library_api(&self) -> DataGuard<'a, LibraryAPI<'i>, Locked> {
        unsafe { <DataGuard<'a, _>>::new(&mut *self.data.get_library_api()).assume_locked() }
    }

    /// Fetches the module api.
    pub fn get_module_api(&self) -> DataGuard<'a, ModuleAPI<'i>, Locked> {
        unsafe { <DataGuard<'a, _>>::new(&mut *self.data.get_module_api()).assume_locked() }
    }

    /// Fetches the sys api.
    pub fn get_sys_api(&self) -> DataGuard<'a, SysAPI<'i>, Locked> {
        unsafe { <DataGuard<'a, _>>::new(&mut *self.data.get_sys_api()).assume_locked() }
    }

    /// Fetches the version api.
    pub fn get_version_api(&self) -> DataGuard<'a, VersionAPI<'i>, Locked> {
        unsafe { <DataGuard<'a, _>>::new(&mut *self.data.get_version_api()).assume_locked() }
    }
}
