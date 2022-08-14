//! Marshalling utilities.

/// Bridge for Rust to Rust types.
pub trait RustTypeBridge {
    /// Type to marshal to.
    type Type;

    /// Marshals the type.
    fn marshal(self) -> Self::Type;

    /// Demarshals the type.
    fn demarshal(x: Self::Type) -> Self;
}

impl<T> const RustTypeBridge for T {
    type Type = Self;

    #[inline(always)]
    fn marshal(self) -> Self::Type {
        self
    }

    #[inline(always)]
    fn demarshal(x: Self::Type) -> Self {
        x
    }
}

/// Bridge for Rust to C types.
///
/// # Safety
///
/// Implementations must always implement the entire trait
/// without making use of the default implementations.
pub unsafe trait CTypeBridge {
    /// Type to marshal to.
    type Type;

    /// Marshals the type.
    fn marshal(self) -> Self::Type;

    /// Demarshals the type.
    ///
    /// # Safety
    ///
    /// The marshaling operation represents a non injective mapping
    /// from the type `T` to an arbitrary type `U`. Therefore it is likely,
    /// that multiple types are mapped to the same `U` type.
    ///
    /// When calling this method, one must ensure that the same marshaler
    /// is used for both marshalling and demarshalling, i. e. `T::marshal`
    /// followed by `T::demarshal`, or that the marshaler is able to work
    /// with the value one intends to demarshal.
    unsafe fn demarshal(x: Self::Type) -> Self;
}

unsafe impl<T> const CTypeBridge for T {
    default type Type = Self;

    #[inline(always)]
    default fn marshal(self) -> Self::Type {
        let this = std::mem::ManuallyDrop::new(self);
        unsafe { std::mem::transmute_copy(&this) }
    }

    #[inline(always)]
    default unsafe fn demarshal(x: Self::Type) -> Self {
        let x = std::mem::ManuallyDrop::new(x);
        std::mem::transmute_copy(&x)
    }
}

unsafe impl const CTypeBridge for () {
    type Type = u8;

    fn marshal(self) -> Self::Type {
        0
    }

    unsafe fn demarshal(_x: Self::Type) -> Self {}
}

unsafe impl const CTypeBridge for char {
    type Type = u32;

    fn marshal(self) -> Self::Type {
        self as u32
    }

    unsafe fn demarshal(x: Self::Type) -> Self {
        Self::from_u32_unchecked(x)
    }
}

unsafe impl<T, const N: usize> CTypeBridge for [T; N]
where
    T: CTypeBridge,
{
    type Type = [T::Type; N];

    fn marshal(self) -> Self::Type {
        self.map(|x| x.marshal())
    }

    unsafe fn demarshal(x: Self::Type) -> Self {
        x.map(|x| T::demarshal(x))
    }
}
