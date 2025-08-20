const std = @import("std");
const fs = std.fs;
const log = std.log;
const mem = std.mem;
const heap = std.heap;
const process = std.process;
const fatal = std.process.fatal;
const ArrayList = std.ArrayList;

const c = @import("headers");
const fimo_std = @import("fimo_std");
const tracing = fimo_std.tracing;
const db = tracing.db;

const usage =
    \\Usage: profiler [options] [file]
    \\
    \\   Visualizes a captured trace.
    \\
    \\Supported file types:
    \\                       .ftrdb    Fimo trace database
    \\
    \\Options:
    \\  -h, --help                Print this help and exit
    \\
;

fn errorCallback(errn: c_int, str: [*c]const u8) callconv(.c) void {
    log.err("GLFW Error '{}'': {s}", .{ errn, str });
}

var arena: heap.ArenaAllocator = undefined;
var gpa: mem.Allocator = undefined;

var is_empty: bool = true;
var db_reader: db.DBReader = undefined;
var curr_session: *const db.Session = undefined;

var session_idx: usize = 0;
var show_sessions: bool = false;
var show_messages: bool = false;
var show_events: bool = false;
var show_info: bool = false;

pub fn main() !void {
    arena = .init(heap.page_allocator);
    defer arena.deinit();

    gpa = arena.allocator();
    var args_it = try process.argsWithAllocator(gpa);
    if (!args_it.skip()) @panic("expected self arg");

    var opt_input: ?[]const u8 = null;
    while (args_it.next()) |arg| {
        if (mem.startsWith(u8, arg, "-")) {
            if (mem.eql(u8, arg, "-h") or mem.eql(u8, arg, "--help")) {
                try fs.File.stdout().writeAll(usage);
                process.exit(0);
            } else {
                fatal("unrecognized option: '{s}'", .{arg});
            }
        } else {
            if (opt_input == null)
                opt_input = arg
            else
                fatal("unexpected positional argument: '{s}'", .{arg});
        }
    }
    const input = opt_input orelse fatal("no input file provided", .{});
    const input_extension = fs.path.extension(input);
    if (!mem.eql(u8, input_extension, ".ftrdb"))
        fatal("unsupported file format: '{s}'", .{input_extension});

    db_reader = try db.DBReader.init(input);
    defer db_reader.deinit();

    _ = c.glfwSetErrorCallback(&errorCallback);
    if (c.glfwInit() != c.GLFW_TRUE) {
        return;
    }
    defer c.glfwTerminate();

    c.glfwWindowHint(c.GLFW_CONTEXT_VERSION_MAJOR, 4);
    c.glfwWindowHint(c.GLFW_CONTEXT_VERSION_MINOR, 1);
    c.glfwWindowHint(c.GLFW_OPENGL_PROFILE, c.GLFW_OPENGL_CORE_PROFILE);
    c.glfwWindowHint(c.GLFW_RESIZABLE, c.GL_TRUE);
    c.glfwWindowHint(c.GLFW_SAMPLES, 4);

    var width: c_int = 1600;
    var height: c_int = 900;
    const window = c.glfwCreateWindow(width, height, "Profiler", null, null);
    if (window == null) return;
    defer c.glfwDestroyWindow(window);
    c.glfwMakeContextCurrent(window);

    if (c.gladLoadGL(c.glfwGetProcAddress) == 0) {
        log.err("failed to initialize OpenGL context", .{});
        return;
    }

    const ctx = c.igCreateContext(null);
    defer c.igDestroyContext(ctx);

    const io = c.igGetIO_Nil();
    io.*.IniFilename = "profiler.imgui.ini";
    io.*.ConfigFlags |= c.ImGuiConfigFlags_DockingEnable;

    if (!c.ImGui_ImplGlfw_InitForOpenGL(window, true)) return;
    defer c.ImGui_ImplGlfw_Shutdown();

    if (!c.ImGui_ImplOpenGL3_Init("#version 410 core")) return;
    defer c.ImGui_ImplOpenGL3_Shutdown();

    c.igStyleColorsDark(null);

    c.glfwGetWindowSize(window, &width, &height);
    c.glViewport(0, 0, width, height);

    while (c.glfwWindowShouldClose(window) == 0) {
        c.glfwPollEvents();
        defer c.glfwSwapBuffers(window);
        _ = arena.reset(.retain_capacity);

        c.ImGui_ImplOpenGL3_NewFrame();
        defer c.ImGui_ImplOpenGL3_RenderDrawData(c.igGetDrawData());

        c.ImGui_ImplGlfw_NewFrame();
        c.igNewFrame();
        defer {
            c.glfwGetWindowSize(window, &width, &height);
            c.glViewport(0, 0, width, height);
            c.glClear(c.GL_COLOR_BUFFER_BIT);
            c.glClearColor(0.0, 0.0, 0.0, 0.0);
            c.igRender();
        }

        viewMainMenuBar();
        viewMainView();

        if (show_sessions) viewSessions();
        if (show_messages) viewMessages();
        if (show_events) viewEvents();
        if (show_info) viewInfo();

        c.igShowDemoWindow(null);
    }
}

const ImGui = struct {
    fn text(string: []const u8) void {
        c.igTextEx(string.ptr, string.ptr + string.len, 0);
    }

    fn textFmt(comptime fmt: []const u8, args: anytype) void {
        const msg = std.fmt.allocPrint(gpa, fmt, args) catch @panic("oom");
        text(msg);
    }
};

fn viewMainMenuBar() void {
    if (!c.igBeginMainMenuBar()) return;
    defer c.igEndMainMenuBar();

    c.igAlignTextToFramePadding();

    const pressed_button_color: c.ImVec4 = .{ .x = 0.0, .y = 0.6, .z = 0.0, .w = 1.0 };
    const pressed_button_hovered_color: c.ImVec4 = .{ .x = 0.0, .y = 0.7, .z = 0.0, .w = 1.0 };
    const pressed_button_active_color: c.ImVec4 = .{ .x = 0.0, .y = 0.5, .z = 0.0, .w = 1.0 };

    {
        const old = show_sessions;
        if (old) {
            c.igPushStyleColor_Vec4(c.ImGuiCol_Button, pressed_button_color);
            c.igPushStyleColor_Vec4(c.ImGuiCol_ButtonHovered, pressed_button_hovered_color);
            c.igPushStyleColor_Vec4(c.ImGuiCol_ButtonActive, pressed_button_active_color);
        }
        defer if (old) c.igPopStyleColor(3);
        if (c.igButton("Sessions", .{})) show_sessions = !show_sessions;
    }
    {
        const old = show_messages;
        if (old) {
            c.igPushStyleColor_Vec4(c.ImGuiCol_Button, pressed_button_color);
            c.igPushStyleColor_Vec4(c.ImGuiCol_ButtonHovered, pressed_button_hovered_color);
            c.igPushStyleColor_Vec4(c.ImGuiCol_ButtonActive, pressed_button_active_color);
        }
        defer if (old) c.igPopStyleColor(3);
        if (c.igButton("Messages", .{})) show_messages = !show_messages;
    }
    {
        const old = show_events;
        if (old) {
            c.igPushStyleColor_Vec4(c.ImGuiCol_Button, pressed_button_color);
            c.igPushStyleColor_Vec4(c.ImGuiCol_ButtonHovered, pressed_button_hovered_color);
            c.igPushStyleColor_Vec4(c.ImGuiCol_ButtonActive, pressed_button_active_color);
        }
        defer if (old) c.igPopStyleColor(3);
        if (c.igButton("Events", .{})) show_events = !show_events;
    }
    {
        const old = show_info;
        if (old) {
            c.igPushStyleColor_Vec4(c.ImGuiCol_Button, pressed_button_color);
            c.igPushStyleColor_Vec4(c.ImGuiCol_ButtonHovered, pressed_button_hovered_color);
            c.igPushStyleColor_Vec4(c.ImGuiCol_ButtonActive, pressed_button_active_color);
        }
        defer if (old) c.igPopStyleColor(3);
        if (c.igButton("Info", .{})) show_info = !show_info;
    }

    const num_sessions = db_reader.getAllSessions().len;
    is_empty = num_sessions == 0;
    if (is_empty) return;

    if (c.igArrowButton("##prev_session", c.ImGuiDir_Left) and session_idx > 0) session_idx -= 1;
    c.igText("Session: %zu of %zu", session_idx + 1, num_sessions);
    if (c.igArrowButton("##next_session", c.ImGuiDir_Right) and session_idx < num_sessions - 1) session_idx += 1;

    curr_session = db_reader.getSession(session_idx);
    const name = db_reader.getInternedSlice(u8, curr_session.app_name);
    c.igSeparator();
    c.igText("Name: %.*s", @as(c_int, @intCast(name.len)), name.ptr);
    c.igSeparator();
    c.igText("Number of events: %zu", @as(usize, @intCast(curr_session.num_events)));
}

fn viewSessions() void {
    if (is_empty) return;
    defer c.igEnd();
    if (!c.igBegin("Sessions", null, 0)) return;
}

fn viewMessages() void {
    if (is_empty) return;
    defer c.igEnd();
    if (!c.igBegin("Messages", null, 0)) return;

    const events = db_reader.getSessionEvents(session_idx);
    var msg_indices: ArrayList(usize) = .empty;
    for (events, 0..) |ev, i| if (ev.tag == .log_message) msg_indices.append(gpa, i) catch @panic("oom");

    c.igText("Message count: %zu", msg_indices.items.len);
    const flags = c.ImGuiTableFlags_Resizable | c.ImGuiTableFlags_ScrollY | c.ImGuiTableFlags_Borders;
    const outer_size: c.ImVec2 = .{};
    defer c.igEndTable();
    if (c.igBeginTable("##messages", 5, flags, outer_size, 0.0)) {
        c.igTableSetupScrollFreeze(0, 1); // Make top row always visible
        c.igTableSetupColumn("Time", c.ImGuiTableColumnFlags_None, 0.0, 0);
        c.igTableSetupColumn("Target", c.ImGuiTableColumnFlags_None, 0.0, 0);
        c.igTableSetupColumn("Scope", c.ImGuiTableColumnFlags_None, 0.0, 0);
        c.igTableSetupColumn("Level", c.ImGuiTableColumnFlags_None, 0.0, 0);
        c.igTableSetupColumn("Message", c.ImGuiTableColumnFlags_None, 0.0, 0);
        c.igTableHeadersRow();

        const clipper = c.ImGuiListClipper_ImGuiListClipper();
        defer c.ImGuiListClipper_destroy(clipper);
        c.ImGuiListClipper_Begin(clipper, @intCast(msg_indices.items.len), c.igGetTextLineHeightWithSpacing());
        while (c.ImGuiListClipper_Step(clipper)) {
            const start: usize = @intCast(clipper.*.DisplayStart);
            const end: usize = @intCast(clipper.*.DisplayEnd);
            for (msg_indices.items[start..end]) |msg_idx| {
                const msg = events[msg_idx].bitCast(db.LogMessage);
                const extra = db_reader.getInternedValue(db.LogMessageExt, msg.extra);
                const info = db_reader.getEventInfoById(extra.info).?;

                c.igTableNextRow(0, c.igGetTextLineHeightWithSpacing());
                _ = c.igTableSetColumnIndex(0);
                ImGui.textFmt("{}ns", .{@intFromEnum(msg.time) - @intFromEnum(curr_session.start_time)});

                _ = c.igTableSetColumnIndex(1);
                ImGui.text(db_reader.getInternedSlice(u8, info.target));

                _ = c.igTableSetColumnIndex(2);
                ImGui.text(db_reader.getInternedSlice(u8, info.scope));

                _ = c.igTableSetColumnIndex(3);
                const color: c.ImVec4 = switch (info.level) {
                    .err => .{ .x = 205.0 / 255.0, .w = 1.0 },
                    .warn => .{ .x = 205.0 / 255.0, .y = 205.0 / 255.0, .w = 1.0 },
                    .info => .{ .y = 205.0 / 255.0, .w = 1.0 },
                    .debug => .{ .z = 238.0 / 255.0, .w = 1.0 },
                    .off, .trace => .{ .x = 205.0 / 255.0, .z = 205.0 / 255.0, .w = 1.0 },
                };
                const level = switch (info.level) {
                    .err => "Error",
                    .warn => "Warn",
                    .info => "Info",
                    .debug => "Debug",
                    .off, .trace => "Trace",
                };

                c.igPushStyleColor_Vec4(c.ImGuiCol_Text, color);
                ImGui.text(level);
                c.igPopStyleColor(1);

                _ = c.igTableSetColumnIndex(4);
                ImGui.text(db_reader.getInternedSlice(u8, extra.message));
            }
        }
    }
}

fn viewEvents() void {
    if (is_empty) return;
    defer c.igEnd();
    if (!c.igBegin("Events", null, 0)) return;

    const events = db_reader.getSessionEvents(session_idx);
    c.igText("Event count: %zu", events.len);
}

fn viewInfo() void {
    if (is_empty) return;
    defer c.igEnd();
    if (!c.igBegin("Info", null, 0)) return;
}

fn viewMainView() void {
    if (is_empty) return;
    const viewport = c.igGetMainViewport();
    c.igSetNextWindowPos(viewport.*.WorkPos, 0, .{});
    c.igSetNextWindowSize(viewport.*.WorkSize, 0);
    c.igSetNextWindowViewport(viewport.*.ID);

    _ = c.igBegin("Profiler", null, c.ImGuiWindowFlags_NoDecoration |
        c.ImGuiWindowFlags_NoBringToFrontOnFocus);
    defer c.igEnd();
}
