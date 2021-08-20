//! Definition of the `fimo-core` interface.
#![feature(negative_impls)]
#![warn(
    missing_docs,
    rust_2018_idioms,
    missing_debug_implementations,
    rustdoc::broken_intra_doc_links
)]
use fimo_version_core::{ReleaseType, Version};

pub mod rust;

/// Name of the interface.
pub const INTERFACE_NAME: &str = "fimo-tasks";

/// Implemented interface version.
pub const INTERFACE_VERSION: Version = Version::new_long(0, 1, 0, ReleaseType::Unstable, 0);
