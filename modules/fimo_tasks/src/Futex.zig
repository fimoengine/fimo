const std = @import("std");
const atomic = std.atomic;
const Thread = std.Thread;
const Allocator = std.mem.Allocator;

const fimo_std = @import("fimo_std");
const time = fimo_std.time;
const Instant = time.Instant;
const Duration = time.Duration;
const fimo_tasks_meta = @import("fimo_tasks_meta");
pub const max_waitv_key_count = fimo_tasks_meta.sync.Futex.max_waitv_key_count;
pub const KeyExpect = fimo_tasks_meta.sync.Futex.KeyExpect;
pub const Filter = fimo_tasks_meta.sync.Futex.Filter;
pub const RequeueResult = fimo_tasks_meta.sync.Futex.RequeueResult;

const Executor = @import("Executor.zig");
const Worker = Executor.Worker;

const Self = @This();

gpa: Allocator,
num_waiters: atomic.Value(usize) = .init(0),
hash_table: atomic.Value(?*HashTable) = .init(null),

const Waiter = struct {
    lock: Thread.Mutex = .{},
    futex: atomic.Value(u32) = .init(1),
    signaled_key: ?*const anyopaque = null,
    entry_count: usize,
    executor: ?*Executor,

    const futex_empty: u32 = 0;
    const futex_waiting: u32 = 1;
    const futex_signaling: u32 = 2;

    fn init(entry_count: usize) Waiter {
        return .{ .entry_count = entry_count, .executor = Worker.currentExecutorIfInTask() };
    }

    fn wait(self: *Waiter, timeout: ?Instant) bool {
        var value = self.futex.load(.acquire);
        while (value != futex_empty) {
            if (self.executor == null) {
                if (timeout) |t| {
                    const now = Instant.now();
                    if (now.order(t) == .gt) return false;

                    const duration = t.durationSince(now) catch unreachable;
                    const nanos = duration.nanos();

                    // Wait indefinitely on overflow.
                    if (nanos > std.math.maxInt(usize)) {
                        Thread.Futex.wait(&self.futex, value);
                        value = self.futex.load(.acquire);
                        continue;
                    }

                    // Wait for the specified duration.
                    Thread.Futex.timedWait(&self.futex, value, @truncate(nanos)) catch return false;
                } else {
                    Thread.Futex.wait(&self.futex, value);
                }
            } else {
                if (timeout) |t| {
                    const now = Instant.now();
                    if (now.order(t) == .gt) return false;

                    // Wait for the specified duration.
                    Worker.timedWait(&self.futex, value, t) catch return false;
                } else {
                    Worker.wait(&self.futex, value);
                }
            }
            value = self.futex.load(.acquire);
        }
        return true;
    }

    fn prepareWake(self: *Waiter, key: *const anyopaque) bool {
        // Before locking the waiter, try setting the state of the futex to `signaled`.
        // This won`t wake the waiter, but ensures that no other thread tries to wake it.
        if (self.futex.cmpxchgStrong(
            futex_waiting,
            futex_signaling,
            .monotonic,
            .monotonic,
        )) |_| return false;

        // Now that we own the waiter we can lock it.
        self.lock.lock();
        self.signaled_key = key;
        return true;
    }

    fn wake(self: *Waiter, futex: *Self) void {
        self.futex.store(futex_empty, .release);
        if (self.executor) |exe| {
            exe.wakeByAddress(futex, &self.futex);
        } else {
            Thread.Futex.wake(&self.futex, 1);
        }
        self.lock.unlock();
    }

    fn checkWaiting(self: *Waiter) bool {
        return self.futex.load(.monotonic) == futex_waiting;
    }

    fn ensureUniqueReference(self: *Waiter) void {
        self.lock.lock();
    }
};

const Entry = struct {
    key: atomic.Value(*const anyopaque),
    key_size: atomic.Value(usize),
    token: usize,
    next: ?*Entry = null,
    waiter: *Waiter,

    fn init(
        futex: *Self,
        key: *const anyopaque,
        key_size: usize,
        token: usize,
        waiter: *Waiter,
    ) Entry {
        const num_waiters = futex.num_waiters.fetchAdd(1, .monotonic) + 1;
        futex.growHashTable(num_waiters);
        return .{
            .key = .init(key),
            .key_size = .init(key_size),
            .token = token,
            .waiter = waiter,
        };
    }

    fn unregisterOne(futex: *Self) void {
        _ = futex.num_waiters.fetchSub(1, .monotonic);
    }

    fn unregisterMany(futex: *Self, count: usize) void {
        _ = futex.num_waiters.fetchSub(count, .monotonic);
    }

    fn keyEquals(self: *Entry, expect: u64) bool {
        const key = self.key.load(.monotonic);
        const key_size = self.key_size.load(.monotonic);
        return checkKeyEquality(key, key_size, expect);
    }

    fn enqueue(self: *Entry, futex: *Self, expect: u64) error{Invalid}!void {
        // Lock the bucket for the key of the entry.
        const key = self.key.load(.monotonic);
        var bucket = Bucket.lockForKey(futex, key);
        defer bucket.unlock();

        // Check that the waiter has not been woken up by another key.
        if (self.waiter.entry_count != 1 and !self.waiter.checkWaiting()) return error.Invalid;

        // Check that the key still equals the expected value.
        // If the value changed we must interrupt the enqueue.
        if (!self.keyEquals(expect)) return error.Invalid;

        // Append the entry to the queue and unlock the bucket.
        if (bucket.head) |_| {
            bucket.tail.?.next = self;
        } else {
            bucket.head = self;
        }
        bucket.tail = self;
    }

    fn dequeueCheckTimeout(self: *Entry, futex: *Self) bool {
        // Lock the bucket of the entry, the key might change in the meantime.
        const bucket = Bucket.lockForKeyChecked(futex, &self.key);
        defer bucket.unlock();

        // If there we are the only entry for the waiter and the waiter has been signaled
        // we know that the entry must have been dequeued already.
        if (self.waiter.entry_count == 1 and !self.waiter.checkWaiting()) return false;

        // We might be enqueued out, so we remove the entry from the queue.
        var link: *?*Entry = &bucket.head;
        var current: ?*Entry = bucket.head;
        var previous: ?*Entry = null;
        while (current) |curr| {
            if (curr != self) {
                link = &curr.next;
                current = curr.next;
                previous = curr;
                continue;
            }

            link.* = curr.next;
            current = curr.next;
            if (bucket.tail == curr) bucket.tail = previous;
            return self.waiter.checkWaiting();
        }

        // If we were not contained in the queue, we must have been dequeued and therefore
        // did not timeout.
        return false;
    }
};

const Bucket = struct {
    lock: Thread.Mutex = .{},
    head: ?*Entry = null,
    tail: ?*Entry = null,

    /// Returns and locks the bucket that contains all entries for the given key.
    fn lockForKey(futex: *Self, key: *const anyopaque) *Bucket {
        while (true) {
            // Fetch the active table and lock the bucket.
            // The active table might be different after the locking is complete.
            const table = futex.getHashTable();
            const bucket = table.getBucket(key);
            bucket.lock.lock();

            // If the table is not active anymore, we retry.
            if (table == futex.hash_table.load(.monotonic)) return bucket;
            bucket.lock.unlock();
        }
    }

    /// Returns and locks the bucket that contains all entries for the given key.
    fn lockForKeyChecked(
        futex: *Self,
        key: *atomic.Value(*const anyopaque),
    ) *Bucket {
        while (true) {
            // Fetch the active table and lock the bucket.
            // The active table or the key might be different after the locking is complete.
            const table = futex.getHashTable();
            const key_value = key.load(.monotonic);
            const bucket = table.getBucket(key_value);
            bucket.lock.lock();

            // If either the active table or the key changed we unlock and retry.
            if (table == futex.hash_table.load(.monotonic) and key_value == key.load(.monotonic))
                return bucket;
            bucket.lock.unlock();
        }
    }

    /// Fetches and locks the bucket pair that contains all entries for the given key pair,
    /// ensuring no deadlock.
    fn lockForKeyPair(
        futex: *Self,
        key_1: *const anyopaque,
        key_2: *const anyopaque,
    ) struct { *Bucket, *Bucket } {
        while (true) {
            // Fetch the active table and lock the first bucket.
            // The active table might be different after the locking is complete.
            const table = futex.getHashTable();
            const bucket_1 = if (@intFromPtr(key_1) <= @intFromPtr(key_2))
                table.getBucket(key_1)
            else
                table.getBucket(key_2);

            // Lock the first bucket.
            bucket_1.lock.lock();

            // Check if the table has been rehashed, in whick case we unlock and retry.
            // The active table can not change while we hold the bucket lock.
            if (futex.hash_table.load(.monotonic) != table) {
                bucket_1.lock.unlock();
                continue;
            }

            // Lock the second bucket and return.
            if (key_1 == key_2) return .{ bucket_1, bucket_1 };
            if (@intFromPtr(key_1) <= @intFromPtr(key_2)) {
                const bucket_2 = table.getBucket(key_2);
                if (bucket_1 != bucket_2) bucket_2.lock.lock();
                return .{ bucket_1, bucket_2 };
            } else {
                const bucket_2 = table.getBucket(key_1);
                if (bucket_1 != bucket_2) bucket_2.lock.lock();
                return .{ bucket_2, bucket_1 };
            }
        }
    }

    fn unlock(self: *Bucket) void {
        self.lock.unlock();
    }

    /// Unlocks a bucket pair locked with `lockForKeyPair`.
    fn unlockBucketPair(bucket_1: *Bucket, bucket_2: *Bucket) void {
        bucket_1.lock.unlock();
        if (bucket_1 != bucket_2) bucket_2.lock.unlock();
    }

    /// Rehashes the current bucket into a new hash table.
    /// The bucket must be locked and the table must not be shared.
    fn rehashInto(self: *Bucket, table: *HashTable) void {
        var current = self.head;
        while (current) |curr| : ({
            current = curr.next;
            curr.next = null;
        }) {
            const slot = table.slotForPtr(curr.key.load(.monotonic));
            if (table.buckets[slot].tail) |tail| {
                tail.next = curr;
            } else {
                table.buckets[slot].head = curr;
            }
            table.buckets[slot].tail = curr;
        }
    }
};

const HashTable = struct {
    buckets: []Bucket,
    hash_bits: u32,
    prev: ?*HashTable,

    const load_factor = 3;

    fn init(allocator: Allocator, num_waiters: usize, prev: ?*HashTable) *HashTable {
        const num_buckets = std.math.ceilPowerOfTwo(
            usize,
            num_waiters * load_factor,
        ) catch @panic("overflow");
        const hash_bits = @bitSizeOf(usize) - @clz(num_buckets) - 1;

        const buckets = allocator.alloc(Bucket, num_buckets) catch @panic("oom");
        for (buckets) |*b| b.* = .{};

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

/// Initializes a new wait list.
pub fn init(gpa: std.mem.Allocator) Self {
    return Self{ .gpa = gpa };
}

/// Destroys the wait list.
///
/// Panics if there are any pending waiters.
pub fn deinit(self: *Self) void {
    if (self.num_waiters.load(.monotonic) != 0) @panic("not empty");
    if (self.hash_table.load(.acquire)) |table| table.deinit(self.gpa);
}

/// Fetches the active hash table.
fn getHashTable(self: *Self) *HashTable {
    // Fast path, the table is already initialized.
    const table = self.hash_table.load(.acquire);
    if (table) |t| return t;

    // Try to initialize and promote a hash table to be active.
    const allocator = self.gpa;
    const new_table = HashTable.init(allocator, HashTable.load_factor, null);
    if (self.hash_table.cmpxchgStrong(null, new_table, .acq_rel, .acquire)) |old| {
        new_table.deinit(allocator);
        return old.?;
    }
    return new_table;
}

/// Atomically replaces the active hash table with one big enough to accomodate all waiters.
fn growHashTable(self: *Self, num_waiters: usize) void {
    const old_table: *HashTable = blk: {
        while (true) {
            // Fast path, the table is already big enough.
            const table = self.getHashTable();
            if (table.buckets.len >= HashTable.load_factor * num_waiters) return;

            // Lock all buckets to ensure that the table is not in use while we rehash
            // the buckets into the new table. Multiple grow operations may run in parallel,
            // so we must recheck that the table is still active.
            for (table.buckets) |*bucket| bucket.lock.lock();
            if (self.hash_table.load(.monotonic) == table) break :blk table;

            // Now that we know that the active table was swapped, unlock all buckets and retry.
            for (table.buckets) |*bucket| bucket.lock.unlock();
        }
    };
    defer for (old_table.buckets) |*bucket| bucket.lock.unlock();

    // With all buckets locked, we rehash them into the new table and promote it to be active.
    const new_table = HashTable.init(self.gpa, num_waiters, old_table);
    for (old_table.buckets) |*bucket| {
        bucket.rehashInto(new_table);
    }
    self.hash_table.store(new_table, .release);
}

/// Atomically checks if the value at `key` equals `expect`.
fn checkKeyEquality(key: *const anyopaque, key_size: usize, expect: u64) bool {
    switch (key_size) {
        1 => {
            const value: *const atomic.Value(u8) = @ptrCast(@alignCast(key));
            return value.load(.monotonic) == @as(u8, @truncate(expect));
        },
        2 => {
            const value: *const atomic.Value(u16) = @ptrCast(@alignCast(key));
            return value.load(.monotonic) == @as(u16, @truncate(expect));
        },
        4 => {
            const value: *const atomic.Value(u32) = @ptrCast(@alignCast(key));
            return value.load(.monotonic) == @as(u32, @truncate(expect));
        },
        8 => {
            const value: *const atomic.Value(u64) = @ptrCast(@alignCast(key));
            return value.load(.monotonic) == @as(u64, expect);
        },
        else => @panic("invalid key size"),
    }
}

/// Puts the caller to sleep if the value pointed to by `key` equals `expect`.
///
/// If the value does not match, the function returns imediately with `error.Invalid`. The
/// `key_size` parameter specifies the size of the value in bytes and must be either of `1`, `2`,
/// `4` or `8`, in which case `key` is treated as pointer to `u8`, `u16`, `u32`, or
/// `u64` respectively, and `expect` is truncated. The `token` is a user definable integer to store
/// additional metadata about the waiter, which can be utilized to controll some wake operations.
///
/// If `timeout` is set, and it is reached before a wake operation wakes the task, the task will be
/// resumed, and the function returns `error.Timeout`.
pub fn wait(
    self: *Self,
    key: *const anyopaque,
    key_size: usize,
    expect: u64,
    token: usize,
    timeout: ?Instant,
) error{ Invalid, Timeout }!void {
    // Create a new waiter and a new entry for the key.
    var waiter = Waiter.init(1);
    var entry = Entry.init(self, key, key_size, token, &waiter);
    defer Entry.unregisterOne(self);

    // Try to enqueue the entry into its bucket.
    try entry.enqueue(self, expect);

    // Wait until we are notified or the timeout is reached.
    const woken_up = waiter.wait(timeout);

    // If wait returned true we know for shure that we were woken up and are not
    // enqueued in a bucket. Note: The inverse is not always true.
    if (woken_up) {
        waiter.ensureUniqueReference();
        return;
    }

    // Dequeue the entry.
    const timed_out = entry.dequeueCheckTimeout(self);
    if (timed_out) return error.Timeout;
}

/// Puts the caller to sleep if all keys match their expected values.
///
/// Is a generalization of `wait` for multiple keys. At least `1` key, and at most
/// `max_waitv_key_count` may be passed to this function. Otherwise it returns `error.KeyError`.
pub fn waitv(
    self: *Self,
    keys: []const KeyExpect,
    timeout: ?Instant,
) error{ KeyError, Invalid, Timeout }!usize {
    if (keys.len == 0 or keys.len > max_waitv_key_count) return error.KeyError;
    if (keys.len == 1) {
        const k = keys[0];
        try self.wait(k.key, k.key_size, k.expect, k.token, timeout);
        return 0;
    }

    // Create a new waiter and a new entry for each key.
    var waiter = Waiter.init(keys.len);
    var entry_buffer: [max_waitv_key_count]Entry = undefined;
    const entries = entry_buffer[0..keys.len];
    for (keys, entries) |k, *e| e.* = Entry.init(self, k.key, k.key_size, k.token, &waiter);
    defer Entry.unregisterMany(self, keys.len);

    // Try to enqueue all entries into their buckets.
    var enqueue_count: usize = 0;
    for (keys, entries) |k, *entry| {
        entry.enqueue(self, k.expect) catch break;
        enqueue_count += 1;
    }

    // If we weren`t able to enqueue all entries, there either was a value mismatch,
    // or we were woken up. Either way, we must dequeue all entries before returning.
    if (enqueue_count != keys.len) {
        for (entries[0..enqueue_count]) |*entry| _ = entry.dequeueCheckTimeout(self);
        waiter.ensureUniqueReference();
        const signaled_key = waiter.signaled_key orelse return error.Invalid;
        for (entries[0..enqueue_count], 0..) |*entry, i| {
            if (entry.key.load(.monotonic) == signaled_key) return i;
        }
        unreachable;
    }

    // Wait until we are woken up or the timeout is reached.
    const woken_up = waiter.wait(timeout);

    // If we were woken up, we optimize the dequeue slightly, as we lock one less bucket.
    if (woken_up) {
        waiter.ensureUniqueReference();
        var wake_index: usize = undefined;
        const signaled_key = waiter.signaled_key.?;
        for (entries, 0..) |*entry, i| {
            if (entry.key.load(.monotonic) == signaled_key) {
                wake_index = i;
                continue;
            }
            const timed_out = entry.dequeueCheckTimeout(self);
            std.debug.assert(!timed_out);
        }
        return wake_index;
    }

    // Dequeue all entries.
    var timed_out: bool = false;
    for (entries) |*entry| timed_out = entry.dequeueCheckTimeout(self) or timed_out;
    if (timed_out) return error.Timeout;

    // Now that we know that we didn`t timeout, we return the index of the signaled key.
    waiter.ensureUniqueReference();
    const signaled_key = waiter.signaled_key.?;
    for (entries, 0..) |*entry, i| {
        if (entry.key.load(.monotonic) == signaled_key) return i;
    }
    unreachable;
}

/// Wakes at most `max_waiters` waiting on `key`.
///
/// Uses the token provided by the waiter and the `filter` to determine whether to ignore it from
/// being woken up. Returns the number of woken waiters.
pub fn wakeFilter(self: *Self, key: *const anyopaque, max_waiters: usize, filter: Filter) usize {
    if (max_waiters == 0) return 0;
    var wake_count: usize = 0;

    var wake_list_head: ?*Entry = null;
    var wake_list_tail: ?*Entry = null;
    {
        const bucket = Bucket.lockForKey(self, key);
        defer bucket.unlock();

        // Go through the queue looking for entries with a matching key.
        var link: *?*Entry = &bucket.head;
        var current: ?*Entry = bucket.head;
        var previous: ?*Entry = null;
        while (current) |curr| {
            // Skip entries with wrong keys.
            if (wake_count == max_waiters) break;
            if (curr.key.load(.monotonic) != key) {
                link = &curr.next;
                current = curr.next;
                previous = curr;
                continue;
            }

            // Skip entries that are filtered out.
            if (!filter.checkToken(curr.token)) {
                link = &curr.next;
                current = curr.next;
                previous = curr;
                continue;
            }

            // Check if the waiter can be woken up.
            if (!curr.waiter.prepareWake(key)) {
                link = &curr.next;
                current = curr.next;
                previous = curr;
                continue;
            }

            // Remove the entry from the queue.
            link.* = curr.next;
            current = curr.next;
            if (bucket.tail == curr) bucket.tail = previous;

            // Append the entry to the waker list.
            curr.next = null;
            if (wake_list_head != null) wake_list_tail.?.next = curr else wake_list_head = curr;
            wake_list_tail = curr;
            wake_count += 1;
        }
    }

    // Wake all entries.
    while (wake_list_head) |entry| {
        // The read of the `next` pointer must occur first, as the entry is
        // invalidated after `wake`.
        wake_list_head = entry.next;
        entry.waiter.wake(self);
    }

    return wake_count;
}

/// Wakes at most `max_waiters` waiting on `key`.
///
/// Returns the number of woken waiters.
pub fn wake(self: *Self, key: *const anyopaque, max_waiters: usize) usize {
    return self.wakeFilter(key, max_waiters, .all);
}

/// Requeues waiters from `key_from` to `key_to`.
///
/// Checks if the value behind `key_from` equals `expect`, in which case up to a maximum of
/// `max_wakes` waiters are woken up from `key_from` and a maximum of `max_requeues` waiters
/// are requeued from the `key_from` queue to the `key_to` queue. If the value does not match
/// the function returns `error.Invalid`. Uses the token provided by the waiter and the `filter`
/// to determine whether to ignore it from being woken up.
pub fn requeueFilter(
    self: *Self,
    key_from: *const anyopaque,
    key_to: *const anyopaque,
    key_size: usize,
    expect: u64,
    max_wakes: usize,
    max_requeues: usize,
    filter: Filter,
) error{Invalid}!RequeueResult {
    if (max_wakes == 0 and max_requeues == 0) return .{};
    var wake_count: usize = 0;
    var requeue_count: usize = 0;

    var wake_list_head: ?*Entry = null;
    var wake_list_tail: ?*Entry = null;
    {
        // Lock the two buckets.
        const bucket_from, const bucket_to = Bucket.lockForKeyPair(self, key_from, key_to);
        defer Bucket.unlockBucketPair(bucket_from, bucket_to);
        if (!checkKeyEquality(key_from, key_size, expect)) return error.Invalid;

        // Go through the queue looking for entries with a matching key.
        var link: *?*Entry = &bucket_from.head;
        var current: ?*Entry = bucket_from.head;
        var previous: ?*Entry = null;
        while (current) |curr| {
            // Skip entries with wrong keys.
            if (wake_count == max_wakes and requeue_count == max_requeues) break;
            if (curr.key.load(.monotonic) != key_from) {
                link = &curr.next;
                current = curr.next;
                previous = curr;
                continue;
            }

            // Skip entries that are filtered out.
            if (!filter.checkToken(curr.token)) {
                link = &curr.next;
                current = curr.next;
                previous = curr;
                continue;
            }

            if (wake_count < max_wakes) {
                // Check if the waiter can be woken up.
                if (!curr.waiter.prepareWake(key_from)) {
                    link = &curr.next;
                    current = curr.next;
                    previous = curr;
                    continue;
                }

                // Remove the entry from the queue.
                link.* = curr.next;
                current = curr.next;
                if (bucket_from.tail == curr) bucket_from.tail = previous;

                // Append the entry to the waker list.
                curr.next = null;
                if (wake_list_head != null) wake_list_tail.?.next = curr else wake_list_head = curr;
                wake_list_tail = curr;
                wake_count += 1;
            } else {
                curr.key.store(key_to, .monotonic);
                requeue_count += 1;

                // If the entries are in the same bucket we keep them in place.
                if (bucket_from == bucket_to) {
                    link = &curr.next;
                    current = curr.next;
                    previous = curr;
                    continue;
                }

                // Remove the entry from the queue.
                link.* = curr.next;
                if (bucket_from.tail == curr) bucket_from.tail = previous;

                // Requeue the entry onto the destination bucket.
                if (bucket_to.head) |_| {
                    bucket_to.tail.?.next = curr;
                } else {
                    bucket_to.head = curr;
                }
                bucket_to.tail = curr;

                current = curr.next;
                curr.next = null;
            }
        }
    }

    // Wake all entries.
    while (wake_list_head) |entry| {
        // The read of the `next` pointer must occur first, as the entry is
        // invalidated after `wake`.
        wake_list_head = entry.next;
        entry.waiter.wake(self);
    }

    return .{ .wake_count = wake_count, .requeue_count = requeue_count };
}

/// Requeues waiters from `key_from` to `key_to`.
///
/// Checks if the value behind `key_from` equals `expect`, in which case up to a maximum of
/// `max_wakes` waiters are woken up from `key_from` and a maximum of `max_requeues` waiters
/// are requeued from the `key_from` queue to the `key_to` queue. If the value does not match
/// the function returns `error.Invalid`.
pub fn requeue(
    self: *Self,
    key_from: *const anyopaque,
    key_to: *const anyopaque,
    key_size: usize,
    expect: u64,
    max_wakes: usize,
    max_requeues: usize,
) error{Invalid}!RequeueResult {
    return self.requeueFilter(key_from, key_to, key_size, expect, max_wakes, max_requeues, .all);
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

    fn wakeOne(self: *@This(), unpark_index: usize) void {
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
            const woken_up = self.pl.wake(self.semaphoreAddr(), std.math.maxInt(usize));
            std.debug.assert(woken_up <= num_threads_left);
            while (self.num_awake.load(.seq_cst) != num_awake_before_unpark + woken_up) {
                Thread.yield() catch unreachable;
            }
            num_threads_left -= woken_up;
        }
    }

    fn down(self: *@This()) void {
        const old_semaphore_value = self.semaphore.fetchSub(1, .seq_cst);
        if (old_semaphore_value > 0) return;

        while (true) {
            const current = self.semaphore.load(.seq_cst);
            self.pl.wait(self.semaphoreAddr(), @sizeOf(isize), @bitCast(current), 0, null) catch continue;
            break;
        }
    }

    fn up(self: *@This()) void {
        const old_semaphore_value = self.semaphore.fetchAdd(1, .seq_cst);
        if (old_semaphore_value < 0) {
            while (true) {
                const woken_up = self.pl.wake(self.semaphoreAddr(), 1);
                switch (woken_up) {
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
        for (tests) |*t| t.wakeOne(unpark_index);
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

test "futex wake all one fast" {
    try testWrapper(1000, 1, 0, 1, 0);
}

test "futex wake all hundred fast" {
    try testWrapper(100, 1, 0, 100, 0);
}

test "futex wake one one fast" {
    try testWrapper(1000, 1, 0, 1, 1);
}

test "futex wake one hundred fast" {
    try testWrapper(20, 1, 0, 100, 100);
}

test "futex wake one fifty then fifty all fast" {
    try testWrapper(50, 1, 0, 100, 50);
}

test "futex wake all one" {
    try testWrapper(100, 1, 10000, 1, 0);
}

test "futex wake all hundred" {
    try testWrapper(100, 1, 10000, 100, 0);
}

test "futex wake one one" {
    try testWrapper(10, 1, 10000, 1, 1);
}

test "futex wake one fifty" {
    try testWrapper(1, 1, 10000, 50, 50);
}

test "futex wake one fifty then fifty all" {
    try testWrapper(2, 1, 10000, 100, 50);
}

test "futex wake hundred all one fast" {
    try testWrapper(100, 100, 0, 1, 0);
}

test "futex wake hundred all one" {
    try testWrapper(1, 100, 10000, 1, 0);
}
