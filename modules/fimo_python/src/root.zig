const std = @import("std");
const testing = std.testing;
const Thread = std.Thread;
const Mutex = Thread.Mutex;
const Condition = Thread.Condition;
const builtin = @import("builtin");

const fimo_python_meta = @import("fimo_python_meta");
const fimo_std = @import("fimo_std");
const heap = fimo_std.heap;
const Path = fimo_std.path.Path;
const Context = fimo_std.Context;
const Tracing = Context.Tracing;
const Module = Context.Module;

const Python = @cImport({
    @cInclude("Python.h");
});

const Instance = Module.exports.Builder
    .init("fimo_python")
    .withDescription("Embedded Python interpreter")
    .withAuthor("Gabriel Borrelli")
    .withLicense("MIT + APACHE 2.0")
    .withResource(.{ .name = "home", .path = Path.init("") catch unreachable })
    .withResource(.{ .name = "module_path", .path = Path.init("module.fimo_module") catch unreachable })
    .withResource(.{ .name = "lib_path", .path = Path.init("Lib") catch unreachable })
    .withResource(.{ .name = "dynload_path", .path = Path.init("DLLs") catch unreachable })
    .withDynamicExport(
    fimo_python_meta.symbols.RunString,
    "run_string",
    State.initRunString,
    State.deinitRunString,
)
    .withState(State, State.init, State.deinit)
    .exportModule();

comptime {
    _ = Instance;
}

const State = struct {
    thread: Thread,
    mutex: Mutex,
    condition: Condition,
    state: enum { run, stop } = .stop,
    err: ?anyerror,
    thread_state: *PyThreadState,

    const PyConfig = Python.PyConfig;
    const PyConfig_Clear = Python.PyConfig_Clear;
    const PyConfig_InitIsolatedConfig = Python.PyConfig_InitIsolatedConfig;
    const PyConfig_SetBytesString = Python.PyConfig_SetBytesString;

    const PyStatus = Python.PyStatus;
    const PyStatus_Exception = Python.PyStatus_Exception;

    const PyMem_RawFree = Python.PyMem_RawFree;
    const Py_DecodeLocale = Python.Py_DecodeLocale;
    const PyWideStringList_Append = Python.PyWideStringList_Append;

    const Py_InitializeFromConfig = Python.Py_InitializeFromConfig;
    const Py_FinalizeEx = Python.Py_FinalizeEx;

    const PyThreadState = opaque {};
    extern fn PyEval_SaveThread() ?*PyThreadState;
    extern fn PyEval_RestoreThread(?*PyThreadState) void;
    extern fn PyThreadState_Clear(?*PyThreadState) void;
    extern fn PyThreadState_New(?*PyInterpreterState) ?*PyThreadState;
    extern fn PyThreadState_DeleteCurrent() void;

    extern fn Py_EndInterpreter(?*PyThreadState) void;
    extern fn Py_NewInterpreterFromConfig(tstate_p: [*c]?*PyThreadState, config: [*c]const PyInterpreterConfig) PyStatus;

    const PyInterpreterState = Python.PyInterpreterState;
    const PyInterpreterConfig = Python.PyInterpreterConfig;
    const PyInterpreterState_Main = Python.PyInterpreterState_Main;

    fn init(octx: *const Module.OpaqueInstance, set: Module.LoadingSet) !*State {
        const ctx: *const Instance = @alignCast(@ptrCast(octx));
        ctx.context().tracing().emitTraceSimple("initializing fimo_python", .{}, @src());
        _ = set;

        const self = try heap.fimo_allocator.create(State);
        errdefer heap.fimo_allocator.destroy(self);
        self.* = .{
            .thread = undefined,
            .mutex = .{},
            .condition = .{},
            .state = .stop,
            .err = null,
            .thread_state = undefined,
        };

        // Python must be initialized and finalized from the same thread, which the fimo
        // library does not guarantee. Therefore we create a custom idle thread, whose sole
        // purpose is to initialize and eventually finalize the runtime.
        self.thread = try Thread.spawn(.{}, initDeinitSubThread, .{ self, ctx });
        errdefer self.thread.join();

        {
            self.mutex.lock();
            defer self.mutex.unlock();
            while (self.state == .stop) {
                self.condition.wait(&self.mutex);
            }
        }
        if (self.err) |err| return err;

        return self;
    }

    fn deinit(octx: *const Module.OpaqueInstance, self: *State) void {
        _ = octx;

        {
            self.mutex.lock();
            defer self.mutex.unlock();
            self.state = .stop;
            self.condition.signal();
        }

        self.thread.join();
        heap.fimo_allocator.destroy(self);
    }

    fn initDeinitSubThread(self: *State, ctx: *const Instance) void {
        self.initSubThread(ctx) catch return;

        self.mutex.lock();
        while (self.state == .run) {
            self.condition.wait(&self.mutex);
        }

        self.deinitSubThread(ctx);
    }

    fn initSubThread(self: *State, ctx: *const Instance) !void {
        self.mutex.lock();
        defer self.mutex.unlock();
        defer self.condition.signal();
        defer self.state = .run;
        errdefer |err| self.err = err;

        var cfg: PyConfig = undefined;
        PyConfig_InitIsolatedConfig(&cfg);
        defer PyConfig_Clear(&cfg);

        var status = PyConfig_SetBytesString(&cfg, &cfg.home, ctx.resources.home);
        if (PyStatus_Exception(status) != 0) return error.HomeConfig;

        status = PyConfig_SetBytesString(&cfg, &cfg.program_name, ctx.resources.module_path);
        if (PyStatus_Exception(status) != 0) return error.ProgramNameConfig;

        cfg.module_search_paths_set = 1;
        const lib_path = Py_DecodeLocale(ctx.resources.lib_path, null);
        defer if (lib_path != null) PyMem_RawFree(lib_path);
        if (lib_path == null) return error.DecodeLibPath;
        status = PyWideStringList_Append(&cfg.module_search_paths, lib_path);
        if (PyStatus_Exception(status) != 0) return error.AppendLibPath;

        if (builtin.target.os.tag == .windows) {
            const dynload_path = Py_DecodeLocale(ctx.resources.dynload_path, null);
            defer if (dynload_path != null) PyMem_RawFree(dynload_path);
            if (dynload_path == null) return error.DecodeDynloadPath;
            status = PyWideStringList_Append(&cfg.module_search_paths, dynload_path);
            if (PyStatus_Exception(status) != 0) return error.AppendDynloadPath;
        }

        status = Py_InitializeFromConfig(&cfg);
        if (PyStatus_Exception(status) != 0) {
            ctx.context().tracing().emitErrSimple(
                "{s}",
                .{@as([*:0]const u8, status.err_msg)},
                @src(),
            );
            return error.PythonInit;
        }

        self.thread_state = PyEval_SaveThread().?;
    }

    fn deinitSubThread(self: *State, ctx: *const Instance) void {
        PyEval_RestoreThread(self.thread_state);
        const result = Py_FinalizeEx();
        if (result != 0) ctx.context().tracing().emitErrSimple(
            "Python interpreter could not be finalized",
            .{},
            @src(),
        );
    }

    fn initRunString(octx: *const Module.OpaqueInstance) !*fimo_python_meta.RunString {
        const ctx: *const Instance = @alignCast(@ptrCast(octx));
        const sym = try heap.fimo_allocator.create(fimo_python_meta.RunString);
        errdefer heap.fimo_allocator.destroy(sym);

        sym.* = fimo_python_meta.RunString{
            .data = @constCast(ctx),
            .call_f = &struct {
                fn f(data: ?*anyopaque, code: [*:0]const u8, home: ?[*:0]const u8) callconv(.C) fimo_std.c.FimoResult {
                    const ctx_: *const Instance = @alignCast(@ptrCast(data));
                    const code_ = std.mem.span(code);
                    const home_ = if (home) |h| std.mem.span(h) else null;
                    State.runString(ctx_, code_, home_) catch |err| {
                        if (@errorReturnTrace()) |tr|
                            ctx_.context().tracing().emitStackTraceSimple(tr.*, @src());
                        return fimo_std.AnyError.initError(err).err;
                    };
                    return fimo_std.AnyError.intoCResult(null);
                }
            }.f,
        };

        return sym;
    }

    fn deinitRunString(sym: *fimo_python_meta.RunString) void {
        heap.fimo_allocator.destroy(sym);
    }

    fn runString(ctx: *const Instance, code: [:0]const u8, home: ?[:0]const u8) !void {
        _ = ctx;

        const main_interpreter = PyInterpreterState_Main();
        const state = PyThreadState_New(main_interpreter);
        defer {
            PyEval_RestoreThread(state);
            PyThreadState_Clear(state);
            PyThreadState_DeleteCurrent();
        }

        const cfg = PyInterpreterConfig{
            .use_main_obmalloc = 0,
            .allow_fork = 0,
            .allow_exec = 0,
            .allow_threads = 1,
            .allow_daemon_threads = 0,
            .check_multi_interp_extensions = 1,
            .gil = Python.PyInterpreterConfig_OWN_GIL,
        };

        var sub_state: *PyThreadState = undefined;
        const status = Py_NewInterpreterFromConfig(@ptrCast(&sub_state), &cfg);
        if (PyStatus_Exception(status) != 0) return error.SubInterpreterInit;
        defer Py_EndInterpreter(sub_state);

        if (home) |h| {
            const path = Python.PySys_GetObject("path");
            defer Python.Py_DecRef(path);

            const home_object = Python.PyUnicode_FromString(h);
            if (home_object == null) {
                const ex = Python.PyErr_Occurred();
                Python.PyErr_DisplayException(ex);
                Python.PyErr_Clear();
                return error.SetHomePath;
            }
            defer Python.Py_DecRef(home_object);

            const result = Python.PyList_Append(path, home_object);
            if (result != 0) {
                const ex = Python.PyErr_Occurred();
                Python.PyErr_DisplayException(ex);
                Python.PyErr_Clear();
                return error.AppendHomePath;
            }
        }

        const compiled_code = Python.Py_CompileString(code, "<string_eval>", Python.Py_file_input);
        if (compiled_code == null) {
            const ex = Python.PyErr_Occurred();
            Python.PyErr_DisplayException(ex);
            Python.PyErr_Clear();
            return error.CompileCode;
        }
        defer Python.Py_DecRef(compiled_code);

        const code_module = Python.PyImport_ExecCodeModule("__main__", compiled_code);
        if (code_module == null) {
            const ex = Python.PyErr_Occurred();
            Python.PyErr_DisplayException(ex);
            Python.PyErr_Clear();
            return error.ExecudeCode;
        }
        defer Python.Py_DecRef(code_module);
    }
};
