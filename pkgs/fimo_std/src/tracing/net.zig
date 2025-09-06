const std = @import("std");
const net = std.net;
const atomic = std.atomic;
const Allocator = std.mem.Allocator;
const Io = std.Io;
const Writer = Io.Writer;
const Reader = Io.Reader;
const Thread = std.Thread;
const Mutex = Thread.Mutex;
const Condition = Thread.Condition;
const ResetEvent = Thread.ResetEvent;
const DoublyLinkedList = std.DoublyLinkedList;

const time = @import("../time.zig");
const Instant = time.Instant;
const Time = time.Time;
const Duration = time.Duration;
const tracing = @import("../tracing.zig");
const Level = tracing.Level;
const EventInfo = tracing.EventInfo;
const Subscriber = tracing.Subscriber;
const events = tracing.events;

pub const protocol = struct {
    pub const name: *const [27]u8 = "Fimo Tracing Network Client";
    pub const version_major = 1;
    pub const version_minor = 0;
    pub const max_buffer_len = std.math.maxInt(u16);

    pub const default_host = "127.0.0.1";
    pub const default_port = 5882;

    pub const messages = struct {
        pub const Tag = enum(u16) {
            accept,
            reject,
            close,
            raw_block,
            _,
        };
        pub const Hello = packed struct(u232) {
            name: u216 = @bitCast(name.*),
            version_major: u8 = version_major,
            version_minor: u8 = version_minor,
        };
        pub const Accept = packed struct(u16) {
            tag: Tag = .accept,
        };
        pub const Reject = packed struct(u32) {
            tag: Tag = .reject,
            version_major: u8 = version_major,
            version_minor: u8 = version_minor,
        };
        pub const Close = packed struct(u16) {
            tag: Tag = .close,
        };
        pub const RawBlock = packed struct(u32) {
            tag: Tag = .raw_block,
            len: u16,
        };
    };

    pub const events = struct {
        pub const Event = union(enum) {
            start: struct {
                main: Start,
                strings: []u8,

                pub fn deinit(self: @This(), gpa: Allocator) void {
                    gpa.free(self.strings);
                }

                pub fn getCpuVendor(self: @This()) []const u8 {
                    const len = self.main.cpu_vendor_len;
                    return self.strings[0..len];
                }

                pub fn getAppName(self: @This()) []const u8 {
                    const offset = self.main.cpu_vendor_len;
                    const len = self.main.app_name_len;
                    return self.strings[offset..][0..len];
                }

                pub fn getHostInfo(self: @This()) []const u8 {
                    const offset = self.main.cpu_vendor_len + self.main.app_name_len;
                    const len = self.main.host_info_len;
                    return self.strings[offset..][0..len];
                }
            },
            finish: Finish,
            register_thread: RegisterThread,
            unregister_thread: UnregisterThread,
            create_call_stack: CreateCallStack,
            destroy_call_stack: DestroyCallStack,
            unblock_call_stack: UnblockCallStack,
            suspend_call_stack: SuspendCallStack,
            resume_call_stack: ResumeCallStack,
            enter_span: struct {
                main: EnterSpan,
                message: []u8,

                pub fn deinit(self: @This(), gpa: Allocator) void {
                    gpa.free(self.message);
                }

                pub fn getMessage(self: @This()) []const u8 {
                    const len = self.main.message_len;
                    return self.message[0..len];
                }
            },
            exit_span: ExitSpan,
            log_message: struct {
                main: LogMessage,
                message: []u8,

                pub fn deinit(self: @This(), gpa: Allocator) void {
                    gpa.free(self.message);
                }

                pub fn getMessage(self: @This()) []const u8 {
                    const len = self.main.message_len;
                    return self.message[0..len];
                }
            },
            declare_event_info: struct {
                main: DeclareEventInfo,
                strings: []u8,

                pub fn deinit(self: @This(), gpa: Allocator) void {
                    gpa.free(self.strings);
                }

                pub fn getName(self: @This()) []const u8 {
                    const len = self.main.name_len;
                    return self.strings[0..len];
                }

                pub fn getTarget(self: @This()) []const u8 {
                    const offset = self.main.name_len;
                    const len = self.main.target_len;
                    return self.strings[offset..][0..len];
                }

                pub fn getScope(self: @This()) []const u8 {
                    const offset = self.main.name_len + self.main.target_len;
                    const len = self.main.scope_len;
                    return self.strings[offset..][0..len];
                }

                pub fn getFileName(self: @This()) []const u8 {
                    const offset = self.main.name_len + self.main.target_len + self.main.scope_len;
                    const len = self.main.file_name_len;
                    return self.strings[offset..][0..len];
                }
            },
            start_thread: StartThread,
            stop_thread: StopThread,
            load_image: struct {
                main: LoadImage,
                path: []u8,

                pub fn deinit(self: @This(), gpa: Allocator) void {
                    gpa.free(self.path);
                }
            },
            unload_image: UnloadImage,
            context_switch: ContextSwitch,
            thread_wakeup: ThreadWakeup,
            call_stack_sample: struct {
                main: CallStackSample,
                call_stack: []u64,

                pub fn deinit(self: @This(), gpa: Allocator) void {
                    gpa.free(self.call_stack);
                }
            },
            _,

            pub fn deinit(self: Event, gpa: Allocator) void {
                switch (self) {
                    .start => |event| event.deinit(gpa),
                    .enter_span => |event| event.deinit(gpa),
                    .log_message => |event| event.deinit(gpa),
                    .declare_event_info => |event| event.deinit(gpa),
                    .load_image => |event| event.deinit(gpa),
                    .call_stack_sample => |event| event.deinit(gpa),
                    else => {},
                }
            }
        };

        pub const Start = packed struct(u400) {
            tag: tracing.events.EventTag = .start,
            time: u64,
            epoch: u64,
            resolution: u16,
            available_memory: u64,
            process_id: u64,
            num_cores: u16,
            cpu_arch: tracing.events.CpuArch,
            cpu_id: u32,
            cpu_vendor_len: u8,
            app_name_len: u16,
            host_info_len: u16,
        };
        pub const Finish = packed struct(u96) {
            tag: tracing.events.EventTag = .finish,
            time: u64,
        };
        pub const RegisterThread = packed struct(u160) {
            tag: tracing.events.EventTag = .register_thread,
            time: u64,
            thread_id: u64,
        };
        pub const UnregisterThread = packed struct(u160) {
            tag: tracing.events.EventTag = .unregister_thread,
            time: u64,
            thread_id: u64,
        };
        pub const CreateCallStack = packed struct(u160) {
            tag: tracing.events.EventTag = .create_call_stack,
            time: u64,
            stack: u64,
        };
        pub const DestroyCallStack = packed struct(u160) {
            tag: tracing.events.EventTag = .destroy_call_stack,
            time: u64,
            stack: u64,
        };
        pub const UnblockCallStack = packed struct(u160) {
            tag: tracing.events.EventTag = .unblock_call_stack,
            time: u64,
            stack: u64,
        };
        pub const SuspendCallStack = packed struct(u168) {
            tag: tracing.events.EventTag = .suspend_call_stack,
            time: u64,
            stack: u64,
            mark_blocked: bool,
            padding: u7 = 0,
        };
        pub const ResumeCallStack = packed struct(u224) {
            tag: tracing.events.EventTag = .resume_call_stack,
            time: u64,
            stack: u64,
            thread_id: u64,
        };
        pub const EnterSpan = packed struct(u240) {
            tag: tracing.events.EventTag = .enter_span,
            time: u64,
            stack: u64,
            info_id: u64,
            message_len: u16,
        };
        pub const ExitSpan = packed struct(u168) {
            tag: tracing.events.EventTag = .exit_span,
            time: u64,
            stack: u64,
            is_unwinding: bool,
            padding: u7 = 0,
        };
        pub const LogMessage = packed struct(u240) {
            tag: tracing.events.EventTag = .log_message,
            time: u64,
            stack: u64,
            info_id: u64,
            message_len: u16,
        };
        pub const DeclareEventInfo = packed struct(u224) {
            tag: tracing.events.EventTag = .declare_event_info,
            id: u64,
            name_len: u16,
            target_len: u16,
            scope_len: u16,
            file_name_len: u16,
            line_number: i32,
            level: Level,
        };
        pub const StartThread = packed struct(u224) {
            tag: tracing.events.EventTag = .start_thread,
            time: u64,
            thread_id: u64,
            process_id: u64,
        };
        pub const StopThread = packed struct(u224) {
            tag: tracing.events.EventTag = .stop_thread,
            time: u64,
            thread_id: u64,
            process_id: u64,
        };
        pub const LoadImage = packed struct(u240) {
            tag: tracing.events.EventTag = .load_image,
            time: u64,
            image_base: u64,
            image_size: u64,
            image_path_len: u16,
        };
        pub const UnloadImage = packed struct(u160) {
            tag: tracing.events.EventTag = .unload_image,
            time: u64,
            image_base: u64,
        };
        pub const ContextSwitch = packed struct(u272) {
            tag: tracing.events.EventTag = .context_switch,
            time: u64,
            old_thread_id: u64,
            new_thread_id: u64,
            cpu: u8,
            old_thread_wait_reason: u8,
            old_thread_state: u8,
            previous_cstate: u8,
            new_thread_priority: i8,
            old_thread_priority: i8,
        };
        pub const ThreadWakeup = packed struct(u184) {
            tag: tracing.events.EventTag = .thread_wakeup,
            time: u64,
            thread_id: u64,
            cpu: u8,
            adjust_reason: i8,
            adjust_increment: i8,
        };
        pub const CallStackSample = packed struct(u176) {
            tag: tracing.events.EventTag = .call_stack_sample,
            time: u64,
            thread_id: u64,
            call_stack_len: u16,
        };
    };
};

pub const NetLogger = struct {
    gpa: Allocator,
    worker: Thread,
    address: net.Address,
    connected: atomic.Value(bool) = .init(false),
    mutex: std.Thread.Mutex = .{},
    condition: std.Thread.Condition = .{},
    queue: std.DoublyLinkedList = .{},
    free_list: std.DoublyLinkedList = .{},
    closed: atomic.Value(bool) = .init(false),

    pub const fimo_subscriber = .{
        .start = onStart,
        .finish = onFinish,
        .register_thread = onRegisterThread,
        .unregister_thread = onUnregisterThread,
        .create_call_stack = onCreateCallStack,
        .destroy_call_stack = onDestroyCallStack,
        .unblock_call_stack = onUnblockCallStack,
        .suspend_call_stack = onSuspendCallStack,
        .resume_call_stack = onResumeCallStack,
        .enter_span = onEnterSpan,
        .exit_span = onExitSpan,
        .log_message = onLogMessage,
        .declare_event_info = onDeclareEventInfo,
        .start_thread = onStartThread,
        .stop_thread = onStopThread,
        .load_image = onLoadImage,
        .unload_image = onUnloadImage,
        .context_switch = onContextSwitch,
        .thread_wakeup = onThreadWakeup,
        .call_stack_sample = onCallStackSample,
    };

    pub const Options = struct {
        gpa: Allocator,
        server: Server.Options = .{},
        wait_for_connection: bool = false,
    };

    pub fn init(self: *NetLogger, options: Options) !void {
        const address = try net.Address.parseIp(options.server.host_name, options.server.port);
        var server: ?Server = if (options.wait_for_connection) blk: {
            while (true) {
                break :blk Server.initAddress(.{
                    .address = address,
                    .reuse_address = options.server.reuse_address,
                }) catch continue;
            }
        } else null;
        errdefer if (server) |*s| {
            s.close() catch {};
            s.deinit();
        };

        self.* = .{
            .gpa = options.gpa,
            .worker = undefined,
            .address = address,
        };
        self.worker = try .spawn(.{}, runWorker, .{ self, server, options.wait_for_connection });
    }

    pub fn deinit(self: *NetLogger) void {
        self.mutex.lock();
        self.closed.store(true, .monotonic);
        self.condition.signal();
        self.mutex.unlock();

        if (!self.connected.load(.monotonic)) {
            if (net.tcpConnectToAddress(self.address)) |stream| stream.close() else |_| {}
        }
        self.worker.join();

        while (self.free_list.pop()) |node| {
            const block: *Block = @alignCast(@fieldParentPtr("node", node));
            block.deinit(self.gpa);
        }
        self.* = undefined;
    }

    pub fn subscriber(self: *NetLogger) Subscriber {
        return .of(NetLogger, self);
    }

    fn onStart(self: *NetLogger, event: *const events.Start) void {
        if (self.closed.load(.acquire)) return;
        const cpu_vendor = event.cpu_vendor.intoSliceOrEmpty();
        const app_name = event.app_name.intoSliceOrEmpty();
        const host_info = event.host_info.intoSliceOrEmpty();

        const cpu_vendor_len = @min(cpu_vendor.len, std.math.maxInt(u8));
        const app_name_len = @min(app_name.len, std.math.maxInt(u16));
        const host_info_len = @min(host_info.len, std.math.maxInt(u16));
        const strings_len = cpu_vendor_len + app_name_len + host_info_len;

        const strings = self.gpa.alloc(u8, strings_len) catch @panic("oom");
        var w: std.Io.Writer = .fixed(strings);
        w.writeAll(cpu_vendor[0..cpu_vendor_len]) catch unreachable;
        w.writeAll(app_name[0..app_name_len]) catch unreachable;
        w.writeAll(host_info[0..host_info_len]) catch unreachable;

        const ev: protocol.events.Start = .{
            .time = @intCast((Instant.initC(event.time).durationSince(.{}) catch unreachable).nanos()),
            .epoch = @intCast((Time.initC(event.epoch).durationSince(.UnixEpoch) catch unreachable).nanos()),
            .resolution = @intCast(Duration.initC(event.resolution).nanos()),
            .available_memory = event.available_memory,
            .process_id = event.process_id,
            .num_cores = @intCast(event.num_cores),
            .cpu_arch = event.cpu_arch,
            .cpu_id = event.cpu_id,
            .cpu_vendor_len = @truncate(cpu_vendor_len),
            .app_name_len = @truncate(app_name_len),
            .host_info_len = @truncate(host_info_len),
        };

        self.pushMessage(.{ .start = .{ .main = ev, .strings = strings } });
    }

    fn onFinish(self: *NetLogger, event: *const events.Finish) void {
        if (self.closed.load(.acquire)) return;
        const ev: protocol.events.Finish = .{
            .time = @intCast((Instant.initC(event.time).durationSince(.{}) catch unreachable).nanos()),
        };
        self.pushMessage(.{ .finish = ev });
    }

    fn onRegisterThread(self: *NetLogger, event: *const events.RegisterThread) void {
        if (self.closed.load(.acquire)) return;
        const ev: protocol.events.RegisterThread = .{
            .time = @intCast((Instant.initC(event.time).durationSince(.{}) catch unreachable).nanos()),
            .thread_id = event.thread_id,
        };
        self.pushMessage(.{ .register_thread = ev });
    }

    fn onUnregisterThread(self: *NetLogger, event: *const events.UnregisterThread) void {
        if (self.closed.load(.acquire)) return;
        const ev: protocol.events.UnregisterThread = .{
            .time = @intCast((Instant.initC(event.time).durationSince(.{}) catch unreachable).nanos()),
            .thread_id = event.thread_id,
        };
        self.pushMessage(.{ .unregister_thread = ev });
    }

    fn onCreateCallStack(self: *NetLogger, event: *const events.CreateCallStack) void {
        if (self.closed.load(.acquire)) return;
        const ev: protocol.events.CreateCallStack = .{
            .time = @intCast((Instant.initC(event.time).durationSince(.{}) catch unreachable).nanos()),
            .stack = @intFromPtr(event.stack),
        };
        self.pushMessage(.{ .create_call_stack = ev });
    }

    fn onDestroyCallStack(self: *NetLogger, event: *const events.DestroyCallStack) void {
        if (self.closed.load(.acquire)) return;
        const ev: protocol.events.DestroyCallStack = .{
            .time = @intCast((Instant.initC(event.time).durationSince(.{}) catch unreachable).nanos()),
            .stack = @intFromPtr(event.stack),
        };
        self.pushMessage(.{ .destroy_call_stack = ev });
    }

    fn onUnblockCallStack(self: *NetLogger, event: *const events.UnblockCallStack) void {
        if (self.closed.load(.acquire)) return;
        const ev: protocol.events.UnblockCallStack = .{
            .time = @intCast((Instant.initC(event.time).durationSince(.{}) catch unreachable).nanos()),
            .stack = @intFromPtr(event.stack),
        };
        self.pushMessage(.{ .unblock_call_stack = ev });
    }

    fn onSuspendCallStack(self: *NetLogger, event: *const events.SuspendCallStack) void {
        if (self.closed.load(.acquire)) return;
        const ev: protocol.events.SuspendCallStack = .{
            .time = @intCast((Instant.initC(event.time).durationSince(.{}) catch unreachable).nanos()),
            .stack = @intFromPtr(event.stack),
            .mark_blocked = event.mark_blocked,
        };
        self.pushMessage(.{ .suspend_call_stack = ev });
    }

    fn onResumeCallStack(self: *NetLogger, event: *const events.ResumeCallStack) void {
        if (self.closed.load(.acquire)) return;
        const ev: protocol.events.ResumeCallStack = .{
            .time = @intCast((Instant.initC(event.time).durationSince(.{}) catch unreachable).nanos()),
            .stack = @intFromPtr(event.stack),
            .thread_id = event.thread_id,
        };
        self.pushMessage(.{ .resume_call_stack = ev });
    }

    fn onEnterSpan(self: *NetLogger, event: *const events.EnterSpan) void {
        if (self.closed.load(.acquire)) return;
        const msg = event.message.intoSliceOrEmpty();
        const message_len = @min(msg.len, std.math.maxInt(u16));
        const message = self.gpa.dupe(u8, msg[0..message_len]) catch @panic("oom");

        const ev: protocol.events.EnterSpan = .{
            .time = @intCast((Instant.initC(event.time).durationSince(.{}) catch unreachable).nanos()),
            .stack = @intFromPtr(event.stack),
            .info_id = @intFromPtr(event.span),
            .message_len = @truncate(message_len),
        };
        self.pushMessage(.{ .enter_span = .{ .main = ev, .message = message } });
    }

    fn onExitSpan(self: *NetLogger, event: *const events.ExitSpan) void {
        if (self.closed.load(.acquire)) return;
        const ev: protocol.events.ExitSpan = .{
            .time = @intCast((Instant.initC(event.time).durationSince(.{}) catch unreachable).nanos()),
            .stack = @intFromPtr(event.stack),
            .is_unwinding = event.is_unwinding,
        };
        self.pushMessage(.{ .exit_span = ev });
    }

    fn onLogMessage(self: *NetLogger, event: *const events.LogMessage) void {
        if (self.closed.load(.acquire)) return;
        const msg = event.message.intoSliceOrEmpty();
        const message_len = @min(msg.len, std.math.maxInt(u16));
        const message = self.gpa.dupe(u8, msg[0..message_len]) catch @panic("oom");

        const ev: protocol.events.LogMessage = .{
            .time = @intCast((Instant.initC(event.time).durationSince(.{}) catch unreachable).nanos()),
            .stack = @intFromPtr(event.stack),
            .info_id = @intFromPtr(event.info),
            .message_len = @truncate(message_len),
        };
        self.pushMessage(.{ .log_message = .{ .main = ev, .message = message } });
    }

    fn onDeclareEventInfo(self: *NetLogger, event: *const events.DeclareEventInfo) void {
        if (self.closed.load(.acquire)) return;
        const name_len = @min(std.mem.span(event.info.name).len, std.math.maxInt(u16));
        const target_len = @min(std.mem.span(event.info.target).len, std.math.maxInt(u16));
        const scope_len = @min(std.mem.span(event.info.scope).len, std.math.maxInt(u16));
        const file_name_len = if (event.info.file_name) |file_name|
            @min(std.mem.span(file_name).len, std.math.maxInt(u16))
        else
            0;
        const strings_len = name_len + target_len + scope_len + file_name_len;

        const strings = self.gpa.alloc(u8, strings_len) catch @panic("oom");
        var w: std.Io.Writer = .fixed(strings);
        w.writeAll(event.info.name[0..name_len]) catch unreachable;
        w.writeAll(event.info.target[0..target_len]) catch unreachable;
        w.writeAll(event.info.scope[0..scope_len]) catch unreachable;
        if (event.info.file_name) |file_name| w.writeAll(file_name[0..file_name_len]) catch unreachable;

        const ev: protocol.events.DeclareEventInfo = .{
            .id = @intFromPtr(event.info),
            .name_len = @truncate(name_len),
            .target_len = @truncate(target_len),
            .scope_len = @truncate(scope_len),
            .file_name_len = @truncate(file_name_len),
            .line_number = event.info.line_number,
            .level = event.info.level,
        };
        self.pushMessage(.{ .declare_event_info = .{ .main = ev, .strings = strings } });
    }

    fn onStartThread(self: *NetLogger, event: *const events.StartThread) void {
        if (self.closed.load(.acquire)) return;
        const ev: protocol.events.StartThread = .{
            .time = @intCast((Instant.initC(event.time).durationSince(.{}) catch unreachable).nanos()),
            .thread_id = event.thread_id,
            .process_id = event.process_id,
        };
        self.pushMessage(.{ .start_thread = ev });
    }

    fn onStopThread(self: *NetLogger, event: *const events.StopThread) void {
        if (self.closed.load(.acquire)) return;
        const ev: protocol.events.StopThread = .{
            .time = @intCast((Instant.initC(event.time).durationSince(.{}) catch unreachable).nanos()),
            .thread_id = event.thread_id,
            .process_id = event.process_id,
        };
        self.pushMessage(.{ .stop_thread = ev });
    }

    fn onLoadImage(self: *NetLogger, event: *const events.LoadImage) void {
        if (self.closed.load(.acquire)) return;

        const path = self.gpa.dupe(u8, event.image_path.intoSliceOrEmpty()) catch @panic("oom");
        const ev: protocol.events.LoadImage = .{
            .time = @intCast((Instant.initC(event.time).durationSince(.{}) catch unreachable).nanos()),
            .image_base = event.image_base,
            .image_size = event.image_size,
            .image_path_len = @intCast(event.image_path.len),
        };
        self.pushMessage(.{ .load_image = .{ .main = ev, .path = path } });
    }

    fn onUnloadImage(self: *NetLogger, event: *const events.UnloadImage) void {
        if (self.closed.load(.acquire)) return;
        const ev: protocol.events.UnloadImage = .{
            .time = @intCast((Instant.initC(event.time).durationSince(.{}) catch unreachable).nanos()),
            .image_base = event.image_base,
        };
        self.pushMessage(.{ .unload_image = ev });
    }

    fn onContextSwitch(self: *NetLogger, event: *const events.ContextSwitch) void {
        if (self.closed.load(.acquire)) return;
        const ev: protocol.events.ContextSwitch = .{
            .time = @intCast((Instant.initC(event.time).durationSince(.{}) catch unreachable).nanos()),
            .old_thread_id = event.old_thread_id,
            .new_thread_id = event.new_thread_id,
            .cpu = event.cpu,
            .old_thread_wait_reason = event.old_thread_wait_reason,
            .old_thread_state = event.old_thread_state,
            .previous_cstate = event.previous_cstate,
            .new_thread_priority = event.new_thread_priority,
            .old_thread_priority = event.old_thread_priority,
        };
        self.pushMessage(.{ .context_switch = ev });
    }

    fn onThreadWakeup(self: *NetLogger, event: *const events.ThreadWakeup) void {
        if (self.closed.load(.acquire)) return;
        const ev: protocol.events.ThreadWakeup = .{
            .time = @intCast((Instant.initC(event.time).durationSince(.{}) catch unreachable).nanos()),
            .thread_id = event.thread_id,
            .cpu = event.cpu,
            .adjust_reason = event.adjust_reason,
            .adjust_increment = event.adjust_increment,
        };
        self.pushMessage(.{ .thread_wakeup = ev });
    }

    fn onCallStackSample(self: *NetLogger, event: *const events.CallStackSample) void {
        if (self.closed.load(.acquire)) return;
        const cs = event.call_stack.intoSliceOrEmpty();
        const call_stack = self.gpa.alloc(u64, cs.len) catch @panic("oom");
        for (call_stack, cs) |*dst, src| dst.* = src;

        const ev: protocol.events.CallStackSample = .{
            .time = @intCast((Instant.initC(event.time).durationSince(.{}) catch unreachable).nanos()),
            .thread_id = event.thread_id,
            .call_stack_len = @truncate(cs.len),
        };
        self.pushMessage(.{ .call_stack_sample = .{ .main = ev, .call_stack = call_stack } });
    }

    const Block = struct {
        events: [block_size]protocol.events.Event = undefined,
        count: u8 = 0,
        idx: u8 = 0,
        node: std.DoublyLinkedList.Node = .{},

        const block_size = 128;
        comptime {
            if (!std.math.isPowerOfTwo(block_size)) @compileError("block_size must be a power of two");
        }

        fn init(event: protocol.events.Event, gpa: std.mem.Allocator) *Block {
            const block = gpa.create(Block) catch @panic("oom");
            block.reset(event);
            return block;
        }

        fn deinit(self: *Block, gpa: std.mem.Allocator) void {
            std.debug.assert(self.idx == self.count);
            gpa.destroy(self);
        }

        fn reset(self: *Block, event: protocol.events.Event) void {
            self.* = .{};
            self.events[0] = event;
            self.count = 1;
            self.idx = 0;
        }

        fn tryWrite(self: *Block, event: protocol.events.Event) bool {
            if (self.count == block_size) return false;
            self.events[self.count] = event;
            self.count += 1;
            return true;
        }

        fn read(self: *Block) ?protocol.events.Event {
            if (self.idx == self.count) return null;
            const event = self.events[self.idx];
            self.idx += 1;
            return event;
        }
    };

    fn takeOrAllocEmptyBlock(self: *NetLogger, event: protocol.events.Event) *Block {
        const node = self.free_list.popFirst() orelse {
            return Block.init(event, self.gpa);
        };
        const block: *Block = @alignCast(@fieldParentPtr("node", node));
        block.reset(event);
        return block;
    }

    fn pushMessage(self: *NetLogger, event: protocol.events.Event) void {
        self.mutex.lock();
        defer self.mutex.unlock();
        if (self.closed.load(.monotonic)) {
            event.deinit(self.gpa);
            return;
        }

        const tail = self.queue.last orelse {
            const block = self.takeOrAllocEmptyBlock(event);
            self.queue.append(&block.node);
            self.condition.signal();
            return;
        };

        const block: *Block = @alignCast(@fieldParentPtr("node", tail));
        if (!block.tryWrite(event)) {
            const new_block = self.takeOrAllocEmptyBlock(event);
            self.queue.append(&new_block.node);
        }
        self.condition.signal();
    }

    fn waitOnMessage(self: *NetLogger) enum { stop, has_message } {
        self.mutex.lock();
        defer self.mutex.unlock();
        while (true) {
            if (self.queue.first == null) {
                if (self.closed.load(.monotonic)) return .stop;
                self.condition.wait(&self.mutex);
                continue;
            }
            return .has_message;
        }
    }

    fn popAll(self: *NetLogger) ?*Block {
        const head = blk: {
            self.mutex.lock();
            defer self.mutex.unlock();
            const head = self.queue.first;
            self.queue.first = null;
            self.queue.last = null;
            break :blk head orelse return null;
        };
        const block: *Block = @alignCast(@fieldParentPtr("node", head));
        return block;
    }

    fn freeBlocks(self: *NetLogger, block: *Block) void {
        var tail = block;
        while (tail.node.next) |next| {
            std.debug.assert(tail.idx == tail.count);
            tail = @alignCast(@fieldParentPtr("node", next));
        }
        std.debug.assert(tail.idx == tail.count);
        self.mutex.lock();
        defer self.mutex.unlock();

        if (tail == block) {
            self.free_list.prepend(&tail.node);
        } else {
            if (self.free_list.first) |first| {
                tail.node.next = first;
                first.prev = &tail.node;
            } else self.free_list.last = &tail.node;
            self.free_list.first = &block.node;
        }
    }

    fn clearQueue(self: *NetLogger) void {
        self.closed.store(true, .release);
        while (self.popAll()) |blocks| {
            defer self.freeBlocks(blocks);
            var curr: ?*Block = blocks;
            while (curr) |block| {
                while (block.read()) |event| event.deinit(self.gpa);
                curr = if (block.node.next) |next| @alignCast(@fieldParentPtr("node", next)) else null;
            }
        }
    }

    fn flushBlock(writer: *std.Io.Writer) std.io.Writer.Error!void {
        const header: *align(1) protocol.messages.RawBlock = @ptrCast(writer.buffer);
        header.len = @truncate(writer.end - @sizeOf(protocol.messages.RawBlock));
        try writer.flush();
    }

    fn writeStruct(writer: *std.Io.Writer, event: anytype) std.io.Writer.Error!void {
        if (writer.end + @divExact(@bitSizeOf(@TypeOf(event)), 8) > writer.buffer.len) {
            try flushBlock(writer);
            writer.writeStruct(protocol.messages.RawBlock{ .len = 0 }, .little) catch unreachable;
        }
        writer.writeStruct(event, .little) catch unreachable;
    }

    fn writeString(writer: *std.Io.Writer, string: []const u8) std.io.Writer.Error!void {
        var remaining = string;
        while (remaining.len > 0) {
            if (writer.end == writer.buffer.len) {
                try flushBlock(writer);
                writer.writeStruct(protocol.messages.RawBlock{ .len = 0 }, .little) catch unreachable;
            }
            const write_len = @min(writer.buffer[writer.end..].len, remaining.len);
            const slice = remaining[0..write_len];
            remaining = remaining[write_len..];
            writer.writeAll(slice) catch unreachable;
        }
    }

    fn runWorker(self: *NetLogger, s: ?Server, reuse_address: bool) void {
        defer self.clearQueue();
        var server = s orelse blk: {
            while (true) {
                if (self.closed.load(.acquire)) return;
                break :blk Server.initAddress(.{
                    .address = self.address,
                    .reuse_address = reuse_address,
                }) catch continue;
            }
        };
        defer server.deinit();

        self.connected.store(true, .monotonic);
        while (true) {
            if (self.waitOnMessage() == .stop) {
                server.close() catch {};
                return;
            }

            while (self.popAll()) |blocks| {
                defer self.freeBlocks(blocks);
                var curr: ?*Block = blocks;
                while (curr) |block| {
                    while (block.read()) |event| {
                        defer event.deinit(self.gpa);
                        server.writeEvent(event) catch return;
                    }
                    curr = if (block.node.next) |next| @alignCast(@fieldParentPtr("node", next)) else null;
                }
            }

            server.flush() catch return;
        }
    }
};

pub const Server = struct {
    connection: net.Stream,
    started_writing: bool = false,
    remaining_buffer: []u8 = &.{},
    buffer: [@sizeOf(protocol.messages.RawBlock) + protocol.max_buffer_len]u8 = undefined,

    pub const Options = struct {
        host_name: []const u8 = protocol.default_host,
        port: u16 = protocol.default_port,
        reuse_address: bool = true,
    };

    pub const Options2 = struct {
        address: net.Address,
        reuse_address: bool = true,
    };

    pub fn init(options: Options) !Server {
        const address = try net.Address.parseIp(options.host_name, options.port);
        return initAddress(.{ .address = address, .reuse_address = options.reuse_address });
    }

    pub fn initAddress(options: Options2) !Server {
        var server = try options.address.listen(.{
            .reuse_address = options.reuse_address,
        });
        defer server.deinit();

        const connection = try server.accept();
        errdefer connection.stream.close();

        var buffer: [@sizeOf(protocol.messages.Hello)]u8 = undefined;
        var reader = connection.stream.reader(&buffer);
        const hello = try reader.interface().takeStruct(protocol.messages.Hello, .little);

        var writer = connection.stream.writer(&.{});
        if (hello.name != @as(u216, @bitCast(protocol.name.*)) or
            hello.version_major != protocol.version_major or
            hello.version_minor > protocol.version_minor)
        {
            try writer.interface.writeStruct(protocol.messages.Reject{}, .little);
            return error.ConnectionRejected;
        }

        try writer.interface.writeStruct(protocol.messages.Accept{}, .little);
        return .{ .connection = connection.stream };
    }

    pub fn deinit(self: *Server) void {
        self.connection.close();
    }

    fn writeStruct(self: *Server, value: anytype) Writer.Error!void {
        if (self.remaining_buffer.len < @divExact(@bitSizeOf(@TypeOf(value)), 8)) try self.flush();
        var writer: Writer = .fixed(self.remaining_buffer);
        defer self.remaining_buffer = self.remaining_buffer[writer.end..];
        writer.writeStruct(value, .little) catch unreachable;
    }

    fn writeString(self: *Server, value: []const u8) Writer.Error!void {
        var remaining = value;
        while (remaining.len != 0) {
            if (self.remaining_buffer.len == 0) try self.flush();
            const writable_len = @min(self.remaining_buffer.len, remaining.len);
            @memcpy(self.remaining_buffer[0..writable_len], remaining[0..writable_len]);
            remaining = remaining[writable_len..];
            self.remaining_buffer = self.remaining_buffer[writable_len..];
        }
    }

    pub fn writeEvent(self: *Server, event: protocol.events.Event) Writer.Error!void {
        if (!self.started_writing) {
            self.remaining_buffer = self.buffer[@sizeOf(protocol.messages.RawBlock)..][0..protocol.max_buffer_len];
            self.started_writing = true;
        }
        switch (event) {
            .start => |m| {
                const main, const strings = .{ m.main, m.strings };
                try self.writeStruct(main);
                try self.writeString(strings);
            },
            .finish => |m| try self.writeStruct(m),
            .register_thread => |m| try self.writeStruct(m),
            .unregister_thread => |m| try self.writeStruct(m),
            .create_call_stack => |m| try self.writeStruct(m),
            .destroy_call_stack => |m| try self.writeStruct(m),
            .unblock_call_stack => |m| try self.writeStruct(m),
            .suspend_call_stack => |m| try self.writeStruct(m),
            .resume_call_stack => |m| try self.writeStruct(m),
            .enter_span => |m| {
                const main, const strings = .{ m.main, m.message };
                try self.writeStruct(main);
                try self.writeString(strings);
            },
            .exit_span => |m| try self.writeStruct(m),
            .log_message => |m| {
                const main, const strings = .{ m.main, m.message };
                try self.writeStruct(main);
                try self.writeString(strings);
            },
            .declare_event_info => |m| {
                const main, const strings = .{ m.main, m.strings };
                try self.writeStruct(main);
                try self.writeString(strings);
            },
            .start_thread => |m| try self.writeStruct(m),
            .stop_thread => |m| try self.writeStruct(m),
            .load_image => |m| {
                const main, const path = .{ m.main, m.path };
                try self.writeStruct(main);
                try self.writeString(path);
            },
            .unload_image => |m| try self.writeStruct(m),
            .context_switch => |m| try self.writeStruct(m),
            .thread_wakeup => |m| try self.writeStruct(m),
            .call_stack_sample => |m| {
                const main, const call_stack = .{ m.main, m.call_stack };
                try self.writeStruct(main);
                try self.writeString(@ptrCast(call_stack));
            },
            else => @panic("unknown event"),
        }
    }

    pub fn flush(self: *Server) Writer.Error!void {
        if (!self.started_writing) return;
        if (self.remaining_buffer.len == protocol.max_buffer_len) return;
        const block_len = protocol.max_buffer_len - self.remaining_buffer.len;
        const block: *align(1) protocol.messages.RawBlock = @ptrCast(&self.buffer);
        block.* = .{ .len = @truncate(block_len) };
        const send_buffer = self.buffer[0 .. @sizeOf(protocol.messages.RawBlock) + block_len];
        var writer = self.connection.writer(&.{});
        try writer.interface.writeAll(send_buffer);
        self.remaining_buffer = self.buffer[@sizeOf(protocol.messages.RawBlock)..][0..protocol.max_buffer_len];
    }

    pub fn close(self: *Server) Writer.Error!void {
        try self.flush();
        const message: protocol.messages.Close = .{};
        var writer = self.connection.writer(&.{});
        try writer.interface.writeStruct(message, .little);
    }
};

pub const Client = struct {
    gpa: Allocator,
    stream: net.Stream,
    buffer: [protocol.max_buffer_len]u8 = undefined,
    unprocessed: []u8 = &.{},
    closed: bool = false,

    pub const Options = struct {
        gpa: Allocator,
        host_name: []const u8 = protocol.default_host,
        port: u16 = protocol.default_port,
    };

    pub fn init(options: Options) !Client {
        const address = try net.Address.parseIp(options.host_name, options.port);
        const stream = try net.tcpConnectToAddress(address);
        errdefer stream.close();

        var w = stream.writer(&.{});
        try w.interface.writeStruct(protocol.messages.Hello{}, .little);

        var buffer: [64]u8 = undefined;
        var r = stream.reader(&buffer);
        const tag = try r.interface().peekInt(@typeInfo(protocol.messages.Tag).@"enum".tag_type, .little);
        switch (@as(protocol.messages.Tag, @enumFromInt(tag))) {
            .accept => {
                const msg = try r.interface().takeStruct(protocol.messages.Accept, .little);
                std.debug.assert(msg.tag == .accept);
                return .{ .gpa = options.gpa, .stream = stream };
            },
            .reject => {
                const msg = try r.interface().takeStruct(protocol.messages.Reject, .little);
                std.debug.assert(msg.tag == .reject);
                if (msg.version_major != protocol.version_major or msg.version_minor > protocol.version_minor)
                    return error.VersionMismatch;
                return error.ConnectionRejected;
            },
            else => return error.ProtocolError,
        }
    }

    pub fn close(self: *Client) void {
        if (!self.closed) self.stream.close();
        self.closed = true;
    }

    fn revcMessage(self: *Client) !enum { closed, buffered } {
        if (self.unprocessed.len != 0) return .buffered;
        var buffer: [@sizeOf(protocol.messages.Tag)]u8 = undefined;
        var r = self.stream.reader(@ptrCast(&buffer));
        const tag = r.interface().takeEnumNonexhaustive(protocol.messages.Tag, .little) catch switch (r.getError().?) {
            error.ConnectionResetByPeer => {
                self.close();
                return .closed;
            },
            else => return error.ReadFailed,
        };
        switch (tag) {
            .close => {
                self.close();
                return .closed;
            },
            .raw_block => {
                const len = try r.interface().takeInt(@FieldType(protocol.messages.RawBlock, "len"), .little);
                r = self.stream.reader(&.{});
                try r.interface().readSliceAll(self.buffer[0..len]);
                self.unprocessed = self.buffer[0..len];
                std.debug.assert(len != 0);
                return .buffered;
            },
            else => return error.ProtocolError,
        }
    }

    fn recvBlock(self: *Client) Reader.Error!void {
        std.debug.assert(self.unprocessed.len == 0);
        var buffer: [@sizeOf(protocol.messages.RawBlock)]u8 = undefined;
        var r = self.stream.reader(@ptrCast(&buffer));
        const block = try r.interface().takeStruct(protocol.messages.RawBlock, .little);
        std.debug.assert(block.tag == .raw_block);

        r = self.stream.reader(&.{});
        try r.interface().readSliceAll(self.buffer[0..block.len]);
        self.unprocessed = self.buffer[0..block.len];
    }

    fn peekTag(self: *Client) tracing.events.EventTag {
        std.debug.assert(self.unprocessed.len >= @divExact(@bitSizeOf(tracing.events.EventTag), 8));
        var r: std.Io.Reader = .fixed(self.unprocessed);
        const tag = r.peekInt(@typeInfo(tracing.events.EventTag).@"enum".tag_type, .little) catch unreachable;
        return @enumFromInt(tag);
    }

    fn readStruct(self: *Client, T: type) T {
        std.debug.assert(self.unprocessed.len >= @divExact(@bitSizeOf(T), 8));
        var r: std.Io.Reader = .fixed(self.unprocessed);
        defer self.unprocessed = self.unprocessed[r.seek..];
        return r.takeStruct(T, .little) catch unreachable;
    }

    fn readString(self: *Client, buffer: []u8) Reader.Error!void {
        var remaining = buffer;
        while (remaining.len != 0) {
            if (self.unprocessed.len == 0) try self.recvBlock();
            const min_len = @min(remaining.len, self.unprocessed.len);
            @memcpy(remaining[0..min_len], self.unprocessed[0..min_len]);
            self.unprocessed = self.unprocessed[min_len..];
            remaining = remaining[min_len..];
        }
    }

    pub fn readEvent(self: *Client) !?protocol.events.Event {
        errdefer self.close();
        if ((try self.revcMessage()) == .closed) return null;
        switch (self.peekTag()) {
            .start => {
                const event = self.readStruct(protocol.events.Start);
                const cpu_vendor_len: usize = event.cpu_vendor_len;
                const app_name_len: usize = event.app_name_len;
                const host_info_len: usize = event.host_info_len;
                const strings_len = cpu_vendor_len + app_name_len + host_info_len;
                const strings = try self.gpa.alloc(u8, strings_len);
                try self.readString(strings);
                return .{ .start = .{ .main = event, .strings = strings } };
            },
            .finish => {
                const event = self.readStruct(protocol.events.Finish);
                return .{ .finish = event };
            },
            .register_thread => {
                const event = self.readStruct(protocol.events.RegisterThread);
                return .{ .register_thread = event };
            },
            .unregister_thread => {
                const event = self.readStruct(protocol.events.UnregisterThread);
                return .{ .unregister_thread = event };
            },
            .create_call_stack => {
                const event = self.readStruct(protocol.events.CreateCallStack);
                return .{ .create_call_stack = event };
            },
            .destroy_call_stack => {
                const event = self.readStruct(protocol.events.DestroyCallStack);
                return .{ .destroy_call_stack = event };
            },
            .unblock_call_stack => {
                const event = self.readStruct(protocol.events.UnblockCallStack);
                return .{ .unblock_call_stack = event };
            },
            .suspend_call_stack => {
                const event = self.readStruct(protocol.events.SuspendCallStack);
                return .{ .suspend_call_stack = event };
            },
            .resume_call_stack => {
                const event = self.readStruct(protocol.events.ResumeCallStack);
                return .{ .resume_call_stack = event };
            },
            .enter_span => {
                const event = self.readStruct(protocol.events.EnterSpan);
                const message_len: usize = event.message_len;
                const message = try self.gpa.alloc(u8, message_len);
                try self.readString(message);
                return .{ .enter_span = .{ .main = event, .message = message } };
            },
            .exit_span => {
                const event = self.readStruct(protocol.events.ExitSpan);
                return .{ .exit_span = event };
            },
            .log_message => {
                const event = self.readStruct(protocol.events.LogMessage);
                const message_len: usize = event.message_len;
                const message = try self.gpa.alloc(u8, message_len);
                try self.readString(message);
                return .{ .log_message = .{ .main = event, .message = message } };
            },
            .declare_event_info => {
                const event = self.readStruct(protocol.events.DeclareEventInfo);
                const name_len: usize = event.name_len;
                const target_len: usize = event.target_len;
                const scope_len: usize = event.scope_len;
                const file_name_len: usize = event.file_name_len;
                const strings_len = name_len + target_len + scope_len + file_name_len;
                const strings = try self.gpa.alloc(u8, strings_len);
                try self.readString(strings);
                return .{ .declare_event_info = .{ .main = event, .strings = strings } };
            },
            .start_thread => {
                const event = self.readStruct(protocol.events.StartThread);
                return .{ .start_thread = event };
            },
            .stop_thread => {
                const event = self.readStruct(protocol.events.StopThread);
                return .{ .stop_thread = event };
            },
            .load_image => {
                const event = self.readStruct(protocol.events.LoadImage);
                const path = try self.gpa.alloc(u8, event.image_path_len);
                try self.readString(path);
                return .{ .load_image = .{ .main = event, .path = path } };
            },
            .unload_image => {
                const event = self.readStruct(protocol.events.UnloadImage);
                return .{ .unload_image = event };
            },
            .context_switch => {
                const event = self.readStruct(protocol.events.ContextSwitch);
                return .{ .context_switch = event };
            },
            .thread_wakeup => {
                const event = self.readStruct(protocol.events.ThreadWakeup);
                return .{ .thread_wakeup = event };
            },
            .call_stack_sample => {
                const event = self.readStruct(protocol.events.CallStackSample);
                const call_stack = try self.gpa.alloc(u64, event.call_stack_len);
                try self.readString(@ptrCast(call_stack));
                return .{ .call_stack_sample = .{ .main = event, .call_stack = call_stack } };
            },
            else => return error.UnknownEvent,
        }
    }
};

test "Server: no connection" {
    var server: NetLogger = undefined;
    try server.init(.{ .gpa = std.testing.allocator });
    server.deinit();
}

test "Client-Server" {
    var server: NetLogger = undefined;
    try server.init(.{ .gpa = std.testing.allocator, .server = .{ .port = 5883 } });
    defer server.deinit();

    var client: Client = blk: while (true)
        break :blk Client.init(.{ .gpa = std.testing.allocator, .port = 5883 }) catch |err| switch (err) {
            error.ConnectionRefused => continue,
            else => return err,
        };
    defer client.close();

    const epoch: Time = .now();
    const resolution: Duration = .initNanos(100);
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
        .epoch = epoch.intoC(),
        .resolution = resolution.intoC(),
        .available_memory = 1234,
        .process_id = 5678,
        .num_cores = 9,
        .cpu_arch = .x86_64,
        .cpu_id = 0,
        .cpu_vendor = .fromSlice(cpu_vendor),
        .app_name = .fromSlice(app_name),
        .host_info = .fromSlice(host_info),
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
        .message = .fromSlice(message),
    };
    const ev4 = tracing.events.Finish{
        .time = t4.intoC(),
    };

    const subscriber = server.subscriber();
    subscriber.start(ev0);
    subscriber.registerThread(ev1);
    subscriber.unregisterThread(ev2);
    subscriber.logMessage(ev3);
    subscriber.finish(ev4);

    const cl_ev0 = (try client.readEvent()).?.start;
    defer cl_ev0.deinit(std.testing.allocator);
    try std.testing.expectEqual((t0.durationSince(.{}) catch unreachable).nanos(), cl_ev0.main.time);
    try std.testing.expectEqual((epoch.durationSince(.UnixEpoch) catch unreachable).nanos(), cl_ev0.main.epoch);
    try std.testing.expectEqual(resolution.nanos(), cl_ev0.main.resolution);
    try std.testing.expectEqual(ev0.available_memory, cl_ev0.main.available_memory);
    try std.testing.expectEqual(ev0.process_id, cl_ev0.main.process_id);
    try std.testing.expectEqual(ev0.num_cores, cl_ev0.main.num_cores);
    try std.testing.expectEqual(ev0.cpu_arch, cl_ev0.main.cpu_arch);
    try std.testing.expectEqual(ev0.cpu_id, cl_ev0.main.cpu_id);
    try std.testing.expectEqual(cpu_vendor.len, cl_ev0.main.cpu_vendor_len);
    try std.testing.expectEqual(app_name.len, cl_ev0.main.app_name_len);
    try std.testing.expectEqual(host_info.len, cl_ev0.main.host_info_len);
    try std.testing.expectEqualStrings(cpu_vendor, cl_ev0.getCpuVendor());
    try std.testing.expectEqualStrings(app_name, cl_ev0.getAppName());
    try std.testing.expectEqualStrings(host_info, cl_ev0.getHostInfo());

    const cl_ev1 = (try client.readEvent()).?.register_thread;
    try std.testing.expectEqual((t1.durationSince(.{}) catch unreachable).nanos(), cl_ev1.time);

    const cl_ev2 = (try client.readEvent()).?.unregister_thread;
    try std.testing.expectEqual((t2.durationSince(.{}) catch unreachable).nanos(), cl_ev2.time);

    const cl_ev3 = (try client.readEvent()).?.log_message;
    defer cl_ev3.deinit(std.testing.allocator);
    try std.testing.expectEqual((t3.durationSince(.{}) catch unreachable).nanos(), cl_ev3.main.time);
    try std.testing.expectEqual(@intFromPtr(&info), cl_ev3.main.info_id);
    try std.testing.expectEqual(message.len, cl_ev3.main.message_len);
    try std.testing.expectEqualStrings(message, cl_ev3.getMessage());

    const cl_ev4 = (try client.readEvent()).?.finish;
    try std.testing.expectEqual((t4.durationSince(.{}) catch unreachable).nanos(), cl_ev4.time);
}
