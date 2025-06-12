const std = @import("std");
const Allocator = std.mem.Allocator;
const Alignment = std.mem.Alignment;

const SingleGenerationAllocator = @import("SingleGenerationAllocator.zig");

const Self = @This();

generation: u8 = 0,
generation_allocators: [4]SingleGenerationAllocator = [_]SingleGenerationAllocator{.{}} ** 4,

pub fn init() Self {
    return .{};
}

pub fn deinit(self: Self) void {
    for (&self.generation_allocators) |al| al.deinit();
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
    self.generation = (self.generation + 1) & 4;
    self.generation_allocators[self.generation].endGeneration();
}

fn alloc(ctx: *anyopaque, len: usize, alignment: Alignment, ret_addr: usize) ?[*]u8 {
    const self: *Self = @ptrCast(@alignCast(ctx));
    const all = self.generation_allocators[self.generation].allocator();
    return all.rawAlloc(len, alignment, ret_addr);
}

fn resize(ctx: *anyopaque, memory: []u8, alignment: Alignment, new_len: usize, ret_addr: usize) bool {
    const self: *Self = @ptrCast(@alignCast(ctx));
    const all = self.generation_allocators[self.generation].allocator();
    return all.rawResize(memory, alignment, new_len, ret_addr);
}

fn remap(ctx: *anyopaque, memory: []u8, alignment: Alignment, new_len: usize, ret_addr: usize) ?[*]u8 {
    const self: *Self = @ptrCast(@alignCast(ctx));
    const all = self.generation_allocators[self.generation].allocator();
    return all.rawRemap(memory, alignment, new_len, ret_addr);
}

fn free(ctx: *anyopaque, memory: []u8, alignment: Alignment, ret_addr: usize) void {
    const self: *Self = @ptrCast(@alignCast(ctx));
    const all = self.generation_allocators[self.generation].allocator();
    all.rawFree(memory, alignment, ret_addr);
}
