//! Utilities for defining and working with module exports.

use crate::{
    r#async::{EnqueuedFuture, Fallible},
    context::ContextView,
    module::{
        info::Info,
        instance::{GenericInstance, OpaqueInstanceView, Stage0InstanceView, Stage1InstanceView},
        loading_set::LoadingSetView,
        parameters::{
            ParameterAccessGroup, ParameterCast, ParameterData, ParameterRepr, ParameterType,
        },
        symbols::{Share, SymbolInfo, SymbolPointer},
    },
    utils::{ConstNonNull, OpaqueHandle, View},
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

use super::symbols::{AssertSharable, SliceRef, StrRef};

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
    pub read: Option<
        AssertSharable<unsafe extern "C" fn(parameter: ParameterData<'_, ()>, value: NonNull<()>)>,
    >,
    pub write: Option<
        AssertSharable<
            unsafe extern "C" fn(parameter: ParameterData<'_, ()>, value: ConstNonNull<()>),
        >,
    >,
    pub name: StrRef<'a>,
    // Safety: Must match the type provided in `type`.
    pub default_value: DefaultParameterValueUnion,
    _private: PhantomData<()>,
}

sa::assert_impl_all!(Parameter<'_>: Send, Sync);
sa::assert_impl_all!(Parameter<'static>: Share);

impl<'a> Parameter<'a> {
    /// Constructs a new `Parameter`.
    pub const fn new<T: ParameterRepr>(default_value: T, name: &'a CStr) -> Self {
        let name = StrRef::new(name);
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
        read: AssertSharable<
            unsafe extern "C" fn(parameter: ParameterData<'_, ()>, value: NonNull<()>),
        >,
    ) -> Self {
        self.read = Some(read);
        self
    }

    /// Sets a custom write function.
    pub const fn with_write(
        mut self,
        write: AssertSharable<
            unsafe extern "C" fn(parameter: ParameterData<'_, ()>, value: ConstNonNull<()>),
        >,
    ) -> Self {
        self.write = Some(write);
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
    pub path: StrRef<'a>,
    _private: PhantomData<()>,
}

sa::assert_impl_all!(Resource<'_>: Send, Sync);
sa::assert_impl_all!(Resource<'static>: Share);

impl<'a> Resource<'a> {
    /// Constructs a new `Resource`.
    pub const fn new(path: &'a CStr) -> Self {
        Self {
            path: StrRef::new(path),
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
    pub name: StrRef<'a>,
    _private: PhantomData<()>,
}

sa::assert_impl_all!(Namespace<'_>: Send, Sync);
sa::assert_impl_all!(Namespace<'static>: Share);

impl<'a> Namespace<'a> {
    /// Constructs a new `Namespace`.
    pub const fn new(name: &'a CStr) -> Self {
        Self {
            name: StrRef::new(name),
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
    pub version: Version<'a>,
    pub name: StrRef<'a>,
    pub namespace: StrRef<'a>,
    _private: PhantomData<()>,
}

sa::assert_impl_all!(SymbolImport<'_>: Send, Sync);
sa::assert_impl_all!(SymbolImport<'static>: Share);

impl<'a> SymbolImport<'a> {
    /// Constructs a new `SymbolImport`.
    pub const fn new(version: Version<'a>, name: &'a CStr) -> Self {
        Self {
            version,
            name: StrRef::new(name),
            namespace: StrRef::new(c""),
            _private: PhantomData,
        }
    }

    /// Sets the namespace of the symbol.
    pub const fn with_namespace(mut self, namespace: &'a CStr) -> Self {
        self.namespace = StrRef::new(namespace);
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

/// Linkage of an symbol export.
#[repr(i32)]
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SymbolLinkage {
    /// The symbol is visible to other instances and is unique.
    Global,
}

/// Declaration of a static module symbol export.
#[repr(C)]
#[derive(Copy, Clone)]
pub struct SymbolExport<'a> {
    pub symbol: AssertSharable<ConstNonNull<()>>,
    pub linkage: SymbolLinkage,
    pub version: Version<'a>,
    pub name: StrRef<'a>,
    pub namespace: StrRef<'a>,
    _private: PhantomData<()>,
}

sa::assert_impl_all!(SymbolExport<'_>: Send, Sync);
sa::assert_impl_all!(SymbolExport<'static>: Share);

unsafe impl Send for SymbolExport<'_> {}
unsafe impl Sync for SymbolExport<'_> {}

impl<'a> SymbolExport<'a> {
    /// Constructs a new `SymbolExport`.
    pub const fn new<T: ~const SymbolPointer + 'a>(
        symbol: T::Target<'a>,
        version: Version<'a>,
        name: &'a CStr,
        linkage: SymbolLinkage,
    ) -> Self {
        Self {
            symbol: unsafe { AssertSharable::new(T::ptr_from_target(symbol)) },
            linkage,
            version,
            name: StrRef::new(name),
            namespace: StrRef::new(c""),
            _private: PhantomData,
        }
    }

    /// Sets the namespace of the symbol.
    pub const fn with_namespace(mut self, namespace: &'a CStr) -> Self {
        self.namespace = StrRef::new(namespace);
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
            .field("linkage", &self.linkage)
            .field("version", &self.version)
            .field("name", &self.name())
            .field("namespace", &self.namespace())
            .finish()
    }
}

/// Declaration of a static module symbol export.
#[repr(C)]
#[derive(Copy, Clone)]
#[allow(clippy::type_complexity)]
pub struct DynamicSymbolExport<'a> {
    pub constructor: AssertSharable<
        unsafe extern "C" fn(
            instance: Pin<&OpaqueInstanceView<'_>>,
        ) -> EnqueuedFuture<Fallible<NonNull<()>, dyn Share>>,
    >,
    pub destructor: AssertSharable<
        unsafe extern "C" fn(instance: Pin<&OpaqueInstanceView<'_>>, symbol: NonNull<()>),
    >,
    pub linkage: SymbolLinkage,
    pub version: Version<'a>,
    pub name: StrRef<'a>,
    pub namespace: StrRef<'a>,
    _private: PhantomData<()>,
}

sa::assert_impl_all!(DynamicSymbolExport<'_>: Send, Sync);
sa::assert_impl_all!(DynamicSymbolExport<'static>: Share);

impl<'a> DynamicSymbolExport<'a> {
    /// Constructs a new `DynamicSymbolExport`.
    ///
    /// # Safety
    ///
    /// `constructor` must construct an instance of a type that implements [`SymbolPointer`] and
    /// `destructor` must release the instance of the same type.
    #[allow(clippy::type_complexity)]
    pub const unsafe fn new(
        constructor: AssertSharable<
            unsafe extern "C" fn(
                instance: Pin<&OpaqueInstanceView<'_>>,
            ) -> EnqueuedFuture<Fallible<NonNull<()>, dyn Share>>,
        >,
        destructor: AssertSharable<
            unsafe extern "C" fn(instance: Pin<&OpaqueInstanceView<'_>>, symbol: NonNull<()>),
        >,
        version: Version<'a>,
        name: &'a CStr,
        linkage: SymbolLinkage,
    ) -> Self {
        Self {
            constructor,
            destructor,
            linkage,
            version,
            name: StrRef::new(name),
            namespace: StrRef::new(c""),
            _private: PhantomData,
        }
    }

    /// Sets the namespace of the symbol.
    pub const fn with_namespace(mut self, namespace: &'a CStr) -> Self {
        self.namespace = StrRef::new(namespace);
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
#[derive(Debug, Clone)]
pub enum Modifier<'a> {
    Destructor(&'a DestructorModifier<'a>),
    Dependency(Info),
    DebugInfo(&'a DebugInfoModifier),
    InstanceState(&'a InstanceStateModifier),
    StartEvent(&'a StartEventModifier),
    StopEvent(&'a StopEventModifier),
}

sa::assert_impl_all!(Modifier<'_>: Send, Sync);
sa::assert_impl_all!(Modifier<'static>: Share);

/// A modifier for an export destructor.
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct DestructorModifier<'a> {
    pub handle: Option<OpaqueHandle<dyn Send + Sync + Share + 'a>>,
    pub destructor:
        unsafe extern "C" fn(handle: Option<OpaqueHandle<dyn Send + Sync + Share + 'a>>),
    _private: PhantomData<()>,
}

sa::assert_impl_all!(DestructorModifier<'_>: Send, Sync);
sa::assert_impl_all!(DestructorModifier<'static>: Share);

/// A modifier for the debug info of a module.
#[derive(Debug, Copy, Clone)]
pub enum DebugInfoModifier {}

/// A constructor and destructor for the state of a module.
///
/// Can be specified to bind a state to an instance. The constructor will be called before the
/// modules exports are initialized and returning an error will abort the loading of the
/// instance. Inversely, the destructor function will be called after all exports have been
/// deinitialized. May only be specified once.
#[repr(C)]
#[derive(Debug, Copy, Clone)]
#[allow(clippy::type_complexity)]
pub struct InstanceStateModifier {
    pub constructor: AssertSharable<
        unsafe extern "C" fn(
            instance: Pin<&OpaqueInstanceView<'_>>,
            loading_set: LoadingSetView<'_>,
        ) -> EnqueuedFuture<Fallible<Option<NonNull<()>>, dyn Share>>,
    >,
    pub destructor: AssertSharable<
        unsafe extern "C" fn(instance: Pin<&OpaqueInstanceView<'_>>, state: Option<NonNull<()>>),
    >,
    _private: PhantomData<()>,
}

impl InstanceStateModifier {
    /// Constructs a new `InstanceStateModifier`.
    ///
    /// # Safety
    ///
    /// The instance is only partially initialized while the constructor and destructor functions
    /// are called. One must ensure that no uninitialized fields are read.
    #[allow(clippy::type_complexity)]
    pub const unsafe fn new(
        constructor: AssertSharable<
            unsafe extern "C" fn(
                instance: Pin<&OpaqueInstanceView<'_>>,
                loading_set: LoadingSetView<'_>,
            )
                -> EnqueuedFuture<Fallible<Option<NonNull<()>>, dyn Share>>,
        >,
        destructor: AssertSharable<
            unsafe extern "C" fn(
                instance: Pin<&OpaqueInstanceView<'_>>,
                state: Option<NonNull<()>>,
            ),
        >,
    ) -> Self {
        Self {
            constructor,
            destructor,
            _private: PhantomData,
        }
    }
}

sa::assert_impl_all!(InstanceStateModifier: Send, Sync, Share);

/// A listener for the start event of the instance.
///
/// The event will be dispatched immediately after the instance has been loaded. An error will
/// result in the destruction of the instance. May only be specified once.
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct StartEventModifier {
    pub on_event: AssertSharable<
        unsafe extern "C" fn(
            instance: Pin<&OpaqueInstanceView<'_>>,
        ) -> EnqueuedFuture<Fallible<(), dyn Share>>,
    >,
    _private: PhantomData<()>,
}

impl StartEventModifier {
    /// Constructs a new `StartEventModifier`.
    ///
    /// # Safety
    ///
    /// TBD
    pub const unsafe fn new(
        on_event: AssertSharable<
            unsafe extern "C" fn(
                instance: Pin<&OpaqueInstanceView<'_>>,
            ) -> EnqueuedFuture<Fallible<(), dyn Share>>,
        >,
    ) -> Self {
        Self {
            on_event,
            _private: PhantomData,
        }
    }
}

sa::assert_impl_all!(StartEventModifier: Send, Sync, Share);

/// A listener for the stop event of the instance.
///
/// The event will be dispatched immediately before any exports are deinitialized. May only be
/// specified once.
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct StopEventModifier {
    pub on_event: AssertSharable<unsafe extern "C" fn(instance: Pin<&OpaqueInstanceView<'_>>)>,
    _private: PhantomData<()>,
}

impl StopEventModifier {
    /// Constructs a new `StopEventModifier`.
    ///
    /// # Safety
    ///
    /// TBD
    pub const unsafe fn new(
        on_event: AssertSharable<unsafe extern "C" fn(instance: Pin<&OpaqueInstanceView<'_>>)>,
    ) -> Self {
        Self {
            on_event,
            _private: PhantomData,
        }
    }
}

sa::assert_impl_all!(StopEventModifier: Send, Sync, Share);

/// Declaration of a module export.
#[repr(C)]
#[derive(Debug, Copy, Clone)]
#[allow(clippy::type_complexity)]
pub struct Export<'a> {
    pub next: Option<OpaqueHandle<dyn Send + Sync + Share>>,
    pub version: Version<'a>,
    pub name: StrRef<'a>,
    pub description: Option<StrRef<'a>>,
    pub author: Option<StrRef<'a>>,
    pub license: Option<StrRef<'a>>,
    pub parameters: SliceRef<'a, Parameter<'a>>,
    pub resources: SliceRef<'a, Resource<'a>>,
    pub namespace_imports: SliceRef<'a, Namespace<'a>>,
    pub symbol_imports: SliceRef<'a, SymbolImport<'a>>,
    pub symbol_exports: SliceRef<'a, SymbolExport<'a>>,
    pub dynamic_symbol_exports: SliceRef<'a, DynamicSymbolExport<'a>>,
    pub modifiers: SliceRef<'a, Modifier<'a>>,
    _private: PhantomData<&'a ()>,
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
        #[allow(clippy::too_many_arguments, clippy::type_complexity)]
        pub const unsafe fn new(
            name: &'a CStr,
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
                )
            }
        }
    }

    #[doc(hidden)]
    #[allow(clippy::too_many_arguments, clippy::type_complexity)]
    pub const unsafe fn __new_private(
        name: &'a CStr,
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
    ) -> Self {
        let description = match description {
            None => None,
            Some(x) => Some(StrRef::new(x)),
        };
        let author = match author {
            None => None,
            Some(x) => Some(StrRef::new(x)),
        };
        let license = match license {
            None => None,
            Some(x) => Some(StrRef::new(x)),
        };
        let parameters = SliceRef::new(parameters);
        let resources = SliceRef::new(resources);
        let namespace_imports = SliceRef::new(namespace_imports);
        let symbol_imports = SliceRef::new(symbol_imports);
        let symbol_exports = SliceRef::new(symbol_exports);
        let dynamic_symbol_exports = SliceRef::new(dynamic_symbol_exports);
        let modifiers = SliceRef::new(modifiers);

        Self {
            next: None,
            version: ContextView::CURRENT_VERSION,
            name: StrRef::new(name),
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
            _private: PhantomData,
        }
    }
}

sa::assert_impl_all!(Export<'_>: Send, Sync);
sa::assert_impl_all!(Export<'static>: Share);

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
        _linkage: SymbolLinkage,
        _value: <T::Type as SymbolPointer>::Target<'static>,
    ) -> &mut Self {
        self
    }

    /// Adds a static export to the module.
    #[allow(clippy::type_complexity)]
    pub const fn with_dynamic_export<'a, T, E>(
        &mut self,
        _table_name: &str,
        _linkage: SymbolLinkage,
        _init: impl AsyncFn(
            Pin<&'a Stage1InstanceView<'a, InstanceView>>,
        ) -> Result<<T::Type as SymbolPointer>::Target<'a>, E>
        + 'a,
        _deinit: impl Fn(
            Pin<&Stage1InstanceView<'_, InstanceView>>,
            <T::Type as SymbolPointer>::Target<'_>,
        ),
    ) -> &mut Self
    where
        T: SymbolInfo,
        E: Debug + Display,
        InstanceView: 'a,
    {
        #[allow(clippy::mem_forget)]
        std::mem::forget(_init);
        #[allow(clippy::mem_forget)]
        std::mem::forget(_deinit);
        self
    }

    /// Adds a state to the module.
    #[allow(clippy::type_complexity)]
    pub const fn with_state<'a, T, E>(
        &mut self,
        _init: impl AsyncFn(
            Pin<&'a Stage0InstanceView<'a, InstanceView>>,
            LoadingSetView<'a>,
        ) -> Result<NonNull<T>, E>
        + 'a,
        _deinit: fn(Pin<&Stage0InstanceView<'_, InstanceView>>, NonNull<T>),
    ) -> &mut Self
    where
        T: Send + Sync + 'static,
        E: Debug + Display,
        InstanceView: 'a,
    {
        #[allow(clippy::mem_forget)]
        std::mem::forget(_init);
        self
    }

    /// Adds an `on_start` event to the module.
    pub const fn with_on_start_event<'a, E>(
        &mut self,
        _on_event: impl AsyncFn(Pin<&'a InstanceView>) -> Result<(), E> + 'a,
    ) -> &mut Self
    where
        E: Debug + Display,
        InstanceView: 'a,
    {
        #[allow(clippy::mem_forget)]
        std::mem::forget(_on_event);
        self
    }

    /// Adds an `on_stop` event to the module.
    pub const fn with_on_stop_event(&mut self, _on_event: fn(Pin<&InstanceView>)) -> &mut Self {
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
