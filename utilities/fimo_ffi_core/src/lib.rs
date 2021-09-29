//! Library implementing FFI-safe equivalents of Rust's data types.
#![feature(unboxed_closures)]
#![feature(fn_traits)]
#![feature(c_unwind)]
#![warn(
    missing_docs,
    rust_2018_idioms,
    missing_debug_implementations,
    rustdoc::broken_intra_doc_links
)]
pub mod arc;
pub mod array_string;
pub mod error;
pub mod error_info;
pub mod fn_wrapper;
pub mod non_null_const;
pub mod optional;
pub mod result;
pub mod span;
pub mod type_wrapper;

pub use arc::{Arc, Weak};
pub use array_string::ArrayString;
pub use error::Error;
pub use error_info::ErrorInfo;
pub use non_null_const::NonNullConst;
pub use optional::Optional;
pub use result::Result;
pub use span::{ConstSpan, MutSpan, Span};
pub use type_wrapper::TypeWrapper;
