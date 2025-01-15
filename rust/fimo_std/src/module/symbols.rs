use crate::module::{GenericInstance, InfoView};
use std::{
    marker::PhantomData,
    ops::{Deref, DerefMut},
    pin::Pin,
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
/// the [`GenericInstance::to_owned_instance`] or [`InfoView::try_ref_instance_strong`] methods.
///
/// This trait marks whether a type is safe to pass the instance boundary without breaking the
/// assumption of isolation. Notably, this can not be guaranteed for pointer-like types, like
/// references and functions, as they potentially point to data owned by the instance.
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

/// Marker trait for instances that can not be hot-swapped.
///
/// # Safety
///
/// Can only be implemented if the instance did not specify hot-reload support in the module export.
/// Should not be implemented manually.
pub unsafe trait StableInstance: GenericInstance {}

/// Unsafe wrapper type implementing `[Share]`.
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

#[repr(transparent)]
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct InstanceOwned<T, I>(T, PhantomData<fn() -> I>)
where
    I: GenericInstance;

unsafe impl<T, I> Share for InstanceOwned<T, I> where I: StableInstance {}
