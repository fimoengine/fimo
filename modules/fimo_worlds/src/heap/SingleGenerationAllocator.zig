const std = @import("std");
const Allocator = std.mem.Allocator;
const Alignment = std.mem.Alignment;
const ArenaAllocator = std.heap.ArenaAllocator;

const fimo_tasks_meta = @import("fimo_tasks_meta");
const Mutex = fimo_tasks_meta.sync.Mutex;

const FimoWorlds = @import("../FimoWorlds.zig");
const Universe = @import("../Universe.zig");

const Self = @This();

mutex: Mutex = .{},
arena: ArenaAllocator = .init(std.heap.page_allocator),

pub fn init() Self {
    return .{};
}

pub fn deinit(self: Self) void {
    self.arena.deinit();
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

pub fn endGeneration(self: *Self) void {
    _ = self.arena.reset(.retain_capacity);
}

fn alloc(ctx: *anyopaque, len: usize, alignment: Alignment, ret_addr: usize) ?[*]u8 {
    const self: *Self = @ptrCast(@alignCast(ctx));
    self.mutex.lock();
    defer self.mutex.unlock();
    return self.arena.allocator().rawAlloc(len, alignment, ret_addr);
}

fn resize(ctx: *anyopaque, memory: []u8, alignment: Alignment, new_len: usize, ret_addr: usize) bool {
    const self: *Self = @ptrCast(@alignCast(ctx));
    self.mutex.lock();
    defer self.mutex.unlock();
    return self.arena.allocator().rawResize(memory, alignment, new_len, ret_addr);
}

fn remap(ctx: *anyopaque, memory: []u8, alignment: Alignment, new_len: usize, ret_addr: usize) ?[*]u8 {
    const self: *Self = @ptrCast(@alignCast(ctx));
    self.mutex.lock();
    defer self.mutex.unlock();
    return self.arena.allocator().rawRemap(memory, alignment, new_len, ret_addr);
}

fn free(ctx: *anyopaque, memory: []u8, alignment: Alignment, ret_addr: usize) void {
    const self: *Self = @ptrCast(@alignCast(ctx));
    self.mutex.lock();
    defer self.mutex.unlock();
    self.arena.allocator().rawFree(memory, alignment, ret_addr);
}
