//! Raw bindings to the ffi bindings.
#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(clippy::type_complexity)]
#![allow(clippy::undocumented_unsafe_blocks)]

use fimo_std::bindings::*;

include!(concat!(env!("OUT_DIR"), "/bindings.rs"));
