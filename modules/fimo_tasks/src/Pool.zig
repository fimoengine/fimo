const std = @import("std");
const atomic = std.atomic;
const Allocator = std.mem.Allocator;

const fimo_tasks_meta = @import("fimo_tasks_meta");
const meta_pool = fimo_tasks_meta.pool;
const MetaPool = meta_pool.Pool;

const mpsc_channel = @import("mpsc_channel.zig");
const root = @import("root.zig");
const Instance = root.Instance;
const spmc_channel = @import("spmc_channel.zig");
const Task = @import("Task.zig");
const Worker = @import("Worker.zig");

allocator: Allocator,
instance: *const Instance,
// message_queue: mpsc_channel.Fifo(Message) = .empty,
task_queue: spmc_channel.Channel(*Task),
ref_count: atomic.Value(usize) = .init(1),
weak_ref_count: atomic.Value(usize) = .init(1),

const Self = @This();

pub const Message = union(enum) {
    worker: WorkerMessage,
    outer: OuterMessage,
};

pub const WorkerMessage = union(enum) {};

pub const OuterMessage = union(enum) {};

pub fn ref(self: *Self) void {
    const old = self.ref_count.fetchAdd(1, .monotonic);
    std.debug.assert(old != 0 and old != std.math.maxInt(usize));
}

pub fn unref(self: *Self) void {
    const old = self.ref_count.fetchSub(1, .release);
    std.debug.assert(old != 0);
    if (old > 1) return;

    _ = self.ref_count.load(.acquire);

    // TODO: Shutdown the pool.

    self.unrefWeak();
}

pub fn refWeak(self: *Self) void {
    const old = self.weak_ref_count.fetchAdd(1, .monotonic);
    std.debug.assert(old != 0 and old != std.math.maxInt(usize));
}

pub fn unrefWeak(self: *Self) void {
    const old = self.weak_ref_count.fetchSub(1, .release);
    std.debug.assert(old != 0);
    if (old > 1) return;

    _ = self.weak_ref_count.load(.acquire);

    @panic("TODO");
}

pub fn downgradeStrong(self: *Self) void {
    const old = self.weak_ref_count.fetchAdd(1, .acquire);
    std.debug.assert(old != 0 and old != std.math.maxInt(usize));
}

pub fn upgradeWeak(self: *Self) bool {
    var old = self.ref_count.load(.acquire);
    while (old != 0) {
        if (self.ref_count.cmpxchgWeak(old, old + 1, .acquire, .monotonic)) |new| {
            old = new;
            continue;
        }
        return true;
    }
    return false;
}

pub fn asMetaPool(self: *Self) MetaPool {
    _ = self;
    @panic("not implemented");
}
