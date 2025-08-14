const std = @import("std");
const Mutex = std.Thread.Mutex;
const Allocator = std.mem.Allocator;
const AutoArrayHashMapUnmanaged = std.AutoArrayHashMapUnmanaged;

const fimo_tasks_meta = @import("fimo_tasks_meta");
const meta_pool = fimo_tasks_meta.pool;
const StackSize = meta_pool.StackSize;
const MetaId = meta_pool.Id;
const MetaWorker = meta_pool.Worker;
const MetaPool = meta_pool.Pool;
const MetaQuery = meta_pool.Query;

const Pool = @import("Pool.zig");

const Self = @This();

mutex: Mutex = .{},
pools: AutoArrayHashMapUnmanaged(MetaId, *Pool) = .empty,

pub fn deinit(self: *Self, allocator: Allocator) void {
    self.mutex.lock();
    defer self.mutex.unlock();

    while (self.pools.pop()) |entry| {
        entry.value.requestClose();
        entry.value.unrefWeakAndJoin();
    }
    self.pools.deinit(allocator);
}

pub fn spawnPool(self: *Self, allocator: Allocator, spawn_options: Pool.InitOptions) !*Pool {
    const pool = try Pool.init(spawn_options);
    errdefer {
        pool.asMetaPool().requestClose();
        pool.thread.join();
        pool.unref();
    }
    const pool_weak = pool.refWeakFromStrong();
    errdefer pool_weak.unrefWeak();

    self.mutex.lock();
    defer self.mutex.unlock();

    const id: MetaId = @enumFromInt(@intFromPtr(pool));
    try self.pools.put(allocator, id, pool_weak);

    return pool;
}

pub fn queryPoolById(self: *Self, id: MetaId) ?*Pool {
    self.mutex.lock();
    defer self.mutex.unlock();

    const pool = self.pools.get(id) orelse return null;
    if (!pool.refStrongFromWeakOrJoinAndUnref()) {
        _ = self.pools.swapRemove(id);
        return null;
    }

    return pool;
}

pub fn queryAllPools(self: *Self, allocator: Allocator) Allocator.Error![]MetaQuery.Node {
    self.mutex.lock();
    defer self.mutex.unlock();

    var nodes = try std.ArrayList(MetaQuery.Node).initCapacity(allocator, self.pools.count());
    errdefer {
        for (nodes.items) |node| {
            node.pool.unref();
        }
        nodes.deinit(allocator);
    }

    var it = self.pools.iterator();
    while (it.next()) |entry| {
        const pool = entry.value_ptr.*;
        if (!pool.is_public) continue;

        if (!pool.refStrongFromWeakOrJoinAndUnref()) {
            _ = self.pools.swapRemove(entry.key_ptr.*);
            it.len -= 1;
            it.index -= 1;
            continue;
        }

        nodes.appendAssumeCapacity(MetaQuery.Node{ .pool = pool.asMetaPool(), .next = null });
    }

    const items = try nodes.toOwnedSlice(allocator);
    for (items[0 .. items.len - 1], items[1..items.len]) |*node, *next| {
        node.next = next;
    }
    return items;
}
