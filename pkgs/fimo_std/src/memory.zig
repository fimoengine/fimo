const std = @import("std");
const atomic = std.atomic;
const Thread = std.Thread;
const math = std.math;
const mem = std.mem;
const heap = std.heap;
const testing = std.testing;
const Alignment = mem.Alignment;
const StdAllocator = mem.Allocator;
pub const Error = StdAllocator.Error;
const posix = std.posix;
const builtin = @import("builtin");

const win32 = @import("win32");

const utils = @import("utils.zig");

const Memory = utils.Slice(u8);

pub const Allocator = extern struct {
    const VTable = extern struct {
        /// Allocates a new buffer.
        alloc: *const fn (ptr: ?*anyopaque, len: usize, alignment: usize) callconv(.c) ?[*]u8,
        /// Tries to resize the buffer in place.
        resize: *const fn (ptr: ?*anyopaque, memory: Memory, alignment: usize, new_len: usize) callconv(.c) bool,
        /// Resizes the buffer, allowing relocation.
        remap: *const fn (ptr: ?*anyopaque, memory: Memory, alignment: usize, new_len: usize) callconv(.c) ?[*]u8,
        /// Frees a previously allocated buffer.
        free: *const fn (ptr: ?*anyopaque, memory: Memory, alignment: usize) callconv(.c) void,
    };

    ptr: ?*anyopaque,
    vtable: *const VTable,

    const std_to_fstd_vtable: VTable = .{
        .alloc = &std_to_fstd_alloc,
        .resize = &std_to_fstd_resize,
        .remap = &std_to_fstd_remap,
        .free = &std_to_fstd_free,
    };

    fn std_to_fstd_alloc(ptr: ?*anyopaque, len: usize, alignment: usize) callconv(.c) ?[*]u8 {
        const allocator: *const StdAllocator = @ptrCast(@alignCast(ptr));
        return allocator.rawAlloc(len, .fromByteUnits(alignment), @returnAddress());
    }
    fn std_to_fstd_resize(ptr: ?*anyopaque, memory: Memory, alignment: usize, new_len: usize) callconv(.c) bool {
        const allocator: *const StdAllocator = @ptrCast(@alignCast(ptr));
        return allocator.rawResize(memory.intoSliceOrEmpty(), .fromByteUnits(alignment), new_len, @returnAddress());
    }
    fn std_to_fstd_remap(ptr: ?*anyopaque, memory: Memory, alignment: usize, new_len: usize) callconv(.c) ?[*]u8 {
        const allocator: *const StdAllocator = @ptrCast(@alignCast(ptr));
        return allocator.rawRemap(memory.intoSliceOrEmpty(), .fromByteUnits(alignment), new_len, @returnAddress());
    }
    fn std_to_fstd_free(ptr: ?*anyopaque, memory: Memory, alignment: usize) callconv(.c) void {
        const allocator: *const StdAllocator = @ptrCast(@alignCast(ptr));
        return allocator.rawFree(memory.intoSliceOrEmpty(), .fromByteUnits(alignment), @returnAddress());
    }

    pub fn adaptFromStdAllocator(allocator: *const StdAllocator) Allocator {
        return .{ .ptr = @constCast(allocator), .vtable = &std_to_fstd_vtable };
    }

    const fstd_to_std_vtable: StdAllocator.VTable = .{
        .alloc = &fstd_to_std_alloc,
        .resize = &fstd_to_std_resize,
        .remap = &fstd_to_std_remap,
        .free = &fstd_to_std_free,
    };

    fn fstd_to_std_alloc(ptr: *anyopaque, len: usize, alignment: Alignment, ret_addr: usize) ?[*]u8 {
        _ = ret_addr;
        const allocator: *const Allocator = @ptrCast(@alignCast(ptr));
        return allocator.rawAlloc(len, alignment);
    }
    fn fstd_to_std_resize(ptr: *anyopaque, memory: []u8, alignment: Alignment, new_len: usize, ret_addr: usize) bool {
        _ = ret_addr;
        const allocator: *const Allocator = @ptrCast(@alignCast(ptr));
        return allocator.rawResize(memory, alignment, new_len);
    }
    fn fstd_to_std_remap(ptr: *anyopaque, memory: []u8, alignment: Alignment, new_len: usize, ret_addr: usize) ?[*]u8 {
        _ = ret_addr;
        const allocator: *const Allocator = @ptrCast(@alignCast(ptr));
        return allocator.rawRemap(memory, alignment, new_len);
    }
    fn fstd_to_std_free(ptr: *anyopaque, memory: []u8, alignment: Alignment, ret_addr: usize) void {
        _ = ret_addr;
        const allocator: *const Allocator = @ptrCast(@alignCast(ptr));
        return allocator.rawFree(memory, alignment);
    }

    pub fn adaptIntoStdAllocator(allocator: *const Allocator) StdAllocator {
        return .{ .ptr = @constCast(allocator), .vtable = &fstd_to_std_vtable };
    }

    // The following is adapted from the zig standard library.

    pub inline fn rawAlloc(self: Allocator, len: usize, alignment: Alignment) ?[*]u8 {
        return self.vtable.alloc(self.ptr, len, alignment.toByteUnits());
    }

    pub inline fn rawResize(self: Allocator, memory: []u8, alignment: Alignment, new_len: usize) bool {
        return self.vtable.resize(self.ptr, .fromSlice(memory), alignment.toByteUnits(), new_len);
    }

    pub inline fn rawRemap(self: Allocator, memory: []u8, alignment: Alignment, new_len: usize) ?[*]u8 {
        return self.vtable.remap(self.ptr, .fromSlice(memory), alignment.toByteUnits(), new_len);
    }

    pub inline fn rawFree(self: Allocator, memory: []u8, alignment: Alignment) void {
        self.vtable.free(self.ptr, .fromSlice(memory), alignment.toByteUnits());
    }

    pub fn create(self: Allocator, comptime T: type) Error!*T {
        if (@sizeOf(T) == 0) {
            const ptr = comptime mem.alignBackward(usize, math.maxInt(usize), @alignOf(T));
            return @ptrFromInt(ptr);
        }
        const ptr: *T = @ptrCast(try self.allocBytesWithAlignment(.of(T), @sizeOf(T)));
        return ptr;
    }

    pub fn destroy(self: Allocator, ptr: anytype) void {
        const info = @typeInfo(@TypeOf(ptr)).pointer;
        if (info.size != .one) @compileError("ptr must be a single item pointer");
        const T = info.child;
        if (@sizeOf(T) == 0) return;
        const non_const_ptr = @as([*]u8, @ptrCast(@constCast(ptr)));
        self.rawFree(non_const_ptr[0..@sizeOf(T)], .fromByteUnits(info.alignment));
    }

    pub fn alloc(self: Allocator, comptime T: type, n: usize) Error![]T {
        return self.allocAdvanced(T, null, n);
    }

    pub fn allocWithOptions(
        self: Allocator,
        comptime Elem: type,
        n: usize,
        /// null means naturally aligned
        comptime optional_alignment: ?Alignment,
        comptime optional_sentinel: ?Elem,
    ) Error!AllocWithOptionsPayload(Elem, optional_alignment, optional_sentinel) {
        if (optional_sentinel) |sentinel| {
            const ptr = try self.allocAdvanced(Elem, optional_alignment, n + 1);
            ptr[n] = sentinel;
            return ptr[0..n :sentinel];
        } else {
            return self.allocAdvanced(Elem, optional_alignment, n);
        }
    }

    fn AllocWithOptionsPayload(comptime Elem: type, comptime alignment: ?Alignment, comptime sentinel: ?Elem) type {
        if (sentinel) |s| {
            return [:s]align(if (alignment) |a| a.toByteUnits() else @alignOf(Elem)) Elem;
        } else {
            return []align(if (alignment) |a| a.toByteUnits() else @alignOf(Elem)) Elem;
        }
    }

    pub fn allocSentinel(
        self: Allocator,
        comptime Elem: type,
        n: usize,
        comptime sentinel: Elem,
    ) Error![:sentinel]Elem {
        return self.allocWithOptions(Elem, n, null, sentinel);
    }

    pub fn alignedAlloc(
        self: Allocator,
        comptime T: type,
        /// null means naturally aligned
        comptime alignment: ?Alignment,
        n: usize,
    ) Error![]align(if (alignment) |a| a.toByteUnits() else @alignOf(T)) T {
        return self.allocAdvanced(T, alignment, n);
    }

    pub inline fn allocAdvanced(
        self: Allocator,
        comptime T: type,
        /// null means naturally aligned
        comptime alignment: ?Alignment,
        n: usize,
    ) Error![]align(if (alignment) |a| a.toByteUnits() else @alignOf(T)) T {
        const a = comptime (alignment orelse Alignment.of(T));
        const ptr: [*]align(a.toByteUnits()) T = @ptrCast(try self.allocWithSizeAndAlignment(@sizeOf(T), a, n));
        return ptr[0..n];
    }

    fn allocWithSizeAndAlignment(
        self: Allocator,
        comptime size: usize,
        comptime alignment: Alignment,
        n: usize,
    ) Error![*]align(alignment.toByteUnits()) u8 {
        const byte_count = math.mul(usize, size, n) catch return Error.OutOfMemory;
        return self.allocBytesWithAlignment(alignment, byte_count);
    }

    fn allocBytesWithAlignment(
        self: Allocator,
        comptime alignment: Alignment,
        byte_count: usize,
    ) Error![*]align(alignment.toByteUnits()) u8 {
        if (byte_count == 0) {
            const ptr = comptime alignment.backward(math.maxInt(usize));
            return @as([*]align(alignment.toByteUnits()) u8, @ptrFromInt(ptr));
        }

        const byte_ptr = self.rawAlloc(byte_count, alignment) orelse return Error.OutOfMemory;
        @memset(byte_ptr[0..byte_count], undefined);
        return @alignCast(byte_ptr);
    }

    pub fn resize(self: Allocator, allocation: anytype, new_len: usize) bool {
        const Slice = @typeInfo(@TypeOf(allocation)).pointer;
        const T = Slice.child;
        const alignment = Slice.alignment;
        if (new_len == 0) {
            self.free(allocation);
            return true;
        }
        if (allocation.len == 0) {
            return false;
        }
        const old_memory = mem.sliceAsBytes(allocation);
        // I would like to use saturating multiplication here, but LLVM cannot lower it
        // on WebAssembly: https://github.com/ziglang/zig/issues/9660
        //const new_len_bytes = new_len *| @sizeOf(T);
        const new_len_bytes = math.mul(usize, @sizeOf(T), new_len) catch return false;
        return self.rawResize(old_memory, .fromByteUnits(alignment), new_len_bytes);
    }

    pub fn remap(self: Allocator, allocation: anytype, new_len: usize) t: {
        const Slice = @typeInfo(@TypeOf(allocation)).pointer;
        break :t ?[]align(Slice.alignment) Slice.child;
    } {
        const Slice = @typeInfo(@TypeOf(allocation)).pointer;
        const T = Slice.child;

        const alignment = Slice.alignment;
        if (new_len == 0) {
            self.free(allocation);
            return allocation[0..0];
        }
        if (allocation.len == 0) {
            return null;
        }
        if (@sizeOf(T) == 0) {
            var new_memory = allocation;
            new_memory.len = new_len;
            return new_memory;
        }
        const old_memory = mem.sliceAsBytes(allocation);
        // I would like to use saturating multiplication here, but LLVM cannot lower it
        // on WebAssembly: https://github.com/ziglang/zig/issues/9660
        //const new_len_bytes = new_len *| @sizeOf(T);
        const new_len_bytes = math.mul(usize, @sizeOf(T), new_len) catch return null;
        const new_ptr = self.rawRemap(old_memory, .fromByteUnits(alignment), new_len_bytes) orelse return null;
        const new_memory: []align(alignment) u8 = @alignCast(new_ptr[0..new_len_bytes]);
        return mem.bytesAsSlice(T, new_memory);
    }

    pub fn realloc(self: Allocator, old_mem: anytype, new_n: usize) t: {
        const Slice = @typeInfo(@TypeOf(old_mem)).pointer;
        break :t Error![]align(Slice.alignment) Slice.child;
    } {
        return self.reallocAdvanced(old_mem, new_n);
    }

    pub fn reallocAdvanced(
        self: Allocator,
        old_mem: anytype,
        new_n: usize,
    ) t: {
        const Slice = @typeInfo(@TypeOf(old_mem)).pointer;
        break :t Error![]align(Slice.alignment) Slice.child;
    } {
        const Slice = @typeInfo(@TypeOf(old_mem)).pointer;
        const T = Slice.child;
        if (old_mem.len == 0) {
            return self.allocAdvanced(T, .fromByteUnits(Slice.alignment), new_n);
        }
        if (new_n == 0) {
            self.free(old_mem);
            const ptr = comptime std.mem.alignBackward(usize, math.maxInt(usize), Slice.alignment);
            return @as([*]align(Slice.alignment) T, @ptrFromInt(ptr))[0..0];
        }

        const old_byte_slice = mem.sliceAsBytes(old_mem);
        const byte_count = math.mul(usize, @sizeOf(T), new_n) catch return Error.OutOfMemory;
        // Note: can't set shrunk memory to undefined as memory shouldn't be modified on realloc failure
        if (self.rawRemap(old_byte_slice, .fromByteUnits(Slice.alignment), byte_count)) |p| {
            const new_bytes: []align(Slice.alignment) u8 = @alignCast(p[0..byte_count]);
            return mem.bytesAsSlice(T, new_bytes);
        }

        const new_mem = self.rawAlloc(byte_count, .fromByteUnits(Slice.alignment)) orelse
            return error.OutOfMemory;
        const copy_len = @min(byte_count, old_byte_slice.len);
        @memcpy(new_mem[0..copy_len], old_byte_slice[0..copy_len]);
        @memset(old_byte_slice, undefined);
        self.rawFree(old_byte_slice, .fromByteUnits(Slice.alignment));

        const new_bytes: []align(Slice.alignment) u8 = @alignCast(new_mem[0..byte_count]);
        return mem.bytesAsSlice(T, new_bytes);
    }

    pub fn free(self: Allocator, memory: anytype) void {
        const Slice = @typeInfo(@TypeOf(memory)).pointer;
        const bytes = mem.sliceAsBytes(memory);
        const bytes_len = bytes.len + if (Slice.sentinel() != null) @sizeOf(Slice.child) else 0;
        if (bytes_len == 0) return;
        const non_const_ptr = @constCast(bytes.ptr);
        @memset(non_const_ptr[0..bytes_len], undefined);
        self.rawFree(non_const_ptr[0..bytes_len], .fromByteUnits(Slice.alignment));
    }

    pub fn dupe(allocator: Allocator, comptime T: type, m: []const T) Error![]T {
        const new_buf = try allocator.alloc(T, m.len);
        @memcpy(new_buf, m);
        return new_buf;
    }

    pub fn dupeZ(allocator: Allocator, comptime T: type, m: []const T) Error![:0]T {
        const new_buf = try allocator.alloc(T, m.len + 1);
        @memcpy(new_buf[0..m.len], m);
        new_buf[m.len] = 0;
        return new_buf[0..m.len :0];
    }
};

test "wrap std allocator" {
    const allocator = Allocator.adaptFromStdAllocator(&testing.allocator);
    try std.heap.testAllocator(allocator.adaptIntoStdAllocator());
    try std.heap.testAllocatorAligned(allocator.adaptIntoStdAllocator());
    try std.heap.testAllocatorAlignedShrink(allocator.adaptIntoStdAllocator());
    try std.heap.testAllocatorLargeAlignment(allocator.adaptIntoStdAllocator());
}

/// Flags for the creation of an arena.
pub const ArenaFlags = packed struct(u32) {
    large_pages: bool = false,
    _: u31 = 0,
};

/// A growable thread-safe memory arena.
pub const Arena = extern struct {
    grow_futex: atomic.Value(u32) = .init(unlocked),
    flags: ArenaFlags = .{},
    page_size: usize = 0,
    reserve_len: usize = 0,
    commit_len: atomic.Value(usize) = .init(0),
    ptr: ?[*]u8 = null,
    pos: atomic.Value(usize) = .init(0),

    const unlocked: u32 = 0;
    const locked: u32 = 1;
    const contended: u32 = 2;

    pub const InitOptions = struct {
        flags: ArenaFlags = .{},
        reserve: usize,
        commit: usize = 0,
    };

    pub fn init(options: InitOptions) Error!Arena {
        std.debug.assert(options.commit <= options.reserve);
        const page_size = if (options.flags.large_pages) blk: {
            if (comptime builtin.target.os.tag == .windows) {
                const large_size = win32.system.memory.GetLargePageMinimum();
                if (large_size == 0) break :blk heap.pageSize();
                break :blk large_size;
            } else break :blk heap.pageSize();
        } else heap.pageSize();

        const reserve = mem.alignForward(usize, options.reserve, page_size);
        const commit = mem.alignForward(usize, options.commit, page_size);

        const ptr = if (comptime builtin.target.os.tag == .windows) blk: {
            const allocated = win32.system.memory.VirtualAlloc(
                null,
                reserve + page_size,
                .{ .RESERVE = 1, .LARGE_PAGES = if (options.flags.large_pages) 1 else 0 },
                .{ .PAGE_READWRITE = 1 },
            ) orelse return error.OutOfMemory;
            errdefer _ = win32.system.memory.VirtualFree(allocated, reserve + page_size, .RELEASE);
            const allocated_u8: [*]u8 = @ptrCast(allocated);

            _ = win32.system.memory.VirtualAlloc(
                allocated_u8 + reserve,
                page_size,
                .{ .COMMIT = 1, .LARGE_PAGES = if (options.flags.large_pages) 1 else 0 },
                .{ .PAGE_READWRITE = 1, .PAGE_GUARD = 1 },
            ) orelse return error.OutOfMemory;
            break :blk allocated_u8;
        } else blk: {
            const allocated = posix.mmap(
                null,
                reserve + page_size,
                posix.PROT.NONE,
                .{ .TYPE = .PRIVATE, .ANONYMOUS = true },
                -1,
                0,
            ) catch return error.OutOfMemory;
            break :blk allocated.ptr;
        };

        var arena: Arena = .{
            .flags = options.flags,
            .page_size = page_size,
            .reserve_len = reserve,
            .ptr = ptr,
        };
        errdefer arena.deinit();
        try arena.grow(commit);
        return arena;
    }

    pub fn deinit(self: *Arena) void {
        std.debug.assert(self.page_size != 0);
        const ptr = self.ptr orelse return;
        if (comptime builtin.target.os.tag == .windows) {
            _ = win32.system.memory.VirtualFree(ptr, self.reserve_len + self.page_size, .RELEASE);
        } else {
            posix.munmap(@alignCast(ptr[0 .. self.reserve_len + self.page_size]));
        }
    }

    pub fn grow(self: *Arena, len: usize) Error!void {
        std.debug.assert(self.page_size != 0);
        std.debug.assert(self.ptr != null);
        if (self.commit_len.load(.monotonic) >= len) return;
        if (self.reserve_len < len) return error.OutOfMemory;

        self.lock();
        defer self.unlock();

        const commited = self.commit_len.load(.monotonic);
        if (commited >= len) return;
        const additional = mem.alignForward(usize, len - commited, self.page_size);
        std.debug.assert(additional <= self.reserve_len);
        if (comptime builtin.target.os.tag == .windows) {
            _ = win32.system.memory.VirtualAlloc(
                self.ptr.? + commited,
                additional,
                .{ .COMMIT = 1, .LARGE_PAGES = if (self.flags.large_pages) 1 else 0 },
                .{ .PAGE_READWRITE = 1 },
            ) orelse return error.OutOfMemory;
        } else {
            posix.mprotect(
                @alignCast(self.ptr.?[commited .. commited + additional]),
                posix.PROT.READ | posix.PROT.WRITE,
            ) catch return error.OutOfMemory;
        }

        self.commit_len.store(commited + additional, .monotonic);
    }

    fn lock(self: *Arena) void {
        if (self.grow_futex.cmpxchgWeak(unlocked, locked, .acquire, .monotonic)) |v| {
            var orig = v;
            while (true) {
                if (orig == unlocked) {
                    orig = self.grow_futex.cmpxchgWeak(
                        unlocked,
                        locked,
                        .acquire,
                        .monotonic,
                    ) orelse return;
                    continue;
                }

                if (orig & contended == 0) {
                    if (self.grow_futex.cmpxchgWeak(orig, orig | contended, .acquire, .monotonic)) |n| {
                        orig = n;
                        continue;
                    }
                }

                Thread.Futex.wait(&self.grow_futex, contended);
                orig = self.grow_futex.load(.monotonic);
            }
        }
    }

    fn unlock(self: *Arena) void {
        const state = self.grow_futex.swap(unlocked, .release);
        if (state & contended != 0) Thread.Futex.wake(&self.grow_futex, 1);
    }

    const min_align: usize = 16;
    const allocator_vtable: Allocator.VTable = .{
        .alloc = alloc,
        .resize = resize,
        .remap = remap,
        .free = free,
    };

    fn alloc(ptr: ?*anyopaque, len: usize, alignment: usize) callconv(.c) ?[*]u8 {
        const self: *Arena = @ptrCast(@alignCast(ptr));
        const al = @max(alignment, min_align);
        if (len == 0) return null;

        var pos = self.pos.load(.monotonic);
        var start_pos: usize = undefined;
        const offset = @intFromPtr(self.ptr);
        while (true) {
            start_pos = mem.alignForward(usize, offset + pos, al) - offset;
            const end_pos = start_pos + len;
            if (self.commit_len.load(.monotonic) < end_pos) {
                self.grow(end_pos) catch return null;
            }
            pos = self.pos.cmpxchgWeak(pos, end_pos, .monotonic, .monotonic) orelse break;
        }

        return self.ptr.? + start_pos;
    }

    fn resize(ptr: ?*anyopaque, memory: Memory, alignment: usize, new_len: usize) callconv(.c) bool {
        _ = alignment;
        const self: *Arena = @ptrCast(@alignCast(ptr));
        const slice = memory.intoSliceOrEmpty();
        if (slice.len == 0) return false;
        if (new_len <= slice.len) return true;

        const arena_ptr = @intFromPtr(self.ptr);
        const mem_ptr = @intFromPtr(slice.ptr);
        std.debug.assert(arena_ptr <= mem_ptr);

        const start_pos = mem_ptr - arena_ptr;
        const end_pos = start_pos + slice.len;
        const new_end_pos = start_pos + new_len;
        std.debug.assert(end_pos <= self.pos.load(.monotonic));
        return self.pos.cmpxchgStrong(end_pos, new_end_pos, .monotonic, .monotonic) == null;
    }

    fn remap(ptr: ?*anyopaque, memory: Memory, alignment: usize, new_len: usize) callconv(.c) ?[*]u8 {
        const self: *Arena = @ptrCast(@alignCast(ptr));
        const slice = memory.intoSliceOrEmpty();
        if (slice.len == 0) return alloc(ptr, new_len, alignment);
        if (new_len <= slice.len) return slice.ptr;

        const arena_ptr = @intFromPtr(self.ptr);
        const mem_ptr = @intFromPtr(slice.ptr);
        std.debug.assert(arena_ptr <= mem_ptr);

        const start_pos = mem_ptr - arena_ptr;
        const end_pos = start_pos + slice.len;
        const new_end_pos = start_pos + new_len;
        std.debug.assert(end_pos <= self.pos.load(.monotonic));
        if (self.pos.cmpxchgStrong(end_pos, new_end_pos, .monotonic, .monotonic) == null) {
            return slice.ptr;
        }

        const new_mem = alloc(ptr, new_len, alignment).?;
        @memcpy(new_mem[0..slice.len], slice);
        return new_mem;
    }

    fn free(ptr: ?*anyopaque, memory: Memory, alignment: usize) callconv(.c) void {
        _ = alignment;
        const self: *Arena = @ptrCast(@alignCast(ptr));
        const slice = memory.intoSliceOrEmpty();
        if (slice.len == 0) return;

        const arena_ptr = @intFromPtr(self.ptr);
        const mem_ptr = @intFromPtr(slice.ptr);
        std.debug.assert(arena_ptr <= mem_ptr);

        const start_pos = mem_ptr - arena_ptr;
        const end_pos = start_pos + slice.len;
        std.debug.assert(end_pos <= self.pos.load(.monotonic));
        _ = self.pos.cmpxchgStrong(end_pos, start_pos, .monotonic, .monotonic);
    }

    pub fn allocator(self: *Arena) Allocator {
        return .{ .ptr = self, .vtable = &allocator_vtable };
    }
};

/// Temporary scope of a memory arena.
pub const TmpArena = extern struct {
    arena: *Arena,
    pos: usize,
};

test Arena {
    var arena = try Arena.init(.{ .reserve = 16 * 1024 * 1024 });
    defer arena.deinit();

    const allocator = arena.allocator();
    try std.heap.testAllocator(allocator.adaptIntoStdAllocator());
    try std.heap.testAllocatorAligned(allocator.adaptIntoStdAllocator());
    try std.heap.testAllocatorAlignedShrink(allocator.adaptIntoStdAllocator());
    try std.heap.testAllocatorLargeAlignment(allocator.adaptIntoStdAllocator());
}

// ----------------------------------------------------
// FFI
// ----------------------------------------------------

const ffi = struct {
    export fn fstd_arena_init(arena: *Arena, base: ?[*]u8, flags: ArenaFlags, reserve: usize, commit: usize) bool {
        std.debug.assert(base == null);
        arena.* = Arena.init(.{
            .flags = flags,
            .reserve = reserve,
            .commit = commit,
        }) catch return false;
        return true;
    }

    export fn fstd_arena_deinit(arena: *Arena) void {
        arena.deinit();
    }

    export fn fstd_arena_grow(arena: *Arena, new_len: usize) void {
        arena.grow(new_len) catch {
            @breakpoint();
            @trap();
        };
    }
};

comptime {
    _ = ffi;
}
