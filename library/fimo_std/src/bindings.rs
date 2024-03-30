//! Raw bindings to the ffi library.
#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(clippy::undocumented_unsafe_blocks)]

include!(concat!(env!("OUT_DIR"), "/bindings.rs"));

/// Type-erased symbol definition.
#[repr(C)]
#[derive(Debug)]
pub struct FimoModuleRawSymbol {
    /// Pointer to the symbol.
    pub data: *const core::ffi::c_void,
    /// Lock count of the symbol.
    pub lock: core::sync::atomic::AtomicUsize,
}
