//! Low-level representation of fimo-objects.
#![warn(
    missing_docs,
    rust_2018_idioms,
    missing_debug_implementations,
    rustdoc::broken_intra_doc_links
)]
#![allow(incomplete_features)]
#![feature(allocator_api)]
#![feature(layout_for_ptr)]
#![feature(dropck_eyepatch)]
#![feature(specialization)]
#![feature(unboxed_closures)]
#![feature(fn_traits)]
#![feature(unsize)]
#![feature(coerce_unsized)]

#[macro_use]
extern crate fimo_object_proc_macro;
extern crate self as fimo_object;

pub mod obj_box;
pub mod object;
pub mod raw;
pub mod span;
pub mod str;
pub mod vtable;

pub use crate::str::{ConstStr, MutStr};
pub use object::{CoerceObject, CoerceObjectMut, Object};
pub use span::{ConstSpan, MutSpan};