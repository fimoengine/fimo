const std = @import("std");
const Allocator = std.mem.Allocator;
const Thread = std.Thread;
const Mutex = Thread.Mutex;
const Condition = Thread.Condition;

const Context = @import("../../context.zig");
const Async = @import("../async.zig");
const Task = @import("Task.zig");

const Self = @This();

allocator: Allocator,
mutex: Mutex = .{},
cvar: Condition = .{},
queue: Task.TaskQueue = .{},
running: bool = false,
should_quit: bool = false,

pub fn init(ctx: *Context) !Self {
    return .{ .allocator = ctx.allocator };
}

pub fn deinit(self: *Self) void {
    self.mutex.lock();
    defer self.mutex.unlock();
    std.debug.assert(self.queue.len == 0);
    std.debug.assert(!self.running);
}

pub fn asContext(self: *Self) *Context {
    const @"async": *Async = @fieldParentPtr("sys", self);
    return @"async".asContext();
}

pub fn startEventLoop(self: *Self, exit_on_completion: bool) !void {
    {
        self.mutex.lock();
        defer self.mutex.unlock();
        if (self.running) return error.AlreadyRunning;
        self.running = true;
        self.should_quit = exit_on_completion;
    }
    self.executorEventLoop();
}

pub fn startEventLoopThread(self: *Self) !Thread {
    {
        self.mutex.lock();
        defer self.mutex.unlock();
        if (self.running) return error.AlreadyRunning;
        self.running = true;
        self.should_quit = false;
    }
    errdefer {
        self.mutex.lock();
        defer self.mutex.unlock();
        self.running = false;
        self.should_quit = false;
    }

    const f = struct {
        fn f(this: *Self) !void {
            this.asContext().ref();
            defer this.asContext().unref();

            this.asContext().tracing.registerThread();
            defer this.asContext().tracing.unregisterThread();
            this.executorEventLoop();
        }
    }.f;
    return Thread.spawn(.{}, f, .{self});
}

pub fn stopEventLoop(self: *Self) void {
    {
        self.mutex.lock();
        defer self.mutex.unlock();
        std.debug.assert(self.running);
        self.should_quit = true;
    }
    self.cvar.signal();
}

fn executorEventLoop(self: *Self) void {
    while (true) {
        self.mutex.lock();
        defer self.mutex.unlock();
        if (self.queue.len == 0 and self.should_quit) break;
        const task = self.queue.popFirst() orelse {
            self.cvar.wait(&self.mutex);
            continue;
        };

        self.mutex.unlock();
        task.data.poll();
        self.mutex.lock();
    }

    {
        self.mutex.lock();
        defer self.mutex.unlock();
        self.running = false;
    }
}
