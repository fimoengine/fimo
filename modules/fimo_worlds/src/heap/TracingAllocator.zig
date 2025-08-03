const std = @import("std");
const Allocator = std.mem.Allocator;
const Alignment = std.mem.Alignment;
const AutoArrayHashMapUnmanaged = std.AutoArrayHashMapUnmanaged;

const fimo_tasks_meta = @import("fimo_tasks_meta");
const Mutex = fimo_tasks_meta.sync.Mutex;

const FimoWorlds = @import("../FimoWorlds.zig");
const Universe = @import("../Universe.zig");

const Self = @This();

mutex: Mutex = .{},
allocations: AutoArrayHashMapUnmanaged(usize, AllocInfo) = .empty,

const AllocInfo = struct {
    len: usize,
    alignment: Alignment,
};

pub fn init() Self {
    return .{};
}

pub fn deinit(self: *Self) void {
    const all = FimoWorlds.get().allocator;
    var it = self.allocations.iterator();
    while (it.next()) |info| {
        const memory = @as([*]u8, @ptrFromInt(info.key_ptr.*))[0..info.value_ptr.len];
        all.rawFree(memory, info.value_ptr.alignment, @returnAddress());
    }
    self.allocations.deinit(all);
    self.* = .{};
}

pub fn allocator(self: *Self) Allocator {
    return .{
        .ptr = self,
        .vtable = &.{
            .alloc = &alloc,
            .resize = &resize,
            .remap = &remap,
            .free = &free,
        },
    };
}

fn alloc(ctx: *anyopaque, len: usize, alignment: Alignment, ret_addr: usize) ?[*]u8 {
    const self: *Self = @ptrCast(@alignCast(ctx));
    self.mutex.lock();
    defer self.mutex.unlock();

    const all = FimoWorlds.get().allocator;
    self.allocations.ensureUnusedCapacity(all, 1) catch return null;

    const memory = all.rawAlloc(len, alignment, ret_addr) orelse return null;
    self.allocations.putAssumeCapacity(@intFromPtr(memory), .{ .len = len, .alignment = alignment });
    return memory;
}

fn resize(ctx: *anyopaque, memory: []u8, alignment: Alignment, new_len: usize, ret_addr: usize) bool {
    const self: *Self = @ptrCast(@alignCast(ctx));
    self.mutex.lock();
    defer self.mutex.unlock();

    const all = FimoWorlds.get().allocator;
    if (!all.rawResize(memory, alignment, new_len, ret_addr)) return false;

    const info = self.allocations.getPtr(@intFromPtr(memory.ptr)).?;
    info.len = new_len;
    return true;
}

fn remap(ctx: *anyopaque, memory: []u8, alignment: Alignment, new_len: usize, ret_addr: usize) ?[*]u8 {
    const self: *Self = @ptrCast(@alignCast(ctx));
    self.mutex.lock();
    defer self.mutex.unlock();

    const all = FimoWorlds.get().allocator;
    const new_memory = all.rawRemap(memory, alignment, new_len, ret_addr) orelse return null;

    _ = self.allocations.swapRemove(@intFromPtr(memory.ptr));
    self.allocations.putAssumeCapacity(
        @intFromPtr(new_memory),
        .{ .len = new_len, .alignment = alignment },
    );
    return new_memory;
}

fn free(ctx: *anyopaque, memory: []u8, alignment: Alignment, ret_addr: usize) void {
    const self: *Self = @ptrCast(@alignCast(ctx));
    self.mutex.lock();
    defer self.mutex.unlock();

    const all = FimoWorlds.get().allocator;
    all.rawFree(memory, alignment, ret_addr);

    _ = self.allocations.swapRemove(@intFromPtr(memory.ptr));
}
