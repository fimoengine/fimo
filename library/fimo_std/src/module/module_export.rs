use core::ffi::CStr;

use crate::{bindings, ffi::FFITransferable, version::Version};

use super::{ParameterAccess, ParameterType, ParameterValue};

/// Declaration of a module parameter.
#[repr(transparent)]
pub struct ParameterDeclaration(bindings::FimoModuleParamDecl);

impl ParameterDeclaration {
    /// Fetches the type of the parameter.
    pub fn parameter_type(&self) -> ParameterType {
        self.0.type_.try_into().expect("expected known enum value")
    }

    /// Fetches the access group specifier for the read permission.
    pub fn read_access(&self) -> ParameterAccess {
        self.0
            .read_access
            .try_into()
            .expect("expected known enum value")
    }

    /// Fetches the access group specifier for the write permission.
    pub fn write_access(&self) -> ParameterAccess {
        self.0
            .write_access
            .try_into()
            .expect("expected known enum value")
    }

    /// Fetches the name of the parameter.
    pub fn name(&self) -> &CStr {
        // Safety: The value is always a valid string.
        unsafe { CStr::from_ptr(self.0.name) }
    }

    /// Fetches the default value of the parameter.
    pub fn default_value(&self) -> ParameterValue {
        // Safety: We check the tag of the union.
        unsafe {
            match self.parameter_type() {
                ParameterType::U8 => ParameterValue::U8(self.0.default_value.u8_),
                ParameterType::U16 => ParameterValue::U16(self.0.default_value.u16_),
                ParameterType::U32 => ParameterValue::U32(self.0.default_value.u32_),
                ParameterType::U64 => ParameterValue::U64(self.0.default_value.u64_),
                ParameterType::I8 => ParameterValue::I8(self.0.default_value.i8_),
                ParameterType::I16 => ParameterValue::I16(self.0.default_value.i16_),
                ParameterType::I32 => ParameterValue::I32(self.0.default_value.i32_),
                ParameterType::I64 => ParameterValue::I64(self.0.default_value.i64_),
            }
        }
    }
}

// Safety: `FimoModuleParamDecl` is always `Send + Sync`.
unsafe impl Send for ParameterDeclaration {}

// Safety: `FimoModuleParamDecl` is always `Send + Sync`.
unsafe impl Sync for ParameterDeclaration {}

impl core::fmt::Debug for ParameterDeclaration {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("ParameterDeclaration")
            .field("read_access", &self.read_access())
            .field("write_access", &self.write_access())
            .field("name", &self.name())
            .field("default_value", &self.default_value())
            .finish()
    }
}

impl core::fmt::Display for ParameterDeclaration {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "{} ({}/{}), Default={}",
            self.name().to_string_lossy(),
            self.read_access(),
            self.write_access(),
            self.default_value()
        )
    }
}

impl crate::ffi::FFITransferable<bindings::FimoModuleParamDecl> for ParameterDeclaration {
    fn into_ffi(self) -> bindings::FimoModuleParamDecl {
        self.0
    }

    unsafe fn from_ffi(ffi: bindings::FimoModuleParamDecl) -> Self {
        Self(ffi)
    }
}

/// Declaration of a module namespace import.
#[repr(transparent)]
pub struct NamespaceImport(bindings::FimoModuleNamespaceImport);

impl NamespaceImport {
    /// Fetches the name of the namespace.
    pub fn name(&self) -> &CStr {
        // Safety: The value is always a valid string.
        unsafe { CStr::from_ptr(self.0.name) }
    }
}

// Safety: `FimoModuleNamespaceImport` is always `Send + Sync`.
unsafe impl Send for NamespaceImport {}

// Safety: `FimoModuleNamespaceImport` is always `Send + Sync`.
unsafe impl Sync for NamespaceImport {}

impl core::fmt::Debug for NamespaceImport {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("NamespaceImport")
            .field("name", &self.name())
            .finish()
    }
}

impl core::fmt::Display for NamespaceImport {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}", self.name().to_string_lossy(),)
    }
}

impl crate::ffi::FFITransferable<bindings::FimoModuleNamespaceImport> for NamespaceImport {
    fn into_ffi(self) -> bindings::FimoModuleNamespaceImport {
        self.0
    }

    unsafe fn from_ffi(ffi: bindings::FimoModuleNamespaceImport) -> Self {
        Self(ffi)
    }
}

/// Declaration of a module symbol import.
#[repr(transparent)]
pub struct SymbolImport(bindings::FimoModuleSymbolImport);

impl SymbolImport {
    /// Fetches the version of the symbol.
    pub fn version(&self) -> Version {
        // Safety: Is safe.
        unsafe { Version::from_ffi(self.0.version) }
    }

    /// Fetches the name of the symbol.
    pub fn name(&self) -> &CStr {
        // Safety: The value is always a valid string.
        unsafe { CStr::from_ptr(self.0.name) }
    }

    /// Fetches the namespace of the namespace.
    pub fn namespace(&self) -> &CStr {
        // Safety: The value is always a valid string.
        unsafe { CStr::from_ptr(self.0.ns) }
    }
}

// Safety: `FimoModuleSymbolImport` is always `Send + Sync`.
unsafe impl Send for SymbolImport {}

// Safety: `FimoModuleSymbolImport` is always `Send + Sync`.
unsafe impl Sync for SymbolImport {}

impl core::fmt::Debug for SymbolImport {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("SymbolImport")
            .field("name", &self.name())
            .field("namespace", &self.namespace())
            .field("version", &self.version())
            .finish()
    }
}

impl core::fmt::Display for SymbolImport {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "{}::{} ({})",
            self.namespace().to_string_lossy(),
            self.name().to_string_lossy(),
            self.version()
        )
    }
}

impl crate::ffi::FFITransferable<bindings::FimoModuleSymbolImport> for SymbolImport {
    fn into_ffi(self) -> bindings::FimoModuleSymbolImport {
        self.0
    }

    unsafe fn from_ffi(ffi: bindings::FimoModuleSymbolImport) -> Self {
        Self(ffi)
    }
}

/// Declaration of a module symbol export.
#[repr(transparent)]
pub struct SymbolExport(bindings::FimoModuleSymbolExport);

impl SymbolExport {
    /// Fetches a pointer to the exported symbol.
    pub fn symbol(&self) -> *const core::ffi::c_void {
        self.0.symbol
    }

    /// Fetches the version of the symbol.
    pub fn version(&self) -> Version {
        // Safety: Is safe.
        unsafe { Version::from_ffi(self.0.version) }
    }

    /// Fetches the name of the symbol.
    pub fn name(&self) -> &CStr {
        // Safety: The value is always a valid string.
        unsafe { CStr::from_ptr(self.0.name) }
    }

    /// Fetches the namespace of the namespace.
    pub fn namespace(&self) -> &CStr {
        // Safety: The value is always a valid string.
        unsafe { CStr::from_ptr(self.0.ns) }
    }
}

// Safety: `FimoModuleSymbolExport` is always `Send + Sync`.
unsafe impl Send for SymbolExport {}

// Safety: `FimoModuleSymbolExport` is always `Send + Sync`.
unsafe impl Sync for SymbolExport {}

impl core::fmt::Debug for SymbolExport {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("SymbolExport")
            .field("name", &self.name())
            .field("namespace", &self.namespace())
            .field("version", &self.version())
            .field("symbol", &self.symbol())
            .finish()
    }
}

impl core::fmt::Display for SymbolExport {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "{}::{} ({})",
            self.namespace().to_string_lossy(),
            self.name().to_string_lossy(),
            self.version()
        )
    }
}

impl crate::ffi::FFITransferable<bindings::FimoModuleSymbolExport> for SymbolExport {
    fn into_ffi(self) -> bindings::FimoModuleSymbolExport {
        self.0
    }

    unsafe fn from_ffi(ffi: bindings::FimoModuleSymbolExport) -> Self {
        Self(ffi)
    }
}

/// Declaration of a dynamic module symbol export.
#[repr(transparent)]
pub struct DynamicSymbolExport(bindings::FimoModuleDynamicSymbolExport);

impl DynamicSymbolExport {
    /// Fetches the symbol constructor.
    pub fn constructor(&self) -> bindings::FimoModuleDynamicSymbolConstructor {
        self.0.constructor
    }

    /// Fetches the symbol destructor.
    pub fn destructor(&self) -> bindings::FimoModuleDynamicSymbolDestructor {
        self.0.destructor
    }

    /// Fetches the version of the symbol.
    pub fn version(&self) -> Version {
        // Safety: Is safe.
        unsafe { Version::from_ffi(self.0.version) }
    }

    /// Fetches the name of the symbol.
    pub fn name(&self) -> &CStr {
        // Safety: The value is always a valid string.
        unsafe { CStr::from_ptr(self.0.name) }
    }

    /// Fetches the namespace of the namespace.
    pub fn namespace(&self) -> &CStr {
        // Safety: The value is always a valid string.
        unsafe { CStr::from_ptr(self.0.ns) }
    }
}

// Safety: `FimoModuleDynamicSymbolExport` is always `Send + Sync`.
unsafe impl Send for DynamicSymbolExport {}

// Safety: `FimoModuleDynamicSymbolExport` is always `Send + Sync`.
unsafe impl Sync for DynamicSymbolExport {}

impl core::fmt::Debug for DynamicSymbolExport {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("DynamicSymbolExport")
            .field("name", &self.name())
            .field("namespace", &self.namespace())
            .field("version", &self.version())
            .field("constructor", &self.constructor())
            .field("destructor", &self.destructor())
            .finish()
    }
}

impl core::fmt::Display for DynamicSymbolExport {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "{}::{} ({})",
            self.namespace().to_string_lossy(),
            self.name().to_string_lossy(),
            self.version()
        )
    }
}

impl crate::ffi::FFITransferable<bindings::FimoModuleDynamicSymbolExport> for DynamicSymbolExport {
    fn into_ffi(self) -> bindings::FimoModuleDynamicSymbolExport {
        self.0
    }

    unsafe fn from_ffi(ffi: bindings::FimoModuleDynamicSymbolExport) -> Self {
        Self(ffi)
    }
}

/// Declaration of an exported module.
#[repr(transparent)]
#[derive(Clone, Copy)]
pub struct ModuleExport<'a>(&'a bindings::FimoModuleExport);

impl ModuleExport<'_> {
    /// Export abi of the module.
    pub const EXPORT_ABI: i32 = bindings::FIMO_MODULE_EXPORT_ABI as i32;

    /// Fetches the name of the module declaration.
    pub fn name(&self) -> &CStr {
        // Safety: The value is always a valid string.
        unsafe { CStr::from_ptr(self.0.name) }
    }

    /// Fetches the description of the module declaration.
    pub fn description(&self) -> &CStr {
        // Safety: The value is always a valid string.
        unsafe { CStr::from_ptr(self.0.description) }
    }

    /// Fetches the author of the module declaration.
    pub fn author(&self) -> &CStr {
        // Safety: The value is always a valid string.
        unsafe { CStr::from_ptr(self.0.author) }
    }

    /// Fetches the license of the module declaration.
    pub fn license(&self) -> &CStr {
        // Safety: The value is always a valid string.
        unsafe { CStr::from_ptr(self.0.license) }
    }

    /// Fetches the list of parameters exposed by the module.
    pub fn parameters(&self) -> &[ParameterDeclaration] {
        // Safety: The layout is compatible.
        unsafe {
            let parameters = self.0.parameters.cast::<ParameterDeclaration>();
            if parameters.is_null() {
                &[]
            } else {
                core::slice::from_raw_parts(parameters, self.0.parameters_count as usize)
            }
        }
    }

    /// Fetches the list of namespaces imported by the module.
    pub fn imported_namespaces(&self) -> &[NamespaceImport] {
        // Safety: The layout is compatible.
        unsafe {
            let namespaces = self.0.namespace_imports.cast::<NamespaceImport>();
            if namespaces.is_null() {
                &[]
            } else {
                core::slice::from_raw_parts(namespaces, self.0.namespace_imports_count as usize)
            }
        }
    }

    /// Fetches the list of symbols imported by the module.
    pub fn imported_symbols(&self) -> &[SymbolImport] {
        // Safety: The layout is compatible.
        unsafe {
            let symbols = self.0.symbol_imports.cast::<SymbolImport>();
            if symbols.is_null() {
                &[]
            } else {
                core::slice::from_raw_parts(symbols, self.0.symbol_imports_count as usize)
            }
        }
    }

    /// Fetches the list of symbols exported by the module.
    pub fn exported_symbols(&self) -> &[SymbolExport] {
        // Safety: The layout is compatible.
        unsafe {
            let symbols = self.0.symbol_exports.cast::<SymbolExport>();
            if symbols.is_null() {
                &[]
            } else {
                core::slice::from_raw_parts(symbols, self.0.symbol_exports_count as usize)
            }
        }
    }

    /// Fetches the list of dynamic symbols exported by the module.
    pub fn exported_dynamic_symbols(&self) -> &[DynamicSymbolExport] {
        // Safety: The layout is compatible.
        unsafe {
            let symbols = self.0.dynamic_symbol_exports.cast::<DynamicSymbolExport>();
            if symbols.is_null() {
                &[]
            } else {
                core::slice::from_raw_parts(symbols, self.0.dynamic_symbol_exports_count as usize)
            }
        }
    }

    /// Fetches the module constructor.
    pub fn module_constructor(&self) -> bindings::FimoModuleConstructor {
        self.0.module_constructor
    }

    /// Fetches the module destructor.
    pub fn module_destructor(&self) -> bindings::FimoModuleDestructor {
        self.0.module_destructor
    }
}

/// Safety: `FimoModuleExport` is required to be `Send + Sync`.
unsafe impl Send for ModuleExport<'_> {}

/// Safety: `FimoModuleExport` is required to be `Send + Sync`.
unsafe impl Sync for ModuleExport<'_> {}

impl core::fmt::Debug for ModuleExport<'_> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("ModuleExport")
            .field("export_abi", &Self::EXPORT_ABI)
            .field("name", &self.name())
            .field("description", &self.description())
            .field("author", &self.author())
            .field("license", &self.license())
            .field("parameters", &self.parameters())
            .field("imported_namespaces", &self.imported_namespaces())
            .field("imported_symbols", &self.imported_symbols())
            .field("exported_symbols", &self.exported_symbols())
            .field("exported_dynamic_symbols", &self.exported_dynamic_symbols())
            .field("module_constructor", &self.module_constructor())
            .field("module_destructor", &self.module_destructor())
            .finish_non_exhaustive()
    }
}

impl core::fmt::Display for ModuleExport<'_> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "{} ({}/{}): {}",
            self.name().to_string_lossy(),
            self.author().to_string_lossy(),
            self.license().to_string_lossy(),
            self.description().to_string_lossy()
        )
    }
}

impl crate::ffi::FFISharable<*const bindings::FimoModuleExport> for ModuleExport<'_> {
    type BorrowedView<'a> = ModuleExport<'a>;

    fn share_to_ffi(&self) -> *const bindings::FimoModuleExport {
        self.0
    }

    unsafe fn borrow_from_ffi<'a>(
        ffi: *const bindings::FimoModuleExport,
    ) -> Self::BorrowedView<'a> {
        // Safety: `ffi` can not be null.
        unsafe {
            debug_assert_eq!((*ffi).export_abi, Self::EXPORT_ABI);
            debug_assert_eq!(
                (*ffi).type_,
                bindings::FimoStructType::FIMO_STRUCT_TYPE_MODULE_EXPORT
            );
            ModuleExport(&*ffi)
        }
    }
}

impl crate::ffi::FFITransferable<*const bindings::FimoModuleExport> for ModuleExport<'_> {
    fn into_ffi(self) -> *const bindings::FimoModuleExport {
        self.0
    }

    unsafe fn from_ffi(ffi: *const bindings::FimoModuleExport) -> Self {
        // Safety: `ffi` can not be null.
        unsafe {
            debug_assert_eq!((*ffi).export_abi, Self::EXPORT_ABI);
            debug_assert_eq!(
                (*ffi).type_,
                bindings::FimoStructType::FIMO_STRUCT_TYPE_MODULE_EXPORT
            );
            Self(&*ffi)
        }
    }
}
