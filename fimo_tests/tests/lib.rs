use fimo_ffi::Version;
use fimo_module::VersionQuery;
use fimo_tests::ContextBuilder;
use std::alloc::System;

#[global_allocator]
static A: System = System;

mod actix;
mod core;

const ACTIX_VERSION: VersionQuery = VersionQuery::Minimum(Version::new_short(0, 1, 0));
const CORE_VERSION: VersionQuery = VersionQuery::Minimum(Version::new_short(0, 1, 0));
#[allow(unused)]
const LOGGING_VERSION: VersionQuery = VersionQuery::Minimum(Version::new_short(0, 1, 0));
#[allow(unused)]
const TASKS_VERSION: VersionQuery = VersionQuery::Minimum(Version::new_short(0, 1, 0));

#[test]
fn construct_context() -> fimo_module::Result<()> {
    ContextBuilder::new()
        .with_actix()
        .with_core()
        .with_logging()
        .with_tasks()
        .build(|_| Ok(()))
}
