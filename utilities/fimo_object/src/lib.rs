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
#![feature(const_fn_trait_bound)]
#![feature(const_fn_fn_ptr_basics)]
#![feature(alloc_layout_extra)]
#![feature(const_type_name)]
#![feature(set_ptr_value)]
#![feature(cfg_sanitize)]
#![feature(ptr_metadata)]

pub mod obj_arc;
pub mod obj_box;
pub mod object;
pub mod raw;
pub mod span;
pub mod str;
pub mod vtable;

pub use crate::str::{ConstStr, MutStr};
pub use obj_arc::{ObjArc, ObjWeak};
pub use obj_box::ObjBox;
pub use object::{CoerceObject, CoerceObjectMut, Object};
pub use span::{ConstSpan, MutSpan};
