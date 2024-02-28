use core::{ffi::CStr, marker::PhantomData, ops::Deref};

use crate::{error::Error, version::Version};

use super::OpaqueModule;

/// A symbol of a module.
#[derive(Clone, Copy)]
pub struct Symbol<'a, T>(*const core::ffi::c_void, PhantomData<&'a T>);

impl<T> Deref for Symbol<'_, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        // Safety: Is safe by contract.
        unsafe { &*self.0.cast::<T>() }
    }
}

// Safety: Symbol is essentially a `&'a T`.
unsafe impl<T> Send for Symbol<'_, T> where T: Send {}

// Safety: Symbol is essentially a `&'a T`.
unsafe impl<T> Sync for Symbol<'_, T> where T: Sync {}

impl<T> crate::ffi::FFITransferable<*const core::ffi::c_void> for Symbol<'_, T> {
    fn into_ffi(self) -> *const core::ffi::c_void {
        self.0
    }

    unsafe fn from_ffi(ffi: *const core::ffi::c_void) -> Self {
        Self(ffi, PhantomData)
    }
}

/// Information of a symbol namespace.
pub trait NamespaceInfo {
    /// Name of the namespace.
    const NAME: &'static CStr;
}

/// Global namespace for symbols.
pub struct GlobalNs;

impl NamespaceInfo for GlobalNs {
    const NAME: &'static CStr = c"";
}

/// Common information of an exported item.
pub trait SymbolInfo {
    /// Name of the export.
    const NAME: &'static CStr;

    /// Namespace of the export.
    type Namespace: NamespaceInfo;

    /// Version of the export.
    const VERSION: Version;
}

/// Information of a statically exported item.
pub trait StaticSymbolInfo: SymbolInfo {
    /// Type of the exported item.
    type Type: 'static + Sync;

    /// Symbol to export.
    const SYMBOL: &'static Self::Type;
}

/// Information of a dynamically exported item.
pub trait DynamicSymbolInfo: SymbolInfo {
    /// Type of the exported item.
    type Type: Sync;

    /// Constructor for the item.
    const NEW: fn(OpaqueModule<'static>, *const ()) -> Result<Self::Type, Error>;

    /// Drop function for the item.
    const DROP: fn(Self::Type, *const ());
}

/// Specialization of [`Drop`] for symbols.
pub trait SymbolDrop: Sized {
    /// Drops the symbol.
    fn drop_symbol(self);
}

impl<T> SymbolDrop for T {
    default fn drop_symbol(self) {
        core::mem::drop(self);
    }
}

#[doc(hidden)]
pub fn drop_symbol_private(symbol: impl SymbolDrop) {
    symbol.drop_symbol();
}

/// Creates items describing the exported symbols and namespaces.
///
/// # Examples
///
/// ## Static symbol
///
/// ```
/// use fimo_std::declare_exports;
///
/// declare_exports! {
///     // Creates the item `SymbolExport` with the version "1.2.3".
///     static symbol @ (1, 2, 3): usize = 5;
/// }
/// ```
///
/// ## Dynamic symbol
///
/// ```
/// use fimo_std::declare_exports;
///
/// declare_exports! {
///     // Creates the item `DynamicSymbolExport` with the version "1.2.3+4".
///     dyn dynamic_symbol @ (1, 2, 3, 4): Box<usize> = |module| Ok(Box::new(5));
/// }
/// ```
///
/// ## New namespaces
///
/// ```
/// use fimo_std::declare_exports;
///
/// declare_exports! {
///     // Creates the namespace item `MyNsNs`.
///     mod my_ns {
///         // Creates the symbol `MyNs_ASymExport`.
///         static a_sym @ (1, 2, 3): usize = 5;
///         // Creates the symbol `MyNs_BSymExport`.
///         dyn b_sym @ (1, 2, 3, 4): Box<usize> = |module| Ok(Box::new(5));
///     }
/// }
/// ```
///
/// ## Existing namespaces
///
/// ```
/// use fimo_std::declare_exports;
/// use fimo_std::module::GlobalNs;
///
/// declare_exports! {
///     // Creates the new items in the `GlobalNs` namespace.
///     mod global: GlobalNs {
///         // Creates the symbol `MyNs_ASymExport`.
///         static a_sym @ (1, 2, 3): usize = 5;
///         // Creates the symbol `MyNs_BSymExport`.
///         dyn b_sym @ (1, 2, 3, 4): Box<usize> = |module| Ok(Box::new(5));
///     }
/// }
/// ```
#[macro_export]
macro_rules! declare_exports {
    () => {};
    (static $name:ident @ ($major:literal, $minor:literal, $patch:literal $(, $build:literal)? $(,)?): $type:ty = $expr:expr; $($tt:tt)*) => {
        $crate::declare_exports_private!(item static : $crate::module::GlobalNs; $name @ ($major, $minor, $patch $(, $build)?): $type = $expr;);
        $crate::declare_exports!($($tt)*);
    };
    (dyn $name:ident @ ($major:literal, $minor:literal, $patch:literal $(, $build:literal)? $(,)?): $type:ty = $expr:expr; $($tt:tt)*) => {
        $crate::declare_exports_private!(item dyn : $crate::module::GlobalNs; $name @ ($major, $minor, $patch $(, $build)?): $type = $expr;);
        $crate::declare_exports!($($tt)*);
    };
    (mod $ns:ident: $ns_type:ty { $($block:tt)* } $($tt:tt)*) => {
        $crate::declare_exports_private!(namespace $ns: $ns_type; $($block)*);
        $crate::declare_exports!($($tt)*);
    };
    (mod $ns:ident { $($block:tt)* } $($tt:tt)*) => {
        $crate::paste::paste! {
            #[doc = "Namespace `" $ns "`"]
            pub struct [<$ns:camel Ns>];
            impl $crate::module::NamespaceInfo for [<$ns:camel Ns>] {
                const NAME: &'static core::ffi::CStr = match core::ffi::CStr::from_bytes_with_nul(
                    core::concat!(core::stringify!($ns), '\0').as_bytes()
                ) {
                    Ok(x) => x,
                    Err(_) => unreachable!()
                };
            }

            $crate::declare_exports_private!(namespace $ns: [<$ns:camel Ns>]; $($block)*);
        }

        $crate::declare_exports!($($tt)*);
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! declare_exports_private {
    () => {};
    (namespace $($ns:ident)?: $ns_type:ty;) => {};
    (namespace $($ns:ident)?: $ns_type:ty; { $($tt:tt)* }) => {
        $crate::declare_exports_private!(namespace $($ns)?: $ns_type; $($tt)*)
    };
    (namespace $($ns:ident)?: $ns_type:ty; static $name:ident @ ($major:literal, $minor:literal, $patch:literal $(, $build:literal)? $(,)?): $type:ty = $expr:expr; $($tt:tt)*) => {
        $crate::declare_exports_private!(item static $($ns)?: $ns_type; $name @ ($major, $minor, $patch $(, $build)?): $type = $expr;);
        $crate::declare_exports_private!(namespace $($ns)?: $ns_type; $($tt)*);
    };
    (namespace $($ns:ident)?: $ns_type:ty; dyn $name:ident @ ($major:literal, $minor:literal, $patch:literal $(, $build:literal)? $(,)?): $type:ty = $expr:expr; $($tt:tt)*) => {
        $crate::declare_exports_private!(item dyn $($ns)?: $ns_type; $name @ ($major, $minor, $patch $(, $build)?) : $type = $expr;);
        $crate::declare_exports_private!(namespace $($ns)?: $ns_type; $($tt)*);
    };
    (item static $($ns:ident)?: $ns_type:ty; $name:ident @ ($major:literal, $minor:literal, $patch:literal $(, $build:literal)?) : $type:ty = $expr:expr;) => {
        $crate::paste::paste! {
            #[allow(non_camel_case_types)]
            #[doc = "Export `" $name "`" $(" in the namespace `" $ns "`")? "."]
            pub struct [<$($ns:camel _)? $name:camel Export>];
            impl [<$($ns:camel _)? $name:camel Export>] {
                const SYMBOL_VALUE: $type = { $expr };
            }
            impl $crate::module::SymbolInfo for [<$($ns:camel _)? $name:camel Export>] {
                const NAME: &'static core::ffi::CStr = match core::ffi::CStr::from_bytes_with_nul(
                    core::concat!(core::stringify!($name), '\0').as_bytes()
                ) {
                    Ok(x) => x,
                    Err(_) => unreachable!()
                };
                type Namespace = $ns_type;
                const VERSION: $crate::version::Version = $crate::version!($major, $minor, $patch, $($build)?);
            }
            impl $crate::module::StaticSymbolInfo for [<$($ns:camel _)? $name:camel Export>]
            {
                type Type = $type;
                const SYMBOL: &'static Self::Type = &[<$($ns:camel _)? $name:camel Export>]::SYMBOL_VALUE;
            }
        }
    };
    (item dyn $($ns:ident)?: $ns_type:ty; $name:ident @ ($major:literal, $minor:literal, $patch:literal $(, $build:literal)?) : $type:ty = $expr:expr;) => {
        $crate::paste::paste! {
            #[allow(non_camel_case_types)]
            #[doc = "Export `" $name "`" $(" in the namespace `" $ns "`")? "."]
            pub struct [<$($ns:camel _)? $name:camel Export>];
            impl [<$($ns:camel _)? $name:camel Export>] {
                fn construct(
                    module: $crate::module::OpaqueModule<'static>,
                    _reserved: *const ()
                ) -> Result<$type, $crate::error::Error> {
                    ($expr)(module)
                }

                fn drop(symbol: $type, _: *const ()) {
                    $crate::module::drop_symbol_private(symbol);
                }
            }
            impl $crate::module::SymbolInfo for [<$($ns:camel _)? $name:camel Export>] {
                const NAME: &'static core::ffi::CStr = match core::ffi::CStr::from_bytes_with_nul(
                    core::concat!(core::stringify!($name), '\0').as_bytes()
                ) {
                    Ok(x) => x,
                    Err(_) => unreachable!()
                };
                type Namespace = $ns_type;
                const VERSION: $crate::version::Version = $crate::version!($major, $minor, $patch, $($build)?);
            }
            impl $crate::module::DynamicSymbolInfo for [<$($ns:camel _)? $name:camel Export>]
            {
                type Type = $type;
                const NEW: fn(
                    $crate::module::OpaqueModule<'static>,
                    *const ()
                ) -> Result<Self::Type, $crate::error::Error> = [<$($ns:camel _)? $name:camel Export>]::construct;
                const DROP: fn(Self::Type, *const ()) = [<$($ns:camel _)? $name:camel Export>]::drop;
            }
        }
    };
}
