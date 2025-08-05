const std = @import("std");

const tracing = @import("../tracing.zig");
const Level = tracing.Level;
const EventInfo = tracing.EventInfo;
const Subscriber = tracing.Subscriber;
const events = tracing.events;

mutex: std.Thread.Mutex = .{},
condition: std.Thread.Condition = .{},
queue: std.DoublyLinkedList = .{},
free_list: std.DoublyLinkedList = .{},
print_buffer_length: usize,
gpa: std.mem.Allocator,
max_level: Level,
worker: std.Thread,
quit: bool = false,

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
    arena: std.heap.ArenaAllocator,
    safe_allocator: std.heap.ThreadSafeAllocator,
    spans: std.DoublyLinkedList = .{},
};

const Block = struct {
    messages: [block_size]Message = undefined,
    count: usize = 0,
    read_idx: usize = 0,
    write_idx: usize = 0,
    node: std.DoublyLinkedList.Node = .{},

    const block_size = 32;
    comptime {
        if (!std.math.isPowerOfTwo(block_size)) @compileError("block_size must be a power of two");
    }

    fn init(msg: Message, gpa: std.mem.Allocator) *Block {
        const block = gpa.create(Block) catch @panic("oom");
        block.reset(msg);
        return block;
    }

    fn deinit(self: *Block, gpa: std.mem.Allocator) void {
        std.debug.assert(self.count == 0);
        gpa.destroy(self);
    }

    fn reset(self: *Block, msg: Message) void {
        self.* = .{};
        self.messages[0] = msg;
        self.count = 1;
        self.write_idx = 1;
    }

    fn tryRead(self: *Block) ?Message {
        if (self.count == 0) return null;
        const msg = self.messages[self.read_idx];
        self.messages[self.read_idx] = undefined;
        self.count -= 1;
        self.read_idx = (self.read_idx + 1) & (block_size - 1);
        return msg;
    }

    fn tryWrite(self: *Block, msg: Message) bool {
        if (self.count == block_size) return false;
        self.messages[self.write_idx] = msg;
        self.count += 1;
        self.write_idx = (self.write_idx + 1) & (block_size - 1);
        return true;
    }
};

const Message = union(enum) {
    destroy_stack: *Stack,
    append_frame: struct {
        stack: *Stack,
        frame: *Frame,
    },
    destroy_frame: struct {
        stack: *Stack,
        id: *const EventInfo,
    },
    log: struct {
        stack: *Stack,
        info: *const EventInfo,
        message: []u8,
    },
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
        .worker = try .spawn(.{}, runWorker, .{self}),
    };
}

pub fn deinit(self: *Self) void {
    self.mutex.lock();
    self.quit = true;
    self.condition.signal();
    self.mutex.unlock();
    self.worker.join();

    while (self.free_list.pop()) |node| {
        const block: *Block = @fieldParentPtr("node", node);
        block.deinit(self.gpa);
    }
    self.* = undefined;
}

pub fn subscriber(self: *Self) Subscriber {
    return .of(Self, self);
}

fn onCreateCallStack(self: *Self, event: *const events.CreateCallStack) *Stack {
    _ = event;
    const stack = self.gpa.create(Stack) catch @panic("oom");
    stack.* = .{ .arena = .init(self.gpa), .safe_allocator = undefined };
    stack.safe_allocator = .{ .child_allocator = stack.arena.allocator() };
    return stack;
}

fn onDestroyCallStack(self: *Self, event: *const events.DestroyCallStack) void {
    const stack: *Stack = @ptrCast(@alignCast(event.stack));
    self.pushMessage(.{ .destroy_stack = stack });
}

fn onEnterSpan(self: *Self, event: *const events.EnterSpan) void {
    const message = event.message[0..event.message_length];
    const stack: *Stack = @ptrCast(@alignCast(event.stack));
    const frame = Frame.init(event.span, message, stack.safe_allocator.allocator());
    self.pushMessage(.{ .append_frame = .{ .stack = stack, .frame = frame } });
}

fn onExitSpan(self: *Self, event: *const events.ExitSpan) void {
    const stack: *Stack = @ptrCast(@alignCast(event.stack));
    const id = event.span;
    self.pushMessage(.{ .destroy_frame = .{ .stack = stack, .id = id } });
}

fn onLogMessage(self: *Self, event: *const events.LogMessage) void {
    const stack: *Stack = @ptrCast(@alignCast(event.stack));
    const info = event.info;
    const message = event.message[0..event.message_length];
    const dupe = stack.safe_allocator.allocator().dupe(u8, message) catch @panic("oom");
    self.pushMessage(.{ .log = .{ .stack = stack, .info = info, .message = dupe } });
}

fn takeOrAllocEmptyBlock(self: *Self, msg: Message) *Block {
    const node = self.free_list.popFirst() orelse {
        return Block.init(msg, self.gpa);
    };
    const block: *Block = @fieldParentPtr("node", node);
    block.reset(msg);
    return block;
}

fn pushMessage(self: *Self, msg: Message) void {
    self.mutex.lock();
    defer self.mutex.unlock();
    const tail = self.queue.last orelse {
        const block = self.takeOrAllocEmptyBlock(msg);
        self.queue.append(&block.node);
        self.condition.signal();
        return;
    };

    const block: *Block = @fieldParentPtr("node", tail);
    if (!block.tryWrite(msg)) {
        const new_block = self.takeOrAllocEmptyBlock(msg);
        self.queue.append(&new_block.node);
    }
    self.condition.signal();
}

fn waitOnMessage(self: *Self) ?Message {
    self.mutex.lock();
    defer self.mutex.unlock();
    while (true) {
        const head = self.queue.first orelse {
            if (self.quit) return null;
            self.condition.wait(&self.mutex);
            continue;
        };

        const block: *Block = @fieldParentPtr("node", head);
        const msg = block.tryRead() orelse unreachable;
        if (block.count == 0) {
            _ = self.queue.popFirst();
            self.free_list.prepend(&block.node);
        }

        return msg;
    }
}

fn runWorker(self: *Self) void {
    const print_buffer = self.gpa.alloc(
        u8,
        self.print_buffer_length + overlength_correction.len,
    ) catch @panic("oom");
    defer self.gpa.free(print_buffer);

    const config = std.Io.tty.Config.detect(std.fs.File.stderr());
    const use_escape_codes = config == .escape_codes;
    _ = use_escape_codes;
    while (self.waitOnMessage()) |msg| switch (msg) {
        .destroy_stack => |stack| {
            std.debug.assert(stack.spans.first == null);
            stack.arena.deinit();
            self.gpa.destroy(stack);
        },
        .append_frame => |m| {
            const stack, const frame = .{ m.stack, m.frame };
            stack.spans.append(&frame.node);
        },
        .destroy_frame => |m| {
            const stack, const id = .{ m.stack, m.id };
            const node = stack.spans.pop() orelse unreachable;
            const frame: *Frame = @fieldParentPtr("node", node);
            std.debug.assert(frame.id == id);
            frame.deinit(stack.safe_allocator.allocator());
        },
        .log => |m| {
            const stack, const info, const message = .{ m.stack, m.info, m.message };
            self.emitLogEC(print_buffer, stack, info, message);
            stack.safe_allocator.allocator().free(message);
        },
    };
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

export fn fimo_tracing_stderr_logger_new() Subscriber {
    const allocator = std.heap.c_allocator;
    const logger = allocator.create(Self) catch @panic("oom");
    logger.init(.{ .gpa = allocator }) catch |err| @panic(@errorName(err));
    return logger.subscriber();
}

export fn fimo_tracing_stderr_logger_destroy(sub: Subscriber) void {
    const logger: *Self = @ptrCast(@alignCast(sub.data));
    logger.deinit();
}
