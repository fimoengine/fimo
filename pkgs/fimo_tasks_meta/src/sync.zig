//! Implementation of synchronization primitives.

pub const Condition = @import("sync/Condition.zig");
pub const Futex = @import("sync/Futex.zig");
pub const Mutex = @import("sync/Mutex.zig");
pub const RwLock = @import("sync/RwLock.zig");
