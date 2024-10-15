const std = @import("std");
const builtin = @import("builtin");

const c = std.c;
const posix = std.posix;
const windows = std.os.windows;

const allocator = std.heap.c_allocator;

pub const TlsError = error{
    /// The process-wide limit on the total number of tls slots has been reached.
    TlsSlotsQuotaExceeded,
    /// There was insufficient memory to complete the operation.
    SystemResources,
} || posix.UnexpectedError;

const PosixTlsImpl = struct {
    key: c.pthread_key_t,

    const Self = @This();

    fn init(destructor: ?*const fn (ptr: *anyopaque) callconv(.C) void) TlsError!Self {
        var self = Self{ .key = undefined };
        switch (c.pthread_key_create(&self.key, @ptrCast(destructor))) {
            .SUCCESS => return self,
            else => |err| return posix.unexpectedErrno(err),

            .AGAIN => return error.TlsSlotsQuotaExceeded,
            .NOMEM => return error.SystemResources,
        }
    }

    fn deinit(self: Self) void {
        switch (c.pthread_key_delete(self.key)) {
            .SUCCESS => return,
            else => |err| {
                posix.unexpectedErrno(err);
                std.process.abort();
            },

            .INVAL => unreachable,
        }
    }

    fn get(self: Self) ?*anyopaque {
        return c.pthread_getspecific(self.key);
    }

    fn set(self: Self, value: ?*anyopaque) TlsError!void {
        switch (c.pthread_setspecific(self.key, value)) {
            .SUCCESS => return,
            else => |err| return posix.unexpectedErrno(err),

            .INVAL => unreachable,
            .NOMEM => return error.SystemResources,
        }
    }
};

const WindowsTlsImpl = struct {
    tls: windows.DWORD,

    const Self = @This();
    const max_indices_per_process = 1088;
    const WINAPI = windows.WINAPI;
    var dtors = [_]?*const fn (*anyopaque) callconv(.C) void{null} ** max_indices_per_process;

    extern "kernel32" fn TlsAlloc() callconv(WINAPI) windows.DWORD;
    extern "kernel32" fn TlsFree(index: windows.DWORD) callconv(WINAPI) windows.BOOL;
    extern "kernel32" fn TlsGetValue(index: windows.DWORD) callconv(WINAPI) ?windows.LPVOID;
    extern "kernel32" fn TlsSetValue(index: windows.DWORD, value: ?windows.LPVOID) callconv(WINAPI) windows.BOOL;

    const Data = struct {
        value: ?*anyopaque = null,
        tls: WindowsTlsImpl,
        next: ?*Data = null,

        threadlocal var head: ?*Data = null;
        threadlocal var tail: ?*Data = null;
        export var callback: windows.PIMAGE_TLS_CALLBACK linksection(".CRT$XLB") = @ptrCast(&tss_callback);

        fn cleanup() void {
            const tss_dtor_iterations = 4;

            var again = true;
            for (0..tss_dtor_iterations) |_| {
                if (!again) break;
                again = false;

                var current = head;
                while (current) |data| {
                    current = data.next;
                    if (data.value) |value| {
                        data.value = null;
                        if (dtors[data.tls.tls]) |dtor| {
                            again = true;
                            dtor(value);
                        }
                    }
                }

                while (head) |data| {
                    head = data.next;
                    allocator.destroy(data);
                }

                head = null;
                tail = null;
            }
        }

        fn tss_callback(handle: ?windows.PVOID, dwReason: windows.DWORD, pv: ?windows.PVOID) callconv(WINAPI) void {
            _ = handle;
            _ = pv;

            const DLL_PROCESS_DETACH = 0;
            const DLL_THREAD_DETACH = 3;
            if (head != null and (dwReason == DLL_PROCESS_DETACH or dwReason == DLL_THREAD_DETACH)) {
                cleanup();
            }
        }
    };

    fn init(destructor: ?*const fn (ptr: *anyopaque) callconv(.C) void) TlsError!Self {
        const self = Self{ .tls = TlsAlloc() };
        if (self.tls == windows.TLS_OUT_OF_INDEXES) {
            return error.TlsSlotsQuotaExceeded;
        }
        dtors[self.tls] = destructor;
        return self;
    }

    fn deinit(self: Self) void {
        dtors[self.tls] = null;
        if (TlsFree(self.tls) == 0) unreachable;
    }

    fn get(self: Self) ?*anyopaque {
        const data: ?*Data = @alignCast(@ptrCast(TlsGetValue(self.tls)));
        if (data) |d| return d.value;
        return null;
    }

    fn set(self: Self, value: ?*anyopaque) TlsError!void {
        var data: ?*Data = @alignCast(@ptrCast(TlsGetValue(self.tls)));
        if (data == null) {
            const d = allocator.create(Data) catch return error.SystemResources;
            errdefer allocator.destroy(d);
            data = d;

            d.* = .{ .tls = self };
            if (Data.head == null) Data.head = d;
            if (Data.tail) |tail| {
                tail.next = d;
            } else {
                Data.tail = d;
            }

            if (TlsSetValue(self.tls, d) == 0) return error.SystemResources;
        }
        data.?.value = value;
    }
};

const UnsupportedTlsImpl = struct {
    const Self = @This();

    fn init(destructor: ?*const fn (ptr: *anyopaque) callconv(.C) void) TlsError!Self {
        unsupported(.{destructor});
    }

    fn deinit(self: Self) void {
        unsupported(.{self});
    }

    fn get(self: Self) ?*anyopaque {
        unsupported(.{self});
    }

    fn set(self: Self, value: ?*anyopaque) TlsError!void {
        unsupported(.{ self, value });
    }

    fn unsupported(v: anytype) noreturn {
        _ = v;
        @compileError("Unsupported operating system " ++ @tagName(builtin.os.tag));
    }
};

const Impl = if (builtin.os.tag == .windows)
    WindowsTlsImpl
else if (builtin.link_libc)
    PosixTlsImpl
else
    UnsupportedTlsImpl;

pub fn Tls(comptime T: type) type {
    return struct {
        impl: Impl,

        const Self = @This();

        pub fn init(destructor: ?*const fn (ptr: *T) callconv(.C) void) TlsError!Self {
            return Self{ .impl = try Impl.init(@ptrCast(destructor)) };
        }

        pub fn deinit(self: Self) void {
            self.impl.deinit();
        }

        pub fn get(self: Self) ?*T {
            const ptr = self.impl.get();
            return @alignCast(@ptrCast(ptr));
        }

        pub fn set(self: Self, value: ?*T) TlsError!void {
            return self.impl.set(value);
        }
    };
}
