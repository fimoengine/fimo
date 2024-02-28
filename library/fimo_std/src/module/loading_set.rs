use core::{ffi::CStr, marker::PhantomData};

use crate::{
    bindings,
    error::{self, to_result, to_result_indirect_in_place, Error},
    ffi::{FFISharable, FFITransferable},
    version::Version,
};

use super::{ModuleBackendGuard, ModuleExport, ModuleInfo};

/// Result of the filter operation of a [`LoadingSet`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum LoadingFilterRequest {
    Load,
    Skip,
}

/// Status of a module loading operation.
#[derive(Debug)]
pub enum LoadingStatus<'ctx, 'a> {
    Success {
        info: ModuleInfo<'ctx>,
    },
    Error {
        export: ModuleExport<'ctx>,
        should_rollback: &'a mut bool,
    },
}

/// Request of a [`LoadingSet`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum LoadingSetRequest {
    Load,
    Dismiss,
}

/// A set of modules that should be loaded.
#[repr(transparent)]
#[derive(Debug, Clone, Copy)]
pub struct LoadingSet<'a>(*mut bindings::FimoModuleLoadingSet, PhantomData<&'a mut ()>);

impl<'a> LoadingSet<'a> {
    /// Constructs a new loading set.
    ///
    /// If the closure `f` return [`LoadingSetRequest::Load`] then the module backend will start
    /// loading the modules contained in the set. The loading of the set can be dismissed by
    /// returning [`LoadingSetRequest::Dismiss`] or an error from the closure.
    pub fn with_loading_set(
        guard: &mut ModuleBackendGuard<'a>,
        f: impl FnOnce(&mut Self, &mut ModuleBackendGuard<'a>) -> Result<LoadingSetRequest, Error>,
    ) -> error::Result {
        // Safety: The ffi call is safe, as we own all pointers.
        let mut loading_set = unsafe {
            let x = to_result_indirect_in_place(|error, loading_set| {
                *error = bindings::fimo_module_set_new(guard.into_ffi(), loading_set.as_mut_ptr());
            })?;

            LoadingSet::from_ffi(x)
        };

        let request = f(&mut loading_set, guard);
        if matches!(request, Ok(LoadingSetRequest::Dismiss) | Err(_)) {
            // Safety: The ffi call is safe.
            let error = unsafe {
                bindings::fimo_module_set_dismiss(guard.share_to_ffi(), loading_set.into_ffi())
            };
            to_result(error)?;
            request?;
            Ok(())
        } else {
            // Safety: The ffi call is safe.
            let error = unsafe {
                bindings::fimo_module_set_finish(guard.share_to_ffi(), loading_set.into_ffi())
            };
            to_result(error)
        }
    }

    /// Checks if the `LoadingSet` contains the module.
    pub fn has_module(&self, guard: &ModuleBackendGuard<'_>, name: &CStr) -> Result<bool, Error> {
        // Safety: The ffi call is safe, as we own all pointers.
        unsafe {
            to_result_indirect_in_place(|error, has_module| {
                *error = bindings::fimo_module_set_has_module(
                    guard.into_ffi(),
                    self.share_to_ffi(),
                    name.as_ptr(),
                    has_module.as_mut_ptr(),
                );
            })
        }
    }

    /// Checks whether the `LoadingSet` set contains a symbol.
    pub fn has_symbol(
        &self,
        guard: &ModuleBackendGuard<'_>,
        name: &CStr,
        ns: &CStr,
        version: Version,
    ) -> Result<bool, Error> {
        // Safety: The ffi call is safe, as we own all pointers.
        unsafe {
            to_result_indirect_in_place(|error, has_module| {
                *error = bindings::fimo_module_set_has_symbol(
                    guard.into_ffi(),
                    self.share_to_ffi(),
                    name.as_ptr(),
                    ns.as_ptr(),
                    version.into_ffi(),
                    has_module.as_mut_ptr(),
                );
            })
        }
    }

    /// Adds a module to the module set.
    ///
    /// Opens up a module binary to select which modules to load. The binary path `module_path` must
    /// be encoded as `UTF-8`, and point to the binary that contains the modules. The binary
    /// path does not require to contain the file extension. If the path is [`None`], it
    /// iterates over the exported modules of the current binary. Each exported module is then
    /// passed to the `filter`, which can then filter which modules to load.
    ///
    /// This function may skip invalid module exports. Trying to include a module with duplicate
    /// exports will result in an error. This function signals an error, if the binary does not
    /// contain the symbols necessary to query the exported modules, but does not result in an
    /// error, if it does not export any modules. The necessary symbols are setup automatically,
    /// if the binary was linked with the fimo library.
    pub fn append<F>(
        &mut self,
        guard: &ModuleBackendGuard<'_>,
        module_path: Option<&CStr>,
        mut filter: F,
    ) -> error::Result
    where
        F: FnMut(ModuleExport<'_>) -> LoadingFilterRequest,
    {
        unsafe extern "C" fn filter_func<T>(
            export: *const bindings::FimoModuleExport,
            data: *mut core::ffi::c_void,
        ) -> bool
        where
            T: FnMut(ModuleExport<'_>) -> LoadingFilterRequest,
        {
            // Safety: `data` is a mutable reference to `T`.
            let func = unsafe { &mut *data.cast::<T>() };

            // Safety: Is safe.
            let export = unsafe { ModuleExport::borrow_from_ffi(export) };
            let request = func(export);
            matches!(request, LoadingFilterRequest::Load)
        }

        let filter_data = core::ptr::from_mut(&mut filter).cast::<core::ffi::c_void>();
        let filter = Some(filter_func::<F> as _);

        // Safety: The ffi call is safe.
        let error = unsafe {
            bindings::fimo_module_set_append(
                guard.share_to_ffi(),
                self.share_to_ffi(),
                module_path.map_or(core::ptr::null(), |x| x.as_ptr()),
                filter,
                filter_data,
                None,
                None,
                None,
                core::ptr::null_mut(),
            )
        };
        to_result(error)
    }

    /// Adds a module to the module set.
    ///
    /// Opens up a module binary to select which modules to load. The binary path `module_path` must
    /// be encoded as `UTF-8`, and point to the binary that contains the modules. The binary
    /// path does not require to contain the file extension. If the path is [`None`], it
    /// iterates over the exported modules of the current binary. Each exported module is then
    /// passed to the `filter`, which can then filter which modules to load.
    ///
    /// This function may skip invalid module exports. Trying to include a module with duplicate
    /// exports will result in an error. This function signals an error, if the binary does not
    /// contain the symbols necessary to query the exported modules, but does not result in an
    /// error, if it does not export any modules. The necessary symbols are setup automatically,
    /// if the binary was linked with the fimo library.
    ///
    /// Unlike [`LoadingSet::append`], this method requires a status reporting callback, which
    /// is called after the loading/dismissal of the set.
    pub fn append_with_callback<F, L>(
        &mut self,
        guard: &ModuleBackendGuard<'_>,
        module_path: Option<&CStr>,
        filter: F,
        after_load: L,
    ) -> error::Result
    where
        F: FnMut(ModuleExport<'_>) -> LoadingFilterRequest,
        L: FnOnce(LoadingStatus<'_, '_>),
    {
        unsafe extern "C" fn filter_func<T>(
            export: *const bindings::FimoModuleExport,
            data: *mut core::ffi::c_void,
        ) -> bool
        where
            T: FnMut(ModuleExport<'_>) -> LoadingFilterRequest,
        {
            // Safety: `data` is a mutable reference to `T`.
            let func = unsafe { &mut *data.cast::<T>() };

            // Safety: Is safe.
            let export = unsafe { ModuleExport::borrow_from_ffi(export) };
            let request = func(export);
            matches!(request, LoadingFilterRequest::Load)
        }
        unsafe extern "C" fn success_callback<T>(
            module: *const bindings::FimoModuleInfo,
            data: *mut core::ffi::c_void,
        ) where
            T: FnMut(LoadingStatus<'_, '_>),
        {
            // Safety: `data` is a mutable reference to `T`.
            let func = unsafe { &mut *data.cast::<T>() };

            // Safety: Is safe.
            let module = unsafe { ModuleInfo::borrow_from_ffi(module) };
            let status = LoadingStatus::Success { info: module };
            func(status);
        }
        unsafe extern "C" fn error_callback<T>(
            export: *const bindings::FimoModuleExport,
            data: *mut core::ffi::c_void,
        ) -> bool
        where
            T: FnMut(LoadingStatus<'_, '_>),
        {
            // Safety: `data` is a mutable reference to `T`.
            let func = unsafe { &mut *data.cast::<T>() };

            // Safety: Is safe.
            let export = unsafe { ModuleExport::borrow_from_ffi(export) };
            let mut should_rollback = true;
            let status = LoadingStatus::Error {
                export,
                should_rollback: &mut should_rollback,
            };
            func(status);
            should_rollback
        }
        unsafe extern "C" fn cleanup_callback<T>(data: *mut core::ffi::c_void)
        where
            T: FnMut(LoadingStatus<'_, '_>),
        {
            let _ =
                // Safety: Is safe.
                unsafe { alloc::boxed::Box::from_raw_in(data.cast::<T>(), crate::allocator::FimoAllocator) };
        }

        fn append_internal<F, L>(
            this: &mut LoadingSet<'_>,
            guard: &ModuleBackendGuard<'_>,
            module_path: Option<&CStr>,
            mut filter: F,
            after_load: L,
        ) -> error::Result
        where
            F: FnMut(ModuleExport<'_>) -> LoadingFilterRequest,
            L: FnMut(LoadingStatus<'_, '_>),
        {
            let filter_data = core::ptr::from_mut(&mut filter).cast::<core::ffi::c_void>();
            let after_load =
                alloc::boxed::Box::try_new_in(after_load, crate::allocator::FimoAllocator)
                    .map_err(|_err| Error::ENOMEM)?;
            let user_data = alloc::boxed::Box::into_raw(after_load).cast::<core::ffi::c_void>();

            let filter = Some(filter_func::<F> as _);
            let on_success = Some(success_callback::<L> as _);
            let on_error = Some(error_callback::<L> as _);
            let on_cleanup = Some(cleanup_callback::<L> as _);

            // Safety: The ffi call is safe.
            let error = unsafe {
                bindings::fimo_module_set_append(
                    guard.share_to_ffi(),
                    this.share_to_ffi(),
                    module_path.map_or(core::ptr::null(), |x| x.as_ptr()),
                    filter,
                    filter_data,
                    on_success,
                    on_error,
                    on_cleanup,
                    user_data,
                )
            };
            to_result(error)
        }

        let mut after_load = Some(after_load);
        let after_load = move |status: LoadingStatus<'_, '_>| {
            let after_load = after_load.take().unwrap();
            after_load(status);
        };

        append_internal(self, guard, module_path, filter, after_load)
    }
}

// Safety: `FimoModuleLoadingSet` is always `Send + Sync`.
unsafe impl Send for LoadingSet<'_> {}

// Safety: `FimoModuleLoadingSet` is always `Send + Sync`.
unsafe impl Sync for LoadingSet<'_> {}

impl crate::ffi::FFISharable<*mut bindings::FimoModuleLoadingSet> for LoadingSet<'_> {
    type BorrowedView<'a> = LoadingSet<'a>;

    fn share_to_ffi(&self) -> *mut bindings::FimoModuleLoadingSet {
        self.0
    }

    unsafe fn borrow_from_ffi<'a>(
        ffi: *mut bindings::FimoModuleLoadingSet,
    ) -> Self::BorrowedView<'a> {
        LoadingSet(ffi, PhantomData)
    }
}

impl crate::ffi::FFITransferable<*mut bindings::FimoModuleLoadingSet> for LoadingSet<'_> {
    fn into_ffi(self) -> *mut bindings::FimoModuleLoadingSet {
        self.0
    }

    unsafe fn from_ffi(ffi: *mut bindings::FimoModuleLoadingSet) -> Self {
        Self(ffi, PhantomData)
    }
}
