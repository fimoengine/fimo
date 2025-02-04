use crate::ffi::{ConstNonNull, OpaqueHandle};
use std::{
    fmt::{Debug, Display, Formatter},
    marker::{PhantomData, PhantomPinned},
    mem::MaybeUninit,
    pin::Pin,
    ptr::NonNull,
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

impl Display for ParameterType {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
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

impl Display for ParameterAccessGroup {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
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
    pub type_: ParameterType,
    pub read: ParameterAccessGroup,
    pub write: ParameterAccessGroup,
}

/// Internal representation of a parameter.
pub trait ParameterRepr: Copy + private::Sealed {
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
#[const_trait]
pub trait ParameterCast {
    /// Internal representation of the parameter.
    type Repr: ParameterRepr;

    /// Constructs a value from the internal representation.
    fn from_repr(repr: Self::Repr) -> Self;

    /// Constructs the internal representation from the value.
    fn into_repr(self) -> Self::Repr;
}

impl<T: ParameterRepr> const ParameterCast for T {
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
#[derive(Debug)]
pub struct ParameterVTable {
    pub r#type: extern "C" fn(data: Pin<&'_ Parameter<()>>) -> ParameterType,
    pub read: extern "C" fn(data: Pin<&'_ Parameter<()>>, out: NonNull<()>),
    pub write: extern "C" fn(data: Pin<&'_ Parameter<()>>, value: ConstNonNull<()>),
    pub(crate) _private: PhantomData<()>,
}

impl ParameterVTable {
    cfg_internal! {
        /// Constructs a new `ParameterVTable`.
        ///
        /// # Unstable
        ///
        /// **Note**: This is an [unstable API][unstable]. The public API of this type may break
        /// with any semver compatible release. See
        /// [the documentation on unstable features][unstable] for details.
        ///
        /// [unstable]: crate#unstable-features
        pub const fn new(
            r#type: extern "C" fn(data: Pin<&'_ Parameter<()>>) -> ParameterType,
            read: extern "C" fn(data: Pin<&'_ Parameter<()>>, out: NonNull<()>),
            write: extern "C" fn(data: Pin<&'_ Parameter<()>>, value: ConstNonNull<()>),
        ) -> Self {
            Self {
                r#type,
                read,
                write,
                _private: PhantomData,
            }
        }
    }
}

/// A module parameter.
#[repr(C)]
#[derive(Debug)]
pub struct Parameter<T> {
    pub vtable: ParameterVTable,
    pub(crate) _pinned: PhantomData<PhantomPinned>,
    pub(crate) _private: PhantomData<fn(T) -> T>,
}

impl<T> Parameter<T> {
    cfg_internal! {
        /// Constructs a new `Parameter`.
        ///
        /// # Safety
        ///
        /// Is only safely constructible by the implementation.
        ///
        /// # Unstable
        ///
        /// **Note**: This is an [unstable API][unstable]. The public API of this type may break
        /// with any semver compatible release. See
        /// [the documentation on unstable features][unstable] for details.
        ///
        /// [unstable]: crate#unstable-features
        pub const unsafe fn new_in(out: Pin<&mut MaybeUninit<Self>>, vtable: ParameterVTable) {
            let this = Self {
                vtable,
                _pinned: PhantomData,
                _private: PhantomData,
            };
            unsafe {
                let inner = Pin::get_unchecked_mut(out);
                inner.write(this);
            }
        }
    }

    /// Casts the `Parameter` to an untyped one.
    pub const fn into_opaque(self: Pin<&Self>) -> Pin<&Parameter<()>> {
        unsafe { std::mem::transmute::<Pin<&Self>, Pin<&Parameter<()>>>(self) }
    }

    /// Casts the untyped `Parameter` to a typed one.
    pub fn into_typed<U: ParameterRepr>(self: Pin<&Self>) -> Pin<&Parameter<U>> {
        assert_eq!(self.r#type(), U::TYPE);
        unsafe { std::mem::transmute::<Pin<&Self>, Pin<&Parameter<U>>>(self) }
    }

    /// Returns the internal representation of the parameter.
    pub fn r#type(self: Pin<&Self>) -> ParameterType {
        unsafe {
            let inner = Pin::into_inner_unchecked(self);
            let f = inner.vtable.r#type;
            f(self.into_opaque())
        }
    }

    /// Reads a value from the parameter.
    pub fn read(self: Pin<&Self>) -> T
    where
        T: ParameterCast,
    {
        let mut value = MaybeUninit::<T::Repr>::uninit();
        unsafe {
            let inner = Pin::into_inner_unchecked(self);
            let f = inner.vtable.read;
            f(
                self.into_opaque(),
                NonNull::new_unchecked(&raw mut value).cast(),
            );
            T::from_repr(value.assume_init())
        }
    }

    /// Writes a value into the parameter.
    pub fn write(self: Pin<&Self>, value: T)
    where
        T: ParameterCast,
    {
        unsafe {
            let inner = Pin::into_inner_unchecked(self);
            let f = inner.vtable.write;
            f(
                self.into_opaque(),
                ConstNonNull::new_unchecked(&raw const value).cast(),
            );
        }
    }
}

/// Virtual function table of a [`ParameterData`].
#[repr(C)]
#[derive(Debug)]
pub struct ParameterDataVTable {
    pub r#type: unsafe extern "C" fn(handle: OpaqueHandle<dyn Send + Sync + '_>) -> ParameterType,
    pub read: unsafe extern "C" fn(handle: OpaqueHandle<dyn Send + Sync + '_>, out: NonNull<()>),
    pub write:
        unsafe extern "C" fn(handle: OpaqueHandle<dyn Send + Sync + '_>, value: ConstNonNull<()>),
    pub(crate) _private: PhantomData<()>,
}

impl ParameterDataVTable {
    cfg_internal! {
        /// Constructs a new `VTable`.
        ///
        /// # Unstable
        ///
        /// **Note**: This is an [unstable API][unstable]. The public API of this type may break
        /// with any semver compatible release. See
        /// [the documentation on unstable features][unstable] for details.
        ///
        /// [unstable]: crate#unstable-features
        pub const fn new(
            r#type: unsafe extern "C" fn(handle: OpaqueHandle<dyn Send + Sync + '_>) -> ParameterType,
            read: unsafe extern "C" fn(handle: OpaqueHandle<dyn Send + Sync + '_>, out: NonNull<()>),
            write: unsafe extern "C" fn(
                handle: OpaqueHandle<dyn Send + Sync + '_>,
                value: ConstNonNull<()>,
            ),
        ) -> Self {
            Self {
                r#type,
                read,
                write,
                _private: PhantomData,
            }
        }
    }
}

/// Internal parameter data.
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct ParameterData<'a, T> {
    pub handle: OpaqueHandle<dyn Send + Sync + 'a>,
    pub vtable: &'a ParameterDataVTable,
    pub(crate) _phantom: PhantomData<fn(T) -> T>,
}

impl<'a, T> ParameterData<'a, T> {
    /// Constructs a new `ParameterData`.
    ///
    /// # Safety
    ///
    /// Is only safely constructible by the implementation.
    pub const unsafe fn new(
        handle: OpaqueHandle<dyn Send + Sync + 'a>,
        vtable: &'a ParameterDataVTable,
    ) -> Self {
        Self {
            handle,
            vtable,
            _phantom: PhantomData,
        }
    }

    /// Casts the `ParameterData` to an untyped one.
    pub const fn into_opaque(self) -> ParameterData<'a, ()> {
        ParameterData {
            handle: self.handle,
            vtable: self.vtable,
            _phantom: PhantomData,
        }
    }

    /// Casts the untyped `ParameterData` to a typed one.
    pub fn into_typed<U: ParameterRepr>(self) -> ParameterData<'a, U> {
        let handle = self.handle;
        let vtable = self.vtable;
        assert_eq!(self.r#type(), U::TYPE);

        ParameterData {
            handle,
            vtable,
            _phantom: PhantomData,
        }
    }

    /// Returns the internal representation of the parameter.
    pub fn r#type(self) -> ParameterType {
        unsafe { (self.vtable.r#type)(self.handle) }
    }

    /// Reads a value from the parameter.
    pub fn read(self) -> T
    where
        T: ParameterRepr,
    {
        let mut value = MaybeUninit::<T>::uninit();
        unsafe {
            (self.vtable.read)(self.handle, NonNull::new_unchecked(&raw mut value).cast());
            value.assume_init()
        }
    }

    /// Writes a value into the parameter.
    pub fn write(self, value: T)
    where
        T: ParameterRepr,
    {
        unsafe {
            (self.vtable.write)(
                self.handle,
                ConstNonNull::new_unchecked(&raw const value).cast(),
            );
        }
    }
}
