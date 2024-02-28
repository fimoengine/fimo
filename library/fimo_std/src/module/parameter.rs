use core::{ffi::CStr, marker::PhantomData, ops::Deref};

use crate::{
    bindings,
    error::{self, to_result_indirect, Error},
    ffi::{FFISharable, FFITransferable},
};

use super::{Module, ModuleBackendGuard};

/// Type of a module parameter.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ParameterType {
    U8,
    U16,
    U32,
    U64,
    I8,
    I16,
    I32,
    I64,
}

impl core::fmt::Display for ParameterType {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            ParameterType::U8 => write!(f, "u8"),
            ParameterType::U16 => write!(f, "u16"),
            ParameterType::U32 => write!(f, "u32"),
            ParameterType::U64 => write!(f, "u64"),
            ParameterType::I8 => write!(f, "i8"),
            ParameterType::I16 => write!(f, "i16"),
            ParameterType::I32 => write!(f, "i32"),
            ParameterType::I64 => write!(f, "i64"),
        }
    }
}

impl TryFrom<bindings::FimoModuleParamType> for ParameterType {
    type Error = Error;

    fn try_from(value: bindings::FimoModuleParamType) -> Result<Self, Self::Error> {
        match value {
            bindings::FimoModuleParamType::FIMO_MODULE_PARAM_TYPE_U8 => Ok(ParameterType::U8),
            bindings::FimoModuleParamType::FIMO_MODULE_PARAM_TYPE_U16 => Ok(ParameterType::U16),
            bindings::FimoModuleParamType::FIMO_MODULE_PARAM_TYPE_U32 => Ok(ParameterType::U32),
            bindings::FimoModuleParamType::FIMO_MODULE_PARAM_TYPE_U64 => Ok(ParameterType::U64),
            bindings::FimoModuleParamType::FIMO_MODULE_PARAM_TYPE_I8 => Ok(ParameterType::I8),
            bindings::FimoModuleParamType::FIMO_MODULE_PARAM_TYPE_I16 => Ok(ParameterType::I16),
            bindings::FimoModuleParamType::FIMO_MODULE_PARAM_TYPE_I32 => Ok(ParameterType::I32),
            bindings::FimoModuleParamType::FIMO_MODULE_PARAM_TYPE_I64 => Ok(ParameterType::I64),
            _ => Err(Error::EINVAL),
        }
    }
}

impl From<ParameterType> for bindings::FimoModuleParamType {
    fn from(value: ParameterType) -> Self {
        match value {
            ParameterType::U8 => bindings::FimoModuleParamType::FIMO_MODULE_PARAM_TYPE_U8,
            ParameterType::U16 => bindings::FimoModuleParamType::FIMO_MODULE_PARAM_TYPE_U16,
            ParameterType::U32 => bindings::FimoModuleParamType::FIMO_MODULE_PARAM_TYPE_U32,
            ParameterType::U64 => bindings::FimoModuleParamType::FIMO_MODULE_PARAM_TYPE_U64,
            ParameterType::I8 => bindings::FimoModuleParamType::FIMO_MODULE_PARAM_TYPE_I8,
            ParameterType::I16 => bindings::FimoModuleParamType::FIMO_MODULE_PARAM_TYPE_I16,
            ParameterType::I32 => bindings::FimoModuleParamType::FIMO_MODULE_PARAM_TYPE_I32,
            ParameterType::I64 => bindings::FimoModuleParamType::FIMO_MODULE_PARAM_TYPE_I64,
        }
    }
}

impl crate::ffi::FFITransferable<bindings::FimoModuleParamType> for ParameterType {
    fn into_ffi(self) -> bindings::FimoModuleParamType {
        self.into()
    }

    unsafe fn from_ffi(ffi: bindings::FimoModuleParamType) -> Self {
        ffi.try_into().expect("expected known enum value")
    }
}

/// Access group of a module parameter.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ParameterAccess {
    Public,
    Dependency,
    Private,
}

impl core::fmt::Display for ParameterAccess {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            ParameterAccess::Public => write!(f, "Public"),
            ParameterAccess::Dependency => write!(f, "Dependency"),
            ParameterAccess::Private => write!(f, "Private"),
        }
    }
}

impl TryFrom<bindings::FimoModuleParamAccess> for ParameterAccess {
    type Error = Error;

    fn try_from(value: bindings::FimoModuleParamAccess) -> Result<Self, Self::Error> {
        match value {
            bindings::FimoModuleParamAccess::FIMO_MODULE_CONFIG_ACCESS_PUBLIC => {
                Ok(ParameterAccess::Public)
            }
            bindings::FimoModuleParamAccess::FIMO_MODULE_CONFIG_ACCESS_DEPENDENCY => {
                Ok(ParameterAccess::Dependency)
            }
            bindings::FimoModuleParamAccess::FIMO_MODULE_CONFIG_ACCESS_PRIVATE => {
                Ok(ParameterAccess::Private)
            }
            _ => Err(Error::EINVAL),
        }
    }
}

impl From<ParameterAccess> for bindings::FimoModuleParamAccess {
    fn from(value: ParameterAccess) -> Self {
        match value {
            ParameterAccess::Public => {
                bindings::FimoModuleParamAccess::FIMO_MODULE_CONFIG_ACCESS_PUBLIC
            }
            ParameterAccess::Dependency => {
                bindings::FimoModuleParamAccess::FIMO_MODULE_CONFIG_ACCESS_DEPENDENCY
            }
            ParameterAccess::Private => {
                bindings::FimoModuleParamAccess::FIMO_MODULE_CONFIG_ACCESS_PRIVATE
            }
        }
    }
}

impl crate::ffi::FFITransferable<bindings::FimoModuleParamAccess> for ParameterAccess {
    fn into_ffi(self) -> bindings::FimoModuleParamAccess {
        self.into()
    }

    unsafe fn from_ffi(ffi: bindings::FimoModuleParamAccess) -> Self {
        ffi.try_into().expect("expected known enum value")
    }
}

/// Value of a module parameter.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ParameterValue {
    U8(u8),
    U16(u16),
    U32(u32),
    U64(u64),
    I8(i8),
    I16(i16),
    I32(i32),
    I64(i64),
}

union ValueTypes {
    u8_: u8,
    u16_: u16,
    u32_: u32,
    u64_: u64,
    i8_: i8,
    i16_: i16,
    i32_: i32,
    i64_: i64,
}

impl ParameterValue {
    /// Reads a module parameter with public read access.
    ///
    /// Reads the value of a module parameter with public read access. The operation fails, if
    /// the parameter does not exist, or if the parameter does not allow reading with a public
    /// access.
    pub fn read_public(
        guard: &ModuleBackendGuard<'_>,
        module: &CStr,
        parameter: &CStr,
    ) -> Result<Self, Error> {
        let mut value = ValueTypes { u8_: 0 };
        let mut type_ = bindings::FimoModuleParamType::FIMO_MODULE_PARAM_TYPE_U8;

        // Safety: The ffi call is safe.
        to_result_indirect(|error| unsafe {
            *error = bindings::fimo_module_param_get_public(
                guard.share_to_ffi(),
                core::ptr::from_mut(&mut value).cast(),
                &mut type_,
                module.as_ptr(),
                parameter.as_ptr(),
            );
        })?;
        let type_ = ParameterType::try_from(type_)?;

        // Safety: We checked the type tag.
        unsafe {
            match type_ {
                ParameterType::U8 => Ok(Self::U8(value.u8_)),
                ParameterType::U16 => Ok(Self::U16(value.u16_)),
                ParameterType::U32 => Ok(Self::U32(value.u32_)),
                ParameterType::U64 => Ok(Self::U64(value.u64_)),
                ParameterType::I8 => Ok(Self::I8(value.i8_)),
                ParameterType::I16 => Ok(Self::I16(value.i16_)),
                ParameterType::I32 => Ok(Self::I32(value.i32_)),
                ParameterType::I64 => Ok(Self::I64(value.i64_)),
            }
        }
    }

    /// Reads a module parameter with dependency read access.
    ///
    /// Reads the value of a module parameter with dependency read access. The operation fails, if
    /// the parameter does not exist, or if the parameter does not allow reading with a dependency
    /// access.
    pub fn read_dependency(
        caller: &impl Module,
        module: &CStr,
        parameter: &CStr,
    ) -> Result<Self, Error> {
        let mut value = ValueTypes { u8_: 0 };
        let mut type_ = bindings::FimoModuleParamType::FIMO_MODULE_PARAM_TYPE_U8;

        // Safety: The ffi call is safe.
        to_result_indirect(|error| unsafe {
            *error = bindings::fimo_module_param_get_dependency(
                caller.share_to_ffi(),
                core::ptr::from_mut(&mut value).cast(),
                &mut type_,
                module.as_ptr(),
                parameter.as_ptr(),
            );
        })?;
        let type_ = ParameterType::try_from(type_)?;

        // Safety: We checked the type tag.
        unsafe {
            match type_ {
                ParameterType::U8 => Ok(Self::U8(value.u8_)),
                ParameterType::U16 => Ok(Self::U16(value.u16_)),
                ParameterType::U32 => Ok(Self::U32(value.u32_)),
                ParameterType::U64 => Ok(Self::U64(value.u64_)),
                ParameterType::I8 => Ok(Self::I8(value.i8_)),
                ParameterType::I16 => Ok(Self::I16(value.i16_)),
                ParameterType::I32 => Ok(Self::I32(value.i32_)),
                ParameterType::I64 => Ok(Self::I64(value.i64_)),
            }
        }
    }

    /// Reads a module parameter with private read access.
    pub fn read_private(
        caller: &impl Module,
        parameter: &OpaqueParameter<'_>,
    ) -> Result<Self, Error> {
        let mut value = ValueTypes { u8_: 0 };
        let mut type_ = bindings::FimoModuleParamType::FIMO_MODULE_PARAM_TYPE_U8;

        // Safety: The ffi call is safe.
        to_result_indirect(|error| unsafe {
            *error = bindings::fimo_module_param_get_private(
                caller.share_to_ffi(),
                core::ptr::from_mut(&mut value).cast(),
                &mut type_,
                parameter.share_to_ffi(),
            );
        })?;
        let type_ = ParameterType::try_from(type_)?;

        // Safety: We checked the type tag.
        unsafe {
            match type_ {
                ParameterType::U8 => Ok(Self::U8(value.u8_)),
                ParameterType::U16 => Ok(Self::U16(value.u16_)),
                ParameterType::U32 => Ok(Self::U32(value.u32_)),
                ParameterType::U64 => Ok(Self::U64(value.u64_)),
                ParameterType::I8 => Ok(Self::I8(value.i8_)),
                ParameterType::I16 => Ok(Self::I16(value.i16_)),
                ParameterType::I32 => Ok(Self::I32(value.i32_)),
                ParameterType::I64 => Ok(Self::I64(value.i64_)),
            }
        }
    }

    /// Reads a module parameter.
    pub fn read_inner(
        caller: &impl Module,
        parameter: &bindings::FimoModuleParamData,
    ) -> Result<Self, Error> {
        let mut value = ValueTypes { u8_: 0 };
        let mut type_ = bindings::FimoModuleParamType::FIMO_MODULE_PARAM_TYPE_U8;

        // Safety: The ffi call is safe.
        to_result_indirect(|error| unsafe {
            *error = bindings::fimo_module_param_get_inner(
                caller.share_to_ffi(),
                core::ptr::from_mut(&mut value).cast(),
                &mut type_,
                parameter,
            );
        })?;
        let type_ = ParameterType::try_from(type_)?;

        // Safety: We checked the type tag.
        unsafe {
            match type_ {
                ParameterType::U8 => Ok(Self::U8(value.u8_)),
                ParameterType::U16 => Ok(Self::U16(value.u16_)),
                ParameterType::U32 => Ok(Self::U32(value.u32_)),
                ParameterType::U64 => Ok(Self::U64(value.u64_)),
                ParameterType::I8 => Ok(Self::I8(value.i8_)),
                ParameterType::I16 => Ok(Self::I16(value.i16_)),
                ParameterType::I32 => Ok(Self::I32(value.i32_)),
                ParameterType::I64 => Ok(Self::I64(value.i64_)),
            }
        }
    }

    /// Writes a module parameter with public write access.
    ///
    /// Sets the value of a module parameter with public write access. The operation fails, if
    /// the parameter does not exist, or if the parameter does not allow writing with a public
    /// access.
    pub fn write_public(
        self,
        guard: &ModuleBackendGuard<'_>,
        module: &CStr,
        parameter: &CStr,
    ) -> error::Result {
        let (value, type_) = match self {
            ParameterValue::U8(x) => (ValueTypes { u8_: x }, ParameterType::U8),
            ParameterValue::U16(x) => (ValueTypes { u16_: x }, ParameterType::U16),
            ParameterValue::U32(x) => (ValueTypes { u32_: x }, ParameterType::U32),
            ParameterValue::U64(x) => (ValueTypes { u64_: x }, ParameterType::U64),
            ParameterValue::I8(x) => (ValueTypes { i8_: x }, ParameterType::I8),
            ParameterValue::I16(x) => (ValueTypes { i16_: x }, ParameterType::I16),
            ParameterValue::I32(x) => (ValueTypes { i32_: x }, ParameterType::I32),
            ParameterValue::I64(x) => (ValueTypes { i64_: x }, ParameterType::I64),
        };

        // Safety: The ffi call is safe.
        to_result_indirect(|error| unsafe {
            *error = bindings::fimo_module_param_set_public(
                guard.share_to_ffi(),
                core::ptr::from_ref(&value).cast(),
                type_.into_ffi(),
                module.as_ptr(),
                parameter.as_ptr(),
            );
        })
    }

    /// Writes a module parameter with dependency write access.
    ///
    /// Sets the value of a module parameter with dependency write access. The operation fails, if
    /// the parameter does not exist, or if the parameter does not allow writing with a dependency
    /// access.
    pub fn write_dependency(
        self,
        caller: &impl Module,
        module: &CStr,
        parameter: &CStr,
    ) -> error::Result {
        let (value, type_) = match self {
            ParameterValue::U8(x) => (ValueTypes { u8_: x }, ParameterType::U8),
            ParameterValue::U16(x) => (ValueTypes { u16_: x }, ParameterType::U16),
            ParameterValue::U32(x) => (ValueTypes { u32_: x }, ParameterType::U32),
            ParameterValue::U64(x) => (ValueTypes { u64_: x }, ParameterType::U64),
            ParameterValue::I8(x) => (ValueTypes { i8_: x }, ParameterType::I8),
            ParameterValue::I16(x) => (ValueTypes { i16_: x }, ParameterType::I16),
            ParameterValue::I32(x) => (ValueTypes { i32_: x }, ParameterType::I32),
            ParameterValue::I64(x) => (ValueTypes { i64_: x }, ParameterType::I64),
        };

        // Safety: The ffi call is safe.
        to_result_indirect(|error| unsafe {
            *error = bindings::fimo_module_param_set_dependency(
                caller.share_to_ffi(),
                core::ptr::from_ref(&value).cast(),
                type_.into_ffi(),
                module.as_ptr(),
                parameter.as_ptr(),
            );
        })
    }

    /// Writes a module parameter.
    pub fn write_private(
        self,
        caller: &impl Module,
        parameter: &OpaqueParameter<'_>,
    ) -> error::Result {
        let (value, type_) = match self {
            ParameterValue::U8(x) => (ValueTypes { u8_: x }, ParameterType::U8),
            ParameterValue::U16(x) => (ValueTypes { u16_: x }, ParameterType::U16),
            ParameterValue::U32(x) => (ValueTypes { u32_: x }, ParameterType::U32),
            ParameterValue::U64(x) => (ValueTypes { u64_: x }, ParameterType::U64),
            ParameterValue::I8(x) => (ValueTypes { i8_: x }, ParameterType::I8),
            ParameterValue::I16(x) => (ValueTypes { i16_: x }, ParameterType::I16),
            ParameterValue::I32(x) => (ValueTypes { i32_: x }, ParameterType::I32),
            ParameterValue::I64(x) => (ValueTypes { i64_: x }, ParameterType::I64),
        };

        // Safety: The ffi call is safe.
        to_result_indirect(|error| unsafe {
            *error = bindings::fimo_module_param_set_private(
                caller.share_to_ffi(),
                core::ptr::from_ref(&value).cast(),
                type_.into_ffi(),
                parameter.share_to_ffi(),
            );
        })
    }

    /// Writes a module parameter.
    pub fn write_inner(
        self,
        caller: &impl Module,
        parameter: &mut bindings::FimoModuleParamData,
    ) -> error::Result {
        let (value, type_) = match self {
            ParameterValue::U8(x) => (ValueTypes { u8_: x }, ParameterType::U8),
            ParameterValue::U16(x) => (ValueTypes { u16_: x }, ParameterType::U16),
            ParameterValue::U32(x) => (ValueTypes { u32_: x }, ParameterType::U32),
            ParameterValue::U64(x) => (ValueTypes { u64_: x }, ParameterType::U64),
            ParameterValue::I8(x) => (ValueTypes { i8_: x }, ParameterType::I8),
            ParameterValue::I16(x) => (ValueTypes { i16_: x }, ParameterType::I16),
            ParameterValue::I32(x) => (ValueTypes { i32_: x }, ParameterType::I32),
            ParameterValue::I64(x) => (ValueTypes { i64_: x }, ParameterType::I64),
        };

        // Safety: The ffi call is safe.
        to_result_indirect(|error| unsafe {
            *error = bindings::fimo_module_param_set_inner(
                caller.share_to_ffi(),
                core::ptr::from_ref(&value).cast(),
                type_.into_ffi(),
                parameter,
            );
        })
    }
}

impl core::fmt::Display for ParameterValue {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            ParameterValue::U8(x) => write!(f, "{x}"),
            ParameterValue::U16(x) => write!(f, "{x}"),
            ParameterValue::U32(x) => write!(f, "{x}"),
            ParameterValue::U64(x) => write!(f, "{x}"),
            ParameterValue::I8(x) => write!(f, "{x}"),
            ParameterValue::I16(x) => write!(f, "{x}"),
            ParameterValue::I32(x) => write!(f, "{x}"),
            ParameterValue::I64(x) => write!(f, "{x}"),
        }
    }
}

/// Information of a parameter.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ParameterInfo {
    type_: ParameterType,
    read: ParameterAccess,
    write: ParameterAccess,
}

impl ParameterInfo {
    /// Queries the info of a module parameter.
    pub fn query(
        guard: &ModuleBackendGuard<'_>,
        module: &CStr,
        parameter: &CStr,
    ) -> Result<Self, Error> {
        let mut type_ = bindings::FimoModuleParamType::FIMO_MODULE_PARAM_TYPE_U8;
        let mut read = bindings::FimoModuleParamAccess::FIMO_MODULE_CONFIG_ACCESS_PRIVATE;
        let mut write = bindings::FimoModuleParamAccess::FIMO_MODULE_CONFIG_ACCESS_PRIVATE;

        // Safety: The ffi call is safe.
        to_result_indirect(|error| unsafe {
            *error = bindings::fimo_module_param_query(
                guard.share_to_ffi(),
                module.as_ptr(),
                parameter.as_ptr(),
                &mut type_,
                &mut read,
                &mut write,
            );
        })?;

        let type_ = TryFrom::try_from(type_)?;
        let read = TryFrom::try_from(read)?;
        let write = TryFrom::try_from(write)?;
        Ok(Self { type_, read, write })
    }

    /// Fetches the type of the parameter.
    pub fn parameter_type(&self) -> ParameterType {
        self.type_
    }

    /// Fetches the access group specifier for the read permission.
    pub fn read_access(&self) -> ParameterAccess {
        self.read
    }

    /// Fetches the access group specifier for the write permission.
    pub fn write_access(&self) -> ParameterAccess {
        self.read
    }
}

/// A module parameter.
#[derive(Debug)]
pub struct OpaqueParameter<'a>(*mut bindings::FimoModuleParam, PhantomData<&'a mut ()>);

/// Safety: A parameter is a reference to an atomic integer.
unsafe impl Send for OpaqueParameter<'_> {}

/// Safety: A parameter is a reference to an atomic integer.
unsafe impl Sync for OpaqueParameter<'_> {}

impl crate::ffi::FFISharable<*mut bindings::FimoModuleParam> for OpaqueParameter<'_> {
    type BorrowedView<'a> = OpaqueParameter<'a>;

    fn share_to_ffi(&self) -> *mut bindings::FimoModuleParam {
        self.0
    }

    unsafe fn borrow_from_ffi<'a>(ffi: *mut bindings::FimoModuleParam) -> Self::BorrowedView<'a> {
        OpaqueParameter(ffi, PhantomData)
    }
}

impl crate::ffi::FFITransferable<*mut bindings::FimoModuleParam> for OpaqueParameter<'_> {
    fn into_ffi(self) -> *mut bindings::FimoModuleParam {
        self.0
    }

    unsafe fn from_ffi(ffi: *mut bindings::FimoModuleParam) -> Self {
        Self(ffi, PhantomData)
    }
}

/// A typed parameter.
pub struct Parameter<'a, T: ParameterCast>(
    OpaqueParameter<'a>,
    PhantomData<&'a core::cell::Cell<T>>,
);

impl<T: ParameterCast> Parameter<'_, T> {
    /// Reads from the private parameter of the module.
    pub fn read(&self, caller: &impl Module) -> Result<T, Error> {
        let value = ParameterValue::read_private(caller, self)?;
        T::from_parameter_value(value)
    }

    /// Writes to the private parameter of the module.
    pub fn write(&self, caller: &impl Module, value: T) -> error::Result {
        let value = value.to_parameter_value();
        value.write_private(caller, self)
    }
}

impl<'a, T: ParameterCast> Deref for Parameter<'a, T> {
    type Target = OpaqueParameter<'a>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T: ParameterCast> crate::ffi::FFITransferable<*mut bindings::FimoModuleParam>
    for Parameter<'_, T>
{
    fn into_ffi(self) -> *mut bindings::FimoModuleParam {
        self.0.into_ffi()
    }

    unsafe fn from_ffi(ffi: *mut bindings::FimoModuleParam) -> Self {
        // Safety: We assume that the caller ensures that it is safe.
        unsafe { Self(OpaqueParameter::from_ffi(ffi), PhantomData) }
    }
}

/// Casting to and from a [`ParameterValue`].
pub trait ParameterCast: Sized {
    fn to_parameter_value(self) -> ParameterValue;
    fn from_parameter_value(value: ParameterValue) -> Result<Self, Error>;
}
impl ParameterCast for u8 {
    fn to_parameter_value(self) -> ParameterValue {
        ParameterValue::U8(self)
    }

    fn from_parameter_value(value: ParameterValue) -> Result<Self, Error> {
        match value {
            ParameterValue::U8(x) => Ok(x),
            _ => Err(Error::EINVAL),
        }
    }
}
impl ParameterCast for u16 {
    fn to_parameter_value(self) -> ParameterValue {
        ParameterValue::U16(self)
    }

    fn from_parameter_value(value: ParameterValue) -> Result<Self, Error> {
        match value {
            ParameterValue::U16(x) => Ok(x),
            _ => Err(Error::EINVAL),
        }
    }
}
impl ParameterCast for u32 {
    fn to_parameter_value(self) -> ParameterValue {
        ParameterValue::U32(self)
    }

    fn from_parameter_value(value: ParameterValue) -> Result<Self, Error> {
        match value {
            ParameterValue::U32(x) => Ok(x),
            _ => Err(Error::EINVAL),
        }
    }
}
impl ParameterCast for u64 {
    fn to_parameter_value(self) -> ParameterValue {
        ParameterValue::U64(self)
    }

    fn from_parameter_value(value: ParameterValue) -> Result<Self, Error> {
        match value {
            ParameterValue::U64(x) => Ok(x),
            _ => Err(Error::EINVAL),
        }
    }
}
impl ParameterCast for i8 {
    fn to_parameter_value(self) -> ParameterValue {
        ParameterValue::I8(self)
    }

    fn from_parameter_value(value: ParameterValue) -> Result<Self, Error> {
        match value {
            ParameterValue::I8(x) => Ok(x),
            _ => Err(Error::EINVAL),
        }
    }
}
impl ParameterCast for i16 {
    fn to_parameter_value(self) -> ParameterValue {
        ParameterValue::I16(self)
    }

    fn from_parameter_value(value: ParameterValue) -> Result<Self, Error> {
        match value {
            ParameterValue::I16(x) => Ok(x),
            _ => Err(Error::EINVAL),
        }
    }
}
impl ParameterCast for i32 {
    fn to_parameter_value(self) -> ParameterValue {
        ParameterValue::I32(self)
    }

    fn from_parameter_value(value: ParameterValue) -> Result<Self, Error> {
        match value {
            ParameterValue::I32(x) => Ok(x),
            _ => Err(Error::EINVAL),
        }
    }
}
impl ParameterCast for i64 {
    fn to_parameter_value(self) -> ParameterValue {
        ParameterValue::I64(self)
    }

    fn from_parameter_value(value: ParameterValue) -> Result<Self, Error> {
        match value {
            ParameterValue::I64(x) => Ok(x),
            _ => Err(Error::EINVAL),
        }
    }
}
