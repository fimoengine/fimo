const std = @import("std");
const Io = std.Io;
const File = std.fs.File;
const Writer = File.Writer;
const builtin = @import("builtin");

const paths = @import("../path.zig");
const OsPath = paths.OsPath;
const time = @import("../time.zig");
const Instant = time.Instant;
const Time = time.Time;
const Duration = time.Duration;
const tracing = @import("../tracing.zig");

const max_alignment = @alignOf(u64);
pub const min_block_len: u16 = 1 << 12;

pub const FileHeader = extern struct {
    pub const expect_magic = "Fimo Trace Stream";
    pub const expect_version_major = 1;
    pub const expect_version_minor = 0;

    file_magic: [expect_magic.len]u8 = expect_magic.*,
    version_major: u8 = expect_version_major,
    version_minor: u8 = expect_version_minor,
    block_compression: BlockCompression,
    block_byte_len: u32,
    num_sessions: u16,
    sessions_offset: u16,
};

pub const BlockCompression = enum(u8) {
    none,
    _,
};

pub const Session = extern struct {
    byte_len: u64,
    start_time: u64,
    end_time: u64,
    epoch: u64,
    resolution: u16,
    cpu_arch: tracing.events.CpuArch,
    cpu_vendor_len: u8,
    app_name_len: u16,
    host_info_len: u16,
    available_memory: u64,
    process_id: u64,
    cpu_id: u32,
    num_cores: u16,
    block_offset: u16,
    num_blocks: u32,

    pub fn getStartTime(self: *const Session) Instant {
        const offset: Duration = .initNanos(self.start_time);
        return (Instant{}).add(offset) catch unreachable;
    }

    pub fn getEndTime(self: *const Session) Instant {
        const offset: Duration = .initNanos(self.end_time);
        return (Instant{}).add(offset) catch unreachable;
    }

    pub fn getEpoch(self: *const Session) Time {
        const offset: Duration = .initNanos(self.epoch);
        return (Time.UnixEpoch).add(offset) catch unreachable;
    }

    pub fn getResolution(self: *const Session) Duration {
        return .initNanos(self.resolution);
    }

    pub fn getCpuVendor(self: *const Session) []const u8 {
        const ptr: [*]const u8 = @ptrCast(self);
        const start = @sizeOf(Session);
        const end = start + self.cpu_vendor_len;
        return ptr[start..end];
    }

    pub fn getAppName(self: *const Session) []const u8 {
        const ptr: [*]const u8 = @ptrCast(self);
        const start = @sizeOf(Session) + self.cpu_vendor_len;
        const end = start + self.app_name_len;
        return ptr[start..end];
    }

    pub fn getHostInfo(self: *const Session) []const u8 {
        const ptr: [*]const u8 = @ptrCast(self);
        const start = @sizeOf(Session) + self.cpu_vendor_len + self.app_name_len;
        const end = start + self.host_info_len;
        return ptr[start..end];
    }

    pub fn getBlock(self: *const Session, idx: usize) ?*const BlockHeader {
        if (idx >= self.num_blocks) return null;
        var block: *const BlockHeader = @ptrFromInt(@intFromPtr(self) + self.block_offset);
        std.debug.assert(std.mem.isAligned(@intFromPtr(block), max_alignment));
        for (0..idx) |_| {
            const offset = @as(u64, block.data_len) + @as(u64, block.data_offset);
            const next: *const BlockHeader = @ptrFromInt(@intFromPtr(block) + offset);
            std.debug.assert(std.mem.isAligned(@intFromPtr(next), max_alignment));
            block = next;
        }
        return block;
    }
};

pub const BlockHeader = extern struct {
    start_time: u64,
    time_range: u32,
    num_events: u32,
    data_len: u32,
    data_offset: u16,
    reserved: u16 = undefined,

    pub const EventIterator = struct {
        remaining: u32,
        curr: *const EventHeader,

        pub fn next(self: *EventIterator) ?*const EventHeader {
            if (self.remaining == 0) return null;
            const event = self.curr;
            self.remaining -= 1;
            self.curr = @ptrFromInt(@intFromPtr(event) + event.byte_len);
            return event;
        }
    };

    pub fn getStartTime(self: *const BlockHeader) Instant {
        const offset: Duration = .initNanos(self.start_time);
        return (Instant{}).add(offset) catch unreachable;
    }

    pub fn getEndTime(self: *const BlockHeader) Instant {
        return self.getStartTime().add(self.getTimeRange()) catch unreachable;
    }

    pub fn getTimeRange(self: *const BlockHeader) Duration {
        return .initNanos(self.time_range);
    }

    pub fn getData(self: *const BlockHeader) []const u8 {
        std.debug.assert(self.data_len != 0);
        const ptr: [*]const u8 = @ptrFromInt(@intFromPtr(self) + self.data_offset);
        return ptr[0..self.data_len];
    }

    pub fn iterator(self: *const BlockHeader) EventIterator {
        return .{
            .remaining = self.num_events,
            .curr = @ptrCast(@alignCast(self.getData())),
        };
    }
};

pub const EventHeader = extern struct {
    time: u64,
    tag: tracing.events.Event,
    prev_offset: u16,
    byte_len: u16,

    pub const Event = union(tracing.events.Event) {
        start: void,
        finish: void,
        register_thread: *const events.RegisterThread,
        unregister_thread: *const events.UnregisterThread,
        create_call_stack: *const events.CreateCallStack,
        destroy_call_stack: *const events.DestroyCallStack,
        unblock_call_stack: *const events.UnblockCallStack,
        suspend_call_stack: *const events.SuspendCallStack,
        resume_call_stack: *const events.ResumeCallStack,
        enter_span: *const events.EnterSpan,
        exit_span: *const events.ExitSpan,
        log_message: *const events.LogMessage,
    };

    pub fn getTime(self: *const EventHeader) Instant {
        const offset: Duration = .initNanos(self.time);
        return (Instant{}).add(offset) catch unreachable;
    }

    pub fn getEvent(self: *const EventHeader) Event {
        const header_end = @intFromPtr(self) + @sizeOf(EventHeader);

        switch (self.tag) {
            .start => unreachable,
            .finish => unreachable,
            .register_thread => {
                const start = std.mem.alignForward(
                    usize,
                    header_end,
                    @alignOf(events.RegisterThread),
                );
                return .{ .register_thread = @ptrFromInt(start) };
            },
            .unregister_thread => {
                const start = std.mem.alignForward(
                    usize,
                    header_end,
                    @alignOf(events.UnregisterThread),
                );
                return .{ .unregister_thread = @ptrFromInt(start) };
            },
            .create_call_stack => {
                const start = std.mem.alignForward(
                    usize,
                    header_end,
                    @alignOf(events.CreateCallStack),
                );
                return .{ .create_call_stack = @ptrFromInt(start) };
            },
            .destroy_call_stack => {
                const start = std.mem.alignForward(
                    usize,
                    header_end,
                    @alignOf(events.DestroyCallStack),
                );
                return .{ .destroy_call_stack = @ptrFromInt(start) };
            },
            .unblock_call_stack => {
                const start = std.mem.alignForward(
                    usize,
                    header_end,
                    @alignOf(events.UnblockCallStack),
                );
                return .{ .unblock_call_stack = @ptrFromInt(start) };
            },
            .suspend_call_stack => {
                const start = std.mem.alignForward(
                    usize,
                    header_end,
                    @alignOf(events.SuspendCallStack),
                );
                return .{ .suspend_call_stack = @ptrFromInt(start) };
            },
            .resume_call_stack => {
                const start = std.mem.alignForward(
                    usize,
                    header_end,
                    @alignOf(events.ResumeCallStack),
                );
                return .{ .resume_call_stack = @ptrFromInt(start) };
            },
            .enter_span => {
                const start = std.mem.alignForward(
                    usize,
                    header_end,
                    @alignOf(events.EnterSpan),
                );
                return .{ .enter_span = @ptrFromInt(start) };
            },
            .exit_span => {
                const start = std.mem.alignForward(
                    usize,
                    header_end,
                    @alignOf(events.ExitSpan),
                );
                return .{ .exit_span = @ptrFromInt(start) };
            },
            .log_message => {
                const start = std.mem.alignForward(
                    usize,
                    header_end,
                    @alignOf(events.LogMessage),
                );
                return .{ .log_message = @ptrFromInt(start) };
            },
            else => @panic("unknown event"),
        }
    }
};

pub const EventInfo = extern struct {
    name_len: u16,
    target_len: u16,
    scope_len: u16,
    file_name_len: u16,
    line_number: i32,
    level: tracing.Level,
};

pub const events = struct {
    pub const RegisterThread = extern struct {
        thread_id: u64,
    };

    pub const UnregisterThread = extern struct {
        thread_id: u64,
    };

    pub const CreateCallStack = extern struct {
        stack: u64,
    };

    pub const DestroyCallStack = extern struct {
        stack: u64,
    };

    pub const UnblockCallStack = extern struct {
        stack: u64,
    };

    pub const SuspendCallStack = extern struct {
        stack: u64,
        mark_blocked: bool,
    };

    pub const ResumeCallStack = extern struct {
        stack: u64,
        thread_id: u64,
    };

    pub const EnterSpan = extern struct {
        stack: u64,
        span_id: u64,
        info: EventInfo,
        message_len: u16,

        pub fn getName(self: *const EnterSpan) []const u8 {
            const offset = @sizeOf(EnterSpan);
            const ptr: [*]const u8 = @ptrFromInt(@intFromPtr(self) + offset);
            return ptr[0..self.info.name_len];
        }

        pub fn getTarget(self: *const EnterSpan) []const u8 {
            const offset = @sizeOf(EnterSpan) + self.info.name_len;
            const ptr: [*]const u8 = @ptrFromInt(@intFromPtr(self) + offset);
            return ptr[0..self.info.target_len];
        }

        pub fn getScope(self: *const EnterSpan) []const u8 {
            const offset = @sizeOf(EnterSpan) + self.info.name_len + self.info.target_len;
            const ptr: [*]const u8 = @ptrFromInt(@intFromPtr(self) + offset);
            return ptr[0..self.info.scope_len];
        }

        pub fn getFileName(self: *const EnterSpan) []const u8 {
            const offset = @sizeOf(EnterSpan) + self.info.name_len + self.info.target_len +
                self.info.scope_len;
            const ptr: [*]const u8 = @ptrFromInt(@intFromPtr(self) + offset);
            return ptr[0..self.info.file_name_len];
        }

        pub fn getMessage(self: *const EnterSpan) []const u8 {
            const offset = @sizeOf(EnterSpan) + self.info.name_len + self.info.target_len +
                self.info.scope_len + self.info.file_name_len;
            const ptr: [*]const u8 = @ptrFromInt(@intFromPtr(self) + offset);
            return ptr[0..self.message_len];
        }
    };

    pub const ExitSpan = extern struct {
        stack: u64,
        is_unwinding: bool,
    };

    pub const LogMessage = extern struct {
        stack: u64,
        info: EventInfo,
        message_len: u16,

        pub fn getName(self: *const LogMessage) []const u8 {
            const offset = @sizeOf(LogMessage);
            const ptr: [*]const u8 = @ptrFromInt(@intFromPtr(self) + offset);
            return ptr[0..self.info.name_len];
        }

        pub fn getTarget(self: *const LogMessage) []const u8 {
            const offset = @sizeOf(LogMessage) + self.info.name_len;
            const ptr: [*]const u8 = @ptrFromInt(@intFromPtr(self) + offset);
            return ptr[0..self.info.target_len];
        }

        pub fn getScope(self: *const LogMessage) []const u8 {
            const offset = @sizeOf(LogMessage) + self.info.name_len + self.info.target_len;
            const ptr: [*]const u8 = @ptrFromInt(@intFromPtr(self) + offset);
            return ptr[0..self.info.scope_len];
        }

        pub fn getFileName(self: *const LogMessage) []const u8 {
            const offset = @sizeOf(LogMessage) + self.info.name_len + self.info.target_len +
                self.info.scope_len;
            const ptr: [*]const u8 = @ptrFromInt(@intFromPtr(self) + offset);
            return ptr[0..self.info.file_name_len];
        }

        pub fn getMessage(self: *const LogMessage) []const u8 {
            const offset = @sizeOf(LogMessage) + self.info.name_len + self.info.target_len +
                self.info.scope_len + self.info.file_name_len;
            const ptr: [*]const u8 = @ptrFromInt(@intFromPtr(self) + offset);
            return ptr[0..self.message_len];
        }
    };
};

comptime {
    std.debug.assert(@alignOf(FileHeader) <= max_alignment);
    std.debug.assert(@alignOf(Session) <= max_alignment);
    std.debug.assert(@alignOf(BlockHeader) <= max_alignment);
    std.debug.assert(@alignOf(EventHeader) <= max_alignment);
}

pub const StreamWriter = struct {
    writer: *Writer,
    block_buffer: []u8,
    state: union(enum) {
        start: struct {
            start_pos: u64,
            num_sessions: u16,
        },
        session: struct {
            file_start_pos: u64,
            file_num_sessions: u16,
            start_pos: u64,
            byte_len: u64,
            end_time: u64,
            num_blocks: u32,
        },
        block: struct {
            file_start_pos: u64,
            file_num_sessions: u16,
            session_start_pos: u64,
            session_byte_len: u64,
            session_end_time: u64,
            session_num_blocks: u32,
            start_time: u64,
            end_time: u64,
            num_events: u32,
            data_end: u32,
            tail: *align(1) EventHeader,
        },
    },

    pub fn init(writer: *Writer, block_buffer: []u8) !StreamWriter {
        if (block_buffer.len < min_block_len) return error.BlockLengthTooSmall;
        if (block_buffer.len > std.math.maxInt(u32)) return error.BlockLengthTooLarge;
        const start_pos = writer.pos + writer.interface.end;
        try writer.interface.writeStruct(FileHeader{
            .block_compression = .none,
            .block_byte_len = @intCast(block_buffer.len),
            .num_sessions = 0,
            .sessions_offset = 0,
        }, .little);
        const align_offset = std.mem.alignForward(
            u64,
            @sizeOf(FileHeader),
            max_alignment,
        ) - @sizeOf(FileHeader);
        _ = try writer.interface.splatByte(undefined, align_offset);

        return .{
            .writer = writer,
            .block_buffer = block_buffer,
            .state = .{ .start = .{
                .start_pos = start_pos,
                .num_sessions = 0,
            } },
        };
    }

    pub fn finish(self: *StreamWriter) !void {
        if (self.state == .block) try self.flushBlock();
        try self.writer.interface.flush();
        self.* = undefined;
    }

    pub fn writeEvent(self: *StreamWriter, event: *const tracing.events.Event) !void {
        switch (event.*) {
            .start => try self.writeStart(@alignCast(@fieldParentPtr("event", event))),
            .finish => try self.writeFinish(@alignCast(@fieldParentPtr("event", event))),
            .register_thread => try self.writeRegisterThread(@alignCast(@fieldParentPtr("event", event))),
            .unregister_thread => try self.writeUnregisterThread(@alignCast(@fieldParentPtr("event", event))),
            .create_call_stack => try self.writeCreateCallStack(@alignCast(@fieldParentPtr("event", event))),
            .destroy_call_stack => try self.writeDestroyCallStack(@alignCast(@fieldParentPtr("event", event))),
            .unblock_call_stack => try self.writeUnblockCallStack(@alignCast(@fieldParentPtr("event", event))),
            .suspend_call_stack => try self.writeSuspendCallStack(@alignCast(@fieldParentPtr("event", event))),
            .resume_call_stack => try self.writeResumeCallStack(@alignCast(@fieldParentPtr("event", event))),
            .enter_span => try self.writeEnterSpan(@alignCast(@fieldParentPtr("event", event))),
            .exit_span => try self.writeExitSpan(@alignCast(@fieldParentPtr("event", event))),
            .log_message => try self.writeLogMessage(@alignCast(@fieldParentPtr("event", event))),
            else => return error.UnknownEvent,
        }
    }

    fn writeStart(self: *StreamWriter, event: *const tracing.events.Start) !void {
        if (self.state != .start) return error.UnexpectedState;
        var state = self.state.start;
        state.num_sessions += 1;

        const start_pos = self.writer.pos + self.writer.interface.end;
        std.debug.assert(std.mem.isAlignedGeneric(u64, start_pos, max_alignment));
        const start_time = Instant.initC(event.time).durationSince(.{}) catch unreachable;
        const epoch = Time.initC(event.epoch).durationSince(.UnixEpoch) catch unreachable;
        const resolution = Duration.initC(event.resolution);

        if (start_time.nanos() > std.math.maxInt(u64)) return error.TimeOverflow;
        if (epoch.nanos() > std.math.maxInt(u64)) return error.EpochOverflow;
        if (resolution.nanos() > std.math.maxInt(u16)) return error.ResolutionOverflow;
        if (event.cpu_vendor_length > std.math.maxInt(u8)) return error.CpuVendorOverflow;
        if (event.app_name_length > std.math.maxInt(u16)) return error.AppNameOverflow;
        if (event.host_info_length > std.math.maxInt(u16)) return error.HostInfoOverflow;
        if (event.num_cores > std.math.maxInt(u16)) return error.NumCoresOverflow;

        const data_len = @sizeOf(Session) +
            event.cpu_vendor_length +
            event.app_name_length +
            event.host_info_length;
        const byte_len = std.mem.alignForward(u64, data_len, max_alignment);

        try self.writer.interface.writeStruct(Session{
            .byte_len = byte_len,
            .start_time = @truncate(start_time.nanos()),
            .end_time = @truncate(start_time.nanos()),
            .epoch = @truncate(epoch.nanos()),
            .resolution = @truncate(resolution.nanos()),
            .cpu_arch = event.cpu_arch,
            .cpu_vendor_len = @truncate(event.cpu_vendor_length),
            .app_name_len = @truncate(event.app_name_length),
            .host_info_len = @truncate(event.host_info_length),
            .available_memory = event.available_memory,
            .process_id = event.process_id,
            .cpu_id = event.cpu_id,
            .num_cores = @truncate(event.num_cores),
            .block_offset = 0,
            .num_blocks = 0,
        }, .little);
        try self.writer.interface.writeAll(event.cpu_vendor[0..event.cpu_vendor_length]);
        try self.writer.interface.writeAll(event.app_name[0..event.app_name_length]);
        try self.writer.interface.writeAll(event.host_info[0..event.host_info_length]);
        const align_offset = byte_len - data_len;
        _ = try self.writer.interface.splatByte(undefined, align_offset);

        try self.writer.interface.flush();
        const curr_pos = self.writer.pos;
        try self.writer.seekTo(state.start_pos + @offsetOf(FileHeader, "num_sessions"));
        try self.writer.interface.writeInt(u16, state.num_sessions, .little);
        if (state.num_sessions == 1) {
            try self.writer.interface.writeInt(u16, @intCast(start_pos - state.start_pos), .little);
        }
        try self.writer.interface.flush();
        try self.writer.seekTo(curr_pos);

        self.state = .{ .session = .{
            .file_start_pos = state.start_pos,
            .file_num_sessions = state.num_sessions,
            .start_pos = start_pos,
            .byte_len = byte_len,
            .end_time = @truncate(start_time.nanos()),
            .num_blocks = 0,
        } };
    }

    fn writeFinish(self: *StreamWriter, event: *const tracing.events.Finish) !void {
        if (self.state == .start) return error.UnexpectedState;
        if (self.state == .block) try self.flushBlock();
        var state = self.state.session;

        const end_time = Instant.initC(event.time).durationSince(.{}) catch unreachable;
        if (end_time.nanos() > std.math.maxInt(u64)) return error.TimeOverflow;
        state.end_time = @max(state.end_time, @as(u64, @truncate(end_time.nanos())));

        try self.writer.interface.flush();
        const curr_pos = self.writer.pos;
        try self.writer.seekTo(state.start_pos + @offsetOf(Session, "end_time"));
        try self.writer.interface.writeInt(u64, state.end_time, .little);
        try self.writer.interface.flush();
        try self.writer.seekTo(curr_pos);

        self.state = .{ .start = .{
            .start_pos = state.file_start_pos,
            .num_sessions = state.file_num_sessions,
        } };
    }

    fn writeRegisterThread(self: *StreamWriter, event: *const tracing.events.RegisterThread) !void {
        const buffer = try self.prepareBlock(
            event.event,
            event.time,
            @sizeOf(events.RegisterThread),
            @alignOf(events.RegisterThread),
        );
        var writer: Io.Writer = .fixed(buffer);
        try writer.writeStruct(events.RegisterThread{
            .thread_id = event.thread_id,
        }, .little);
    }

    fn writeUnregisterThread(self: *StreamWriter, event: *const tracing.events.UnregisterThread) !void {
        const buffer = try self.prepareBlock(
            event.event,
            event.time,
            @sizeOf(events.UnregisterThread),
            @alignOf(events.UnregisterThread),
        );
        var writer: Io.Writer = .fixed(buffer);
        try writer.writeStruct(events.UnregisterThread{
            .thread_id = event.thread_id,
        }, .little);
    }

    fn writeCreateCallStack(self: *StreamWriter, event: *const tracing.events.CreateCallStack) !void {
        const buffer = try self.prepareBlock(
            event.event,
            event.time,
            @sizeOf(events.CreateCallStack),
            @alignOf(events.CreateCallStack),
        );
        var writer: Io.Writer = .fixed(buffer);
        try writer.writeStruct(events.CreateCallStack{
            .stack = @intFromPtr(event.stack),
        }, .little);
    }

    fn writeDestroyCallStack(self: *StreamWriter, event: *const tracing.events.DestroyCallStack) !void {
        const buffer = try self.prepareBlock(
            event.event,
            event.time,
            @sizeOf(events.DestroyCallStack),
            @alignOf(events.DestroyCallStack),
        );
        var writer: Io.Writer = .fixed(buffer);
        try writer.writeStruct(events.DestroyCallStack{
            .stack = @intFromPtr(event.stack),
        }, .little);
    }

    fn writeUnblockCallStack(self: *StreamWriter, event: *const tracing.events.UnblockCallStack) !void {
        const buffer = try self.prepareBlock(
            event.event,
            event.time,
            @sizeOf(events.UnblockCallStack),
            @alignOf(events.UnblockCallStack),
        );
        var writer: Io.Writer = .fixed(buffer);
        try writer.writeStruct(events.UnblockCallStack{
            .stack = @intFromPtr(event.stack),
        }, .little);
    }

    fn writeSuspendCallStack(self: *StreamWriter, event: *const tracing.events.SuspendCallStack) !void {
        const buffer = try self.prepareBlock(
            event.event,
            event.time,
            @sizeOf(events.SuspendCallStack),
            @alignOf(events.SuspendCallStack),
        );
        var writer: Io.Writer = .fixed(buffer);
        try writer.writeStruct(events.SuspendCallStack{
            .stack = @intFromPtr(event.stack),
            .mark_blocked = event.mark_blocked,
        }, .little);
    }

    fn writeResumeCallStack(self: *StreamWriter, event: *const tracing.events.ResumeCallStack) !void {
        const buffer = try self.prepareBlock(
            event.event,
            event.time,
            @sizeOf(events.ResumeCallStack),
            @alignOf(events.ResumeCallStack),
        );
        var writer: Io.Writer = .fixed(buffer);
        try writer.writeStruct(events.ResumeCallStack{
            .stack = @intFromPtr(event.stack),
            .thread_id = event.thread_id,
        }, .little);
    }

    fn writeEnterSpan(self: *StreamWriter, event: *const tracing.events.EnterSpan) !void {
        const name_len = std.mem.len(event.span.name);
        const target_len = std.mem.len(event.span.target);
        const scope_len = std.mem.len(event.span.scope);
        const file_name_len = if (event.span.file_name) |n| std.mem.len(n) else 0;
        const message_len = event.message_length;
        const strings_len = name_len + target_len + scope_len + file_name_len + message_len;

        const buffer = try self.prepareBlock(
            event.event,
            event.time,
            @sizeOf(events.EnterSpan) + strings_len,
            @alignOf(events.EnterSpan),
        );
        var writer: Io.Writer = .fixed(buffer);
        try writer.writeStruct(events.EnterSpan{
            .stack = @intFromPtr(event.stack),
            .span_id = @intFromPtr(event.span),
            .info = .{
                .name_len = @truncate(name_len),
                .target_len = @truncate(target_len),
                .scope_len = @truncate(scope_len),
                .file_name_len = @truncate(file_name_len),
                .line_number = event.span.line_number,
                .level = event.span.level,
            },
            .message_len = @truncate(message_len),
        }, .little);
        try writer.writeAll(event.span.name[0..name_len]);
        try writer.writeAll(event.span.target[0..target_len]);
        try writer.writeAll(event.span.scope[0..scope_len]);
        if (event.span.file_name) |file| try writer.writeAll(file[0..file_name_len]);
        try writer.writeAll(event.message[0..message_len]);
    }

    fn writeExitSpan(self: *StreamWriter, event: *const tracing.events.ExitSpan) !void {
        const buffer = try self.prepareBlock(
            event.event,
            event.time,
            @sizeOf(events.ExitSpan),
            @alignOf(events.ExitSpan),
        );
        var writer: Io.Writer = .fixed(buffer);
        try writer.writeStruct(events.ExitSpan{
            .stack = @intFromPtr(event.stack),
            .is_unwinding = event.is_unwinding,
        }, .little);
    }

    fn writeLogMessage(self: *StreamWriter, event: *const tracing.events.LogMessage) !void {
        const name_len = std.mem.len(event.info.name);
        const target_len = std.mem.len(event.info.target);
        const scope_len = std.mem.len(event.info.scope);
        const file_name_len = if (event.info.file_name) |n| std.mem.len(n) else 0;
        const message_len = event.message_length;
        const strings_len = name_len + target_len + scope_len + file_name_len + message_len;

        const buffer = try self.prepareBlock(
            event.event,
            event.time,
            @sizeOf(events.LogMessage) + strings_len,
            @alignOf(events.LogMessage),
        );
        var writer: Io.Writer = .fixed(buffer);
        try writer.writeStruct(events.LogMessage{
            .stack = @intFromPtr(event.stack),
            .info = .{
                .name_len = @truncate(name_len),
                .target_len = @truncate(target_len),
                .scope_len = @truncate(scope_len),
                .file_name_len = @truncate(file_name_len),
                .line_number = event.info.line_number,
                .level = event.info.level,
            },
            .message_len = @truncate(message_len),
        }, .little);
        try writer.writeAll(event.info.name[0..name_len]);
        try writer.writeAll(event.info.target[0..target_len]);
        try writer.writeAll(event.info.scope[0..scope_len]);
        if (event.info.file_name) |file| try writer.writeAll(file[0..file_name_len]);
        try writer.writeAll(event.message[0..message_len]);
    }

    fn prepareBlock(
        self: *StreamWriter,
        tag: tracing.events.Event,
        event_time_compat: time.compat.Instant,
        write_size: u64,
        write_align: u64,
    ) ![]u8 {
        const event_duration = Instant.initC(event_time_compat).durationSince(.{}) catch unreachable;
        const event_time: u64 = @truncate(event_duration.nanos());

        const event_offset = std.mem.alignForward(u64, @sizeOf(EventHeader), write_align);
        const event_len = std.mem.alignForward(u64, event_offset + write_size, max_alignment);
        if (event_len > min_block_len) return error.EventTooLarge;

        if (self.state == .start) return error.UnexpectedState;
        if (self.state == .block) {
            var state = self.state.block;
            const start_time = @min(event_time, state.start_time);
            const end_time = @max(event_time, state.end_time);
            std.debug.assert(std.mem.isAlignedGeneric(u64, state.data_end, max_alignment));
            const data_end = state.data_end + event_len;

            var block_full = end_time - start_time > std.math.maxInt(u32);
            block_full |= state.num_events == std.math.maxInt(u32);
            block_full |= data_end > self.block_buffer.len;
            if (!block_full) {
                var curr = state.tail;
                while (curr.time > event_time) {
                    if (@intFromPtr(curr) == @intFromPtr(self.block_buffer.ptr)) break;
                    const address = @intFromPtr(curr);
                    curr = @ptrFromInt(address - curr.prev_offset);
                }

                if (curr != state.tail) {
                    const block_idx = @intFromPtr(curr) - @intFromPtr(self.block_buffer.ptr);
                    const data = self.block_buffer[block_idx..state.data_end];
                    const off_data = self.block_buffer[block_idx + event_len .. state.data_end + event_len];
                    @memmove(off_data, data);
                    state.tail = @ptrFromInt(@intFromPtr(state.tail) + event_len);

                    curr.time = event_time;
                    curr.tag = tag;
                    curr.byte_len = @truncate(event_len);

                    const next: *align(1) EventHeader = @ptrFromInt(@intFromPtr(curr) + event_len);
                    next.prev_offset = @truncate(event_len);
                } else {
                    curr = @ptrCast(&self.block_buffer[state.data_end]);
                    curr.* = .{
                        .time = event_time,
                        .tag = tag,
                        .prev_offset = state.tail.byte_len,
                        .byte_len = @truncate(event_len),
                    };
                    state.tail = curr;
                }
                state.start_time = start_time;
                state.end_time = end_time;
                state.num_events += 1;
                state.data_end = @truncate(data_end);

                self.state = .{ .block = state };
                const event_buffer = @as([*]u8, @ptrCast(curr))[event_offset .. event_offset + write_size];
                return event_buffer;
            }
            try self.flushBlock();
        }

        const tail: *align(1) EventHeader = @ptrCast(&self.block_buffer[0]);
        tail.* = .{
            .time = event_time,
            .tag = tag,
            .prev_offset = 0,
            .byte_len = @truncate(event_len),
        };

        const state = self.state.session;
        if (state.num_blocks == std.math.maxInt(u32)) return error.NumBlocksOverflow;
        const session = self.state.session;
        self.state = .{ .block = .{
            .file_start_pos = session.file_start_pos,
            .file_num_sessions = session.file_num_sessions,
            .session_start_pos = session.start_pos,
            .session_byte_len = session.byte_len,
            .session_end_time = session.end_time,
            .session_num_blocks = session.num_blocks,
            .start_time = event_time,
            .end_time = event_time,
            .num_events = 1,
            .data_end = @truncate(event_len),
            .tail = tail,
        } };

        const event_buffer = @as([*]u8, @ptrCast(tail))[event_offset .. event_offset + write_size];
        return event_buffer;
    }

    fn flushBlock(self: *StreamWriter) !void {
        var state = switch (self.state) {
            .block => |v| v,
            else => return error.UnexpectedState,
        };

        if (state.data_end != 0) {
            const data_len = std.mem.alignForward(u64, state.data_end, max_alignment);
            state.session_byte_len += data_len;
            state.session_end_time = @max(state.session_end_time, state.end_time);
            state.session_num_blocks += 1;

            const block_start = self.writer.pos + self.writer.interface.end;
            std.debug.assert(std.mem.isAlignedGeneric(u64, block_start, max_alignment));
            const header_end = block_start + @sizeOf(BlockHeader);
            const data_start = std.mem.alignForward(u64, header_end, max_alignment);
            try self.writer.interface.writeStruct(BlockHeader{
                .start_time = state.start_time,
                .time_range = @truncate(state.end_time - state.start_time),
                .num_events = state.num_events,
                .data_len = @truncate(data_len),
                .data_offset = @truncate(data_start - block_start),
            }, .little);
            _ = try self.writer.interface.splatByte(undefined, @truncate(data_start - header_end));
            try self.writer.interface.writeAll(self.block_buffer[0..state.data_end]);
            _ = try self.writer.interface.splatByte(undefined, @truncate(data_len - state.data_end));

            try self.writer.interface.flush();
            const curr_pos = self.writer.pos;
            try self.writer.seekTo(state.session_start_pos);
            try self.writer.interface.writeInt(u64, state.session_byte_len, .little);
            try self.writer.interface.flush();
            try self.writer.seekTo(state.session_start_pos + @offsetOf(Session, "end_time"));
            try self.writer.interface.writeInt(u64, state.session_end_time, .little);
            try self.writer.interface.flush();

            if (state.session_num_blocks == 1) {
                const block_offset = block_start - state.session_start_pos;
                try self.writer.seekTo(state.session_start_pos + @offsetOf(Session, "block_offset"));
                try self.writer.interface.writeInt(u16, @truncate(block_offset), .little);
                try self.writer.interface.writeInt(u32, state.session_num_blocks, .little);
            } else {
                try self.writer.seekTo(state.session_start_pos + @offsetOf(Session, "num_blocks"));
                try self.writer.interface.writeInt(u32, state.session_num_blocks, .little);
            }
            try self.writer.interface.flush();
            try self.writer.seekTo(curr_pos);
        }

        self.state = .{ .session = .{
            .file_start_pos = state.file_start_pos,
            .file_num_sessions = state.file_num_sessions,
            .start_pos = state.session_start_pos,
            .byte_len = state.session_byte_len,
            .end_time = state.session_end_time,
            .num_blocks = state.session_num_blocks,
        } };
    }
};

extern "kernel32" fn CreateFileMappingA(
    hFile: std.os.windows.HANDLE,
    lpFileMappingAttributes: ?*std.os.windows.SECURITY_ATTRIBUTES,
    flProtect: std.os.windows.DWORD,
    dwMaximumSizeHigh: std.os.windows.DWORD,
    dwMaximumSizeLow: std.os.windows.DWORD,
    lpName: ?std.os.windows.LPCSTR,
) callconv(.winapi) ?std.os.windows.LPVOID;

extern "kernel32" fn MapViewOfFileEx(
    hFileMappingObject: std.os.windows.HANDLE,
    dwDesiredAccess: std.os.windows.DWORD,
    dwFileOffsetHigh: std.os.windows.DWORD,
    dwFileOffsetLow: std.os.windows.DWORD,
    dwNumberOfBytesToMap: std.os.windows.SIZE_T,
    lpBaseAddress: ?std.os.windows.LPVOID,
) callconv(.winapi) ?std.os.windows.LPVOID;

extern "kernel32" fn UnmapViewOfFile(
    lpBaseAddress: std.os.windows.LPCVOID,
) callconv(.winapi) std.os.windows.BOOL;

pub const StreamReader = struct {
    header: *const FileHeader,
    handle: if (builtin.target.os.tag == .windows)
        std.os.windows.HANDLE
    else
        usize,

    pub fn init(file: File) !StreamReader {
        var buffer: [64]u8 = undefined;
        var reader = file.reader(&buffer);
        const header = try reader.interface.takeStruct(FileHeader, .little);
        if (!std.mem.eql(u8, &header.file_magic, FileHeader.expect_magic)) return error.InvalidStream;
        if (header.version_major != FileHeader.expect_version_major) return error.InvalidStream;
        if (header.version_minor > FileHeader.expect_version_minor) return error.InvalidStream;
        if (header.block_compression != .none) return error.InvalidStream;
        if (header.block_byte_len < min_block_len) return error.InvalidStream;
        if (header.num_sessions != 0 and header.sessions_offset < @sizeOf(FileHeader))
            return error.InvalidStream;

        if (comptime builtin.target.os.tag == .windows) {
            const handle = CreateFileMappingA(
                file.handle,
                null,
                std.os.windows.PAGE_READONLY,
                0,
                0,
                null,
            ) orelse return error.FileMappingFailed;
            errdefer std.os.windows.CloseHandle(handle);

            const FILE_MAP_READ = 4;
            const mapping = MapViewOfFileEx(
                handle,
                FILE_MAP_READ,
                0,
                0,
                0,
                null,
            ) orelse return error.FileMappingFailed;
            errdefer _ = UnmapViewOfFile(mapping);
            if (!std.mem.isAligned(@intFromPtr(mapping), @alignOf(FileHeader)))
                return error.InvalidStream;

            return .{
                .header = @ptrCast(@alignCast(mapping)),
                .handle = handle,
            };
        } else {
            const stat = try file.stat();
            const mapping = try std.posix.mmap(
                null,
                stat.size,
                std.posix.PROT.READ,
                std.posix.MAP.TYPE.PRIVATE,
                file.handle,
                0,
            );
            return .{
                .header = @ptrCast(@alignCast(mapping)),
                .handle = mapping.len,
            };
        }
    }

    pub fn deinit(self: *StreamReader) void {
        if (comptime builtin.target.os.tag == .windows) {
            _ = UnmapViewOfFile(self.header);
            std.os.windows.CloseHandle(self.handle);
        } else {
            const ptr: [*]const u8 = @ptrCast(self.header);
            std.posix.munmap(ptr[0..self.handle]);
        }
        self.* = undefined;
    }

    pub fn getSession(self: *const StreamReader, idx: usize) ?*const Session {
        if (idx >= self.header.num_sessions) return null;
        var session: *const Session = @ptrFromInt(@intFromPtr(self.header) + self.header.sessions_offset);
        std.debug.assert(std.mem.isAligned(@intFromPtr(session), max_alignment));
        for (0..idx) |_| {
            const next: *const Session = @ptrFromInt(@intFromPtr(session) + session.byte_len);
            std.debug.assert(std.mem.isAligned(@intFromPtr(next), max_alignment));
            session = next;
        }
        return session;
    }
};

test {
    const file = try std.fs.cwd().createFile("test.ftrstr", .{ .read = true });
    defer {
        file.close();
        std.fs.cwd().deleteFile("test.ftrstr") catch unreachable;
    }

    var file_buffer: [256]u8 = undefined;
    var file_writer = file.writer(&file_buffer);
    var block_buffer: [min_block_len]u8 = undefined;
    var writer: StreamWriter = try .init(&file_writer, &block_buffer);

    const t0: Instant = .now();
    const t1 = t0.addSaturating(.initSeconds(1));
    const t2 = t1.addSaturating(.initSeconds(1));
    const t3 = t0.addSaturating(.initMillis(500));
    const t4 = t2.addSaturating(.initSeconds(1));

    const cpu_vendor = "test cpu";
    const app_name = "test app";
    const host_info = "test host";
    const info: tracing.EventInfo = .at(@src(), .@"test", .err);
    const message = "test message";
    const ev0 = tracing.events.Start{
        .time = t0.intoC(),
        .epoch = Time.now().intoC(),
        .resolution = Duration.initNanos(100).intoC(),
        .available_memory = 1234,
        .process_id = 5678,
        .num_cores = 9,
        .cpu_arch = .x86_64,
        .cpu_id = 0,
        .cpu_vendor = cpu_vendor,
        .cpu_vendor_length = cpu_vendor.len,
        .app_name = app_name,
        .app_name_length = app_name.len,
        .host_info = host_info,
        .host_info_length = host_info.len,
    };
    const ev1 = tracing.events.RegisterThread{
        .time = t1.intoC(),
        .thread_id = 5,
    };
    const ev2 = tracing.events.UnregisterThread{
        .time = t2.intoC(),
        .thread_id = 5,
    };
    const ev3 = tracing.events.LogMessage{
        .time = t3.intoC(),
        .stack = @ptrFromInt(12345),
        .info = &info,
        .message = message,
        .message_length = message.len,
    };
    const ev4 = tracing.events.Finish{
        .time = t4.intoC(),
    };

    try writer.writeEvent(&ev0.event);
    try writer.writeEvent(&ev1.event);
    try writer.writeEvent(&ev2.event);
    try writer.writeEvent(&ev3.event);
    try writer.writeEvent(&ev4.event);
    try writer.finish();

    var reader = try StreamReader.init(file);
    defer reader.deinit();

    try std.testing.expectEqual(.none, reader.header.block_compression);
    try std.testing.expectEqual(min_block_len, reader.header.block_byte_len);
    try std.testing.expectEqual(1, reader.header.num_sessions);

    const session = reader.getSession(0).?;
    try std.testing.expectEqual(ev0.time, session.getStartTime().intoC());
    try std.testing.expectEqual(ev4.time, session.getEndTime().intoC());
    try std.testing.expectEqual(ev0.epoch, session.getEpoch().intoC());
    try std.testing.expectEqual(ev0.resolution, session.getResolution().intoC());
    try std.testing.expectEqual(ev0.available_memory, session.available_memory);
    try std.testing.expectEqual(ev0.process_id, session.process_id);
    try std.testing.expectEqual(ev0.num_cores, session.num_cores);
    try std.testing.expectEqual(ev0.cpu_arch, session.cpu_arch);
    try std.testing.expectEqualStrings(cpu_vendor, session.getCpuVendor());
    try std.testing.expectEqualStrings(app_name, session.getAppName());
    try std.testing.expectEqualStrings(host_info, session.getHostInfo());
    try std.testing.expectEqual(1, session.num_blocks);

    const block = session.getBlock(0).?;
    try std.testing.expectEqual(ev3.time, block.getStartTime().intoC());
    try std.testing.expectEqual(ev2.time, block.getEndTime().intoC());
    try std.testing.expectEqual(3, block.num_events);

    var iterator = block.iterator();
    const block_ev0_header = iterator.next().?;
    try std.testing.expectEqual(ev3.event, block_ev0_header.tag);
    const block_ev0 = block_ev0_header.getEvent().log_message;
    try std.testing.expectEqual(@intFromPtr(ev3.stack), block_ev0.stack);
    try std.testing.expectEqualStrings(std.mem.span(info.name), block_ev0.getName());
    try std.testing.expectEqualStrings(std.mem.span(info.target), block_ev0.getTarget());
    try std.testing.expectEqualStrings(std.mem.span(info.scope), block_ev0.getScope());
    try std.testing.expectEqualStrings(std.mem.span(info.file_name.?), block_ev0.getFileName());
    try std.testing.expectEqualStrings(message, block_ev0.getMessage());

    const block_ev1_header = iterator.next().?;
    try std.testing.expectEqual(ev1.event, block_ev1_header.tag);
    const block_ev1 = block_ev1_header.getEvent().register_thread;
    try std.testing.expectEqual(ev1.thread_id, block_ev1.thread_id);

    const block_ev2_header = iterator.next().?;
    try std.testing.expectEqual(ev2.event, block_ev2_header.tag);
    const block_ev2 = block_ev2_header.getEvent().unregister_thread;
    try std.testing.expectEqual(ev2.thread_id, block_ev2.thread_id);

    try std.testing.expectEqual(null, iterator.next());
}
