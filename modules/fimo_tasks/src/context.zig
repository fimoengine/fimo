const std = @import("std");
const Allocator = std.mem.Allocator;
const builtin = @import("builtin");

extern fn make_fcontext(
    stack_pointer: [*]u8,
    stack_size: usize,
    f: *const fn (t: Transfer) callconv(.c) noreturn,
) callconv(.c) *const anyopaque;

extern fn jump_fcontext(to: *const anyopaque, data: usize) callconv(.c) Transfer;

extern fn ontop_fcontext(
    to: *const anyopaque,
    data: usize,
    f: *const fn (t: Transfer) callconv(.c) Transfer,
) callconv(.c) Transfer;

/// State of execution.
pub const Context = extern struct {
    ptr: *const anyopaque,

    // Constructs a new context with the given stack and entry point.
    pub fn init(stack: []u8, enter: *const fn (t: Transfer) callconv(.c) noreturn) Context {
        const ptr = make_fcontext(stack.ptr + stack.len, stack.len, enter);
        return .{ .ptr = ptr };
    }

    /// Yields control to the context and passes `data` to it.
    pub fn yieldTo(self: Context, data: usize) Transfer {
        return jump_fcontext(self.ptr, data);
    }

    /// Yields control to the context and passes `data` to it.
    ///
    /// The function `on_top` is invoked after the context switch.
    pub fn yieldToOnTop(
        self: Context,
        data: usize,
        on_top: fn (t: Transfer) callconv(.c) Transfer,
    ) Transfer {
        return ontop_fcontext(self.ptr, data, on_top);
    }
};

test "yield to context" {
    const stack = try std.testing.allocator.alloc(u8, 1024 * 3);
    defer std.testing.allocator.free(stack);

    const f = struct {
        fn f(t: Transfer) callconv(.c) noreturn {
            var tr = t;
            for (0..1000) |i| {
                tr = tr.context.yieldTo(i);
            }
            unreachable;
        }
    }.f;

    var t = Transfer{ .context = Context.init(stack, &f) };
    for (0..10) |i| {
        t = t.context.yieldTo(0);
        try std.testing.expectEqual(i, t.data);
    }
}

/// Data passed between contexts during a context switch.
pub const Transfer = extern struct {
    context: Context,
    data: usize = 0,
};

pub const Stack = struct {
    memory: []align(std.heap.page_size_min) u8,

    pub fn init(size: usize) Allocator.Error!Stack {
        return StackAllocator.allocate(size);
    }

    pub fn deinit(self: Stack) void {
        StackAllocator.deallocate(self);
    }

    pub fn transitionCold(self: Stack) void {
        StackAllocator.transitionCold(self);
    }

    pub fn transitionHot(self: Stack) void {
        StackAllocator.transitionHot(self);
    }
};

pub const StackAllocator = struct {
    var page_size_: std.atomic.Value(usize) = if (std.heap.page_size_min == std.heap.page_size_max)
        .init(std.heap.page_size_min)
    else
        .init(0);
    var min_stack_size_: std.atomic.Value(usize) = switch (builtin.os.tag) {
        .windows => if (builtin.cpu.arch == .x86) .init(4 * 1024) else .init(8 * 1024),
        else => .init(0),
    };
    var max_stack_size_: std.atomic.Value(usize) = switch (builtin.os.tag) {
        .windows => .init(std.math.maxInt(usize)),
        else => .init(0),
    };

    fn pageSize() usize {
        var size = page_size_.load(.monotonic);
        if (size != 0) return size;
        if (builtin.os.tag == .windows) unreachable;
        size = std.heap.pageSize();
        page_size_.store(size, .monotonic);
        return size;
    }

    pub fn minStackSize() usize {
        var size = min_stack_size_.load(.monotonic);
        if (size != 0) return size;

        if (builtin.os.tag == .windows) unreachable;
        size = pageSize();
        min_stack_size_.store(size, .monotonic);
        return size;
    }

    pub fn maxStackSize() usize {
        var size = max_stack_size_.load(.monotonic);
        if (size != 0) return size;

        if (builtin.os.tag == .windows) unreachable;
        const stack_limit = std.posix.getrlimit(.STACK) catch unreachable;
        if (stack_limit.max == std.c.RLIM.INFINITY or stack_limit.max >= std.math.maxInt(usize)) {
            size = std.math.maxInt(usize);
            max_stack_size_.store(size, .monotonic);
            return size;
        }

        size = @intCast(stack_limit.max);
        max_stack_size_.store(size, .monotonic);
        return size;
    }

    /// Allocates a new stack.
    fn allocate(size: usize) Allocator.Error!Stack {
        const page_size = pageSize();
        const min_stack_size = minStackSize();
        const pages = (@max(size, min_stack_size) + page_size - 1) / page_size;
        const rounded_size = (pages + 1) * page_size;

        const max_stack_size = maxStackSize();
        if (rounded_size > max_stack_size) return Allocator.Error.OutOfMemory;

        if (builtin.os.tag == .windows) {
            const memory: [*]align(std.heap.page_size_min) u8 = @ptrCast(@alignCast(std.os.windows.VirtualAlloc(
                null,
                rounded_size,
                std.os.windows.MEM_COMMIT,
                std.os.windows.PAGE_READWRITE,
            ) catch return Allocator.Error.OutOfMemory));
            errdefer std.os.windows.VirtualFree(memory, 0, std.os.windows.MEM_RELEASE);

            // Protect last page.
            var old: std.os.windows.DWORD = undefined;
            std.os.windows.VirtualProtect(
                memory,
                page_size,
                std.os.windows.PAGE_READWRITE | std.os.windows.PAGE_GUARD,
                &old,
            ) catch return Allocator.Error.OutOfMemory;
            return Stack{ .memory = memory[0..rounded_size] };
        } else {
            const opt: std.posix.system.MAP = if (@hasField(std.posix.system.MAP, "STACK"))
                .{ .TYPE = .PRIVATE, .ANONYMOUS = true, .STACK = true }
            else
                .{ .TYPE = .PRIVATE, .ANONYMOUS = true };

            const memory = std.posix.mmap(
                null,
                rounded_size,
                std.posix.PROT.READ | std.posix.PROT.WRITE,
                opt,
                -1,
                0,
            ) catch return Allocator.Error.OutOfMemory;
            errdefer std.posix.munmap(memory);

            // Protect last page.
            std.posix.mprotect(
                memory[0..page_size],
                std.posix.PROT.NONE,
            ) catch return Allocator.Error.OutOfMemory;
            return Stack{ .memory = memory };
        }
    }

    /// Deallocates a stack.
    pub fn deallocate(stack: Stack) void {
        if (builtin.os.tag == .windows) {
            std.os.windows.VirtualFree(stack.memory.ptr, 0, std.os.windows.MEM_RELEASE);
        } else {
            std.posix.munmap(stack.memory);
        }
    }

    /// Marks the stack memory as cold.
    ///
    /// Depending on the operating system, this function may decommit the stack memory.
    /// Use `transitionHot` to make the memory region reusable.
    pub fn transitionCold(stack: Stack) void {
        if (builtin.os.tag == .windows) {
            std.os.windows.VirtualFree(stack.memory.ptr, 0, std.os.windows.MEM_DECOMMIT);
        } else {
            std.posix.madvise(
                stack.memory.ptr,
                stack.memory.len,
                std.posix.MADV.DONTNEED,
            ) catch unreachable;
        }
    }

    /// Reverses the effect of `transitionCold`.
    pub fn transitionHot(stack: Stack) void {
        if (builtin.os.tag == .windows) {
            _ = std.os.windows.VirtualAlloc(
                stack.memory.ptr,
                stack.memory.len,
                std.os.windows.MEM_COMMIT,
                std.os.windows.PAGE_READWRITE,
            ) catch unreachable;

            // Protect last page.
            var old: std.os.windows.DWORD = undefined;
            std.os.windows.VirtualProtect(
                stack.memory.ptr,
                pageSize(),
                std.os.windows.PAGE_READWRITE | std.os.windows.PAGE_GUARD,
                &old,
            ) catch unreachable;
        }
    }
};

test "allocate stack" {
    const stack = try Stack.init(1024 * 1024);
    defer stack.deinit();
    try std.testing.expect(stack.memory.len >= 1024 * 1024);
}

test "hot-cold" {
    const stack = try Stack.init(1024 * 1024);
    defer stack.deinit();

    stack.transitionCold();
    stack.transitionHot();

    std.debug.assertReadable(stack.memory[StackAllocator.pageSize()..]);
}
