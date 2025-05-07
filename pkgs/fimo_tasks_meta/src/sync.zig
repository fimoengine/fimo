//! Implementation of synchronization primitives.

pub const Condition = @import("sync/Condition.zig");
pub const Futex = @import("sync/futex.zig").Futex;
pub const Mutex = @import("sync/Mutex.zig");
pub const ParkingLot = @import("sync/ParkingLot.zig");
pub const RwLock = @import("sync/RwLock.zig");
