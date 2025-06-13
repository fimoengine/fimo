//! Standard library used by the Fimo engine.
//!
//! ### Unstable features
//!
//! Some parts of the API are only available when specifying the `fimo_internals` flag
//!
//! This flag enables **unstable** features. The public API of these features may break with any
//! semver compatible release. To enable these features, the `--cfg fimo_internals` argument must be
//! passed to `rustc` when compiling. This serves to explicitly opt in to features which may break
//! semver conventions.
//!
//! You can specify it in your project's `.cargo/config.toml` file:
//!
//! ```toml
//! [build]
//! rustflags = ["--cfg", "fimo_internals"]
//! ```
//!
//! <div class="warning">
//! The <code>[build]</code> section does <strong>not</strong> go in a
//! <code>Cargo.toml</code> file. Instead, it must be placed in the Cargo config
//! file <code>.cargo/config.toml</code>.
//! </div>
//!
//! Alternatively, you can specify it with an environment variable:
//!
//! ```sh
//! ## Many *nix shells:
//! export RUSTFLAGS="--cfg fimo_internals"
//! cargo build
//! ```
//!
//! ```powershell
//! ## Windows PowerShell:
//! $Env:RUSTFLAGS="--cfg fimo_internals"
//! cargo build
//! ```
#![feature(unsize)]
#![feature(doc_cfg)]
#![feature(extend_one)]
#![feature(auto_traits)]
#![feature(thread_local)]
#![feature(allocator_api)]
#![feature(coerce_unsized)]
#![feature(negative_impls)]
#![feature(trivial_bounds)]
#![feature(const_trait_impl)]
#![feature(panic_update_hook)]
#![feature(maybe_uninit_slice)]
#![feature(vec_into_raw_parts)]
#![feature(min_specialization)]
#![feature(anonymous_lifetime_in_impl_trait)]
#![allow(clashing_extern_declarations)]

#[doc(hidden)]
pub use paste;
#[doc(hidden)]
pub use static_assertions as __private_sa;

extern crate static_assertions as sa;

#[macro_use]
mod macros;

pub mod bindings;

pub mod context;
pub mod error;
pub mod panic;
pub mod time;
pub mod utils;
pub mod version;

pub mod r#async;
pub mod module;
pub mod tracing;
