//! Implementation of the `fimo-core` interface.
use fimo_version_core::{ReleaseType, Version};
use std::any::Any;

pub mod module_registry;

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
