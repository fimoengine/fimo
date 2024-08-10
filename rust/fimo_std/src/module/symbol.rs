use core::{ffi::CStr, marker::PhantomData, ops::Deref};

use crate::{
    error::Error,
    module::{GenericModule, Module},
    version::Version,
};

/// A symbol of a module.
#[repr(transparent)]
#[derive(Clone, Copy)]
pub struct Symbol<'a, T>(*const std::ffi::c_void, PhantomData<&'a T>);

// Safety: Symbol is essentially a `&T`.
unsafe impl<T> Send for Symbol<'_, T> where T: Send {}

// Safety: Symbol is essentially a `&T`.
unsafe impl<T> Sync for Symbol<'_, T> where T: Sync {}

impl<T> crate::ffi::FFITransferable<*const std::ffi::c_void> for Symbol<'_, T> {
    fn into_ffi(self) -> *const std::ffi::c_void {
        self.0
    }

    unsafe fn from_ffi(ffi: *const std::ffi::c_void) -> Self {
        Self(ffi, PhantomData)
    }
}

impl<T> Deref for Symbol<'_, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        // Safety: We hold a reference to a `T`.
        unsafe { &*self.0.cast::<T>() }
    }
}

/// Information of a symbol namespace.
pub trait NamespaceItem {
    /// Name of the namespace.
    const NAME: &'static CStr;
}

/// Global namespace for symbols.
pub struct GlobalNs;

impl NamespaceItem for GlobalNs {
    const NAME: &'static CStr = c"";
}

/// Common information of an exported symbol item.
pub trait SymbolItem {
    /// Type of the symbol.
    type Type;

    /// Name of the export.
    const NAME: &'static CStr;

    /// Namespace of the export.
    type Namespace: NamespaceItem;

    /// Version of the export.
    const VERSION: Version;
}

/// A partially constructed module.
pub type PartialModule<'a, T> = GenericModule<
    'a,
    <T as Module>::Parameters,
    <T as Module>::Resources,
    <T as Module>::Imports,
    core::mem::MaybeUninit<<T as Module>::Exports>,
    <T as Module>::Data,
>;

/// Helper trait for constructing and destroying dynamic symbols.
pub trait DynamicExport<T>
where
    T: Module,
{
    /// [`SymbolItem`] describing the symbol.
    type Item: SymbolItem;

    /// Constructs a new instance of the symbol.
    fn construct(
        module: PartialModule<'_, T>,
    ) -> Result<&mut <Self::Item as SymbolItem>::Type, Error>;

    /// Destroys the symbol.
    fn destroy(symbol: &mut <Self::Item as SymbolItem>::Type);
}

/// Creates symbol and namespace items that can later be used for import and export.
///
/// # Examples
///
/// ## Global symbol
///
/// ```
/// use fimo_std::declare_items;
///
/// declare_items! {
///     // Creates the item `Symbol` with the version "1.2.3".
///     extern symbol @ (1, 2, 3): usize;
/// }
/// ```
///
/// ## New namespaces
///
/// ```
/// use fimo_std::declare_items;
///
/// declare_items! {
///     // Creates the namespace item `my_ns::NamespaceItem`.
///     mod my_ns {
///         // Creates the symbol `my_ns::ASym`.
///         extern a_sym @ (1, 2, 3): usize;
///     }
/// }
/// ```
///
/// ## Existing namespaces
///
/// ```
/// use fimo_std::{declare_items, module::GlobalNs};
///
/// declare_items! {
///     // Creates the new items in the `GlobalNs` namespace.
///     // Also creates the namespace item type alias `global::NamespaceItem`.
///     mod global = fimo_std::module::GlobalNs {
///         // Creates the symbol `global::ASym`.
///         extern a_sym @ (1, 2, 3): usize;
///     }
/// }
/// ```
#[macro_export]
macro_rules! declare_items {
    () => {};
    (extern $name:ident @ ($major:literal, $minor:literal, $patch:literal $(, $build:literal)? $(,)?): $type:ty; $($tt:tt)*) => {
        $crate::declare_items_private!(item $crate::module::GlobalNs; $name @ ($major, $minor, $patch $(, $build)?): $type;);
        $crate::declare_items!($($tt)*);
    };
    (mod $ns:ident = $ns_type:path { $($block:tt)* } $($tt:tt)*) => {
        $crate::paste::paste! {
            #[doc = "Namespace `" $ns "`"]
            pub mod $ns {
                #[allow(unused_imports)]
                use super::*;
                pub type NamespaceItem = $ns_type;
                $crate::declare_items_private!(namespace NamespaceItem $($block)*);
            }
        }
        $crate::declare_items!($($tt)*);
    };
    (mod $ns:ident { $($block:tt)* } $($tt:tt)*) => {
        $crate::paste::paste! {
            #[doc = "Namespace `" $ns "`"]
            pub mod $ns {
                #[allow(unused_imports)]
                use super::*;
                #[doc = "Namespace `" $ns "` item"]
                pub struct NamespaceItem;
                impl $crate::module::NamespaceItem for NamespaceItem {
                    const NAME: &'static core::ffi::CStr = match core::ffi::CStr::from_bytes_with_nul(
                        core::concat!(core::stringify!($ns), '\0').as_bytes()
                    ) {
                        Ok(x) => x,
                        Err(_) => unreachable!()
                    };
                }
                $crate::declare_items_private!(namespace NamespaceItem $($block)*);
            }
        }
        $crate::declare_items!($($tt)*);
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! declare_items_private {
    (item $ns_type:ty; $name:ident @ ($major:literal, $minor:literal, $patch:literal $(, $build:literal)?) : $type:ty;) => {
        $crate::paste::paste! {
            #[allow(non_camel_case_types)]
            #[doc = "Symbol `" $name "`."]
            pub struct [<$name:camel>];
            impl $crate::module::SymbolItem for [<$name:camel>] {
                type Type = $type;
                const NAME: &'static core::ffi::CStr = match core::ffi::CStr::from_bytes_with_nul(
                    core::concat!(core::stringify!($name), '\0').as_bytes()
                ) {
                    Ok(x) => x,
                    Err(_) => unreachable!()
                };
                type Namespace = $ns_type;
                const VERSION: $crate::version::Version = $crate::version!($major, $minor, $patch, $($build)?);
            }
        }
    };
    (namespace $ns_type:ident) => {};
    (namespace $ns_type:ident extern $name:ident @ ($major:literal, $minor:literal, $patch:literal $(, $build:literal)? $(,)?): $type:ty; $($tt:tt)*) => {
        $crate::declare_items_private!(item $ns_type; $name @ ($major, $minor, $patch $(, $build)?): $type;);
        $crate::declare_items_private!(namespace $ns_type $($tt)*);
    };
}
