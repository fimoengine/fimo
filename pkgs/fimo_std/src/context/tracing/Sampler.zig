const std = @import("std");
const mem = std.mem;
const Thread = std.Thread;
const windows = std.os.windows;
const builtin = @import("builtin");

const win32 = @import("win32");
const zig = win32.zig;
const security = win32.security;
const foundation = win32.foundation;
const system = win32.system;
const threading = system.threading;
const etw = system.diagnostics.etw;
const system_services = system.system_services;

const paths = @import("../../paths.zig");
const Path = paths.Path;
const time = @import("../../time.zig");
const tracing = @import("../tracing.zig");

pub fn start() !void {
    return Impl.start();
}

pub fn stop() void {
    Impl.stop();
}

const Impl = switch (builtin.os.tag) {
    .windows => WindowsImpl,
    else => UnsupportedImpl,
};

const UnsupportedImpl = struct {
    fn start() !void {
        return error.SamplerNotSupported;
    }

    fn stop() void {}
};

// Adapted from the tracy profiler client:
//
// Tracy Profiler (https://github.com/wolfpld/tracy) is licensed under the
// 3-clause BSD license.
//
// Copyright (c) 2017-2025, Bartosz Taudul <wolf@nereid.pl>
// All rights reserved.
//
// Redistribution and use in source and binary forms, with or without
// modification, are permitted provided that the following conditions are met:
//     * Redistributions of source code must retain the above copyright
//       notice, this list of conditions and the following disclaimer.
//     * Redistributions in binary form must reproduce the above copyright
//       notice, this list of conditions and the following disclaimer in the
//       documentation and/or other materials provided with the distribution.
//     * Neither the name of the <organization> nor the
//       names of its contributors may be used to endorse or promote products
//       derived from this software without specific prior written permission.
//
// THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS AND CONTRIBUTORS "AS IS" AND
// ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE IMPLIED
// WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE ARE
// DISCLAIMED. IN NO EVENT SHALL <COPYRIGHT HOLDER> BE LIABLE FOR ANY
// DIRECT, INDIRECT, INCIDENTAL, SPECIAL, EXEMPLARY, OR CONSEQUENTIAL DAMAGES
// (INCLUDING, BUT NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS OR SERVICES;
// LOSS OF USE, DATA, OR PROFITS; OR BUSINESS INTERRUPTION) HOWEVER CAUSED AND
// ON ANY THEORY OF LIABILITY, WHETHER IN CONTRACT, STRICT LIABILITY, OR TORT
// (INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY OUT OF THE USE OF THIS
// SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF SUCH DAMAGE.

const WindowsImpl = struct {
    const ImageLoad = extern struct {
        ImageBase: u64,
        ImageSize: u64,
        ProcessId: u32,
        ImageChecksum: u32,
        TimeDateStamp: u32,
        SignatureLevel: u8,
        SignatureType: u8,
        Reserved0: u16,
        DefaultBase: u64,
        Reserved1: u32,
        Reserved2: u32,
        Reserved3: u32,
        Reserved4: u32,
        FileName: [0]u16,
    };
    const CSwitch = extern struct {
        NewThreadId: u32,
        OldThreadId: u32,
        NewThreadPriority: i8,
        OldThreadPriority: i8,
        PreviousCState: i8,
        SpareByte: i8,
        OldThreadWaitReason: i8,
        OldThreadWaitMode: i8,
        OldThreadState: i8,
        OldThreadWaitIdealProcessor: i8,
        NewThreadWaitTime: u32,
        Reserved: u32,
    };
    const ReadyThread = extern struct {
        TThreadId: u32,
        AdjustReason: i8,
        AdjustIncrement: i8,
        Flag: i8,
        Reserved: i8,
    };
    const Thread_TypeGroup1 = extern struct {
        ProcessId: u32,
        TThreadId: u32,
        StackBase: u32,
        StackLimit: u32,
        UserStackBase: u32,
        UserStackLimit: u32,
        StartAddr: u32,
        Win32StartAddr: u32,
        TebBase: u32,
        SubProcessTag: u32,
    };
    const StackWalk_Event = extern struct {
        EventTimeStamp: u64,
        StackProcess: u32,
        StackThread: u32,
        Stack: [192]u64,
    };
    const StackWalkGuid: zig.Guid = .initString("def2fe46-7bd6-4b80-bd94-f57fe20d0ce3");

    const sampling_frequency = 8000; // Hz
    const sampling_period = 1000000000 / sampling_frequency; // ns
    const sampling_interval = sampling_period / 100; // 100ns

    var running = false;
    var process_id: u32 = undefined;
    var properties: *etw.EVENT_TRACE_PROPERTIES = undefined;
    var trace_handle: etw.CONTROLTRACE_HANDLE = undefined;
    var trace_handle2: etw.CONTROLTRACE_HANDLE = undefined;
    var thread: Thread = undefined;

    fn start() !void {
        process_id = threading.GetCurrentProcessId();

        // Enable the privilege to gather profiling information.
        var privileges: security.TOKEN_PRIVILEGES = undefined;
        privileges.PrivilegeCount = 1;
        privileges.Privileges[0].Attributes = security.SE_PRIVILEGE_ENABLED;
        if (security.LookupPrivilegeValueA(null, system_services.SE_SYSTEM_PROFILE_NAME, &privileges.Privileges[0].Luid) == 0)
            return error.PivilegeFailure;
        {
            var process_token: ?windows.HANDLE = undefined;
            if (threading.OpenProcessToken(windows.GetCurrentProcess(), security.TOKEN_ADJUST_PRIVILEGES, &process_token) == 0)
                return error.PivilegeFailure;
            defer windows.CloseHandle(process_token.?);
            if (security.AdjustTokenPrivileges(process_token, windows.FALSE, &privileges, 0, null, null) == 0)
                return error.PivilegeFailure;
            if (windows.GetLastError() != .SUCCESS) return error.PivilegeFailure;
        }

        // Set the tracing frequency.
        var interval: etw.TRACE_PROFILE_INTERVAL = .{ .Source = 0, .Interval = sampling_interval };
        if (etw.TraceSetInformation(0, .TraceSampledProfileIntervalInfo, &interval, @truncate(@sizeOf(etw.TRACE_PROFILE_INTERVAL))) != .NO_ERROR)
            return error.SetFrequencyFailed;

        // Setup the trace.
        const buffer = try tracing.allocator.alignedAlloc(
            u8,
            .of(etw.EVENT_TRACE_PROPERTIES),
            @sizeOf(etw.EVENT_TRACE_PROPERTIES) + etw.KERNEL_LOGGER_NAME.len + 1,
        );
        errdefer tracing.allocator.free(buffer);
        @memset(buffer, 0);

        properties = @ptrCast(buffer[0..@sizeOf(etw.EVENT_TRACE_PROPERTIES)]);
        errdefer properties = undefined;
        properties.EnableFlags = .{
            .THREAD = 1,
            .IMAGE_LOAD = 1,
            .CSWITCH = 1,
            .DISPATCHER = 1,
            .PROFILE = 1,
        };
        properties.LogFileMode = etw.EVENT_TRACE_REAL_TIME_MODE;
        properties.Wnode.BufferSize = @truncate(buffer.len);
        properties.Wnode.Flags = etw.WNODE_FLAG_TRACED_GUID;
        properties.Wnode.ClientContext = 1; // Use Query performance counter
        properties.Wnode.Guid = etw.SystemTraceControlGuid;
        properties.BufferSize = 1024;
        properties.MinimumBuffers = @truncate((try Thread.getCpuCount()) * 4);
        properties.MaximumBuffers = @truncate((try Thread.getCpuCount()) * 6);
        properties.LoggerNameOffset = @truncate(@sizeOf(etw.EVENT_TRACE_PROPERTIES));
        @memcpy(buffer[@sizeOf(etw.EVENT_TRACE_PROPERTIES) .. buffer.len - 1], etw.KERNEL_LOGGER_NAME);

        // Disable any old trace.
        {
            const backup_buffer = try tracing.allocator.alignedAlloc(u8, .of(etw.EVENT_TRACE_PROPERTIES), buffer.len);
            defer tracing.allocator.free(backup_buffer);
            @memcpy(backup_buffer, buffer);
            defer @memcpy(buffer, backup_buffer);

            const status = etw.ControlTraceA(0, etw.KERNEL_LOGGER_NAME, properties, etw.EVENT_TRACE_CONTROL_STOP);
            if (status != .NO_ERROR and status != .ERROR_WMI_INSTANCE_NOT_FOUND) return error.TraceAlreadyRunning;
        }

        // Start the new tracing session.
        if (etw.StartTraceA(&trace_handle, etw.KERNEL_LOGGER_NAME, properties) != .NO_ERROR) return error.FailedToStartTrace;

        // Enable stack tracing events.
        var stack_id = [2]etw.CLASSIC_EVENT_ID{
            .{ .EventGuid = etw.PerfInfoGuid, .Type = 46, .Reserved = @splat(0) }, // SampledProfile
            .{ .EventGuid = etw.ThreadGuid, .Type = 36, .Reserved = @splat(0) }, // CSwitch
        };
        if (etw.TraceSetInformation(trace_handle, .TraceStackTracingInfo, &stack_id, @truncate(@sizeOf(@TypeOf(stack_id)))) != .NO_ERROR)
            return error.FailedToEnableStackTracing;
        errdefer _ = etw.CloseTrace(trace_handle);

        var logger_name = etw.KERNEL_LOGGER_NAME.*;
        var log = mem.zeroes(etw.EVENT_TRACE_LOGFILEA);
        log.LoggerName = &logger_name;
        log.Anonymous1.ProcessTraceMode = etw.PROCESS_TRACE_MODE_REAL_TIME |
            etw.PROCESS_TRACE_MODE_EVENT_RECORD |
            etw.PROCESS_TRACE_MODE_RAW_TIMESTAMP;
        log.Anonymous2.EventRecordCallback = &recordCallback;

        trace_handle2 = etw.OpenTraceA(&log);
        if (trace_handle2 == @intFromPtr(foundation.INVALID_HANDLE_VALUE)) {
            return error.FailedToOpenTrace;
        }

        thread = try Thread.spawn(.{}, runWorker, .{});
        thread.setName("Fimo Sampler") catch {};
        running = true;
    }

    pub fn stop() void {
        if (!running) return;
        _ = etw.CloseTrace(trace_handle2);
        _ = etw.CloseTrace(trace_handle);
        thread.join();

        thread = undefined;
        trace_handle2 = undefined;
        trace_handle = undefined;
        properties = undefined;
        running = false;
    }

    fn recordCallback(opt_record: ?*etw.EVENT_RECORD) callconv(.winapi) void {
        const record = opt_record orelse return;
        const header = &record.EventHeader;
        switch (header.ProviderId.Ints.a) {
            etw.ImageLoadGuid.Ints.a => {
                if (header.EventDescriptor.Version != 3) return;
                switch (header.EventDescriptor.Opcode) {
                    3, 10 => { // Image load
                        const ev: *const ImageLoad = @ptrCast(@alignCast(record.UserData));
                        if (ev.ProcessId != process_id) return;
                        const file_name_wtf16: [*:0]const u16 = @ptrCast(&ev.FileName);
                        var file_name_wtf8: [std.fs.max_path_bytes]u8 = undefined;
                        const len = std.unicode.wtf16LeToWtf8(&file_name_wtf8, mem.span(file_name_wtf16));
                        const file_path: Path = .{ .raw = file_name_wtf8[0..len] };
                        const event: tracing.events.LoadImage = .{
                            .time = time.Instant.initQPC(@bitCast(header.TimeStamp)).intoC(),
                            .image_base = ev.ImageBase,
                            .image_size = ev.ImageSize,
                            .image_path = file_path.intoC(),
                        };
                        for (tracing.subscribers) |subscriber| subscriber.load_image(event);
                    },
                    2 => { // Image unload
                        const ev: *const ImageLoad = @ptrCast(@alignCast(record.UserData));
                        if (ev.ProcessId != process_id) return;
                        const event: tracing.events.UnloadImage = .{
                            .time = time.Instant.initQPC(@bitCast(header.TimeStamp)).intoC(),
                            .image_base = ev.ImageBase,
                        };
                        for (tracing.subscribers) |subscriber| subscriber.unload_image(event);
                    },
                    else => {},
                }
            },
            etw.ThreadGuid.Ints.a => {
                switch (header.EventDescriptor.Opcode) {
                    36 => { // Context switch event
                        const cswitch: *const CSwitch = @ptrCast(@alignCast(record.UserData));
                        const event: tracing.events.ContextSwitch = .{
                            .time = time.Instant.initQPC(@bitCast(header.TimeStamp)).intoC(),
                            .old_thread_id = cswitch.OldThreadId,
                            .new_thread_id = cswitch.NewThreadId,
                            .cpu = record.BufferContext.Anonymous.Anonymous.ProcessorNumber,
                            .old_thread_wait_reason = @bitCast(cswitch.OldThreadWaitReason),
                            .old_thread_state = @bitCast(cswitch.OldThreadState),
                            .new_thread_priority = @bitCast(cswitch.NewThreadPriority),
                            .old_thread_priority = @bitCast(cswitch.OldThreadPriority),
                            .previous_cstate = @bitCast(cswitch.PreviousCState),
                        };
                        for (tracing.subscribers) |subscriber| subscriber.contextSwitch(event);
                    },
                    50 => { // Ready thread event
                        const ready: *const ReadyThread = @ptrCast(@alignCast(record.UserData));
                        const event: tracing.events.ThreadWakeup = .{
                            .time = time.Instant.initQPC(@bitCast(header.TimeStamp)).intoC(),
                            .cpu = record.BufferContext.Anonymous.Anonymous.ProcessorNumber,
                            .thread_id = ready.TThreadId,
                            .adjust_reason = ready.AdjustReason,
                            .adjust_increment = ready.AdjustIncrement,
                        };
                        for (tracing.subscribers) |subscriber| subscriber.threadWakeup(event);
                    },
                    1, 3 => { // Start (data collection) thread
                        const trace: *const Thread_TypeGroup1 = @ptrCast(@alignCast(record.UserData));
                        const tid = trace.TThreadId;
                        if (tid == 0) return;
                        const event: tracing.events.StartThread = .{
                            .time = time.Instant.initQPC(@bitCast(header.TimeStamp)).intoC(),
                            .thread_id = tid,
                            .process_id = trace.ProcessId,
                        };
                        for (tracing.subscribers) |subscriber| subscriber.start_thread(event);
                    },
                    2 => { // Stop thread
                        const trace: *const Thread_TypeGroup1 = @ptrCast(@alignCast(record.UserData));
                        const tid = trace.TThreadId;
                        if (tid == 0) return;
                        const event: tracing.events.StopThread = .{
                            .time = time.Instant.initQPC(@bitCast(header.TimeStamp)).intoC(),
                            .thread_id = tid,
                            .process_id = trace.ProcessId,
                        };
                        for (tracing.subscribers) |subscriber| subscriber.stop_thread(event);
                    },
                    else => {},
                }
            },
            StackWalkGuid.Ints.a => {
                switch (header.EventDescriptor.Opcode) {
                    32 => { // Stack tracing event
                        const stack_walk: *const StackWalk_Event = @ptrCast(@alignCast(record.UserData));
                        if (stack_walk.StackProcess != process_id) return;
                        const len = (record.UserDataLength - 16) / 8;
                        if (len == 0) return;
                        const event: tracing.events.CallStackSample = .{
                            .time = time.Instant.initQPC(stack_walk.EventTimeStamp).intoC(),
                            .thread_id = stack_walk.StackThread,
                            .call_stack = .fromSlice(@as([*]const usize, @ptrCast(&stack_walk.Stack))[0..len]),
                        };
                        for (tracing.subscribers) |subscriber| subscriber.callStackSample(event);
                    },
                    else => {},
                }
            },
            else => {},
        }
    }

    fn runWorker() void {
        _ = threading.SetThreadPriority(threading.GetCurrentThread(), threading.THREAD_PRIORITY_TIME_CRITICAL);
        _ = etw.ProcessTrace(@ptrCast(&trace_handle2), 1, null, null);
        _ = etw.ControlTraceA(0, etw.KERNEL_LOGGER_NAME, properties, etw.EVENT_TRACE_CONTROL_STOP);

        const buffer: [*]align(@alignOf(etw.EVENT_TRACE_PROPERTIES)) u8 = @ptrCast(properties);
        tracing.allocator.free(buffer[0 .. @sizeOf(etw.EVENT_TRACE_PROPERTIES) + etw.KERNEL_LOGGER_NAME.len + 1]);
    }
};
