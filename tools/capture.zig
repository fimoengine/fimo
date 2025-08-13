const std = @import("std");
const mem = std.mem;
const fs = std.fs;
const process = std.process;
const fatal = std.process.fatal;

const fimo_std = @import("fimo_std");
const tracing = fimo_std.tracing;
const db = tracing.db;
const net = tracing.net;
const default_host = net.protocol.default_host;
const default_port = net.protocol.default_port;

const usage =
    \\Usage: capture [options]
    \\
    \\   Captures a profiling trace using the network trace protocol.
    \\
    \\Options:
    \\   --host [string]        Host name to connect to (default: '127.0.0.1')
    \\   --port [number]        Port to connect to (default: '5882')
    \\   --output [path]        Output trace file path (default: 'trace.ftrdb')
    \\   -h, --help             Print this help and exit
    \\
;

const default_output = "trace.ftrdb";

pub fn main() !void {
    var arena = std.heap.ArenaAllocator.init(std.heap.page_allocator);
    defer arena.deinit();

    const gpa = arena.allocator();
    var args_it = try process.argsWithAllocator(gpa);
    if (!args_it.skip()) @panic("expected self arg");

    var opt_host: ?[]const u8 = null;
    var opt_port: ?[]const u8 = null;
    var opt_output: ?[]const u8 = null;
    while (args_it.next()) |arg| {
        if (mem.startsWith(u8, arg, "-")) {
            if (mem.eql(u8, arg, "-h") or mem.eql(u8, arg, "--help")) {
                try fs.File.stdout().writeAll(usage);
                process.exit(0);
            } else if (mem.eql(u8, arg, "--host")) {
                if (args_it.next()) |param| {
                    opt_host = param;
                } else {
                    fatal("expected parameter after --host", .{});
                }
            } else if (mem.eql(u8, arg, "--port")) {
                if (args_it.next()) |param| {
                    opt_port = param;
                } else {
                    fatal("expected parameter after --port", .{});
                }
            } else if (mem.eql(u8, arg, "--host")) {
                if (args_it.next()) |param| {
                    opt_output = param;
                } else {
                    fatal("expected parameter after --output", .{});
                }
            } else {
                fatal("unrecognized option: '{s}'", .{arg});
            }
        } else {
            fatal("unexpected positional argument: '{s}'", .{arg});
        }
    }

    const host = opt_host orelse default_host;
    const port = if (opt_port) |p|
        std.fmt.parseInt(u16, p, 10) catch fatal("--port expects a number", .{})
    else
        default_port;
    const output = opt_output orelse default_output;

    var writer = try db.DBWriter.init(output);
    defer writer.deinit();

    std.log.info("connecting...", .{});
    var client: net.Client = blk: while (true) {
        break :blk net.Client.init(.{
            .gpa = gpa,
            .host_name = host,
            .port = port,
        }) catch |err| {
            std.log.err("{t}", .{err});
            std.log.info("retrying...", .{});
            continue;
        };
    };
    defer client.close();
    defer std.log.info("closing", .{});
    std.log.info("...connected", .{});

    while (try client.readEvent()) |event| {
        defer event.deinit(gpa);
        switch (event) {
            .start => |v| try writer.startSession(
                @enumFromInt(v.main.time),
                @enumFromInt(v.main.epoch),
                @enumFromInt(v.main.resolution),
                v.main.available_memory,
                v.main.process_id,
                v.main.num_cores,
                v.main.cpu_arch,
                v.main.cpu_id,
                v.getCpuVendor(),
                v.getAppName(),
                v.getHostInfo(),
            ),
            .finish => |v| try writer.finishSession(@enumFromInt(v.time)),
            .register_thread => |v| try writer.registerThread(
                @enumFromInt(v.time),
                @enumFromInt(v.thread_id),
            ),
            .unregister_thread => |v| try writer.unregisterThread(
                @enumFromInt(v.time),
                @enumFromInt(v.thread_id),
            ),
            .create_call_stack => |v| try writer.createCallStack(
                @enumFromInt(v.time),
                @enumFromInt(v.stack),
            ),
            .destroy_call_stack => |v| try writer.destroyCallStack(
                @enumFromInt(v.time),
                @enumFromInt(v.stack),
            ),
            .unblock_call_stack => |v| try writer.unblockCallStack(
                @enumFromInt(v.time),
                @enumFromInt(v.stack),
            ),
            .suspend_call_stack => |v| try writer.suspendCallStack(
                @enumFromInt(v.time),
                @enumFromInt(v.stack),
                v.mark_blocked,
            ),
            .resume_call_stack => |v| try writer.resumeCallStack(
                @enumFromInt(v.time),
                @enumFromInt(v.stack),
                @enumFromInt(v.thread_id),
            ),
            .enter_span => |v| try writer.enterSpan(
                @enumFromInt(v.main.time),
                @enumFromInt(v.main.stack),
                @enumFromInt(v.main.info_id),
                v.getMessage(),
            ),
            .exit_span => |v| try writer.exitSpan(
                @enumFromInt(v.time),
                @enumFromInt(v.stack),
                v.is_unwinding,
            ),
            .log_message => |v| try writer.logMessage(
                @enumFromInt(v.main.time),
                @enumFromInt(v.main.stack),
                @enumFromInt(v.main.info_id),
                v.getMessage(),
            ),
            .declare_event_info => |v| _ = try writer.internEventInfo(
                @enumFromInt(v.main.id),
                v.getName(),
                v.getTarget(),
                v.getScope(),
                v.getFileName(),
                if (v.main.line_number < 0) std.math.maxInt(u32) else @intCast(v.main.line_number),
                v.main.level,
            ),
        }
    }
}
