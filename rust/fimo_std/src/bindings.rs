//! Raw bindings to the ffi library.
#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(clippy::type_complexity)]
#![allow(clippy::manual_c_str_literals)]
#![allow(clippy::undocumented_unsafe_blocks)]

include!(concat!(env!("OUT_DIR"), "/bindings.rs"));
