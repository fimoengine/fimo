use core::{ffi::CStr, future::Future, marker::PhantomData};

use super::{Module, ModuleExport, ModuleInfo, ModuleInfoView, NamespaceItem, SymbolItem};
use crate::{
    bindings,
    context::ContextView,
    error::{to_result_indirect, to_result_indirect_in_place, Error},
    ffi::{FFISharable, FFITransferable, Viewable},
    r#async::{EnqueuedFuture, Fallible},
    version::Version,
};

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

/// View of a loading set.
#[repr(transparent)]
#[derive(Debug, Clone, Copy)]
pub struct LoadingSetView<'a>(bindings::FimoModuleLoadingSet, PhantomData<&'a mut ()>);

impl LoadingSetView<'_> {
    pub(crate) fn data(&self) -> *mut std::ffi::c_void {
        self.0.data
    }

    pub(crate) fn vtable(&self) -> &bindings::FimoModuleLoadingSetVTable {
        // Safety: Is always valid.
        unsafe { &*self.0.vtable.cast() }
    }

    /// Promotes the view to an owned set.
    pub fn to_loading_set(&self) -> LoadingSet {
        unsafe {
            let f = self.vtable().acquire.unwrap_unchecked();
            f(self.data());
            LoadingSet(LoadingSetView(self.0, PhantomData))
        }
    }

    /// Checks whether the set contains a specific module.
    pub fn query_module(&self, module: &CStr) -> bool {
        unsafe {
            let f = self.vtable().query_module.unwrap_unchecked();
            f(self.data(), module.as_ptr())
        }
    }

    /// Checks whether the set contains a specific symbol.
    pub fn query_symbol<T: SymbolItem>(&self) -> bool {
        self.query_symbol_raw(T::NAME, T::Namespace::NAME, T::VERSION)
    }

    /// Checks whether the set contains a specific symbol.
    pub fn query_symbol_raw(&self, name: &CStr, namespace: &CStr, version: Version) -> bool {
        unsafe {
            let f = self.vtable().query_symbol.unwrap_unchecked();
            f(
                self.data(),
                name.as_ptr(),
                namespace.as_ptr(),
                version.into_ffi(),
            )
        }
    }

    /// Adds a status callback to the set.
    ///
    /// Adds a callback to report a successful or failed loading of a module. The success path
    /// wil be called if the set was able to load all requested modules, whereas the error path
    /// will be called immediately after the failed loading of the module. Since the module set
    /// can be in a partially loaded state at the time of calling this function, the error path
    /// may be invoked immediately. If the requested module does not exist, the function will return
    /// an error.
    pub fn add_callback<F>(&self, module: &CStr, callback: F) -> Result<(), Error>
    where
        F: FnOnce(LoadingStatus<'_>) + Send + 'static,
    {
        unsafe extern "C" fn success_callback<F>(
            module: *const bindings::FimoModuleInfo,
            data: *mut core::ffi::c_void,
        ) where
            F: FnOnce(LoadingStatus<'_>),
        {
            // Safety: `data` is a `Box<F>`.
            let func = unsafe { Box::from_raw(data.cast::<F>()) };

            // Safety: Is safe.
            let module = unsafe { ModuleInfo::borrow_from_ffi(module) };
            let status = LoadingStatus::Success { info: module };
            func(status);
        }
        unsafe extern "C" fn error_callback<F>(
            export: *const bindings::FimoModuleExport,
            data: *mut core::ffi::c_void,
        ) where
            F: FnOnce(LoadingStatus<'_>),
        {
            // Safety: `data` is a `Box<F>`.
            let func = unsafe { Box::from_raw(data.cast::<F>()) };

            // Safety: Is safe.
            let export = unsafe { ModuleExport::borrow_from_ffi(export) };
            let status = LoadingStatus::Error { export };
            func(status);
        }
        unsafe extern "C" fn drop_callback<F>(data: *mut core::ffi::c_void)
        where
            F: FnOnce(LoadingStatus<'_>),
        {
            // Safety: `data` is a `Box<F>`.
            let _ = unsafe { Box::from_raw(data.cast::<F>()) };
        }

        let on_success = Some(success_callback::<F> as _);
        let on_error = Some(error_callback::<F> as _);
        let on_abort = Some(drop_callback::<F> as _);
        let callback = Box::try_new(callback).map_err(<Error>::new)?;
        let callback = Box::into_raw(callback);

        // Safety:
        unsafe {
            let f = self.vtable().add_callback.unwrap_unchecked();
            to_result_indirect(|error| {
                *error = f(
                    self.data(),
                    module.as_ptr(),
                    on_success,
                    on_error,
                    on_abort,
                    callback.cast(),
                );
            })
        }
    }

    /// Adds a module to the set.
    ///
    /// Adds a module to the set, so that it may be loaded by a future call to [`commit`]. Trying to
    /// include an invalid module, a module with duplicate exports or duplicate name will result in
    /// an error. Unlike [`add_modules_from_path`], this function allows for the loading of dynamic
    /// modules, i.e. modules that are created at runtime, like non-native modules, which may
    /// require a runtime to be executed in. The new module inherits a strong reference to the same
    /// binary as the caller's module.
    ///
    /// Note that the new module is not setup to automatically depend on the owner, but may prevent
    /// it from being unloaded while the set exists.
    ///
    /// # Safety
    ///
    /// The export must outlive the set.
    pub unsafe fn add_module(
        &self,
        owner: &impl Module,
        export: impl FFITransferable<*const bindings::FimoModuleExport>,
    ) -> Result<(), Error> {
        // Safety:
        unsafe {
            let f = self.vtable().add_module.unwrap_unchecked();
            to_result_indirect(|error| {
                *error = f(self.data(), owner.share_to_ffi(), export.into_ffi());
            })
        }
    }

    /// Adds modules to the set.
    ///
    /// Opens up a module binary to select which modules to load. If the path points to a file, the
    /// function will try to load the file as a binary, whereas, if it points to a directory, it
    /// will try to load a file named `module.fimo_module` contained in the directory. Each exported
    /// module is then passed to the filter, along with the provided data, which can then filter
    /// which modules to load. This function may skip invalid module exports. Trying to include a
    /// module with duplicate exports or duplicate name will result in an error. This function
    /// signals an error, if the binary does not contain the symbols necessary to query the exported
    /// modules, but does not return an error, if it does not export any modules. The necessary
    /// symbols are set up automatically, if the binary was linked with the fimo library. In case of
    /// an error, no modules are appended to the set.
    ///
    /// # Safety
    ///
    /// Loading a library may execute arbitrary code.
    pub unsafe fn add_modules_from_path<F>(&self, path: &str, filter: F) -> Result<(), Error>
    where
        F: FnMut(ModuleExport<'_>) -> LoadingFilterRequest,
    {
        unsafe extern "C" fn filter_func<F>(
            export: *const bindings::FimoModuleExport,
            data: *mut core::ffi::c_void,
        ) -> bool
        where
            F: FnMut(ModuleExport<'_>) -> LoadingFilterRequest,
        {
            // Safety: `data` is a mutable reference to `F`.
            let func = unsafe { &mut *data.cast::<F>() };

            // Safety: Is safe.
            let export = unsafe { ModuleExport::borrow_from_ffi(export) };
            let request = (func)(export);
            matches!(request, LoadingFilterRequest::Load)
        }
        unsafe extern "C" fn filter_drop<F>(data: *mut core::ffi::c_void)
        where
            F: FnMut(ModuleExport<'_>) -> LoadingFilterRequest,
        {
            // Safety: `data` is a `Box<F>`.
            let _ = unsafe { Box::from_raw(data.cast::<F>()) };
        }

        let filter_fn = Some(filter_func::<F> as _);
        let filter_drop_fn = Some(filter_drop::<F> as _);
        let filter = Box::try_new(filter).map_err(<Error>::new)?;
        let filter = Box::into_raw(filter);

        // Safety:
        unsafe {
            let f = self.vtable().add_modules_from_path.unwrap_unchecked();
            to_result_indirect(|error| {
                *error = f(
                    self.data(),
                    bindings::FimoUTF8Path {
                        path: path.as_ptr().cast(),
                        length: path.len(),
                    },
                    filter_fn,
                    filter_drop_fn,
                    filter.cast(),
                );
            })
        }
    }

    /// Adds modules to the set.
    ///
    /// Iterates over the exported modules of the current binary. Each exported module is then
    /// passed to the filter, along with the provided data, which can then filter which modules to
    /// load. This function may skip invalid module exports. Trying to include a module with
    /// duplicate exports or duplicate name will result in an error. This function signals an error,
    /// if the binary does not contain the symbols necessary to query the exported modules, but does
    /// not return an error, if it does not export any modules. The necessary symbols are set up
    /// automatically, if the binary was linked with the fimo library. In case of an error, no
    /// modules are appended to the set.
    pub fn add_modules_from_local<F>(&self, filter: F) -> Result<(), Error>
    where
        F: FnMut(ModuleExport<'_>) -> LoadingFilterRequest,
    {
        unsafe extern "C" fn filter_func<F>(
            export: *const bindings::FimoModuleExport,
            data: *mut core::ffi::c_void,
        ) -> bool
        where
            F: FnMut(ModuleExport<'_>) -> LoadingFilterRequest,
        {
            // Safety: `data` is a mutable reference to `F`.
            let func = unsafe { &mut *data.cast::<F>() };

            // Safety: Is safe.
            let export = unsafe { ModuleExport::borrow_from_ffi(export) };
            let request = (func)(export);
            matches!(request, LoadingFilterRequest::Load)
        }
        unsafe extern "C" fn filter_drop<F>(data: *mut core::ffi::c_void)
        where
            F: FnMut(ModuleExport<'_>) -> LoadingFilterRequest,
        {
            // Safety: `data` is a `Box<F>`.
            let _ = unsafe { Box::from_raw(data.cast::<F>()) };
        }

        let filter_fn = Some(filter_func::<F> as _);
        let filter_drop_fn = Some(filter_drop::<F> as _);
        let filter = Box::try_new(filter).map_err(<Error>::new)?;
        let filter = Box::into_raw(filter);

        let iterator = super::fimo_impl_module_export_iterator;

        // Safety:
        unsafe {
            let f = self.vtable().add_modules_from_local.unwrap_unchecked();
            to_result_indirect(|error| {
                *error = f(
                    self.data(),
                    filter_fn,
                    filter_drop_fn,
                    filter.cast(),
                    Some(iterator),
                    iterator as _,
                );
            })
        }
    }

    /// Loads the modules contained in the set.
    ///
    /// If the returned future is successful, the contained modules and their resources are made
    /// available to the remaining modules. Some conditions may hinder the loading of some module,
    /// like missing dependencies, duplicates, and other loading errors. In those cases, the modules
    /// will be skipped without errors.
    ///
    /// It is possible to submit multiple concurrent commit requests, even from the same loading
    /// set. In that case, the requests will be handled atomically, in an unspecified order.
    pub fn commit(&self) -> impl Future<Output = Result<(), Error<dyn Send + Sync>>> {
        // Safety:
        unsafe {
            let f = self.vtable().commit.unwrap_unchecked();
            let fut = f(self.data());
            let fut = std::mem::transmute::<
                bindings::FimoModuleLoadingSetCommitFuture,
                EnqueuedFuture<Fallible<()>>,
            >(fut);
            async move { fut.await.unwrap() }
        }
    }
}

// Safety: `FimoModuleLoadingSet` is always `Send + Sync`.
unsafe impl Send for LoadingSetView<'_> {}

// Safety: `FimoModuleLoadingSet` is always `Send + Sync`.
unsafe impl Sync for LoadingSetView<'_> {}

impl FFISharable<bindings::FimoModuleLoadingSet> for LoadingSetView<'_> {
    type BorrowedView<'a> = LoadingSetView<'a>;

    fn share_to_ffi(&self) -> bindings::FimoModuleLoadingSet {
        self.0
    }

    unsafe fn borrow_from_ffi<'a>(ffi: bindings::FimoModuleLoadingSet) -> Self::BorrowedView<'a> {
        LoadingSetView(ffi, PhantomData)
    }
}

impl FFITransferable<bindings::FimoModuleLoadingSet> for LoadingSetView<'_> {
    fn into_ffi(self) -> bindings::FimoModuleLoadingSet {
        self.0
    }

    unsafe fn from_ffi(ffi: bindings::FimoModuleLoadingSet) -> Self {
        Self(ffi, PhantomData)
    }
}

/// A loading set.
#[repr(transparent)]
pub struct LoadingSet(LoadingSetView<'static>);

impl LoadingSet {
    /// Constructs a new loading set.
    pub fn new<'a, T: Viewable<ContextView<'a>>>(ctx: &T) -> Result<Self, Error> {
        unsafe {
            let f = ctx.view().vtable().module_v0.set_new.unwrap_unchecked();
            let set = to_result_indirect_in_place(|error, set| {
                *error = f(ctx.view().data(), set.as_mut_ptr());
            })?;
            Ok(Self(LoadingSetView::from_ffi(set)))
        }
    }

    /// Returns a view to the loading set.
    pub fn view(&self) -> LoadingSetView<'_> {
        self.0
    }
}

impl Clone for LoadingSet {
    fn clone(&self) -> Self {
        self.view().to_loading_set()
    }
}

impl Drop for LoadingSet {
    fn drop(&mut self) {
        // Safety:
        unsafe {
            let f = self.view().vtable().release.unwrap_unchecked();
            f(self.view().data());
        }
    }
}

impl FFISharable<bindings::FimoModuleLoadingSet> for LoadingSet {
    type BorrowedView<'a> = LoadingSetView<'a>;

    fn share_to_ffi(&self) -> bindings::FimoModuleLoadingSet {
        self.view().share_to_ffi()
    }

    unsafe fn borrow_from_ffi<'a>(ffi: bindings::FimoModuleLoadingSet) -> Self::BorrowedView<'a> {
        // Safety:
        unsafe { LoadingSetView::borrow_from_ffi(ffi) }
    }
}

impl FFITransferable<bindings::FimoModuleLoadingSet> for LoadingSet {
    fn into_ffi(self) -> bindings::FimoModuleLoadingSet {
        self.view().into_ffi()
    }

    unsafe fn from_ffi(ffi: bindings::FimoModuleLoadingSet) -> Self {
        // Safety:
        unsafe { Self(LoadingSetView::from_ffi(ffi)) }
    }
}
