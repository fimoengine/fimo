//! Definition of the `fimo-core` interface.
#![warn(
    missing_docs,
    rust_2018_idioms,
    missing_debug_implementations,
    rustdoc::broken_intra_doc_links
)]
#![feature(const_ptr_offset_from)]
#![feature(const_trait_impl)]
#![feature(const_mut_refs)]
#![feature(unsize)]

pub mod modules;
pub mod settings;

use crate::modules::IModuleRegistry;
use crate::settings::ISettingsRegistry;
use fimo_module::fimo_ffi::{interface, DynObj};
use fimo_module::{FimoInterface, IModuleInterface, ReleaseType, Version};

interface! {
    #![interface_cfg(uuid = "c2173cd4-767c-4ac2-a8ac-52a2cbebda0a")]

    /// Type-erased `fimo-core` interface.
    pub frozen interface IFimoCore: IModuleInterface @ frozen version("0.0") {
        /// Returns the contained [`IModuleRegistry`].
        fn modules(&self) -> &DynObj<dyn IModuleRegistry + '_>;

        /// Returns the contained [`ISettingsRegistry`].
        fn settings(&self) -> &DynObj<dyn ISettingsRegistry + '_>;
    }
}

impl FimoInterface for dyn IFimoCore {
    const NAME: &'static str = "fimo::interfaces::core::fimo_core";
    const VERSION: Version = Version::new_long(0, 1, 0, ReleaseType::Unstable, 0);
    const EXTENSIONS: &'static [&'static str] = &[];
}
