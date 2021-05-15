//! Implementation of the `emf-core-base` interface.
use emf_core_base_rs::ffi::version::Version;
use emf_core_base_rs::ffi::CBase;
use fimo_version_rs::new_short;
use std::cell::UnsafeCell;
use std::ptr::NonNull;

mod library;
mod module;
mod sys;
mod version;

pub use library::LibraryAPI;
pub use module::ModuleAPI;
pub use sys::SysAPI;
pub use version::VersionAPI;

/// An unlocked resource.
#[derive(Debug, Default)]
pub struct Unlocked {}

/// A locked resource.
#[derive(Debug, Default)]
pub struct Locked {}

/// A guarded resource.
#[derive(Debug)]
pub struct DataGuard<'a, T, L = Unlocked> {
    data: &'a mut T,
    lock_type: L,
}

impl<'a, T, L> DataGuard<'a, T, L> {
    /// Creates a new guard.
    pub fn new(data: &'a mut T) -> DataGuard<'a, T, Unlocked> {
        DataGuard {
            data,
            lock_type: Unlocked {},
        }
    }
}

impl<'a, T> DataGuard<'a, T, Unlocked> {
    /// Assumes that the contained data is locked.
    ///
    /// # Safety
    ///
    /// The assumption is not checked.
    pub unsafe fn assume_locked(self) -> DataGuard<'a, T, Locked> {
        DataGuard {
            data: self.data,
            lock_type: Locked {},
        }
    }
}

impl<'a, T> DataGuard<'a, T, Locked> {
    /// Assumes that the contained data is unlocked.
    ///
    /// # Safety
    ///
    /// The assumption is not checked.
    pub unsafe fn assume_unlocked(self) -> DataGuard<'a, T, Unlocked> {
        DataGuard {
            data: self.data,
            lock_type: Unlocked {},
        }
    }
}

/// Interface implementation.
#[derive(Debug)]
pub struct BaseAPI {
    version: Version,
    library_api: UnsafeCell<LibraryAPI>,
    module_api: UnsafeCell<ModuleAPI>,
    sys_api: UnsafeCell<SysAPI>,
    version_api: UnsafeCell<VersionAPI>,
}

impl Default for BaseAPI {
    fn default() -> Self {
        Self::new()
    }
}

impl BaseAPI {
    /// Constructs a new interface.
    #[inline]
    pub fn new() -> Self {
        Self {
            version: new_short(0, 1, 0),
            library_api: UnsafeCell::new(LibraryAPI::new()),
            module_api: UnsafeCell::new(ModuleAPI::new()),
            sys_api: UnsafeCell::new(SysAPI::new()),
            version_api: UnsafeCell::new(VersionAPI::new()),
        }
    }

    /// Interprets the pointer as an unlocked instance.
    ///
    /// # Safety
    ///
    /// The validity of the pointer is not checked.
    /// It is assumed that the instance is unlocked.
    pub unsafe fn from_raw_unlocked(ptr: NonNull<CBase>) -> DataGuard<'static, Self, Unlocked> {
        <DataGuard<'_, Self>>::new(ptr.cast::<Self>().as_mut())
    }

    /// Interprets the pointer as an unlocked instance.
    ///
    /// # Safety
    ///
    /// The validity of the pointer is not checked.
    /// It is assumed that the instance is locked.
    pub unsafe fn from_raw_locked(ptr: NonNull<CBase>) -> DataGuard<'static, Self, Locked> {
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
    pub unsafe fn get_library_api(&self) -> *mut LibraryAPI {
        self.library_api.get()
    }

    /// Fetches the module api.
    ///
    /// # Safety
    ///
    /// This gives direct access to the api, bypassing the locking mechanism.
    #[inline]
    pub unsafe fn get_module_api(&self) -> *mut ModuleAPI {
        self.module_api.get()
    }

    /// Fetches the sys api.
    ///
    /// # Safety
    ///
    /// This gives direct access to the api, bypassing the locking mechanism.
    #[inline]
    pub unsafe fn get_sys_api(&self) -> *mut SysAPI {
        self.sys_api.get()
    }

    /// Fetches the version api.
    ///
    /// # Safety
    ///
    /// This gives direct access to the api, bypassing the locking mechanism.
    #[inline]
    pub unsafe fn get_version_api(&self) -> *mut VersionAPI {
        self.version_api.get()
    }
}

impl<'a> DataGuard<'a, BaseAPI, Unlocked> {
    /// Fetches the library api.
    pub fn get_library_api(&self) -> DataGuard<'a, LibraryAPI, Unlocked> {
        <DataGuard<'a, _>>::new(unsafe { &mut *self.data.get_library_api() })
    }

    /// Fetches the module api.
    pub fn get_module_api(&self) -> DataGuard<'a, ModuleAPI, Unlocked> {
        <DataGuard<'a, _>>::new(unsafe { &mut *self.data.get_module_api() })
    }

    /// Fetches the sys api.
    pub fn get_sys_api(&self) -> DataGuard<'a, SysAPI, Unlocked> {
        <DataGuard<'a, _>>::new(unsafe { &mut *self.data.get_sys_api() })
    }

    /// Fetches the version api.
    pub fn get_version_api(&self) -> DataGuard<'a, VersionAPI, Unlocked> {
        <DataGuard<'a, _>>::new(unsafe { &mut *self.data.get_version_api() })
    }
}

impl<'a> DataGuard<'a, BaseAPI, Locked> {
    /// Fetches the library api.
    pub fn get_library_api(&self) -> DataGuard<'a, LibraryAPI, Locked> {
        unsafe { <DataGuard<'a, _>>::new(&mut *self.data.get_library_api()).assume_locked() }
    }

    /// Fetches the module api.
    pub fn get_module_api(&self) -> DataGuard<'a, ModuleAPI, Locked> {
        unsafe { <DataGuard<'a, _>>::new(&mut *self.data.get_module_api()).assume_locked() }
    }

    /// Fetches the sys api.
    pub fn get_sys_api(&self) -> DataGuard<'a, SysAPI, Locked> {
        unsafe { <DataGuard<'a, _>>::new(&mut *self.data.get_sys_api()).assume_locked() }
    }

    /// Fetches the version api.
    pub fn get_version_api(&self) -> DataGuard<'a, VersionAPI, Locked> {
        unsafe { <DataGuard<'a, _>>::new(&mut *self.data.get_version_api()).assume_locked() }
    }
}
