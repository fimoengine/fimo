use crate::ffi::{OpaqueHandle, VTablePtr};
use std::{
    fmt::{Debug, Formatter},
    marker::{PhantomData, PhantomPinned},
    mem::MaybeUninit,
    pin::Pin,
};

/// Type of module parameter.
#[repr(i32)]
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
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
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

/// Access group of a module parameter.
#[repr(i32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ParameterAccessGroup {
    Public,
    Dependency,
    Private,
}

impl core::fmt::Display for ParameterAccessGroup {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        match self {
            ParameterAccessGroup::Public => write!(f, "Public"),
            ParameterAccessGroup::Dependency => write!(f, "Dependency"),
            ParameterAccessGroup::Private => write!(f, "Private"),
        }
    }
}

/// Information of a parameter.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ParameterInfo {
    type_: ParameterType,
    read: ParameterAccessGroup,
    write: ParameterAccessGroup,
}

/// Internal representation of a parameter.
pub trait ParameterRepr: private::Sealed {
    const TYPE: ParameterType;
}

impl ParameterRepr for u8 {
    const TYPE: ParameterType = ParameterType::U8;
}

impl ParameterRepr for u16 {
    const TYPE: ParameterType = ParameterType::U16;
}

impl ParameterRepr for u32 {
    const TYPE: ParameterType = ParameterType::U32;
}

impl ParameterRepr for u64 {
    const TYPE: ParameterType = ParameterType::U64;
}

impl ParameterRepr for i8 {
    const TYPE: ParameterType = ParameterType::I8;
}

impl ParameterRepr for i16 {
    const TYPE: ParameterType = ParameterType::I16;
}

impl ParameterRepr for i32 {
    const TYPE: ParameterType = ParameterType::I32;
}

impl ParameterRepr for i64 {
    const TYPE: ParameterType = ParameterType::I64;
}

mod private {
    pub trait Sealed {}

    impl Sealed for u8 {}
    impl Sealed for u16 {}
    impl Sealed for u32 {}
    impl Sealed for u64 {}

    impl Sealed for i8 {}
    impl Sealed for i16 {}
    impl Sealed for i32 {}
    impl Sealed for i64 {}
}

/// Helper trait for all types that can be used as module parameters.
pub trait ParameterCast {
    /// Internal representation of the parameter.
    type Repr: ParameterRepr;

    /// Constructs a value from the internal representation.
    fn from_repr(repr: Self::Repr) -> Self;

    /// Constructs the internal representation from the value.
    fn into_repr(self) -> Self::Repr;
}

impl<T: ParameterRepr> ParameterCast for T {
    type Repr = T;

    fn from_repr(repr: Self::Repr) -> Self {
        repr
    }

    fn into_repr(self) -> Self::Repr {
        self
    }
}

/// Virtual function table of a [`Parameter`].
#[repr(C)]
pub struct ParameterVTable<T: ParameterCast> {
    pub r#type: extern "C" fn(data: Pin<&'_ Parameter<T>>) -> ParameterType,
    pub read: extern "C" fn(data: Pin<&'_ Parameter<T>>, &'_ mut MaybeUninit<T::Repr>),
    pub write: extern "C" fn(data: Pin<&'_ Parameter<T>>, &'_ T::Repr),
    pub phantom: PhantomData<fn(T::Repr) -> T::Repr>,
}

impl<T: ParameterCast> Debug for ParameterVTable<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ParameterVTable")
            .field("type", &self.r#type)
            .field("read", &self.read)
            .field("write", &self.write)
            .finish()
    }
}

/// A module parameter.
#[repr(C)]
#[derive(Debug)]
pub struct Parameter<T: ParameterCast> {
    pub vtable: ParameterVTable<T>,
    _pinned: PhantomPinned,
}

impl<T: ParameterCast> Parameter<T> {
    /// Returns the internal representation of the parameter.
    pub fn r#type(self: Pin<&Self>) -> ParameterType {
        (self.vtable.r#type)(self)
    }

    /// Reads a value from the parameter.
    pub fn read(self: Pin<&Self>) -> T {
        let mut value = MaybeUninit::<T::Repr>::uninit();
        (self.vtable.read)(self, &mut value);
        let value = unsafe { value.assume_init() };
        T::from_repr(value)
    }

    /// Writes a value into the parameter.
    pub fn write(self: Pin<&Self>, value: T) {
        let value = value.into_repr();
        (self.vtable.write)(self, &value);
    }
}

pub struct ParameterDataVTable<T: ParameterRepr> {
    pub r#type: extern "C" fn(handle: OpaqueHandle<dyn Send + Sync>) -> ParameterType,
    pub read: extern "C" fn(handle: OpaqueHandle<dyn Send + Sync>, &'_ mut MaybeUninit<T>),
    pub write: extern "C" fn(handle: OpaqueHandle<dyn Send + Sync>, &'_ T),
    pub phantom: PhantomData<fn(T) -> T>,
}

/// Internal parameter data.
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct ParameterData<T: ParameterRepr> {
    pub handle: OpaqueHandle<dyn Send + Sync>,
    pub vtable: VTablePtr<ParameterDataVTable<T>>,
}

impl<T: ParameterRepr> ParameterData<T> {
    /// Returns the internal representation of the parameter.
    pub fn r#type(self) -> ParameterType {
        (self.vtable.r#type)(self.handle)
    }

    /// Reads a value from the parameter.
    pub fn read(self) -> T {
        let mut value = MaybeUninit::<T>::uninit();
        (self.vtable.read)(self.handle, &mut value);
        unsafe { value.assume_init() }
    }

    /// Writes a value into the parameter.
    pub fn write(self, value: T) {
        (self.vtable.write)(self.handle, &value);
    }
}
