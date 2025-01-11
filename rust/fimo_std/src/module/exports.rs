//! Utilities for defining and working with module exports.

use crate::{
    bindings,
    error::AnyResult,
    ffi::{ConstCStr, ConstNonNull, OpaqueHandle},
    module::{Info, ParameterAccessGroup, ParameterData, ParameterType},
    version::Version,
};
use std::{
    ffi::CStr,
    fmt::{Debug, Formatter},
    marker::PhantomData,
    ptr::NonNull,
};

/// Type able to contain all parameter types.
#[repr(C)]
#[derive(Copy, Clone)]
pub union DefaultParameterValueUnion {
    pub u8: u8,
    pub u16: u16,
    pub u32: u32,
    pub u64: u64,
    pub i8: i8,
    pub i16: i16,
    pub i32: i32,
    pub i64: i64,
}

/// Type able to contain all parameter types.
#[derive(Copy, Clone)]
pub enum DefaultParameterValue {
    U8(u8),
    U16(u16),
    U32(u32),
    U64(u64),
    I8(i8),
    I16(i16),
    I32(i32),
    I64(i64),
}

/// Declaration of a module parameter.
#[repr(C)]
pub struct Parameter<'a> {
    pub r#type: ParameterType,
    pub read_group: ParameterAccessGroup,
    pub write_group: ParameterAccessGroup,
    pub read: Option<unsafe extern "C" fn(parameter: ParameterData<u8>, value: NonNull<()>)>,
    pub write: Option<unsafe extern "C" fn(parameter: ParameterData<u8>, value: ConstNonNull<()>)>,
    pub name: ConstCStr,
    /// # Safety
    ///
    /// Must match the type provided in `type`.
    pub unsafe default_value: DefaultParameterValueUnion,
    pub _phantom: PhantomData<&'a ()>,
}

impl<'a> Parameter<'a> {
    /// Constructs a new `Parameter`.
    pub const fn new(default_value: DefaultParameterValue, name: &'a CStr) -> Self {
        let name = ConstCStr::new(name);
        let (r#type, default_value) = match default_value {
            DefaultParameterValue::U8(x) => {
                (ParameterType::U8, DefaultParameterValueUnion { u8: x })
            }
            DefaultParameterValue::U16(x) => {
                (ParameterType::U16, DefaultParameterValueUnion { u16: x })
            }
            DefaultParameterValue::U32(x) => {
                (ParameterType::U32, DefaultParameterValueUnion { u32: x })
            }
            DefaultParameterValue::U64(x) => {
                (ParameterType::U64, DefaultParameterValueUnion { u64: x })
            }
            DefaultParameterValue::I8(x) => {
                (ParameterType::I8, DefaultParameterValueUnion { i8: x })
            }
            DefaultParameterValue::I16(x) => {
                (ParameterType::I64, DefaultParameterValueUnion { i16: x })
            }
            DefaultParameterValue::I32(x) => {
                (ParameterType::I32, DefaultParameterValueUnion { i32: x })
            }
            DefaultParameterValue::I64(x) => {
                (ParameterType::I64, DefaultParameterValueUnion { i64: x })
            }
        };

        unsafe {
            Self {
                r#type,
                read_group: ParameterAccessGroup::Private,
                write_group: ParameterAccessGroup::Private,
                read: None,
                write: None,
                name,
                default_value,
                _phantom: PhantomData,
            }
        }
    }

    /// Sets a custom read group.
    pub const fn with_read_group(mut self, read_group: ParameterAccessGroup) -> Self {
        self.read_group = read_group;
        self
    }

    /// Sets a custom write group.
    pub const fn with_write_group(mut self, write_group: ParameterAccessGroup) -> Self {
        self.write_group = write_group;
        self
    }

    /// Sets a custom read function.
    pub const fn with_read(
        mut self,
        read: Option<unsafe extern "C" fn(parameter: ParameterData<u8>, value: NonNull<()>)>,
    ) -> Self {
        self.read = read;
        self
    }

    /// Sets a custom write function.
    pub const fn with_write(
        mut self,
        write: Option<unsafe extern "C" fn(parameter: ParameterData<u8>, value: ConstNonNull<()>)>,
    ) -> Self {
        self.write = write;
        self
    }

    /// Returns the name of the parameter.
    pub const fn name(&self) -> &CStr {
        unsafe { self.name.as_ref() }
    }

    /// Reads the default value of the parameter.
    pub const fn default_value(&self) -> DefaultParameterValue {
        unsafe {
            match self.r#type {
                ParameterType::U8 => DefaultParameterValue::U8(self.default_value.u8),
                ParameterType::U16 => DefaultParameterValue::U16(self.default_value.u16),
                ParameterType::U32 => DefaultParameterValue::U32(self.default_value.u32),
                ParameterType::U64 => DefaultParameterValue::U64(self.default_value.u64),
                ParameterType::I8 => DefaultParameterValue::I8(self.default_value.i8),
                ParameterType::I16 => DefaultParameterValue::I16(self.default_value.i16),
                ParameterType::I32 => DefaultParameterValue::I32(self.default_value.i32),
                ParameterType::I64 => DefaultParameterValue::I64(self.default_value.i64),
            }
        }
    }
}

impl Debug for Parameter<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        unsafe {
            f.debug_struct("Parameter")
                .field("type", &self.r#type)
                .field("read_group", &self.read_group)
                .field("write_group", &self.write_group)
                .field("read", &self.read)
                .field("write", &self.write)
                .field("name", &self.name.as_ref())
                .field("default_value", match self.r#type {
                    ParameterType::U8 => &self.default_value.u8 as &dyn Debug,
                    ParameterType::U16 => &self.default_value.u16 as &dyn Debug,
                    ParameterType::U32 => &self.default_value.u32 as &dyn Debug,
                    ParameterType::U64 => &self.default_value.u64 as &dyn Debug,
                    ParameterType::I8 => &self.default_value.i8 as &dyn Debug,
                    ParameterType::I16 => &self.default_value.i16 as &dyn Debug,
                    ParameterType::I32 => &self.default_value.i32 as &dyn Debug,
                    ParameterType::I64 => &self.default_value.i64 as &dyn Debug,
                })
                .finish()
        }
    }
}

unsafe impl Copy for Parameter<'_> {}

#[allow(clippy::expl_impl_clone_on_copy)]
impl Clone for Parameter<'_> {
    fn clone(&self) -> Self {
        *self
    }
}

/// Declaration of a module resource.
#[repr(C)]
#[derive(Copy, Clone)]
pub struct Resource<'a> {
    pub path: ConstCStr,
    pub _phantom: PhantomData<&'a [u8]>,
}

impl<'a> Resource<'a> {
    /// Constructs a new `Resource`.
    pub const fn new(path: &'a CStr) -> Self {
        Self {
            path: ConstCStr::new(path),
            _phantom: PhantomData,
        }
    }

    /// Extracts the path of the resource.
    pub const fn path(&self) -> &CStr {
        unsafe { self.path.as_ref() }
    }
}

impl Debug for Resource<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Resource")
            .field("path", &self.path())
            .finish()
    }
}

/// Declaration of a module namespace import.
#[repr(C)]
#[derive(Copy, Clone)]
pub struct Namespace<'a> {
    pub name: ConstCStr,
    pub _phantom: PhantomData<&'a [u8]>,
}

impl<'a> Namespace<'a> {
    /// Constructs a new `Namespace`.
    pub const fn new(name: &'a CStr) -> Self {
        Self {
            name: ConstCStr::new(name),
            _phantom: PhantomData,
        }
    }

    /// Extracts the name of the namespace.
    pub const fn name(&self) -> &CStr {
        unsafe { self.name.as_ref() }
    }
}

impl Debug for Namespace<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Namespace")
            .field("name", &self.name())
            .finish()
    }
}

/// Declaration of a module symbol import.
#[repr(C)]
#[derive(Copy, Clone)]
pub struct SymbolImport<'a> {
    pub version: Version,
    pub name: ConstCStr,
    pub namespace: ConstCStr,
    pub _phantom: PhantomData<&'a [u8]>,
}

impl<'a> SymbolImport<'a> {
    /// Constructs a new `SymbolImport`.
    pub const fn new(version: Version, name: &'a CStr) -> Self {
        Self {
            version,
            name: ConstCStr::new(name),
            namespace: ConstCStr::new(c""),
            _phantom: PhantomData,
        }
    }

    /// Sets the namespace of the symbol.
    pub const fn with_namespace(mut self, namespace: &'a CStr) -> Self {
        self.namespace = ConstCStr::new(namespace);
        self
    }

    /// Extracts the name of the symbol.
    pub const fn name(&self) -> &CStr {
        unsafe { self.name.as_ref() }
    }

    /// Extracts the namespace of the symbol.
    pub const fn namespace(&self) -> &CStr {
        unsafe { self.namespace.as_ref() }
    }
}

impl Debug for SymbolImport<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SymbolImport")
            .field("version", &self.version)
            .field("name", &self.name())
            .field("namespace", &self.namespace())
            .finish()
    }
}

/// Declaration of a static module symbol export.
#[repr(C)]
#[derive(Copy, Clone)]
pub struct SymbolExport<'a> {
    pub symbol: ConstNonNull<()>,
    pub version: Version,
    pub name: ConstCStr,
    pub namespace: ConstCStr,
    pub _phantom: PhantomData<&'a [u8]>,
}

impl<'a> SymbolExport<'a> {
    /// Constructs a new `SymbolExport`.
    pub const fn new<T>(symbol: &'a T, version: Version, name: &'a CStr) -> Self {
        Self {
            symbol: unsafe { ConstNonNull::new_unchecked(symbol).cast() },
            version,
            name: ConstCStr::new(name),
            namespace: ConstCStr::new(c""),
            _phantom: PhantomData,
        }
    }

    /// Sets the namespace of the symbol.
    pub const fn with_namespace(mut self, namespace: &'a CStr) -> Self {
        self.namespace = ConstCStr::new(namespace);
        self
    }

    /// Extracts the name of the symbol.
    pub const fn name(&self) -> &CStr {
        unsafe { self.name.as_ref() }
    }

    /// Extracts the namespace of the symbol.
    pub const fn namespace(&self) -> &CStr {
        unsafe { self.namespace.as_ref() }
    }
}

impl Debug for SymbolExport<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SymbolExport")
            .field("symbol", &self.symbol)
            .field("version", &self.version)
            .field("name", &self.name())
            .field("namespace", &self.namespace())
            .finish()
    }
}

/// Declaration of a static module symbol export.
#[repr(C)]
#[derive(Copy, Clone)]
pub struct DynamicSymbolExport<'a> {
    pub constructor: unsafe extern "C" fn(
        instance: &bindings::FimoModuleInstance,
        symbol: &mut NonNull<()>,
    ) -> AnyResult,
    pub destructor: unsafe extern "C" fn(symbol: NonNull<()>),
    pub version: Version,
    pub name: ConstCStr,
    pub namespace: ConstCStr,
    pub _phantom: PhantomData<&'a [u8]>,
}

impl<'a> DynamicSymbolExport<'a> {
    // /// Constructs a new `DynamicSymbolExport`.
    // pub const fn new<T>(symbol: &'a T, version: Version, name: &'a CStr) -> Self {
    //     Self {
    //         symbol: unsafe { ConstNonNull::new_unchecked(symbol).cast() },
    //         version,
    //         name: ConstCStr::new(name),
    //         namespace: ConstCStr::new(c""),
    //         _phantom: PhantomData,
    //     }
    // }

    /// Sets the namespace of the symbol.
    pub const fn with_namespace(mut self, namespace: &'a CStr) -> Self {
        self.namespace = ConstCStr::new(namespace);
        self
    }

    /// Extracts the name of the symbol.
    pub const fn name(&self) -> &CStr {
        unsafe { self.name.as_ref() }
    }

    /// Extracts the namespace of the symbol.
    pub const fn namespace(&self) -> &CStr {
        unsafe { self.namespace.as_ref() }
    }
}

impl Debug for DynamicSymbolExport<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DynamicSymbolExport")
            .field("constructor", &self.constructor)
            .field("destructor", &self.destructor)
            .field("version", &self.version)
            .field("name", &self.name())
            .field("namespace", &self.namespace())
            .finish()
    }
}

/// A modifier declaration for a module export.
#[repr(C, i32)]
#[non_exhaustive]
#[derive(Clone)]
pub enum Modifier<'a> {
    Destructor(&'a DestructorModifier),
    Dependency(Info),
    DebugInfo,
}

/// A modifier for an export destructor.
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct DestructorModifier {
    handle: OpaqueHandle<dyn Send + Sync>,
    destructor: unsafe extern "C" fn(handle: OpaqueHandle<dyn Send + Sync>),
}

/// Declaration of a module export.
#[repr(C)]
pub struct Export<'a> {
    pub next: Option<OpaqueHandle<dyn Send + Sync>>,
    pub version: Version,
    pub name: ConstCStr,
    pub description: ConstCStr,
    pub author: ConstCStr,
    pub license: ConstCStr,
    pub parameters: Option<ConstNonNull<Parameter<'a>>>,
    /// # Safety
    ///
    /// Must match the number of parameters pointed to by `parameters`.
    pub unsafe parameters_count: u32,
    pub resources: Option<ConstNonNull<Resource<'a>>>,
    /// # Safety
    ///
    /// Must match the number of resources pointed to by `resources`.
    pub unsafe resources_count: u32,
    pub namespace_imports: Option<ConstNonNull<Namespace<'a>>>,
    /// # Safety
    ///
    /// Must match the number of namespace imports pointed to by `namespace_imports`.
    pub unsafe namespace_imports_count: u32,
    pub symbol_imports: Option<ConstNonNull<SymbolImport<'a>>>,
    /// # Safety
    ///
    /// Must match the number of symbol imports pointed to by `symbol_imports`.
    pub unsafe symbol_imports_count: u32,
    pub symbol_exports: Option<ConstNonNull<SymbolExport<'a>>>,
    /// # Safety
    ///
    /// Must match the number of symbol exports pointed to by `symbol_exports`.
    pub unsafe symbol_exports_count: u32,
    pub dynamic_symbol_exports: Option<ConstNonNull<DynamicSymbolExport<'a>>>,
    /// # Safety
    ///
    /// Must match the number of symbol exports pointed to by `dynamic_symbol_exports`.
    pub unsafe dynamic_symbol_exports_count: u32,
    pub modifiers: Option<ConstNonNull<Modifier<'a>>>,
    /// # Safety
    ///
    /// Must match the number of modifiers pointed to by `modifiers`.
    pub unsafe modifiers_count: u32,
    pub constructor: Option<
        unsafe extern "C" fn(
            instance: &bindings::FimoModuleInstance,
            loading_set: bindings::FimoModuleLoadingSet,
            state: &mut Option<NonNull<()>>,
        ) -> AnyResult,
    >,
    pub destructor: Option<
        unsafe extern "C" fn(instance: &bindings::FimoModuleInstance, state: Option<NonNull<()>>),
    >,
}
