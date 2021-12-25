//! Low-level representation of fimo-objects.
#![warn(
    missing_docs,
    rust_2018_idioms,
    missing_debug_implementations,
    rustdoc::broken_intra_doc_links
)]

pub mod object;
pub mod raw;
pub mod span;
pub mod str;
pub mod vtable;

pub use crate::str::{ConstStr, MutStr};
pub use object::{CoerceObject, CoerceObjectMut, Object};
pub use span::{ConstSpan, MutSpan};
