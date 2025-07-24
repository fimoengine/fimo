const std = @import("std");

const fimo_std = @import("fimo_std");
const ctx = fimo_std.ctx;
const tracing = fimo_std.tracing;
const tasks = fimo_std.tasks;
const AnyError = fimo_std.AnyError;

pub fn main() !void {
    const tracing_cfg = tracing.Config{
        .max_level = .trace,
        .subscribers = &.{tracing.default_subscriber},
        .subscriber_count = 1,
    };
    defer tracing_cfg.deinit();
    const init_options: [:null]const ?*const ctx.ConfigHead = &.{@ptrCast(&tracing_cfg)};
    try ctx.init(init_options);
    defer ctx.deinit();

    var err: ?AnyError = null;
    defer if (err) |e| e.deinit();
    defer tasks.EventLoop.flushWithCurrentThread(&err) catch unreachable;

    tracing.registerThread();
    defer tracing.unregisterThread();

    const event_loop = try tasks.EventLoop.init(&err);
    defer event_loop.join();

    const async_ctx = try tasks.BlockingContext.init(&err);
    defer async_ctx.deinit();

    const ab: NestedFuture.Result = (try NestedFuture.init(&err)).awaitBlocking(async_ctx);
    const a = ab.a;
    const b = ab.b;

    try std.testing.expectEqual(NestedFuture.a_iter, a);
    try std.testing.expectEqual(NestedFuture.b_iter, b);
}

const NestedFuture = union(enum) {
    start: struct {
        a: tasks.EnqueuedFuture(usize),
        b: tasks.EnqueuedFuture(usize),
    },
    stage_0: struct {
        a: usize,
        b: tasks.EnqueuedFuture(usize),
    },
    stage_1: struct {
        a: usize,
        b: usize,
    },
    stop: struct {},

    const Result = struct { a: usize, b: usize };
    const Future = tasks.Future(@This(), Result, poll, deinit);

    const a_iter = 5;
    const b_iter = 10;

    fn init(err: *?AnyError) !Future {
        var a_fut = try LoopFuture(a_iter).init().enqueue(null, err);
        errdefer a_fut.deinit();

        var b_fut = try LoopFuture(b_iter).init().enqueue(null, err);
        errdefer b_fut.deinit();

        return Future.init(.{
            .start = .{
                .a = a_fut,
                .b = b_fut,
            },
        });
    }

    fn deinit(self: *@This()) void {
        switch (self.*) {
            .start => |*x| {
                x.b.deinit();
                x.a.deinit();
            },
            .stage_0 => |*x| {
                x.b.deinit();
            },
            .stage_1 => |_| {},
            .stop => {},
        }
    }

    fn poll(self: *@This(), waker: tasks.Waker) tasks.Poll(Result) {
        switch (self.*) {
            .start => |*x| {
                tracing.emitTraceSimple("Polled state=`{any}`", .{self}, @src());

                const a = switch (x.a.poll(waker)) {
                    .pending => return .pending,
                    .ready => |v| v,
                };
                x.a.deinit();
                const b = x.b;

                self.* = .{ .stage_0 = .{ .a = a, .b = b } };
                waker.wake();
                return .pending;
            },
            .stage_0 => |*x| {
                tracing.emitTraceSimple("Polled state=`{any}`", .{self}, @src());

                const b = switch (x.b.poll(waker)) {
                    .pending => return .pending,
                    .ready => |v| v,
                };
                x.b.deinit();
                const a = x.a;

                self.* = .{ .stage_1 = .{ .a = a, .b = b } };
                waker.wake();
                return .pending;
            },
            .stage_1 => |*x| {
                tracing.emitTraceSimple("Polled state=`{any}`", .{self}, @src());

                const a = x.a;
                const b = x.b;

                self.* = .{ .stop = .{} };
                return .{ .ready = .{ .a = a, .b = b } };
            },
            .stop => unreachable,
        }
    }
};

fn LoopFuture(comptime iterations: usize) type {
    return struct {
        i: usize = 0,

        const Result = usize;
        const Future = tasks.Future(@This(), Result, poll, null);

        fn init() Future {
            return Future.init(.{});
        }

        fn poll(self: *@This(), waker: tasks.Waker) tasks.Poll(usize) {
            tracing.emitTraceSimple(
                "Iteration i='{}', data=`{*}`",
                .{ self.i, self },
                @src(),
            );

            self.i += 1;
            if (self.i < iterations) {
                waker.wake();
                return .pending;
            }

            return .{ .ready = self.i };
        }
    };
}
