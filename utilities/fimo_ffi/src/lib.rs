//! Library implementing FFI-safe equivalents of Rust's data types.
#![warn(
    missing_docs,
    rust_2018_idioms,
    missing_debug_implementations,
    rustdoc::broken_intra_doc_links
)]
#![feature(const_fn_trait_bound)]
#![feature(const_fn_fn_ptr_basics)]

pub mod error;

pub use fimo_object::obj_arc;
pub use fimo_object::obj_box;
pub use fimo_object::object;
pub use fimo_object::raw;
pub use fimo_object::span;
pub use fimo_object::str;
pub use fimo_object::vtable;
pub use fimo_object::{fimo_object, fimo_vtable};

pub use fimo_ffi_core::array_string;
pub use fimo_ffi_core::optional;
pub use fimo_ffi_core::result;

pub use crate::str::{ConstStr, MutStr, StrInner};
pub use array_string::ArrayString;
pub use error::Error;
pub use obj_arc::{ObjArc, ObjWeak};
pub use obj_box::ObjBox;
pub use object::Object;
pub use optional::Optional;
pub use result::Result;
pub use span::{ConstSpan, MutSpan, SpanInner};
