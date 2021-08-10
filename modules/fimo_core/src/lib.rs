//! Implementation of the `fimo-core` interface.
#![feature(const_fn_trait_bound)]
#![feature(maybe_uninit_extra)]
#![feature(allocator_api)]
#![feature(c_unwind)]
#![warn(
    missing_docs,
    rust_2018_idioms,
    missing_debug_implementations,
    rustdoc::broken_intra_doc_links
)]
use fimo_version_core::{ReleaseType, Version};
use std::any::Any;

#[cfg(feature = "module")]
pub mod core_module;
pub mod module_registry;

#[cfg(feature = "module")]
pub use core_module::MODULE_NAME;

/// Name of the interface.
pub const INTERFACE_NAME: &str = "fimo-core";

/// Implemented interface version.
pub const INTERFACE_VERSION: Version = Version::new_long(0, 1, 0, ReleaseType::Unstable, 0);

/// Interface implementation.
#[derive(Debug)]
pub struct CoreInterface {
    module_registry: module_registry::ModuleRegistry,
}

impl CoreInterface {
    /// Initializes the `CoreInterface`.
    pub fn new() -> Self {
        Self {
            module_registry: module_registry::ModuleRegistry::new(),
        }
    }

    /// Extracts the interface version.
    pub fn get_interface_version(&self) -> Version {
        INTERFACE_VERSION
    }

    /// Extracts a reference to an extension from the interface.
    pub fn find_extension(&self, _extension: impl AsRef<str>) -> Option<&(dyn Any + 'static)> {
        None
    }

    /// Extracts a mutable reference to an extension from the interface.
    pub fn find_extension_mut(
        &mut self,
        _extension: impl AsRef<str>,
    ) -> Option<&mut (dyn Any + 'static)> {
        None
    }

    /// Extracts a reference to the module registry.
    pub fn as_module_registry(&self) -> &module_registry::ModuleRegistry {
        &self.module_registry
    }

    /// Extracts a mutable reference to the module registry.
    pub fn as_module_registry_mut(&mut self) -> &mut module_registry::ModuleRegistry {
        &mut self.module_registry
    }
}

impl AsRef<module_registry::ModuleRegistry> for CoreInterface {
    fn as_ref(&self) -> &module_registry::ModuleRegistry {
        self.as_module_registry()
    }
}

impl AsMut<module_registry::ModuleRegistry> for CoreInterface {
    fn as_mut(&mut self) -> &mut module_registry::ModuleRegistry {
        self.as_module_registry_mut()
    }
}

impl Default for CoreInterface {
    fn default() -> Self {
        CoreInterface::new()
    }
}
