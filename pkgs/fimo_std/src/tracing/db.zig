const std = @import("std");
const fs = std.fs;
const mem = std.mem;
const meta = std.meta;
const heap = std.heap;
const math = std.math;
const debug = std.debug;
const File = fs.File;
const posix = std.posix;
const windows = std.os.windows;
const testing = std.testing;
const builtin = @import("builtin");

const tracing = @import("../tracing.zig");
pub const EventLevel = tracing.Level;
pub const CpuArch = tracing.events.CpuArch;

pub const SuperBlock = extern struct {
    pub const expect_magic = "Fimo Trace DB\x00\x00\x00";
    pub const expect_version_major = 1;
    pub const expect_version_minor = 0;

    file_magic: [expect_magic.len]u8 = expect_magic.*,
    version_major: u16 = expect_version_major,
    version_minor: u16 = expect_version_minor,
    _reserved: u32 = 0,
    page_size: u64,
    session_table: SuperBlockOffset,
    event_info_table: SuperBlockOffset,
    data_table: SuperBlockOffset,
    event_table: SuperBlockOffset,
};

pub const SuperBlockOffset = enum(u64) { _ };

pub const SessionTable = extern struct {
    table_end: SessionTableOffset,
    num_entries: u64,
};

pub const SessionTableOffset = enum(u64) { _ };

pub const Session = extern struct {
    start_time: Instant,
    end_time: Instant,
    epoch: Time,
    events_start: EventRef,
    num_events: u64,
    resolution: Duration,
    available_memory: u64,
    process_id: u64,
    num_cores: u16,
    cpu_arch: CpuArch,
    _reserved1: u8 = 0,
    cpu_id: u32,
    cpu_vendor: DataRef,
    app_name: DataRef,
    host_info: DataRef,
};

pub const EventInfoTable = extern struct {
    table_end: EventInfoOffset,
    capacity: u64,
    num_entries: u64,
};

pub const EventInfoOffset = enum(u64) { _ };
pub const EventInfoId = enum(u64) { _ };
pub const EventInfoRef = enum(u64) { _ };

pub const EventInfoTableBucket = extern struct {
    hash: u64,
    ref: EventInfoRef,
};

pub const EventInfo = extern struct {
    id: EventInfoId,
    name: DataRef,
    target: DataRef,
    scope: DataRef,
    file_name: DataRef,
    line_number: u32,
    level: EventLevel,
};

fn getEventInfoTableValue(
    header: []const u8,
    buckets: []const EventInfoTableBucket,
    entries: []const EventInfo,
    id: EventInfoId,
) ?EventInfoRef {
    debug.assert(header.len * 8 >= buckets.len);
    debug.assert(math.isPowerOfTwo(buckets.len));
    const mask: u64 = buckets.len - 1;
    var hasher = std.hash.Wyhash.init(0);
    std.hash.autoHash(&hasher, id);
    const hash = hasher.final();
    for (0..buckets.len) |i| {
        const idx = (hash + i) & mask;
        const header_idx = idx / 8;
        const header_bit_idx: u3 = @truncate(idx % 8);
        const header_byte = header[header_idx];
        const is_set = header_byte & (@as(u8, 1) << header_bit_idx) != 0;
        if (!is_set) return null;

        const bucket = buckets[idx];
        if (bucket.hash != hash) continue;
        const entry = entries[@intFromEnum(bucket.ref)];
        if (entry.id == id) return bucket.ref;
        continue;
    }

    return null;
}

fn putEventInfoTableValue(
    header: []u8,
    buckets: []EventInfoTableBucket,
    entries: []EventInfo,
    num_entries: *u64,
    info: EventInfo,
) EventInfoRef {
    debug.assert(header.len * 8 >= buckets.len);
    debug.assert(math.isPowerOfTwo(buckets.len));
    debug.assert(num_entries.* < entries.len);
    const mask: u64 = buckets.len - 1;
    var hasher = std.hash.Wyhash.init(0);
    std.hash.autoHash(&hasher, info.id);
    const hash = hasher.final();
    for (0..buckets.len) |i| {
        const idx = (hash + i) & mask;
        const header_idx = idx / 8;
        const header_bit_idx: u3 = @truncate(idx % 8);
        const header_byte = header[header_idx];
        const is_set = header_byte & (@as(u8, 1) << header_bit_idx) != 0;
        if (is_set) {
            const bucket = buckets[idx];
            if (bucket.hash != hash) continue;
            const entry = entries[@intFromEnum(bucket.ref)];
            if (entry.id == info.id) return bucket.ref;
            continue;
        }

        const entry_idx = num_entries.*;
        header[header_idx] = header_byte | (@as(u8, 1) << header_bit_idx);
        buckets[idx] = .{ .hash = hash, .ref = @enumFromInt(entry_idx) };
        entries[entry_idx] = info;
        num_entries.* = entry_idx + 1;
        return @enumFromInt(entry_idx);
    }

    unreachable;
}

fn putEventInfoTableRef(
    header: []u8,
    buckets: []EventInfoTableBucket,
    hash: u64,
    ref: EventInfoRef,
) void {
    debug.assert(header.len * 8 >= buckets.len);
    debug.assert(math.isPowerOfTwo(buckets.len));
    debug.assert(@intFromEnum(ref) < buckets.len);
    const mask: u64 = buckets.len - 1;
    for (0..buckets.len) |i| {
        const idx = (hash + i) & mask;
        const header_idx = idx / 8;
        const header_bit_idx: u3 = @truncate(idx % 8);
        const header_byte = header[header_idx];
        const is_set = header_byte & (@as(u8, 1) << header_bit_idx) != 0;
        if (is_set) continue;
        header[header_idx] = header_byte | (@as(u8, 1) << header_bit_idx);
        buckets[idx] = .{ .hash = hash, .ref = ref };
        return;
    }
}

fn rehashEventInfoTable(
    header: []u8,
    buckets: []EventInfoTableBucket,
    entries: []EventInfo,
) void {
    @memset(header, 0);
    for (entries, 0..) |entry, i| {
        var hasher = std.hash.Wyhash.init(0);
        std.hash.autoHash(&hasher, entry.id);
        const hash = hasher.final();
        putEventInfoTableRef(header, buckets, hash, @enumFromInt(i));
    }
}

pub const DataTable = extern struct {
    table_end: DataTableOffset,
    capacity: u64,
    num_entries: u64,
    buffer_start: DataTableOffset,
    buffer_end: DataTableOffset,
};

pub const data_table_buffer_alignment = 16;

pub const DataTableOffset = enum(u64) { _ };
pub const DataRef = enum(u64) { _ };
pub const DataOffset = enum(u64) { _ };

pub const DataTableBucket = extern struct {
    hash: u64,
    ref: DataRef,
};

pub const DataTableEntry = extern struct {
    start: DataOffset,
    end: DataOffset,
};

fn putDataTableValue(
    header: []u8,
    buckets: []DataTableBucket,
    entries: []DataTableEntry,
    num_entries: *u64,
    buffer: []u8,
    buffer_end: *u64,
    value: []const u8,
    alignment: u64,
) DataRef {
    debug.assert(header.len * 8 >= buckets.len);
    debug.assert(math.isPowerOfTwo(buckets.len));
    debug.assert(num_entries.* < entries.len);
    debug.assert(buffer.len >= mem.alignForward(u64, buffer_end.*, alignment) + value.len);
    const mask: u64 = buckets.len - 1;
    const hash = std.hash.Wyhash.hash(0, value);
    for (0..buckets.len) |i| {
        const idx = (hash + i) & mask;
        const header_idx = idx / 8;
        const header_bit_idx: u3 = @truncate(idx % 8);
        const header_byte = header[header_idx];
        const is_set = header_byte & (@as(u8, 1) << header_bit_idx) != 0;
        if (is_set) {
            const bucket = buckets[idx];
            if (bucket.hash != hash) continue;
            const entry = entries[@intFromEnum(bucket.ref)];
            const entry_value = buffer[@intFromEnum(entry.start)..@intFromEnum(entry.end)];
            if (mem.eql(u8, value, entry_value)) return bucket.ref;
            continue;
        }

        const entry_idx = num_entries.*;
        header[header_idx] = header_byte | (@as(u8, 1) << header_bit_idx);
        buckets[idx] = .{ .hash = hash, .ref = @enumFromInt(entry_idx) };

        const start = mem.alignForward(u64, buffer_end.*, alignment);
        const end = start + value.len;
        entries[entry_idx] = .{ .start = @enumFromInt(start), .end = @enumFromInt(end) };
        num_entries.* = entry_idx + 1;

        const slice = buffer[start..end];
        @memcpy(slice, value);
        buffer_end.* = end;
        return @enumFromInt(entry_idx);
    }

    unreachable;
}

fn putDataTableRef(
    header: []u8,
    buckets: []DataTableBucket,
    hash: u64,
    ref: DataRef,
) void {
    debug.assert(header.len * 8 >= buckets.len);
    debug.assert(math.isPowerOfTwo(buckets.len));
    debug.assert(@intFromEnum(ref) < buckets.len);
    const mask: u64 = buckets.len - 1;
    for (0..buckets.len) |i| {
        const idx = (hash + i) & mask;
        const header_idx = idx / 8;
        const header_bit_idx: u3 = @truncate(idx % 8);
        const header_byte = header[header_idx];
        const is_set = header_byte & (@as(u8, 1) << header_bit_idx) != 0;
        if (is_set) continue;
        header[header_idx] = header_byte | (@as(u8, 1) << header_bit_idx);
        buckets[idx] = .{ .hash = hash, .ref = ref };
        return;
    }
}

fn rehashDataTable(
    header: []u8,
    buckets: []DataTableBucket,
    entries: []DataTableEntry,
    buffer: []u8,
) void {
    @memset(header, 0);
    for (entries, 0..) |entry, i| {
        const value_start = @intFromEnum(entry.start);
        const value_end = @intFromEnum(entry.end);
        const entry_value = buffer[value_start..value_end];
        const hash = std.hash.Wyhash.hash(0, entry_value);
        putDataTableRef(header, buckets, hash, @enumFromInt(i));
    }
}

pub const EventTable = extern struct {
    table_end: EventTableOffset,
    num_entries: u64,
};

pub const max_event_alignment = @sizeOf(OpaqueEvent);
pub const max_event_size = @sizeOf(OpaqueEvent);

pub const EventTableOffset = enum(u64) { _ };
pub const EventRef = enum(u64) { _ };

pub const EventTag = enum(u16) {
    register_thread,
    unregister_thread,
    create_call_stack,
    destroy_call_stack,
    unblock_call_stack,
    suspend_call_stack,
    resume_call_stack,
    enter_span,
    exit_span,
    log_message,
};

pub const Instant = enum(u64) { _ };
pub const Time = enum(u64) { _ };
pub const Duration = enum(u64) { _ };
pub const ThreadId = enum(u64) { _ };
pub const StackRef = enum(u64) { _ };

pub const OpaqueEvent = extern struct {
    time: Instant,
    tag: EventTag,
    _padding: u16 = 0,
    _padding2: u32 = 0,
    _data1: u64 = 0,
    _data2: u64 = 0,

    pub fn bitCast(self: OpaqueEvent, T: type) T {
        const tag: EventTag = switch (T) {
            RegisterThread => .register_thread,
            UnregisterThread => .unregister_thread,
            CreateCallStack => .create_call_stack,
            DestroyCallStack => .destroy_call_stack,
            UnblockCallStack => .unblock_call_stack,
            SuspendCallStack => .suspend_call_stack,
            ResumeCallStack => .resume_call_stack,
            EnterSpan => .enter_span,
            ExitSpan => .exit_span,
            LogMessage => .log_message,
            else => @compileError("Unknown event type, got " ++ @typeName(T)),
        };
        debug.assert(self.tag == tag);
        if (comptime @sizeOf(T) == @sizeOf(OpaqueEvent)) return @bitCast(self);
        var event: T = undefined;
        @memcpy(mem.asBytes(&event), mem.asBytes(&self)[0..@sizeOf(T)]);
        return event;
    }
};

comptime {
    debug.assert(@sizeOf(OpaqueEvent) == 32);
    debug.assert(@alignOf(OpaqueEvent) <= 32);
}

pub const RegisterThread = extern struct {
    time: Instant,
    tag: EventTag = .register_thread,
    _padding: u16 = 0,
    _padding2: u32 = 0,
    thread_id: ThreadId,
};

pub const UnregisterThread = extern struct {
    time: Instant,
    tag: EventTag = .unregister_thread,
    _padding: u16 = 0,
    _padding2: u32 = 0,
    thread_id: ThreadId,
};

pub const CreateCallStack = extern struct {
    time: Instant,
    tag: EventTag = .create_call_stack,
    _padding: u16 = 0,
    _padding2: u32 = 0,
    stack: StackRef,
};

pub const DestroyCallStack = extern struct {
    time: Instant,
    tag: EventTag = .destroy_call_stack,
    _padding: u16 = 0,
    _padding2: u32 = 0,
    stack: StackRef,
};

pub const UnblockCallStack = extern struct {
    time: Instant,
    tag: EventTag = .unblock_call_stack,
    _padding: u16 = 0,
    _padding2: u32 = 0,
    stack: StackRef,
};

pub const SuspendCallStack = extern struct {
    time: Instant,
    tag: EventTag = .suspend_call_stack,
    _padding: u16 = 0,
    _padding2: u32 = 0,
    stack: StackRef,
    flags: packed struct(u64) {
        mark_blocked: bool,
        reserved: u63 = 0,
    },
};

pub const ResumeCallStack = extern struct {
    time: Instant,
    tag: EventTag = .resume_call_stack,
    _padding: u16 = 0,
    _padding2: u32 = 0,
    stack: StackRef,
    thread_id: ThreadId,
};

pub const EnterSpan = extern struct {
    time: Instant,
    tag: EventTag = .enter_span,
    _padding: u16 = 0,
    _padding2: u32 = 0,
    stack: StackRef,
    extra: DataRef,
};

pub const EnterSpanExt = extern struct {
    info: EventInfoId,
    message: DataRef,
};

pub const ExitSpan = extern struct {
    time: Instant,
    tag: EventTag = .exit_span,
    _padding: u16 = 0,
    _padding2: u32 = 0,
    stack: StackRef,
    flags: packed struct(u64) {
        is_unwinding: bool,
        reserved: u63 = 0,
    },
};

pub const LogMessage = extern struct {
    time: Instant,
    tag: EventTag = .log_message,
    _padding: u16 = 0,
    _padding2: u32 = 0,
    stack: StackRef,
    extra: DataRef,
};

pub const LogMessageExt = extern struct {
    info: EventInfoId,
    message: DataRef,
};

comptime {
    debug.assert(meta.hasUniqueRepresentation(SuperBlock));
    debug.assert(meta.hasUniqueRepresentation(SuperBlockOffset));
    debug.assert(meta.hasUniqueRepresentation(SessionTable));
    debug.assert(meta.hasUniqueRepresentation(Session));
    debug.assert(meta.hasUniqueRepresentation(EventInfoTable));
    debug.assert(meta.hasUniqueRepresentation(EventInfoId));
    debug.assert(meta.hasUniqueRepresentation(EventInfoRef));
    debug.assert(meta.hasUniqueRepresentation(EventInfo));
    debug.assert(meta.hasUniqueRepresentation(DataTable));
    debug.assert(meta.hasUniqueRepresentation(DataTableOffset));
    debug.assert(meta.hasUniqueRepresentation(DataRef));
    debug.assert(meta.hasUniqueRepresentation(DataOffset));
    debug.assert(meta.hasUniqueRepresentation(DataTableBucket));
    debug.assert(meta.hasUniqueRepresentation(DataTableEntry));
    debug.assert(meta.hasUniqueRepresentation(EventTable));
    debug.assert(meta.hasUniqueRepresentation(EventTableOffset));
    debug.assert(meta.hasUniqueRepresentation(EventRef));
    debug.assert(meta.hasUniqueRepresentation(Instant));
    debug.assert(meta.hasUniqueRepresentation(Time));
    debug.assert(meta.hasUniqueRepresentation(Duration));
    debug.assert(meta.hasUniqueRepresentation(ThreadId));
    debug.assert(meta.hasUniqueRepresentation(StackRef));
    debug.assert(meta.hasUniqueRepresentation(RegisterThread));
    debug.assert(meta.hasUniqueRepresentation(UnregisterThread));
    debug.assert(meta.hasUniqueRepresentation(CreateCallStack));
    debug.assert(meta.hasUniqueRepresentation(DestroyCallStack));
    debug.assert(meta.hasUniqueRepresentation(UnblockCallStack));
    debug.assert(meta.hasUniqueRepresentation(SuspendCallStack));
    debug.assert(meta.hasUniqueRepresentation(ResumeCallStack));
    debug.assert(meta.hasUniqueRepresentation(EnterSpan));
    debug.assert(meta.hasUniqueRepresentation(EnterSpanExt));
    debug.assert(meta.hasUniqueRepresentation(ExitSpan));
    debug.assert(meta.hasUniqueRepresentation(LogMessage));
    debug.assert(meta.hasUniqueRepresentation(LogMessageExt));
}

extern "kernel32" fn CreateFileMappingA(
    hFile: windows.HANDLE,
    lpFileMappingAttributes: ?*windows.SECURITY_ATTRIBUTES,
    flProtect: windows.DWORD,
    dwMaximumSizeHigh: windows.DWORD,
    dwMaximumSizeLow: windows.DWORD,
    lpName: ?windows.LPCSTR,
) callconv(.winapi) ?windows.LPVOID;

extern "kernel32" fn MapViewOfFileEx(
    hFileMappingObject: windows.HANDLE,
    dwDesiredAccess: windows.DWORD,
    dwFileOffsetHigh: windows.DWORD,
    dwFileOffsetLow: windows.DWORD,
    dwNumberOfBytesToMap: windows.SIZE_T,
    lpBaseAddress: ?windows.LPVOID,
) callconv(.winapi) ?windows.LPVOID;

extern "kernel32" fn UnmapViewOfFile(lpBaseAddress: windows.LPCVOID) callconv(.winapi) windows.BOOL;

extern "kernel32" fn FlushViewOfFile(
    lpBaseAddress: windows.LPCVOID,
    dwNumberOfBytesToFlush: windows.SIZE_T,
) callconv(.winapi) windows.BOOL;

pub const DBWriter = struct {
    file: File,
    handle: if (builtin.target.os.tag == .windows)
        windows.HANDLE
    else
        void,
    block: *align(heap.page_size_min) SuperBlock,
    file_len: u64,
    page_size: usize,
    session_table: *align(heap.page_size_min) SessionTable,
    event_info_table: *align(heap.page_size_min) EventInfoTable,
    data_table: *align(heap.page_size_min) DataTable,
    event_table: *align(heap.page_size_min) EventTable,

    const FILE_MAP_WRITE: windows.DWORD = 2;
    const FILE_MAP_READ: windows.DWORD = 4;
    const FILE_MAP_READWRITE: windows.DWORD = FILE_MAP_READ | FILE_MAP_WRITE;

    pub fn init(path: []const u8) !DBWriter {
        if (builtin.target.cpu.arch.endian() != .little) return error.ExpectedLittleEndian;
        const file = try fs.cwd().createFile(path, .{ .read = true, .lock = .exclusive });
        errdefer file.close();

        const page_size = heap.pageSize();
        const file_len = 5 * page_size;
        try file.setEndPos(file_len);
        const sessions_start = mem.alignForward(u64, @sizeOf(SuperBlock), page_size);
        const event_infos_start = mem.alignForward(
            u64,
            sessions_start + @sizeOf(SessionTable),
            page_size,
        );
        const data_start = mem.alignForward(
            u64,
            event_infos_start + @sizeOf(EventInfoTable),
            page_size,
        );
        const events_start = mem.alignForward(
            u64,
            data_start + @sizeOf(DataTable),
            page_size,
        );

        const handle, const block = if (comptime builtin.target.os.tag == .windows) blk: {
            const res = CreateFileMappingA(file.handle, null, windows.PAGE_READWRITE, 0, 0, null);
            const handle = res orelse return error.FileMappingFailed;
            errdefer windows.CloseHandle(handle);
            const res2 = MapViewOfFileEx(handle, FILE_MAP_READWRITE, 0, 0, 0, null);
            const mapping = res2 orelse return error.FileMappingFailed;
            const block: *align(heap.page_size_min) SuperBlock = @ptrCast(@alignCast(mapping));
            break :blk .{ handle, block };
        } else blk: {
            const mapping = try posix.mmap(
                null,
                file_len,
                posix.PROT.READ | posix.PROT.WRITE,
                .{ .TYPE = .SHARED },
                file.handle,
                0,
            );
            const block: *align(heap.page_size_min) SuperBlock = @ptrCast(@alignCast(mapping));
            break :blk .{ {}, block };
        };

        block.* = .{
            .page_size = page_size,
            .session_table = @enumFromInt(sessions_start),
            .event_info_table = @enumFromInt(event_infos_start),
            .data_table = @enumFromInt(data_start),
            .event_table = @enumFromInt(events_start),
        };

        const session_table: *align(heap.page_size_min) SessionTable =
            @ptrFromInt(@intFromPtr(block) + sessions_start);
        session_table.* = .{
            .table_end = @enumFromInt(page_size),
            .num_entries = 0,
        };

        const event_info_table: *align(heap.page_size_min) EventInfoTable =
            @ptrFromInt(@intFromPtr(block) + event_infos_start);
        event_info_table.* = .{
            .table_end = @enumFromInt(page_size),
            .capacity = 0,
            .num_entries = 0,
        };

        const data_table: *align(heap.page_size_min) DataTable =
            @ptrFromInt(@intFromPtr(block) + data_start);
        data_table.* = .{
            .table_end = @enumFromInt(page_size),
            .capacity = 0,
            .num_entries = 0,
            .buffer_start = @enumFromInt(0),
            .buffer_end = @enumFromInt(0),
        };

        const event_table: *align(heap.page_size_min) EventTable =
            @ptrFromInt(@intFromPtr(block) + events_start);
        event_table.* = .{
            .table_end = @enumFromInt(page_size),
            .num_entries = 0,
        };

        return .{
            .file = file,
            .handle = handle,
            .block = block,
            .file_len = file_len,
            .page_size = page_size,
            .session_table = session_table,
            .event_info_table = event_info_table,
            .data_table = data_table,
            .event_table = event_table,
        };
    }

    pub fn deinit(self: *DBWriter) void {
        self.flush() catch {};
        if (comptime builtin.target.os.tag == .windows) {
            _ = UnmapViewOfFile(self.block);
            windows.CloseHandle(self.handle);
        } else {
            const ptr: [*]align(heap.page_size_min) u8 = @ptrCast(@alignCast(self.block));
            posix.munmap(ptr[0..self.file_len]);
        }

        self.file.close();
        self.* = undefined;
    }

    pub fn flush(self: *DBWriter) !void {
        if (comptime builtin.target.os.tag == .windows) {
            if (FlushViewOfFile(self.block, 0) == 0) return error.FlushFailed;
            if (windows.kernel32.FlushFileBuffers(self.file.handle) == 0) return error.FlushFailed;
        } else {
            const ptr: [*]align(heap.page_size_min) u8 = @ptrCast(@alignCast(self.block));
            try posix.msync(ptr[0..self.file_len], posix.MSF.SYNC);
        }
    }

    fn extendFile(self: *DBWriter, additional: u64) !u64 {
        const rounded = mem.alignForward(u64, additional, self.page_size);
        try self.file.setEndPos(self.file_len + rounded);
        self.file_len += rounded;

        if (comptime builtin.target.os.tag == .windows) {
            const res = CreateFileMappingA(self.file.handle, null, windows.PAGE_READWRITE, 0, 0, null);
            const handle = res orelse return error.FileMappingFailed;
            errdefer windows.CloseHandle(handle);
            const res2 = MapViewOfFileEx(handle, FILE_MAP_READWRITE, 0, 0, 0, null);
            const mapping = res2 orelse return error.FileMappingFailed;

            _ = UnmapViewOfFile(self.block);
            windows.CloseHandle(self.handle);
            self.handle = handle;
            self.block = @ptrCast(@alignCast(mapping));
        } else {
            const mapping = try posix.mmap(
                null,
                self.file_len,
                posix.PROT.READ | posix.PROT.WRITE,
                .{ .TYPE = .SHARED },
                self.file.handle,
                0,
            );
            const ptr: [*]align(heap.page_size_min) u8 = @ptrCast(@alignCast(self.block));
            posix.munmap(ptr[0 .. self.file_len - rounded]);
            self.block = @ptrCast(@alignCast(mapping));
        }

        self.session_table = @ptrFromInt(@intFromPtr(self.block) + @intFromEnum(self.block.session_table));
        self.event_info_table = @ptrFromInt(@intFromPtr(self.block) + @intFromEnum(self.block.event_info_table));
        self.data_table = @ptrFromInt(@intFromPtr(self.block) + @intFromEnum(self.block.data_table));
        self.event_table = @ptrFromInt(@intFromPtr(self.block) + @intFromEnum(self.block.event_table));

        return rounded;
    }

    fn getLastSession(self: *DBWriter) *Session {
        debug.assert(self.session_table.num_entries != 0);
        const sessions_start = mem.alignForward(u64, @sizeOf(SessionTable), @alignOf(Session));
        const sessions: [*]Session = @ptrFromInt(@intFromPtr(self.session_table) + sessions_start);
        return &sessions[self.session_table.num_entries - 1];
    }

    pub fn internEventInfo(
        self: *DBWriter,
        id: EventInfoId,
        name: []const u8,
        target: []const u8,
        scope: []const u8,
        file_name: []const u8,
        line_number: u32,
        level: EventLevel,
    ) !EventInfoRef {
        const capacity = if (3 * self.event_info_table.capacity < 4 * (self.event_info_table.num_entries + 1))
            @max(1, self.event_info_table.capacity * 2)
        else
            self.event_info_table.capacity;
        const header_size = mem.alignForward(u64, capacity, 8) / 8;
        const buckets_size = @sizeOf(EventInfoTableBucket) * capacity;
        const entries_size = @sizeOf(EventInfo) * capacity;

        const header_start = @sizeOf(EventInfoTable);
        const buckets_start = mem.alignForward(u64, header_start + header_size, @alignOf(EventInfoTableBucket));
        const entries_start = mem.alignForward(u64, buckets_start + buckets_size, @alignOf(EventInfo));
        const entries_end = entries_start + entries_size;

        // Shift the file, if the table ran out of space.
        if (@intFromEnum(self.event_info_table.table_end) < entries_end) {
            const additional = entries_end - @intFromEnum(self.event_info_table.table_end);
            const offset = try self.extendFile(additional);
            const shift_start = @intFromEnum(self.block.data_table);
            const file_end = @intFromEnum(self.block.event_table) + @intFromEnum(self.event_table.table_end);
            const bytes: [*]u8 = @ptrCast(self.block);
            const src = bytes[shift_start..file_end];
            const dst = bytes[shift_start + offset .. file_end + offset];
            @memmove(dst, src);

            self.event_info_table.table_end = @enumFromInt(@intFromEnum(self.event_info_table.table_end) + offset);
            self.block.data_table = @enumFromInt(@intFromEnum(self.block.data_table) + offset);
            self.block.event_table = @enumFromInt(@intFromEnum(self.block.event_table) + offset);

            self.data_table = @ptrFromInt(@intFromPtr(self.block) + @intFromEnum(self.block.data_table));
            self.event_table = @ptrFromInt(@intFromPtr(self.block) + @intFromEnum(self.block.event_table));
        }

        const name_ref = try self.internString(name);
        const target_ref = try self.internString(target);
        const scope_ref = try self.internString(scope);
        const file_name_ref = try self.internString(file_name);

        const bytes: [*]u8 = @ptrCast(self.event_info_table);
        const header: []u8 = bytes[header_start .. header_start + header_size];
        const buckets: []EventInfoTableBucket = @ptrCast(@alignCast(bytes[buckets_start .. buckets_start + buckets_size]));
        const entries: []EventInfo = @ptrCast(@alignCast(bytes[entries_start..entries_end]));

        // Grow the hash table, if we need more capacity.
        if (capacity != self.event_info_table.capacity and capacity > 1) {
            const old_capacity = self.event_info_table.capacity;
            const old_header_size = mem.alignForward(u64, old_capacity, 8) / 8;
            const old_buckets_size = @sizeOf(EventInfoTableBucket) * old_capacity;
            const old_entries_size = @sizeOf(EventInfo) * old_capacity;

            const old_buckets_start = mem.alignForward(u64, header_start + old_header_size, @alignOf(EventInfoTableBucket));
            const old_entries_start = mem.alignForward(u64, old_buckets_start + old_buckets_size, @alignOf(EventInfo));
            const old_entries_end = old_entries_start + old_entries_size;
            const src = bytes[old_entries_start..old_entries_end];
            const dst = bytes[entries_start .. entries_start + old_entries_size];
            @memmove(dst, src);
            rehashEventInfoTable(header, buckets, entries[0..self.event_info_table.num_entries]);
        }

        // Insert the entry.
        const ref = putEventInfoTableValue(
            header,
            buckets,
            entries,
            &self.event_info_table.num_entries,
            .{
                .id = id,
                .name = name_ref,
                .target = target_ref,
                .scope = scope_ref,
                .file_name = file_name_ref,
                .line_number = line_number,
                .level = level,
            },
        );
        self.event_info_table.capacity = capacity;
        return ref;
    }

    pub fn internStruct(self: *DBWriter, value: anytype) !DataRef {
        const T = @TypeOf(value);
        if (@typeInfo(T) != .@"struct") @compileError("DBWriter: expected struct, got " ++ @typeName(T));
        if (comptime !meta.hasUniqueRepresentation(T)) @compileError("DBWriter: type has no unique bit pattern, got " ++ @typeName(T));
        if (@typeInfo(T).@"struct".layout == .auto) @compileError("DBWriter: `auto` structs are not supported, got " ++ @typeName(T));
        return self.internData(mem.asBytes(&value), @alignOf(T));
    }

    pub fn internString(self: *DBWriter, value: []const u8) !DataRef {
        return self.internData(value, 1);
    }

    pub fn internData(self: *DBWriter, value: []const u8, alignment: u64) !DataRef {
        const capacity = if (3 * self.data_table.capacity < 4 * (self.data_table.num_entries + 1))
            @max(1, self.data_table.capacity * 2)
        else
            self.data_table.capacity;
        const header_size = mem.alignForward(u64, capacity, 8) / 8;
        const buckets_size = @sizeOf(DataTableBucket) * capacity;
        const entries_size = @sizeOf(DataTableEntry) * capacity;
        const buffer_size = @as(u64, value.len) +
            (mem.alignForward(u64, @intFromEnum(self.data_table.buffer_end), alignment) -
                @intFromEnum(self.data_table.buffer_start));

        const header_start = @sizeOf(DataTable);
        const buckets_start = mem.alignForward(u64, header_start + header_size, @alignOf(DataTableBucket));
        const entries_start = mem.alignForward(u64, buckets_start + buckets_size, @alignOf(DataTableEntry));
        const buffer_start = mem.alignForward(u64, entries_start + entries_size, data_table_buffer_alignment);
        const buffer_end = buffer_start + buffer_size;

        // Shift the file, if the table ran out of space.
        if (@intFromEnum(self.data_table.table_end) < buffer_end) {
            const additional = buffer_end - @intFromEnum(self.data_table.table_end);
            const offset = try self.extendFile(additional);
            const shift_start = @intFromEnum(self.block.event_table);
            const file_end = @intFromEnum(self.block.event_table) + @intFromEnum(self.event_table.table_end);
            const bytes: [*]u8 = @ptrCast(self.block);
            const src = bytes[shift_start..file_end];
            const dst = bytes[shift_start + offset .. file_end + offset];
            @memmove(dst, src);

            self.data_table.table_end = @enumFromInt(@intFromEnum(self.data_table.table_end) + offset);
            self.block.event_table = @enumFromInt(@intFromEnum(self.block.event_table) + offset);

            self.event_table = @ptrFromInt(@intFromPtr(self.block) + @intFromEnum(self.block.event_table));
        }

        const bytes: [*]u8 = @ptrCast(self.data_table);
        const header: []u8 = bytes[header_start .. header_start + header_size];
        const buckets: []DataTableBucket = @ptrCast(@alignCast(bytes[buckets_start .. buckets_start + buckets_size]));
        const entries: []DataTableEntry = @ptrCast(@alignCast(bytes[entries_start .. entries_start + entries_size]));
        const buffer: []u8 = @ptrCast(bytes[buffer_start..buffer_end]);

        // Grow the hash table, if we need more capacity.
        if (capacity != self.data_table.capacity and capacity > 1) {
            const old_capacity = self.data_table.capacity;
            const old_header_size = mem.alignForward(u64, old_capacity, 8) / 8;
            const old_buckets_size = @sizeOf(DataTableBucket) * old_capacity;
            const old_entries_size = @sizeOf(DataTableEntry) * old_capacity;

            const old_buckets_start = mem.alignForward(u64, header_start + old_header_size, @alignOf(DataTableBucket));
            const old_entries_start = mem.alignForward(u64, old_buckets_start + old_buckets_size, @alignOf(DataTableEntry));
            const old_entries_end = old_entries_start + old_entries_size;
            const old_buffer_start = @intFromEnum(self.data_table.buffer_start);
            const old_buffer_end = @intFromEnum(self.data_table.buffer_end);
            const buffer_shift_size = old_buffer_end - old_buffer_start;
            const buffer_src = bytes[old_buffer_start..old_buffer_end];
            const buffer_dst = bytes[buffer_start .. buffer_start + buffer_shift_size];
            @memmove(buffer_dst, buffer_src);

            const entries_shift_size = old_entries_size;
            const entries_src = bytes[old_entries_start..old_entries_end];
            const entries_dst = bytes[entries_start .. entries_start + entries_shift_size];
            @memmove(entries_dst, entries_src);
            rehashDataTable(header, buckets, entries[0..self.data_table.num_entries], buffer);
        }

        // Insert the value into the buffer.
        var end = @intFromEnum(self.data_table.buffer_end) - @intFromEnum(self.data_table.buffer_start);
        const ref = putDataTableValue(
            header,
            buckets,
            entries,
            &self.data_table.num_entries,
            buffer,
            &end,
            value,
            alignment,
        );

        self.data_table.capacity = capacity;
        self.data_table.buffer_start = @enumFromInt(buffer_start);
        self.data_table.buffer_end = @enumFromInt(buffer_start + end);
        return ref;
    }

    fn writeEvent(self: *DBWriter, event: anytype) !void {
        const T = @TypeOf(event);
        if (@sizeOf(T) > max_event_size) @compileError("DBWriter: event size too large, got " ++ @typeName(T));
        if (@alignOf(T) > max_event_alignment) @compileError("DBWriter: event alignment too large, got " ++ @typeName(T));
        if (@typeInfo(T) != .@"struct") @compileError("DBWriter: expected struct, got " ++ @typeName(T));
        if (@typeInfo(T).@"struct".layout == .auto) @compileError("DBWriter: `auto` structs are not supported, got " ++ @typeName(T));
        if (@typeInfo(T).@"struct".fields[0].type != Instant) @compileError("DBWriter: first element of struct must be an Instant");
        if (@typeInfo(T).@"struct".fields[1].type != EventTag) @compileError("DBWriter: second element of struct must be an EventTag");
        if (comptime !meta.hasUniqueRepresentation(T)) @compileError("DBWriter: type has no unique bit pattern, got " ++ @typeName(T));

        const entries_start = mem.alignForward(u64, @sizeOf(EventTable), max_event_alignment);
        const entries_end = entries_start + ((self.event_table.num_entries + 1) * max_event_size);

        // Expand the file to fit the events.
        if (@intFromEnum(self.event_table.table_end) < entries_end) {
            const additional = entries_end - @intFromEnum(self.event_table.table_end);
            const offset = try self.extendFile(additional);
            self.event_table.table_end = @enumFromInt(@intFromEnum(self.event_table.table_end) + offset);
        }

        const session = self.getLastSession();

        var ev: OpaqueEvent = mem.zeroInit(OpaqueEvent, .{});
        @memcpy(mem.asBytes(&ev)[0..@sizeOf(T)], mem.asBytes(&event));

        const bytes: [*]u8 = @ptrCast(self.event_table);
        const entries: []OpaqueEvent = @ptrCast(@alignCast(bytes[entries_start..entries_end]));

        const Context = struct {
            t: u64,
            fn compare(ctx: @This(), e: OpaqueEvent) math.Order {
                const t = @intFromEnum(e.time);
                return math.order(ctx.t, t);
            }
        };
        const old_events: []OpaqueEvent = entries[0..self.event_table.num_entries];
        const insertion_idx = std.sort.upperBound(
            OpaqueEvent,
            old_events,
            Context{ .t = @intFromEnum(ev.time) },
            Context.compare,
        );
        if (insertion_idx != old_events.len) {
            @memmove(entries[insertion_idx + 1 ..], entries[insertion_idx..old_events.len]);
        }
        entries[insertion_idx] = ev;

        const start_time = @min(@intFromEnum(ev.time), @intFromEnum(session.start_time));
        const end_time = @max(@intFromEnum(ev.time), @intFromEnum(session.end_time));

        session.start_time = @enumFromInt(start_time);
        session.end_time = @enumFromInt(end_time);
        session.num_events += 1;
        self.event_table.num_entries += 1;
    }

    pub fn startSession(
        self: *DBWriter,
        time: Instant,
        epoch: Time,
        resolution: Duration,
        available_memory: u64,
        process_id: u64,
        num_cores: u16,
        cpu_arch: CpuArch,
        cpu_id: u32,
        cpu_vendor: []const u8,
        app_name: []const u8,
        host_info: []const u8,
    ) !void {
        const sessions_start = mem.alignForward(u64, @sizeOf(SessionTable), @alignOf(Session));
        const sessions_end = sessions_start + ((self.session_table.num_entries + 1) * @sizeOf(Session));
        if (@intFromEnum(self.session_table.table_end) < sessions_end) {
            const additional = sessions_end - @intFromEnum(self.session_table.table_end);
            const offset = try self.extendFile(additional);
            const shift_start = @intFromEnum(self.block.event_info_table);
            const file_end = @intFromEnum(self.block.event_table) + @intFromEnum(self.event_table.table_end);
            const bytes: [*]u8 = @ptrCast(self.block);
            const src = bytes[shift_start..file_end];
            const dst = bytes[shift_start + offset .. file_end + offset];
            @memmove(dst, src);

            self.session_table.table_end = @enumFromInt(@intFromEnum(self.session_table.table_end) + offset);
            self.block.event_info_table = @enumFromInt(@intFromEnum(self.block.event_info_table) + offset);
            self.block.data_table = @enumFromInt(@intFromEnum(self.block.data_table) + offset);
            self.block.event_table = @enumFromInt(@intFromEnum(self.block.event_table) + offset);

            self.event_info_table = @ptrFromInt(@intFromPtr(self.block) + @intFromEnum(self.block.event_info_table));
            self.data_table = @ptrFromInt(@intFromPtr(self.block) + @intFromEnum(self.block.data_table));
            self.event_table = @ptrFromInt(@intFromPtr(self.block) + @intFromEnum(self.block.event_table));
        }

        const cpu_vendor_ref = try self.internString(cpu_vendor);
        const app_name_ref = try self.internString(app_name);
        const host_info_ref = try self.internString(host_info);
        const sessions: [*]Session = @ptrFromInt(@intFromPtr(self.session_table) + sessions_start);
        sessions[self.session_table.num_entries] = .{
            .start_time = time,
            .end_time = time,
            .epoch = epoch,
            .events_start = @enumFromInt(self.event_table.num_entries),
            .num_events = 0,
            .resolution = resolution,
            .available_memory = available_memory,
            .process_id = process_id,
            .num_cores = num_cores,
            .cpu_arch = cpu_arch,
            .cpu_id = cpu_id,
            .cpu_vendor = cpu_vendor_ref,
            .app_name = app_name_ref,
            .host_info = host_info_ref,
        };
        self.session_table.num_entries += 1;
    }

    pub fn finishSession(self: *DBWriter, time: Instant) !void {
        const session = self.getLastSession();
        if (@intFromEnum(session.end_time) > @intFromEnum(time)) return error.UnexpectedTime;
        session.end_time = time;
    }

    pub fn registerThread(self: *DBWriter, time: Instant, thread_id: ThreadId) !void {
        try self.writeEvent(RegisterThread{ .time = time, .thread_id = thread_id });
    }

    pub fn unregisterThread(self: *DBWriter, time: Instant, thread_id: ThreadId) !void {
        try self.writeEvent(UnregisterThread{ .time = time, .thread_id = thread_id });
    }

    pub fn createCallStack(self: *DBWriter, time: Instant, stack: StackRef) !void {
        try self.writeEvent(CreateCallStack{ .time = time, .stack = stack });
    }

    pub fn destroyCallStack(self: *DBWriter, time: Instant, stack: StackRef) !void {
        try self.writeEvent(DestroyCallStack{ .time = time, .stack = stack });
    }

    pub fn unblockCallStack(self: *DBWriter, time: Instant, stack: StackRef) !void {
        try self.writeEvent(UnblockCallStack{ .time = time, .stack = stack });
    }

    pub fn suspendCallStack(
        self: *DBWriter,
        time: Instant,
        stack: StackRef,
        mark_blocked: bool,
    ) !void {
        try self.writeEvent(SuspendCallStack{
            .time = time,
            .stack = stack,
            .flags = .{ .mark_blocked = mark_blocked },
        });
    }

    pub fn resumeCallStack(
        self: *DBWriter,
        time: Instant,
        stack: StackRef,
        thread_id: ThreadId,
    ) !void {
        try self.writeEvent(ResumeCallStack{
            .time = time,
            .stack = stack,
            .thread_id = thread_id,
        });
    }

    pub fn enterSpan(
        self: *DBWriter,
        time: Instant,
        stack: StackRef,
        info: EventInfoId,
        message: []const u8,
    ) !void {
        const message_ref = try self.internString(message);
        const extra = try self.internStruct(EnterSpanExt{
            .info = info,
            .message = message_ref,
        });
        try self.writeEvent(EnterSpan{
            .time = time,
            .stack = stack,
            .extra = extra,
        });
    }

    pub fn exitSpan(
        self: *DBWriter,
        time: Instant,
        stack: StackRef,
        is_unwinding: bool,
    ) !void {
        try self.writeEvent(ExitSpan{
            .time = time,
            .stack = stack,
            .flags = .{ .is_unwinding = is_unwinding },
        });
    }

    pub fn logMessage(
        self: *DBWriter,
        time: Instant,
        stack: StackRef,
        info: EventInfoId,
        message: []const u8,
    ) !void {
        const message_ref = try self.internString(message);
        const extra = try self.internStruct(LogMessageExt{
            .info = info,
            .message = message_ref,
        });
        try self.writeEvent(LogMessage{
            .time = time,
            .stack = stack,
            .extra = extra,
        });
    }
};

pub const DBReader = struct {
    block: *align(heap.page_size_min) const SuperBlock,
    handle: if (builtin.target.os.tag == .windows)
        windows.HANDLE
    else
        usize,

    pub fn init(path: []const u8) !DBReader {
        if (builtin.target.cpu.arch.endian() != .little) return error.ExpectedLittleEndian;
        const file = try fs.cwd().openFile(path, .{ .lock = .shared });
        defer file.close();
        var buffer: [@sizeOf(SuperBlock)]u8 = undefined;
        var reader = file.reader(&buffer);
        const header = try reader.interface.takeStruct(SuperBlock, .little);
        if (!std.mem.eql(u8, &header.file_magic, SuperBlock.expect_magic)) return error.InvalidDB;
        if (header.version_major != SuperBlock.expect_version_major) return error.InvalidDB;
        if (header.version_minor > SuperBlock.expect_version_minor) return error.InvalidDB;
        if (header.page_size < heap.page_size_min) return error.InvalidDB;

        if (comptime builtin.target.os.tag == .windows) {
            const handle = CreateFileMappingA(file.handle, null, windows.PAGE_READONLY, 0, 0, null) orelse
                return error.FileMappingFailed;
            errdefer std.os.windows.CloseHandle(handle);

            const mapping = MapViewOfFileEx(handle, DBWriter.FILE_MAP_READ, 0, 0, 0, null) orelse
                return error.FileMappingFailed;
            errdefer _ = UnmapViewOfFile(mapping);
            return .{
                .block = @ptrCast(@alignCast(mapping)),
                .handle = handle,
            };
        } else {
            const file_len = try file.getEndPos();
            const mapping = try posix.mmap(
                null,
                file_len,
                posix.PROT.READ,
                .{ .TYPE = .SHARED },
                file.handle,
                0,
            );
            return .{
                .block = @ptrCast(@alignCast(mapping)),
                .handle = mapping.len,
            };
        }
    }

    pub fn deinit(self: *DBReader) void {
        if (comptime builtin.target.os.tag == .windows) {
            _ = UnmapViewOfFile(self.block);
            windows.CloseHandle(self.handle);
        } else {
            const ptr: [*]align(heap.page_size_min) const u8 = @ptrCast(@alignCast(self.block));
            posix.munmap(ptr[0..self.handle]);
        }
        self.* = undefined;
    }

    pub fn getSessionTable(self: *const DBReader) *align(heap.page_size_min) const SessionTable {
        const bytes: [*]const u8 = @ptrCast(self.block);
        const offset = @intFromEnum(self.block.session_table);
        return @ptrCast(@alignCast(bytes + offset));
    }

    pub fn getAllSessions(self: *const DBReader) []const Session {
        const table = self.getSessionTable();
        const sessions_start = mem.alignForward(u64, @sizeOf(SessionTable), @alignOf(Session));
        const bytes: [*]const u8 = @ptrCast(table);
        const sessions: [*]const Session = @ptrCast(@alignCast(bytes + sessions_start));
        return sessions[0..table.num_entries];
    }

    pub fn getSession(self: *const DBReader, idx: u64) *const Session {
        const sessions = self.getAllSessions();
        return &sessions[idx];
    }

    pub fn getSessionLowerBound(self: *const DBReader, time: Instant) u64 {
        const Context = struct {
            t: u64,
            fn compare(ctx: @This(), session: Session) math.Order {
                const t = @intFromEnum(session.start_time);
                return math.order(ctx.t, t);
            }
        };
        const sessions = self.getAllSessions();
        return std.sort.lowerBound(Session, sessions, Context{ .t = @enumFromInt(time) }, Context.compare);
    }

    pub fn getSessionUpperBound(self: *const DBReader, time: Instant) u64 {
        const Context = struct {
            t: u64,
            fn compare(ctx: @This(), session: Session) math.Order {
                const t = @intFromEnum(session.end_time);
                return math.order(ctx.t, t);
            }
        };
        const sessions = self.getAllSessions();
        return std.sort.upperBound(Session, sessions, Context{ .t = @enumFromInt(time) }, Context.compare);
    }

    pub fn getEventInfoTable(self: *const DBReader) *align(heap.page_size_min) const EventInfoTable {
        const bytes: [*]const u8 = @ptrCast(self.block);
        const offset = @intFromEnum(self.block.event_info_table);
        return @ptrCast(@alignCast(bytes + offset));
    }

    pub fn getEventInfoById(self: *const DBReader, id: EventInfoId) ?*const EventInfo {
        const table = self.getEventInfoTable();
        const header_size = mem.alignForward(u64, table.capacity, 8) / 8;
        const buckets_size = @sizeOf(EventInfoTableBucket) * table.capacity;
        const entries_size = @sizeOf(EventInfo) * table.capacity;

        const header_start = @sizeOf(EventInfoTable);
        const header_end = header_start + header_size;
        const buckets_start = mem.alignForward(
            u64,
            header_start + header_size,
            @alignOf(EventInfoTableBucket),
        );
        const buckets_end = buckets_start + buckets_size;
        const entries_start = mem.alignForward(
            u64,
            buckets_start + buckets_size,
            @alignOf(EventInfo),
        );
        const entries_end = entries_start + entries_size;

        const bytes: [*]const u8 = @ptrCast(table);
        const header: []const u8 = @ptrCast(bytes[header_start..header_end]);
        const buckets: []const EventInfoTableBucket = @ptrCast(@alignCast(bytes[buckets_start..buckets_end]));
        const entries: []const EventInfo = @ptrCast(@alignCast(bytes[entries_start..entries_end]));
        const ref = getEventInfoTableValue(header, buckets, entries, id) orelse return null;
        return &entries[@intFromEnum(ref)];
    }

    pub fn getEventInfoByRef(self: *const DBReader, ref: EventInfoRef) *const EventInfo {
        const table = self.getEventInfoTable();
        debug.assert(@intFromEnum(ref) < table.num_entries);
        const header_size = mem.alignForward(u64, table.capacity, 8) / 8;
        const buckets_size = @sizeOf(EventInfoTableBucket) * table.capacity;

        const header_start = @sizeOf(EventInfoTable);
        const buckets_start = mem.alignForward(
            u64,
            header_start + header_size,
            @alignOf(EventInfoTableBucket),
        );
        const entries_start = mem.alignForward(
            u64,
            buckets_start + buckets_size,
            @alignOf(EventInfo),
        );

        const bytes: [*]const u8 = @ptrCast(table);
        const entries: [*]const EventInfo = @ptrCast(@alignCast(bytes + entries_start));
        return &entries[@intFromEnum(ref)];
    }

    pub fn getDataTable(self: *const DBReader) *align(heap.page_size_min) const DataTable {
        const bytes: [*]const u8 = @ptrCast(self.block);
        const offset = @intFromEnum(self.block.data_table);
        return @ptrCast(@alignCast(bytes + offset));
    }

    pub fn getInternedSlice(self: *const DBReader, T: type, ref: DataRef) []const T {
        if (comptime !meta.hasUniqueRepresentation(T)) @compileError("DBReader: type has no unique bit pattern, got " ++ @typeName(T));
        switch (@typeInfo(T)) {
            .int, .float, .@"enum" => {},
            .@"struct" => |v| {
                if (v.layout == .auto) @compileError("DBReader: `auto` structs are not supported, got " ++ @typeName(T));
            },
            .@"union" => |v| {
                if (v.layout == .auto) @compileError("DBReader: `auto` unions are not supported, got " ++ @typeName(T));
            },
            else => @compileError("DBReader: type not supported, got " ++ @typeName(T)),
        }

        const table = self.getDataTable();
        debug.assert(@intFromEnum(ref) < table.num_entries);
        const header_size = mem.alignForward(u64, table.capacity, 8) / 8;
        const buckets_size = @sizeOf(DataTableBucket) * table.capacity;

        const header_start = @sizeOf(DataTable);
        const buckets_start = mem.alignForward(u64, header_start + header_size, @alignOf(DataTableBucket));
        const entries_start = mem.alignForward(u64, buckets_start + buckets_size, @alignOf(DataTableEntry));

        const bytes: [*]const u8 = @ptrCast(table);
        const entries: [*]const DataTableEntry = @ptrCast(@alignCast(bytes + entries_start));
        const buffer: []const u8 = bytes[@intFromEnum(table.buffer_start)..@intFromEnum(table.buffer_end)];
        const entry = entries[@intFromEnum(ref)];
        const entry_slice = buffer[@intFromEnum(entry.start)..@intFromEnum(entry.end)];
        return @ptrCast(@alignCast(entry_slice));
    }

    pub fn getInternedValue(self: *const DBReader, T: type, ref: DataRef) *const T {
        const slice = self.getInternedSlice(T, ref);
        debug.assert(slice.len == 1);
        return &slice[0];
    }

    pub fn getEventTable(self: *const DBReader) *align(heap.page_size_min) const EventTable {
        const bytes: [*]const u8 = @ptrCast(self.block);
        const offset = @intFromEnum(self.block.event_table);
        return @ptrCast(@alignCast(bytes + offset));
    }

    pub fn getAllEvents(self: *const DBReader) []const OpaqueEvent {
        const table = self.getEventTable();
        const entries_start = mem.alignForward(u64, @sizeOf(EventTable), max_event_alignment);
        const entries_end = entries_start + (table.num_entries * max_event_size);

        const bytes: [*]const u8 = @ptrCast(table);
        return @ptrCast(@alignCast(bytes[entries_start..entries_end]));
    }

    pub fn getEventsLowerBound(self: *const DBReader, time: Instant) u64 {
        const Context = struct {
            t: u64,
            fn compare(ctx: @This(), event: OpaqueEvent) math.Order {
                const t = @intFromEnum(event.time);
                return math.order(ctx.t, t);
            }
        };
        const events = self.getAllEvents();
        return std.sort.lowerBound(OpaqueEvent, events, Context{ .t = @enumFromInt(time) }, Context.compare);
    }

    pub fn getEventsUpperBound(self: *const DBReader, time: Instant) u64 {
        const Context = struct {
            t: u64,
            fn compare(ctx: @This(), event: OpaqueEvent) math.Order {
                const t = @intFromEnum(event.time);
                return math.order(ctx.t, t);
            }
        };
        const events = self.getAllEvents();
        return std.sort.upperBound(OpaqueEvent, events, Context{ .t = @enumFromInt(time) }, Context.compare);
    }

    pub fn getSessionEvents(self: *const DBReader, session_idx: u64) []const OpaqueEvent {
        const session = self.getSession(session_idx);
        const events = self.getAllEvents();
        const start = @intFromEnum(session.events_start);
        const end = start + session.num_events;
        return events[start..end];
    }
};

test {
    const epoch: Time = @enumFromInt(12345);
    const resolution: Duration = @enumFromInt(100);
    const available_memory = 512;
    const process_id = 1997;
    const num_cores = 256;
    const cpu_arch: CpuArch = .aarch64;
    const cpu_id = 99999;
    const cpu_vendor = "test cpu";
    const app_name = "test app";
    const host_info = "test host";
    const thread_id: ThreadId = @enumFromInt(987654321);
    const stack: StackRef = @enumFromInt(147258369);
    const message = "test message";

    const start_time: Instant = @enumFromInt(5);
    const register_time: Instant = @enumFromInt(15);
    const unregister_time: Instant = @enumFromInt(30);
    const log_time: Instant = @enumFromInt(10);
    const end_time: Instant = @enumFromInt(1000);

    const event_info_id: EventInfoId = @enumFromInt(963852741);
    const name = "test event";
    const target = "test target";
    const scope = "test scope";
    const file_name = "test.zig";
    const line_number = 1305;
    const level: EventLevel = .trace;

    const path = "test.ftrdb";
    defer std.fs.cwd().deleteFile(path) catch {};
    {
        var writer: DBWriter = try .init(path);
        errdefer writer.deinit();
        const event_info = try writer.internEventInfo(
            event_info_id,
            name,
            target,
            scope,
            file_name,
            line_number,
            level,
        );
        try testing.expectEqual(event_info, try writer.internEventInfo(
            event_info_id,
            name,
            target,
            scope,
            file_name,
            line_number,
            level,
        ));

        try writer.startSession(
            start_time,
            epoch,
            resolution,
            available_memory,
            process_id,
            num_cores,
            cpu_arch,
            cpu_id,
            cpu_vendor,
            app_name,
            host_info,
        );
        try writer.registerThread(register_time, thread_id);
        try writer.unregisterThread(unregister_time, thread_id);
        try writer.logMessage(log_time, stack, event_info_id, message);
        try writer.finishSession(end_time);
        try writer.flush();
        writer.deinit();
    }

    var reader: DBReader = try .init(path);
    defer reader.deinit();

    const sessions = reader.getAllSessions();
    try testing.expectEqual(1, sessions.len);
    try testing.expectEqual(start_time, sessions[0].start_time);
    try testing.expectEqual(end_time, sessions[0].end_time);
    try testing.expectEqual(epoch, sessions[0].epoch);
    try testing.expectEqual(@as(EventRef, @enumFromInt(0)), sessions[0].events_start);
    try testing.expectEqual(3, sessions[0].num_events);
    try testing.expectEqual(resolution, sessions[0].resolution);
    try testing.expectEqual(available_memory, sessions[0].available_memory);
    try testing.expectEqual(process_id, sessions[0].process_id);
    try testing.expectEqual(num_cores, sessions[0].num_cores);
    try testing.expectEqual(cpu_arch, sessions[0].cpu_arch);
    try testing.expectEqual(cpu_id, sessions[0].cpu_id);
    try testing.expectEqualStrings(cpu_vendor, reader.getInternedSlice(u8, sessions[0].cpu_vendor));
    try testing.expectEqualStrings(app_name, reader.getInternedSlice(u8, sessions[0].app_name));
    try testing.expectEqualStrings(host_info, reader.getInternedSlice(u8, sessions[0].host_info));

    const events = reader.getSessionEvents(0);
    try testing.expectEqual(sessions[0].num_events, events.len);
    try testing.expectEqual(log_time, events[0].time);
    try testing.expectEqual(.log_message, events[0].tag);
    try testing.expectEqual(register_time, events[1].time);
    try testing.expectEqual(.register_thread, events[1].tag);
    try testing.expectEqual(unregister_time, events[2].time);
    try testing.expectEqual(.unregister_thread, events[2].tag);

    const log_message = events[0].bitCast(LogMessage);
    try testing.expectEqual(stack, log_message.stack);
    const log_message_extra = reader.getInternedValue(LogMessageExt, log_message.extra);
    try testing.expectEqualStrings(message, reader.getInternedSlice(u8, log_message_extra.message));

    const event_info = reader.getEventInfoById(log_message_extra.info).?;
    try testing.expectEqualStrings(name, reader.getInternedSlice(u8, event_info.name));
    try testing.expectEqualStrings(target, reader.getInternedSlice(u8, event_info.target));
    try testing.expectEqualStrings(target, reader.getInternedSlice(u8, event_info.target));
    try testing.expectEqualStrings(scope, reader.getInternedSlice(u8, event_info.scope));
    try testing.expectEqualStrings(file_name, reader.getInternedSlice(u8, event_info.file_name));
    try testing.expectEqual(line_number, event_info.line_number);
    try testing.expectEqual(level, event_info.level);

    const register_thread = events[1].bitCast(RegisterThread);
    try testing.expectEqual(thread_id, register_thread.thread_id);

    const unregister_thread = events[2].bitCast(UnregisterThread);
    try testing.expectEqual(thread_id, unregister_thread.thread_id);
}
