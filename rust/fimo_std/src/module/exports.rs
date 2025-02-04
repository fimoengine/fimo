//! Utilities for defining and working with module exports.

use crate::{
    context::ContextView,
    error::AnyResult,
    utils::{ConstCStr, ConstNonNull, OpaqueHandle, View},
    module::{
        info::Info,
        instance::{GenericInstance, OpaqueInstanceView, UninitInstanceView},
        loading_set::LoadingSetView,
        parameters::{
            ParameterAccessGroup, ParameterCast, ParameterData, ParameterRepr, ParameterType,
        },
        symbols::{SymbolInfo, SymbolPointer},
    },
    version::Version,
};
use std::{
    ffi::CStr,
    fmt::{Debug, Display, Formatter},
    marker::PhantomData,
    pin::Pin,
    ptr::NonNull,
};

pub use fimo_std_macros::export_module;

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
#[derive(Copy, Clone)]
pub struct Parameter<'a> {
    pub r#type: ParameterType,
    pub read_group: ParameterAccessGroup,
    pub write_group: ParameterAccessGroup,
    pub read: Option<unsafe extern "C" fn(parameter: ParameterData<'_, ()>, value: NonNull<()>)>,
    pub write:
        Option<unsafe extern "C" fn(parameter: ParameterData<'_, ()>, value: ConstNonNull<()>)>,
    pub name: ConstCStr,
    // Safety: Must match the type provided in `type`.
    pub default_value: DefaultParameterValueUnion,
    pub(crate) _private: PhantomData<&'a ()>,
}

sa::assert_impl_all!(Parameter<'_>: Send, Sync);

impl<'a> Parameter<'a> {
    /// Constructs a new `Parameter`.
    pub const fn new<T: ParameterRepr>(default_value: T, name: &'a CStr) -> Self {
        let name = ConstCStr::new(name);
        let r#type = T::TYPE;
        let value = unsafe {
            match r#type {
                ParameterType::U8 => DefaultParameterValueUnion {
                    u8: std::mem::transmute_copy(&default_value),
                },
                ParameterType::U16 => DefaultParameterValueUnion {
                    u16: std::mem::transmute_copy(&default_value),
                },
                ParameterType::U32 => DefaultParameterValueUnion {
                    u32: std::mem::transmute_copy(&default_value),
                },
                ParameterType::U64 => DefaultParameterValueUnion {
                    u64: std::mem::transmute_copy(&default_value),
                },
                ParameterType::I8 => DefaultParameterValueUnion {
                    i8: std::mem::transmute_copy(&default_value),
                },
                ParameterType::I16 => DefaultParameterValueUnion {
                    i16: std::mem::transmute_copy(&default_value),
                },
                ParameterType::I32 => DefaultParameterValueUnion {
                    i32: std::mem::transmute_copy(&default_value),
                },
                ParameterType::I64 => DefaultParameterValueUnion {
                    i64: std::mem::transmute_copy(&default_value),
                },
            }
        };

        Self {
            r#type,
            read_group: ParameterAccessGroup::Private,
            write_group: ParameterAccessGroup::Private,
            read: None,
            write: None,
            name,
            default_value: value,
            _private: PhantomData,
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
        read: Option<unsafe extern "C" fn(parameter: ParameterData<'_, ()>, value: NonNull<()>)>,
    ) -> Self {
        self.read = read;
        self
    }

    /// Sets a custom write function.
    pub const fn with_write(
        mut self,
        write: Option<
            unsafe extern "C" fn(parameter: ParameterData<'_, ()>, value: ConstNonNull<()>),
        >,
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
                .field(
                    "default_value",
                    match self.r#type {
                        ParameterType::U8 => &self.default_value.u8 as &dyn Debug,
                        ParameterType::U16 => &self.default_value.u16 as &dyn Debug,
                        ParameterType::U32 => &self.default_value.u32 as &dyn Debug,
                        ParameterType::U64 => &self.default_value.u64 as &dyn Debug,
                        ParameterType::I8 => &self.default_value.i8 as &dyn Debug,
                        ParameterType::I16 => &self.default_value.i16 as &dyn Debug,
                        ParameterType::I32 => &self.default_value.i32 as &dyn Debug,
                        ParameterType::I64 => &self.default_value.i64 as &dyn Debug,
                    },
                )
                .finish()
        }
    }
}

/// Declaration of a module resource.
#[repr(C)]
#[derive(Copy, Clone)]
pub struct Resource<'a> {
    pub path: ConstCStr,
    pub(crate) _private: PhantomData<&'a [u8]>,
}

sa::assert_impl_all!(Resource<'_>: Send, Sync);

impl<'a> Resource<'a> {
    /// Constructs a new `Resource`.
    pub const fn new(path: &'a CStr) -> Self {
        Self {
            path: ConstCStr::new(path),
            _private: PhantomData,
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
    pub(crate) _private: PhantomData<&'a [u8]>,
}

sa::assert_impl_all!(Namespace<'_>: Send, Sync);

impl<'a> Namespace<'a> {
    /// Constructs a new `Namespace`.
    pub const fn new(name: &'a CStr) -> Self {
        Self {
            name: ConstCStr::new(name),
            _private: PhantomData,
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
    pub(crate) _private: PhantomData<&'a [u8]>,
}

sa::assert_impl_all!(SymbolImport<'_>: Send, Sync);

impl<'a> SymbolImport<'a> {
    /// Constructs a new `SymbolImport`.
    pub const fn new(version: Version, name: &'a CStr) -> Self {
        Self {
            version,
            name: ConstCStr::new(name),
            namespace: ConstCStr::new(c""),
            _private: PhantomData,
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
    pub(crate) _private: PhantomData<&'a [u8]>,
}

sa::assert_impl_all!(SymbolExport<'_>: Send, Sync);

unsafe impl Send for SymbolExport<'_> {}
unsafe impl Sync for SymbolExport<'_> {}

impl<'a> SymbolExport<'a> {
    /// Constructs a new `SymbolExport`.
    pub const fn new<T: ~const SymbolPointer + 'a>(
        symbol: T,
        version: Version,
        name: &'a CStr,
    ) -> Self {
        Self {
            symbol: ConstNonNull::new(symbol.into_opaque_ptr()).unwrap(),
            version,
            name: ConstCStr::new(name),
            namespace: ConstCStr::new(c""),
            _private: PhantomData,
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
        instance: Pin<&OpaqueInstanceView<'_>>,
        symbol: &mut NonNull<()>,
    ) -> AnyResult,
    pub destructor: unsafe extern "C" fn(symbol: NonNull<()>),
    pub version: Version,
    pub name: ConstCStr,
    pub namespace: ConstCStr,
    pub(crate) _private: PhantomData<&'a [u8]>,
}

sa::assert_impl_all!(DynamicSymbolExport<'_>: Send, Sync);

impl<'a> DynamicSymbolExport<'a> {
    /// Constructs a new `DynamicSymbolExport`.
    ///
    /// # Safety
    ///
    /// `constructor` must construct an instance of a type that implements [`SymbolPointer`] and
    /// `destructor` must release the instance of the same type.
    pub const unsafe fn new(
        constructor: unsafe extern "C" fn(
            instance: Pin<&OpaqueInstanceView<'_>>,
            symbol: &mut NonNull<()>,
        ) -> AnyResult,
        destructor: unsafe extern "C" fn(symbol: NonNull<()>),
        version: Version,
        name: &'a CStr,
    ) -> Self {
        Self {
            constructor,
            destructor,
            version,
            name: ConstCStr::new(name),
            namespace: ConstCStr::new(c""),
            _private: PhantomData,
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
    Destructor(&'a DestructorModifier<'a>),
    Dependency(Info),
    DebugInfo,
}

sa::assert_impl_all!(Modifier<'_>: Send, Sync);

/// A modifier for an export destructor.
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct DestructorModifier<'a> {
    pub handle: Option<OpaqueHandle<dyn Send + Sync + 'a>>,
    pub destructor: unsafe extern "C" fn(handle: Option<OpaqueHandle<dyn Send + Sync + 'a>>),
    pub(crate) _private: PhantomData<()>,
}

sa::assert_impl_all!(DestructorModifier<'_>: Send, Sync);

/// Declaration of a module export.
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct Export<'a> {
    pub next: Option<OpaqueHandle<dyn Send + Sync>>,
    pub version: Version,
    pub name: ConstCStr,
    pub description: Option<ConstCStr>,
    pub author: Option<ConstCStr>,
    pub license: Option<ConstCStr>,
    // Safety: Must match the number of parameters pointed to by `parameters`.
    pub parameters: Option<ConstNonNull<Parameter<'a>>>,
    pub parameters_count: u32,
    // Safety: Must match the number of resources pointed to by `resources`.
    pub resources: Option<ConstNonNull<Resource<'a>>>,
    pub resources_count: u32,
    // Safety: Must match the number of namespace imports pointed to by `namespace_imports`.
    pub namespace_imports: Option<ConstNonNull<Namespace<'a>>>,
    pub namespace_imports_count: u32,
    // Safety: Must match the number of symbol imports pointed to by `symbol_imports`.
    pub symbol_imports: Option<ConstNonNull<SymbolImport<'a>>>,
    pub symbol_imports_count: u32,
    // Safety: Must match the number of symbol exports pointed to by `symbol_exports`.
    pub symbol_exports: Option<ConstNonNull<SymbolExport<'a>>>,
    pub symbol_exports_count: u32,
    // Safety: Must match the number of symbol exports pointed to by `dynamic_symbol_exports`.
    pub dynamic_symbol_exports: Option<ConstNonNull<DynamicSymbolExport<'a>>>,
    pub dynamic_symbol_exports_count: u32,
    // Safety: Must match the number of modifiers pointed to by `modifiers`.
    pub modifiers: Option<ConstNonNull<Modifier<'a>>>,
    pub modifiers_count: u32,
    pub constructor: Option<
        unsafe extern "C" fn(
            instance: Pin<&OpaqueInstanceView<'_>>,
            loading_set: LoadingSetView<'_>,
            state: &mut Option<NonNull<()>>,
        ) -> AnyResult,
    >,
    pub destructor: Option<
        unsafe extern "C" fn(instance: Pin<&OpaqueInstanceView<'_>>, state: Option<NonNull<()>>),
    >,
    pub on_start_event:
        Option<unsafe extern "C" fn(instance: Pin<&OpaqueInstanceView<'_>>) -> AnyResult>,
    pub on_stop_event: Option<unsafe extern "C" fn(instance: Pin<&OpaqueInstanceView<'_>>)>,
    pub(crate) _private: PhantomData<&'a ()>,
}

impl<'a> Export<'a> {
    cfg_internal! {
        /// Constructs a new `Export`.
        ///
        /// # Safety
        ///
        /// The behavior is undefined if the contract of the runtime is not upheld. The exact contract
        /// is, as yet, still in flux.
        ///
        /// (Authors wishing to avoid unsafe code may use one of the provided builder types.)
        ///
        /// # Unstable
        ///
        /// **Note**: This is an [unstable API][unstable]. The public API of this type may break
        /// with any semver compatible release. See
        /// [the documentation on unstable features][unstable] for details.
        ///
        /// [unstable]: crate#unstable-features
        #[allow(clippy::too_many_arguments)]
        pub const unsafe fn new(
            name: &CStr,
            description: Option<&'a CStr>,
            author: Option<&'a CStr>,
            license: Option<&'a CStr>,
            parameters: &'a [Parameter<'a>],
            resources: &'a [Resource<'a>],
            namespace_imports: &'a [Namespace<'a>],
            symbol_imports: &'a [SymbolImport<'a>],
            symbol_exports: &'a [SymbolExport<'a>],
            dynamic_symbol_exports: &'a [DynamicSymbolExport<'a>],
            modifiers: &'a [Modifier<'a>],
            constructor: Option<
                unsafe extern "C" fn(
                    instance: Pin<&OpaqueInstanceView<'_>>,
                    loading_set: LoadingSetView<'_>,
                    state: &mut Option<NonNull<()>>,
                ) -> AnyResult,
            >,
            destructor: Option<
                unsafe extern "C" fn(
                    instance: Pin<&OpaqueInstanceView<'_>>,
                    state: Option<NonNull<()>>,
                ),
            >,
            on_start_event: Option<
                unsafe extern "C" fn(instance: Pin<&OpaqueInstanceView<'_>>) -> AnyResult,
            >,
            on_stop_event: Option<unsafe extern "C" fn(instance: Pin<&OpaqueInstanceView<'_>>)>,
        ) -> Self {
            unsafe {
                Self::__new_private(
                    name,
                    description,
                    author,
                    license,
                    parameters,
                    resources,
                    namespace_imports,
                    symbol_imports,
                    symbol_exports,
                    dynamic_symbol_exports,
                    modifiers,
                    constructor,
                    destructor,
                    on_start_event,
                    on_stop_event,
                )
            }
        }
    }

    #[doc(hidden)]
    #[allow(clippy::too_many_arguments)]
    pub const unsafe fn __new_private(
        name: &CStr,
        description: Option<&'a CStr>,
        author: Option<&'a CStr>,
        license: Option<&'a CStr>,
        parameters: &'a [Parameter<'a>],
        resources: &'a [Resource<'a>],
        namespace_imports: &'a [Namespace<'a>],
        symbol_imports: &'a [SymbolImport<'a>],
        symbol_exports: &'a [SymbolExport<'a>],
        dynamic_symbol_exports: &'a [DynamicSymbolExport<'a>],
        modifiers: &'a [Modifier<'a>],
        constructor: Option<
            unsafe extern "C" fn(
                instance: Pin<&OpaqueInstanceView<'_>>,
                loading_set: LoadingSetView<'_>,
                state: &mut Option<NonNull<()>>,
            ) -> AnyResult,
        >,
        destructor: Option<
            unsafe extern "C" fn(
                instance: Pin<&OpaqueInstanceView<'_>>,
                state: Option<NonNull<()>>,
            ),
        >,
        on_start_event: Option<
            unsafe extern "C" fn(instance: Pin<&OpaqueInstanceView<'_>>) -> AnyResult,
        >,
        on_stop_event: Option<unsafe extern "C" fn(instance: Pin<&OpaqueInstanceView<'_>>)>,
    ) -> Self {
        let description = match description {
            None => None,
            Some(x) => Some(ConstCStr::new(x)),
        };
        let author = match author {
            None => None,
            Some(x) => Some(ConstCStr::new(x)),
        };
        let license = match license {
            None => None,
            Some(x) => Some(ConstCStr::new(x)),
        };
        let (parameters, parameters_count) = slice_to_raw_parts(parameters);
        let (resources, resources_count) = slice_to_raw_parts(resources);
        let (namespace_imports, namespace_imports_count) = slice_to_raw_parts(namespace_imports);
        let (symbol_imports, symbol_imports_count) = slice_to_raw_parts(symbol_imports);
        let (symbol_exports, symbol_exports_count) = slice_to_raw_parts(symbol_exports);
        let (dynamic_symbol_exports, dynamic_symbol_exports_count) =
            slice_to_raw_parts(dynamic_symbol_exports);
        let (modifiers, modifiers_count) = slice_to_raw_parts(modifiers);

        Self {
            next: None,
            version: ContextView::CURRENT_VERSION,
            name: ConstCStr::new(name),
            description,
            author,
            license,
            parameters,
            parameters_count,
            resources,
            resources_count,
            namespace_imports,
            namespace_imports_count,
            symbol_imports,
            symbol_imports_count,
            symbol_exports,
            symbol_exports_count,
            dynamic_symbol_exports,
            dynamic_symbol_exports_count,
            modifiers,
            modifiers_count,
            constructor,
            destructor,
            on_start_event,
            on_stop_event,
            _private: PhantomData,
        }
    }
}

const fn slice_to_raw_parts<T>(slice: &[T]) -> (Option<ConstNonNull<T>>, u32) {
    assert!(slice.len() <= u32::MAX as usize);
    match slice.is_empty() {
        true => (None, 0),
        false => (
            unsafe { Some(ConstNonNull::new_unchecked(slice.as_ptr())) },
            slice.len() as u32,
        ),
    }
}

sa::assert_impl_all!(Export<'_>: Send, Sync);

unsafe impl Send for Export<'_> {}
unsafe impl Sync for Export<'_> {}

/// Builder for an [`Export`].
pub struct Builder<InstanceView, OwnedInstance>(PhantomData<fn(InstanceView, OwnedInstance)>)
where
    for<'a> Pin<&'a InstanceView>: GenericInstance + View,
    for<'a> &'a OwnedInstance: GenericInstance;

impl<InstanceView, OwnedInstance> Builder<InstanceView, OwnedInstance>
where
    for<'a> Pin<&'a InstanceView>: GenericInstance + View,
    for<'a> &'a OwnedInstance: GenericInstance,
{
    /// Initializes a new `Builder`.
    pub const fn new(_name: &CStr) -> Self {
        Self(PhantomData)
    }

    /// Adds a description to the module.
    pub const fn with_description(&mut self, _description: &CStr) -> &mut Self {
        self
    }

    /// Adds an author to the module.
    pub const fn with_author(&mut self, _author: &'static CStr) -> &mut Self {
        self
    }

    /// Adds a license to the module.
    pub const fn with_license(&mut self, _license: &'static CStr) -> &mut Self {
        self
    }

    /// Adds a new parameter to the module.
    #[allow(clippy::type_complexity)]
    #[allow(clippy::too_many_arguments)]
    pub const fn with_parameter<T: const ParameterCast>(
        &mut self,
        _table_name: &str,
        _name: &CStr,
        _default_value: T,
        _read_group: Option<ParameterAccessGroup>,
        _write_group: Option<ParameterAccessGroup>,
        _read: Option<fn(ParameterData<'_, T::Repr>) -> T::Repr>,
        _write: Option<fn(ParameterData<'_, T::Repr>, T::Repr)>,
    ) -> &mut Self {
        #[allow(clippy::mem_forget)]
        std::mem::forget(_default_value);
        self
    }

    /// Adds a resource path to the module.
    pub const fn with_resource(&mut self, _table_name: &str, _path: &CStr) -> &mut Self {
        self
    }

    /// Adds a namespace import to the module.
    ///
    /// A namespace may be imported multiple times.
    pub const fn with_namespace(&mut self, _name: &CStr) -> &mut Self {
        self
    }

    /// Adds an import to the module.
    ///
    /// Automatically imports the required namespace.
    pub const fn with_import<T: SymbolInfo>(&mut self, _table_name: &str) -> &mut Self {
        self
    }

    /// Adds a static export to the module.
    pub const fn with_export<T: SymbolInfo>(
        &mut self,
        _table_name: &str,
        _value: T::Type,
    ) -> &mut Self {
        self
    }

    /// Adds a static export to the module.
    #[allow(clippy::type_complexity)]
    pub const fn with_dynamic_export<T, E>(
        &mut self,
        _table_name: &str,
        _init: fn(Pin<&UninitInstanceView<'_, InstanceView>>) -> Result<T::Type, E>,
        _deinit: fn(T::Type),
    ) -> &mut Self
    where
        T: SymbolInfo,
        E: Debug + Display,
    {
        self
    }

    /// Adds a state to the module.
    #[allow(clippy::type_complexity)]
    pub const fn with_state<T, E>(
        &mut self,
        _init: fn(
            Pin<&UninitInstanceView<'_, InstanceView>>,
            LoadingSetView<'_>,
        ) -> Result<NonNull<T>, E>,
        _deinit: fn(Pin<&UninitInstanceView<'_, InstanceView>>, NonNull<T>),
    ) -> &mut Self
    where
        T: Send + Sync + 'static,
        E: Debug + Display,
    {
        self
    }

    pub const fn build(&mut self) {
        panic!("the builder must be consumed by the `#[export_module]` macro")
    }

    #[doc(hidden)]
    pub const fn __private_build(&mut self) -> __PrivateBuildToken {
        __PrivateBuildToken(())
    }
}

#[doc(hidden)]
pub struct __PrivateBuildToken(());
