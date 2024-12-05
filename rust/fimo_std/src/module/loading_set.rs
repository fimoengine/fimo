use core::{ffi::CStr, marker::PhantomData};

use crate::{
    bindings,
    error::{self, to_result, to_result_indirect, to_result_indirect_in_place, Error},
    ffi::{FFISharable, FFITransferable},
    version::Version,
};

use super::{Module, ModuleExport, ModuleInfo, ModuleInfoView, ModuleSubsystem};

/// Result of the filter operation of a [`LoadingSet`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum LoadingFilterRequest {
    Load,
    Skip,
}

/// Status of a module loading operation.
#[derive(Debug)]
pub enum LoadingStatus<'a> {
    Success { info: ModuleInfoView<'a> },
    Error { export: ModuleExport<'a> },
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

impl LoadingSet<'_> {
    /// Constructs a new loading set.
    ///
    /// If the closure `f` return [`LoadingSetRequest::Load`] then the module backend will start
    /// loading the modules contained in the set. The loading of the set can be dismissed by
    /// returning [`LoadingSetRequest::Dismiss`] or an error from the closure.
    pub fn with_loading_set<T: ModuleSubsystem>(
        ctx: &T,
        f: impl FnOnce(&T, &Self) -> Result<LoadingSetRequest, Error>,
    ) -> error::Result {
        // Safety: Is always set.
        let f_ = unsafe { ctx.view().vtable().module_v0.set_new.unwrap_unchecked() };

        // Safety: The ffi call is safe, as we own all pointers.
        let loading_set = unsafe {
            let x = to_result_indirect_in_place(|error, loading_set| {
                *error = f_(ctx.view().data(), loading_set.as_mut_ptr());
            })?;

            LoadingSet::from_ffi(x)
        };

        let request = f(ctx, &loading_set);
        if matches!(request, Ok(LoadingSetRequest::Dismiss) | Err(_)) {
            // Safety: Is always set.
            let f_ = unsafe { ctx.view().vtable().module_v0.set_dismiss.unwrap_unchecked() };

            // Safety: The ffi call is safe.
            let error = unsafe { f_(ctx.view().data(), loading_set.into_ffi()) };
            // Safety:
            unsafe {
                to_result(error)?;
            }
            request?;
            Ok(())
        } else {
            // Safety: Is always set.
            let f_ = unsafe { ctx.view().vtable().module_v0.set_finish.unwrap_unchecked() };

            // Safety: The ffi call is safe.
            let error = unsafe { f_(ctx.view().data(), loading_set.into_ffi()) };
            // Safety:
            unsafe { to_result(error) }
        }
    }

    /// Checks if the `LoadingSet` contains the module.
    pub fn has_module(&self, ctx: &impl ModuleSubsystem, name: &CStr) -> Result<bool, Error> {
        // Safety: Is always set.
        let f = unsafe {
            ctx.view()
                .vtable()
                .module_v0
                .set_has_module
                .unwrap_unchecked()
        };

        // Safety: The ffi call is safe, as we own all pointers.
        unsafe {
            to_result_indirect_in_place(|error, has_module| {
                *error = f(
                    ctx.view().data(),
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
        ctx: &impl ModuleSubsystem,
        name: &CStr,
        ns: &CStr,
        version: Version,
    ) -> Result<bool, Error> {
        // Safety: Is always set.
        let f = unsafe {
            ctx.view()
                .vtable()
                .module_v0
                .set_has_symbol
                .unwrap_unchecked()
        };

        // Safety: The ffi call is safe, as we own all pointers.
        unsafe {
            to_result_indirect_in_place(|error, has_module| {
                *error = f(
                    ctx.view().data(),
                    self.share_to_ffi(),
                    name.as_ptr(),
                    ns.as_ptr(),
                    version.into_ffi(),
                    has_module.as_mut_ptr(),
                );
            })
        }
    }

    /// Adds a status callback to the module set.
    ///
    /// Adds a set of callbacks to report a successful or failed loading of a module. If the set is
    /// able to load all modules contained in the set, the callback will be invoked with the
    /// [`LoadingStatus::Success`] variant, while the error path will invoke the callback with the
    /// [`LoadingStatus::Error`] value immediately after the module fails to load. Since the
    /// `LoadingSet` can be in a partially loaded state, the error path may be invoked immediately,
    /// if it tried to load the module already. If the requested module `module` is not contained
    /// in the `LoadingSet`, this method will return an error.
    pub fn append_callback<T>(
        &self,
        ctx: &impl ModuleSubsystem,
        module: &CStr,
        callback: T,
    ) -> error::Result
    where
        T: FnOnce(LoadingStatus<'_>),
    {
        unsafe extern "C" fn success_callback<T>(
            module: *const bindings::FimoModuleInfo,
            data: *mut core::ffi::c_void,
        ) where
            T: FnOnce(LoadingStatus<'_>),
        {
            // Safety: `data` is a `Box<T>`.
            let func = unsafe {
                alloc::boxed::Box::from_raw_in(data.cast::<T>(), crate::allocator::FimoAllocator)
            };

            // Safety: Is safe.
            let module = unsafe { ModuleInfo::borrow_from_ffi(module) };
            let status = LoadingStatus::Success { info: module };
            func(status);
        }
        unsafe extern "C" fn error_callback<T>(
            export: *const bindings::FimoModuleExport,
            data: *mut core::ffi::c_void,
        ) where
            T: FnOnce(LoadingStatus<'_>),
        {
            // Safety: `data` is a `Box<T>`.
            let func = unsafe {
                alloc::boxed::Box::from_raw_in(data.cast::<T>(), crate::allocator::FimoAllocator)
            };

            // Safety: Is safe.
            let export = unsafe { ModuleExport::borrow_from_ffi(export) };
            let status = LoadingStatus::Error { export };
            func(status);
        }

        let on_success = Some(success_callback::<T> as _);
        let on_error = Some(error_callback::<T> as _);
        let callback = alloc::boxed::Box::try_new_in(callback, crate::allocator::FimoAllocator)
            .map_err(|_err| <Error>::ENOMEM)?;
        let callback = alloc::boxed::Box::into_raw(callback);

        // Safety: Is always set.
        let f = unsafe {
            ctx.view()
                .vtable()
                .module_v0
                .set_append_callback
                .unwrap_unchecked()
        };

        // Safety: FFI call is safe.
        unsafe {
            to_result_indirect(|error| {
                *error = f(
                    ctx.view().data(),
                    self.share_to_ffi(),
                    module.as_ptr(),
                    on_success,
                    on_error,
                    callback.cast(),
                );
            })
        }
    }

    /// Adds a freestanding module to the module set.
    ///
    /// Adds a freestanding module to the set, so that it may be loaded. Trying to include an
    /// invalid module, a module with duplicate exports or duplicate name will result in an
    /// error. Unlike [`append_modules`](Self::append_modules), this function allows for the loading
    /// of dynamic modules, i.e. modules that are created at runtime, like non-native modules,
    /// which may require a runtime to be executed in. To ensure that the binary of the module
    /// calling this function is not unloaded while the new module is instantiated, the new
    /// module inherits a strong reference to the same binary as the caller's module. Note that
    /// the new module is not setup to automatically depend on `module`, but may prevent it from
    /// being unloaded while the set exists.
    ///
    /// # Safety
    ///
    /// `export` must remain valid until the module is unloaded and the set is dropped.
    pub unsafe fn append_freestanding_module(
        &self,
        module: &impl Module,
        export: impl FFITransferable<*const bindings::FimoModuleExport>,
    ) -> error::Result {
        // Safety: Is always set.
        let f = unsafe {
            module
                .context()
                .vtable()
                .module_v0
                .set_append_freestanding_module
                .unwrap_unchecked()
        };

        // Safety: FFI call is safe.
        unsafe {
            to_result_indirect(|error| {
                *error = f(
                    module.context().data(),
                    module.share_to_ffi(),
                    self.share_to_ffi(),
                    export.into_ffi(),
                );
            })
        }
    }

    /// Adds modules to the module set.
    ///
    /// Opens up a module binary to select which modules to load. The binary path `module_path` must
    /// be encoded as `UTF-8`, and point to the binary that contains the modules. The binary path
    /// does not require to contain the file extension. If the path is `Null`, it iterates over the
    /// exported modules of the current binary. Each exported module is then passed to the `filter`,
    /// along with the provided `filter_data`, which can then filter which modules to load. This
    /// function may skip invalid module exports. Trying to include a module with duplicate exports
    /// or duplicate name will result in an error. This function signals an error, if the binary
    /// does not contain the symbols necessary to query the exported modules, but does not return in
    /// an error, if it does not export any modules. The necessary symbols are set-up automatically,
    /// if the binary was linked with the fimo library. In case of an error, no modules are appended
    /// to the set.
    pub fn append_modules<T>(
        &self,
        ctx: &impl ModuleSubsystem,
        module_path: Option<&CStr>,
        mut filter: T,
    ) -> error::Result
    where
        T: FnMut(ModuleExport<'_>) -> LoadingFilterRequest,
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
            let request = (func)(export);
            matches!(request, LoadingFilterRequest::Load)
        }

        let filter_data = core::ptr::from_mut(&mut filter).cast::<core::ffi::c_void>();
        let filter = Some(filter_func::<T> as _);

        let iterator = super::fimo_impl_module_export_iterator;

        // Safety: Is always set.
        let f = unsafe {
            ctx.view()
                .vtable()
                .module_v0
                .set_append_modules
                .unwrap_unchecked()
        };

        // Safety: FFI call is safe.
        unsafe {
            to_result_indirect(|error| {
                *error = f(
                    ctx.view().data(),
                    self.share_to_ffi(),
                    module_path.map_or(core::ptr::null(), |x| x.as_ptr()),
                    filter,
                    filter_data,
                    Some(iterator),
                    iterator as _,
                );
            })
        }
    }
}

// Safety: `FimoModuleLoadingSet` is always `Send + Sync`.
unsafe impl Send for LoadingSet<'_> {}

// Safety: `FimoModuleLoadingSet` is always `Send + Sync`.
unsafe impl Sync for LoadingSet<'_> {}

impl FFISharable<*mut bindings::FimoModuleLoadingSet> for LoadingSet<'_> {
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

impl FFITransferable<*mut bindings::FimoModuleLoadingSet> for LoadingSet<'_> {
    fn into_ffi(self) -> *mut bindings::FimoModuleLoadingSet {
        self.0
    }

    unsafe fn from_ffi(ffi: *mut bindings::FimoModuleLoadingSet) -> Self {
        Self(ffi, PhantomData)
    }
}
