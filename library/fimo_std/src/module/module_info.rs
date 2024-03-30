use core::{ffi::CStr, mem::ManuallyDrop, ops::Deref};

use crate::{
    bindings,
    context::{Context, ContextView},
    error::{self, to_result, to_result_indirect, to_result_indirect_in_place, Error},
    ffi::{FFISharable, FFITransferable},
    version::Version,
};

use super::{ModuleBackend, ModuleBackendGuard, Symbol};

/// Info of a loaded module.
#[repr(transparent)]
pub struct ModuleInfo<'a>(&'a bindings::FimoModuleInfo);

impl ModuleInfo<'_> {
    /// Searches for a module by it's name.
    pub fn find_by_name<'a>(
        guard: &'a ModuleBackendGuard<'_>,
        name: &CStr,
    ) -> Result<ModuleInfo<'a>, Error> {
        // Safety: Either we get an error, or we initialize the module.
        let module = unsafe {
            to_result_indirect_in_place(|error, module| {
                *error = bindings::fimo_module_find_by_name(
                    guard.share_to_ffi(),
                    name.as_ptr(),
                    module.as_mut_ptr(),
                );
            })
        }?;

        // Safety: We own the module info.
        unsafe { Ok(ModuleInfo::from_ffi(module)) }
    }

    /// Searches for a module by a symbol it exports.
    pub fn find_by_symbol<'a>(
        guard: &'a ModuleBackendGuard<'_>,
        name: &CStr,
        namespace: &CStr,
        version: Version,
    ) -> Result<ModuleInfo<'a>, Error> {
        // Safety: Either we get an error, or we initialize the module.
        let module = unsafe {
            to_result_indirect_in_place(|error, module| {
                *error = bindings::fimo_module_find_by_symbol(
                    guard.share_to_ffi(),
                    name.as_ptr(),
                    namespace.as_ptr(),
                    version.into_ffi(),
                    module.as_mut_ptr(),
                );
            })
        }?;

        // Safety: We own the module info.
        unsafe { Ok(ModuleInfo::from_ffi(module)) }
    }

    /// Unloads the module.
    ///
    /// If successful, this function unloads the module and all modules that have it as a
    /// dependency. To succeed, the module must have no dependencies left.
    pub fn unload(self, guard: &mut ModuleBackendGuard<'_>) -> error::Result {
        // Safety: The ffi call is safe.
        to_result_indirect(|error| unsafe {
            *error = bindings::fimo_module_unload(guard.share_to_ffi(), self.into_ffi());
        })
    }
}

impl ModuleInfo<'_> {
    /// Unique module name.
    pub fn name(&self) -> &CStr {
        // Safety: The name is a valid string.
        unsafe { CStr::from_ptr(self.0.name) }
    }

    /// Module description.
    pub fn description(&self) -> &CStr {
        // Safety: The description is a valid string.
        unsafe { CStr::from_ptr(self.0.description) }
    }

    /// Module author.
    pub fn author(&self) -> &CStr {
        // Safety: The author is a valid string.
        unsafe { CStr::from_ptr(self.0.author) }
    }

    /// Module license.
    pub fn license(&self) -> &CStr {
        // Safety: The author is a valid string.
        unsafe { CStr::from_ptr(self.0.license) }
    }

    /// Module description.
    pub fn module_path(&self) -> &CStr {
        // Safety: The module path is a valid string.
        unsafe { CStr::from_ptr(self.0.module_path) }
    }
}

// Safety: `FimoModuleInfo` must be `Send + Sync`.
unsafe impl Send for ModuleInfo<'_> {}

// Safety: `FimoModuleInfo` must be `Send + Sync`.
unsafe impl Sync for ModuleInfo<'_> {}

impl PartialEq for ModuleInfo<'_> {
    fn eq(&self, other: &Self) -> bool {
        core::ptr::eq(self.0, other.0)
    }
}

impl Eq for ModuleInfo<'_> {}

impl core::fmt::Debug for ModuleInfo<'_> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("ModuleInfo")
            .field("name", &self.name())
            .field("description", &self.description())
            .field("author", &self.author())
            .field("license", &self.license())
            .field("module_path", &self.module_path())
            .finish_non_exhaustive()
    }
}

impl core::fmt::Display for ModuleInfo<'_> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "{} ({})",
            self.name().to_string_lossy(),
            self.author().to_string_lossy()
        )
    }
}

impl crate::ffi::FFISharable<*const bindings::FimoModuleInfo> for ModuleInfo<'_> {
    type BorrowedView<'a> = ModuleInfo<'a>;

    fn share_to_ffi(&self) -> *const bindings::FimoModuleInfo {
        self.0
    }

    unsafe fn borrow_from_ffi<'a>(ffi: *const bindings::FimoModuleInfo) -> Self::BorrowedView<'a> {
        // Safety: `ffi` can not be null.
        unsafe {
            debug_assert_eq!(
                (*ffi).type_,
                bindings::FimoStructType::FIMO_STRUCT_TYPE_MODULE_INFO
            );
            ModuleInfo(&*ffi)
        }
    }
}

impl crate::ffi::FFITransferable<*const bindings::FimoModuleInfo> for ModuleInfo<'_> {
    fn into_ffi(self) -> *const bindings::FimoModuleInfo {
        self.0
    }

    unsafe fn from_ffi(ffi: *const bindings::FimoModuleInfo) -> Self {
        // Safety: `ffi` can not be null.
        unsafe {
            debug_assert_eq!(
                (*ffi).type_,
                bindings::FimoStructType::FIMO_STRUCT_TYPE_MODULE_INFO
            );
            Self(&*ffi)
        }
    }
}

impl<'a> From<&'a bindings::FimoModuleInfo> for ModuleInfo<'a> {
    fn from(value: &'a bindings::FimoModuleInfo) -> Self {
        debug_assert_eq!(
            value.type_,
            bindings::FimoStructType::FIMO_STRUCT_TYPE_MODULE_INFO
        );

        Self(value)
    }
}

impl<'a> From<ModuleInfo<'a>> for &'a bindings::FimoModuleInfo {
    fn from(value: ModuleInfo<'a>) -> Self {
        value.0
    }
}

/// Type of dependency between a module and a namespace.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum DependencyType {
    StaticDependency,
    DynamicDependency,
    NoDependency,
}

/// Shared functionality of all modules.
///
/// A module is self-contained, and may not be passed to other modules.
/// An instance of [`Module`] is valid for as long as the owning module
/// remains loaded. Modules mut not leak any resources outside it's own
/// module, ensuring that they are destroyed upon module unloading.
///
/// # Safety
///
/// The implementor must ensure that the associated table types are compatible
/// with the ones the module expects.
pub unsafe trait Module:
    Send
    + Sync
    + for<'a> crate::ffi::FFISharable<
        *const bindings::FimoModule,
        BorrowedView<'a> = OpaqueModule<'a>,
    >
{
    /// Type of the parameter table.
    type Parameters: Send + Sync;

    /// Type of the resource table.
    type Resources: Send + Sync;

    /// Type of the import table.
    type Imports: Send + Sync;

    /// Type of the export table.
    type Exports: Send + Sync;

    /// Type of the associated module data.
    type Data: Send + Sync;

    /// Fetches the parameter table of the module.
    fn parameters(&self) -> &Self::Parameters;

    /// Fetches the resource path table of the module.
    fn resources(&self) -> &Self::Resources;

    /// Fetches the symbol import table of the module.
    fn imports(&self) -> &Self::Imports;

    /// Fetches the symbol export table of the module.
    fn exports(&self) -> &Self::Exports;

    /// Fetches the module info.
    fn module_info(&self) -> ModuleInfo<'_>;

    /// Fetches the context of the module.
    fn context(&self) -> ContextView<'_>;

    /// Fetches the data of the module.
    fn data(&self) -> &Self::Data;

    /// Checks if a module includes a namespace.
    ///
    /// Checks if the module specified that it includes the namespace `namespace`. In that case, the
    /// module is allowed access to the symbols in the namespace.
    fn has_namespace_dependency(&self, namespace: &CStr) -> Result<DependencyType, Error>;

    /// Includes a namespace by the module.
    ///
    /// Once included, the module gains access to the symbols of its dependencies that are exposed
    /// in said namespace. A namespace can not be included multiple times.
    fn include_namespace(
        &mut self,
        guard: &mut ModuleBackendGuard<'_>,
        namespace: &CStr,
    ) -> error::Result;

    /// Removes a namespace from the module.
    ///
    /// Once excluded, the caller guarantees to relinquish access to the symbols contained in said
    /// namespace. It is only possible to exclude namespaces that were manually added, whereas
    /// static namespace includes remain valid until the module is unloaded. Trying to exclude a
    /// namespace that is currently in use by the module will result in an error.
    fn exclude_namespace(
        &mut self,
        guard: &mut ModuleBackendGuard<'_>,
        namespace: &CStr,
    ) -> error::Result;

    /// Checks if a module depends on another module.
    ///
    /// Checks if `module` is a dependency of the current instance. In that case the instance is
    /// allowed to access the symbols exported by `module`.
    fn has_dependency(&self, module: &ModuleInfo<'_>) -> Result<DependencyType, Error>;

    /// Acquires another module as a dependency.
    ///
    /// After acquiring a module as a dependency, the module is allowed access to the symbols and
    /// protected parameters of said dependency. Trying to acquire a dependency to a module that is
    /// already a dependency, or to a module that would result in a circular dependency will result
    /// in an error.
    fn acquire_dependency<'ctx>(
        &mut self,
        guard: &mut ModuleBackendGuard<'ctx>,
        dependency: &ModuleInfo<'_>,
    ) -> Result<ModuleInfo<'ctx>, Error>;

    /// Removes a module as a dependency.
    ///
    /// By removing a module as a dependency, the caller ensures that it does not own any references
    /// to resources originating from the former dependency, and allows for the unloading of the
    /// module. A module can only relinquish dependencies to modules that were acquired dynamically,
    /// as static dependencies remain valid until the module is unloaded.
    fn remove_dependency(
        &mut self,
        guard: &mut ModuleBackendGuard<'_>,
        dependency: ModuleInfo<'_>,
    ) -> error::Result;

    /// Loads a symbol from the module backend.
    ///
    /// The caller can query the backend for a symbol of a loaded module. This is useful for loading
    /// optional symbols, or for loading symbols after the creation of a module. The symbol, if it
    /// exists, is returned, and can be used until the module relinquishes the dependency to the
    /// module that exported the symbol. This function fails, if the module containing the symbol is
    /// not a dependency of the module.
    ///
    /// # Safety
    ///
    /// Users of this API must specify the correct type of the symbol.
    unsafe fn load_symbol<T>(
        &self,
        name: &CStr,
        namespace: &CStr,
        version: Version,
    ) -> Result<Symbol<'_, T>, Error>;
}

/// Reference to an unknown module.
#[repr(transparent)]
#[derive(Debug, Clone, Copy)]
pub struct OpaqueModule<'a>(&'a bindings::FimoModule);

// Safety: `FimoModule` must be `Send + Sync`.
unsafe impl Send for OpaqueModule<'_> {}

// Safety: `FimoModule` must be `Send + Sync`.
unsafe impl Sync for OpaqueModule<'_> {}

// Safety: `()` is compatible with any type.
unsafe impl Module for OpaqueModule<'_> {
    type Parameters = ();
    type Resources = ();
    type Imports = ();
    type Exports = ();
    type Data = ();

    fn parameters(&self) -> &() {
        // Safety: Is safe due to `()` being a ZST.
        unsafe { self.0.parameters.cast::<()>().as_ref().unwrap_or(&()) }
    }

    fn resources(&self) -> &() {
        // Safety: Is safe due to `()` being a ZST.
        unsafe { self.0.resources.cast::<()>().as_ref().unwrap_or(&()) }
    }

    fn imports(&self) -> &() {
        // Safety: Is safe due to `()` being a ZST.
        unsafe { self.0.imports.cast::<()>().as_ref().unwrap_or(&()) }
    }

    fn exports(&self) -> &() {
        // Safety: Is safe due to `()` being a ZST.
        unsafe { self.0.exports.cast::<()>().as_ref().unwrap_or(&()) }
    }

    fn module_info(&self) -> ModuleInfo<'_> {
        // Safety: `ModuleInfo` is only a wrapper over a `FimoModuleInfo`.
        unsafe { ModuleInfo::borrow_from_ffi(self.0.module_info) }
    }

    fn context(&self) -> ContextView<'_> {
        // Safety: `ContextView` is only a wrapper over a `FimoContext`.
        unsafe { ContextView::borrow_from_ffi(self.0.context) }
    }

    fn data(&self) -> &Self::Data {
        // Safety: Is safe due to `()` being a ZST.
        unsafe { self.0.module_data.cast::<()>().as_ref().unwrap_or(&()) }
    }

    fn has_namespace_dependency(&self, namespace: &CStr) -> Result<DependencyType, Error> {
        let mut has_dependency = false;
        let mut is_static = false;

        // Safety: FFI call is safe.
        let error = unsafe {
            bindings::fimo_module_namespace_included(
                self.share_to_ffi(),
                namespace.as_ptr(),
                &mut has_dependency,
                &mut is_static,
            )
        };
        to_result(error)?;

        match (has_dependency, is_static) {
            (true, true) => Ok(DependencyType::StaticDependency),
            (true, false) => Ok(DependencyType::DynamicDependency),
            (false, _) => Ok(DependencyType::NoDependency),
        }
    }

    fn include_namespace(
        &mut self,
        _guard: &mut ModuleBackendGuard<'_>,
        namespace: &CStr,
    ) -> error::Result {
        // Safety: FFI call is safe.
        let error = unsafe {
            bindings::fimo_module_namespace_include(self.share_to_ffi(), namespace.as_ptr())
        };
        to_result(error)
    }

    fn exclude_namespace(
        &mut self,
        _guard: &mut ModuleBackendGuard<'_>,
        namespace: &CStr,
    ) -> error::Result {
        // Safety: FFI call is safe.
        let error = unsafe {
            bindings::fimo_module_namespace_exclude(self.share_to_ffi(), namespace.as_ptr())
        };
        to_result(error)
    }

    fn has_dependency(&self, module: &ModuleInfo<'_>) -> Result<DependencyType, Error> {
        let mut has_dependency = false;
        let mut is_static = false;

        // Safety: FFI call is safe.
        let error = unsafe {
            bindings::fimo_module_has_dependency(
                self.share_to_ffi(),
                module.share_to_ffi(),
                &mut has_dependency,
                &mut is_static,
            )
        };
        to_result(error)?;

        match (has_dependency, is_static) {
            (true, true) => Ok(DependencyType::StaticDependency),
            (true, false) => Ok(DependencyType::DynamicDependency),
            (false, _) => Ok(DependencyType::NoDependency),
        }
    }

    fn acquire_dependency<'ctx>(
        &mut self,
        _guard: &mut ModuleBackendGuard<'ctx>,
        dependency: &ModuleInfo<'_>,
    ) -> Result<ModuleInfo<'ctx>, Error> {
        // Safety: FFI call is safe.
        let error = unsafe {
            bindings::fimo_module_acquire_dependency(self.share_to_ffi(), dependency.share_to_ffi())
        };
        to_result(error)?;

        // Safety: Now that we acquired the dependency we can extend our reference to the module.
        unsafe { Ok(ModuleInfo::from_ffi(dependency.share_to_ffi())) }
    }

    fn remove_dependency(
        &mut self,
        _guard: &mut ModuleBackendGuard<'_>,
        dependency: ModuleInfo<'_>,
    ) -> error::Result {
        // Safety: FFI call is safe.
        let error = unsafe {
            bindings::fimo_module_relinquish_dependency(self.share_to_ffi(), dependency.into_ffi())
        };
        to_result(error)
    }

    unsafe fn load_symbol<T>(
        &self,
        name: &CStr,
        namespace: &CStr,
        version: Version,
    ) -> Result<Symbol<'_, T>, Error> {
        // Safety: We either initialize `symbol` or write an error.
        let symbol = unsafe {
            to_result_indirect_in_place(|error, symbol| {
                *error = bindings::fimo_module_load_symbol(
                    self.share_to_ffi(),
                    name.as_ptr(),
                    namespace.as_ptr(),
                    version.into_ffi(),
                    symbol.as_mut_ptr(),
                );
            })
        }?;

        // Safety: We own the symbol.
        unsafe { Ok(Symbol::from_ffi(symbol)) }
    }
}

impl crate::ffi::FFISharable<*const bindings::FimoModule> for OpaqueModule<'_> {
    type BorrowedView<'a> = OpaqueModule<'a>;

    fn share_to_ffi(&self) -> *const bindings::FimoModule {
        self.0
    }

    unsafe fn borrow_from_ffi<'a>(ffi: *const bindings::FimoModule) -> Self::BorrowedView<'a> {
        // Safety: `OpaqueModule` is a wrapper over a `FimoModule`.
        unsafe { OpaqueModule(&*ffi) }
    }
}

impl crate::ffi::FFITransferable<*const bindings::FimoModule> for OpaqueModule<'_> {
    fn into_ffi(self) -> *const bindings::FimoModule {
        self.0
    }

    unsafe fn from_ffi(ffi: *const bindings::FimoModule) -> Self {
        // Safety: The value must be valid.
        unsafe { Self(&*ffi) }
    }
}

/// A pseudo module.
///
/// The functions of the module backend require that the caller owns
/// a reference to their own module. This is a problem, as the constructor
/// of the context won't be assigned a module instance during bootstrapping.
/// As a workaround, we allow for the creation of pseudo modules, i.e.,
/// module handles without an associated module.
#[repr(transparent)]
#[derive(Debug)]
pub struct PseudoModule<'a>(OpaqueModule<'a>);

impl<'ctx> PseudoModule<'ctx> {
    /// Constructs a new `PseudoModule`.
    pub fn new(guard: &mut ModuleBackendGuard<'ctx>) -> Result<Self, Error> {
        // Safety: We either initialize `module` or write an error.
        let module = unsafe {
            to_result_indirect_in_place(|error, module| {
                *error =
                    bindings::fimo_module_pseudo_module_new(guard.into_ffi(), module.as_mut_ptr());
            })
        }?;

        // Safety: We own the module.
        unsafe { Ok(PseudoModule::from_ffi(module)) }
    }

    unsafe fn destroy_by_ref(
        &mut self,
        _guard: &mut ModuleBackendGuard<'_>,
    ) -> Result<Context, Error> {
        let module = self.share_to_ffi();

        // Safety: The ffi call is safe.
        let context = unsafe {
            to_result_indirect_in_place(|error, context| {
                *error = bindings::fimo_module_pseudo_module_destroy(module, context.as_mut_ptr());
            })
        }?;

        // Safety: The context returned by destroying the pseudo module is valid.
        unsafe { Ok(Context::from_ffi(context)) }
    }

    /// Destroys the `PseudoModule`.
    ///
    /// Unlike [`PseudoModule::drop`] this method can be called while the module
    /// backend is still locked.
    pub fn destroy(self, guard: &mut ModuleBackendGuard<'_>) -> Result<Context, Error> {
        let mut this = ManuallyDrop::new(self);

        // Safety: The module is not used afterward.
        unsafe { this.destroy_by_ref(guard) }
    }
}

impl<'a> Deref for PseudoModule<'a> {
    type Target = OpaqueModule<'a>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl crate::ffi::FFISharable<*const bindings::FimoModule> for PseudoModule<'_> {
    type BorrowedView<'a> = OpaqueModule<'a>;

    fn share_to_ffi(&self) -> *const bindings::FimoModule {
        self.0.into_ffi()
    }

    unsafe fn borrow_from_ffi<'a>(ffi: *const bindings::FimoModule) -> Self::BorrowedView<'a> {
        // Safety: `PseudoModule` is a wrapper over a `FimoModule`.
        unsafe { OpaqueModule::from_ffi(ffi) }
    }
}

impl crate::ffi::FFITransferable<*const bindings::FimoModule> for PseudoModule<'_> {
    fn into_ffi(self) -> *const bindings::FimoModule {
        let this = ManuallyDrop::new(self);
        this.0.into_ffi()
    }

    unsafe fn from_ffi(ffi: *const bindings::FimoModule) -> Self {
        // Safety: The value must be valid.
        unsafe { Self(OpaqueModule::from_ffi(ffi)) }
    }
}

impl Drop for PseudoModule<'_> {
    fn drop(&mut self) {
        let mut guard = self
            .context()
            .to_context()
            .lock_module_backend()
            .expect("the context should be valid");

        // Safety: The module is not used afterward.
        unsafe {
            self.destroy_by_ref(&mut guard)
                .expect("no module should depend on the pseudo module");
        }
    }
}
