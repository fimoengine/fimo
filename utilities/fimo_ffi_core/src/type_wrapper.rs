//! Implementation of the `TypeWrapper<T>` type.

/// Wrapper around a type.
///
/// Is currently used to implement traits on existing types.
#[repr(transparent)]
pub struct TypeWrapper<T>(pub T);

// Impls for function pointers
// Modified from https://doc.rust-lang.org/src/core/ptr/mod.rs.html#1484
macro_rules! fnptr_impls_safety_abi {
    ($FnTy: ty, $($Arg: ident),*) => {
        impl<Ret, $($Arg),*> PartialEq for TypeWrapper<$FnTy> {
            #[inline]
            fn eq(&self, other: &Self) -> bool {
                self.0 as usize == other.0 as usize
            }
        }

        impl<Ret, $($Arg),*> Copy for TypeWrapper<$FnTy> {}

        impl<Ret, $($Arg),*> Clone for TypeWrapper<$FnTy> {
            fn clone(&self) -> Self {
                *self
            }
        }

        impl<Ret, $($Arg),*> Eq for TypeWrapper<$FnTy> {}

        impl<Ret, $($Arg),*> PartialOrd for TypeWrapper<$FnTy> {
            #[inline]
            fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
                (self.0 as usize).partial_cmp(&(other.0 as usize))
            }
        }

        impl<Ret, $($Arg),*> Ord for TypeWrapper<$FnTy> {
            #[inline]
            fn cmp(&self, other: &Self) -> std::cmp::Ordering {
                (self.0 as usize).cmp(&(other.0 as usize))
            }
        }

        impl<Ret, $($Arg),*> std::hash::Hash for TypeWrapper<$FnTy> {
            fn hash<HH: std::hash::Hasher>(&self, state: &mut HH) {
                state.write_usize(self.0 as usize)
            }
        }

        impl<Ret, $($Arg),*> std::ops::Deref for TypeWrapper<$FnTy> {
            type Target = $FnTy;

            #[inline]
            fn deref(&self) -> &Self::Target {
                &self.0
            }
        }

        impl<Ret, $($Arg),*> std::ops::DerefMut for TypeWrapper<$FnTy> {
            #[inline]
            fn deref_mut(&mut self) -> &mut Self::Target {
                &mut self.0
            }
        }

        impl<Ret, $($Arg),*> std::fmt::Pointer for TypeWrapper<$FnTy> {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                // HACK: The intermediate cast as usize is required for AVR
                // so that the address space of the source function pointer
                // is preserved in the final function pointer.
                //
                // https://github.com/avr-rust/rust/issues/143
                std::fmt::Pointer::fmt(&(self.0 as usize as *const ()), f)
            }
        }

        impl<Ret, $($Arg),*> std::fmt::Debug for TypeWrapper<$FnTy> {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                // HACK: The intermediate cast as usize is required for AVR
                // so that the address space of the source function pointer
                // is preserved in the final function pointer.
                //
                // https://github.com/avr-rust/rust/issues/143
                std::fmt::Pointer::fmt(&(self.0 as usize as *const ()), f)
            }
        }
    }
}

macro_rules! fnptr_impls_args {
    ($($Arg: ident),+) => {
        fnptr_impls_safety_abi! { extern "Rust" fn($($Arg),+) -> Ret, $($Arg),+ }
        fnptr_impls_safety_abi! { extern "C" fn($($Arg),+) -> Ret, $($Arg),+ }
        fnptr_impls_safety_abi! { extern "C" fn($($Arg),+ , ...) -> Ret, $($Arg),+ }
        fnptr_impls_safety_abi! { extern "C-unwind" fn($($Arg),+) -> Ret, $($Arg),+ }
        fnptr_impls_safety_abi! { extern "C-unwind" fn($($Arg),+ , ...) -> Ret, $($Arg),+ }
        fnptr_impls_safety_abi! { unsafe extern "Rust" fn($($Arg),+) -> Ret, $($Arg),+ }
        fnptr_impls_safety_abi! { unsafe extern "C" fn($($Arg),+) -> Ret, $($Arg),+ }
        fnptr_impls_safety_abi! { unsafe extern "C" fn($($Arg),+ , ...) -> Ret, $($Arg),+ }
        fnptr_impls_safety_abi! { unsafe extern "C-unwind" fn($($Arg),+) -> Ret, $($Arg),+ }
        fnptr_impls_safety_abi! { unsafe extern "C-unwind" fn($($Arg),+ , ...) -> Ret, $($Arg),+ }
    };
    () => {
        // No variadic functions with 0 parameters
        fnptr_impls_safety_abi! { extern "Rust" fn() -> Ret, }
        fnptr_impls_safety_abi! { extern "C" fn() -> Ret, }
        fnptr_impls_safety_abi! { extern "C-unwind" fn() -> Ret, }
        fnptr_impls_safety_abi! { unsafe extern "Rust" fn() -> Ret, }
        fnptr_impls_safety_abi! { unsafe extern "C" fn() -> Ret, }
        fnptr_impls_safety_abi! { unsafe extern "C-unwind" fn() -> Ret, }
    };
}

fnptr_impls_args! {}
fnptr_impls_args! { A }
fnptr_impls_args! { A, B }
fnptr_impls_args! { A, B, C }
fnptr_impls_args! { A, B, C, D }
fnptr_impls_args! { A, B, C, D, E }
fnptr_impls_args! { A, B, C, D, E, F }
fnptr_impls_args! { A, B, C, D, E, F, G }
fnptr_impls_args! { A, B, C, D, E, F, G, H }
fnptr_impls_args! { A, B, C, D, E, F, G, H, I }
fnptr_impls_args! { A, B, C, D, E, F, G, H, I, J }
fnptr_impls_args! { A, B, C, D, E, F, G, H, I, J, K }
fnptr_impls_args! { A, B, C, D, E, F, G, H, I, J, K, L }
