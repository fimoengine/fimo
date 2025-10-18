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
                .{ .PAGE_NOACCESS = 1 },
            ) orelse return error.OutOfMemory;
            errdefer _ = win32.system.memory.VirtualFree(allocated, reserve + page_size, .RELEASE);
            const allocated_u8: [*]u8 = @ptrCast(allocated);
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
        self.* = undefined;
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

pub const FreelistAllocator = extern struct {
    fallback: Allocator,
    freelist: ?*Header,
    futex: atomic.Value(u32) = .init(unlocked),

    const unlocked: u32 = 0;
    const locked: u32 = 1;
    const contended: u32 = 2;

    const allocator_vtable: Allocator.VTable = .{
        .alloc = alloc,
        .resize = resize,
        .remap = remap,
        .free = free,
    };

    const Header = struct {
        size: usize,
        next: ?*Header,
    };

    pub fn init(fallback: Allocator) FreelistAllocator {
        return .{
            .fallback = fallback,
            .freelist = null,
        };
    }

    pub fn deinit(self: *FreelistAllocator) void {
        var current = self.freelist;
        while (current) |header| {
            current = header.next;
            const buffer: [*]align(@alignOf(Header)) u8 = @ptrCast(header);
            self.fallback.free(buffer[0..header.size]);
        }
        self.* = undefined;
    }

    fn lock(self: *FreelistAllocator) void {
        if (self.futex.cmpxchgWeak(unlocked, locked, .acquire, .monotonic)) |v| {
            var orig = v;
            while (true) {
                if (orig == unlocked) {
                    orig = self.futex.cmpxchgWeak(
                        unlocked,
                        locked,
                        .acquire,
                        .monotonic,
                    ) orelse return;
                    continue;
                }

                if (orig & contended == 0) {
                    if (self.futex.cmpxchgWeak(orig, orig | contended, .acquire, .monotonic)) |n| {
                        orig = n;
                        continue;
                    }
                }

                Thread.Futex.wait(&self.futex, contended);
                orig = self.futex.load(.monotonic);
            }
        }
    }

    fn unlock(self: *FreelistAllocator) void {
        const state = self.futex.swap(unlocked, .release);
        if (state & contended != 0) Thread.Futex.wake(&self.futex, 1);
    }

    fn getHeaderPtr(ptr: [*]u8) **Header {
        return @ptrCast(@alignCast(ptr - @sizeOf(usize)));
    }

    fn getHeader(ptr: [*]u8) *Header {
        return getHeaderPtr(ptr).*;
    }

    fn alloc(ptr: ?*anyopaque, len: usize, alignment: usize) callconv(.c) ?[*]u8 {
        const self: *FreelistAllocator = @ptrCast(@alignCast(ptr));

        self.lock();
        defer self.unlock();

        var link: *?*Header = &self.freelist;
        var current = self.freelist;
        while (current) |node| {
            if (node.size >= len + alignment - 1 + @sizeOf(Header) + @sizeOf(usize)) {
                link.* = node.next;
                const unaligned_ptr: [*]u8 = @ptrCast(node);
                const unaligned_addr = @intFromPtr(unaligned_ptr);
                const aligned_addr = std.mem.alignForward(usize, unaligned_addr + @sizeOf(Header) + @sizeOf(usize), alignment);
                const aligned_ptr = unaligned_ptr + (aligned_addr - unaligned_addr);
                getHeaderPtr(aligned_ptr).* = node;
                return aligned_ptr;
            } else {
                link = &node.next;
                current = node.next;
            }
        }

        const alloc_size = len + alignment - 1 + @sizeOf(Header) + @sizeOf(usize);
        const buffer = self.fallback.alignedAlloc(u8, .of(Header), alloc_size) catch return null;
        const header: *Header = @ptrCast(buffer);
        header.* = .{
            .size = alloc_size,
            .next = null,
        };

        const unaligned_ptr: [*]u8 = @ptrCast(header);
        const unaligned_addr = @intFromPtr(unaligned_ptr);
        const aligned_addr = std.mem.alignForward(usize, unaligned_addr + @sizeOf(Header) + @sizeOf(usize), alignment);
        const aligned_ptr = unaligned_ptr + (aligned_addr - unaligned_addr);
        getHeaderPtr(aligned_ptr).* = header;
        return aligned_ptr;
    }

    fn resize(ptr: ?*anyopaque, memory: Memory, alignment: usize, new_len: usize) callconv(.c) bool {
        if (memory.len == 0 or new_len == 0) return false;
        if (new_len <= memory.len) return true;
        const self: *FreelistAllocator = @ptrCast(@alignCast(ptr));
        const header = getHeader(memory.ptr.?);
        const offset = @intFromPtr(memory.ptr) - @intFromPtr(header);
        const remaining_len = header.size - offset;
        if (remaining_len >= new_len - memory.len) return true;

        const block = @as([*]u8, @ptrCast(header))[0..header.size];
        const fallback_len = new_len + alignment - 1 + @sizeOf(Header) + @sizeOf(usize);
        if (self.fallback.rawResize(block, .of(Header), fallback_len)) {
            header.size = fallback_len;
            return true;
        }
        return false;
    }

    fn remap(ptr: ?*anyopaque, memory: Memory, alignment: usize, new_len: usize) callconv(.c) ?[*]u8 {
        if (memory.len == 0 or new_len == 0) return null;
        if (new_len <= memory.len) return memory.ptr;
        const self: *FreelistAllocator = @ptrCast(@alignCast(ptr));
        const header = getHeader(memory.ptr.?);
        const offset = @intFromPtr(memory.ptr) - @intFromPtr(header);
        const remaining_len = header.size - offset;
        if (remaining_len >= new_len - memory.len) return memory.ptr;

        const block = @as([*]u8, @ptrCast(header))[0..header.size];
        const fallback_len = new_len + alignment - 1 + @sizeOf(Header) + @sizeOf(usize);
        const unaligned_ptr = self.fallback.rawRemap(block, .of(Header), fallback_len) orelse return null;
        const new_header: *Header = @ptrCast(@alignCast(unaligned_ptr));
        new_header.* = .{
            .size = fallback_len,
            .next = null,
        };

        const unaligned_addr = @intFromPtr(unaligned_ptr);
        const aligned_addr = std.mem.alignForward(usize, unaligned_addr + @sizeOf(Header) + @sizeOf(usize), alignment);
        const aligned_ptr = unaligned_ptr + (aligned_addr - unaligned_addr);
        getHeaderPtr(aligned_ptr).* = new_header;
        return aligned_ptr;
    }

    fn free(ptr: ?*anyopaque, memory: Memory, alignment: usize) callconv(.c) void {
        _ = alignment;
        if (memory.len == 0) return;
        const self: *FreelistAllocator = @ptrCast(@alignCast(ptr));

        self.lock();
        defer self.unlock();

        const header = getHeader(memory.ptr.?);
        header.next = self.freelist;
        self.freelist = header;
    }

    pub fn allocator(self: *FreelistAllocator) Allocator {
        return .{ .ptr = self, .vtable = &allocator_vtable };
    }
};

test FreelistAllocator {
    const fallback = Allocator.adaptFromStdAllocator(&std.testing.allocator);
    var freelist = FreelistAllocator.init(fallback);
    defer freelist.deinit();

    const allocator = freelist.allocator();
    try std.heap.testAllocator(allocator.adaptIntoStdAllocator());
    try std.heap.testAllocatorAligned(allocator.adaptIntoStdAllocator());
    try std.heap.testAllocatorAlignedShrink(allocator.adaptIntoStdAllocator());
    try std.heap.testAllocatorLargeAlignment(allocator.adaptIntoStdAllocator());
}

pub const BuddyAllocator = extern struct {
    freelist: FreelistAllocator,
    futex: atomic.Value(u32) = .init(unlocked),
    block_size: usize,
    max_order: u8,
    pages: ?*Page,

    const unlocked: u32 = 0;
    const locked: u32 = 1;
    const contended: u32 = 2;

    const min_block_size = 16;
    const max_supported_order = 20;
    const allocator_vtable: Allocator.VTable = .{
        .alloc = alloc,
        .resize = resize,
        .remap = remap,
        .free = free,
    };

    const Block = packed struct(u64) {
        idx: u20,
        prev: u20,
        next: u20,
        has_prev: bool,
        has_next: bool,
        _unused: u2 = 0,
    };

    pub const Page = extern struct {
        blocks: [*]u8,
        bit_tree: [*]u8,
        free_lists: [max_supported_order]?*Block = @splat(null),
        next: ?*Page = null,

        fn checkBit(self: *Page, max_order: u8, order: u8, index: u20) bool {
            const order_start: usize = (@as(usize, 1) << @truncate(max_order - order)) - 1;
            const bit_index = order_start + index;
            const byte_index = @divTrunc(bit_index, 8);
            const bit_offset = @rem(bit_index, 8);
            const byte = self.bit_tree[byte_index];
            return (byte & (@as(u8, 1) << @truncate(bit_offset))) != 0;
        }

        fn setBit(self: *Page, max_order: u8, order: u8, index: u20) void {
            const order_start: usize = (@as(usize, 1) << @truncate(max_order - order)) - 1;
            const bit_index = order_start + index;
            const byte_index = @divTrunc(bit_index, 8);
            const bit_offset = @rem(bit_index, 8);
            std.debug.assert((self.bit_tree[byte_index] & (@as(u8, 1) << @truncate(bit_offset))) == 0);
            self.bit_tree[byte_index] |= @as(u8, 1) << @truncate(bit_offset);
        }

        fn clearBit(self: *Page, max_order: u8, order: u8, index: u20) void {
            const order_start: usize = (@as(usize, 1) << @truncate(max_order - order)) - 1;
            const bit_index = order_start + index;
            const byte_index = @divTrunc(bit_index, 8);
            const bit_offset = @rem(bit_index, 8);
            std.debug.assert((self.bit_tree[byte_index] & (@as(u8, 1) << @truncate(bit_offset))) != 0);
            self.bit_tree[byte_index] &= ~(@as(u8, 1) << @truncate(bit_offset));
        }

        fn flattenBlockIdx(order: u8, index: u20) usize {
            return @as(usize, index) << @truncate(order);
        }

        fn allocateBlock(self: *Page, max_order: u8, order: u8, block_size: usize) ?[*]u8 {
            // Try to take the block from the free list directly.
            if (self.free_lists[order]) |block| {
                std.debug.assert(!block.has_prev);
                self.clearBit(max_order, order, block.idx);
                if (block.has_next) {
                    const next_idx = block.next;
                    const next_idx_flat = flattenBlockIdx(order, next_idx);
                    const offset = next_idx_flat * block_size;
                    const next: *Block = @ptrCast(@alignCast(self.blocks + offset));
                    next.has_prev = false;
                    self.free_lists[order] = next;
                } else {
                    self.free_lists[order] = null;
                }
                const bytes: []u8 = @ptrCast(block);
                @memset(bytes, 0);
                return bytes.ptr;
            }

            // If no block of the current order is available, we try
            // splitting blocks of higher order.
            var split_order: ?u8 = null;
            for (order + 1..max_order + 1) |i| {
                if (self.free_lists[i] != null) {
                    split_order = @truncate(i);
                    break;
                }
            }

            // If no block was found, we quit.
            var current_order = split_order orelse return null;
            const block = self.free_lists[current_order].?;
            if (block.has_next) {
                const next_idx = block.next;
                const next_idx_flat = flattenBlockIdx(current_order, next_idx);
                const offset = next_idx_flat * block_size;
                const next: *Block = @ptrCast(@alignCast(self.blocks + offset));
                next.has_prev = false;
                self.free_lists[current_order] = next;
            } else {
                self.free_lists[current_order] = null;
            }

            // At each step we half the size of the block and
            // insert it into the free list of the size class.
            // We know that the free lists must be empty.
            self.clearBit(max_order, current_order, block.idx);
            while (current_order > order) {
                current_order -= 1;

                const block_idx = 2 * block.idx;
                const buddy_idx = block_idx + 1;
                const buddy_idx_flat = flattenBlockIdx(current_order, buddy_idx);
                const buddy_offset = buddy_idx_flat * block_size;
                const buddy: *Block = @ptrCast(@alignCast(self.blocks + buddy_offset));

                block.* = .{
                    .idx = block_idx,
                    .prev = 0,
                    .has_prev = false,
                    .next = 0,
                    .has_next = false,
                };
                buddy.* = .{
                    .idx = buddy_idx,
                    .prev = 0,
                    .has_prev = false,
                    .next = 0,
                    .has_next = false,
                };
                self.free_lists[current_order] = buddy;
                self.setBit(max_order, current_order, buddy_idx);
            }

            const bytes: []u8 = @ptrCast(block);
            @memset(bytes, 0);
            return bytes.ptr;
        }

        fn deallocateBlock(
            self: *Page,
            max_order: u8,
            order: u8,
            block_size: usize,
            flat_block_idx: usize,
        ) void {
            const block_offset = flat_block_idx * block_size;
            var block: *Block = @ptrCast(@alignCast(self.blocks + block_offset));
            block.* = .{
                .idx = @truncate(flat_block_idx >> @truncate(order)),
                .prev = 0,
                .has_prev = false,
                .next = 0,
                .has_next = false,
            };

            // Try to coalesce the block with its buddy.
            for (order..max_order + 1) |i| {
                const buddy_idx = if ((block.idx & 1) == 0) block.idx + 1 else block.idx - 1;
                // If the buddy is not free we simply insert the block into the free list.
                if (!self.checkBit(max_order, @truncate(i), buddy_idx)) {
                    const next_idx, const has_next = if (self.free_lists[i]) |next| blk: {
                        next.prev = block.idx;
                        next.has_prev = true;
                        break :blk .{ next.idx, true };
                    } else .{ 0, false };

                    block.next = next_idx;
                    block.has_next = has_next;
                    self.free_lists[i] = block;
                    self.setBit(max_order, @truncate(i), block.idx);
                    return;
                }

                // If the buddy is free we must remove it from the free list.
                self.clearBit(max_order, @truncate(i), buddy_idx);
                const buddy_idx_flat = flattenBlockIdx(@truncate(i), buddy_idx);
                const buddy_offset = buddy_idx_flat * block_size;
                const buddy: *Block = @ptrCast(@alignCast(self.blocks + buddy_offset));
                if (buddy.has_prev) {
                    const prev_idx = buddy.prev;
                    const prev_idx_flat = flattenBlockIdx(@truncate(i), prev_idx);
                    const prev_offset = prev_idx_flat * block_size;
                    const prev: *Block = @ptrCast(@alignCast(self.blocks + prev_offset));
                    prev.next = buddy.next;
                    prev.has_next = buddy.has_next;
                } else {
                    std.debug.assert(self.free_lists[i] == buddy);
                    self.free_lists[i] = null;
                }

                block = if ((buddy_idx & 1) == 1) block else buddy;
                block.idx /= 2;
            }
        }
    };

    pub fn init(fallback: Allocator, block_size: usize, page_size: usize) !BuddyAllocator {
        const rounded_block_size = @max(min_block_size, try std.math.ceilPowerOfTwo(usize, block_size));
        const rounded_page_size = @max(rounded_block_size, try std.math.ceilPowerOfTwo(usize, page_size));
        const max_order = @ctz(rounded_page_size) - @ctz(rounded_block_size);
        if (max_order > max_supported_order) return error.UnsupportedSizeCombination;
        return .{
            .freelist = FreelistAllocator.init(fallback),
            .block_size = rounded_block_size,
            .max_order = max_order,
            .pages = null,
        };
    }

    pub fn deinit(self: *BuddyAllocator) void {
        const num_blocks: usize = @as(usize, 1) << @truncate(self.max_order);
        const bit_tree_elements = (num_blocks * 2) - 1;
        const blocks_memory_size = self.block_size * num_blocks;
        const blocks_offset = std.mem.alignForward(usize, @sizeOf(Page) + bit_tree_elements, self.block_size);
        const page_size = blocks_offset + blocks_memory_size;

        var current = self.pages;
        while (current) |page| {
            current = page.next;
            const ptr: [*]u8 = @ptrCast(page);
            self.freelist.fallback.rawFree(ptr[0..page_size], .fromByteUnits(self.block_size));
        }
        self.freelist.deinit();
        self.* = undefined;
    }

    fn lock(self: *BuddyAllocator) void {
        if (self.futex.cmpxchgWeak(unlocked, locked, .acquire, .monotonic)) |v| {
            var orig = v;
            while (true) {
                if (orig == unlocked) {
                    orig = self.futex.cmpxchgWeak(
                        unlocked,
                        locked,
                        .acquire,
                        .monotonic,
                    ) orelse return;
                    continue;
                }

                if (orig & contended == 0) {
                    if (self.futex.cmpxchgWeak(orig, orig | contended, .acquire, .monotonic)) |n| {
                        orig = n;
                        continue;
                    }
                }

                Thread.Futex.wait(&self.futex, contended);
                orig = self.futex.load(.monotonic);
            }
        }
    }

    fn unlock(self: *BuddyAllocator) void {
        const state = self.futex.swap(unlocked, .release);
        if (state & contended != 0) Thread.Futex.wake(&self.futex, 1);
    }

    fn blockOrderForSize(self: *BuddyAllocator, len: usize, alignment: usize) u8 {
        const block_size = if (alignment <= self.block_size)
            std.math.ceilPowerOfTwoAssert(usize, @max(len, self.block_size))
        else
            std.math.ceilPowerOfTwoAssert(usize, @max(len + alignment - 1, self.block_size));
        return @ctz(block_size) - @ctz(self.block_size);
    }

    fn alloc(ptr: ?*anyopaque, len: usize, alignment: usize) callconv(.c) ?[*]u8 {
        const self: *BuddyAllocator = @ptrCast(@alignCast(ptr));
        if (len == 0) return null;
        const block_order = self.blockOrderForSize(len, alignment);
        if (block_order > self.max_order)
            return FreelistAllocator.alloc(&self.freelist, len, alignment);

        self.lock();
        defer self.unlock();

        var link: *?*Page = &self.pages;
        var current = self.pages;
        while (true) {
            const page = current orelse blk: {
                const num_blocks: usize = @as(usize, 1) << @truncate(self.max_order);
                const bit_tree_elements = (num_blocks * 2) - 1;
                const blocks_memory_size = self.block_size * num_blocks;
                const blocks_offset = std.mem.alignForward(usize, @sizeOf(Page) + bit_tree_elements, self.block_size);
                const allocation_size = blocks_offset + blocks_memory_size;
                const page_bytes = self.freelist.fallback.rawAlloc(
                    allocation_size,
                    .fromByteUnits(self.block_size),
                ) orelse return null;

                const p: *Page = @ptrCast(@alignCast(page_bytes));
                p.* = .{
                    .blocks = page_bytes + blocks_offset,
                    .bit_tree = page_bytes + @sizeOf(Page),
                };

                const super_block: *Block = @ptrCast(@alignCast(p.blocks));
                super_block.* = .{
                    .idx = 0,
                    .prev = 0,
                    .has_prev = false,
                    .next = 0,
                    .has_next = false,
                };
                p.setBit(self.max_order, self.max_order, 0);
                p.free_lists[self.max_order] = super_block;

                link.* = p;
                break :blk p;
            };

            if (page.allocateBlock(self.max_order, block_order, self.block_size)) |block| {
                // NOTE(gabriel): Blocks are always aligned to the block-size.
                // For overallocated buffers, we allocate a bigger block and
                // return an allocated pointer inside the block.
                if (alignment <= self.block_size)
                    return block
                else {
                    const unaligned_addr = @intFromPtr(block);
                    const aligned_addr = std.mem.alignForward(usize, unaligned_addr + @sizeOf(usize), alignment);
                    return block + (aligned_addr - unaligned_addr);
                }
            }
            link = &page.next;
            current = page.next;
        }
    }

    fn resize(ptr: ?*anyopaque, memory: Memory, alignment: usize, new_len: usize) callconv(.c) bool {
        if (memory.len == 0) return false;
        const self: *BuddyAllocator = @ptrCast(@alignCast(ptr));
        const old_block_order = self.blockOrderForSize(memory.len, alignment);
        if (old_block_order > self.max_order)
            return FreelistAllocator.resize(&self.freelist, memory, alignment, new_len);
        const new_block_order = self.blockOrderForSize(new_len, alignment);
        return old_block_order == new_block_order;
    }

    fn remap(ptr: ?*anyopaque, memory: Memory, alignment: usize, new_len: usize) callconv(.c) ?[*]u8 {
        if (memory.len == 0) return null;
        const self: *BuddyAllocator = @ptrCast(@alignCast(ptr));
        const old_block_order = self.blockOrderForSize(memory.len, alignment);
        if (old_block_order > self.max_order)
            return FreelistAllocator.remap(&self.freelist, memory, alignment, new_len);
        const new_block_order = self.blockOrderForSize(new_len, alignment);
        return if (old_block_order == new_block_order) memory.ptr else null;
    }

    fn free(ptr: ?*anyopaque, memory: Memory, alignment: usize) callconv(.c) void {
        if (memory.len == 0) return;
        const self: *BuddyAllocator = @ptrCast(@alignCast(ptr));
        const block_order = self.blockOrderForSize(memory.len, alignment);
        if (block_order > self.max_order)
            return FreelistAllocator.free(&self.freelist, memory, alignment);
        const num_blocks: usize = @as(usize, 1) << @truncate(self.max_order);
        const max_offset = num_blocks * self.block_size;
        const address = @intFromPtr(memory.ptr);

        self.lock();
        defer self.unlock();

        var current = self.pages;
        while (current) |page| : (current = page.next) {
            const start = @intFromPtr(page.blocks);
            const end = start + max_offset;
            if (address < start or address > end) continue;

            const offset = address - start;
            // NOTE(gabriel): For overallocated buffers, the allocation
            // is done post hoc, which may lead to pointers pointing
            // in the middle of an allocated block.
            const block_idx = if (alignment <= self.block_size) blk: {
                // NOTE(gabriel): This is a division by a power of two:
                //
                // idx = offset / self.block_size
                break :blk offset >> @truncate(@ctz(self.block_size));
            } else blk: {
                // NOTE(gabriel):
                //
                // Shift the pointer to the beginning of the block:
                // block_start = (offset / block_size) * block_size
                //
                // Then we compute the index, like before:
                // idx = block_start / self.block_size
                const block_size = self.block_size << @truncate(block_order);
                const clear_bits = @ctz(block_size);
                const shift = clear_bits - @ctz(self.block_size);
                break :blk (offset >> @truncate(clear_bits)) << @truncate(shift);
            };
            page.deallocateBlock(self.max_order, block_order, self.block_size, block_idx);
            return;
        }
        unreachable;
    }

    pub fn allocator(self: *BuddyAllocator) Allocator {
        return .{ .ptr = self, .vtable = &allocator_vtable };
    }
};

test BuddyAllocator {
    const fallback = Allocator.adaptFromStdAllocator(&std.testing.allocator);
    const block_size = 256;
    const page_size = 1024 * 1024;
    var buddy = try BuddyAllocator.init(fallback, block_size, page_size);
    defer buddy.deinit();

    const allocator = buddy.allocator();
    try std.heap.testAllocator(allocator.adaptIntoStdAllocator());
    try std.heap.testAllocatorAligned(allocator.adaptIntoStdAllocator());
    try std.heap.testAllocatorAlignedShrink(allocator.adaptIntoStdAllocator());
    try std.heap.testAllocatorLargeAlignment(allocator.adaptIntoStdAllocator());
}

pub const MultiSlabAllocatorOptions = struct {
    /// Length in bytes of one slab.
    ///
    /// Must be a power of two.
    slab_len: usize = @max(std.heap.page_size_max, 1024 * 64),
    /// Maximum number of arenas.
    ///
    /// An arena contains multiple slabs.
    /// This value is optimally set to the expected number of cpu cores on the target system.
    max_arenas_count: usize = 64,
    /// Maximum number of allocation retries before allocating a new slab.
    max_alloc_retries: usize = 1,
};

/// General purpose allocator, inspired by the Zig SmpAllocator.
///
/// The allocator utilizes a mixture of local and global state.
/// Specifically, the allocator defines multiple memory arenas, each containing multiple slabs and free lists.
/// Optimally, each thread is assigned one memory arena, whose index is stored in a global threadlocal variable.
///
/// A new allocator can be instantiated by providing an unique type tag.
pub fn MultiSlabAllocator(comptime Unique: type, comptime options: MultiSlabAllocatorOptions) type {
    std.debug.assert(std.math.isPowerOfTwo(options.slab_len));
    std.debug.assert(options.max_arenas_count != 0);

    return struct {
        cpu_count: usize,
        fallback_allocator: Allocator,
        arenas: [max_arenas_count]Self.Arena = @splat(.{}),

        threadlocal var arena_index: usize = 0;

        const Self = @This();
        const _ = Unique;

        const slab_len = options.slab_len;
        const max_arenas_count = options.max_arenas_count;
        const max_alloc_retries = options.max_alloc_retries;

        const min_size_class = std.math.log2(@sizeOf(usize));
        const size_class_count = std.math.log2(slab_len) - min_size_class;

        const allocator_vtable: Allocator.VTable = .{
            .alloc = alloc,
            .resize = resize,
            .remap = remap,
            .free = free,
        };
        const std_allocator_vtable: StdAllocator.VTable = .{
            .alloc = stdAlloc,
            .resize = stdResize,
            .remap = stdRemap,
            .free = stdFree,
        };

        const Arena = struct {
            _: void align(std.atomic.cache_line) = {},
            mutex: std.Thread.Mutex = .{},
            slabs: [size_class_count]usize = @splat(0),
            free_lists: [size_class_count]usize = @splat(0),

            fn unlock(self: *Self.Arena) void {
                self.mutex.unlock();
            }
        };

        pub fn init(fallback: Allocator) Self {
            return .{
                .cpu_count = @min(std.Thread.getCpuCount() catch max_arenas_count, max_arenas_count),
                .fallback_allocator = fallback,
            };
        }

        pub fn deinit(self: *Self) void {
            // The free list almost certainly contains multiple slabs,
            // split into multiple blocks. Before deallocating the slabs, we
            // must first find all allocations.
            var slabs_head: ?*usize = null;
            var slabs_tail: ?*usize = null;

            for (self.arenas[0..self.cpu_count]) |*arena| {
                for (arena.free_lists) |head| {
                    var current = head;
                    while (current != 0) {
                        const node: *usize = @ptrFromInt(current);
                        const next = node.*;
                        current = next;

                        if ((@intFromPtr(node) % slab_len) == 0) {
                            node.* = 0;
                            if (slabs_tail) |tail| {
                                tail.* = @intFromPtr(node);
                            } else {
                                slabs_head = node;
                            }
                            slabs_tail = node;
                        }
                    }
                }
            }

            while (slabs_head) |slab| {
                slabs_head = @ptrFromInt(slab.*);
                const bytes: [*]u8 = @ptrCast(slab);
                self.fallback_allocator.rawFree(bytes[0..slab_len], .fromByteUnits(slab_len));
            }

            self.* = undefined;
        }

        fn alloc(ptr: ?*anyopaque, len: usize, alignment: usize) callconv(.c) ?[*]u8 {
            const self: *Self = @ptrCast(@alignCast(ptr));
            const size_class = sizeClassIndex(len, .fromByteUnits(alignment));
            if (size_class >= size_class_count) {
                @branchHint(.unlikely);
                return self.fallback_allocator.rawAlloc(len, .fromByteUnits(alignment));
            }

            const slot_size = slotSize(size_class);
            std.debug.assert(slab_len % slot_size == 0);
            var retry_count: u8 = 0;

            var arena = self.lockArena();
            defer arena.unlock();

            outer: while (true) {
                const free_list_top = arena.free_lists[size_class];
                if (free_list_top != 0) {
                    @branchHint(.likely);
                    const node: *usize = @ptrFromInt(free_list_top);
                    arena.free_lists[size_class] = node.*;
                    return @ptrFromInt(free_list_top);
                }

                const slab_top = arena.slabs[size_class];
                if ((slab_top % slab_len) != 0) {
                    @branchHint(.likely);
                    arena.slabs[size_class] = slab_top + slot_size;
                    return @ptrFromInt(slab_top);
                }

                if (retry_count >= max_alloc_retries) {
                    @branchHint(.likely);
                    const slab = self.fallback_allocator.rawAlloc(slab_len, .fromByteUnits(slab_len)) orelse return null;
                    arena.slabs[size_class] = @intFromPtr(slab) + slot_size;
                    return slab;
                }

                arena.unlock();
                var index = arena_index;
                while (true) {
                    index = (index + 1) % self.cpu_count;
                    arena = &self.arenas[index];
                    if (arena.mutex.tryLock()) {
                        arena_index = index;
                        retry_count +%= 1;
                        continue :outer;
                    }
                }
            }
        }

        fn resize(ptr: ?*anyopaque, memory: Memory, alignment: usize, new_len: usize) callconv(.c) bool {
            const self: *Self = @ptrCast(@alignCast(ptr));
            const slice = memory.intoSliceOrEmpty();
            const size_class = sizeClassIndex(slice.len, .fromByteUnits(alignment));
            const new_size_class = sizeClassIndex(new_len, .fromByteUnits(alignment));
            if (size_class >= size_class_count) {
                if (new_size_class < size_class_count) return false;
                return self.fallback_allocator.rawResize(slice, .fromByteUnits(alignment), new_len);
            }
            return size_class == new_size_class;
        }

        fn remap(ptr: ?*anyopaque, memory: Memory, alignment: usize, new_len: usize) callconv(.c) ?[*]u8 {
            const self: *Self = @ptrCast(@alignCast(ptr));
            const slice = memory.intoSliceOrEmpty();
            const size_class = sizeClassIndex(slice.len, .fromByteUnits(alignment));
            const new_size_class = sizeClassIndex(new_len, .fromByteUnits(alignment));
            if (size_class >= size_class_count) {
                if (new_size_class < size_class_count) return null;
                return self.fallback_allocator.rawRemap(slice, .fromByteUnits(alignment), new_len);
            }
            return if (size_class == new_size_class) slice.ptr else null;
        }

        fn free(ptr: ?*anyopaque, memory: Memory, alignment: usize) callconv(.c) void {
            const self: *Self = @ptrCast(@alignCast(ptr));
            const slice = memory.intoSliceOrEmpty();
            const size_class = sizeClassIndex(slice.len, .fromByteUnits(alignment));
            if (size_class >= size_class_count) {
                @branchHint(.unlikely);
                return self.fallback_allocator.rawFree(slice, .fromByteUnits(alignment));
            }

            const node: *usize = @ptrCast(@alignCast(slice.ptr));

            const arena = self.lockArena();
            defer arena.unlock();

            node.* = arena.free_lists[size_class];
            arena.free_lists[size_class] = @intFromPtr(node);
        }

        fn lockArena(self: *Self) *Self.Arena {
            var index = arena_index;
            {
                const arena = &self.arenas[index];
                if (arena.mutex.tryLock()) {
                    @branchHint(.likely);
                    return arena;
                }
            }

            std.debug.assert(self.cpu_count != 0);
            while (true) {
                index = (index + 1) % self.cpu_count;
                const arena = &self.arenas[index];
                if (arena.mutex.tryLock()) {
                    @branchHint(.likely);
                    arena_index = index;
                    return arena;
                }
            }
        }

        fn sizeClassIndex(len: usize, alignment: mem.Alignment) usize {
            return @max(@bitSizeOf(usize) - @clz(len - 1), @intFromEnum(alignment), min_size_class) - min_size_class;
        }

        fn slotSize(class: usize) usize {
            return @as(usize, 1) << @intCast(class + min_size_class);
        }

        pub fn allocator(self: *Self) Allocator {
            return .{ .ptr = self, .vtable = &allocator_vtable };
        }

        fn stdAlloc(ptr: *anyopaque, len: usize, alignment: Alignment, ret_addr: usize) ?[*]u8 {
            _ = ret_addr;
            return alloc(ptr, len, alignment.toByteUnits());
        }
        fn stdResize(ptr: *anyopaque, memory: []u8, alignment: Alignment, new_len: usize, ret_addr: usize) bool {
            _ = ret_addr;
            return resize(ptr, .fromSlice(memory), alignment.toByteUnits(), new_len);
        }
        fn stdRemap(ptr: *anyopaque, memory: []u8, alignment: Alignment, new_len: usize, ret_addr: usize) ?[*]u8 {
            _ = ret_addr;
            return remap(ptr, .fromSlice(memory), alignment.toByteUnits(), new_len);
        }
        fn stdFree(ptr: *anyopaque, memory: []u8, alignment: Alignment, ret_addr: usize) void {
            _ = ret_addr;
            return free(ptr, .fromSlice(memory), alignment.toByteUnits());
        }

        pub fn stdAllocator(self: *Self) StdAllocator {
            return .{ .ptr = self, .vtable = &std_allocator_vtable };
        }
    };
}

test MultiSlabAllocator {
    const MSAllocator = MultiSlabAllocator(struct {}, .{});
    const fallback = Allocator.adaptFromStdAllocator(&std.testing.allocator);
    var ms_allocator = MSAllocator.init(fallback);
    defer ms_allocator.deinit();

    const allocator = ms_allocator.allocator();
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
