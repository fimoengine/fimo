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
    pub struct IFimoCore<vtable = IFimoCoreVTable> {
        name: "fimo::interfaces::core::fimo_core",
        version: Version::new_long(0, 1, 0, ReleaseType::Unstable, 0),
    }
}

impl IFimoCore {
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
    /// VTable of an [`IFimoCore`].
    #[derive(Copy, Clone, Debug, Ord, PartialOrd, Eq, PartialEq, Hash)]
    #![marker = SendSyncMarker]
    #![uuid(0xc2173cd4, 0x767c, 0x4ac2, 0xa8ac, 0x52a2cbebda0a)]
    pub struct IFimoCoreVTable {
        /// Fetches the module registry.
        pub get_module_registry: fn(*const ()) -> *const module_registry::IModuleRegistry,
        /// Fetches the settings registry.
        pub get_settings_registry: fn(*const ()) -> *const settings_registry::SettingsRegistry,
    }
}

/// Builds the [`ModuleInterfaceDescriptor`] for the interface.
pub fn build_interface_descriptor() -> ModuleInterfaceDescriptor {
    ModuleInterfaceDescriptor {
        name: unsafe { ArrayString::from_utf8_unchecked(IFimoCore::NAME.as_bytes()) },
        version: IFimoCore::VERSION,
        extensions: Default::default(),
    }
}
