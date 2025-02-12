const std = @import("std");
const Allocator = std.mem.Allocator;
const ArrayListUnmanaged = std.ArrayListUnmanaged;

const fimo_std = @import("fimo_std");
const AnyError = fimo_std.AnyError;
const AnyResult = AnyError.AnyResult;

const command_buffer = @import("command_buffer.zig");
const OpaqueCommandBuffer = command_buffer.OpaqueCommandBuffer;
const Handle = command_buffer.Handle;
const symbols = @import("symbols.zig");
const testing = @import("testing.zig");

/// Unique identifier of a `Pool`.
///
/// The identifier remains valid until the pool is destroyed.
pub const Id = enum(usize) {
    _,

    /// Acquires a reference to the worker pool with the provided id.
    pub fn query(self: Id, provider: anytype) ?Pool {
        return Pool.queryById(provider, self);
    }

    test "query own pool" {
        try testing.initTestContextInTask(struct {
            fn f(ctx: *const testing.TestContext, err: *?AnyError) anyerror!void {
                _ = err;
                const pool = Pool.current(ctx).?;
                defer pool.unref();

                const id = pool.id();
                const queried = id.query(ctx).?;
                defer queried.unref();
                try std.testing.expectEqual(id, queried.id());
            }
        }.f);
    }
};

/// Identifier of a worker thread in a `Pool`.
pub const Worker = enum(usize) {
    _,

    /// Returns the id of the current worker.
    pub fn current(provider: anytype) ?Worker {
        const sym = symbols.worker_id.requestFrom(provider);
        var id: Worker = undefined;
        if (sym(&id) == false) return null;
        return id;
    }

    test "no task" {
        var ctx = try testing.initTestContext();
        defer ctx.deinit();
        try std.testing.expectEqual(null, Worker.current(ctx));
    }

    test "in task" {
        try testing.initTestContextInTask(struct {
            fn f(ctx: *const testing.TestContext, err: *?AnyError) anyerror!void {
                _ = err;
                try std.testing.expect(Worker.current(ctx) != null);
            }
        }.f);
    }
};

/// A stack size.
pub const StackSize = enum(usize) {
    _,

    /// Sentinel for the default stack size of a worker pool.
    pub const default: StackSize = fromInt(0);

    /// Constructs a new size from an integer.
    pub fn fromInt(size: usize) StackSize {
        return @enumFromInt(size);
    }

    /// Casts the size to an integer.
    pub fn toInt(self: StackSize) usize {
        return @intFromEnum(self);
    }
};

/// VTable of a `Pool`.
pub const VTable = extern struct {
    id: *const fn (pool: *anyopaque) callconv(.c) Id,
    ref: *const fn (pool: *anyopaque) callconv(.c) void,
    unref: *const fn (pool: *anyopaque) callconv(.c) void,
    request_close: *const fn (pool: *anyopaque) callconv(.c) void,
    accepts_requests: *const fn (pool: *anyopaque) callconv(.c) bool,
    owns_current_thread: *const fn (pool: *anyopaque) callconv(.c) bool,
    label: *const fn (pool: *anyopaque, len: *usize) callconv(.c) ?[*]const u8,
    workers: *const fn (pool: *anyopaque, ptr: ?[*]Worker, len: usize) callconv(.c) usize,
    stack_sizes: *const fn (pool: *anyopaque, ptr: ?[*]StackSize, len: usize) callconv(.c) usize,
    enqueue_buffer: *const fn (
        pool: *anyopaque,
        buffer: *OpaqueCommandBuffer,
        handle: ?*Handle,
    ) callconv(.c) AnyResult,
};

/// A worker pool.
pub const Pool = extern struct {
    data: *anyopaque,
    vtable: *const VTable,

    /// Creates a new worker pool with the specified configuration.
    pub fn init(provider: anytype, config: *const Config, err: *?AnyError) AnyError.Error!Pool {
        const sym = symbols.create_worker_pool.requestFrom(provider);
        var p: Pool = undefined;
        try sym(config, &p).intoErrorUnion(err);
        return p;
    }

    /// Returns the pool managing the current thread.
    pub fn current(provider: anytype) ?Pool {
        const sym = symbols.worker_pool.requestFrom(provider);
        var p: Pool = undefined;
        if (sym(&p) == false) return null;
        return p;
    }

    test "current, no task" {
        var ctx = try testing.initTestContext();
        defer ctx.deinit();
        try std.testing.expectEqual(null, Pool.current(ctx));
    }

    test "current, in task" {
        try testing.initTestContextInTask(struct {
            fn f(ctx: *const testing.TestContext, err: *?AnyError) anyerror!void {
                _ = err;
                const pool = Pool.current(ctx);
                defer if (pool) |p| p.unref();
                try std.testing.expect(pool != null);
            }
        }.f);
    }

    /// Acquires a reference to the worker pool with the provided id.
    pub fn queryById(provider: anytype, pool_id: Id) ?Pool {
        const sym = symbols.worker_pool_by_id.requestFrom(provider);
        var p: Pool = undefined;
        if (sym(pool_id, &p) == false) return null;
        return p;
    }

    /// Queries all public and active worker pools managed by the runtime.
    pub fn queryAll(provider: anytype, err: *?AnyError) AnyError.Error!Query {
        const sym = symbols.query_worker_pools.requestFrom(provider);
        var query: Query = undefined;
        try sym(&query).intoErrorUnion(err);
        return query;
    }

    test "query all" {
        var ctx = try testing.initTestContext();
        defer ctx.deinit();

        var err: ?AnyError = null;
        defer if (err) |e| e.deinit();

        const cfg = Config{ .is_queryable = true };
        const pool = try Pool.init(&ctx, &cfg, &err);
        defer pool.unref();
        const pool_id = pool.id();

        const query = try Pool.queryAll(&ctx, &err);
        defer query.deinit();

        var found = false;
        var node = query.root;
        while (node) |it| : (node = it.next) {
            if (it.pool.id() == pool_id) {
                found = true;
                break;
            }
        }

        try std.testing.expect(found);
    }

    /// Returns the id of the pool.
    pub fn id(self: Pool) Id {
        return self.vtable.id(self.data);
    }

    /// Acquires a new reference to the pool.
    pub fn ref(self: Pool) Pool {
        self.vtable.ref(self.data);
        return self;
    }

    /// Releases the reference to the pool.
    pub fn unref(self: Pool) void {
        self.vtable.unref(self.data);
    }

    /// Sends a request to stop accepting new requests.
    pub fn requestClose(self: Pool) void {
        self.vtable.request_close(self.data);
    }

    /// Checks if the pool accepts new requests.
    pub fn acceptsRequests(self: Pool) bool {
        return self.vtable.accepts_requests(self.data);
    }

    /// Checks if the current thread is managed by the pool.
    pub fn ownsCurrentThread(self: Pool) bool {
        return self.vtable.owns_current_thread(self.data);
    }

    /// Returns the optional label of the pool.
    pub fn label(self: Pool) ?[]const u8 {
        var len: usize = undefined;
        const l = self.vtable.label(self.data, &len) orelse return null;
        return l[0..len];
    }

    /// Returns an owned slice of all workers managed by the pool.
    pub fn workers(self: Pool, allocator: Allocator) Allocator.Error![]Worker {
        var w = ArrayListUnmanaged(Worker){};
        errdefer w.deinit(allocator);
        while (true) {
            const len = self.vtable.workers(self.data, w.items.ptr, w.capacity);
            if (w.capacity >= len) {
                w.items.len = len;
                return try w.toOwnedSlice(allocator);
            }
            try w.resize(allocator, len);
        }
    }

    /// Returns an owned slice of all stack sizes supported by the pool.
    pub fn stackSizes(self: Pool, allocator: Allocator) Allocator.Error![]StackSize {
        var w = ArrayListUnmanaged(StackSize){};
        errdefer w.deinit(allocator);
        while (true) {
            const len = self.vtable.stack_sizes(self.data, w.items.ptr, w.capacity);
            if (w.capacity >= len) {
                w.items.len = len;
                return try w.toOwnedSlice(allocator);
            }
            try w.resize(allocator, len);
        }
    }

    /// Enqueues the command buffer in the pool.
    ///
    /// The buffer must remain valid until it is deinitialized by the pool.
    pub fn enqueueCommandBuffer(
        self: Pool,
        buffer: *OpaqueCommandBuffer,
        err: *?AnyError,
    ) AnyError.Error!Handle {
        var handle: Handle = undefined;
        try self.vtable.enqueue_buffer(self.data, buffer, &handle)
            .intoErrorUnion(err);
        return handle;
    }

    /// Enqueues the command buffer in the pool and detaches it.
    ///
    /// The buffer must remain valid until it is deinitialized by the pool.
    pub fn enqueueCommandBufferDetached(
        self: Pool,
        buffer: *OpaqueCommandBuffer,
        err: *?AnyError,
    ) AnyError.Error!void {
        try self.vtable.enqueue_buffer(self.data, buffer, null)
            .intoErrorUnion(err);
    }
};

/// Configuration for the creation of a new worker pool.
pub const Config = extern struct {
    /// Reserved for future use.
    next: ?*const anyopaque = null,
    /// Optional label of the worker pool.
    label_: ?[*]const u8 = null,
    /// Length of the label.
    label_len: usize = 0,
    /// Configuration of the stack sizes provided by the pool.
    ///
    /// The runtime chooses the most restrictive stack config available when a stack is assigned to
    /// a new task.
    stacks_: [*]const StackConfig = &.{.{}},
    /// Number of stack configs.
    stacks_len: usize = 1,
    /// Index of the default stack configuration.
    default_stack_index: usize = 0,
    /// Number of worker threads to start.
    ///
    /// A value of `0` indicates to use the default number of workers, specified by the runtime.
    worker_count: usize = 0,
    /// Indicates whether to make the pool queryable. The pool can always be acquired through the
    /// pool id.
    is_queryable: bool = false,

    /// Stack configuration for the worker pool.
    pub const StackConfig = extern struct {
        /// Reserved for future use.
        next: ?*const anyopaque = null,
        /// Size of the stack allocation.
        size: StackSize = .default,
        /// Number of stacks to allocate at pool creation time.
        preallocated_count: usize = 0,
        /// Number of cold stacks to keep allocated.
        cold_count: usize = 0,
        /// Number of hot stacks to keep allocated.
        hot_count: usize = 0,
        /// Maximum number of allocated stacks.
        ///
        /// A value of `0` indicates no upper limit.
        max_allocated: usize = 0,
    };
};

/// A query of the available worker pools.
///
/// The pool references are owned by the query and are released upon calling deinit.
pub const Query = extern struct {
    root: ?*Node,
    deinit_fn: *const fn (root: ?*Node) callconv(.c) void,

    pub const Node = extern struct {
        pool: Pool,
        next: ?*Node,
    };

    /// Deinitializes the query.
    pub fn deinit(self: @This()) void {
        self.deinit_fn(self.root);
    }
};
