//! Symbols declared by the `fimo_tasks` bindings.

use fimo_std::declare_items;

declare_items! {
    mod fimo_tasks {
        extern context @ (0, 1, 0): crate::Context;
    }
}
