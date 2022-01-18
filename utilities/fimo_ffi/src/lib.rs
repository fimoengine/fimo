//! Library implementing FFI-safe equivalents of Rust's data types.
#![warn(
    missing_docs,
    rust_2018_idioms,
    missing_debug_implementations,
    rustdoc::broken_intra_doc_links
)]
#![allow(incomplete_features)]
#![feature(const_fn_trait_bound)]
#![feature(const_fn_fn_ptr_basics)]
#![feature(try_reserve_kind)]
#![feature(dropck_eyepatch)]
#![feature(specialization)]
#![feature(allocator_api)]
#![feature(slice_range)]
#![feature(new_uninit)]
#![feature(trusted_len)]
#![feature(slice_ptr_len)]

pub mod error;
pub mod string;
pub mod vec;

pub use fimo_object::obj_arc;
pub use fimo_object::obj_box;
pub use fimo_object::object;
pub use fimo_object::raw;
pub use fimo_object::span;
pub use fimo_object::str;
pub use fimo_object::vtable;
pub use fimo_object::{fimo_marker, fimo_object, fimo_vtable, impl_vtable, is_object};

pub use fimo_ffi_core::array_string;
pub use fimo_ffi_core::fn_wrapper;
pub use fimo_ffi_core::optional;
pub use fimo_ffi_core::result;

pub use fimo_version_core as version;

pub use crate::str::{ConstStr, MutStr, StrInner};
pub use array_string::ArrayString;
pub use error::IError;
pub use fn_wrapper::{HeapFn, HeapFnMut, HeapFnOnce};
pub use obj_arc::{ObjArc, ObjWeak};
pub use obj_box::ObjBox;
pub use object::Object;
pub use optional::Optional;
pub use result::Result;
pub use span::{ConstSpan, MutSpan, SpanInner};
pub use string::String;
pub use vec::Vec;
pub use version::{ReleaseType, Version};
