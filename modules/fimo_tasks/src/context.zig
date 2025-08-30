const std = @import("std");
const Allocator = std.mem.Allocator;
const builtin = @import("builtin");

const Executor = @import("Executor.zig");

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

pub const Stack = struct {
    memory: []align(std.heap.page_size_min) u8,
    commited_size: if (builtin.os.tag == .windows) usize else void,

    pub fn updateFromContext(self: *Stack, context: Context) void {
        if (builtin.os.tag == .windows) {
            std.debug.assert(@intFromPtr(self.memory.ptr) == context.ptr.deallocation_stack);
            std.debug.assert(self.memory.len == context.ptr.stack_base - context.ptr.deallocation_stack);
            self.commited_size = context.ptr.stack_base - context.ptr.stack_limit;
        }
    }
};

pub const StackInfo = extern struct {
    start: [*]u8,
    reserved_size: usize,
    commited_size: usize,

    pub fn forSlice(memory: []u8) StackInfo {
        return .{
            .start = memory.ptr + memory.len,
            .reserved_size = memory.len,
            .commited_size = memory.len,
        };
    }

    pub fn forStack(stack: Stack) StackInfo {
        if (builtin.os.tag == .windows) {
            return .{
                .start = stack.memory.ptr + stack.memory.len,
                .reserved_size = stack.memory.len,
                .commited_size = stack.commited_size,
            };
        } else {
            return .forSlice(stack.memory);
        }
    }
};

/// State of execution.
pub const Context = extern struct {
    ptr: *Impl,

    // Constructs a new context with the given stack and entry point.
    pub fn init(info: StackInfo, enter: *const fn (t: Transfer) callconv(.c) noreturn) Context {
        const ptr = Impl.init(info, enter);
        return .{ .ptr = ptr };
    }

    /// Yields control to the context and passes `data` to it.
    pub fn yieldTo(self: Context, data: usize) Transfer {
        return self.ptr.yieldTo(data);
    }

    /// Yields control to the context and passes `data` to it.
    ///
    /// The function `on_top` is invoked after the context switch.
    pub fn yieldToOnTop(
        self: Context,
        data: usize,
        on_top: fn (t: Transfer) callconv(.c) Transfer,
    ) Transfer {
        self.ptr.yieldToOnTop(data, on_top);
    }
};

const Impl = switch (builtin.target.os.tag) {
    .windows => switch (builtin.target.cpu.arch) {
        .aarch64 => WindowsContextAarch64,
        .x86_64 => WindowsContextX86_64,
        else => @compileError("unsupported platform"),
    },
    else => BoostContext,
};

const BoostContext = opaque {
    extern fn make_fcontext(
        stack_pointer: [*]u8,
        stack_size: usize,
        f: *const fn (t: Transfer) callconv(.c) noreturn,
    ) callconv(.c) *BoostContext;

    extern fn jump_fcontext(to: *BoostContext, data: usize) callconv(.c) Transfer;

    extern fn ontop_fcontext(
        to: *BoostContext,
        data: usize,
        f: *const fn (t: Transfer) callconv(.c) Transfer,
    ) callconv(.c) Transfer;

    pub fn init(
        info: StackInfo,
        f: *const fn (t: Transfer) callconv(.c) noreturn,
    ) *BoostContext {
        return make_fcontext(info.start, info.reserved_size, f);
    }

    pub fn yieldTo(self: *BoostContext, data: usize) Transfer {
        return jump_fcontext(self, data);
    }

    pub fn yieldToOnTop(
        self: *BoostContext,
        data: usize,
        on_top: fn (t: Transfer) callconv(.c) Transfer,
    ) Transfer {
        return ontop_fcontext(self, data, on_top);
    }
};

// Derived from the Boost Context library:
// Copyright Edward Nevill + Oliver Kowalke 2015
// Distributed under the Boost Software License, Version 1.0.
//
// Boost Software License - Version 1.0 - August 17th, 2003
//
// Permission is hereby granted, free of charge, to any person or organization
// obtaining a copy of the software and accompanying documentation covered by
// this license (the "Software") to use, reproduce, display, distribute,
// execute, and transmit the Software, and to prepare derivative works of the
// Software, and to permit third-parties to whom the Software is furnished to
// do so, all subject to the following:
//
// The copyright notices in the Software and this entire statement, including
// the above license grant, this restriction and the following disclaimer,
// must be included in all copies of the Software, in whole or in part, and
// all derivative works of the Software, unless such copies or derivative
// works are solely in the form of machine-executable object code generated by
// a source language processor.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE, TITLE AND NON-INFRINGEMENT. IN NO EVENT
// SHALL THE COPYRIGHT HOLDERS OR ANYONE DISTRIBUTING THE SOFTWARE BE LIABLE
// FOR ANY DAMAGES OR OTHER LIABILITY, WHETHER IN CONTRACT, TORT OR OTHERWISE,
// ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER
// DEALINGS IN THE SOFTWARE.
const WindowsContextAarch64 = extern struct {
    d8: u64,
    d9: u64,
    d10: u64,
    d11: u64,
    d12: u64,
    d13: u64,
    d14: u64,
    d15: u64,
    x19: u64,
    x20: u64,
    x21: u64,
    x22: u64,
    x23: u64,
    x24: u64,
    x25: u64,
    x26: u64,
    x27: u64,
    x28: u64,
    fp: u64,
    lr: u64,
    stack_base: u64,
    stack_limit: u64,
    deallocation_stack: u64,
    fiber_data: u64,
    pc: u64,
    alignment: u64 = undefined,

    extern fn jump_fcontext(to: *WindowsContextAarch64, data: usize) callconv(.c) Transfer;

    extern fn ontop_fcontext(
        to: *WindowsContextAarch64,
        data: usize,
        f: *const fn (t: Transfer) callconv(.c) Transfer,
    ) callconv(.c) Transfer;

    pub fn init(
        info: StackInfo,
        f: *const fn (t: Transfer) callconv(.c) noreturn,
    ) callconv(.c) *WindowsContextAarch64 {
        @setRuntimeSafety(false);
        const context_end = std.mem.alignBackward(usize, @intFromPtr(info.start), 16);
        const context: *WindowsContextAarch64 = @ptrFromInt(context_end - @sizeOf(WindowsContextAarch64));
        context.stack_base = @intFromPtr(info.start);
        context.stack_limit = @intFromPtr(info.start - info.commited_size);
        context.deallocation_stack = @intFromPtr(info.start - info.reserved_size);
        context.fiber_storage = 0;

        context.x19 = @intFromPtr(f);
        context.pc = @intFromPtr(&trampoline);
        context.lr = @intFromPtr(&finish);

        return context;
    }

    pub fn yieldTo(self: *WindowsContextAarch64, data: usize) Transfer {
        return jump_fcontext(self, data);
    }

    pub fn yieldToOnTop(
        self: *WindowsContextAarch64,
        data: usize,
        on_top: fn (t: Transfer) callconv(.c) Transfer,
    ) Transfer {
        return ontop_fcontext(self, data, on_top);
    }

    fn trampoline() callconv(.naked) noreturn {
        @setRuntimeSafety(false);
        asm volatile (
            \\stp  fp, lr, [sp, #-0x10]!
            \\mov  fp, sp
            \\blr x19
        );
    }

    fn finish() callconv(.naked) noreturn {
        @setRuntimeSafety(false);
        asm volatile (
            \\// exit code is zero
            \\mov  x0, #0
            \\// exit application
            \\bl  _exit
        );
    }
};

// Derived from the Boost Context library:
// Copyright Oliver Kowalke 2009.
// Copyright Thomas Sailer 2013.
// Distributed under the Boost Software License, Version 1.0.
//
// Boost Software License - Version 1.0 - August 17th, 2003
//
// Permission is hereby granted, free of charge, to any person or organization
// obtaining a copy of the software and accompanying documentation covered by
// this license (the "Software") to use, reproduce, display, distribute,
// execute, and transmit the Software, and to prepare derivative works of the
// Software, and to permit third-parties to whom the Software is furnished to
// do so, all subject to the following:
//
// The copyright notices in the Software and this entire statement, including
// the above license grant, this restriction and the following disclaimer,
// must be included in all copies of the Software, in whole or in part, and
// all derivative works of the Software, unless such copies or derivative
// works are solely in the form of machine-executable object code generated by
// a source language processor.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE, TITLE AND NON-INFRINGEMENT. IN NO EVENT
// SHALL THE COPYRIGHT HOLDERS OR ANYONE DISTRIBUTING THE SOFTWARE BE LIABLE
// FOR ANY DAMAGES OR OTHER LIABILITY, WHETHER IN CONTRACT, TORT OR OTHERWISE,
// ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER
// DEALINGS IN THE SOFTWARE.
const WindowsContextX86_64 = extern struct {
    xmm6: u128,
    xmm7: u128,
    xmm8: u128,
    xmm9: u128,
    xmm10: u128,
    xmm11: u128,
    xmm12: u128,
    xmm13: u128,
    xmm14: u128,
    xmm15: u128,
    fc_mxcsr: u32,
    fc_x87_cw: u32,
    alignment: u64 = undefined,
    fiber_storage: u64,
    deallocation_stack: u64,
    stack_limit: u64,
    stack_base: u64,
    r12: u64,
    r13: u64,
    r14: u64,
    r15: u64,
    rdi: u64,
    rsi: u64,
    rbx: u64,
    rbp: u64,
    hidden: u64,
    rip: u64,
    parameters: [4]u64,
    transport: u64,
    data: u64,

    extern fn jump_fcontext(to: *WindowsContextX86_64, data: usize) callconv(.c) Transfer;

    extern fn ontop_fcontext(
        to: *WindowsContextX86_64,
        data: usize,
        f: *const fn (t: Transfer) callconv(.c) Transfer,
    ) callconv(.c) Transfer;

    pub fn init(
        info: StackInfo,
        f: *const fn (t: Transfer) callconv(.c) noreturn,
    ) callconv(.c) *WindowsContextX86_64 {
        @setRuntimeSafety(false);
        const context_end = std.mem.alignBackward(usize, @intFromPtr(info.start), 16);
        const context: *WindowsContextX86_64 = @ptrFromInt(context_end - @sizeOf(WindowsContextX86_64));
        context.rbx = @intFromPtr(f);
        context.stack_base = @intFromPtr(info.start);
        context.stack_limit = @intFromPtr(info.start - info.commited_size);
        context.deallocation_stack = @intFromPtr(info.start - info.reserved_size);
        context.fiber_storage = 0;

        asm volatile (
            \\/* save MMX control- and status-word */
            \\stmxcsr  0xa0(%rax)
            \\/* save x87 control-word */
            \\fnstcw   0xa4(%rax)
            :
            : [context] "{rax}" (context),
        );

        context.hidden = @intFromPtr(&context.transport);
        context.rip = @intFromPtr(&trampoline);
        context.rbp = @intFromPtr(&finish);

        return context;
    }

    pub fn yieldTo(self: *WindowsContextX86_64, data: usize) Transfer {
        return jump_fcontext(self, data);
    }

    pub fn yieldToOnTop(
        self: *WindowsContextX86_64,
        data: usize,
        on_top: fn (t: Transfer) callconv(.c) Transfer,
    ) Transfer {
        return ontop_fcontext(self, data, on_top);
    }

    fn trampoline() callconv(.naked) noreturn {
        @setRuntimeSafety(false);
        asm volatile (
            \\#store return address on stack */
            \\/* fix stack alignment */
            \\pushq %rbp
            \\/* jump to context-function */
            \\jmp *%rbx
        );
    }

    fn finish() callconv(.naked) noreturn {
        @setRuntimeSafety(false);
        asm volatile (
            \\/* 32byte shadow-space for _exit() */
            \\andq  $-32, %rsp
            \\/* 32byte shadow-space for _exit() are
            \\/* already reserved by make_fcontext() */
            \\/* exit code is zero */
            \\xorq  %rcx, %rcx
            \\/* exit application */
            \\call  _exit
            \\hlt
        );
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

    var t = Transfer{ .context = Context.init(.forSlice(stack), &f) };
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
