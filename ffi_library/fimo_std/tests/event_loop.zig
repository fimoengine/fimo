const std = @import("std");

const fimo_std = @import("fimo_std");

const AnyError = fimo_std.AnyError;

const Context = fimo_std.Context;
const Tracing = Context.Tracing;
const Async = Context.Async;

pub fn main() !void {
    const tracing_cfg = Tracing.Config{
        .max_level = .trace,
        .subscribers = &.{Tracing.default_subscriber},
        .subscriber_count = 1,
    };
    defer tracing_cfg.deinit();
    const init_options: [:null]const ?*const Context.TaggedInStruct = &.{@ptrCast(&tracing_cfg)};

    const ctx = try Context.init(init_options);
    defer ctx.unref();

    var err: ?AnyError = null;
    defer if (err) |e| e.deinit();
    defer Async.EventLoop.flushWithCurrentThread(ctx.@"async"(), &err) catch unreachable;

    ctx.tracing().registerThread();
    defer ctx.tracing().unregisterThread();

    const event_loop = try Async.EventLoop.init(ctx.@"async"(), &err);
    defer event_loop.join();

    const async_ctx = try Async.BlockingContext.init(ctx.@"async"(), &err);
    defer async_ctx.deinit();

    const ab: NestedFuture.Result = (try NestedFuture.init(ctx, &err))
        .awaitBlocking(async_ctx);
    const a = ab.a;
    const b = ab.b;

    try std.testing.expectEqual(NestedFuture.a_iter, a);
    try std.testing.expectEqual(NestedFuture.b_iter, b);
}

const NestedFuture = union(enum) {
    start: struct {
        a: Async.EnqueuedFuture(usize),
        b: Async.EnqueuedFuture(usize),
        ctx: Context,
    },
    stage_0: struct {
        a: usize,
        b: Async.EnqueuedFuture(usize),
        ctx: Context,
    },
    stage_1: struct {
        a: usize,
        b: usize,
        ctx: Context,
    },
    stop: struct {},

    const Result = struct { a: usize, b: usize };
    const Future = Async.Future(@This(), Result, poll, deinit);

    const a_iter = 5;
    const b_iter = 10;

    fn init(ctx: Context, err: *?AnyError) !Future {
        var a_fut = try LoopFuture(a_iter).init(ctx).enqueue(
            ctx.@"async"(),
            null,
            err,
        );
        errdefer a_fut.deinit();

        var b_fut = try LoopFuture(b_iter).init(ctx).enqueue(
            ctx.@"async"(),
            null,
            err,
        );
        errdefer b_fut.deinit();

        ctx.ref();
        return Future.init(.{
            .start = .{
                .a = a_fut,
                .b = b_fut,
                .ctx = ctx,
            },
        });
    }

    fn deinit(self: *@This()) void {
        switch (self.*) {
            .start => |*x| {
                x.b.deinit();
                x.a.deinit();
                x.ctx.unref();
            },
            .stage_0 => |*x| {
                x.b.deinit();
                x.ctx.unref();
            },
            .stage_1 => |*x| {
                x.ctx.unref();
            },
            .stop => {},
        }
    }

    fn poll(self: *@This(), waker: Async.Waker) Async.Poll(Result) {
        switch (self.*) {
            .start => |*x| {
                x.ctx.tracing().emitTraceSimple("Polled state=`{any}`", .{self}, @src());

                const a = switch (x.a.poll(waker)) {
                    .pending => return .pending,
                    .ready => |v| v,
                };
                x.a.deinit();
                const b = x.b;
                const ctx = x.ctx;

                self.* = .{ .stage_0 = .{ .a = a, .b = b, .ctx = ctx } };
                waker.wake();
                return .pending;
            },
            .stage_0 => |*x| {
                x.ctx.tracing().emitTraceSimple("Polled state=`{any}`", .{self}, @src());

                const b = switch (x.b.poll(waker)) {
                    .pending => return .pending,
                    .ready => |v| v,
                };
                x.b.deinit();
                const a = x.a;
                const ctx = x.ctx;

                self.* = .{ .stage_1 = .{ .a = a, .b = b, .ctx = ctx } };
                waker.wake();
                return .pending;
            },
            .stage_1 => |*x| {
                x.ctx.tracing().emitTraceSimple("Polled state=`{any}`", .{self}, @src());

                const a = x.a;
                const b = x.b;
                const ctx = x.ctx;
                ctx.unref();

                self.* = .{ .stop = .{} };
                return .{ .ready = .{ .a = a, .b = b } };
            },
            .stop => unreachable,
        }
    }
};

fn LoopFuture(comptime iterations: usize) type {
    return struct {
        i: usize,
        ctx: Context,

        const Result = usize;
        const Future = Async.Future(@This(), Result, poll, deinit);

        fn init(ctx: Context) Future {
            ctx.ref();
            return Future.init(.{ .i = 0, .ctx = ctx });
        }

        fn deinit(self: *@This()) void {
            self.ctx.unref();
        }

        fn poll(self: *@This(), waker: Async.Waker) Async.Poll(usize) {
            self.ctx.tracing().emitTraceSimple(
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
