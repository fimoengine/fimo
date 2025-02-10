//! Symbols declared by the `fimo_tasks` bindings.
use fimo_std::{module::symbols::AssertSharable, symbol};

symbol! {
    symbol Context @ Version("0.1.0") = "fimo_tasks"::context: *const AssertSharable<crate::Context>;
}
