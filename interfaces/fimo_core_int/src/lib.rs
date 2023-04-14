//! Definition of the `fimo-core` interface.
#![warn(
    missing_docs,
    rust_2018_idioms,
    missing_debug_implementations,
    rustdoc::broken_intra_doc_links
)]
#![feature(const_mut_refs)]
#![feature(unsize)]

pub mod settings;

use crate::settings::ISettingsRegistry;
use fimo_module::context::IInterface;
use fimo_module::fimo_ffi::{interface, DynObj};
use fimo_module::Queryable;

impl Queryable for dyn IFimoCore + '_ {
    const NAME: &'static str = "fimo::interfaces::core";
    const CURRENT_VERSION: fimo_ffi::Version = fimo_ffi::Version::new_short(0, 1, 0);
    const EXTENSIONS: &'static [(Option<fimo_ffi::Version>, &'static str)] = &[];
}

interface! {
    #![interface_cfg(uuid = "eaa46386-ff47-405f-beaf-488e5eeaa004")]

    /// Type-erased `fimo-core` interface.
    pub frozen interface IFimoCore: IInterface @ version("0.0") {
        /// Returns the contained [`ISettingsRegistry`].
        fn settings(&self) -> &DynObj<dyn ISettingsRegistry + '_>;
    }
}
