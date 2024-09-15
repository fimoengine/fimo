const std = @import("std");
const builtin = @import("builtin");
const Allocator = std.mem.Allocator;

const c = @import("c.zig");
const errors = @import("errors.zig");

extern fn _aligned_malloc(size: usize, alignment: usize) ?*anyopaque;
extern fn _aligned_free(ptr: ?*anyopaque) void;
extern fn _aligned_msize(ptr: ?*anyopaque, alignment: usize, offset: usize) usize;
extern fn aligned_alloc(alignment: usize, size: usize) ?*anyopaque;

const malloc_size = if (@TypeOf(std.c.malloc_size) != void)
    std.c.malloc_size
else if (@TypeOf(std.c.malloc_usable_size) != void)
    std.c.malloc_usable_size
else {};

fn rawAlloc(size: usize, alignment: usize) ?*anyopaque {
    return switch (builtin.os.tag) {
        .windows => _aligned_malloc(size, alignment),
        else => aligned_alloc(alignment, size),
    };
}

fn rawFree(ptr: ?*anyopaque) void {
    switch (builtin.os.tag) {
        .windows => _aligned_free(ptr),
        else => std.c.free(ptr),
    }
}

fn rawAllocSize(ptr: ?*anyopaque, alignment: usize) usize {
    return switch (builtin.os.tag) {
        .windows => _aligned_msize(ptr, alignment, 0),
        else => malloc_size(ptr),
    };
}

const FimoAllocator = struct {
    fn alloc(
        _: *anyopaque,
        len: usize,
        log2_align: u8,
        return_address: usize,
    ) ?[*]u8 {
        _ = return_address;
        std.debug.assert(len > 0);

        const alignment = @as(usize, 1) << @as(Allocator.Log2Align, @intCast(log2_align));
        // The alignment must be a multiple of the pointer size.
        const eff_alignment = @max(alignment, @sizeOf(usize));
        // The size must also be a multiple of the alignment.
        const size = (len + eff_alignment - 1) & ~(eff_alignment - 1);

        return @as(?[*]u8, @ptrCast(rawAlloc(size, eff_alignment)));
    }

    fn resize(
        _: *anyopaque,
        buf: []u8,
        log2_buf_align: u8,
        new_len: usize,
        return_address: usize,
    ) bool {
        _ = return_address;
        if (new_len <= buf.len) {
            return true;
        }

        const alignment = @as(usize, 1) << @as(Allocator.Log2Align, @intCast(log2_buf_align));
        const eff_alignment = @max(alignment, @sizeOf(usize));
        const full_len = rawAllocSize(buf.ptr, eff_alignment);
        return new_len <= full_len;
    }

    fn free(
        _: *anyopaque,
        buf: []u8,
        log2_buf_align: u8,
        return_address: usize,
    ) void {
        _ = log2_buf_align;
        _ = return_address;
        rawFree(buf.ptr);
    }
};

/// Default allocator of the fimo project.
///
/// Uses a system allocator that works accross shared libraries
/// and is thread safe.
pub const fimo_allocator = Allocator{
    .ptr = undefined,
    .vtable = &fimo_allocator_vtable,
};

/// VTable of the default allocator.
pub const fimo_allocator_vtable = Allocator.VTable{
    .alloc = FimoAllocator.alloc,
    .resize = FimoAllocator.resize,
    .free = FimoAllocator.free,
};

/// Default alignment of the default allocator.
pub const fimo_allocator_alignment: usize = switch (builtin.os.tag) {
    .windows => 16,
    else => @alignOf(std.c.max_align_t),
};

/// Allocate memory.
///
/// This function allocates at least `size` bytes and returns a pointer to the allocated
/// memory. The memory is not initialized. If `size` is `0`, then `fimo_malloc()`
/// returns `NULL`. If `error` is not a null pointer, `fimo_malloc()` writes the
/// success status into the memory pointed to by `error`.
export fn fimo_malloc(size: usize, err: ?*c.FimoResult) ?*anyopaque {
    return fimo_malloc_sized(size, err).ptr;
}

/// Zero-allocate memory.
///
/// This function allocates at least `size` bytes and returns a pointer to the allocated
/// memory. The memory is zero-initialized. If `size` is `0`, then `fimo_malloc()`
/// returns `NULL`. If `error` is not a null pointer, `fimo_calloc()` writes the
/// success status into the memory pointed to by `error`.
export fn fimo_calloc(size: usize, err: ?*c.FimoResult) ?*anyopaque {
    return fimo_calloc_sized(size, err).ptr;
}

/// Allocate memory.
///
/// This function allocates at least `size` bytes and returns a pointer to the allocated
/// memory that is aligned at least as strictly as `alignment`. The memory is not initialized.
/// If `size` is `0`, then `fimo_aligned_alloc()` returns `NULL` and `alignment` is ignored.
/// `alignment` must be a power of two greater than `0`. If `error` is not a null pointer,
/// `fimo_aligned_alloc()` writes the success status into the memory pointed to by `error`.
export fn fimo_aligned_alloc(alignment: usize, size: usize, err: ?*c.FimoResult) ?*anyopaque {
    return fimo_aligned_alloc_sized(alignment, size, err).ptr;
}

/// Allocate memory.
///
/// This function allocates at least `size` bytes and returns a pointer to the allocated
/// memory, along with the usable size in bytes. The memory is not initialized. If `size`
/// is `0`, then `fimo_malloc_sized()` returns `NULL`. If `error` is not a null pointer,
/// `fimo_malloc_sized()` writes the success status into the memory pointed to by `error`.
export fn fimo_malloc_sized(size: usize, err: ?*c.FimoResult) c.FimoMallocBuffer {
    return fimo_aligned_alloc_sized(fimo_allocator_alignment, size, err);
}

/// Zero-allocate memory.
///
/// This function allocates at least `size` bytes and returns a pointer to the allocated
/// memory, along with the usable size in bytes. The memory is zero-initialized. If `size`
/// is `0`, then `fimo_calloc_sized()` returns `NULL`. If `error` is not a null pointer,
/// `fimo_calloc_sized()` writes the success status into the memory pointed to by `error`.
export fn fimo_calloc_sized(size: usize, err: ?*c.FimoResult) c.FimoMallocBuffer {
    const buffer = fimo_malloc_sized(size, err);
    if (buffer.ptr) |ptr| {
        const u8_ptr: [*]u8 = @ptrCast(ptr);
        @memset(u8_ptr[0..buffer.buff_size], 0);
    }
    return buffer;
}

/// Allocate memory.
///
/// This function allocates at least `size` bytes and returns a pointer to the allocated
/// memory that is aligned at least as strictly as `alignment`, along with the usable size
/// in bytes. The memory is not initialized. If `size` is `0`, then
/// `fimo_aligned_alloc_sized()` returns `NULL` and `alignment` is ignored. `alignment`
/// must be a power of two greater than `0`. If `error` is not a null pointer,
/// `fimo_aligned_alloc_sized()` writes the success status into the memory pointed to
/// by `error`.
export fn fimo_aligned_alloc_sized(alignment: usize, size: usize, err: ?*c.FimoResult) c.FimoMallocBuffer {
    const ok_error = errors.Error.intoCResult(null);
    const inval_error = errors.Error.initErrorCode(errors.ErrorCode.inval).?.err;
    const nomem_error = errors.Error.initErrorCode(errors.ErrorCode.nomem).?.err;

    if (size == 0 or alignment == 0 or ((alignment & (alignment - 1)) != 0)) {
        if (err) |e| {
            e.* = if (size == 0) ok_error else inval_error;
        }
        return .{ .ptr = null, .buff_size = 0 };
    }

    const log2_align = @ctz(alignment);
    const allocated = FimoAllocator.alloc(undefined, size, log2_align, 0);
    if (allocated) |ptr| {
        const eff_alignment = @max(alignment, @sizeOf(usize));
        const real_len = rawAllocSize(ptr, eff_alignment);
        if (err) |e| e.* = ok_error;
        return .{ .ptr = ptr, .buff_size = real_len };
    }

    if (err) |e| e.* = nomem_error;
    return .{ .ptr = null, .buff_size = 0 };
}

/// Free allocated memory.
///
/// Deallocates the memory allocated by an allocation function. If `ptr` is a null pointer,
/// no action shall occur. Otherwise, if `ptr` does not match a pointer returned by the
/// allocation function, or if the space has been deallocated by a call to `fimo_free()`,
/// `fimo_free_sized()` or `fimo_free_aligned_sized()`, the behavior is undefined.
export fn fimo_free(ptr: ?*anyopaque) void {
    rawFree(ptr);
}

/// Free allocated memory.
///
/// Deallocates the memory allocated by an allocation function. If `ptr` is a null pointer,
/// no action shall occur. Otherwise, if `ptr` does not match a pointer returned by the
/// allocation function, or if the space has been deallocated by a call to `fimo_free()`,
/// `fimo_free_sized()` or `fimo_free_aligned_sized()`, or if `size` does not match
/// the size used to allocate the memory, the behavior is undefined.
export fn fimo_free_sized(ptr: ?*anyopaque, size: usize) void {
    _ = size;
    fimo_free(ptr);
}

///
/// Free allocated memory.
///
/// Deallocates the memory allocated by an allocation function. If `ptr` is a null pointer,
/// no action shall occur. Otherwise, if `ptr` does not match a pointer returned by the
/// allocation function, or if the space has been deallocated by a call to `fimo_free()`,
/// `fimo_free_sized()` or `fimo_free_aligned_sized()`, or if `alignment` and `size`
/// do not match the alignment and size used to allocate the memory, the behavior is undefined.
export fn fimo_free_aligned_sized(ptr: ?*anyopaque, alignment: usize, size: usize) void {
    _ = alignment;
    _ = size;
    fimo_free(ptr);
}

test "std Allocator tests" {
    try std.heap.testAllocatorAligned(fimo_allocator);
    try std.heap.testAllocatorAlignedShrink(fimo_allocator);
    try std.heap.testAllocatorLargeAlignment(fimo_allocator);
}

test "Allocate memory: zero size results in a null pointer" {
    var err: c.FimoResult = errors.Error.intoCResult(errors.Error.initErrorCode(errors.ErrorCode.ok));
    defer c.fimo_result_release(err);

    const buffer = fimo_malloc(0, &err);
    defer fimo_free(buffer);
    try std.testing.expect(buffer == null);
    try std.testing.expect(errors.Error.initC(err) == null);
}

test "Allocate memory: allocation is properly aligned" {
    var err: c.FimoResult = errors.Error.intoCResult(errors.Error.initErrorCode(errors.ErrorCode.ok));
    defer c.fimo_result_release(err);

    const buffer = fimo_malloc(@sizeOf(c_longlong), &err);
    defer fimo_free(buffer);
    try std.testing.expect(buffer != null);
    try std.testing.expect(errors.Error.initC(err) == null);
    try std.testing.expect(std.mem.isAligned(@intFromPtr(buffer), fimo_allocator_alignment));
}

test "Allocate memory: allocation is properly aligned and sized" {
    var err: c.FimoResult = errors.Error.intoCResult(errors.Error.initErrorCode(errors.ErrorCode.ok));
    defer c.fimo_result_release(err);

    const buffer = fimo_malloc_sized(1339, &err);
    defer fimo_free(buffer.ptr);
    try std.testing.expect(buffer.ptr != null);
    try std.testing.expect(errors.Error.initC(err) == null);
    try std.testing.expect(buffer.buff_size >= 1339);
    try std.testing.expect(
        std.mem.isAligned(@intFromPtr(buffer.ptr), fimo_allocator_alignment),
    );
}

test "Allocate zeroed memory: zero size results in a null pointer" {
    var err: c.FimoResult = errors.Error.intoCResult(errors.Error.initErrorCode(errors.ErrorCode.ok));
    defer c.fimo_result_release(err);

    const buffer = fimo_calloc(0, &err);
    defer fimo_free_sized(buffer, 0);
    try std.testing.expect(buffer == null);
    try std.testing.expect(errors.Error.initC(err) == null);
}

test "Allocate zeroed memory: allocation is properly aligned" {
    var err: c.FimoResult = errors.Error.intoCResult(errors.Error.initErrorCode(errors.ErrorCode.ok));
    defer c.fimo_result_release(err);

    const buffer: ?[*]c_longlong = @ptrCast(@alignCast(fimo_calloc(10 * @sizeOf(c_longlong), &err)));
    defer fimo_free_sized(buffer, 10 * @sizeOf(c_longlong));
    try std.testing.expect(buffer != null);
    try std.testing.expect(errors.Error.initC(err) == null);
    try std.testing.expect(std.mem.isAligned(@intFromPtr(buffer), fimo_allocator_alignment));
    for (buffer.?[0..10]) |e| {
        try std.testing.expect(e == 0);
    }
}

test "Allocate zeroed memory: allocation is properly aligned and sized" {
    var err: c.FimoResult = errors.Error.intoCResult(errors.Error.initErrorCode(errors.ErrorCode.ok));
    defer c.fimo_result_release(err);

    const buffer = fimo_calloc_sized(1339, &err);
    defer fimo_free_sized(buffer.ptr, buffer.buff_size);
    try std.testing.expect(buffer.ptr != null);
    try std.testing.expect(errors.Error.initC(err) == null);
    try std.testing.expect(buffer.buff_size >= 1339);
    try std.testing.expect(std.mem.isAligned(@intFromPtr(buffer.ptr), fimo_allocator_alignment));
    for (@as([*]u8, @ptrCast(buffer.ptr.?))[0..1339]) |e| {
        try std.testing.expect(e == 0);
    }
}

test "Allocate aligned memory: alignment must not be zero" {
    var err: c.FimoResult = errors.Error.intoCResult(errors.Error.initErrorCode(errors.ErrorCode.ok));
    defer c.fimo_result_release(err);

    const buffer = fimo_aligned_alloc(0, 10, &err);
    try std.testing.expect(buffer == null);
    try std.testing.expect(errors.Error.initC(err) != null);
}

test "Allocate aligned memory: alignment must be a power of two" {
    var err: c.FimoResult = errors.Error.intoCResult(errors.Error.initErrorCode(errors.ErrorCode.ok));
    defer c.fimo_result_release(err);

    const buffer = fimo_aligned_alloc(17, 10, &err);
    try std.testing.expect(buffer == null);
    try std.testing.expect(errors.Error.initC(err) != null);
}

test "Allocate aligned memory: zero size results in a null pointer" {
    var err: c.FimoResult = errors.Error.intoCResult(errors.Error.initErrorCode(errors.ErrorCode.ok));
    defer c.fimo_result_release(err);

    const buffer = fimo_aligned_alloc(256, 0, &err);
    defer fimo_free_aligned_sized(buffer, 256, 0);
    try std.testing.expect(buffer == null);
    try std.testing.expect(errors.Error.initC(err) == null);
}

test "Allocate aligned memory: allocation is properly aligned" {
    var err: c.FimoResult = errors.Error.intoCResult(errors.Error.initErrorCode(errors.ErrorCode.ok));
    defer c.fimo_result_release(err);

    const buffer = fimo_aligned_alloc(256, @sizeOf(c_longlong), &err);
    defer fimo_free_aligned_sized(buffer, 256, @sizeOf(c_longlong));
    try std.testing.expect(buffer != null);
    try std.testing.expect(errors.Error.initC(err) == null);
    try std.testing.expect(std.mem.isAligned(@intFromPtr(buffer), 256));
}

test "Allocate aligned memory: allocation is properly aligned and sized" {
    var err: c.FimoResult = errors.Error.intoCResult(errors.Error.initErrorCode(errors.ErrorCode.ok));
    defer c.fimo_result_release(err);

    const buffer = fimo_aligned_alloc_sized(256, 1339, &err);
    defer fimo_free_aligned_sized(buffer.ptr, 256, buffer.buff_size);
    try std.testing.expect(buffer.ptr != null);
    try std.testing.expect(errors.Error.initC(err) == null);
    try std.testing.expect(buffer.buff_size >= 1339);
    try std.testing.expect(std.mem.isAligned(@intFromPtr(buffer.ptr), 256));
}
