//! Utilities for defining and working with symbols.
use crate::version::Version;
use std::{
    ffi::CStr,
    fmt::{Debug, Pointer},
    marker::PhantomData,
    ops::{Deref, DerefMut},
};

/// Marker trait for types that can safely pass the instance boundary.
///
/// The runtime assumes that instances are isolated entities, with the imports and exports serving
/// as the interface between instances. Under this assumption, the runtime can prevent
/// use-after-free errors by tracking dependencies at the instance level, instead of tracking
/// individual resources of each instance. Strictly speaking, by default, interfaces only depend on
/// the imported symbols of other instances, and not on the instances themselves. As a consequence,
/// the runtime is allowed to silently replace any instance if, under the assumption of instance
/// isolation, the action would not be visible by other instances. The only way to ensure that an
/// instance is not unloaded is to acquire a strong reference to the instance in question with
/// the [`GenericInstance::to_owned_instance`][to_owned_instance] or
/// [`InfoView::try_ref_instance_strong`][try_ref_instance_strong] methods.
///
/// This trait marks whether a type is safe to pass the instance boundary without breaking the
/// assumption of isolation. Notably, this can not be guaranteed for pointer-like types, like
/// references and functions, as they potentially point to data owned by the instance.
///
/// [to_owned_instance]: crate::module::GenericInstance::to_owned_instance
/// [try_ref_instance_strong]: crate::module::InfoView::try_ref_instance_strong
///
/// # Safety
///
/// The implementor must ensure that, once passed the instance boundary, the value remains valid
/// irrespective of the liveness of the originating instance. One possible way to accomplish this is
/// to prevent the runtime from unloading the owning instance while the value is alive.
pub unsafe auto trait Share {}

impl<T> !Share for &'_ T {}
impl<T> !Share for &'_ mut T {}
impl<T> !Share for *const T {}
impl<T> !Share for *mut T {}

macro_rules! impl_share_fn_peel {
    () => {};
    ($head:ident $(, $($tail:ident),*)?) => {
        impl_share_fn!($($($tail),*)?);
    };
}
macro_rules! impl_share_fn {
    ($($T:ident),*) => {
        impl<Ret, $($T),*> !Share for fn($($T),*) -> Ret {}
        impl<Ret, $($T),*> !Share for unsafe fn($($T),*) -> Ret {}
        impl<Ret, $($T),*> !Share for extern "C" fn($($T),*) -> Ret {}
        impl<Ret, $($T),*> !Share for unsafe extern "C" fn($($T),*) -> Ret {}
        impl<Ret, $($T),*> !Share for extern "C-unwind" fn($($T),*) -> Ret {}
        impl<Ret, $($T),*> !Share for unsafe extern "C-unwind" fn($($T),*) -> Ret {}

        impl_share_fn_peel!($($T),*);
    };
}
impl_share_fn!(
    A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U, V, W, X, Y, Z
);

/// Unsafe wrapper type implementing [`Share`].
#[repr(transparent)]
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct AssertSharable<T>(T);

impl<T> AssertSharable<T> {
    /// Constructs a new instance.
    ///
    /// # Safety
    ///
    /// Callers must ensure that `value` remains valid even after the destruction of the owning
    /// instance, or ensure that the owning instance can't be unloaded while `value` is used.
    pub const unsafe fn new(value: T) -> Self {
        Self(value)
    }
}

unsafe impl<T> Share for AssertSharable<T> {}

impl<T> Deref for AssertSharable<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> DerefMut for AssertSharable<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

/// Marker trait for all types that can safely be exported as symbols.
///
/// # Safety
///
/// This trait has the same safety requirements like the [`Share`] trait, but additionally function
/// pointers where the parameters and the return types implement [`Share`].
pub unsafe trait SymbolSafe: Sync {}

macro_rules! impl_symbol_safe_fn_peel {
    () => {};
    ($head:ident $(, $($tail:ident),*)?) => {
        impl_symbol_safe_fn!($($($tail),*)?);
    };
}
macro_rules! impl_symbol_safe_fn {
    ($($T:ident),*) => {
        unsafe impl<Ret, $($T),*> SymbolSafe for extern "C" fn($($T),*) -> Ret where Ret: Share, $($T: Share),* {}
        unsafe impl<Ret, $($T),*> SymbolSafe for unsafe extern "C" fn($($T),*) -> Ret where Ret: Share, $($T: Share),* {}
        unsafe impl<Ret, $($T),*> SymbolSafe for extern "C-unwind" fn($($T),*) -> Ret where Ret: Share, $($T: Share),* {}
        unsafe impl<Ret, $($T),*> SymbolSafe for unsafe extern "C-unwind" fn($($T),*) -> Ret where Ret: Share, $($T: Share),* {}

        impl_symbol_safe_fn_peel!($($T),*);
    };
}
impl_symbol_safe_fn!(
    A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U, V, W, X, Y, Z
);

unsafe impl<T> SymbolSafe for T where T: Share + Sync + 'static {}

/// Marker trait for `*const T` and `fn(...) -> ...`.
#[const_trait]
pub trait SymbolPointer: private::Sealed + Copy + Pointer + Debug {
    /// Type of the pointee.
    type Target: SymbolSafe;

    /// Dereferences the pointer and returns a reference.
    ///
    /// # Safety
    ///
    /// The pointer must be dereferencable.
    unsafe fn as_target_ref(&self) -> &Self::Target;

    /// Constructs an opaque pointer from the pointer.
    fn into_opaque_ptr(self) -> *const ();

    /// Constructs the pointer from an opaque pointer.
    fn from_opaque_ptr(ptr: *const ()) -> Self;
}

mod private {
    pub trait Sealed {}
}

impl<T> private::Sealed for *const T {}
impl<T> const SymbolPointer for *const T
where
    T: SymbolSafe,
{
    type Target = T;

    unsafe fn as_target_ref(&self) -> &Self::Target {
        unsafe { &**self }
    }

    fn into_opaque_ptr(self) -> *const () {
        self.cast()
    }

    fn from_opaque_ptr(ptr: *const ()) -> Self {
        ptr.cast()
    }
}

macro_rules! impl_symbol_ptr_fn_peel {
    () => {};
    ($head:ident $(, $($tail:ident),*)?) => {
        impl_symbol_ptr_fn!($($($tail),*)?);
    };
}
macro_rules! impl_symbol_ptr_fn {
    ($($T:ident),*) => {
        impl<Ret, $($T),*> private::Sealed for extern "C" fn($($T),*) -> Ret
            where
                Self: SymbolSafe
        {}
        impl<Ret, $($T),*> private::Sealed for unsafe extern "C" fn($($T),*) -> Ret
            where
                Self: SymbolSafe
        {}
        impl<Ret, $($T),*> private::Sealed for extern "C-unwind" fn($($T),*) -> Ret
            where
                Self: SymbolSafe
        {}
        impl<Ret, $($T),*> private::Sealed for unsafe extern "C-unwind" fn($($T),*) -> Ret
        where
            Self: SymbolSafe
        {}

        impl<Ret, $($T),*> const SymbolPointer for extern "C" fn($($T),*) -> Ret
        where
            Self: SymbolSafe
        {
            type Target = Self;
            unsafe fn as_target_ref(&self) -> &Self::Target {
                self
            }
            fn into_opaque_ptr(self) -> *const () {
                self as _
            }
            #[allow(clippy::not_unsafe_ptr_arg_deref)]
            fn from_opaque_ptr(ptr: *const ()) -> Self {
                unsafe { std::mem::transmute::<*const (), Self>(ptr) }
            }
        }
        impl<Ret, $($T),*> const SymbolPointer for unsafe extern "C" fn($($T),*) -> Ret
        where
            Self: SymbolSafe
        {
            type Target = Self;
            unsafe fn as_target_ref(&self) -> &Self::Target {
                self
            }
            fn into_opaque_ptr(self) -> *const () {
                self as _
            }
            #[allow(clippy::not_unsafe_ptr_arg_deref)]
            fn from_opaque_ptr(ptr: *const ()) -> Self {
                unsafe { std::mem::transmute::<*const (), Self>(ptr) }
            }
        }
        impl<Ret, $($T),*> const SymbolPointer for extern "C-unwind" fn($($T),*) -> Ret
        where
            Self: SymbolSafe
        {
            type Target = Self;
            unsafe fn as_target_ref(&self) -> &Self::Target {
                self
            }
            fn into_opaque_ptr(self) -> *const () {
                self as _
            }
            #[allow(clippy::not_unsafe_ptr_arg_deref)]
            fn from_opaque_ptr(ptr: *const ()) -> Self {
                unsafe { std::mem::transmute::<*const (), Self>(ptr) }
            }
        }
        impl<Ret, $($T),*> const SymbolPointer for unsafe extern "C-unwind" fn($($T),*) -> Ret
        where
            Self: SymbolSafe
        {
            type Target = Self;
            unsafe fn as_target_ref(&self) -> &Self::Target {
                self
            }
            fn into_opaque_ptr(self) -> *const () {
                self as _
            }
            #[allow(clippy::not_unsafe_ptr_arg_deref)]
            fn from_opaque_ptr(ptr: *const ()) -> Self {
                unsafe { std::mem::transmute::<*const (), Self>(ptr) }
            }
        }

        impl_symbol_ptr_fn_peel!($($T),*);
    };
}
impl_symbol_ptr_fn!(
    A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U, V, W, X, Y, Z
);

/// Trait providing info about a symbol.
///
/// # Implementation
///
/// Is implemented automatically by the [`symbol!`](crate::symbol) macro.
pub trait SymbolInfo: Debug + Copy + PartialEq + Eq + PartialOrd + Ord + std::hash::Hash {
    /// Type of the symbol.
    type Type: SymbolPointer;
    /// Name of the symbol.
    const NAME: &'static CStr;
    /// Namespace of the symbol.
    const NAMESPACE: &'static CStr = c"";
    /// Version of the symbol.
    const VERSION: Version;
}

/// Declares new symbols.
///
/// # Examples
///
/// ```
/// use fimo_std::symbol;
///
/// symbol! {
///     symbol Foo @ (0, 0, 2) = foo: *const f32;
///     symbol Bar @ (1, 0, 0) = "my_mod"::bar: extern "C" fn(i32, i32) -> i32;
/// }
/// ```
#[macro_export]
macro_rules! symbol {
    () => {};
    (
        symbol $sym:ident @ ($major:literal, $minor:literal, $patch:literal $(, $build:literal)?) = $($ns:literal::)? $name:ident : $ty:ty;
        $($rest:tt)*
    ) => {
        #[doc = core::concat!("Marker type for the `", core::stringify!($name), "` symbol", $(" in the `", $ns, "` namespace",)? ".")]
        #[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub enum $sym {}
        impl $crate::module::symbols::SymbolInfo for $sym {
            type Type = $ty;
            const NAME: &'static core::ffi::CStr = {
                let string = core::concat!(core::stringify!($name), '\0');
                match core::ffi::CStr::from_bytes_with_nul(string.as_bytes()) {
                    Ok(x) => x,
                    Err(_) => unreachable!(),
                }
            };
            const NAMESPACE: &'static core::ffi::CStr = {
                let string = core::concat!($($ns,)? '\0');
                match core::ffi::CStr::from_bytes_with_nul(string.as_bytes()) {
                    Ok(x) => x,
                    Err(_) => unreachable!(),
                }
            };
            const VERSION: $crate::version::Version = $crate::version!($major, $minor, $patch, $($build)?);
        }

        $crate::symbol!($($rest)*);
    };
}

/// A reference to a symbol.
#[repr(transparent)]
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SymbolRef<'a, T>(T::Type, PhantomData<&'a ()>)
where
    T: SymbolInfo;

impl<T> SymbolRef<'_, T>
where
    T: SymbolInfo,
    T::Type: const SymbolPointer,
{
    /// Constructs a new `SymbolRef` from a symbol pointer.
    ///
    /// # Safety
    ///
    /// The caller must ensure that the pointer is dereferencable for the lifetime `'a` and points
    /// to a value of the type `T::Type`.
    pub const unsafe fn from_opaque_ptr<'a>(ptr: *const ()) -> SymbolRef<'a, T> {
        let ptr = T::Type::from_opaque_ptr(ptr);
        SymbolRef(ptr, PhantomData)
    }

    /// Extracts the contained pointer to the symbol.
    pub const fn into_opaque_ptr(self) -> *const () {
        self.0.into_opaque_ptr()
    }
}

impl<T> Debug for SymbolRef<'_, T>
where
    T: SymbolInfo,
    <T::Type as SymbolPointer>::Target: Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("SymbolRef").field(&**self).finish()
    }
}

impl<T> Pointer for SymbolRef<'_, T>
where
    T: SymbolInfo,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Pointer::fmt(&self.0, f)
    }
}

impl<T> Deref for SymbolRef<'_, T>
where
    T: SymbolInfo,
{
    type Target = <T::Type as SymbolPointer>::Target;

    fn deref(&self) -> &Self::Target {
        unsafe { self.0.as_target_ref() }
    }
}

/// Helper trait for types that can provide access to some specific symbol.
pub trait SymbolProvider<T>: Sized + Copy
where
    T: SymbolInfo,
{
    fn access<'a>(self) -> SymbolRef<'a, T>
    where
        Self: 'a;
}

impl<T> SymbolProvider<T> for SymbolRef<'_, T>
where
    T: SymbolInfo,
{
    fn access<'a>(self) -> SymbolRef<'a, T>
    where
        Self: 'a,
    {
        self
    }
}
