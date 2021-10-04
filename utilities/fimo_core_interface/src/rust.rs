//! Definition of the Rust `fimo-core` interface.
use fimo_ffi_core::ArrayString;
use fimo_module_core::rust::{ModuleInstance, ModuleInstanceArc, ModuleInterfaceArc};
use fimo_module_core::{DynArc, DynArcBase, DynArcCaster, ModuleInterfaceDescriptor, ModulePtr};
use fimo_version_core::{ReleaseType, Version};
use std::any::Any;
use std::sync::Arc;

/// Version the library was linked with.
pub const PKG_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Name of the interface.
pub const INTERFACE_NAME: &str = "fimo-core";

/// Implemented interface version.
pub const INTERFACE_VERSION: Version = Version::new_long(0, 1, 0, ReleaseType::Unstable, 0);

pub mod module_registry;
pub mod settings_registry;

/// Implements part of the [ModuleInstance] vtable for fimo modules.
#[macro_export]
macro_rules! fimo_module_instance_impl {
    (id) => {
        "fimo::module_instance::core"
    };
    (to_ptr, $instance: expr) => {
        unsafe {
            fimo_module_core::ModulePtr::Fat(std::mem::transmute(
                $instance as &dyn $crate::rust::FimoModuleInstanceExtAPIStable,
            ))
        }
    };
    (trait_impl, $instance: ty) => {
        impl $crate::rust::FimoModuleInstanceExtAPIStable for $instance {
            fn pkg_version(&self) -> &str {
                $crate::rust::PKG_VERSION
            }

            fn as_module_instance(&self) -> &fimo_module_core::rust::ModuleInstance {
                &**self
            }

            fn as_fimo_module_instance(
                &self,
            ) -> &(dyn $crate::rust::FimoModuleInstanceExt + 'static) {
                self
            }
        }
    };
}

/// Implements parts of the [`ModuleInterface`] vtable for the `fimo-core` interface.
///
/// [`ModuleInterface`]: fimo_module_core::rust::ModuleInterface
#[macro_export]
macro_rules! fimo_core_interface_impl {
    (id) => {
        "fimo::interface::core"
    };
    (to_ptr, $vtable: expr) => {
        fimo_module_core::ModulePtr::Slim(&$vtable as *const _ as *const u8)
    };
}

/// Type-erased `fimo-core` interface.
///
/// The underlying type must implement `Send` and `Sync`.
pub struct FimoCore {
    // makes `FimoCore` into a DST with size 0 and alignment 1.
    _inner: [()],
}

impl FimoCore {
    /// Fetches the version of the interface.
    #[inline]
    pub fn get_interface_version(&self) -> Version {
        let (ptr, vtable) = self.into_raw_parts();
        (vtable.get_interface_version)(ptr)
    }

    /// Fetches for an extension of the interface.
    #[inline]
    pub fn find_extension(&self, extension: &str) -> Option<&(dyn Any + 'static)> {
        let (ptr, vtable) = self.into_raw_parts();
        (vtable.find_extension)(ptr, extension).map(|e| unsafe { &*e })
    }

    /// Fetches the module registry.
    #[inline]
    pub fn get_module_registry(&self) -> &module_registry::ModuleRegistry {
        let (ptr, vtable) = self.into_raw_parts();
        unsafe { &*(vtable.get_module_registry)(ptr) }
    }

    /// Fetches the settings registry.
    #[inline]
    pub fn get_settings_registry(&self) -> &settings_registry::SettingsRegistry {
        let (ptr, vtable) = self.into_raw_parts();
        unsafe { &*(vtable.get_settings_registry)(ptr) }
    }

    /// Splits the reference into a data- and vtable- pointer.
    #[inline]
    pub fn into_raw_parts(&self) -> (*const (), &'static FimoCoreVTable) {
        // safety: `&Self` has the same layout as `&[()]`
        let s: &[()] = unsafe { std::mem::transmute(self) };

        // safety: the values are properly initialized upon construction.
        let ptr = s.as_ptr();
        let vtable = unsafe { &*(s.len() as *const FimoCoreVTable) };

        (ptr, vtable)
    }

    /// Constructs a `*const FimoCore` from a data- and vtable- pointer.
    #[inline]
    pub fn from_raw_parts(data: *const (), vtable: &'static FimoCoreVTable) -> *const Self {
        // `()` has size 0 and alignment 1, so it should be sound to use an
        // arbitrary ptr and length.
        let vtable_ptr = vtable as *const _ as usize;
        let s = std::ptr::slice_from_raw_parts(data, vtable_ptr);

        // safety: the types have the same layout
        unsafe { std::mem::transmute(s) }
    }
}

impl std::fmt::Debug for FimoCore {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "(FimoCore)")
    }
}

unsafe impl Send for FimoCore {}
unsafe impl Sync for FimoCore {}

/// VTable of the `fimo-core` interface.
#[repr(C)]
#[derive(Copy, Clone, Debug, Ord, PartialOrd, Eq, PartialEq, Hash)]
pub struct FimoCoreVTable {
    get_interface_version: fn(*const ()) -> Version,
    find_extension: fn(*const (), *const str) -> Option<*const (dyn Any + 'static)>,
    get_module_registry: fn(*const ()) -> *const module_registry::ModuleRegistry,
    get_settings_registry: fn(*const ()) -> *const settings_registry::SettingsRegistry,
}

impl FimoCoreVTable {
    /// Constructs a new `FimoCoreVTable`.
    pub const fn new(
        get_interface_version: fn(*const ()) -> Version,
        find_extension: fn(*const (), *const str) -> Option<*const (dyn Any + 'static)>,
        get_module_registry: fn(*const ()) -> *const module_registry::ModuleRegistry,
        get_settings_registry: fn(*const ()) -> *const settings_registry::SettingsRegistry,
    ) -> Self {
        Self {
            get_interface_version,
            find_extension,
            get_module_registry,
            get_settings_registry,
        }
    }
}

/// [`DynArc`] caster for [`FimoCore`].
#[derive(PartialEq, Copy, Clone, Debug)]
pub struct FimoCoreCaster {
    vtable: &'static FimoCoreVTable,
}

impl FimoCoreCaster {
    /// Constructs a new `FimoCoreCaster`.
    pub fn new(vtable: &'static FimoCoreVTable) -> Self {
        Self { vtable }
    }
}

impl DynArcCaster<FimoCore> for FimoCoreCaster {
    unsafe fn as_self_ptr<'a>(&self, base: *const (dyn DynArcBase + 'a)) -> *const FimoCore {
        let data = base as *const ();
        FimoCore::from_raw_parts(data, self.vtable)
    }
}

/// API stable trait for identifying a fimo module.
///
/// Changing this trait is a breaking change because it is used to identify
/// version mismatches. The trait **must** be implemented using the
/// [`fimo_module_instance_impl!{}`] macro.
pub trait FimoModuleInstanceExtAPIStable {
    /// Extracts the linked package version of this crate.
    ///
    /// Must always be [PKG_VERSION].
    fn pkg_version(&self) -> &str;

    /// Casts itself to a `&(dyn FimoModuleInstanceExt + 'static)`.
    fn as_module_instance(&self) -> &ModuleInstance;

    /// Casts itself to a `&(dyn FimoModuleInstanceExt + 'static)`.
    fn as_fimo_module_instance(&self) -> &(dyn FimoModuleInstanceExt + 'static);
}

/// A trait describing a fimo module.
pub trait FimoModuleInstanceExt: FimoModuleInstanceExtAPIStable {
    /// Extracts the linked package version of a crate.
    fn get_pkg_version(&self, pkg: &str) -> Option<&str>;
}

/// Casts an generic interface to a `fimo-core` interface.
///
/// # Safety
///
/// This function is highly unsafe as the compiler can not check the
/// validity of the cast. The interface **must** be implemented using the
/// [`fimo_core_interface_impl!{}`] macro.
pub unsafe fn cast_interface(
    interface: ModuleInterfaceArc,
) -> Result<DynArc<FimoCore, FimoCoreCaster>, std::io::Error> {
    #[allow(unused_unsafe)]
    if interface.get_raw_type_id() != fimo_core_interface_impl! {id} {
        return Err(std::io::Error::new(
            std::io::ErrorKind::Other,
            "Type mismatch",
        ));
    }

    match interface.get_raw_ptr() {
        ModulePtr::Slim(ptr) => {
            let vtable = &*(ptr as *const FimoCoreVTable);
            let caster = FimoCoreCaster::new(vtable);
            let (arc, _) = ModuleInterfaceArc::into_inner(interface);

            Ok(DynArc::from_inner((arc, caster)))
        }
        _ => Err(std::io::Error::new(
            std::io::ErrorKind::Other,
            "Pointer layout mismatch",
        )),
    }
}

/// Casts a generic module instance to a fimo module instance.
///
/// # Safety
///
/// This function is highly unsafe as the compiler can not check the
/// validity of the cast. The instance **must** be implemented using the
/// [`fimo_module_instance_impl!{}`] macro.
pub unsafe fn cast_instance(
    instance: ModuleInstanceArc,
) -> Result<Arc<dyn FimoModuleInstanceExt>, std::io::Error> {
    sa::assert_eq_size!(
        &ModuleInstance,
        &dyn FimoModuleInstanceExt,
        &dyn FimoModuleInstanceExtAPIStable,
        (*const u8, *const u8)
    );
    sa::assert_eq_align!(&ModuleInstance, &dyn FimoModuleInstanceExt,);

    #[allow(unused_unsafe)]
    if instance.get_raw_type_id() != fimo_module_instance_impl! {id} {
        return Err(std::io::Error::new(
            std::io::ErrorKind::Other,
            "Type mismatch",
        ));
    }

    match instance.get_raw_ptr() {
        ModulePtr::Fat(ptr) => {
            let ext_stable: &dyn FimoModuleInstanceExtAPIStable = std::mem::transmute(ptr);

            if PKG_VERSION != ext_stable.pkg_version() {
                Err(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    "Version mismatch",
                ))
            } else {
                std::mem::forget(instance);
                let ext = ext_stable.as_fimo_module_instance();
                Ok(Arc::from_raw(ext as *const _))
            }
        }
        _ => Err(std::io::Error::new(
            std::io::ErrorKind::Other,
            "Pointer layout mismatch",
        )),
    }
}

/// Builds the [`ModuleInterfaceDescriptor`] for the interface.
pub fn build_interface_descriptor() -> ModuleInterfaceDescriptor {
    ModuleInterfaceDescriptor {
        name: unsafe { ArrayString::from_utf8_unchecked(INTERFACE_NAME.as_bytes()) },
        version: INTERFACE_VERSION,
        extensions: Default::default(),
    }
}
