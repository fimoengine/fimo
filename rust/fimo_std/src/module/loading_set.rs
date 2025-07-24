//! Definition of a module loading set.

use core::{ffi::CStr, future::Future, marker::PhantomData};
use std::{mem::MaybeUninit, pin::Pin};

use crate::{
    r#async::{EnqueuedFuture, Fallible},
    bindings,
    context::Handle,
    error::{AnyError, AnyResult},
    handle,
    module::{
        exports::Export,
        info::InfoView,
        instance::{GenericInstance, OpaqueInstanceView},
        symbols::{AssertSharable, Share, StrRef, SymbolInfo},
    },
    utils::{ConstNonNull, OpaqueHandle},
    version::Version,
};

/// Result of the filter operation of a [`LoadingSet`].
#[repr(i32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum FilterRequest {
    Skip,
    Load,
}

/// Status of a module loading operation.
#[derive(Debug)]
pub enum LoadingStatus<'a> {
    Success { info: Pin<&'a InfoView<'a>> },
    Error { export: &'a Export<'a> },
}

handle!(pub handle LoadingSetHandle: Send + Sync + Share);

/// Virtual function table of a [`LoadingSetView`] and [`LoadingSet`].
#[repr(C)]
#[derive(Debug)]
#[allow(clippy::type_complexity)]
pub struct LoadingSetVTable {
    pub acquire: unsafe extern "C" fn(handle: LoadingSetHandle),
    pub release: unsafe extern "C" fn(handle: LoadingSetHandle),
    pub query_module: unsafe extern "C" fn(handle: LoadingSetHandle, module: StrRef<'_>) -> bool,
    pub query_symbol: unsafe extern "C" fn(
        handle: LoadingSetHandle,
        name: StrRef<'_>,
        namespace: StrRef<'_>,
        version: Version<'_>,
    ) -> bool,
    pub add_callback: unsafe extern "C" fn(
        handle: LoadingSetHandle,
        module: StrRef<'_>,
        on_success: unsafe extern "C" fn(
            info: Pin<&InfoView<'_>>,
            handle: Option<OpaqueHandle<dyn Send>>,
        ),
        on_error: for<'export> unsafe extern "C" fn(
            export: &'export Export<'export>,
            handle: Option<OpaqueHandle<dyn Send>>,
        ),
        on_abort: Option<unsafe extern "C" fn(handle: Option<OpaqueHandle<dyn Send>>)>,
        callback_handle: Option<OpaqueHandle<dyn Send>>,
    ) -> AnyResult,
    pub add_module: unsafe extern "C" fn(
        handle: LoadingSetHandle,
        owner: Pin<&OpaqueInstanceView<'_>>,
        export: ConstNonNull<Export<'static>>,
    ) -> AnyResult,
    pub add_modules_from_path: unsafe extern "C" fn(
        handle: LoadingSetHandle,
        path: bindings::FimoUTF8Path,
        filter: unsafe extern "C" fn(
            export: &Export<'_>,
            handle: Option<OpaqueHandle<dyn Send>>,
        ) -> FilterRequest,
        filter_drop: Option<unsafe extern "C" fn(handle: Option<OpaqueHandle<dyn Send>>)>,
        filter_handle: Option<OpaqueHandle<dyn Send>>,
    ) -> AnyResult,
    pub add_modules_from_local: unsafe extern "C" fn(
        handle: LoadingSetHandle,
        filter: unsafe extern "C" fn(
            export: &Export<'_>,
            handle: Option<OpaqueHandle<dyn Send>>,
        ) -> FilterRequest,
        filter_drop: Option<unsafe extern "C" fn(handle: Option<OpaqueHandle<dyn Send>>)>,
        filter_handle: Option<OpaqueHandle<dyn Send>>,
        iterator: unsafe extern "C" fn(
            f: unsafe extern "C" fn(export: &Export<'_>, handle: Option<OpaqueHandle>) -> bool,
            handle: Option<OpaqueHandle>,
        ),
        bin_ptr: OpaqueHandle,
    ) -> AnyResult,
    pub commit: unsafe extern "C" fn(handle: LoadingSetHandle) -> EnqueuedFuture<Fallible<()>>,
    _private: PhantomData<()>,
}

impl LoadingSetVTable {
    cfg_internal! {
        /// Constructs a new `LoadingSetVTable`.
        ///
        /// # Unstable
        ///
        /// **Note**: This is an [unstable API][unstable]. The public API of this type may break
        /// with any semver compatible release. See
        /// [the documentation on unstable features][unstable] for details.
        ///
        /// [unstable]: crate#unstable-features
        #[allow(clippy::too_many_arguments, clippy::type_complexity)]
        pub const fn new(
            acquire: unsafe extern "C" fn(handle: LoadingSetHandle),
            release: unsafe extern "C" fn(handle: LoadingSetHandle),
            query_module: unsafe extern "C" fn(handle: LoadingSetHandle, module: StrRef<'_>) -> bool,
            query_symbol: unsafe extern "C" fn(
                handle: LoadingSetHandle,
                name: StrRef<'_>,
                namespace: StrRef<'_>,
                version: Version<'_>,
            ) -> bool,
            add_callback: unsafe extern "C" fn(
                handle: LoadingSetHandle,
                module: StrRef<'_>,
                on_success: unsafe extern "C" fn(
                    info: Pin<&InfoView<'_>>,
                    handle: Option<OpaqueHandle<dyn Send>>,
                ),
                on_error: for<'export> unsafe extern "C" fn(
                    export: &'export Export<'export>,
                    handle: Option<OpaqueHandle<dyn Send>>,
                ),
                on_abort: Option<unsafe extern "C" fn(handle: Option<OpaqueHandle<dyn Send>>)>,
                callback_handle: Option<OpaqueHandle<dyn Send>>,
            ) -> AnyResult,
            add_module: unsafe extern "C" fn(
                handle: LoadingSetHandle,
                owner: Pin<&OpaqueInstanceView<'_>>,
                export: ConstNonNull<Export<'static>>,
            ) -> AnyResult,
            add_modules_from_path: unsafe extern "C" fn(
                handle: LoadingSetHandle,
                path: bindings::FimoUTF8Path,
                filter: unsafe extern "C" fn(
                    export: &Export<'_>,
                    handle: Option<OpaqueHandle<dyn Send>>,
                ) -> FilterRequest,
                filter_drop: Option<unsafe extern "C" fn(handle: Option<OpaqueHandle<dyn Send>>)>,
                filter_handle: Option<OpaqueHandle<dyn Send>>,
            ) -> AnyResult,
            add_modules_from_local: unsafe extern "C" fn(
                handle: LoadingSetHandle,
                filter: unsafe extern "C" fn(
                    export: &Export<'_>,
                    handle: Option<OpaqueHandle<dyn Send>>,
                ) -> FilterRequest,
                filter_drop: Option<unsafe extern "C" fn(handle: Option<OpaqueHandle<dyn Send>>)>,
                filter_handle: Option<OpaqueHandle<dyn Send>>,
                iterator: unsafe extern "C" fn(
                    f: unsafe extern "C" fn(export: &Export<'_>, handle: Option<OpaqueHandle>) -> bool,
                    handle: Option<OpaqueHandle>,
                ),
                bin_ptr: OpaqueHandle,
            ) -> AnyResult,
            commit: unsafe extern "C" fn(handle: LoadingSetHandle) -> EnqueuedFuture<Fallible<()>>,
        ) -> Self {
            Self {
                acquire,
                release,
                query_module,
                query_symbol,
                add_callback,
                add_module,
                add_modules_from_path,
                add_modules_from_local,
                commit,
                _private: PhantomData,
            }
        }
    }
}

/// View of a loading set.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct LoadingSetView<'a> {
    pub handle: LoadingSetHandle,
    pub vtable: &'a AssertSharable<LoadingSetVTable>,
    _private: PhantomData<()>,
}

sa::assert_impl_all!(LoadingSetView<'_>: Send, Sync);
sa::assert_impl_all!(LoadingSetView<'static>: Share);

impl LoadingSetView<'_> {
    /// Promotes the view to an owned set.
    pub fn to_loading_set(&self) -> LoadingSet {
        unsafe {
            let f = self.vtable.acquire;
            f(self.handle);
            LoadingSet(LoadingSetView {
                handle: self.handle,
                vtable: &*(self.vtable as *const _),
                _private: PhantomData,
            })
        }
    }

    /// Checks whether the set contains a specific module.
    pub fn query_module(&self, module: &CStr) -> bool {
        unsafe {
            let f = self.vtable.query_module;
            f(self.handle, StrRef::new(module))
        }
    }

    /// Checks whether the set contains a specific symbol.
    pub fn query_symbol<T: SymbolInfo>(&self) -> bool {
        self.query_symbol_raw(T::NAME, T::NAMESPACE, T::VERSION)
    }

    /// Checks whether the set contains a specific symbol.
    pub fn query_symbol_raw(&self, name: &CStr, namespace: &CStr, version: Version<'_>) -> bool {
        unsafe {
            let f = self.vtable.query_symbol;
            f(
                self.handle,
                StrRef::new(name),
                StrRef::new(namespace),
                version,
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
    pub fn add_callback<F>(&self, module: &CStr, callback: F) -> Result<(), AnyError>
    where
        F: FnOnce(LoadingStatus<'_>) + Send + 'static,
    {
        unsafe extern "C" fn on_success<F>(
            info: Pin<&InfoView<'_>>,
            handle: Option<OpaqueHandle<dyn Send>>,
        ) where
            F: FnOnce(LoadingStatus<'_>),
        {
            unsafe {
                let handle = handle.unwrap_unchecked().as_ptr::<F>();
                let f = Box::from_raw(handle);

                let status = LoadingStatus::Success { info };
                f(status);
            }
        }
        unsafe extern "C" fn on_error<'a, F>(
            export: &'a Export<'a>,
            handle: Option<OpaqueHandle<dyn Send>>,
        ) where
            F: FnOnce(LoadingStatus<'_>),
        {
            unsafe {
                let handle = handle.unwrap_unchecked().as_ptr::<F>();
                let f = Box::from_raw(handle);

                let status = LoadingStatus::Error { export };
                f(status);
            }
        }
        unsafe extern "C" fn on_abort<F>(handle: Option<OpaqueHandle<dyn Send>>)
        where
            F: FnOnce(LoadingStatus<'_>),
        {
            unsafe {
                let handle = handle.unwrap_unchecked().as_ptr::<F>();
                _ = Box::from_raw(handle);
            }
        }

        unsafe {
            let callback_handle = Box::try_new(callback).map_err(<AnyError>::new)?;
            let callback_handle = OpaqueHandle::new(Box::into_raw(callback_handle));

            let f = self.vtable.add_callback;
            f(
                self.handle,
                StrRef::new(module),
                on_success::<F>,
                on_error::<F>,
                Some(on_abort::<F> as _),
                callback_handle,
            )
            .into_result()
        }
    }

    /// Adds a module to the set.
    ///
    /// Adds a module to the set, so that it may be loaded by a future call to
    /// [`commit`](LoadingSetView::commit). Trying to include an invalid module, a module with
    /// duplicate exports or duplicate name will result in an error. Unlike
    /// [`add_modules_from_path`](LoadingSetView::add_modules_from_path), this function allows for
    /// the loading of dynamic modules, i.e. modules that are created at runtime, like
    /// non-native modules, which may require a runtime to be executed in. The new module
    /// inherits a strong reference to the same binary as the caller's module.
    ///
    /// Note that the new module is not setup to automatically depend on the owner, but may prevent
    /// it from being unloaded while the set exists.
    ///
    /// # Safety
    ///
    /// The export must outlive the set.
    pub unsafe fn add_module(
        &self,
        owner: impl GenericInstance,
        export: &Export<'_>,
    ) -> Result<(), AnyError> {
        unsafe {
            let f = self.vtable.add_module;
            f(
                self.handle,
                owner.to_opaque_instance_view(),
                ConstNonNull::from(export).cast(),
            )
            .into_result()
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
    pub unsafe fn add_modules_from_path<F>(&self, path: &str, filter: F) -> Result<(), AnyError>
    where
        F: FnMut(&Export<'_>) -> FilterRequest + Send,
    {
        unsafe extern "C" fn filter_wrapper<F>(
            export: &Export<'_>,
            handle: Option<OpaqueHandle<dyn Send>>,
        ) -> FilterRequest
        where
            F: FnMut(&Export<'_>) -> FilterRequest + Send,
        {
            unsafe {
                let f = &mut *handle.unwrap_unchecked().as_ptr::<F>();
                f(export)
            }
        }
        unsafe extern "C" fn filter_drop<F>(handle: Option<OpaqueHandle<dyn Send>>)
        where
            F: FnMut(&Export<'_>) -> FilterRequest + Send,
        {
            unsafe {
                let handle = handle.unwrap_unchecked().as_ptr::<F>();
                let _ = Box::from_raw(handle);
            }
        }

        unsafe {
            let filter_handle = Box::try_new(filter).map_err(<AnyError>::new)?;
            let filter_handle = OpaqueHandle::new(Box::into_raw(filter_handle));

            let f = self.vtable.add_modules_from_path;
            f(
                self.handle,
                bindings::FimoUTF8Path {
                    path: path.as_ptr().cast(),
                    length: path.len(),
                },
                filter_wrapper::<F>,
                Some(filter_drop::<F> as _),
                filter_handle,
            )
            .into_result()
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
    pub fn add_modules_from_local<F>(&self, filter: F) -> Result<(), AnyError>
    where
        F: FnMut(&Export<'_>) -> FilterRequest + Send,
    {
        unsafe extern "C" fn filter_wrapper<F>(
            export: &Export<'_>,
            handle: Option<OpaqueHandle<dyn Send>>,
        ) -> FilterRequest
        where
            F: FnMut(&Export<'_>) -> FilterRequest + Send,
        {
            unsafe {
                let f = &mut *handle.unwrap_unchecked().as_ptr::<F>();
                f(export)
            }
        }
        unsafe extern "C" fn filter_drop<F>(handle: Option<OpaqueHandle<dyn Send>>)
        where
            F: FnMut(&Export<'_>) -> FilterRequest + Send,
        {
            unsafe {
                let handle = handle.unwrap_unchecked().as_ptr::<F>();
                let _ = Box::from_raw(handle);
            }
        }

        unsafe {
            let filter_handle = Box::try_new(filter).map_err(<AnyError>::new)?;
            let filter_handle = OpaqueHandle::new(Box::into_raw(filter_handle));

            let f = self.vtable.add_modules_from_local;
            f(
                self.handle,
                filter_wrapper::<F>,
                Some(filter_drop::<F> as _),
                filter_handle,
                super::fimo_impl_module_export_iterator,
                OpaqueHandle::new_unchecked(
                    (super::fimo_impl_module_export_iterator as *const ()).cast_mut(),
                ),
            )
            .into_result()
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
    pub fn commit(&self) -> impl Future<Output = Result<(), AnyError<dyn Send + Sync + Share>>> {
        unsafe {
            let f = self.vtable.commit;
            let fut = f(self.handle);
            async move { fut.await.unwrap() }
        }
    }
}

// Safety: `FimoModuleLoadingSet` is always `Send + Sync`.
unsafe impl Send for LoadingSetView<'_> {}

// Safety: `FimoModuleLoadingSet` is always `Send + Sync`.
unsafe impl Sync for LoadingSetView<'_> {}

/// A loading set.
#[repr(transparent)]
pub struct LoadingSet(LoadingSetView<'static>);

sa::assert_impl_all!(LoadingSet: Send, Sync, Share);

impl LoadingSet {
    /// Constructs a new loading set.
    pub fn new() -> Result<Self, AnyError> {
        unsafe {
            let mut out = MaybeUninit::uninit();
            let handle = Handle::get_handle();
            let f = handle.module_v0.new_loading_set;
            f(&mut out).into_result()?;
            Ok(out.assume_init())
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
        unsafe {
            let f = self.view().vtable.release;
            f(self.view().handle);
        }
    }
}
