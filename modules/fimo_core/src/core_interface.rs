//! Implementation of the `fimo-core` interface.
use fimo_version_core::{ReleaseType, Version};

pub mod module_registry;

/// Implemented interface version.
pub const INTERFACE_VERSION: Version = Version::new_long(0, 1, 0, ReleaseType::Unstable, 0);

/// Interface implementation.
#[derive(Debug)]
pub struct CoreInterface {
    module_registry: module_registry::ModuleRegistry,
}
