//! Module backend.

use core::ffi::CStr;

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

#[macro_export]
macro_rules! export_module {
    (
        mod $mod_type:ty {
            name: $name:literal,
            $(description: $descr:literal,)?
            $(author: $author:literal,)?
            $(license: $license:literal,)?
            $(parameters: { $($param_block:tt)* },)?
            $(resources: { $($res_block:tt)* },)?
            $(namespaces: { $($ns_block:tt)* },)?
            $(imports: { $($imports_block:tt)* },)?
            $(exports: { $($exports_block:tt)* },)?
            $(dyn_exports: { $($dyn_exports_block:tt)* },)?
            $(data_type: $data_type:ty $(,)? )?
        }$(;)?
    ) => {
        const _: () = {
            const fn build_export() -> $crate::bindings::FimoModuleExport {
                let name = $crate::export_module_private!(c_str $name);
                let description = $crate::export_module_private!(c_str $($descr)?);
                let author = $crate::export_module_private!(c_str $($author)?);
                let license = $crate::export_module_private!(c_str $($license)?);
                let (parameters, parameters_count) = $crate::export_module_private!(
                    parameters $mod_type { $($($param_block)*)? }
                );
                let (resources, resources_count) = $crate::export_module_private!(
                    resources { $($($res_block)*)? }
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
                    namespace_imports: core::ptr::null(),
                    namespace_imports_count: 0,
                    symbol_imports: core::ptr::null(),
                    symbol_imports_count: 0,
                    symbol_exports: core::ptr::null(),
                    symbol_exports_count: 0,
                    dynamic_symbol_exports: core::ptr::null(),
                    dynamic_symbol_exports_count: 0,
                    module_constructor: None,
                    module_destructor: None,
                }
            }

            #[allow(dead_code)]
            #[repr(transparent)]
            struct Wrapper(&'static $crate::bindings::FimoModuleExport);
            // Safety: A export is send and sync.
            unsafe impl Send for Wrapper {}
            // Safety: A export is send and sync.
            unsafe impl Sync for Wrapper {}

            #[used]
            #[cfg_attr(windows, link_section = "fi_mod$u")]
            #[cfg_attr(
                all(unix, target_vendor = "apple"),
                link_section = "__DATA,__fimo_module"
            )]
            #[cfg_attr(all(unix, not(target_vendor = "apple")), link_section = "fimo_module")]
            static EXPORT: Wrapper = Wrapper(&build_export());
        };
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! export_module_private {
    (c_str) => {{
        core::ptr::null()
    }};
    (c_str $literal:literal) => {{
        let x: &'static str = core::concat!($literal, '\0');
        x.as_ptr().cast()
    }};
    (parameters $mod_type:ty { }) => {{
        (core::ptr::null(), 0)
    }};
    (parameters $mod_type:ty { $($block:tt)* }) => {{
        const X: &[$crate::bindings::FimoModuleParamDecl] =
            $crate::export_module_private_parameter!(parameter $mod_type; $($block)*);
        (X.as_ptr(), X.len() as u32)
    }};
    (resources { }) => {{
        (core::ptr::null(), 0)
    }};
    (resources { $($block:tt)* }) => {{
        const X: &[$crate::bindings::FimoModuleResourceDecl] =
            $crate::export_module_private_resources!($($block)*);
        (X.as_ptr(), X.len() as u32)
    }};
}

#[doc(hidden)]
#[macro_export]
macro_rules! export_module_private_parameter {
    () => {};
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
    (getter $mod_type:ty;) => {
        Some($crate::bindings::fimo_module_param_get_inner as _)
    };
    (getter $mod_type:ty; $x:ident) => {{
        extern "C" fn getter(
            module: *const $crate::bindings::FimoModule,
            value: *mut core::ffi::c_void,
            type_: *mut $crate::bindings::FimoModuleParamType,
            data: *const $crate::bindings::FimoModuleParamData,
        ) {
            // Safety:
            unsafe {
                let module = &*module.cast::<$mod_type>();
                let data = &*data.cast::<core::cell::UnsafeCell<$crate::bindings::FimoModuleParamData>>();
                match $x(module, data) {
                    Err(x) => x.into_error(),
                    Ok(x) => {
                        match value {
                            $crate::module::ParameterValue::U8(x) => {
                                core::ptr::write(value.cast(), x);
                                core::ptr::write(type_, bindings::FimoModuleParamType::FIMO_MODULE_PARAM_TYPE_U8);
                            },
                            $crate::module::ParameterValue::U16(x) => {
                                core::ptr::write(value.cast(), x);
                                core::ptr::write(type_, bindings::FimoModuleParamType::FIMO_MODULE_PARAM_TYPE_U16);
                            },
                            $crate::module::ParameterValue::U32(x) => {
                                core::ptr::write(value.cast(), x);
                                core::ptr::write(type_, bindings::FimoModuleParamType::FIMO_MODULE_PARAM_TYPE_U32);
                            },
                            $crate::module::ParameterValue::U64(x) => {
                                core::ptr::write(value.cast(), x);
                                core::ptr::write(type_, bindings::FimoModuleParamType::FIMO_MODULE_PARAM_TYPE_U64);
                            },
                            $crate::module::ParameterValue::I8(x) => {
                                core::ptr::write(value.cast(), x);
                                core::ptr::write(type_, bindings::FimoModuleParamType::FIMO_MODULE_PARAM_TYPE_I8);
                            },
                            $crate::module::ParameterValue::I16(x) => {
                                core::ptr::write(value.cast(), x);
                                core::ptr::write(type_, bindings::FimoModuleParamType::FIMO_MODULE_PARAM_TYPE_I16);
                            },
                            $crate::module::ParameterValue::I32(x) => {
                                core::ptr::write(value.cast(), x);
                                core::ptr::write(type_, bindings::FimoModuleParamType::FIMO_MODULE_PARAM_TYPE_I32);
                            },
                            $crate::module::ParameterValue::I64(x) => {
                                core::ptr::write(value.cast(), x);
                                core::ptr::write(type_, bindings::FimoModuleParamType::FIMO_MODULE_PARAM_TYPE_I64);
                            },
                        };
                        $crate::error::Error::EOK.into_error()
                    }
                }
            }
        }

        Some(getter as _)
    }};
    (setter $mod_type:ty;) => {
        Some($crate::bindings::fimo_module_param_set_inner as _)
    };
    (setter $mod_type:ty; $x:ident) => {{
        extern "C" fn setter(
            module: *const $crate::bindings::FimoModule,
            value: *const core::ffi::c_void,
            type_: $crate::bindings::FimoModuleParamType,
            data: *mut $crate::bindings::FimoModuleParamData,
        ) {
            // Safety:
            unsafe {
                let module = &*module.cast::<$mod_type>();
                let data = &*data.cast::<core::cell::UnsafeCell<$crate::bindings::FimoModuleParamData>>();
                let type_ = match $crate::module::ParameterType::try_from(type_) {
                    Err(x) => x.into_error(),
                    Ok(x) => x.
                };
                let value = match type_ {
                    $crate::module::ParameterType::U8 => {
                        $crate::module::ParameterValue::U8(core::ptr::read(value.cast()))
                    },
                    $crate::module::ParameterType::U16 => {
                        $crate::module::ParameterValue::U16(core::ptr::read(value.cast()))
                    },
                    $crate::module::ParameterType::U32 => {
                        $crate::module::ParameterValue::U32(core::ptr::read(value.cast()))
                    },
                    $crate::module::ParameterType::U64 => {
                        $crate::module::ParameterValue::U64(core::ptr::read(value.cast()))
                    },
                    $crate::module::ParameterType::I8 => {
                        $crate::module::ParameterValue::I8(core::ptr::read(value.cast()))
                    },
                    $crate::module::ParameterType::I16 => {
                        $crate::module::ParameterValue::I16(core::ptr::read(value.cast()))
                    },
                    $crate::module::ParameterType::I32 => {
                        $crate::module::ParameterValue::I32(core::ptr::read(value.cast()))
                    },
                    $crate::module::ParameterType::I64 => {
                        $crate::module::ParameterValue::I64(core::ptr::read(value.cast()))
                    },
                };
                match $x(module, value, data) {
                    Err(x) => x.into_error(),
                    Ok(x) => $crate::error::Error::EOK.into_error()
                }
            }
        }

        Some(setter as _)
    }};
    (parameter $mod_type:ty; $( $name:ident: {
        default: $default_ty:ident ( $default:literal ),
        $(read_group: $read:literal,)?
        $(write_group: $write:literal,)?
        $(getter: $getter:ident,)?
        $(setter: $setter:ident,)?
    }),+ $(,)?) => {
        &[
            $(
                $crate::bindings::FimoModuleParamDecl {
                    type_: $crate::export_module_private_parameter!(default_type $default_ty),
                    read_access: $crate::export_module_private_parameter!(group $($read)?),
                    write_access: $crate::export_module_private_parameter!(group $($write)?),
                    setter: $crate::export_module_private_parameter!(setter $mod_type; $($setter)?),
                    getter: $crate::export_module_private_parameter!(getter $mod_type; $($getter)?),
                    name: {
                        let x: &'static str = core::concat!(core::stringify!($name), '\0');
                        x.as_ptr().cast()
                    },
                    default_value: $crate::export_module_private_parameter!(default_value $default_ty $default),
                }
            ),+
        ]
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! export_module_private_resources {
    ($(
        $name:ident: $path:literal
    ),+ $(,)?) => {
        &[
            $(
                $crate::bindings::FimoModuleResourceDecl {
                    path: {
                        let x: &'static str = core::concat!($path, '\0');
                        x.as_ptr().cast()
                    }
                }
            ),+
        ]
    };
}
