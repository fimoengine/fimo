use core::{ffi::CStr, marker::PhantomData, mem::ManuallyDrop, ops::Deref};

use crate::{
    bindings,
    context::{Context, ContextView},
    error::{self, to_result, to_result_indirect, to_result_indirect_in_place, Error},
    ffi::{FFISharable, FFITransferable},
    version::Version,
};

use super::{ModuleSubsystem, NamespaceItem, NoState, Symbol, SymbolItem};

/// View of a `ModuleInfo`.
#[derive(Copy, Clone)]
pub struct ModuleInfoView<'a>(&'a bindings::FimoModuleInfo);

impl ModuleInfoView<'_> {
    /// Unique module name.
    pub fn name(&self) -> &CStr {
        // Safety: The name is a valid string.
        unsafe { CStr::from_ptr(self.0.name) }
    }

    /// Module description.
    pub fn description(&self) -> Option<&CStr> {
        // Safety: The string is valid or null.
        unsafe { self.0.description.as_ref().map(|x| CStr::from_ptr(x)) }
    }

    /// Module author.
    pub fn author(&self) -> Option<&CStr> {
        // Safety: The string is valid or null.
        unsafe { self.0.author.as_ref().map(|x| CStr::from_ptr(x)) }
    }

    /// Module license.
    pub fn license(&self) -> Option<&CStr> {
        // Safety: The string is valid or null.
        unsafe { self.0.license.as_ref().map(|x| CStr::from_ptr(x)) }
    }

    /// Path to the module directory.
    pub fn module_path(&self) -> &CStr {
        // Safety: The module path is a valid string.
        unsafe { CStr::from_ptr(self.0.module_path) }
    }
}

impl ModuleInfoView<'_> {
    /// Searches for a module by its name.
    pub fn find_by_name(ctx: &impl ModuleSubsystem, name: &CStr) -> Result<ModuleInfo, Error> {
        // Safety: Either we get an error, or we initialize the module.
        let module = unsafe {
            to_result_indirect_in_place(|error, module| {
                *error = bindings::fimo_module_find_by_name(
                    ctx.share_to_ffi(),
                    name.as_ptr(),
                    module.as_mut_ptr(),
                );
            })
        }?;

        // Safety: We own the module info.
        let view = unsafe { ModuleInfoView::from_ffi(module) };
        Ok(ModuleInfo(view))
    }

    /// Searches for a module by a symbol it exports.
    pub fn find_by_symbol(
        ctx: &impl ModuleSubsystem,
        name: &CStr,
        namespace: &CStr,
        version: Version,
    ) -> Result<ModuleInfo, Error> {
        // Safety: Either we get an error, or we initialize the module.
        let module = unsafe {
            to_result_indirect_in_place(|error, module| {
                *error = bindings::fimo_module_find_by_symbol(
                    ctx.share_to_ffi(),
                    name.as_ptr(),
                    namespace.as_ptr(),
                    version.into_ffi(),
                    module.as_mut_ptr(),
                );
            })
        }?;

        // Safety: We own the module info.
        let view = unsafe { ModuleInfoView::from_ffi(module) };
        Ok(ModuleInfo(view))
    }

    /// Unloads the module.
    ///
    /// If successful, this function unloads the module. To succeed, the module no other module may
    /// depend on the module. This function automatically unloads cleans up unreferenced modules,
    /// except if they are a pseudo module.
    pub fn unload(&self, ctx: &impl ModuleSubsystem) -> error::Result {
        // Safety: The ffi call is safe.
        to_result_indirect(|error| unsafe {
            *error = bindings::fimo_module_unload(ctx.share_to_ffi(), self.share_to_ffi());
        })
    }

    /// Checks whether the underlying module is still loaded.
    pub fn is_loaded(&self) -> bool {
        let is_loaded = self.0.is_loaded.unwrap();
        // Safety: The ffi call is safe.
        unsafe { (is_loaded)(self.share_to_ffi()) }
    }

    /// Locks the underlying module from being unloaded.
    ///
    /// The module may be locked multiple times.
    pub fn lock_unload(&self) -> Result<ModuleInfoGuard<'_>, Error> {
        let lock_unload = self.0.lock_unload.unwrap();
        // Safety: The ffi call is safe.
        to_result_indirect(|error| unsafe {
            *error = lock_unload(self.share_to_ffi());
        })?;
        Ok(ModuleInfoGuard(*self))
    }

    /// Unlocks the underlying module, allowing it to be unloaded again.
    ///
    /// # Safety
    ///
    /// The module must have been locked.
    pub unsafe fn unlock_unload(&self) {
        let unlock_unload = self.0.unlock_unload.unwrap();
        // Safety: The ffi call is safe.
        unsafe { (unlock_unload)(self.share_to_ffi()) }
    }

    /// Acquires the module info by increasing the reference count.
    pub fn to_owned(&self) -> ModuleInfo {
        let acquire = self.0.acquire.unwrap();
        // Safety: Is sound, as we acquired a strong reference.
        unsafe {
            (acquire)(self.share_to_ffi());
            ModuleInfo::from_ffi(self.0)
        }
    }
}

// Safety: `FimoModuleInfo` must be `Send + Sync`.
unsafe impl Send for ModuleInfoView<'_> {}

// Safety: `FimoModuleInfo` must be `Send + Sync`.
unsafe impl Sync for ModuleInfoView<'_> {}

impl PartialEq for ModuleInfoView<'_> {
    fn eq(&self, other: &Self) -> bool {
        core::ptr::eq(self.0, other.0)
    }
}

impl Eq for ModuleInfoView<'_> {}

impl core::fmt::Debug for ModuleInfoView<'_> {
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

impl core::fmt::Display for ModuleInfoView<'_> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "{} ({})",
            self.name().to_string_lossy(),
            self.author().map_or("".into(), |x| x.to_string_lossy())
        )
    }
}

impl FFISharable<*const bindings::FimoModuleInfo> for ModuleInfoView<'_> {
    type BorrowedView<'a> = ModuleInfoView<'a>;

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
            ModuleInfoView(&*ffi)
        }
    }
}

impl FFITransferable<*const bindings::FimoModuleInfo> for ModuleInfoView<'_> {
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

/// A guard of a locked module.
#[repr(transparent)]
#[derive(Debug, PartialEq, Eq)]
pub struct ModuleInfoGuard<'a>(ModuleInfoView<'a>);

impl<'a> Deref for ModuleInfoGuard<'a> {
    type Target = ModuleInfoView<'a>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Drop for ModuleInfoGuard<'_> {
    fn drop(&mut self) {
        // Safety: We own the lock.
        unsafe { self.0.unlock_unload() }
    }
}

/// Info of a loaded module.
#[repr(transparent)]
#[derive(Debug, PartialEq, Eq)]
pub struct ModuleInfo(ModuleInfoView<'static>);

impl ModuleInfo {
    /// Searches for a module by its name.
    pub fn find_by_name(ctx: &impl ModuleSubsystem, name: &CStr) -> Result<Self, Error> {
        ModuleInfoView::find_by_name(ctx, name)
    }

    /// Searches for a module by a symbol it exports.
    pub fn find_by_symbol(
        ctx: &impl ModuleSubsystem,
        name: &CStr,
        namespace: &CStr,
        version: Version,
    ) -> Result<Self, Error> {
        ModuleInfoView::find_by_symbol(ctx, name, namespace, version)
    }
}

impl core::fmt::Display for ModuleInfo {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        core::fmt::Display::fmt(&**self, f)
    }
}

impl Deref for ModuleInfo {
    type Target = ModuleInfoView<'static>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Clone for ModuleInfo {
    fn clone(&self) -> Self {
        self.to_owned()
    }
}

impl Drop for ModuleInfo {
    fn drop(&mut self) {
        let release = self.0 .0.acquire.unwrap();
        // Safety: The ffi call is safe.
        unsafe { (release)(self.share_to_ffi()) }
    }
}

impl FFISharable<*const bindings::FimoModuleInfo> for ModuleInfo {
    type BorrowedView<'a> = ModuleInfoView<'a>;

    fn share_to_ffi(&self) -> *const bindings::FimoModuleInfo {
        self.0.share_to_ffi()
    }

    unsafe fn borrow_from_ffi<'a>(ffi: *const bindings::FimoModuleInfo) -> Self::BorrowedView<'a> {
        // Safety: See above.
        unsafe { ModuleInfoView::borrow_from_ffi(ffi) }
    }
}

impl FFITransferable<*const bindings::FimoModuleInfo> for ModuleInfo {
    fn into_ffi(self) -> *const bindings::FimoModuleInfo {
        self.0.into_ffi()
    }

    unsafe fn from_ffi(ffi: *const bindings::FimoModuleInfo) -> Self {
        // Safety: The contract of this method allows us to assume ownership.
        unsafe { Self(ModuleInfoView::from_ffi(ffi)) }
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
/// remains loaded. Modules mut not leak any resources outside its own
/// module, ensuring that they are destroyed upon module unloading.
///
/// # Safety
///
/// The implementor must ensure that the associated table types are compatible
/// with the ones the module expects.
pub unsafe trait Module:
    Send + Sync + for<'a> FFISharable<*const bindings::FimoModule, BorrowedView<'a> = OpaqueModule<'a>>
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
    fn module_info(&self) -> ModuleInfoView<'_>;

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
    fn include_namespace(&self, namespace: &CStr) -> error::Result;

    /// Removes a namespace from the module.
    ///
    /// Once excluded, the caller guarantees to relinquish access to the symbols contained in said
    /// namespace. It is only possible to exclude namespaces that were manually added, whereas
    /// static namespace includes remain valid until the module is unloaded.
    ///
    /// # Safety
    ///
    /// The caller must ensure that they don't utilize and symbol from the namespace that will be
    /// excluded.
    unsafe fn exclude_namespace(&self, namespace: &CStr) -> error::Result;

    /// Checks if a module depends on another module.
    ///
    /// Checks if `module` is a dependency of the current instance. In that case the instance is
    /// allowed to access the symbols exported by `module`.
    fn has_dependency(&self, module: &ModuleInfoView<'_>) -> Result<DependencyType, Error>;

    /// Acquires another module as a dependency.
    ///
    /// After acquiring a module as a dependency, the module is allowed access to the symbols and
    /// protected parameters of said dependency. Trying to acquire a dependency to a module that is
    /// already a dependency, or to a module that would result in a circular dependency will result
    /// in an error.
    fn acquire_dependency(&self, dependency: &ModuleInfoView<'_>) -> error::Result;

    /// Removes a module as a dependency.
    ///
    /// By removing a module as a dependency, the caller ensures that it does not own any references
    /// to resources originating from the former dependency, and allows for the unloading of the
    /// module. A module can only relinquish dependencies to modules that were acquired dynamically,
    /// as static dependencies remain valid until the module is unloaded.
    ///
    /// # Safety
    ///
    /// Calling this method invalidates all loaded symbols from the dependency.
    unsafe fn remove_dependency(&self, dependency: ModuleInfoView<'_>) -> error::Result;

    /// Loads a symbol from the module subsystem.
    ///
    /// The caller can query the backend for a symbol of a loaded module. This is useful for loading
    /// optional symbols, or for loading symbols after the creation of a module. The symbol, if it
    /// exists, is returned, and can be used until the module relinquishes the dependency to the
    /// module that exported the symbol. This function fails, if the module containing the symbol is
    /// not a dependency of the module, or if the module has not included the required namespace.
    fn load_symbol<T: SymbolItem>(&self) -> Result<Symbol<'_, T::Type>, Error> {
        // Safety: We know the type of the symbol from the item.
        unsafe { self.load_symbol_unchecked(T::NAME, T::Namespace::NAME, T::VERSION) }
    }

    /// Loads a symbol from the module subsystem.
    ///
    /// The caller can query the backend for a symbol of a loaded module. This is useful for loading
    /// optional symbols, or for loading symbols after the creation of a module. The symbol, if it
    /// exists, is returned, and can be used until the module relinquishes the dependency to the
    /// module that exported the symbol. This function fails, if the module containing the symbol is
    /// not a dependency of the module, or if the module has not included the required namespace.
    ///
    /// # Safety
    ///
    /// Users of this API must specify the correct type of the symbol.
    unsafe fn load_symbol_unchecked<T>(
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
    type Data = NoState;

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

    fn module_info(&self) -> ModuleInfoView<'_> {
        // Safety: `ModuleInfo` is only a wrapper over a `FimoModuleInfo`.
        unsafe { ModuleInfoView::borrow_from_ffi(self.0.module_info) }
    }

    fn context(&self) -> ContextView<'_> {
        // Safety: `ContextView` is only a wrapper over a `FimoContext`.
        unsafe { ContextView::borrow_from_ffi(self.0.context) }
    }

    fn data(&self) -> &Self::Data {
        // Safety: Is safe due to `()` being a ZST.
        unsafe {
            self.0
                .module_data
                .cast::<NoState>()
                .as_ref()
                .unwrap_or(&NoState)
        }
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

    fn include_namespace(&self, namespace: &CStr) -> error::Result {
        // Safety: FFI call is safe.
        let error = unsafe {
            bindings::fimo_module_namespace_include(self.share_to_ffi(), namespace.as_ptr())
        };
        to_result(error)
    }

    unsafe fn exclude_namespace(&self, namespace: &CStr) -> error::Result {
        // Safety: FFI call is safe.
        let error = unsafe {
            bindings::fimo_module_namespace_exclude(self.share_to_ffi(), namespace.as_ptr())
        };
        to_result(error)
    }

    fn has_dependency(&self, module: &ModuleInfoView<'_>) -> Result<DependencyType, Error> {
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

    fn acquire_dependency(&self, dependency: &ModuleInfoView<'_>) -> error::Result {
        // Safety: FFI call is safe.
        let error = unsafe {
            bindings::fimo_module_acquire_dependency(self.share_to_ffi(), dependency.share_to_ffi())
        };
        to_result(error)
    }

    unsafe fn remove_dependency(&self, dependency: ModuleInfoView<'_>) -> error::Result {
        // Safety: FFI call is safe.
        let error = unsafe {
            bindings::fimo_module_relinquish_dependency(self.share_to_ffi(), dependency.into_ffi())
        };
        to_result(error)
    }

    unsafe fn load_symbol_unchecked<T>(
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

impl FFISharable<*const bindings::FimoModule> for OpaqueModule<'_> {
    type BorrowedView<'a> = OpaqueModule<'a>;

    fn share_to_ffi(&self) -> *const bindings::FimoModule {
        self.0
    }

    unsafe fn borrow_from_ffi<'a>(ffi: *const bindings::FimoModule) -> Self::BorrowedView<'a> {
        // Safety: `OpaqueModule` is a wrapper over a `FimoModule`.
        unsafe { OpaqueModule(&*ffi) }
    }
}

impl FFITransferable<*const bindings::FimoModule> for OpaqueModule<'_> {
    fn into_ffi(self) -> *const bindings::FimoModule {
        self.0
    }

    unsafe fn from_ffi(ffi: *const bindings::FimoModule) -> Self {
        // Safety: The value must be valid.
        unsafe { Self(&*ffi) }
    }
}

/// A strong reference to a module.
///
/// An instance of this type may not be shared or transferred to other modules.
#[repr(transparent)]
#[derive(Debug, Clone, Copy)]
pub struct GenericModule<'a, Par, Res, Imp, Exp, Data>
where
    Par: Send + Sync,
    Res: Send + Sync,
    Imp: Send + Sync,
    Exp: Send + Sync,
    Data: Send + Sync,
{
    module: OpaqueModule<'a>,
    _parameters: PhantomData<&'a Par>,
    _resources: PhantomData<&'a Res>,
    _imports: PhantomData<&'a Imp>,
    _exports: PhantomData<&'a Exp>,
    _data: PhantomData<&'a Data>,
}

// Safety:
unsafe impl<Par, Res, Imp, Exp, Data> Module for GenericModule<'_, Par, Res, Imp, Exp, Data>
where
    Par: Send + Sync,
    Res: Send + Sync,
    Imp: Send + Sync,
    Exp: Send + Sync,
    Data: Send + Sync,
{
    type Parameters = Par;
    type Resources = Res;
    type Imports = Imp;
    type Exports = Exp;
    type Data = Data;

    fn parameters(&self) -> &Self::Parameters {
        // Safety: The only way to construct a `GenericModule`
        // is through unsafe functions, where the users have to
        // ensure that the signatures matches the types contained
        // in the module.
        unsafe { &*core::ptr::from_ref(self.module.parameters()).cast() }
    }

    fn resources(&self) -> &Self::Resources {
        // Safety: The only way to construct a `GenericModule`
        // is through unsafe functions, where the users have to
        // ensure that the signatures matches the types contained
        // in the module.
        unsafe { &*core::ptr::from_ref(self.module.resources()).cast() }
    }

    fn imports(&self) -> &Self::Imports {
        // Safety: The only way to construct a `GenericModule`
        // is through unsafe functions, where the users have to
        // ensure that the signatures matches the types contained
        // in the module.
        unsafe { &*core::ptr::from_ref(self.module.imports()).cast() }
    }

    fn exports(&self) -> &Self::Exports {
        // Safety: The only way to construct a `GenericModule`
        // is through unsafe functions, where the users have to
        // ensure that the signatures matches the types contained
        // in the module.
        unsafe { &*core::ptr::from_ref(self.module.exports()).cast() }
    }

    fn module_info(&self) -> ModuleInfoView<'_> {
        self.module.module_info()
    }

    fn context(&self) -> ContextView<'_> {
        self.module.context()
    }

    fn data(&self) -> &Self::Data {
        // Safety: The only way to construct a `GenericModule`
        // is through unsafe functions, where the users have to
        // ensure that the signatures matches the types contained
        // in the module.
        unsafe { &*core::ptr::from_ref(self.module.data()).cast() }
    }

    fn has_namespace_dependency(&self, namespace: &CStr) -> Result<DependencyType, Error> {
        self.module.has_namespace_dependency(namespace)
    }

    fn include_namespace(&self, namespace: &CStr) -> error::Result {
        self.module.include_namespace(namespace)
    }

    unsafe fn exclude_namespace(&self, namespace: &CStr) -> error::Result {
        // Safety: The caller ensures that the contract is valid.
        unsafe { self.module.exclude_namespace(namespace) }
    }

    fn has_dependency(&self, module: &ModuleInfoView<'_>) -> Result<DependencyType, Error> {
        self.module.has_dependency(module)
    }

    fn acquire_dependency(&self, dependency: &ModuleInfoView<'_>) -> error::Result {
        self.module.acquire_dependency(dependency)
    }

    unsafe fn remove_dependency(&self, dependency: ModuleInfoView<'_>) -> error::Result {
        // Safety: The caller ensures that the contract is valid.
        unsafe { self.module.remove_dependency(dependency) }
    }

    unsafe fn load_symbol_unchecked<T>(
        &self,
        name: &CStr,
        namespace: &CStr,
        version: Version,
    ) -> Result<Symbol<'_, T>, Error> {
        // Safety: The caller ensures that the contract is valid.
        unsafe {
            self.module
                .load_symbol_unchecked::<T>(name, namespace, version)
        }
    }
}

impl<Par, Res, Imp, Exp, Data> FFISharable<*const bindings::FimoModule>
    for GenericModule<'_, Par, Res, Imp, Exp, Data>
where
    Par: Send + Sync,
    Res: Send + Sync,
    Imp: Send + Sync,
    Exp: Send + Sync,
    Data: Send + Sync,
{
    type BorrowedView<'a> = OpaqueModule<'a>;

    fn share_to_ffi(&self) -> *const bindings::FimoModule {
        self.module.into_ffi()
    }

    unsafe fn borrow_from_ffi<'a>(ffi: *const bindings::FimoModule) -> Self::BorrowedView<'a> {
        // Safety: `GenericModule` is a wrapper over a `FimoModule`.
        unsafe { OpaqueModule::from_ffi(ffi) }
    }
}

impl<Par, Res, Imp, Exp, Data> FFITransferable<*const bindings::FimoModule>
    for GenericModule<'_, Par, Res, Imp, Exp, Data>
where
    Par: Send + Sync,
    Res: Send + Sync,
    Imp: Send + Sync,
    Exp: Send + Sync,
    Data: Send + Sync,
{
    fn into_ffi(self) -> *const bindings::FimoModule {
        self.module.into_ffi()
    }

    unsafe fn from_ffi(ffi: *const bindings::FimoModule) -> Self {
        // Safety: The value must be valid.
        unsafe {
            Self {
                module: OpaqueModule::from_ffi(ffi),
                _parameters: PhantomData,
                _resources: PhantomData,
                _imports: PhantomData,
                _exports: PhantomData,
                _data: PhantomData,
            }
        }
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
pub struct PseudoModule(OpaqueModule<'static>);

impl PseudoModule {
    /// Constructs a new `PseudoModule`.
    pub fn new(ctx: &impl ModuleSubsystem) -> Result<Self, Error> {
        // Safety: We either initialize `module` or write an error.
        let module = unsafe {
            to_result_indirect_in_place(|error, module| {
                *error = bindings::fimo_module_pseudo_module_new(
                    ctx.share_to_ffi(),
                    module.as_mut_ptr(),
                );
            })
        }?;

        // Safety: We own the module.
        unsafe { Ok(PseudoModule::from_ffi(module)) }
    }

    unsafe fn destroy_by_ref(&mut self) -> Result<Context, Error> {
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
    pub fn destroy(self) -> Result<Context, Error> {
        let mut this = ManuallyDrop::new(self);

        // Safety: The module is not used afterward.
        unsafe { this.destroy_by_ref() }
    }
}

impl Deref for PseudoModule {
    type Target = OpaqueModule<'static>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl FFISharable<*const bindings::FimoModule> for PseudoModule {
    type BorrowedView<'a> = OpaqueModule<'a>;

    fn share_to_ffi(&self) -> *const bindings::FimoModule {
        self.0.into_ffi()
    }

    unsafe fn borrow_from_ffi<'a>(ffi: *const bindings::FimoModule) -> Self::BorrowedView<'a> {
        // Safety: `PseudoModule` is a wrapper over a `FimoModule`.
        unsafe { OpaqueModule::from_ffi(ffi) }
    }
}

impl FFITransferable<*const bindings::FimoModule> for PseudoModule {
    fn into_ffi(self) -> *const bindings::FimoModule {
        let this = ManuallyDrop::new(self);
        this.0.into_ffi()
    }

    unsafe fn from_ffi(ffi: *const bindings::FimoModule) -> Self {
        // Safety: The value must be valid.
        unsafe { Self(OpaqueModule::from_ffi(ffi)) }
    }
}

impl Drop for PseudoModule {
    fn drop(&mut self) {
        // Safety: The module is not used afterward.
        unsafe {
            self.destroy_by_ref()
                .expect("no module should depend on the pseudo module");
        }
    }
}
