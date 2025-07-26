const std = @import("std");
const Allocator = std.mem.Allocator;
const Thread = std.Thread;
const Mutex = Thread.Mutex;
const Condition = Thread.Condition;
const DoublyLinkedList = std.DoublyLinkedList;

const context = @import("../../context.zig");
const tracing = @import("../tracing.zig");
const Task = @import("Task.zig");

pub var allocator: Allocator = undefined;
pub var mutex: Mutex = .{};
pub var cvar: Condition = .{};
pub var enqueued_tasks: usize = 0;
pub var queue: DoublyLinkedList = .{};
pub var running: bool = false;
pub var should_quit: bool = false;

pub fn init() !void {
    allocator = context.allocator;
}

pub fn deinit() void {
    mutex.lock();
    defer mutex.unlock();
    std.debug.assert(enqueued_tasks == 0);
    std.debug.assert(queue.len() == 0);
    std.debug.assert(!running);
}

pub fn startEventLoop(exit_on_completion: bool) !void {
    {
        mutex.lock();
        defer mutex.unlock();
        if (running) return error.AlreadyRunning;
        running = true;
        should_quit = exit_on_completion;
    }
    executorEventLoop();
}

pub fn startEventLoopThread() !Thread {
    {
        mutex.lock();
        defer mutex.unlock();
        if (running) return error.AlreadyRunning;
        running = true;
        should_quit = false;
    }
    errdefer {
        mutex.lock();
        defer mutex.unlock();
        running = false;
        should_quit = false;
    }

    return Thread.spawn(.{}, executorEventLoop, .{});
}

pub fn stopEventLoop() void {
    {
        mutex.lock();
        defer mutex.unlock();
        std.debug.assert(running);
        should_quit = true;
    }
    cvar.signal();
}

fn executorEventLoop() void {
    tracing.registerThread();
    defer tracing.unregisterThread();

    while (true) {
        mutex.lock();
        defer mutex.unlock();
        if (enqueued_tasks == 0 and should_quit) break;
        const node = queue.popFirst() orelse {
            cvar.wait(&mutex);
            continue;
        };
        const task: *Task = @fieldParentPtr("node", node);

        mutex.unlock();
        task.poll();
        mutex.lock();
    }

    {
        mutex.lock();
        defer mutex.unlock();
        running = false;
    }
}
