//! Implementation of the `fimo-core` interface.
#![warn(
    missing_docs,
    rust_2018_idioms,
    missing_debug_implementations,
    rustdoc::broken_intra_doc_links
)]
#![feature(step_trait)]
#![feature(c_unwind)]

pub mod settings;

#[cfg(feature = "module")]
pub mod module;

/// Interface implementation.
#[derive(Debug)]
pub struct Core {
    settings_registry: settings::SettingsRegistry,
}

impl Core {
    /// Initializes the `CoreInterface`.
    pub fn new() -> Self {
        Self {
            settings_registry: settings::SettingsRegistry::new(),
        }
    }

    /// Extracts a reference to the settings registry.
    pub fn as_settings_registry(&self) -> &settings::SettingsRegistry {
        &self.settings_registry
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
