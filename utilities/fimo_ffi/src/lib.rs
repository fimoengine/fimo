//! Library implementing FFI-safe equivalents of Rust's data types.
#![warn(
    missing_docs,
    rust_2018_idioms,
    missing_debug_implementations,
    rustdoc::broken_intra_doc_links
)]
#![allow(
    incomplete_features,
    // https://github.com/rust-lang/rust-clippy/issues/8867
    clippy::derive_partial_eq_without_eq,
)]
#![feature(const_slice_from_raw_parts_mut)]
#![feature(const_precise_live_drops)]
#![feature(maybe_uninit_as_bytes)]
#![feature(try_trait_v2_residual)]
#![feature(const_slice_ptr_len)]
#![feature(alloc_layout_extra)]
#![feature(const_refs_to_cell)]
#![feature(strict_provenance)]
#![feature(must_not_suspend)]
#![feature(try_reserve_kind)]
#![feature(unboxed_closures)]
#![feature(const_ptr_write)]
#![feature(const_type_name)]
#![feature(core_intrinsics)]
#![feature(dropck_eyepatch)]
#![feature(specialization)]
#![feature(const_mut_refs)]
#![feature(layout_for_ptr)]
#![feature(negative_impls)]
#![feature(allocator_api)]
#![feature(const_type_id)]
#![feature(slice_ptr_len)]
#![feature(cfg_sanitize)]
#![feature(ptr_metadata)]
#![feature(try_trait_v2)]
#![feature(slice_range)]
#![feature(trusted_len)]
#![feature(tuple_trait)]
#![feature(new_uninit)]
#![feature(const_heap)]
#![feature(const_box)]
#![feature(fn_traits)]
#![feature(lazy_cell)]
#![feature(c_unwind)]
#![feature(unsize)]

extern crate self as fimo_ffi;

pub mod cell;
pub mod error;
pub mod ffi_fn;
pub mod fmt;
pub mod marshal;
pub mod obj_arc;
pub mod obj_box;
pub mod optional;
pub mod path;
pub mod provider;
pub mod ptr;
pub mod result;
pub mod span;
pub mod str;
pub mod string;
pub mod tuple;
pub mod type_id;
pub mod vec;
pub mod version;

pub use crate::str::{ConstStr, MutStr};
pub use ffi_fn::FfiFn;
pub use obj_arc::{ObjArc, ObjWeak};
pub use obj_box::ObjBox;
pub use optional::Optional;
pub use ptr::{interface, DynObj, Object};
pub use result::Result;
pub use span::{ConstSpan, MutSpan};
pub use string::String;
pub use tuple::{ReprC, ReprRust};
pub use vec::Vec;
pub use version::{ReleaseType, Version};

/// Version of the fimo standard library.
pub const FIMO_VERSION: Version = Version::new_short(0, 1, 0);

/// Version of the fimo standard library used when checking for compatability.
pub const FIMO_VERSION_COMPAT: Version = Version::new_short(0, 1, 0);
