//! Module subsystem.

use alloc::boxed::Box;
use core::{ffi::CStr, mem::MaybeUninit};

use crate::{
    bindings,
    context::private::SealedContext,
    error::{to_result_indirect_in_place, Error},
};

mod loading_set;
mod module_export;
mod module_info;
mod parameter;
mod symbol;

pub use loading_set::*;
pub use module_export::*;
pub use module_info::*;
pub use parameter::*;
pub use symbol::*;

/// Definition of the module subsystem.
pub trait ModuleSubsystem: SealedContext {
    /// Checks for the presence of a namespace in the module backend.
    ///
    /// A namespace exists, if at least one loaded module exports one symbol in said namespace.
    fn namespace_exists(&self, namespace: &CStr) -> Result<bool, Error>;
}

impl<T> ModuleSubsystem for T
where
    T: SealedContext,
{
    fn namespace_exists(&self, namespace: &CStr) -> Result<bool, Error> {
        // Safety: Either we get an error, or we initialize the module.
        unsafe {
            to_result_indirect_in_place(|error, exists| {
                *error = bindings::fimo_module_namespace_exists(
                    self.share_to_ffi(),
                    namespace.as_ptr(),
                    exists.as_mut_ptr(),
                );
            })
        }
    }
}

/// A handle to a module that is being constructed.
pub struct ConstructorModule<'a, T: Module>(PreModule<'a, T>);

impl<'a, T> ConstructorModule<'a, T>
where
    T: Module,
{
    /// Unwraps the handle, checking that the version of the context that loaded the module is
    /// compatible with the required version.
    pub fn unwrap(self) -> Result<PreModule<'a, T>, Error> {
        self.0.context().check_version()?;
        Ok(self.0)
    }

    /// Unwraps the handle without checking that the version of the context that loaded the module
    /// is compatible.
    ///
    /// # Safety
    ///
    /// The caller must ensure to only use some functionality of the context, if it is supported by
    /// the received version.
    pub unsafe fn unwrap_unchecked(self) -> PreModule<'a, T> {
        self.0
    }
}

impl<'a, T> From<PreModule<'a, T>> for ConstructorModule<'a, T>
where
    T: Module,
{
    fn from(value: PreModule<'a, T>) -> Self {
        Self(value)
    }
}

/// A handle to a module that has not been initialized.
pub type PreModule<'a, T> = GenericModule<
    'a,
    <T as Module>::Parameters,
    <T as Module>::Resources,
    <T as Module>::Imports,
    MaybeUninit<<T as Module>::Exports>,
    MaybeUninit<<T as Module>::Data>,
>;

/// Helper trait for defining module constructors.
pub trait ModuleConstructor<T: Module> {
    /// Constructs the module data.
    ///
    /// The constructor function is allowed to call into the [`ModuleSubsystem`], for instance, to
    /// request more modules.
    fn construct<'a>(
        module: ConstructorModule<'a, T>,
        set: LoadingSet<'_>,
    ) -> Result<&'a mut <T as Module>::Data, Error>;

    /// Destroys the module data.
    ///
    /// This function is not allowed to call into the [`ModuleSubsystem`].
    fn destroy(module: PreModule<'_, T>, data: &mut <T as Module>::Data);
}

/// A marker type indicating no state for a module.
#[repr(transparent)]
#[derive(Debug, Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Default)]
pub struct NoState;

/// Default constructor for modules without any associated state.
#[derive(Debug, Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Default)]
pub struct DefaultConstructor;

impl<T> ModuleConstructor<T> for DefaultConstructor
where
    T: Module<Data: Default>,
{
    fn construct<'a>(
        module: ConstructorModule<'a, T>,
        _set: LoadingSet<'_>,
    ) -> Result<&'a mut T::Data, Error> {
        // Check that the version is compatible.
        let _ = module.unwrap()?;
        // Safety: The pointer is valid.
        unsafe { Ok(&mut *Box::into_raw(Box::default())) }
    }

    fn destroy(_module: PreModule<'_, T>, data: &mut T::Data) {
        // Safety: We were returned the ownership.
        unsafe { drop(Box::from_raw(data)) }
    }
}

/// Exports a new module from the current binary.
#[macro_export]
macro_rules! export_module {
    (
        mod $mod_ident:ident {
            name: $name:literal,
            $(description: $descr:literal,)?
            $(author: $author:literal,)?
            $(license: $license:literal,)?
            $(parameters: { $($param_block:tt)* },)?
            $(resources: { $($res_block:tt)* },)?
            $(namespaces: [ $($ns_block:tt)* ],)?
            $(imports: { $($imports_block:tt)* },)?
            $(exports: { $($exports_block:tt)* },)?
            $(dyn_exports: { $($dyn_exports_block:tt)* },)?
            $(state: $state:ty ,)?
            $(constructor: $constructor:path $(,)? )?
        }$(;)?
    ) => {
        $crate::export_module_private_parameter!(table $mod_ident; $($($param_block)*)?);
        $crate::export_module_private_resources!(table $mod_ident; $($($res_block)*)?);
        $crate::export_module_private_imports!(table $mod_ident; $($($imports_block)*)?);
        $crate::export_module_private_exports!(
            table $mod_ident;
            static $($($exports_block)*)?;
            dynamic $($($dyn_exports_block)*)?
        );

        $crate::paste::paste! {
            #[doc = "Alias for the `" $mod_ident "` module."]
            pub type $mod_ident<'a> = $crate::module::GenericModule<'a,
                [<$mod_ident Parameters>],
                [<$mod_ident Resources>],
                [<$mod_ident Imports>],
                [<$mod_ident Exports>],
                $crate::export_module_private_data!(state $($state)?),
            >;

            #[doc = "Alias for the locked `" $mod_ident "` module."]
            pub type [<$mod_ident Locked>] = $crate::module::GenericLockedModule<
                [<$mod_ident Parameters>],
                [<$mod_ident Resources>],
                [<$mod_ident Imports>],
                [<$mod_ident Exports>],
                $crate::export_module_private_data!(state $($state)?),
            >;

            #[doc = "A marker type for accessing the current [`" $mod_ident "`] instance."]
            pub struct [<$mod_ident Token>];
        }

        const _: () = {
            struct ModuleLock(std::sync::RwLock<*const $crate::bindings::FimoModule>);
            // Safety:
            unsafe impl Send for ModuleLock {}
            // Safety:
            unsafe impl Sync for ModuleLock {}

            static CURRENT: ModuleLock = ModuleLock(std::sync::RwLock::new(std::ptr::null()));

            static INIT_COUNTER: core::sync::atomic::AtomicUsize = core::sync::atomic::AtomicUsize::new(0);
            const INIT_STEPS: usize = const {
                $crate::export_module_private_exports!(
                    count_dynamic { $($($dyn_exports_block)*)? }
                )
            };

            $crate::paste::paste! {
                impl [<$mod_ident Token>]{
                    pub fn with_current<F, R>(f: F) -> R
                    where
                        F: for<'ctx> FnOnce(&'ctx [<$mod_ident Locked>]) -> R,
                    {
                        use $crate::module::Module;
                        let module = Self::with_lock(|module| {
                            module.lock_module_strong()
                        }).expect("could not lock the module");

                        let context = module.context();
                        $crate::panic::with_panic_context(context, |_| f(&module))
                    }

                    /// Acquires an instance to the current module.
                    ///
                    /// # Panics
                    ///
                    /// This function panics if the module is not fully loaded.
                    ///
                    /// # Safety
                    ///
                    /// May only be called be a symbol exported from the module, or it is otherwise
                    /// known that the module can not be unloaded.
                    pub unsafe fn with_current_unlocked<F, R>(f: F) -> R
                    where
                        F: for<'ctx> FnOnce($mod_ident<'ctx>) -> R,
                    {
                        use $crate::module::Module;
                        let module: $mod_ident<'_> = Self::with_lock(|module| {
                            // Safety: The caller ensures that the module is locked.
                            unsafe { std::mem::transmute(module) }
                        });

                        let context = module.context();
                        $crate::panic::with_panic_context(context, |_| f(module))
                    }

                    fn with_lock<F, R>(f: F) -> R
                    where
                        F: for<'ctx> FnOnce($mod_ident<'ctx>) -> R,
                    {
                        let init_counter = INIT_COUNTER.load(core::sync::atomic::Ordering::Relaxed);
                        if init_counter != INIT_STEPS {
                            panic!("the module exports are not initialized")
                        }

                        let guard = CURRENT.0.read().unwrap();
                        if guard.is_null() {
                            panic!("the module is not initialized");
                        }

                        // Safety:
                        unsafe {
                            let module = <$mod_ident<'_> as $crate::ffi::FFITransferable<*const $crate::bindings::FimoModule>>::from_ffi(*guard);
                            f(module)
                        }
                    }
                }
            }

            const fn build_export() -> $crate::bindings::FimoModuleExport {
                let name = $crate::optional_c_str!($name);
                let description = $crate::optional_c_str!($($descr)?);
                let author = $crate::optional_c_str!($($author)?);
                let license = $crate::optional_c_str!($($license)?);
                let (parameters, parameters_count) = $crate::export_module_private_parameter!(
                    ptr $mod_ident; { $($($param_block)*)? }
                );
                let (resources, resources_count) = $crate::export_module_private_resources!(
                    ptr { $($($res_block)*)? }
                );
                let (namespace_imports, namespace_imports_count) = $crate::export_module_private_ns!(
                    ptr [ $($($ns_block)*)? ]
                );
                let (symbol_imports, symbol_imports_count) = $crate::export_module_private_imports!(
                    ptr { $($($imports_block)*)? }
                );
                let (symbol_exports, symbol_exports_count) = $crate::export_module_private_exports!(
                    static_ptr { $($($exports_block)*)? }
                );
                let (dynamic_symbol_exports, dynamic_symbol_exports_count) = $crate::export_module_private_exports!(
                    dynamic_ptr $mod_ident; { $($($dyn_exports_block)*)? }
                );
                let module_constructor = $crate::export_module_private_data!(
                    constructor $mod_ident $($constructor)?
                );
                let module_destructor = $crate::export_module_private_data!(
                    destructor $mod_ident $($constructor)?
                );

                $crate::bindings::FimoModuleExport {
                    type_: $crate::bindings::FimoStructType::FIMO_STRUCT_TYPE_MODULE_EXPORT,
                    next: core::ptr::null(),
                    export_abi: $crate::bindings::FIMO_MODULE_EXPORT_ABI
                        as $crate::bindings::FimoI32,
                    name,
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
                    modifiers: core::ptr::null(),
                    modifiers_count: 0,
                    module_constructor,
                    module_destructor,
                }
            }

            #[allow(dead_code)]
            #[repr(transparent)]
            struct Wrapper(&'static $crate::bindings::FimoModuleExport);
            // Safety: A export is `Send` and `Sync`.
            unsafe impl Send for Wrapper {}
            // Safety: A export is `Send` and `Sync`.
            unsafe impl Sync for Wrapper {}

            #[used]
            #[cfg_attr(windows, link_section = "fi_mod$u")]
            #[cfg_attr(
                all(unix, target_vendor = "apple"),
                link_section = "__DATA,fimo_module"
            )]
            #[cfg_attr(all(unix, not(target_vendor = "apple")), link_section = "fimo_module")]
            static EXPORT: Wrapper = Wrapper(&build_export());
        };

        // For ELF targets the linker garbage collection tends to
        // remove our custom section. On the C/C++ side, we can
        // use the `retain` attribute to force the linker to keep
        // the section. As a workaround, we can keep the section
        // alive by adding a relocation.
        #[cfg(all(unix, not(target_vendor = "apple")))]
        core::arch::global_asm!(
            ".pushsection .init_array,\"aw\",%init_array",
            ".reloc ., BFD_RELOC_NONE, fimo_module",
            ".popsection"
        );
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! optional_c_str {
    () => {{
        core::ptr::null()
    }};
    ($literal:literal) => {{
        let x: &'static str = core::concat!($literal, '\0');
        x.as_ptr().cast()
    }};
}

#[doc(hidden)]
#[macro_export]
macro_rules! export_module_private_parameter {
    (ptr $mod_ident:ident; { $($block:tt)* }) => {{
        const X: &[$crate::bindings::FimoModuleParamDecl] =
            $crate::export_module_private_parameter!($mod_ident; $($block)*);
        if X.is_empty() {
            (core::ptr::null(), 0)
        } else {
            (X.as_ptr(), X.len() as u32)
        }
    }};
    (default_type u8) => {
        $crate::bindings::FimoModuleParamType::FIMO_MODULE_PARAM_TYPE_U8
    };
    (default_type u16) => {
        $crate::bindings::FimoModuleParamType::FIMO_MODULE_PARAM_TYPE_U16
    };
    (default_type u32) => {
        $crate::bindings::FimoModuleParamType::FIMO_MODULE_PARAM_TYPE_U32
    };
    (default_type u64) => {
        $crate::bindings::FimoModuleParamType::FIMO_MODULE_PARAM_TYPE_U64
    };
    (default_type i8) => {
        $crate::bindings::FimoModuleParamType::FIMO_MODULE_PARAM_TYPE_I8
    };
    (default_type i16) => {
        $crate::bindings::FimoModuleParamType::FIMO_MODULE_PARAM_TYPE_I16
    };
    (default_type i32) => {
        $crate::bindings::FimoModuleParamType::FIMO_MODULE_PARAM_TYPE_I32
    };
    (default_type i64) => {
        $crate::bindings::FimoModuleParamType::FIMO_MODULE_PARAM_TYPE_I64
    };
    (default_value u8 $x:literal) => {
        $crate::bindings::FimoModuleParamDecl__bindgen_ty_1 { u8_: $x }
    };
    (default_value u16 $x:literal) => {
        $crate::bindings::FimoModuleParamDecl__bindgen_ty_1 { u16_: $x }
    };
    (default_value u32 $x:literal) => {
        $crate::bindings::FimoModuleParamDecl__bindgen_ty_1 { u32_: $x }
    };
    (default_value u64 $x:literal) => {
        $crate::bindings::FimoModuleParamDecl__bindgen_ty_1 { u64_: $x }
    };
    (default_value i8 $x:literal) => {
        $crate::bindings::FimoModuleParamDecl__bindgen_ty_1 { i8_: $x }
    };
    (default_value i16 $x:literal) => {
        $crate::bindings::FimoModuleParamDecl__bindgen_ty_1 { u16_: $x }
    };
    (default_value i32 $x:literal) => {
        $crate::bindings::FimoModuleParamDecl__bindgen_ty_1 { u32_: $x }
    };
    (default_value i64 $x:literal) => {
        $crate::bindings::FimoModuleParamDecl__bindgen_ty_1 { u64_: $x }
    };
    (param_type u8) => {
        u8
    };
    (param_type u16) => {
        u16
    };
    (param_type u32) => {
        u32
    };
    (param_type u64) => {
        u64
    };
    (param_type i8) => {
        i8
    };
    (param_type i16) => {
        i16
    };
    (param_type i32) => {
        i32
    };
    (param_type i64) => {
        i64
    };
    (param_type $tt:tt $ty:ty) => {
        $ty
    };
    (group) => {
        $crate::bindings::FimoModuleParamAccess::FIMO_MODULE_PARAM_ACCESS_PRIVATE
    };
    (group public) => {
        $crate::bindings::FimoModuleParamAccess::FIMO_MODULE_PARAM_ACCESS_PUBLIC
    };
    (group dependency) => {
        $crate::bindings::FimoModuleParamAccess::FIMO_MODULE_PARAM_ACCESS_DEPENDENCY
    };
    (group private) => {
        $crate::bindings::FimoModuleParamAccess::FIMO_MODULE_PARAM_ACCESS_PRIVATE
    };
    (getter $mod_ident:ident;) => {
        Some($crate::bindings::fimo_module_param_get_inner as _)
    };
    (getter $mod_ident:ident; $x:ident) => {{
        extern "C" fn getter(
            module: *const $crate::bindings::FimoModule,
            value: *mut core::ffi::c_void,
            type_: *mut $crate::bindings::FimoModuleParamType,
            data: *const $crate::bindings::FimoModuleParamData,
        ) {
            // Safety:
            unsafe {
                $crate::module::c_ffi::get_param::<$mod_ident<'_>, _>(module, value, type_, data, $x)
            }
        }

        Some(getter as _)
    }};
    (setter $mod_ident:ident;) => {
        Some($crate::bindings::fimo_module_param_set_inner as _)
    };
    (setter $mod_ident:ident; $x:ident) => {{
        extern "C" fn setter(
            module: *const $crate::bindings::FimoModule,
            value: *const core::ffi::c_void,
            type_: $crate::bindings::FimoModuleParamType,
            data: *mut $crate::bindings::FimoModuleParamData,
        ) {
            // Safety:
            unsafe {
                $crate::module::c_ffi::set_param::<$mod_ident<'_>, _>(module, value, type_, data, $x)
            }
        }

        Some(setter as _)
    }};
    ($mod_ident:ident; $( $name:ident: {
        default: $default_ty:ident ( $default:literal ),
        $(read_group: $read:ident,)?
        $(write_group: $write:ident,)?
        $(getter: $getter:ident,)?
        $(setter: $setter:ident,)?
        $(override: $param_ty:ty,)?
    }),* $(,)?) => {
        &[
            $(
                $crate::bindings::FimoModuleParamDecl {
                    type_: $crate::export_module_private_parameter!(default_type $default_ty),
                    read_access: $crate::export_module_private_parameter!(group $($read)?),
                    write_access: $crate::export_module_private_parameter!(group $($write)?),
                    setter: $crate::export_module_private_parameter!(setter $mod_ident; $($setter)?),
                    getter: $crate::export_module_private_parameter!(getter $mod_ident; $($getter)?),
                    name: {
                        let x: &'static str = core::concat!(core::stringify!($name), '\0');
                        x.as_ptr().cast()
                    },
                    default_value: $crate::export_module_private_parameter!(default_value $default_ty $default),
                }
            ),*
        ]
    };
    (table $mod_ident:ident; $( $name:ident: {
        default: $default_ty:ident ( $default:literal ),
        $(read_group: $read:ident,)?
        $(write_group: $write:ident,)?
        $(getter: $getter:ident,)?
        $(setter: $setter:ident,)?
        $(override: $param_ty:ty,)?
    }),* $(,)?) => {
        $crate::paste::paste! {
            #[repr(C)]
            #[doc = "Parameter table for the `" $mod_ident "` module"]
            pub struct [<$mod_ident Parameters>] {
                $(
                    $name: $crate::module::Parameter<
                        'static,
                        $crate::export_module_private_parameter!(param_type $default_ty $($param_ty)?)
                    >,
                )*
            }

            impl [<$mod_ident Parameters>] {
                $(
                    #[doc = "Fetches the `" $name "` parameter"]
                    pub fn $name(&self) -> &$crate::module::Parameter<
                        '_,
                        $crate::export_module_private_parameter!(param_type $default_ty $($param_ty)?)
                    > {
                        &self.$name
                    }
                )*
            }
        }
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! export_module_private_resources {
    (ptr { $($block:tt)* }) => {{
        const X: &[$crate::bindings::FimoModuleResourceDecl] =
            $crate::export_module_private_resources!($($block)*);
        if X.is_empty() {
            (core::ptr::null(), 0)
        } else {
            (X.as_ptr(), X.len() as u32)
        }
    }};
    ($(
        $name:ident: $path:literal
    ),* $(,)?) => {
        &[
            $(
                $crate::bindings::FimoModuleResourceDecl {
                    path: {
                        let x: &'static str = core::concat!($path, '\0');
                        x.as_ptr().cast()
                    }
                }
            ),*
        ]
    };
    (table $mod_type:ty; $(
        $name:ident: $path:literal
    ),* $(,)?) => {
        $crate::paste::paste! {
            #[repr(C)]
            #[doc = "Resource table for the `" $mod_type "` module"]
            pub struct [<$mod_type Resources>] {
                $(
                    $name: &'static core::ffi::c_char,
                )*
            }

            impl [<$mod_type Resources>] {
                $(
                    #[doc = "Fetches the `" $name "` resource path"]
                    pub fn $name(&self) -> &core::ffi::CStr {
                        // Safety: All pointers are non-null and end with a '\0'.
                        unsafe {
                            core::ffi::CStr::from_ptr(self.$name as *const i8)
                        }
                    }
                )*
            }
        }
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! export_module_private_ns {
    (ptr [$($block:tt)*] ) => {{
        const X: &[$crate::bindings::FimoModuleNamespaceImport] =
            $crate::export_module_private_ns!($($block)*);
        if X.is_empty() {
            (core::ptr::null(), 0)
        } else {
            (X.as_ptr(), X.len() as u32)
        }
    }};
    ($(
        $ns:path
    ),* $(,)?) => {
        &[
            $(
                $crate::bindings::FimoModuleNamespaceImport {
                    name: {
                        let x: &'static core::ffi::CStr = <$ns as $crate::module::NamespaceItem>::NAME;
                        x.as_ptr().cast()
                    }
                }
            ),*
        ]
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! export_module_private_imports {
    (ptr { $($block:tt)* }) => {{
        const X: &[$crate::bindings::FimoModuleSymbolImport] =
            $crate::export_module_private_imports!($($block)*);
        if X.is_empty() {
            (core::ptr::null(), 0)
        } else {
            (X.as_ptr(), X.len() as u32)
        }
    }};
    ($(
        $name:ident: $import:path
    ),* $(,)?) => {
        &[
            $(
                $crate::bindings::FimoModuleSymbolImport {
                    version: {
                        let x = <$import as $crate::module::SymbolItem>::VERSION;
                        $crate::module::c_ffi::extract_version(x)
                    },
                    name: {
                        let x: &'static core::ffi::CStr = <$import as $crate::module::SymbolItem>::NAME;
                        x.as_ptr().cast()
                    },
                    ns: {
                        let x: &'static core::ffi::CStr =
                            <<$import as $crate::module::SymbolItem>::Namespace
                                as $crate::module::NamespaceItem>::NAME;
                        x.as_ptr().cast()
                    },
                }
            ),*
        ]
    };
    (table $mod_ident:ident; $(
        $name:ident: $import:path
    ),* $(,)?) => {
        $crate::paste::paste! {
            #[repr(C)]
            #[doc = "Import table for the `" $mod_ident "` module"]
            pub struct [<$mod_ident Imports>] {
                $(
                    $name: $crate::module::Symbol<
                        'static,
                        <$import as $crate::module::SymbolItem>::Type
                    >,
                )*
            }

            impl [<$mod_ident Imports>] {
                $(
                    #[doc = "Fetches the `" $name "` import symbol"]
                    pub fn $name(&self) -> &'_ <$import as $crate::module::SymbolItem>::Type {
                        &*self.$name
                    }
                )*
            }
        }
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! export_module_private_exports {
    (count_dynamic { $($block:tt)* }) => {
        $crate::export_module_private_exports!(count_dynamic_inner $($block)*)
    };
    (count_dynamic_inner $(,)?) => { 0 };
    (count_dynamic_inner $name:ident: $export:path $(, $($rest:tt)*)?) => {
        1 + $($crate::export_module_private_exports!(count_dynamic_inner $($rest)*))?
    };
    (static_ptr { $($block:tt)* }) => {{
        const X: &[$crate::bindings::FimoModuleSymbolExport] =
            $crate::export_module_private_exports!(static $($block)*);
        if X.is_empty() {
            (core::ptr::null(), 0)
        } else {
            (X.as_ptr(), X.len() as u32)
        }
    }};
    (dynamic_ptr $mod_ident:ident;  { $($block:tt)* }) => {{
        const X: &[$crate::bindings::FimoModuleDynamicSymbolExport] =
            $crate::export_module_private_exports!(dynamic $mod_ident; $($block)*);
        if X.is_empty() {
            (core::ptr::null(), 0)
        } else {
            (X.as_ptr(), X.len() as u32)
        }
    }};
    (static $($name:ident: $export:path = $expr:expr),* $(,)?) => {
        &[
            $(
                $crate::bindings::FimoModuleSymbolExport {
                    symbol: {
                        let x: &'static <$export as $crate::module::SymbolItem>::Type = $expr;
                        core::ptr::from_ref(x).cast()
                    },
                    version: {
                        let x = <$export as $crate::module::SymbolItem>::VERSION;
                        $crate::module::c_ffi::extract_version(x)
                    },
                    name: {
                        let x: &'static core::ffi::CStr = <$export as $crate::module::SymbolItem>::NAME;
                        x.as_ptr().cast()
                    },
                    ns: {
                        let x: &'static core::ffi::CStr =
                            <<$export as $crate::module::SymbolItem>::Namespace
                                as $crate::module::NamespaceItem>::NAME;
                        x.as_ptr().cast()
                    },
                }
            ),*
        ]
    };
    (dynamic $mod_ident:ident; $($name:ident: $export:path),* $(,)?) => {{
        unsafe extern "C" fn construct_dynamic_symbol<T, S>(
            module: *const $crate::bindings::FimoModule,
            symbol: *mut *mut core::ffi::c_void,
        ) -> $crate::bindings::FimoResult
        where
            T: $crate::module::Module,
            S: $crate::module::DynamicExport<T>,
        {
            let mut guard = match CURRENT.0.write() {
                Ok(x) => x,
                Err(e) => return $crate::error::Error::new(e).into_error(),
            };
            if guard.is_null() {
                return <$crate::error::Error>::from_string(c"module pointer not set").into_error();
            }

            // Safety:
            unsafe {
                match $crate::module::c_ffi::construct_dynamic_symbol::<T, S>(module, symbol) {
                    Ok(_) => {
                        use $crate::ffi::FFITransferable;
                        INIT_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                        Result::<_, $crate::error::Error>::Ok(()).into_ffi()
                    }
                    Err(e) => e.into_error()
                }
            }
        }

        unsafe extern "C" fn destroy_dynamic_symbol<T, S>(symbol: *mut core::ffi::c_void)
        where
            T: $crate::module::Module,
            S: $crate::module::DynamicExport<T>,
        {
            let mut guard = match CURRENT.0.write() {
                Ok(x) => x,
                Err(_e) => std::process::abort(),
            };
            if guard.is_null() {
                std::process::abort();
            }
            $crate::module::c_ffi::destroy_dynamic_symbol::<T, S>(symbol);
            INIT_COUNTER.fetch_sub(1, std::sync::atomic::Ordering::Relaxed);
        }

        &[
            $(
                $crate::bindings::FimoModuleDynamicSymbolExport {
                    constructor: Some(construct_dynamic_symbol::<$mod_ident<'_>, $export> as _),
                    destructor: Some(destroy_dynamic_symbol::<$mod_ident<'_>, $export> as _),
                    version: {
                        let x = <<$export as $crate::module::DynamicExport<$mod_ident<'_>>>::Item as $crate::module::SymbolItem>::VERSION;
                        $crate::module::c_ffi::extract_version(x)
                    },
                    name: {
                        let x: &'static core::ffi::CStr = <<$export as $crate::module::DynamicExport<$mod_ident<'_>>>::Item as $crate::module::SymbolItem>::NAME;
                        x.as_ptr().cast()
                    },
                    ns: {
                        let x: &'static core::ffi::CStr =
                            <<<$export as $crate::module::DynamicExport<$mod_ident<'_>>>::Item as $crate::module::SymbolItem>::Namespace
                                as $crate::module::NamespaceItem>::NAME;
                        x.as_ptr().cast()
                    },
                }
            ),*
        ]
    }};
    (table $mod_ident:ident;
        static $($s_name:ident: $s_export:path = $s_expr:expr),* $(,)?;
        dynamic $($d_name:ident: $d_export:path),* $(,)?) => {
        $crate::paste::paste! {
            #[repr(C)]
            #[doc = "Export table for the `" $mod_ident "` module"]
            pub struct [<$mod_ident Exports>] {
                $(
                    $s_name: $crate::module::Symbol<
                        'static,
                        <$s_export as $crate::module::SymbolItem>::Type
                    >,
                )*
                $(
                    $d_name: $crate::module::Symbol<
                        'static,
                        <<$d_export as $crate::module::DynamicExport<$mod_ident<'static>>>::Item as $crate::module::SymbolItem>::Type
                    >,
                )*
            }

            impl [<$mod_ident Exports>] {
                $(
                    #[doc = "Fetches the `" $s_name "` import symbol"]
                    pub fn $s_name(&self) -> &'_ <$s_export as $crate::module::SymbolItem>::Type {
                        &*self.$s_name
                    }
                )*
                $(
                    #[doc = "Fetches the `" $d_name "` import symbol"]
                    pub fn $d_name(&self) -> &'_ <<$d_export as $crate::module::DynamicExport<$mod_ident<'_>>>::Item as $crate::module::SymbolItem>::Type {
                        &*self.$d_name
                    }
                )*
            }
        }
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! export_module_private_data {
    (state) => {
        $crate::module::NoState
    };
    (state $state:ty) => {
        $state
    };
    (constructor_path) => {
        $crate::module::DefaultConstructor
    };
    (constructor_path $constructor:path) => {
        $constructor
    };
    (constructor $mod_ident:ident $($constructor:path)?) => {{
        unsafe extern "C" fn construct_module<T, C>(
            module: *const $crate::bindings::FimoModule,
            set: *mut $crate::bindings::FimoModuleLoadingSet,
            data: *mut *mut core::ffi::c_void,
        ) -> $crate::bindings::FimoResult
        where
            T: $crate::module::Module,
            C: $crate::module::ModuleConstructor<T>,
        {
            let mut guard = match CURRENT.0.write() {
                Ok(x) => x,
                Err(e) => return $crate::error::Error::new(e).into_error(),
            };
            if !guard.is_null() {
                return <$crate::error::Error>::EBUSY.into_error();
            }

            // Safety:
            unsafe {
                match $crate::module::c_ffi::construct_module::<T, C>(module, set, data) {
                    Ok(_) => {
                        use $crate::ffi::FFITransferable;
                        *guard = module;
                        Result::<_, $crate::error::Error>::Ok(()).into_ffi()
                    },
                    Err(e) => e.into_error(),
                }
            }
        }

        type Constructor = $crate::export_module_private_data!(constructor_path $($constructor)?);
        Some(construct_module::<$mod_ident<'_>, Constructor> as _)
    }};
    (destructor $mod_ident:ident $($constructor:path)?) => {{
        unsafe extern "C" fn destroy_module<T, C>(
            module: *const $crate::bindings::FimoModule,
            data: *mut core::ffi::c_void,
        ) where
            T: $crate::module::Module,
            C: $crate::module::ModuleConstructor<T>,
        {
            let mut guard = match CURRENT.0.write() {
                Ok(x) => x,
                Err(_e) => std::process::abort(),
            };

            // Safety:
            unsafe {
                $crate::module::c_ffi::destroy_module::<T, C>(module, data);
                *guard = std::ptr::null();
            }
        }

        type Constructor = $crate::export_module_private_data!(constructor_path $($constructor)?);
        Some(destroy_module::<$mod_ident<'_>, Constructor> as _)
    }};
}

#[doc(hidden)]
pub mod c_ffi {
    use crate::{
        bindings, error,
        error::Error,
        ffi::FFITransferable,
        module::{
            DynamicExport, LoadingSet, Module, ModuleConstructor, ParameterType, ParameterValue,
            PartialModule, PreModule,
        },
        version::Version,
    };
    use core::cell::UnsafeCell;

    pub const fn extract_version(version: Version) -> bindings::FimoVersion {
        version.0
    }

    pub unsafe extern "C" fn get_param<T, F>(
        module: *const bindings::FimoModule,
        value: *mut core::ffi::c_void,
        type_: *mut bindings::FimoModuleParamType,
        data: *mut bindings::FimoModuleParamData,
        f: F,
    ) -> bindings::FimoResult
    where
        T: Module,
        F: FnOnce(&T, &UnsafeCell<bindings::FimoModuleParamData>) -> Result<ParameterValue, Error>,
    {
        crate::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            // Safety:
            unsafe {
                let module: &T = &*module.cast();
                let data = &*data.cast::<UnsafeCell<bindings::FimoModuleParamData>>();
                let context = module.context();
                match crate::panic::with_panic_context(context, |_| f(module, data)) {
                    Ok(x) => {
                        use bindings::FimoModuleParamType;
                        match x {
                            ParameterValue::U8(x) => {
                                core::ptr::write(value.cast(), x);
                                core::ptr::write(
                                    type_,
                                    FimoModuleParamType::FIMO_MODULE_PARAM_TYPE_U8,
                                );
                            }
                            ParameterValue::U16(x) => {
                                core::ptr::write(value.cast(), x);
                                core::ptr::write(
                                    type_,
                                    FimoModuleParamType::FIMO_MODULE_PARAM_TYPE_U16,
                                );
                            }
                            ParameterValue::U32(x) => {
                                core::ptr::write(value.cast(), x);
                                core::ptr::write(
                                    type_,
                                    FimoModuleParamType::FIMO_MODULE_PARAM_TYPE_U32,
                                );
                            }
                            ParameterValue::U64(x) => {
                                core::ptr::write(value.cast(), x);
                                core::ptr::write(
                                    type_,
                                    FimoModuleParamType::FIMO_MODULE_PARAM_TYPE_U64,
                                );
                            }
                            ParameterValue::I8(x) => {
                                core::ptr::write(value.cast(), x);
                                core::ptr::write(
                                    type_,
                                    FimoModuleParamType::FIMO_MODULE_PARAM_TYPE_I8,
                                );
                            }
                            ParameterValue::I16(x) => {
                                core::ptr::write(value.cast(), x);
                                core::ptr::write(
                                    type_,
                                    FimoModuleParamType::FIMO_MODULE_PARAM_TYPE_I16,
                                );
                            }
                            ParameterValue::I32(x) => {
                                core::ptr::write(value.cast(), x);
                                core::ptr::write(
                                    type_,
                                    FimoModuleParamType::FIMO_MODULE_PARAM_TYPE_I32,
                                );
                            }
                            ParameterValue::I64(x) => {
                                core::ptr::write(value.cast(), x);
                                core::ptr::write(
                                    type_,
                                    FimoModuleParamType::FIMO_MODULE_PARAM_TYPE_I64,
                                );
                            }
                        }
                        Ok(())
                    }
                    Err(x) => Err(x),
                }
            }
        }))
        .map_err(Into::into)
        .flatten()
        .into_ffi()
    }

    pub unsafe extern "C" fn set_param<T, F>(
        module: *const bindings::FimoModule,
        value: *const core::ffi::c_void,
        type_: bindings::FimoModuleParamType,
        data: *mut bindings::FimoModuleParamData,
        f: F,
    ) -> bindings::FimoResult
    where
        T: Module,
        F: FnOnce(&T, ParameterValue, &UnsafeCell<bindings::FimoModuleParamData>) -> error::Result,
    {
        crate::panic::catch_unwind(|| {
            // Safety:
            unsafe {
                let module: &T = &*module.cast();
                let data = &*data.cast::<UnsafeCell<bindings::FimoModuleParamData>>();
                let type_ = match ParameterType::try_from(type_) {
                    Ok(x) => x,
                    Err(e) => return Err(e),
                };
                let value = match type_ {
                    ParameterType::U8 => ParameterValue::U8(core::ptr::read(value.cast())),
                    ParameterType::U16 => ParameterValue::U16(core::ptr::read(value.cast())),
                    ParameterType::U32 => ParameterValue::U32(core::ptr::read(value.cast())),
                    ParameterType::U64 => ParameterValue::U64(core::ptr::read(value.cast())),
                    ParameterType::I8 => ParameterValue::I8(core::ptr::read(value.cast())),
                    ParameterType::I16 => ParameterValue::I16(core::ptr::read(value.cast())),
                    ParameterType::I32 => ParameterValue::I32(core::ptr::read(value.cast())),
                    ParameterType::I64 => ParameterValue::I64(core::ptr::read(value.cast())),
                };
                let context = module.context();
                crate::panic::with_panic_context(context, |_| f(module, value, data))
            }
        })
        .map_err(Into::into)
        .flatten()
        .into_ffi()
    }

    pub unsafe fn construct_dynamic_symbol<T, S>(
        module: *const bindings::FimoModule,
        symbol: *mut *mut core::ffi::c_void,
    ) -> Result<(), Error>
    where
        T: Module,
        S: DynamicExport<T>,
    {
        crate::panic::catch_unwind(|| {
            // Safety: The function is only called internally, where we know the type of the module.
            unsafe {
                let module = PartialModule::<'_, T>::from_ffi(module);
                let context = module.context();
                crate::panic::with_panic_context(context, |_| match S::construct(module) {
                    Ok(data) => {
                        let data = core::ptr::from_mut(data).cast();
                        core::ptr::write(symbol, data);
                        Ok(())
                    }
                    Err(e) => Err(e),
                })
            }
        })
        .map_err(Into::into)
        .flatten()
    }

    pub unsafe fn destroy_dynamic_symbol<T, S>(symbol: *mut core::ffi::c_void)
    where
        T: Module,
        S: DynamicExport<T>,
    {
        crate::panic::abort_on_panic(|| {
            // Safety: The function is only called internally,
            // where we know the type of the symbol.
            unsafe {
                S::destroy(&mut *symbol.cast());
            }
        });
    }

    pub unsafe fn construct_module<T, C>(
        module: *const bindings::FimoModule,
        set: *mut bindings::FimoModuleLoadingSet,
        data: *mut *mut core::ffi::c_void,
    ) -> Result<(), Error>
    where
        T: Module,
        C: ModuleConstructor<T>,
    {
        crate::panic::catch_unwind(|| {
            // Safety: See above.
            unsafe {
                let module = PreModule::<T>::from_ffi(module);
                let set = LoadingSet::from_ffi(set);
                let context = module.context();
                crate::panic::with_panic_context(context, |_| {
                    match C::construct(module.into(), set) {
                        Ok(v) => {
                            core::ptr::write(data, core::ptr::from_mut(v).cast());
                            Ok(())
                        }
                        Err(x) => Err(x),
                    }
                })
            }
        })
        .map_err(Into::into)
        .flatten()
    }

    pub unsafe fn destroy_module<T, C>(
        module: *const bindings::FimoModule,
        data: *mut core::ffi::c_void,
    ) where
        T: Module,
        C: ModuleConstructor<T>,
    {
        crate::panic::abort_on_panic(|| {
            // Safety: See above
            unsafe {
                let module = PreModule::<T>::from_ffi(module);
                let data = &mut *data.cast();
                let context = module.context();
                crate::panic::with_panic_context(context, |_| {
                    C::destroy(module, data);
                });
            }
        });
    }
}

// Reexport the module entry function.
#[link(name = "fimo_std", kind = "static")]
extern "C" {
    #[no_mangle]
    #[doc(hidden)]
    #[allow(unused_attributes)]
    pub fn fimo_impl_module_export_iterator(
        inspector: Option<
            unsafe extern "C" fn(*const bindings::FimoModuleExport, *mut std::ffi::c_void) -> bool,
        >,
        data: *mut std::ffi::c_void,
    );
}
