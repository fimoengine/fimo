//! Library implementing FFI-safe equivalents of Rust's data types.
#![warn(
    missing_docs,
    rust_2018_idioms,
    missing_debug_implementations,
    rustdoc::broken_intra_doc_links
)]
#![allow(incomplete_features)]
#![feature(const_fn_fn_ptr_basics)]
#![feature(const_ptr_offset_from)]
#![feature(const_fn_trait_bound)]
#![feature(alloc_layout_extra)]
#![feature(try_reserve_kind)]
#![feature(unboxed_closures)]
#![feature(const_type_name)]
#![feature(dropck_eyepatch)]
#![feature(specialization)]
#![feature(layout_for_ptr)]
#![feature(allocator_api)]
#![feature(set_ptr_value)]
#![feature(slice_ptr_len)]
#![feature(cfg_sanitize)]
#![feature(ptr_metadata)]
#![feature(slice_range)]
#![feature(trusted_len)]
#![feature(new_uninit)]
#![feature(fn_traits)]
#![feature(c_unwind)]
#![feature(unsize)]

pub mod error;
pub mod ffi_fn;
pub mod fmt;
pub mod obj_arc;
pub mod obj_box;
pub mod optional;
pub mod ptr;
pub mod result;
pub mod span;
pub mod str;
pub mod string;
pub mod tuple;
pub mod vec;

pub use fimo_version_core as version;

pub use crate::str::{ConstStr, MutStr, StrInner};
pub use ffi_fn::FfiFn;
pub use obj_arc::{ObjArc, ObjWeak};
pub use obj_box::ObjBox;
pub use optional::Optional;
pub use ptr::DynObj;
pub use result::Result;
pub use span::{ConstSpan, MutSpan, SpanInner};
pub use string::String;
pub use vec::Vec;
pub use version::{ReleaseType, Version};
