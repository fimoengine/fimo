const std = @import("std");
const atomic = std.atomic;

const fimo_std = @import("fimo_std");
const time = fimo_std.time;
const Duration = time.Duration;
const Instant = time.Instant;

const Futex = @import("../Futex.zig");
const Worker = @import("../Worker.zig");
const receiver = @import("receiver.zig");
const RecvError = receiver.RecvError;
const TimedRecvError = receiver.TimedRecvError;
const WaitError = receiver.WaitError;
const Receiver = receiver.Receiver;

/// A receiver that receives messages from multiple receivers.
pub fn MultiReceiver(comptime Receivers: type, comptime Ts: []const type) type {
    const recv_type_info = @typeInfo(Receivers);
    if (recv_type_info != .@"struct") {
        @compileError("expected tuple or struct argument, found " ++ @typeName(Receivers));
    }

    const fields_info = recv_type_info.@"struct".fields;
    const num_receivers = fields_info.len;

    var enum_fields: [Ts.len]std.builtin.Type.EnumField = undefined;
    inline for (0..Ts.len) |i| {
        @setEvalBranchQuota(10_000);
        var num_buf: [128]u8 = undefined;
        enum_fields[i] = .{
            .name = std.fmt.bufPrintZ(&num_buf, "{d}", .{i}) catch unreachable,
            .value = i,
        };
    }
    const Enum = @Type(.{
        .@"enum" = .{
            .tag_type = std.math.IntFittingRange(0, Ts.len),
            .decls = &.{},
            .fields = &enum_fields,
            .is_exhaustive = true,
        },
    });

    var union_fields: [Ts.len]std.builtin.Type.UnionField = undefined;
    inline for (Ts, 0..) |T, i| {
        @setEvalBranchQuota(10_000);
        var num_buf: [128]u8 = undefined;
        union_fields[i] = .{
            .name = std.fmt.bufPrintZ(&num_buf, "{d}", .{i}) catch unreachable,
            .type = T,
            .alignment = 0,
        };
    }
    const T = @Type(.{
        .@"union" = .{
            .layout = .auto,
            .tag_type = Enum,
            .decls = &.{},
            .fields = &union_fields,
        },
    });

    return struct {
        receivers: Receivers,

        const Self = @This();

        /// Receives one message from any of the receivers.
        ///
        /// The caller will return `null` immediately, if all channels are empty.
        /// If all channels are closed and empty, an error is returned.
        pub fn tryRecv(self: *const Self) RecvError!?T {
            var num_closed: usize = 0;
            inline for (self.receivers, std.meta.fields(T)) |rx, field| {
                const result = rx.tryRecv() catch |err| switch (err) {
                    RecvError.Closed => blk: {
                        num_closed += 1;
                        break :blk null;
                    },
                    else => return err,
                };
                if (result) |value| return @unionInit(T, field.name, value);
            }
            if (num_closed == num_receivers) return RecvError.Closed;
            return null;
        }

        /// Receives one message from the channels.
        ///
        /// The caller will block until a message is available.
        /// If all channels are closed and empty, an error is returned.
        pub fn recv(self: *const Self, futex: *Futex) RecvError!T {
            if (try self.tryRecv()) |msg| return msg;
            return self.recvSlow(futex, null) catch |err| switch (err) {
                TimedRecvError.Closed => RecvError.Closed,
                TimedRecvError.Timeout => unreachable,
            };
        }

        /// Receives one message from the channels.
        ///
        /// The caller will block until a message is available or the specified duration has elapsed.
        /// If all channels are closed and empty, an error is returned.
        pub fn recvFor(self: *const Self, futex: *Futex, duration: Duration) TimedRecvError!T {
            const timeout = Instant.now().addSaturating(duration);
            return self.recvUntil(futex, timeout);
        }

        /// Receives one message from the channels.
        ///
        /// The caller will block until a message is available or the timeout is reached.
        /// If all channels are closed and empty, an error is returned.
        pub fn recvUntil(self: *const Self, futex: *Futex, timeout: Instant) TimedRecvError!T {
            if (try self.tryRecv()) |msg| return msg;
            return self.recvSlow(futex, timeout);
        }

        fn recvSlow(self: *const Self, futex: *Futex, timeout: ?Instant) TimedRecvError!T {
            @branchHint(.cold);

            var spin_count: usize = 0;
            const spin_relax_limit = 12;
            const spin_limit = spin_relax_limit + 4;
            loop: while (true) {
                // Try receiving a message from any channel and remember which channel is still open.
                var num_closed_receivers: usize = 0;
                var receiver_open = [_]bool{true} ** num_receivers;
                inline for (&receiver_open, self.receivers, std.meta.fields(T)) |*s, rx, field| {
                    const result = rx.tryRecv() catch |err| switch (err) {
                        RecvError.Closed => blk: {
                            s.* = false;
                            num_closed_receivers += 1;
                            break :blk null;
                        },
                        else => return err,
                    };
                    if (result) |value| return @unionInit(T, field.name, value);
                }
                if (num_closed_receivers == num_receivers) return TimedRecvError.Closed;

                // Try spinning a couple of times.
                if (spin_count < spin_limit) {
                    if (spin_count < spin_relax_limit)
                        atomic.spinLoopHint()
                    else
                        Worker.yield();
                    spin_count += 1;
                    continue;
                }

                // Filter the remaining channels.
                var num_keys: usize = 0;
                var keys: [num_receivers]Futex.KeyExpect = undefined;
                var rx_indices: [num_receivers]usize = undefined;
                inline for (&receiver_open, self.receivers, 0..) |open, rx, i| {
                    if (open) {
                        keys[num_keys] = rx.prepareWait() catch |err| switch (err) {
                            WaitError.Retry => continue :loop,
                            WaitError.Timeout => return TimedRecvError.Timeout,
                        };
                        rx_indices[num_keys] = i;
                        num_keys += 1;
                    }
                }

                const idx = futex.waitv(keys[0..num_keys], timeout) catch |err| switch (err) {
                    error.KeyError => unreachable,
                    error.Invalid => continue,
                    error.Timeout => return TimedRecvError.Timeout,
                };
                const rx_idx = rx_indices[idx];
                inline for (self.receivers, std.meta.fields(T), 0..) |rx, field, i| {
                    if (i == rx_idx) {
                        const result = rx.tryRecv() catch |err| switch (err) {
                            RecvError.Closed => null,
                            else => return err,
                        };
                        if (result) |value| return @unionInit(T, field.name, value);
                    }
                }

                // Retry another round.
                spin_count = 0;
            }
        }
    };
}

/// Constructs a new `MultiReceiver`.
pub fn multi_receiver(comptime Ts: []const type, receivers: anytype) MultiReceiver(@TypeOf(receivers), Ts) {
    return .{ .receivers = receivers };
}

fn channelTest(
    repeats: usize,
    num_threads: usize,
    num_messages: usize,
    channel_weight: f32,
) !void {
    const IntrusiveMpscChannel = @import("intrusive_mpsc.zig").IntrusiveMpscChannel;
    const Node = struct {
        next: ?*@This() = null,
        received: bool = false,
    };

    var futex = Futex.init(std.testing.allocator);
    defer futex.deinit();

    for (0..repeats) |_| {
        var channel_1 = IntrusiveMpscChannel(*Node).empty;
        var channel_2 = IntrusiveMpscChannel(*Node).empty;
        const rx = multi_receiver(&.{ *Node, *Node }, .{ channel_1.receiver(), channel_2.receiver() });

        const nodes = try std.testing.allocator.alloc(Node, num_messages);
        defer std.testing.allocator.free(nodes);
        for (nodes) |*node| node.* = .{};

        var node_counter = atomic.Value(usize).init(0);
        var sent_counter = atomic.Value(usize).init(0);

        const Runner = struct {
            thread: std.Thread = undefined,
            rng: std.Random.DefaultPrng,
            channel_weight: f32,
            sx_1: IntrusiveMpscChannel(*Node).Sender,
            sx_2: IntrusiveMpscChannel(*Node).Sender,
            futex: *Futex,
            num_threads: usize,
            nodes: []Node,
            node_counter: *atomic.Value(usize),
            sent_counter: *atomic.Value(usize),

            fn run(self: *@This()) void {
                while (true) {
                    const idx = self.node_counter.fetchAdd(1, .monotonic);
                    if (idx >= self.nodes.len) return;

                    const rng = self.rng.random();
                    const sx = if (rng.float(f32) < self.channel_weight) self.sx_1 else self.sx_2;

                    const node = &self.nodes[idx];
                    sx.send(self.futex, node) catch unreachable;

                    if (self.sent_counter.fetchAdd(1, .release) == self.nodes.len - 1) {
                        self.sx_1.channel.close(self.futex);
                        self.sx_2.channel.close(self.futex);
                    }
                }
            }
        };

        const runners = try std.testing.allocator.alloc(Runner, num_threads);
        defer std.testing.allocator.free(runners);
        for (runners, 0..) |*r, i| {
            r.* = .{
                .rng = .init(@intCast(i)),
                .channel_weight = channel_weight,
                .sx_1 = channel_1.sender(),
                .sx_2 = channel_2.sender(),
                .futex = &futex,
                .num_threads = num_threads,
                .nodes = nodes,
                .node_counter = &node_counter,
                .sent_counter = &sent_counter,
            };
            r.thread = try std.Thread.spawn(.{}, Runner.run, .{r});
        }

        while (true) {
            const elem = rx.recv(&futex) catch break;
            switch (elem) {
                .@"0", .@"1" => |v| v.received = true,
            }
        }
        for (runners) |r| r.thread.join();
        for (nodes) |n| try std.testing.expectEqual(true, n.received);
    }
}

test "multi receiver: one one zero" {
    try channelTest(1000, 1, 1, 0.0);
}

test "multi receiver: one one fifty" {
    try channelTest(1000, 1, 1, 0.5);
}

test "multi receiver: one hundred zero" {
    try channelTest(100, 1, 100, 0.0);
}

test "multi receiver: one hundred twenty" {
    try channelTest(100, 1, 100, 0.2);
}

test "multi receiver: one hundred fifty" {
    try channelTest(100, 1, 100, 0.5);
}

test "multi receiver: one hundred eighty" {
    try channelTest(100, 1, 100, 0.8);
}

test "multi receiver: one hundred hundred" {
    try channelTest(100, 1, 100, 1.0);
}

test "multi receiver: hundred hundred zero" {
    try channelTest(100, 100, 100, 0.0);
}

test "multi receiver: hundred hundred twenty" {
    try channelTest(100, 100, 100, 0.2);
}

test "multi receiver: hundred hundred fifty" {
    try channelTest(100, 100, 100, 0.5);
}

test "multi receiver: hundred hundred eighty" {
    try channelTest(100, 100, 100, 0.8);
}

test "multi receiver: hundred hundred hundred" {
    try channelTest(100, 100, 100, 1.0);
}
