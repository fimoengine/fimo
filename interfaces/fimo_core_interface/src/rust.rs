//! Definition of the Rust `fimo-core` interface.
use fimo_ffi::ArrayString;
use fimo_module_core::{
    fimo_interface, fimo_vtable, FimoInterface, ModuleInterfaceDescriptor, SendSyncMarker,
};
use fimo_version_core::{ReleaseType, Version};

pub mod module_registry;
pub mod settings_registry;

fimo_interface! {
    /// Type-erased `fimo-core` interface.
    ///
    /// The underlying type must implement `Send` and `Sync`.
    pub struct FimoCore<vtable = FimoCoreVTable> {
        name: "fimo::interfaces::core::fimo_core",
        version: Version::new_long(0, 1, 0, ReleaseType::Unstable, 0),
    }
}

impl FimoCore {
    /// Fetches the module registry.
    #[inline]
    pub fn get_module_registry(&self) -> &module_registry::IModuleRegistry {
        let (ptr, vtable) = self.into_raw_parts();
        unsafe { &*(vtable.get_module_registry)(ptr) }
    }

    /// Fetches the settings registry.
    #[inline]
    pub fn get_settings_registry(&self) -> &settings_registry::SettingsRegistry {
        let (ptr, vtable) = self.into_raw_parts();
        unsafe { &*(vtable.get_settings_registry)(ptr) }
    }
}

fimo_vtable! {
    /// VTable of the `fimo-core` interface.
    #[derive(Copy, Clone, Debug, Ord, PartialOrd, Eq, PartialEq, Hash)]
    pub struct FimoCoreVTable<id = "fimo::interfaces::core::fimo_core", marker = SendSyncMarker> {
        /// Fetches the module registry.
        pub get_module_registry: fn(*const ()) -> *const module_registry::IModuleRegistry,
        /// Fetches the settings registry.
        pub get_settings_registry: fn(*const ()) -> *const settings_registry::SettingsRegistry,
    }
}

/// Builds the [`ModuleInterfaceDescriptor`] for the interface.
pub fn build_interface_descriptor() -> ModuleInterfaceDescriptor {
    ModuleInterfaceDescriptor {
        name: unsafe { ArrayString::from_utf8_unchecked(FimoCore::NAME.as_bytes()) },
        version: FimoCore::VERSION,
        extensions: Default::default(),
    }
}
