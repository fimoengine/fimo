const std = @import("std");
const atomic = std.atomic;
const mem = std.mem;
const Alignment = mem.Alignment;
const Allocator = mem.Allocator;

const Futex = @import("Futex.zig");

pub fn ItemPool(Item: type) type {
    return struct {
        const Self = @This();

        const item_size = @max(@sizeOf(Node), @sizeOf(Item));
        const node_alignment: Alignment = .of(*anyopaque);
        const item_alignment: Alignment = node_alignment.max(.of(Item));

        const waiting_bit: usize = 0b1;
        const list_mask: usize = ~waiting_bit;

        const Node = struct {
            next: ?*align(item_alignment.toByteUnits()) @This(),
        };
        const NodePtr = *align(item_alignment.toByteUnits()) Node;
        const ItemPtr = *align(item_alignment.toByteUnits()) Item;

        const Head = packed struct(u128) {
            tag: usize = 0,
            head: usize = 0,
        };

        allocated: []align(item_alignment.toByteUnits()) u8,
        free_list: atomic.Value(Head) = .init(.{}),

        pub fn init(allocator: Allocator, capacity: usize) error{OutOfMemory}!Self {
            const allocated = try allocator.alignedAlloc(u8, item_alignment, capacity * item_size);

            var head: ?*NodePtr = null;
            for (0..capacity) |i| {
                const bytes = allocated[i * item_size ..][0..item_size];
                const node: NodePtr = @ptrCast(@alignCast(bytes));
                node.tag = 0;
                node.next = head;
                head = node;
            }

            return .{
                .allocated = allocated,
                .free_list = head,
            };
        }

        pub fn deinit(self: Self, allocator: Allocator) void {
            allocator.free(self.allocated);
        }

        pub fn create(self: *Self, futex: *Futex) *Item {
            var orig = self.free_list.load(.monotonic);
            while (true) {
                if (orig.head & list_mask == 0) {
                    @branchHint(.cold);
                    const next = Head{ .tag = orig.tag, .head = waiting_bit };
                    if (orig.head == 0) {
                        if (self.free_list.cmpxchgWeak(orig, next, .monotonic, .monotonic)) |v| {
                            orig = v;
                            continue;
                        }
                    }
                    const raw_mem: *[2]usize = @ptrCast(&self.free_list);
                    futex.wait(&raw_mem[1], @sizeOf(usize), waiting_bit, 0, null) catch {};
                    orig = self.free_list.load(.monotonic);
                }
                std.debug.assert(orig.head & waiting_bit == 0);

                const node: NodePtr = @ptrFromInt(orig.head);
                const next = Head{ .tag = orig.tag +% 1, .head = @intFromPtr(node.next) };

                if (self.free_list.cmpxchgWeak(orig, next, .acquire, .monotonic)) |v| {
                    orig = v;
                    continue;
                }

                const raw_mem: *align(item_alignment.toByteUnits()) [item_size]u8 = @ptrCast(node);
                const item = @as(ItemPtr, @ptrCast(raw_mem));
                item.* = undefined;
                return item;
            }
        }

        pub fn destroy(self: *Self, futex: *Futex, item: *Item) void {
            const raw_mem: *align(item_alignment.toByteUnits()) [item_size]u8 = @ptrCast(item);
            const node = @as(NodePtr, @ptrCast(raw_mem));

            var orig = self.free_list.load(.monotonic);
            while (true) {
                node.next = @ptrFromInt(orig.head & list_mask);
                const next = Head{ .tag = orig.tag +% 1, .head = @intFromPtr(node) };
                orig = self.free_list.cmpxchgWeak(
                    orig,
                    next,
                    .release,
                    .monotonic,
                ) orelse break;
            }
            if (orig.head == waiting_bit) futex.wake(&self.free_list, 1);
        }
    };
}
