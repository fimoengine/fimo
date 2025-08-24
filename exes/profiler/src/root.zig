const std = @import("std");
const fs = std.fs;
const log = std.log;
const mem = std.mem;
const Allocator = mem.Allocator;
const heap = std.heap;
const process = std.process;
const fatal = std.process.fatal;
const ArrayList = std.ArrayList;
const AutoArrayHashMap = std.AutoArrayHashMapUnmanaged;

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

var tmp_arena: heap.ArenaAllocator = undefined;
var persistent_arena: heap.ArenaAllocator = undefined;

var is_empty: bool = true;
var is_initialized: bool = false;
var db_reader: db.DBReader = undefined;
var curr_session: *const db.Session = undefined;

var cache: Cache = .{};

var session_idx: usize = 0;
var show_sessions: bool = false;
var show_messages: bool = false;
var show_events: bool = false;
var show_info: bool = false;

pub fn main() !void {
    tmp_arena = .init(heap.page_allocator);
    persistent_arena = .init(heap.page_allocator);

    var args_it = try process.argsWithAllocator(tmp_arena.allocator());
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

    var store: Store = .{};
    store.populate(&db_reader);

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
    c.glfwSwapInterval(1);

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
        _ = tmp_arena.reset(.retain_capacity);

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
        viewMainView(&store);

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
        const msg = std.fmt.allocPrint(tmp_arena.allocator(), fmt, args) catch @panic("oom");
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

    var changed = !is_initialized;
    if (c.igArrowButton("##prev_session", c.ImGuiDir_Left) and session_idx > 0) {
        session_idx -= 1;
        changed = true;
    }
    c.igText("Session: %zu of %zu", session_idx + 1, num_sessions);
    if (c.igArrowButton("##next_session", c.ImGuiDir_Right) and session_idx < num_sessions - 1) {
        session_idx += 1;
        changed = true;
    }

    curr_session = db_reader.getSession(session_idx);
    if (changed) {
        const start_time = @intFromEnum(curr_session.start_time);
        const end_time = @intFromEnum(curr_session.end_time);
        cache.setTimeRange(start_time, end_time);
    }

    const name = db_reader.getInternedSlice(u8, curr_session.app_name);
    c.igSeparator();
    c.igText("Name: %.*s", @as(c_int, @intCast(name.len)), name.ptr);
    c.igSeparator();
    c.igText("Number of events: %zu", @as(usize, @intCast(curr_session.num_events)));

    is_initialized = true;
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
    for (events, 0..) |ev, i| if (ev.tag == .log_message)
        msg_indices.append(tmp_arena.allocator(), i) catch @panic("oom");

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
    ImGui.textFmt("Event count: {}", .{events.len});

    var event_counts: std.EnumArray(db.EventTag, usize) = .initFill(0);
    for (events) |event| {
        event_counts.getPtr(event.tag).* += 1;
    }

    const values = comptime std.enums.values(db.EventTag);
    inline for (values) |tag| {
        const count = event_counts.get(tag);
        ImGui.textFmt("`{t}` count: {}", .{ tag, count });
    }
}

fn viewInfo() void {
    if (is_empty) return;
    defer c.igEnd();
    if (!c.igBegin("Info", null, 0)) return;
}

fn viewMainView(store: *const Store) void {
    if (is_empty) return;
    const viewport = c.igGetMainViewport();
    c.igSetNextWindowPos(viewport.*.WorkPos, 0, .{});
    c.igSetNextWindowSize(viewport.*.WorkSize, 0);
    c.igSetNextWindowViewport(viewport.*.ID);

    c.igPushStyleVar_Float(c.ImGuiStyleVar_WindowRounding, 0.0);
    defer c.igPopStyleVar(1);

    defer c.igEnd();
    if (!c.igBegin(
        "Profiler",
        null,
        c.ImGuiWindowFlags_NoDecoration |
            c.ImGuiWindowFlags_NoBringToFrontOnFocus |
            c.ImGuiWindowFlags_NoResize,
    )) return;

    var time_span = cache.end_time - cache.start_time;
    if (c.igIsWindowHovered(0)) {
        const io = c.igGetIO_Nil();
        if (io.*.MouseWheel != 0.0) {
            const t = (io.*.MousePos.x - viewport.*.WorkPos.x) / viewport.*.WorkSize.x;
            const time_at_pos: f32 = std.math.lerp(
                @as(f32, @floatFromInt(cache.start_time)),
                @as(f32, @floatFromInt(cache.end_time)),
                t,
            );

            const new_time_span_f: f32 = if (io.*.MouseWheel > 0.0)
                @max(1.0, @as(f32, @floatFromInt(time_span)) * 0.75)
            else
                @max(1.0, @as(f32, @floatFromInt(time_span)) / 0.75);

            time_span = @intFromFloat(new_time_span_f);
            const new_start: u64 = @intFromFloat(@max(0.0, time_at_pos - (t * new_time_span_f)));
            const new_end = new_start + time_span;
            cache.setTimeRange(new_start, new_end);
        }

        if (io.*.MouseDown[1]) {
            const delta = io.*.MouseDelta;
            if (delta.x != 0.0) {
                const window_width: u64 = @intFromFloat(viewport.*.WorkSize.x);
                const time_per_pixel = time_span / window_width;
                const offset = @as(u64, @intFromFloat(@abs(delta.x))) * time_per_pixel;
                if (delta.x > 0.0) {
                    const new_start = cache.start_time - offset;
                    const new_end = cache.end_time - offset;
                    cache.setTimeRange(new_start, new_end);
                } else {
                    const new_start = cache.start_time + offset;
                    const new_end = cache.end_time + offset;
                    cache.setTimeRange(new_start, new_end);
                }
            }
        }
    }

    cache.ensureReady(store);
    const events = db_reader.getAllEvents();

    const style = c.igGetStyle();
    const block_width = viewport.*.WorkSize.x - (style.*.WindowPadding.x * 2);
    const block_height = c.igGetTextLineHeight() + (style.*.FramePadding.y * 2);

    c.igBeginGroup();
    defer c.igEndGroup();
    for (cache.stacks.items, 0..) |stack, i| {
        c.igPushID_Int(@intCast(i));
        defer c.igPopID();

        var max_depth: usize = 0;
        for (stack.spans) |span| max_depth = @max(max_depth, span.depth);

        var cursor_pos: c.ImVec2 = undefined;
        c.igGetCursorPos(&cursor_pos);

        const label_pos = cursor_pos;
        const separator_start_pos: c.ImVec2 = .{
            .x = label_pos.x,
            .y = label_pos.y + c.igGetTextLineHeightWithSpacing(),
        };
        const separator_end_pos: c.ImVec2 = .{
            .x = separator_start_pos.x + block_width,
            .y = separator_start_pos.y,
        };

        const activity_line_start_x = label_pos.x;
        const activity_line_end_x = label_pos.x + block_width;
        const activity_line_y = separator_start_pos.y + style.*.FramePadding.y + 5;
        const running_color = c.igGetColorU32_Vec4(.{
            .x = 0.0 / 255.0,
            .y = 255.0 / 255.0,
            .z = 0.0 / 255.0,
            .w = 1.0,
        });
        const suspended_color = c.igGetColorU32_Vec4(.{
            .x = 128.0 / 255.0,
            .y = 128.0 / 255.0,
            .z = 128.0 / 255.0,
            .w = 1.0,
        });
        const blocked_color = c.igGetColorU32_Vec4(.{
            .x = 255.0 / 255.0,
            .y = 0.0 / 255.0,
            .z = 0.0 / 255.0,
            .w = 1.0,
        });

        const frame_height = @as(f32, @floatFromInt(max_depth + 2)) * block_height;
        const frame_bb: c.ImRect = .{ .Min = cursor_pos, .Max = .{ .x = cursor_pos.x + block_width, .y = cursor_pos.y + frame_height } };
        c.igItemSize_Rect(frame_bb, style.*.FramePadding.y);
        if (!c.igItemAdd(frame_bb, 0, &frame_bb, 0)) continue;

        const draw_list = c.igGetWindowDrawList();
        const label = std.fmt.allocPrint(tmp_arena.allocator(), "{}", .{@intFromEnum(stack.ref)}) catch @panic("OOM");
        c.igRenderText(label_pos, label.ptr, label.ptr + label.len, false);
        c.ImDrawList_AddLine(
            draw_list,
            separator_start_pos,
            separator_end_pos,
            c.igGetColorU32_Col(c.ImGuiCol_Text, 0.8),
            1.0,
        );

        for (stack.activity) |activity| {
            const start_time = std.math.clamp(
                @intFromEnum(events[activity.start_event].time),
                cache.start_time,
                cache.end_time,
            );
            const end_time = std.math.clamp(
                @intFromEnum(events[activity.end_event].time),
                cache.start_time,
                cache.end_time,
            );
            const start_x = std.math.lerp(
                activity_line_start_x,
                activity_line_end_x,
                @as(f32, @floatFromInt(start_time - cache.start_time)) /
                    @as(f32, @floatFromInt(time_span)),
            );
            const end_x = std.math.lerp(
                activity_line_start_x,
                activity_line_end_x,
                @as(f32, @floatFromInt(end_time - cache.start_time)) /
                    @as(f32, @floatFromInt(time_span)),
            );
            const color = switch (activity.status) {
                .running => running_color,
                .suspended => suspended_color,
                .blocked => blocked_color,
            };
            c.ImDrawList_AddLine(
                draw_list,
                .{ .x = start_x, .y = activity_line_y },
                .{ .x = end_x, .y = activity_line_y },
                color,
                1.0,
            );
        }
    }
}

const Store = struct {
    arena: heap.ArenaAllocator = .init(heap.page_allocator),
    stacks: AutoArrayHashMap(db.StackRef, StackInfo) = .empty,

    fn populate(self: *Store, reader: *const db.DBReader) void {
        const gpa = self.arena.allocator();
        for (reader.getAllEvents(), 0..) |op_event, i| {
            switch (op_event.tag) {
                .register_thread => {},
                .unregister_thread => {},
                .create_call_stack => {
                    const event = op_event.bitCast(db.CreateCallStack);
                    const stack = self.stacks.getOrPut(gpa, event.stack) catch @panic("OOM");
                    if (!stack.found_existing) stack.value_ptr.* = .{ .ref = event.stack };
                    stack.value_ptr.events.append(gpa, i) catch @panic("OOM");
                },
                .destroy_call_stack => {
                    const event = op_event.bitCast(db.DestroyCallStack);
                    const stack = self.stacks.getOrPut(gpa, event.stack) catch @panic("OOM");
                    if (!stack.found_existing) stack.value_ptr.* = .{ .ref = event.stack };
                    stack.value_ptr.events.append(gpa, i) catch @panic("OOM");
                },
                .unblock_call_stack => {
                    const event = op_event.bitCast(db.UnblockCallStack);
                    const stack = self.stacks.getOrPut(gpa, event.stack) catch @panic("OOM");
                    if (!stack.found_existing) stack.value_ptr.* = .{ .ref = event.stack };
                    stack.value_ptr.events.append(gpa, i) catch @panic("OOM");
                },
                .suspend_call_stack => {
                    const event = op_event.bitCast(db.SuspendCallStack);
                    const stack = self.stacks.getOrPut(gpa, event.stack) catch @panic("OOM");
                    if (!stack.found_existing) stack.value_ptr.* = .{ .ref = event.stack };
                    stack.value_ptr.events.append(gpa, i) catch @panic("OOM");
                },
                .resume_call_stack => {
                    const event = op_event.bitCast(db.ResumeCallStack);
                    const stack = self.stacks.getOrPut(gpa, event.stack) catch @panic("OOM");
                    if (!stack.found_existing) stack.value_ptr.* = .{ .ref = event.stack };
                    stack.value_ptr.events.append(gpa, i) catch @panic("OOM");
                },
                .enter_span => {
                    const event = op_event.bitCast(db.EnterSpan);
                    const stack = self.stacks.getOrPut(gpa, event.stack) catch @panic("OOM");
                    if (!stack.found_existing) stack.value_ptr.* = .{ .ref = event.stack };
                    stack.value_ptr.events.append(gpa, i) catch @panic("OOM");
                },
                .exit_span => {
                    const event = op_event.bitCast(db.ExitSpan);
                    const stack = self.stacks.getOrPut(gpa, event.stack) catch @panic("OOM");
                    if (!stack.found_existing) stack.value_ptr.* = .{ .ref = event.stack };
                    stack.value_ptr.events.append(gpa, i) catch @panic("OOM");
                },
                .log_message => {
                    const event = op_event.bitCast(db.LogMessage);
                    const stack = self.stacks.getOrPut(gpa, event.stack) catch @panic("OOM");
                    if (!stack.found_existing) stack.value_ptr.* = .{ .ref = event.stack };
                    stack.value_ptr.events.append(gpa, i) catch @panic("OOM");
                },
                .start_thread => {},
                .stop_thread => {},
                .load_image => {},
                .unload_image => {},
                .context_switch => {},
                .thread_wakeup => {},
                .call_stack_sample => {},
                _ => unreachable,
            }
        }
        for (self.stacks.values()) |*stack| stack.populate(gpa, reader);
    }
};

const StackInfo = struct {
    ref: db.StackRef,
    events: ArrayList(usize) = .empty,
    activity: ArrayList(Activity) = .empty,
    spans: ArrayList(SpanInfo) = .empty,

    const Status = enum {
        running,
        suspended,
        blocked,
    };

    const Activity = struct {
        start_event: usize,
        end_event: usize,
        status: Status,
        thread_id: ?db.ThreadId,
    };

    const SpanInfo = struct {
        start_event: usize,
        end_event: usize,
        depth: usize,
        parent_offset: ?usize,
        num_children: usize,
    };

    fn getActivity(self: *const StackInfo, reader: *const db.DBReader, start_time: u64, end_time: u64) []const Activity {
        const Context = struct {
            events: []const db.OpaqueEvent,
            start_time: u64,
            end_time: u64,

            fn compare(ctx: @This(), activity: Activity) std.math.Order {
                const start = ctx.events[activity.start_event];
                if (@intFromEnum(start.time) > ctx.end_time) return .gt;
                const end = ctx.events[activity.end_event];
                if (@intFromEnum(end.time) < ctx.start_time) return .lt;
                return .eq;
            }
        };
        const ctx: Context = .{
            .events = reader.getAllEvents(),
            .start_time = start_time,
            .end_time = end_time,
        };
        const start, const end = std.sort.equalRange(Activity, self.activity.items, ctx, Context.compare);
        return self.activity.items[start..end];
    }

    fn getSpans(self: *const StackInfo, reader: *const db.DBReader, start_time: u64, end_time: u64) []const SpanInfo {
        var spans = self.spans.items;
        const events = reader.getAllEvents();
        while (spans.len != 0) {
            const first = spans[0];
            const span_start = @intFromEnum(events[first.start_event].time);
            const span_end = @intFromEnum(events[first.end_event].time);
            if (span_end < start_time) {
                spans = spans[1 + first.num_children ..];
                continue;
            }
            if (span_start > end_time) return &.{};
            break;
        }

        loop: while (spans.len != 0) {
            const last = spans[spans.len - 1];
            const span_start = @intFromEnum(events[last.start_event].time);
            if (span_start <= end_time) break;
            const parent_offset = last.parent_offset orelse {
                spans = spans[0 .. spans.len - 1];
                continue;
            };

            var prev_idx = spans.len - 1 - parent_offset;
            while (prev_idx != spans.len - 1) {
                const prev = spans[prev_idx];
                const prev_start = @intFromEnum(events[prev.start_event].time);
                const prev_end = @intFromEnum(events[prev.end_event].time);
                if (prev_start > end_time) {
                    spans = spans[0..prev_idx];
                    continue :loop;
                }

                if (prev_end <= end_time) {
                    prev_idx += prev.num_children;
                } else {
                    prev_idx += 1;
                }
            }
        }

        return spans;
    }

    fn populate(self: *StackInfo, gpa: Allocator, reader: *const db.DBReader) void {
        var current_depth: usize = 0;
        var current_span: ?usize = null;
        var current_status: ?Status = null;
        var current_thread_id: ?db.ThreadId = null;
        const events = reader.getAllEvents();
        for (self.events.items) |ev_idx| {
            const op_event = events[ev_idx];
            switch (op_event.tag) {
                .create_call_stack => {
                    self.activity.append(gpa, .{
                        .start_event = ev_idx,
                        .end_event = undefined,
                        .status = .suspended,
                        .thread_id = null,
                    }) catch @panic("OOM");
                    current_status = .suspended;
                },
                .destroy_call_stack => {
                    const current = &self.activity.items[self.activity.items.len - 1];
                    current.end_event = ev_idx;
                    current_status = null;
                    current_thread_id = null;
                },
                .unblock_call_stack => {
                    const current = &self.activity.items[self.activity.items.len - 1];
                    current.end_event = ev_idx;
                    self.activity.append(gpa, .{
                        .start_event = ev_idx,
                        .end_event = undefined,
                        .status = .suspended,
                        .thread_id = null,
                    }) catch @panic("OOM");
                    current_status = .suspended;
                },
                .suspend_call_stack => {
                    const current = &self.activity.items[self.activity.items.len - 1];
                    current.end_event = ev_idx;
                    const event = op_event.bitCast(db.SuspendCallStack);
                    const status: Status = if (event.flags.mark_blocked) .blocked else .suspended;
                    self.activity.append(gpa, .{
                        .start_event = ev_idx,
                        .end_event = undefined,
                        .status = status,
                        .thread_id = null,
                    }) catch @panic("OOM");
                    current_status = status;
                },
                .resume_call_stack => {
                    const current = &self.activity.items[self.activity.items.len - 1];
                    current.end_event = ev_idx;
                    const event = op_event.bitCast(db.ResumeCallStack);
                    self.activity.append(gpa, .{
                        .start_event = ev_idx,
                        .end_event = undefined,
                        .status = .running,
                        .thread_id = event.thread_id,
                    }) catch @panic("OOM");
                    current_status = .running;
                },
                .enter_span => {
                    self.spans.append(gpa, .{
                        .start_event = ev_idx,
                        .end_event = undefined,
                        .depth = current_depth,
                        .parent_offset = if (current_span) |sp| (self.spans.items.len + 1) - sp else null,
                        .num_children = 0,
                    }) catch @panic("OOM");

                    var chain = current_span;
                    while (chain) |parent_idx| {
                        const parent = &self.spans.items[parent_idx];
                        parent.num_children += 1;
                        const parent_offset = parent.parent_offset orelse break;
                        chain = parent_idx - parent_offset;
                    }

                    current_depth += 1;
                    current_span = self.spans.items.len - 1;
                },
                .exit_span => {
                    const curr_idx = current_span.?;
                    const current = &self.spans.items[curr_idx];
                    current.end_event = ev_idx;
                    current_depth -= 1;
                    current_span = if (current.parent_offset) |off| curr_idx - off else null;
                },
                .log_message => {},
                else => unreachable,
            }
        }
    }
};

const Cache = struct {
    dirty: bool = true,
    start_time: u64 = 0,
    end_time: u64 = 0,
    arena: heap.ArenaAllocator = .init(heap.page_allocator),

    stacks: ArrayList(StackCache) = .empty,

    const StackCache = struct {
        ref: db.StackRef,
        activity: []const StackInfo.Activity,
        spans: []const StackInfo.SpanInfo,
    };

    fn setTimeRange(self: *Cache, start: u64, end: u64) void {
        self.start_time = start;
        self.end_time = end;
        self.dirty = true;
    }

    fn ensureReady(self: *Cache, store: *const Store) void {
        if (!self.dirty) return;
        self.dirty = false;

        _ = self.arena.reset(.retain_capacity);
        const gpa = self.arena.allocator();
        self.stacks = .empty;

        for (store.stacks.keys(), store.stacks.values()) |stack_ref, *stack| {
            const activity = stack.getActivity(&db_reader, self.start_time, self.end_time);
            const spans = stack.getSpans(&db_reader, self.start_time, self.end_time);
            if (activity.len == 0 and spans.len == 0) continue;
            self.stacks.append(gpa, .{
                .ref = stack_ref,
                .activity = activity,
                .spans = spans,
            }) catch @panic("OOM");
        }
        mem.sort(StackCache, self.stacks.items, {}, struct {
            fn f(ctx: void, a: StackCache, b: StackCache) bool {
                _ = ctx;
                return @intFromEnum(a.ref) <= @intFromEnum(b.ref);
            }
        }.f);
    }
};
