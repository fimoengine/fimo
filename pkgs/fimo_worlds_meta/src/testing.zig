const std = @import("std");

const fimo_std = @import("fimo_std");
const AnyError = fimo_std.AnyError;
const Context = fimo_std.Context;
const Async = Context.Async;
const Tracing = Context.Tracing;
const Module = Context.Module;
const Symbol = Module.Symbol;
const SymbolWrapper = Module.SymbolWrapper;
const SymbolGroup = Module.SymbolGroup;
const PseudoInstance = Module.PseudoInstance;
const fimo_tasks_meta = @import("fimo_tasks_meta");

const symbols = @import("symbols.zig");

comptime {
    @import("test_module").forceExportModules();
}

pub const GlobalCtx = struct {
    var ctx: ?TestContext = null;

    pub fn init(self: @This()) !void {
        _ = self;
        if (ctx != null) @panic("context already initialized");
        ctx = try .init();
    }

    pub fn deinit(self: @This()) void {
        _ = self;
        if (ctx) |*c| {
            c.deinit();
            ctx = null;
        } else @panic("not initialized");
    }

    pub fn provideSymbol(self: @This(), comptime symbol: Symbol) *const symbol.T {
        _ = self;
        if (ctx) |*c| return symbol.requestFrom(c);
        @panic("not initialized");
    }
}{};

const TestContext = struct {
    ctx: Context,
    event_loop: Async.EventLoop,
    instance: *const PseudoInstance,
    symbols: SymbolGroup(symbols.all_symbols ++ fimo_tasks_meta.symbols.all_symbols),

    fn init() !@This() {
        const tracing_cfg = Tracing.Config{
            .max_level = .warn,
            .subscribers = &.{Tracing.default_subscriber},
            .subscriber_count = 1,
        };
        defer tracing_cfg.deinit();
        const init_options: [:null]const ?*const Context.TaggedInStruct = &.{@ptrCast(&tracing_cfg)};

        const ctx = try Context.init(init_options);
        errdefer ctx.unref();

        ctx.tracing().registerThread();
        errdefer ctx.tracing().unregisterThread();

        var err: ?fimo_std.AnyError = null;
        errdefer if (err) |e| {
            ctx.tracing().emitErrSimple("{f}", .{e}, @src());
            e.deinit();
        };

        errdefer Async.EventLoop.flushWithCurrentThread(ctx.async(), &err) catch unreachable;
        const event_loop = try Async.EventLoop.init(ctx.async(), &err);
        errdefer event_loop.join();

        const async_ctx = try Async.BlockingContext.init(ctx.async(), &err);
        defer async_ctx.deinit();

        const set = try Module.LoadingSet.init(ctx.module(), &err);
        defer set.unref();

        try set.addModulesFromLocal(
            &{},
            struct {
                fn f(@"export": *const Module.Export, data: *const void) Module.LoadingSet.FilterRequest {
                    _ = @"export";
                    _ = data;
                    return .load;
                }
            }.f,
            null,
            &err,
        );
        try set.commit().intoFuture().awaitBlocking(async_ctx).unwrap(&err);

        const instance = try Module.PseudoInstance.init(ctx.module(), &err);
        errdefer instance.deinit();

        const tasks = try Module.Info.findByName(ctx.module(), "fimo_tasks", &err);
        defer tasks.unref();
        const worlds = try Module.Info.findByName(ctx.module(), "fimo_worlds", &err);
        defer worlds.unref();

        try instance.addDependency(tasks, &err);
        try instance.addDependency(worlds, &err);
        try instance.addNamespace(symbols.symbol_namespace, &err);
        try instance.addNamespace(fimo_tasks_meta.symbols.symbol_namespace, &err);

        const test_ctx = @This(){
            .ctx = ctx,
            .event_loop = event_loop,
            .instance = instance,
            .symbols = try instance.loadSymbolGroup(
                symbols.all_symbols ++ fimo_tasks_meta.symbols.all_symbols,
                &err,
            ),
        };

        return test_ctx;
    }

    pub fn deinit(self: *@This()) void {
        self.instance.deinit();

        var err: ?fimo_std.AnyError = null;
        self.ctx.module().pruneInstances(&err) catch unreachable;
        self.event_loop.join();
        Async.EventLoop.flushWithCurrentThread(self.ctx.async(), &err) catch unreachable;
        self.ctx.tracing().unregisterThread();
        self.ctx.unref();
    }

    pub fn provideSymbol(self: *const @This(), comptime symbol: Symbol) *const symbol.T {
        return symbol.requestFrom(&self.symbols);
    }
};
