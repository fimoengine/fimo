//! Implementation of the `fimo-core` interface.
#![feature(const_fn_trait_bound)]
#![feature(maybe_uninit_extra)]
#![feature(allocator_api)]
#![feature(c_unwind)]
#![feature(step_trait)]
#![warn(
    missing_docs,
    rust_2018_idioms,
    missing_debug_implementations,
    rustdoc::broken_intra_doc_links
)]
use fimo_core_interface::rust::{FimoCore, FimoCoreVTable, INTERFACE_VERSION};
use std::any::Any;
use std::ops::Deref;

#[cfg(feature = "module")]
pub mod core_module;
pub mod module_registry;
pub mod settings_registry;

#[cfg(feature = "module")]
pub use core_module::MODULE_NAME;

const VTABLE: FimoCoreVTable = FimoCoreVTable::new(
    |ptr, extension| {
        let interface = unsafe { &*(ptr as *const CoreInterface) };
        let extension = unsafe { &*extension };
        CoreInterface::find_extension(interface, extension).map(|ext| ext as *const _)
    },
    |ptr| {
        let interface = unsafe { &*(ptr as *const CoreInterface) };
        &**CoreInterface::as_module_registry(interface)
    },
    |ptr| {
        let interface = unsafe { &*(ptr as *const CoreInterface) };
        &**CoreInterface::as_settings_registry(interface)
    },
);

/// Interface implementation.
#[derive(Debug)]
pub struct CoreInterface {
    module_registry: module_registry::ModuleRegistry,
    settings_registry: settings_registry::SettingsRegistry,
}

impl CoreInterface {
    /// Initializes the `CoreInterface`.
    pub fn new() -> Self {
        Self {
            module_registry: module_registry::ModuleRegistry::new(),
            settings_registry: settings_registry::SettingsRegistry::new(),
        }
    }

    /// Extracts a reference to an extension from the interface.
    pub fn find_extension(&self, _extension: impl AsRef<str>) -> Option<&(dyn Any + 'static)> {
        None
    }

    /// Extracts a reference to the module registry.
    pub fn as_module_registry(&self) -> &module_registry::ModuleRegistry {
        &self.module_registry
    }

    /// Extracts a reference to the settings registry.
    pub fn as_settings_registry(&self) -> &settings_registry::SettingsRegistry {
        &self.settings_registry
    }
}

impl AsRef<module_registry::ModuleRegistry> for CoreInterface {
    fn as_ref(&self) -> &module_registry::ModuleRegistry {
        self.as_module_registry()
    }
}

impl AsRef<settings_registry::SettingsRegistry> for CoreInterface {
    fn as_ref(&self) -> &settings_registry::SettingsRegistry {
        self.as_settings_registry()
    }
}

impl Default for CoreInterface {
    fn default() -> Self {
        CoreInterface::new()
    }
}

impl Deref for CoreInterface {
    type Target = FimoCore;

    fn deref(&self) -> &Self::Target {
        let self_ptr = self as *const _ as *const ();
        let vtable = &VTABLE;

        unsafe { &*FimoCore::from_raw_parts(self_ptr, vtable) }
    }
}
