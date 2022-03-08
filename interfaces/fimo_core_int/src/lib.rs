//! Definition of the `fimo-core` interface.
#![warn(
    missing_docs,
    rust_2018_idioms,
    missing_debug_implementations,
    rustdoc::broken_intra_doc_links
)]
#![feature(const_ptr_offset_from)]
#![feature(const_mut_refs)]
#![feature(unsize)]

pub mod modules;
pub mod settings;

use crate::modules::IModuleRegistry;
use crate::settings::ISettingsRegistry;
use fimo_module::fimo_ffi::{interface, DynObj};
use fimo_module::{FimoInterface, IModuleInterface, IModuleInterfaceVTable, ReleaseType, Version};

/// Type-erased `fimo-core` interface.
#[interface(
    uuid = "c2173cd4-767c-4ac2-a8ac-52a2cbebda0a",
    vtable = "IFimoCoreVTable",
    generate(IModuleInterfaceVTable)
)]
pub trait IFimoCore: IModuleInterface {
    /// Returns the contained [`IModuleRegistry`].
    #[vtable_info(
        return_type = "*const DynObj<dyn IModuleRegistry>",
        into_expr = "unsafe {
            std::mem::transmute::<*const DynObj<dyn IModuleRegistry + '_>, *const DynObj<dyn IModuleRegistry>>(res)
        }",
        from_expr = "unsafe { &*res }"
    )]
    fn modules(&self) -> &DynObj<dyn IModuleRegistry + '_>;

    /// Returns the contained [`ISettingsRegistry`].
    #[vtable_info(
        return_type = "*const DynObj<dyn ISettingsRegistry>",
        into_expr = "unsafe {
            std::mem::transmute::<*const DynObj<dyn ISettingsRegistry + '_>, *const DynObj<dyn ISettingsRegistry>>(res)
        }",
        from_expr = "unsafe { &*res }"
    )]
    fn settings(&self) -> &DynObj<dyn ISettingsRegistry + '_>;
}

impl FimoInterface for dyn IFimoCore {
    const NAME: &'static str = "fimo::interfaces::core::fimo_core";
    const VERSION: Version = Version::new_long(0, 1, 0, ReleaseType::Unstable, 0);
    const EXTENSIONS: &'static [&'static str] = &[];
}
