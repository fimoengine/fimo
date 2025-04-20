const std = @import("std");
const atomic = std.atomic;
const Thread = std.Thread;
const Futex = Thread.Futex;
const Allocator = std.mem.Allocator;

const fimo_std = @import("fimo_std");
const time = fimo_std.time;
const Instant = time.Instant;
const Duration = time.Duration;
const fimo_tasks_meta = @import("fimo_tasks_meta");
const MetaParkingLot = fimo_tasks_meta.sync.ParkingLot;
const ParkToken = MetaParkingLot.ParkToken;
const UnparkToken = MetaParkingLot.UnparkToken;
const ParkResult = MetaParkingLot.ParkResult;
const ParkMultipleResult = MetaParkingLot.ParkMultipleResult;
const UnparkResult = MetaParkingLot.UnparkResult;
const FilterOp = MetaParkingLot.FilterOp;
const RequeueOp = MetaParkingLot.RequeueOp;

const Pool = @import("Pool.zig");
const Task = @import("Task.zig");
const Worker = @import("Worker.zig");

const Self = @This();

gpa: Allocator,
num_waiters: atomic.Value(usize) = .init(0),
hash_table: atomic.Value(?*HashTable) = .init(null),

const HashTable = struct {
    buckets: []Bucket,
    hash_bits: u32,
    prev: ?*HashTable,

    const load_factor = 3;

    const Bucket = struct {
        mutex: WordLock = .{},
        head: atomic.Value(?*Entry) = .init(null),
        tail: atomic.Value(?*Entry) = .init(null),
        fair_timeout: FairTimeout,

        fn rehashInto(self: *Bucket, table: *HashTable) void {
            var current = self.head.load(.monotonic);
            while (current) |curr| : ({
                current = curr.next.load(.monotonic);
                curr.next.store(null, .monotonic);
            }) {
                const slot = table.slotForPtr(curr.key.load(.monotonic));
                if (table.buckets[slot].tail.load(.monotonic)) |tail| {
                    tail.next.store(curr, .monotonic);
                } else {
                    table.buckets[slot].head.store(curr, .monotonic);
                }
                table.buckets[slot].tail.store(curr, .monotonic);
            }
        }
    };

    const Entry = struct {
        key: atomic.Value(*const anyopaque),
        next: atomic.Value(?*Entry) = .init(null),
        multi_queue: atomic.Value(*MultiQueueEntry),
    };

    const MultiQueueEntry = struct {
        mutex: WordLock = .{},
        event: Event,
        unpark_token: UnparkToken = .default,
        park_token: ParkToken = .default,
        consumer_key: ?*const anyopaque = null,

        fn tryLockIfNotConsumed(self: *MultiQueueEntry) bool {
            if (!self.mutex.tryLock()) return false;
            if (self.consumer_key != null) {
                self.mutex.unlock();
                return false;
            }
            return true;
        }

        fn checkConsumed(self: *MultiQueueEntry) bool {
            if (!self.mutex.tryLock()) return false;
            defer self.mutex.unlock();
            return self.consumer_key != null;
        }

        fn consumeAndUnlock(
            self: *MultiQueueEntry,
            unpark_token: UnparkToken,
            consumer_key: *const anyopaque,
        ) void {
            self.unpark_token = unpark_token;
            self.consumer_key = consumer_key;
            self.mutex.unlock();
        }
    };

    const FairTimeout = struct {
        timeout: Instant,
        seed: u32,

        fn shouldTimeout(self: *FairTimeout) bool {
            const now = Instant.now();
            if (now.order(self.timeout) == .gt) {
                const next = Duration.initNanos(self.nextU32());
                self.timeout = now.addSaturating(next);
                return true;
            }
            return false;
        }

        // Pseudorandom number generator from the "Xorshift RNGs" paper by George Marsaglia.
        fn nextU32(self: *FairTimeout) u32 {
            self.seed ^= self.seed << 13;
            self.seed ^= self.seed >> 17;
            self.seed ^= self.seed << 5;
            return self.seed;
        }
    };

    const Event = union(enum) {
        thread: struct {
            futex: atomic.Value(u32) = .init(1),
        },
        task: struct {
            pool: *Pool,
            futex: atomic.Value(u32) = .init(1),
        },

        fn park(self: *Event, timeout: ?Instant) void {
            switch (self.*) {
                .thread => |*thread| {
                    while (thread.futex.load(.acquire) != 0) {
                        if (timeout) |t| {
                            const now = Instant.now();
                            if (now.order(t) == .gt) return;

                            const duration = t.durationSince(now) catch unreachable;
                            const nanos = duration.nanos();

                            // Wait indefinitely on overflow.
                            if (nanos > std.math.maxInt(usize)) {
                                Futex.wait(&thread.futex, 1);
                                return;
                            }

                            // Wait for the specified duration.
                            Futex.timedWait(&thread.futex, 1, @truncate(nanos)) catch return;
                        } else {
                            Futex.wait(&thread.futex, 1);
                        }
                    }
                },
                .task => |*task| {
                    while (task.futex.load(.acquire) != 0) {
                        if (timeout) |t| {
                            const now = Instant.now();
                            if (now.order(t) == .gt) return;

                            // Wait for the specified duration.
                            Worker.timedWaitTask(&task.futex, 1, t) catch return;
                        } else {
                            Worker.waitTask(&task.futex, 1);
                        }
                    }
                },
            }
        }

        fn lock_unpark(self: *Event) UnparkHandle {
            switch (self.*) {
                .thread => |*thread| {
                    thread.futex.store(0, .release);
                    return .{ .thread = .{ .futex = &thread.futex } };
                },
                .task => |*task| {
                    task.futex.store(0, .release);
                    return .{ .task = .{ .pool = task.pool, .futex = &task.futex } };
                },
            }
        }
    };

    const UnparkHandle = union(enum) {
        thread: struct {
            futex: *const atomic.Value(u32),
        },
        task: struct {
            pool: *Pool,
            futex: *const atomic.Value(u32),
        },

        fn unpark(self: UnparkHandle) void {
            switch (self) {
                .thread => |thread| {
                    Futex.wake(thread.futex, 1);
                },
                .task => |task| {
                    task.pool.wakeByAddress(task.futex, 1);
                },
            }
        }
    };

    fn init(allocator: Allocator, num_waiters: usize, prev: ?*HashTable) *HashTable {
        const num_buckets = std.math.ceilPowerOfTwo(
            usize,
            num_waiters * load_factor,
        ) catch @panic("overflow");
        const hash_bits = @bitSizeOf(usize) - @clz(num_buckets) - 1;

        const buckets = allocator.alloc(Bucket, num_buckets) catch @panic("oom");
        for (buckets, 1..) |*bucket, i| {
            bucket.* = Bucket{ .fair_timeout = .{ .timeout = .now(), .seed = @intCast(i) } };
        }

        const table = allocator.create(HashTable) catch @panic("oom");
        table.* = .{
            .buckets = buckets,
            .hash_bits = hash_bits,
            .prev = prev,
        };
        return table;
    }

    fn deinit(self: *HashTable, allocator: Allocator) void {
        var current: ?*HashTable = self;
        while (current) |curr| {
            current = curr.prev;
            allocator.free(curr.buckets);
            allocator.destroy(curr);
        }
    }

    fn slotForPtr(self: *HashTable, key: *const anyopaque) usize {
        return std.hash.int(@intFromPtr(key)) >> @intCast(@bitSizeOf(usize) - self.hash_bits);
    }

    fn getBucket(self: *HashTable, key: *const anyopaque) *Bucket {
        const slot = self.slotForPtr(key);
        return &self.buckets[slot];
    }
};

// Adapted from https://trac.webkit.org/browser/webkit/trunk/Source/WTF/wtf/WordLock.cpp
const WordLock = struct {
    value: atomic.Value(usize) = .init(0),

    const head_mask: usize = 0b11;
    const locked_bit: usize = 0b01;
    const queue_locked_bit: usize = 0b10;

    const QueueEntry = struct {
        event: Event = .{},
        next: ?*QueueEntry = null,
        tail: ?*QueueEntry = null,
    };

    const Event = struct {
        should_park: bool = true,
        mutex: Thread.Mutex = .{},
        condition: Thread.Condition = .{},

        fn wait(self: *Event) void {
            self.mutex.lock();
            defer self.mutex.unlock();
            while (self.should_park) self.condition.wait(&self.mutex);
            std.debug.assert(!self.should_park);
        }

        fn signal(self: *Event) void {
            self.mutex.lock();
            defer self.mutex.unlock();
            self.should_park = false;
            self.condition.signal();
        }
    };

    fn tryLock(self: *WordLock) bool {
        var value = self.value.load(.monotonic);
        while (value & locked_bit == 0) {
            value = self.value.cmpxchgWeak(
                value,
                value | locked_bit,
                .acquire,
                .monotonic,
            ) orelse return true;
        }
        return false;
    }

    fn lock(self: *WordLock) void {
        if (self.value.cmpxchgWeak(0, locked_bit, .acquire, .monotonic)) |_| {
            self.lockSlow();
        }
    }

    fn unlock(self: *WordLock) void {
        if (self.value.cmpxchgWeak(locked_bit, 0, .release, .monotonic)) |_| {
            self.unlockSlow();
        }
    }

    fn lockSlow(self: *WordLock) void {
        @branchHint(.cold);

        const spin_limit = 40;
        var spin_count: usize = 0;
        var value = self.value.load(.monotonic);
        while (true) {
            // Try the fast path again.
            if (value & locked_bit == 0) {
                std.debug.assert(value & queue_locked_bit == 0);
                if (self.value.cmpxchgWeak(
                    value,
                    value | locked_bit,
                    .acquire,
                    .monotonic,
                )) |_| {} else return;
            }

            // If there is no queue we try spinning again.
            if (value & head_mask == 0 and spin_count < spin_limit) {
                spin_count += 1;
                Thread.yield() catch {};
                value = self.value.load(.monotonic);
                continue;
            }

            // Wait until the lock is not held and we acquire the queue lock.
            value = self.value.load(.monotonic);
            if (value & queue_locked_bit != 0 or
                value & locked_bit == 0 or
                self.value.cmpxchgWeak(value, value | queue_locked_bit, .acquire, .monotonic) != null)
            {
                Thread.yield() catch {};
                value = self.value.load(.monotonic);
                continue;
            }

            // Put `entry` in the queue.
            var entry = QueueEntry{};
            var queue: ?*QueueEntry = @ptrFromInt(value & ~head_mask);
            if (queue) |head| {
                head.tail.?.next = &entry;
                head.tail = &entry;

                // Release the queue lock.
                value = self.value.load(.monotonic);
                std.debug.assert(value & ~head_mask != 0);
                std.debug.assert(value & queue_locked_bit != 0);
                std.debug.assert(value & locked_bit != 0);
                self.value.store(value & ~queue_locked_bit, .release);
            } else {
                queue = &entry;
                entry.tail = &entry;

                // Release the queue lock and register us as the queue head.
                value = self.value.load(.monotonic);
                std.debug.assert(value & ~head_mask == 0);
                std.debug.assert(value & queue_locked_bit != 0);
                std.debug.assert(value & locked_bit != 0);
                value |= @intFromPtr(queue);
                value &= ~queue_locked_bit;
                self.value.store(value, .release);
            }

            // Wait until we are unparked.
            entry.event.wait();

            std.debug.assert(entry.next == null);
            std.debug.assert(entry.tail == null);

            value = self.value.load(.monotonic);
        }
    }

    fn unlockSlow(self: *WordLock) void {
        @branchHint(.cold);

        // Acquire the queue lock.
        while (true) {
            const value = self.value.load(.monotonic);
            std.debug.assert(value & locked_bit != 0);

            // If there are no waiters we try again using the fast path.
            if (value == locked_bit) {
                if (self.value.cmpxchgWeak(locked_bit, 0, .release, .monotonic)) |_| {
                    return;
                }
                Thread.yield() catch {};
                continue;
            }

            // Wait until the queue is unlocked.
            if (value & queue_locked_bit != 0) {
                Thread.yield() catch {};
                continue;
            }

            std.debug.assert(value & ~head_mask != 0);
            if (self.value.cmpxchgWeak(
                value,
                value | queue_locked_bit,
                .acquire,
                .monotonic,
            )) |_| {} else break;
        }

        var value = self.value.load(.monotonic);
        const head: *QueueEntry = @ptrFromInt(value & ~head_mask);
        const new_head = head.next;
        if (new_head) |new| new.tail = head.tail;

        // Install the new head and unlock the queue.
        value = self.value.load(.monotonic);
        var new_value = value;
        new_value &= ~locked_bit;
        new_value &= ~queue_locked_bit;
        new_value &= head_mask;
        new_value |= @intFromPtr(new_head);
        self.value.store(new_value, .release);

        // Unpark the waiter.
        head.next = null;
        head.tail = null;
        head.event.signal();
    }
};

pub fn init(gpa: std.mem.Allocator) Self {
    return Self{ .gpa = gpa };
}

pub fn deinit(self: *Self) void {
    if (self.num_waiters.load(.monotonic) != 0) @panic("parking lot is not empty");
    if (self.hash_table.load(.acquire)) |table| table.deinit(self.gpa);
}

fn getHashTable(self: *Self) *HashTable {
    const table = self.hash_table.load(.acquire);
    if (table) |t| return t;

    const allocator = self.gpa;
    const new_table = HashTable.init(allocator, HashTable.load_factor, null);
    if (self.hash_table.cmpxchgStrong(null, new_table, .acq_rel, .acquire)) |old| {
        new_table.deinit(allocator);
        return old.?;
    }
    return new_table;
}

fn growHashTable(self: *Self, num_waiters: usize) void {
    const old_table: *HashTable = blk: {
        while (true) {
            const table = self.getHashTable();
            if (table.buckets.len >= HashTable.load_factor * num_waiters) return;
            for (table.buckets) |*bucket| bucket.mutex.lock();
            if (self.hash_table.load(.monotonic) == table) break :blk table;
            for (table.buckets) |*bucket| bucket.mutex.unlock();
        }
    };
    defer for (old_table.buckets) |*bucket| bucket.mutex.unlock();

    const new_table = HashTable.init(self.gpa, num_waiters, old_table);
    for (old_table.buckets) |*bucket| {
        bucket.rehashInto(new_table);
    }
    self.hash_table.store(new_table, .release);
}

fn lockBucket(self: *Self, key: *const anyopaque) *HashTable.Bucket {
    while (true) {
        const table = self.getHashTable();
        const bucket = table.getBucket(key);
        bucket.mutex.lock();
        if (table == self.hash_table.load(.monotonic)) return bucket;
        bucket.mutex.unlock();
    }
}

fn lockBucketChecked(
    self: *Self,
    key: *atomic.Value(*const anyopaque),
) struct { *const anyopaque, *HashTable.Bucket } {
    while (true) {
        const table = self.getHashTable();
        const key_value = key.load(.monotonic);
        const bucket = table.getBucket(key_value);
        bucket.mutex.lock();
        if (table == self.hash_table.load(.monotonic) and key_value == key.load(.monotonic))
            return .{ key_value, bucket };
        bucket.mutex.unlock();
    }
}

fn lockBucketPair(
    self: *Self,
    key_1: *const anyopaque,
    key_2: *const anyopaque,
) struct { *HashTable.Bucket, *HashTable.Bucket } {
    while (true) {
        const table = self.getHashTable();

        const bucket_1 = if (@intFromPtr(key_1) <= @intFromPtr(key_2))
            table.getBucket(key_1)
        else
            table.getBucket(key_2);

        // Lock the first bucket.
        bucket_1.mutex.lock();

        // Check if the table has been rehashed.
        if (self.hash_table.load(.monotonic) != table) {
            bucket_1.mutex.unlock();
            continue;
        }

        // Lock the second bucket and return.
        if (key_1 == key_2) return .{ bucket_1, bucket_1 };
        if (@intFromPtr(key_1) <= @intFromPtr(key_2)) {
            const bucket_2 = table.getBucket(key_2);
            if (bucket_1 != bucket_2) bucket_2.mutex.lock();
            return .{ bucket_1, bucket_2 };
        } else {
            const bucket_2 = table.getBucket(key_1);
            if (bucket_1 != bucket_2) bucket_2.mutex.lock();
            return .{ bucket_2, bucket_1 };
        }
    }
}

fn unlockBucketPair(bucket_1: *HashTable.Bucket, bucket_2: *HashTable.Bucket) void {
    bucket_1.mutex.unlock();
    if (bucket_1 != bucket_2) bucket_2.mutex.unlock();
}

fn createMultiQueueEntry(token: ParkToken) HashTable.MultiQueueEntry {
    const is_task = Worker.currentTask() != null;
    const event = if (is_task)
        HashTable.Event{ .task = .{ .pool = Worker.currentPool().? } }
    else
        HashTable.Event{ .thread = .{} };
    return .{
        .event = event,
        .park_token = token,
    };
}

fn createBucketEntry(
    self: *Self,
    key: *const anyopaque,
    multi_queue: *HashTable.MultiQueueEntry,
) HashTable.Entry {
    const num_waiters = self.num_waiters.fetchAdd(1, .monotonic) + 1;
    self.growHashTable(num_waiters);
    return .{
        .key = .init(key),
        .multi_queue = .init(multi_queue),
    };
}

fn destroyBucketEntry(self: *Self) void {
    _ = self.num_waiters.fetchSub(1, .monotonic);
}

fn destroyBucketEntryMultiple(self: *Self, entries: []HashTable.Entry) void {
    _ = self.num_waiters.fetchSub(entries.len, .monotonic);
}

/// Parks the current task in the queue associated with the given key.
///
/// The `validation` function is called while the queue is locked and can abort the operation by
/// returning false. If `validation` returns true then the current task is appended to the queue
/// and the queue is unlocked.
///
/// The `before_sleep` function is called after the queue is unlocked but before the task is put to
/// sleep. The task will then sleep until it is unparked or the given timeout is reached. Since it
/// is called while the queue is unlocked, it can be used to perform additional operations, as long
/// as `park` or `parkMultiple` is not called recursively.
///
/// The `timed_out` function is also called while the queue is locked, but only if the timeout was
/// reached. It is passed the key of the queue it was in when it timed out, which may be different
/// from the original key if the task was requeued. It is also passed a bool which indicates whether
/// it was the last task in the queue.
pub fn park(
    self: *Self,
    key: *const anyopaque,
    validation_data: anytype,
    validation: anytype,
    before_sleep_data: anytype,
    before_sleep: anytype,
    timed_out_data: anytype,
    timed_out: anytype,
    token: ParkToken,
    timeout: ?Instant,
) ParkResult {
    // Create an entry for the current thread/task on the stack.
    var multi_queue = createMultiQueueEntry(token);
    var entry = self.createBucketEntry(key, &multi_queue);
    defer self.destroyBucketEntry();

    {
        // Lock the bucket for the given key.
        var bucket = self.lockBucket(key);
        defer bucket.mutex.unlock();

        // If the validation fails, simply return.
        if (!validation(validation_data)) {
            return ParkResult{ .type = .invalid };
        }

        // Append the entry to the queue and unlock the bucket.
        if (bucket.head.load(.monotonic)) |_| {
            bucket.tail.load(.monotonic).?.next.store(&entry, .monotonic);
        } else {
            bucket.head.store(&entry, .monotonic);
        }
        bucket.tail.store(&entry, .monotonic);
    }

    // Invoke the pre-sleep callback.
    before_sleep(before_sleep_data);
    multi_queue.event.park(timeout);

    // It is possible that the current task was not parked, as some other thread may have already
    // unparked it, or a timeout occurred. In that case we must check if the entry is still in the
    // queue before returning. We accomplish this by locking the mutex of the multi-queue entry.
    // Once locked, we can safely check if we were consumed, i.e. we were unparked and are no
    // longer in the queue. We don't need to unlock the mutex, as the multi-queue entry is destroyed
    // on function exit.
    multi_queue.mutex.lock();
    if (multi_queue.consumer_key != null) return ParkResult{
        .type = .unparked,
        .token = multi_queue.unpark_token,
    };

    // At this point we know that we were not consumed, and instead we timed out. So we need to
    // remove the entry from the queue. We do this by locking the bucket again. It is possible
    // that the entry was requeued to a different bucket, so we use the checked variant.
    const new_key: *const anyopaque, const bucket: *HashTable.Bucket =
        self.lockBucketChecked(&entry.key);
    defer bucket.mutex.unlock();

    // We timed out, so we remove the entry from the queue.
    var link: *atomic.Value(?*HashTable.Entry) = &bucket.head;
    var current: ?*HashTable.Entry = bucket.head.load(.monotonic);
    var previous: ?*HashTable.Entry = null;
    var was_last_entry = true;
    while (current) |curr| {
        if (curr != &entry) {
            if (curr.key.load(.monotonic) == new_key) was_last_entry = false;
            link = &curr.next;
            previous = curr;
            current = curr.next.load(.monotonic);
            continue;
        }

        const next = curr.next.load(.monotonic);
        link.store(next, .monotonic);
        if (bucket.tail.load(.monotonic) == curr) {
            bucket.tail.store(previous, .monotonic);
        } else {
            // Scan the rest of the queue to see if there are any other entries with the same key.
            var scan = next;
            while (scan) |scan_entry| : (scan = scan_entry.next.load(.monotonic)) {
                if (scan_entry.key.load(.monotonic) == new_key) {
                    was_last_entry = false;
                    break;
                }
            }
        }

        // Notify that we timed out.
        timed_out(timed_out_data, new_key, was_last_entry);
        break;
    }

    return ParkResult{ .type = .timed_out };
}

/// Parks the current task in the queues associated with the given keys.
///
/// The `validation` function is called while the queue managing the key is locked and can abort
/// the operation by returning false. If `validation` returns true then the current task is
/// appended to the queue and the queue is unlocked.
///
/// The `before_sleep` function is called after the queues are unlocked but before the task is put
/// to sleep. The task will then sleep until it is unparked or the given timeout is reached. Since
/// it is called while the queue is unlocked, it can be used to perform additional operations, as
/// long as `park` or `parkMultiple` is not called recursively.
pub fn parkMultiple(
    self: *Self,
    keys: []const *const anyopaque,
    validation_data: anytype,
    validation: anytype,
    before_sleep_data: anytype,
    before_sleep: anytype,
    token: ParkToken,
    timeout: ?Instant,
) ParkMultipleResult {
    if (keys.len == 0 or keys.len > MetaParkingLot.max_park_multiple_key_count)
        return ParkMultipleResult{ .type = .keys_invalid };

    var multi_queue = createMultiQueueEntry(token);
    var entries_array: [MetaParkingLot.max_park_multiple_key_count]HashTable.Entry = undefined;
    const entries = entries_array[0..keys.len];
    for (keys, entries) |k, *e| e.* = self.createBucketEntry(k, &multi_queue);
    defer self.destroyBucketEntryMultiple(entries);

    // Insert the entry into each queue.
    var enqueued_entries: usize = 0;
    var enqueue_status: enum { invalid, consumed, ok } = .ok;
    for (keys, entries, 0..) |key, *entry, i| {
        // Lock the bucket for the given key.
        var bucket = self.lockBucket(key);
        defer bucket.mutex.unlock();

        // The check that the entry has not yet been consumed.
        if (multi_queue.checkConsumed()) {
            enqueue_status = .consumed;
            break;
        }

        // Validate the key using the provided callback.
        const idx: u32 = @truncate(i);
        if (!validation(validation_data, idx)) {
            enqueue_status = .invalid;
            break;
        }

        // Append the entry to the queue and unlock the bucket.
        if (bucket.head.load(.monotonic)) |_| {
            bucket.tail.load(.monotonic).?.next.store(entry, .monotonic);
        } else {
            bucket.head.store(entry, .monotonic);
        }
        bucket.tail.store(entry, .monotonic);
        enqueued_entries += 1;
    }

    // If the entries could not be enqueued, or if the validation failed, we must dequeue all
    // entries and then return.
    if (enqueue_status != .ok) {
        for (keys[0..enqueued_entries], entries[0..enqueued_entries]) |key, *entry| {
            var bucket = self.lockBucket(key);
            defer bucket.mutex.unlock();

            var link: *atomic.Value(?*HashTable.Entry) = &bucket.head;
            var current: ?*HashTable.Entry = bucket.head.load(.monotonic);
            var previous: ?*HashTable.Entry = null;
            while (current) |curr| {
                if (curr != entry) {
                    link = &curr.next;
                    previous = curr;
                    current = curr.next.load(.monotonic);
                    continue;
                }
                const next = curr.next.load(.monotonic);
                link.store(next, .monotonic);
                if (bucket.tail.load(.monotonic) == curr) bucket.tail.store(previous, .monotonic);
                break;
            }
        }

        multi_queue.mutex.lock();
        if (multi_queue.consumer_key != null) enqueue_status = .consumed;
        switch (enqueue_status) {
            .invalid => return ParkMultipleResult{
                .type = .invalid,
                .index = @truncate(enqueued_entries),
            },
            .consumed => {
                // Now that we are not enqueued we can read the unpark token and the key that woke
                // us up.
                const index = std.mem.indexOfScalar(
                    *const anyopaque,
                    keys,
                    multi_queue.consumer_key.?,
                ).?;
                return ParkMultipleResult{
                    .type = .unparked,
                    .index = @truncate(index),
                    .token = multi_queue.unpark_token,
                };
            },
            .ok => unreachable,
        }
    }

    // Invoke the pre-sleep callback.
    before_sleep(before_sleep_data);
    multi_queue.event.park(timeout);

    // Dequeue the entry from all queues.
    for (keys, entries) |key, *entry| {
        var bucket = self.lockBucket(key);
        defer bucket.mutex.unlock();

        var link: *atomic.Value(?*HashTable.Entry) = &bucket.head;
        var current: ?*HashTable.Entry = bucket.head.load(.monotonic);
        var previous: ?*HashTable.Entry = null;
        while (current) |curr| {
            if (curr != entry) {
                link = &curr.next;
                previous = curr;
                current = curr.next.load(.monotonic);
                continue;
            }
            const next = curr.next.load(.monotonic);
            link.store(next, .monotonic);
            if (bucket.tail.load(.monotonic) == curr) bucket.tail.store(previous, .monotonic);
            break;
        }
    }

    // Same reasoning applies as in the `park` function. This lock ensures that the multi-queue
    // entry is not shared with any other thread before returning fro the function. We don't need
    // to unlock the mutex, as the multi-queue entry is destroyed on function exit.
    multi_queue.mutex.lock();
    return if (multi_queue.consumer_key) |consumer|
        ParkMultipleResult{
            .type = .unparked,
            .index = @truncate(std.mem.indexOfScalar(
                *const anyopaque,
                keys,
                consumer,
            ).?),
            .token = multi_queue.unpark_token,
        }
    else
        ParkMultipleResult{ .type = .timed_out };
}

/// Unparks one task from the queue associated with the given key.
///
/// The `callback` function is called while the queue is locked and before the target task is woken
/// up. The `result` argument to the function indicates whether a task was found in the queue and
/// whether this was the last task in the queue. This value is also returned by `unparkOne`.
pub fn unparkOne(
    self: *Self,
    key: *const anyopaque,
    callback_data: anytype,
    callback: anytype,
) UnparkResult {
    const bucket = self.lockBucket(key);
    var link: *atomic.Value(?*HashTable.Entry) = &bucket.head;
    var current: ?*HashTable.Entry = bucket.head.load(.monotonic);
    var previous: ?*HashTable.Entry = null;
    var result: UnparkResult = .{};
    while (current) |curr| {
        if (curr.key.load(.monotonic) != key) {
            link = &curr.next;
            previous = curr;
            current = curr.next.load(.monotonic);
            continue;
        }

        // Lock the entry and check if it was already consumed.
        // If we can't lock the entry, it means that the task already woke up.
        const curr_multi_queue = curr.multi_queue.load(.monotonic);
        if (!curr_multi_queue.tryLockIfNotConsumed()) {
            link = &curr.next;
            previous = curr;
            current = curr.next.load(.monotonic);
            continue;
        }

        // Remove the entry from the queue.
        const next = curr.next.load(.monotonic);
        link.store(next, .monotonic);
        if (bucket.tail.load(.monotonic) == curr) {
            bucket.tail.store(previous, .monotonic);
        } else {
            // Scan the rest of the queue to see if there are any other entries with the same key.
            var scan = next;
            while (scan) |scan_entry| : (scan = scan_entry.next.load(.monotonic)) {
                if (scan_entry.key.load(.monotonic) == key) {
                    const scan_entry_multi_queue = scan_entry.multi_queue.load(.monotonic);
                    if (scan_entry_multi_queue.checkConsumed()) continue;
                    result.has_more_tasks = true;
                    break;
                }
            }
        }

        // Fetch the unpark token from the callback.
        result.unparked_tasks = 1;
        result.be_fair = bucket.fair_timeout.shouldTimeout();
        const unpark_token = callback(callback_data, result);

        const handle = curr_multi_queue.event.lock_unpark();
        bucket.mutex.unlock();
        handle.unpark();
        curr_multi_queue.consumeAndUnlock(unpark_token, key);
        return result;
    }

    // No matching entry found.
    _ = callback(callback_data, result);
    bucket.mutex.unlock();
    return result;
}

/// Unparks all tasks in the queue associated with the given key.
///
/// The given unpark token is passed to all unparked tasks. This function returns the number of
/// tasks that were unparked.
pub fn unparkAll(self: *Self, key: *const anyopaque, token: UnparkToken) usize {
    const bucket = self.lockBucket(key);

    const HandleTokenPair = struct {
        handle: HashTable.UnparkHandle,
        entry: *HashTable.MultiQueueEntry,
    };

    // Remove all entries from the bucket.
    var link: *atomic.Value(?*HashTable.Entry) = &bucket.head;
    var current: ?*HashTable.Entry = bucket.head.load(.monotonic);
    var previous: ?*HashTable.Entry = null;
    var handles_allocator = std.heap.stackFallback(@sizeOf(HandleTokenPair) * 8, self.gpa);
    const allocator = handles_allocator.get();
    var handles = std.ArrayListUnmanaged(HandleTokenPair){};
    defer handles.deinit(allocator);
    while (current) |curr| {
        if (curr.key.load(.monotonic) != key) {
            link = &curr.next;
            previous = curr;
            current = curr.next.load(.monotonic);
            continue;
        }

        // Lock the entry and check if it was already consumed.
        // If we can't lock the entry, it means that the task already woke up.
        const curr_multi_queue = curr.multi_queue.load(.monotonic);
        if (!curr_multi_queue.tryLockIfNotConsumed()) {
            link = &curr.next;
            previous = curr;
            current = curr.next.load(.monotonic);
            continue;
        }

        // Remove the entry from the bucket.
        const next = curr.next.load(.monotonic);
        link.store(next, .monotonic);
        if (bucket.tail.load(.monotonic) == curr) bucket.tail.store(previous, .monotonic);

        // Unpark all entries after the bucket is unlocked.
        const handle = curr_multi_queue.event.lock_unpark();
        handles.append(allocator, .{
            .handle = handle,
            .entry = curr_multi_queue,
        }) catch @panic("oom");
        current = next;
    }

    // Unlock the bucket and unpark all waiters.
    bucket.mutex.unlock();
    for (handles.items) |pair| {
        pair.handle.unpark();
        pair.entry.consumeAndUnlock(token, key);
    }
    return handles.items.len;
}

/// Unparks a number of tasks from the front of the queue associated with `key` depending on the
/// results of a filter function which inspects the park token associated with each task.
///
/// The `filter` function is called for each task in the queue or until `.stop` is returned. This
/// function is passed the park token associated with a particular task, which is unparked if
/// `.unpark` is returned.
///
/// The `callback` function is also called while both queues are locked. It is passed a result
/// indicating the number of tasks that were unparked and whether there are still parked tasks in
/// the queue. This result value is also returned by `unparkFilter`.
///
/// The `callback` function should return an unpark token value which will be passed to all tasks
/// that are unparked. If no task is unparked then the returned value is ignored.
pub fn unparkFilter(
    self: *Self,
    key: *const anyopaque,
    filter_data: anytype,
    filter: anytype,
    callback_data: anytype,
    callback: anytype,
) UnparkResult {
    const bucket = self.lockBucket(key);

    const HandleTokenPair = struct {
        handle: HashTable.UnparkHandle,
        entry: *HashTable.MultiQueueEntry,
    };

    // Go through the queue looking for entries with a matching key.
    var link: *atomic.Value(?*HashTable.Entry) = &bucket.head;
    var current: ?*HashTable.Entry = bucket.head.load(.monotonic);
    var previous: ?*HashTable.Entry = null;
    var handles_allocator = std.heap.stackFallback(@sizeOf(HandleTokenPair) * 8, self.gpa);
    const allocator = handles_allocator.get();
    var handles = std.ArrayListUnmanaged(HandleTokenPair){};
    defer handles.deinit(allocator);
    var result: UnparkResult = .{};
    while (current) |curr| {
        if (curr.key.load(.monotonic) != key) {
            link = &curr.next;
            previous = curr;
            current = curr.next.load(.monotonic);
            continue;
        }

        // Lock the entry and check if it was already consumed.
        // If we can't lock the entry, it means that the task already woke up.
        const curr_multi_queue = curr.multi_queue.load(.monotonic);
        if (!curr_multi_queue.tryLockIfNotConsumed()) {
            link = &curr.next;
            previous = curr;
            current = curr.next.load(.monotonic);
            continue;
        }

        // Call the filter function with the entries park token.
        const op: FilterOp = filter(filter_data, curr_multi_queue.park_token);
        switch (op) {
            .unpark => {
                // Remove the entry from the queue.
                link.store(curr.next.load(.monotonic), .monotonic);
                if (bucket.tail.load(.monotonic) == curr) bucket.tail.store(previous, .monotonic);

                // Add the current entry to the list.
                handles.append(allocator, .{
                    .handle = curr_multi_queue.event.lock_unpark(),
                    .entry = curr_multi_queue,
                }) catch @panic("oom");

                current = curr.next.load(.monotonic);
            },
            .skip => {
                curr_multi_queue.mutex.unlock();
                result.has_more_tasks = true;
                link = &curr.next;
                previous = curr;
                current = curr.next.load(.monotonic);
            },
            .stop => {
                curr_multi_queue.mutex.unlock();
                result.has_more_tasks = true;
                break;
            },
        }
    }

    // Invoke the callback to fetch the unpark token.
    result.unparked_tasks = handles.items.len;
    if (handles.items.len != 0) result.be_fair = bucket.fair_timeout.shouldTimeout();
    const token = callback(callback_data, result);

    // Unlock the bucket and unpark all waiters.
    bucket.mutex.unlock();
    for (handles.items) |pair| {
        pair.handle.unpark();
        pair.entry.consumeAndUnlock(token, key);
    }
    return result;
}

/// Removes tasks from the queue associated with `key_from`, and requeues them onto the queue
/// associated with `key_to`.
///
/// The `validate` function is called while both queues are locked. Its return value will determine
/// the maximum number or tasks to unpark, and the maximum number of tasks to requeue onto the
/// target queue.
///
/// The `callback` function is also called while both queues are locked. It is passed the result of
/// the `validate` function, and a `result`, indicating the number of unparked and requeued tasks.
/// The result will also be returned as the result of the `unparkRequeue` function. The resulting
/// unpark token will be passed to the unparked task, or will be ignored if no task was unparked.
pub fn unparkRequeue(
    self: *Self,
    key_from: *const anyopaque,
    key_to: *const anyopaque,
    validate_data: anytype,
    validate: anytype,
    callback_data: anytype,
    callback: anytype,
) UnparkResult {
    const bucket_from: *HashTable.Bucket, const bucket_to: *HashTable.Bucket =
        self.lockBucketPair(key_from, key_to);

    const op: RequeueOp = validate(validate_data);
    if (op.num_tasks_to_unpark == 0 and op.num_tasks_to_requeue == 0) {
        unlockBucketPair(bucket_from, bucket_to);
        return UnparkResult{};
    }

    const HandleTokenPair = struct {
        handle: HashTable.UnparkHandle,
        entry: *HashTable.MultiQueueEntry,
    };
    var handles_allocator = std.heap.stackFallback(@sizeOf(HandleTokenPair) * 8, self.gpa);
    const allocator = handles_allocator.get();
    var handles = std.ArrayListUnmanaged(HandleTokenPair){};
    defer handles.deinit(allocator);

    // Go through the queue looking for entries with a matching key.
    var link: *atomic.Value(?*HashTable.Entry) = &bucket_from.head;
    var current: ?*HashTable.Entry = bucket_from.head.load(.monotonic);
    var previous: ?*HashTable.Entry = null;
    var result = UnparkResult{};
    while (current) |curr| {
        if (result.unparked_tasks == op.num_tasks_to_unpark and
            result.requeued_tasks == op.num_tasks_to_requeue)
        {
            break;
        }

        if (curr.key.load(.monotonic) != key_from) {
            link = &curr.next;
            previous = curr;
            current = curr.next.load(.monotonic);
            continue;
        }

        // Lock the entry and check if it was already consumed.
        // If we can't lock the entry, it means that the task already woke up.
        const curr_multi_queue = curr.multi_queue.load(.monotonic);
        if (!curr_multi_queue.tryLockIfNotConsumed()) {
            link = &curr.next;
            previous = curr;
            current = curr.next.load(.monotonic);
            continue;
        }

        // Remove the entry from the queue.
        link.store(curr.next.load(.monotonic), .monotonic);
        if (bucket_from.tail.load(.monotonic) == curr) bucket_from.tail.store(previous, .monotonic);

        if (result.unparked_tasks < op.num_tasks_to_unpark) {
            // Add the current entry to the list.
            handles.append(allocator, .{
                .handle = curr_multi_queue.event.lock_unpark(),
                .entry = curr_multi_queue,
            }) catch @panic("oom");

            result.unparked_tasks += 1;
            current = curr.next.load(.monotonic);
        } else if (result.requeued_tasks < op.num_tasks_to_requeue) {
            curr_multi_queue.mutex.unlock();

            // Requeue the entry onto the destination bucket.
            const next = curr.next.load(.monotonic);
            if (bucket_to.head.load(.monotonic)) |_| {
                bucket_to.tail.load(.monotonic).?.next.store(curr, .monotonic);
            } else {
                bucket_to.head.store(curr, .monotonic);
            }
            bucket_to.tail.store(curr, .monotonic);
            curr.next.store(null, .monotonic);
            curr.key.store(key_to, .monotonic);

            result.requeued_tasks += 1;
            current = next;
        }
    }

    // Scan the rest of the queue for any remaining entries.
    if (result.unparked_tasks != 0) {
        while (current) |curr| : (current = curr.next.load(.monotonic)) {
            const curr_multi_queue = curr.multi_queue.load(.monotonic);
            if (curr_multi_queue.checkConsumed()) continue;
            if (curr.key.load(.monotonic) == key_from) {
                result.has_more_tasks = true;
                break;
            }
        }
    }

    // Invoke the callback to fetch the unpark token.
    if (result.unparked_tasks != 0) result.be_fair = bucket_from.fair_timeout.shouldTimeout();
    const token: UnparkToken = callback(callback_data, op, result);

    // Unlock the buckets and unpark all waiters.
    unlockBucketPair(bucket_from, bucket_to);
    for (handles.items) |pair| {
        pair.handle.unpark();
        pair.entry.consumeAndUnlock(token, key_from);
    }
    return result;
}

// Taken from the rust parking lot implementation.
const SingleLatchTest = struct {
    semaphore: atomic.Value(isize) = .init(0),
    num_awake: atomic.Value(usize) = .init(0),
    num_threads: usize,
    pl: *Self,
    threads: []Thread = &.{},

    fn run(self: *@This()) void {
        self.down();
        _ = self.num_awake.fetchAdd(1, .seq_cst);
    }

    fn unparkOne(self: *@This(), unpark_index: usize) void {
        _ = unpark_index;
        const num_awake_before_up = self.num_awake.load(.seq_cst);
        self.up();
        while (self.num_awake.load(.seq_cst) != num_awake_before_up + 1) {
            Thread.yield() catch unreachable;
        }
    }

    fn finish(self: *@This(), num_single_unparks: usize) void {
        var num_threads_left = self.num_threads - num_single_unparks;
        while (num_threads_left > 0) {
            const num_awake_before_unpark = self.num_awake.load(.seq_cst);
            const num_unparked = self.pl.unparkAll(self.semaphoreAddr(), .default);
            std.debug.assert(num_unparked <= num_threads_left);
            while (self.num_awake.load(.seq_cst) != num_awake_before_unpark + num_unparked) {
                Thread.yield() catch unreachable;
            }
            num_threads_left -= num_unparked;
        }
    }

    fn down(self: *@This()) void {
        const old_semaphore_value = self.semaphore.fetchSub(1, .seq_cst);
        if (old_semaphore_value > 0) return;

        const Validate = struct {
            fn f(this: @This()) bool {
                _ = this;
                return true;
            }
        };
        const BeforeSleep = struct {
            fn f(this: @This()) void {
                _ = this;
            }
        };
        const TimedOut = struct {
            fn f(this: @This(), key: *const anyopaque, was_last: bool) void {
                _ = key;
                _ = was_last;
                _ = this;
            }
        };
        _ = self.pl.park(
            self.semaphoreAddr(),
            Validate{},
            Validate.f,
            BeforeSleep{},
            BeforeSleep.f,
            TimedOut{},
            TimedOut.f,
            .default,
            null,
        );
    }

    fn up(self: *@This()) void {
        const old_semaphore_value = self.semaphore.fetchAdd(1, .seq_cst);
        if (old_semaphore_value < 0) {
            while (true) {
                const Callback = struct {
                    fn f(this: @This(), result: UnparkResult) UnparkToken {
                        _ = this;
                        _ = result;
                        return .default;
                    }
                };
                const unparked = self.pl.unparkOne(
                    self.semaphoreAddr(),
                    Callback{},
                    Callback.f,
                ).unparked_tasks;
                switch (unparked) {
                    1 => break,
                    0 => {},
                    else => |i| std.debug.panic("should not wake up {} threads", .{i}),
                }
            }
        }
    }

    fn semaphoreAddr(self: *@This()) *const anyopaque {
        return &self.semaphore;
    }
};

fn testRun(
    pl: *Self,
    allocator: Allocator,
    num_latches: usize,
    delay: Duration,
    num_threads: usize,
    num_single_unparks: usize,
) !void {
    const tests = allocator.alloc(SingleLatchTest, num_latches) catch @panic("oom");
    defer allocator.free(tests);

    for (tests) |*t| {
        t.* = .{
            .num_threads = num_threads,
            .pl = pl,
            .threads = allocator.alloc(Thread, num_threads) catch @panic("oom"),
        };
        for (t.threads) |*th| {
            th.* = Thread.spawn(.{}, SingleLatchTest.run, .{t}) catch @panic("spawn");
        }
    }

    for (0..num_single_unparks) |unpark_index| {
        Thread.sleep(@intCast(delay.nanos()));
        for (tests) |*t| t.unparkOne(unpark_index);
    }

    for (tests) |*t| {
        t.finish(num_single_unparks);
        for (t.threads) |*th| th.join();
        allocator.free(t.threads);
    }
}

fn testWrapper(
    repeats: usize,
    num_latches: usize,
    delay_micros: time.Micros,
    num_threads: usize,
    num_single_unparks: usize,
) !void {
    var pl = Self.init(std.testing.allocator);
    defer pl.deinit();

    const delay = Duration.initMicros(delay_micros);
    for (0..repeats) |_| {
        try testRun(
            &pl,
            std.testing.allocator,
            num_latches,
            delay,
            num_threads,
            num_single_unparks,
        );
    }
}

test "unpark all one fast" {
    try testWrapper(1000, 1, 0, 1, 0);
}

test "unpark all hundred fast" {
    try testWrapper(100, 1, 0, 100, 0);
}

test "unpark one one fast" {
    try testWrapper(1000, 1, 0, 1, 1);
}

test "unpark one hundred fast" {
    try testWrapper(20, 1, 0, 100, 100);
}

test "unpark one fifty then fifty all fast" {
    try testWrapper(50, 1, 0, 100, 50);
}

test "unpark all one" {
    try testWrapper(100, 1, 10000, 1, 0);
}

test "unpark all hundred" {
    try testWrapper(100, 1, 10000, 100, 0);
}

test "unpark one one" {
    try testWrapper(10, 1, 10000, 1, 1);
}

test "unpark one fifty" {
    try testWrapper(1, 1, 10000, 50, 50);
}

test "unpark one fifty then fifty all" {
    try testWrapper(2, 1, 10000, 100, 50);
}

test "hundred unpark all one fast" {
    try testWrapper(100, 100, 0, 1, 0);
}

test "hundred unpark all one" {
    try testWrapper(1, 100, 10000, 1, 0);
}
