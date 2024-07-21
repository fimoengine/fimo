#![feature(arbitrary_self_types)]
#![feature(exposed_provenance)]
#![feature(result_flattening)]
#![feature(strict_provenance)]
#![feature(thread_local)]

use fimo_std::allocator::FimoAllocator;

// We are currently building each module in separate dynamic library.
// If we decide to support static linking in the future this should be
// hidden behind a `#[cfg(...)]`.
#[global_allocator]
static GLOBAL: FimoAllocator = FimoAllocator;

mod context;
mod module_export;
mod worker_group;
