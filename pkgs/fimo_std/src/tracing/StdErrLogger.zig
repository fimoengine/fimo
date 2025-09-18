const std = @import("std");

const tracing = @import("../tracing.zig");
const Level = tracing.Level;
const EventInfo = tracing.EventInfo;
const Subscriber = tracing.Subscriber;
const events = tracing.events;

ring_buffer: std.atomic.Value(?*RingBuffer) = .init(null),
print_buffer_length: usize,
gpa: std.mem.Allocator,
max_level: Level,
worker: std.Thread,

pub const fimo_subscriber = .{
    .create_call_stack = onCreateCallStack,
    .destroy_call_stack = onDestroyCallStack,
    .enter_span = onEnterSpan,
    .exit_span = onExitSpan,
    .log_message = onLogMessage,
};

const ansi_color_red: []const u8 = "\x1b[31m";
const ansi_color_green: []const u8 = "\x1b[32m";
const ansi_color_yellow: []const u8 = "\x1b[33m";
const ansi_color_blue: []const u8 = "\x1b[34m";
const ansi_color_magenta: []const u8 = "\x1b[35m";
const ansi_color_reset: []const u8 = "\x1b[0m";

const ansi_sgr_italic: []const u8 = "\x1b[3m";
const ansi_sgr_reset: []const u8 = "\x1b[0m";

const error_fmt: []const u8 = ansi_color_red ++ "ERROR {s}: {s}" ++ ansi_color_reset ++ "\n";
const warn_fmt: []const u8 = ansi_color_yellow ++ "WARN {s}: {s}" ++ ansi_color_reset ++ "\n";
const info_fmt: []const u8 = ansi_color_green ++ "INFO {s}: {s}" ++ ansi_color_reset ++ "\n";
const debug_fmt: []const u8 = ansi_color_blue ++ "DEBUG {s}: {s}" ++ ansi_color_reset ++ "\n";
const trace_fmt: []const u8 = ansi_color_magenta ++ "TRACE {s}: {s}" ++ ansi_color_reset ++ "\n";

const file_path_fmt: []const u8 = "\t" ++ ansi_sgr_italic ++ "at" ++ ansi_sgr_reset ++ " {s}:{d}\n";
const unknown_file_path_fmt: []const u8 = "\t" ++ ansi_sgr_italic ++ "at" ++ ansi_sgr_reset ++ " unknown\n";
const backtrace_fmt: []const u8 = "\t" ++ ansi_sgr_italic ++ "in" ++ ansi_sgr_reset ++ " {s}" ++ ansi_sgr_italic ++ " with" ++ ansi_sgr_reset ++ " {s}\n";
const overlength_correction: []const u8 = "\t..." ++ ansi_color_reset ++ "\n";

const Frame = struct {
    id: *const EventInfo,
    message: []u8,
    node: std.DoublyLinkedList.Node = .{},

    fn init(id: *const EventInfo, message: []const u8, gpa: std.mem.Allocator) *Frame {
        const byte_count = @sizeOf(Frame) + message.len;
        const buffer = gpa.alignedAlloc(u8, .of(Frame), byte_count) catch @panic("oom");
        const frame = std.mem.bytesAsValue(Frame, buffer[0..@sizeOf(Frame)]);
        frame.* = .{
            .id = id,
            .message = buffer[@sizeOf(Frame)..],
        };
        @memcpy(frame.message, message);
        return frame;
    }

    fn deinit(self: *Frame, gpa: std.mem.Allocator) void {
        const byte_count = @sizeOf(Frame) + self.message.len;
        const buffer = std.mem.asBytes(self).ptr[0..byte_count];
        gpa.free(buffer);
    }
};

const Stack = struct {
    lock: std.Thread.Mutex = .{},
    arena: std.heap.ArenaAllocator,
    spans: std.DoublyLinkedList = .{},
};

const Msg = union(enum) {
    quit,
    create_call_stack: struct { stack: *anyopaque },
    destroy_call_stack: struct { stack: *anyopaque },
    enter_span: struct {
        stack: *anyopaque,
        info: *const EventInfo,
        msg_len: u16,
    },
    exit_span: struct { stack: *anyopaque },
    log_message: struct {
        stack: *anyopaque,
        info: *const EventInfo,
        msg_len: u16,
    },
};

const RingBuffer = struct {
    const buffer_len = 64 * 1024;
    const max_msg_len = buffer_len - 1;

    mutex: std.Thread.Mutex = .{},
    read_condition: std.Thread.Condition = .{},
    write_condition: std.Thread.Condition = .{},
    buffer: [buffer_len]u8 = undefined,
    read_idx: usize = 0,
    write_idx: usize = 0,

    fn used(self: *RingBuffer) usize {
        return (buffer_len - self.read_idx + self.write_idx) % buffer_len;
    }

    fn free(self: *RingBuffer) usize {
        return (buffer_len - 1 - self.write_idx + self.read_idx) % buffer_len;
    }

    fn write(self: *RingBuffer, msg: []const u8) void {
        if (msg.len == 0) return;
        const msg_len = @min(msg.len, max_msg_len);

        self.mutex.lock();
        defer self.mutex.unlock();
        while (self.free() < msg_len) {
            self.read_condition.wait(&self.mutex);
        }

        const slice_0_len = @min(buffer_len - self.write_idx, msg_len);
        const slice_1_len = msg_len - slice_0_len;
        const slice_0 = self.buffer[self.write_idx..][0..slice_0_len];
        @memcpy(slice_0, msg[0..slice_0_len]);
        const slice_1 = self.buffer[0..slice_1_len];
        @memcpy(slice_1, msg[slice_0_len..msg_len]);

        self.write_idx = (self.write_idx + msg_len) % buffer_len;
        self.write_condition.signal();
    }

    fn read(self: *RingBuffer, buffer: []u8) void {
        self.mutex.lock();
        defer self.mutex.unlock();
        while (self.used() < buffer.len) {
            self.write_condition.wait(&self.mutex);
        }

        const slice_0_len = @min(buffer_len - self.read_idx, buffer.len);
        const slice_1_len = buffer.len - slice_0_len;
        const slice_0 = self.buffer[self.read_idx..][0..slice_0_len];
        @memcpy(buffer[0..slice_0_len], slice_0);
        const slice_1 = self.buffer[0..slice_1_len];
        @memcpy(buffer[slice_0_len..], slice_1);

        self.read_idx = (self.read_idx + buffer.len) % buffer_len;
        self.read_condition.broadcast();
    }
};

const Self = @This();

pub const Options = struct {
    gpa: std.mem.Allocator,
    max_level: Level = .trace,
    print_buffer_length: usize = 1024,
};

pub fn init(self: *Self, options: Options) !void {
    self.* = .{
        .gpa = options.gpa,
        .max_level = options.max_level,
        .print_buffer_length = options.print_buffer_length,
        .worker = undefined,
    };
    self.worker = try .spawn(.{}, runWorker, .{self});
}

pub fn deinit(self: *Self) void {
    self.pushMessage(.quit, &.{});
    self.worker.join();
    self.* = undefined;
}

pub fn subscriber(self: *Self) Subscriber {
    return .of(Self, self);
}

fn onCreateCallStack(self: *Self, event: *const events.CreateCallStack) void {
    self.pushMessage(.{ .create_call_stack = .{ .stack = event.stack } }, &.{});
}

fn onDestroyCallStack(self: *Self, event: *const events.DestroyCallStack) void {
    self.pushMessage(.{ .destroy_call_stack = .{ .stack = event.stack } }, &.{});
}

fn onEnterSpan(self: *Self, event: *const events.EnterSpan) void {
    self.pushMessage(
        .{
            .enter_span = .{
                .stack = event.stack,
                .info = event.span,
                .msg_len = @intCast(@min(event.message.len, RingBuffer.max_msg_len - @sizeOf(Msg))),
            },
        },
        event.message.intoSliceOrEmpty(),
    );
}

fn onExitSpan(self: *Self, event: *const events.ExitSpan) void {
    self.pushMessage(.{ .exit_span = .{ .stack = event.stack } }, &.{});
}

fn onLogMessage(self: *Self, event: *const events.LogMessage) void {
    self.pushMessage(
        .{
            .log_message = .{
                .stack = event.stack,
                .info = event.info,
                .msg_len = @intCast(@min(event.message.len, RingBuffer.max_msg_len - @sizeOf(Msg))),
            },
        },
        event.message.intoSliceOrEmpty(),
    );
}

fn pushMessage(self: *Self, msg: Msg, extra: []const u8) void {
    while (self.ring_buffer.load(.monotonic) == null) {
        std.Thread.yield() catch {};
    }
    const ring_buffer = self.ring_buffer.load(.acquire).?;

    const locals = struct {
        threadlocal var scratch: [RingBuffer.max_msg_len]u8 = undefined;
    };
    const scratch = &locals.scratch;
    @memcpy(scratch[0..@sizeOf(Msg)], @as([]const u8, @ptrCast(&msg)));
    @memcpy(scratch[@sizeOf(Msg)..][0..extra.len], extra);
    ring_buffer.write(scratch[0 .. @sizeOf(Msg) + extra.len]);
}

fn runWorker(self: *Self) void {
    const print_buffer = self.gpa.alloc(
        u8,
        self.print_buffer_length + overlength_correction.len,
    ) catch @panic("oom");
    defer self.gpa.free(print_buffer);

    var ring_buffer: RingBuffer = .{};
    self.ring_buffer.store(&ring_buffer, .release);

    var stacks: std.AutoArrayHashMapUnmanaged(*anyopaque, *Stack) = .empty;
    defer stacks.deinit(self.gpa);
    defer std.debug.assert(stacks.count() == 0);

    const config = std.Io.tty.Config.detect(std.fs.File.stderr());
    const use_escape_codes = config == .escape_codes;
    _ = use_escape_codes;

    var msg_buffer: [RingBuffer.max_msg_len]u8 = undefined;
    while (true) {
        var msg: Msg = undefined;
        ring_buffer.read(@ptrCast(&msg));
        switch (msg) {
            .quit => {
                std.debug.print("quit\n", .{});
                return;
            },
            .create_call_stack => |event| {
                const stack = self.gpa.create(Stack) catch @panic("oom");
                stack.* = .{ .arena = .init(self.gpa) };
                stacks.put(self.gpa, event.stack, stack) catch @panic("oom");
            },
            .destroy_call_stack => |event| {
                const stack = stacks.fetchSwapRemove(event.stack).?.value;
                std.debug.assert(stack.spans.first == null);
                stack.arena.deinit();
                self.gpa.destroy(stack);
            },
            .enter_span => |event| {
                const event_msg = msg_buffer[0..event.msg_len];
                ring_buffer.read(event_msg);

                const stack = stacks.get(event.stack).?;
                const frame = Frame.init(event.info, event_msg, stack.arena.allocator());
                stack.spans.append(&frame.node);
            },
            .exit_span => |event| {
                const stack = stacks.get(event.stack).?;
                const node = stack.spans.pop() orelse unreachable;
                const frame: *Frame = @fieldParentPtr("node", node);
                frame.deinit(stack.arena.allocator());
            },
            .log_message => |event| {
                const event_msg = msg_buffer[0..event.msg_len];
                ring_buffer.read(event_msg);

                const stack = stacks.get(event.stack).?;
                self.emitLogEC(print_buffer, stack, event.info, event_msg);
            },
        }
    }
}

fn emitLogEC(
    self: *Self,
    print_buffer: []u8,
    stack: *Stack,
    info: *const EventInfo,
    message: []const u8,
) void {
    if (@intFromEnum(self.max_level) < @intFromEnum(info.level)) return;
    const format = struct {
        fn f(buffer: []u8, print_buffer_len: usize, cursor: usize, comptime fmt: []const u8, args: anytype) usize {
            const buf = std.fmt.bufPrint(
                buffer[cursor..print_buffer_len],
                fmt,
                args,
            ) catch return 0;
            return buf.len;
        }
    }.f;

    // Write the event message.
    var cursor: usize = 0;
    cursor += switch (info.level) {
        .off => 0,
        .err => format(print_buffer, self.print_buffer_length, cursor, error_fmt, .{ info.name, message }),
        .warn => format(print_buffer, self.print_buffer_length, cursor, warn_fmt, .{ info.name, message }),
        .info => format(print_buffer, self.print_buffer_length, cursor, info_fmt, .{ info.name, message }),
        .debug => format(print_buffer, self.print_buffer_length, cursor, debug_fmt, .{ info.name, message }),
        .trace => format(print_buffer, self.print_buffer_length, cursor, trace_fmt, .{ info.name, message }),
    };

    // Write out the file location.
    if (info.file_name) |file_name| {
        cursor += format(
            print_buffer,
            self.print_buffer_length,
            cursor,
            file_path_fmt,
            .{ file_name, info.line_number },
        );
    } else {
        cursor += format(print_buffer, self.print_buffer_length, cursor, unknown_file_path_fmt, .{});
    }

    // Write out the call stack.
    {
        var curr = stack.spans.last;
        while (curr) |node| : (curr = node.prev) {
            const frame: *Frame = @fieldParentPtr("node", node);
            cursor += format(
                print_buffer,
                self.print_buffer_length,
                cursor,
                backtrace_fmt,
                .{ frame.id.name, frame.message },
            );
        }
    }

    // Correct overlong messages.
    if (cursor >= self.print_buffer_length) {
        // Check if we have an incomplete ANSI escape sequence.
        // Our longest escape sequence consists of 5 bytes.
        for (0..5) |i| {
            if (print_buffer[cursor - i - 1] == 'm') break;
            if (print_buffer[cursor - i - 1] == '\x1b') {
                cursor = cursor - i - 1;
                break;
            }
        }

        const rest_buffer = print_buffer[cursor..];
        const correction_start: usize = if (print_buffer[cursor - 1] == 'n') 0 else 1;
        std.mem.copyForwards(u8, rest_buffer, overlength_correction[correction_start..]);
    }

    const stderr = std.debug.lockStderrWriter(&.{});
    defer std.debug.unlockStderrWriter();
    stderr.writeAll(print_buffer[0..cursor]) catch {};
}

export fn fstd_stderr_logger_init() Subscriber {
    const allocator = std.heap.c_allocator;
    const logger = allocator.create(Self) catch @panic("oom");
    logger.init(.{ .gpa = allocator }) catch |err| @panic(@errorName(err));
    return logger.subscriber();
}

export fn fstd_stderr_logger_deinit(sub: Subscriber) void {
    const logger: *Self = @ptrCast(@alignCast(sub.data));
    logger.deinit();
}
