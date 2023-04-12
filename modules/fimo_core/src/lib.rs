//! Implementation of the `fimo-core` interface.
#![warn(
    missing_docs,
    rust_2018_idioms,
    missing_debug_implementations,
    rustdoc::broken_intra_doc_links
)]
#![feature(step_trait)]
#![feature(c_unwind)]

pub mod modules;
pub mod settings;

#[cfg(feature = "module")]
pub mod module;

#[cfg(feature = "module")]
pub use module::MODULE_NAME;

/// Interface implementation.
#[derive(Debug)]
pub struct Core {
    module_registry: modules::ModuleRegistry,
    settings_registry: settings::SettingsRegistry,
}

impl Core {
    /// Initializes the `CoreInterface`.
    pub fn new() -> Self {
        Self {
            module_registry: modules::ModuleRegistry::new(),
            settings_registry: settings::SettingsRegistry::new(),
        }
    }

    /// Extracts a reference to the module registry.
    pub fn as_module_registry(&self) -> &modules::ModuleRegistry {
        &self.module_registry
    }

    /// Extracts a reference to the settings registry.
    pub fn as_settings_registry(&self) -> &settings::SettingsRegistry {
        &self.settings_registry
    }
}

impl AsRef<modules::ModuleRegistry> for Core {
    fn as_ref(&self) -> &modules::ModuleRegistry {
        self.as_module_registry()
    }
}

impl AsRef<settings::SettingsRegistry> for Core {
    fn as_ref(&self) -> &settings::SettingsRegistry {
        self.as_settings_registry()
    }
}

impl Default for Core {
    fn default() -> Self {
        Core::new()
    }
}
