const std = @import("std");
const testing = std.testing;
const Allocator = std.mem.Allocator;

pub const NodeId = struct { id: usize };
pub const EdgeId = struct { id: usize };

pub const EdgeDirection = enum {
    /// Edge ending at a specific node.
    incoming,
    /// Edge starting from a specific node.
    outgoing,
};

fn Node(comptime T: type) type {
    return struct {
        data: T,
        incoming: std.AutoArrayHashMapUnmanaged(NodeId, EdgeId) = .{},
        outgoing: std.AutoArrayHashMapUnmanaged(NodeId, EdgeId) = .{},
    };
}

fn Edge(comptime T: type) type {
    return struct {
        data: T,
        from: NodeId,
        to: NodeId,
    };
}

fn IdMap(comptime T: type) type {
    return struct {
        const Self = @This();

        next_id: usize = 0,
        dtor_fn: *const fn (allocator: Allocator, ptr: *T) void,
        map: std.AutoArrayHashMapUnmanaged(usize, T) = .{},
        free_ids: std.ArrayListUnmanaged(usize) = .{},

        const Iterator = std.AutoArrayHashMapUnmanaged(usize, T).Iterator;

        fn init(comptime dtor: fn (allocator: Allocator, ptr: *T) void) Self {
            return Self{ .dtor_fn = dtor };
        }

        fn deinit(self: *Self, allocator: Allocator) void {
            self.clear(allocator);
            self.map.deinit(allocator);
            self.free_ids.deinit(allocator);
        }

        fn count(self: *const Self) usize {
            return self.map.count();
        }

        fn add(self: *Self, allocator: Allocator, value: T) !usize {
            const id_from_free_list = self.free_ids.items.len != 0;
            const id = self.free_ids.popOrNull() orelse self.next_id;
            errdefer if (id_from_free_list) self.free_ids.appendAssumeCapacity(id);
            try self.map.put(allocator, id, value);
            if (!id_from_free_list) self.next_id += 1;
            return id;
        }

        fn remove(self: *Self, allocator: Allocator, id: usize) bool {
            var element = self.fetchRemove(allocator, id) catch return false;
            self.dtor_fn(allocator, &element);
            return true;
        }

        fn fetchRemove(self: *Self, allocator: Allocator, id: usize) !T {
            const value = self.map.fetchSwapRemove(id) orelse return error.NotFound;
            errdefer self.map.putAssumeCapacity(id, value.value);
            if (id == self.next_id - 1) {
                self.next_id -= 1;
                return value.value;
            } else {
                try self.free_ids.append(allocator, id);
            }
            return value.value;
        }

        fn contains(self: *const Self, id: usize) bool {
            return self.map.contains(id);
        }

        fn get(self: *const Self, id: usize) ?*T {
            return self.map.getPtr(id);
        }

        fn iterator(self: *const Self) Iterator {
            return self.map.iterator();
        }

        fn clear(self: *Self, allocator: Allocator) void {
            var iter = self.map.iterator();
            while (iter.next()) |entry| {
                self.dtor_fn(allocator, entry.value_ptr);
            }
            self.free_ids.clearAndFree(allocator);
            self.next_id = 0;
        }

        fn clone(
            self: *const Self,
            allocator: Allocator,
            comptime clone_fn: fn (allocator: Allocator, value: *const T) anyerror!T,
        ) !Self {
            const free_ids = try allocator.dupe(usize, self.free_ids.items);
            errdefer allocator.free(free_ids);

            var map: std.AutoArrayHashMapUnmanaged(usize, T) = .{};
            try map.ensureTotalCapacity(allocator, self.map.count());
            errdefer map.deinit(allocator);
            errdefer {
                var it = map.iterator();
                while (it.next()) |e| {
                    self.dtor_fn(allocator, e.value_ptr);
                }
            }

            var it = map.iterator();
            while (it.next()) |e| {
                var duped = try clone_fn(allocator, e.value_ptr);
                errdefer self.dtor_fn(allocator, &duped);
                try map.put(allocator, e.key_ptr.*, duped);
            }

            return Self{
                .next_id = self.next_id,
                .dtor_fn = self.dtor_fn,
                .map = map,
                .free_ids = free_ids,
            };
        }
    };
}

pub fn GraphUnmanaged(comptime N: type, comptime E: type) type {
    return struct {
        const Self = @This();

        nodes: IdMap(Node(N)),
        edges: IdMap(Edge(E)),

        pub const NodeEntry = struct {
            id: NodeId,
            data_ptr: *N,
        };

        pub const EdgeEntry = struct {
            id: EdgeId,
            data_ptr: *E,
        };

        pub const NeighborsEntry = struct {
            node_id: NodeId,
            edge_id: EdgeId,
        };

        pub const NodesIterator = struct {
            inner: IdMap(Node(N)).Iterator,

            pub fn next(it: *NodesIterator) ?NodeEntry {
                const node = it.inner.next();
                if (node) |n| return .{ .id = .{ .id = n.key_ptr.* }, .data_ptr = &n.value_ptr.data };
                return null;
            }

            pub fn reset(it: *NodesIterator) void {
                it.inner.reset();
            }
        };

        pub const EdgesIterator = struct {
            inner: IdMap(Edge(E)).Iterator,

            pub fn next(it: *EdgesIterator) ?EdgeEntry {
                const edge = it.inner.next();
                if (edge) |n| return .{ .id = .{ .id = n.key_ptr.* }, .data_ptr = &n.value_ptr.data };
                return null;
            }

            pub fn reset(it: *EdgesIterator) void {
                it.inner.reset();
            }
        };

        pub const ExternalsIterator = struct {
            inner: IdMap(Node(N)).Iterator,
            direction: EdgeDirection,

            pub fn next(it: *ExternalsIterator) ?NodeEntry {
                while (it.inner.next()) |node| {
                    const edges = switch (it.direction) {
                        .incoming => &node.value_ptr.incoming,
                        .outgoing => &node.value_ptr.outgoing,
                    };
                    if (edges.count() == 0) return .{
                        .id = .{ .id = node.key_ptr.* },
                        .data_ptr = &node.value_ptr.data,
                    };
                }
                return null;
            }

            pub fn reset(it: *ExternalsIterator) void {
                it.inner.reset();
            }
        };

        pub const NeighborsIterator = struct {
            inner: std.AutoArrayHashMapUnmanaged(NodeId, EdgeId).Iterator,

            pub fn next(it: *NeighborsIterator) ?NeighborsEntry {
                const entry = it.inner.next();
                if (entry) |e| return .{ .node_id = e.key_ptr.*, .edge_id = e.value_ptr.* };
                return null;
            }

            pub fn reset(it: *NeighborsIterator) void {
                it.inner.reset();
            }
        };

        /// Initializes a new graph.
        pub fn init(
            comptime node_destructor: ?fn (allocator: Allocator, element: *N) void,
            comptime edge_destructor: ?fn (allocator: Allocator, element: *E) void,
        ) Self {
            const node_dtor = struct {
                fn f(allocator: Allocator, node: *Node(N)) void {
                    if (node_destructor) |dtor| dtor(allocator, &node.data);
                    node.incoming.deinit(allocator);
                    node.outgoing.deinit(allocator);
                }
            }.f;
            const edge_dtor = struct {
                fn f(allocator: Allocator, edge: *Edge(E)) void {
                    if (edge_destructor) |dtor| dtor(allocator, &edge.data);
                }
            }.f;

            return Self{
                .nodes = IdMap(Node(N)).init(
                    node_dtor,
                ),
                .edges = IdMap(Edge(E)).init(
                    edge_dtor,
                ),
            };
        }

        /// Deinitializes the graph, running the provided destructor
        /// function for each contained element.
        pub fn deinit(self: *Self, allocator: Allocator) void {
            self.nodes.deinit(allocator);
            self.edges.deinit(allocator);
        }

        /// Returns the number of nodes in the graph.
        pub fn nodeCount(self: *const Self) usize {
            return self.nodes.count();
        }

        /// Returns the number of edges in the graph.
        pub fn edgeCount(self: *const Self) usize {
            return self.edges.count();
        }

        /// Returns the number of neighbors of a node.
        ///
        /// The edge direction specified whether to interpret the node as
        /// a start node or as an end node.
        pub fn neighborsCount(self: *const Self, node: NodeId, direction: EdgeDirection) !usize {
            const n = self.nodes.get(node.id) orelse return error.NodeNotFound;
            return switch (direction) {
                .incoming => n.incoming.count(),
                .outgoing => n.outgoing.count(),
            };
        }

        /// Adds a new node to the graph.
        pub fn addNode(self: *Self, allocator: Allocator, node: N) !NodeId {
            const id = try self.nodes.add(allocator, .{ .data = node });
            return .{ .id = id };
        }

        /// Access the data associated with a node.
        pub fn nodePtr(self: *const Self, node: NodeId) ?*N {
            const n = self.nodes.get(node.id) orelse return null;
            return &n.data;
        }

        /// Adds an edge from the `from` node to the `to` node.
        ///
        /// If the edge already existed, this function updates the associated
        /// edge data and returns the original edge id.
        pub fn addEdge(
            self: *Self,
            allocator: Allocator,
            edge: E,
            from: NodeId,
            to: NodeId,
        ) (Allocator.Error || error{NodeNotFound})!struct { id: EdgeId, old: ?E } {
            const from_n = self.nodes.get(from.id) orelse return error.NodeNotFound;
            const to_n = self.nodes.get(to.id) orelse return error.NodeNotFound;

            if (from_n.outgoing.get(to)) |id| {
                const e = self.edges.get(id.id) orelse unreachable;
                const old = e.data;
                e.data = edge;
                return .{ .id = id, .old = old };
            }

            const id = try self.edges.add(allocator, .{
                .data = edge,
                .from = from,
                .to = to,
            });
            errdefer if (!self.edges.remove(allocator, id)) @panic("Unrecoverable error");

            try from_n.outgoing.put(allocator, to, .{ .id = id });
            errdefer if (!from_n.outgoing.orderedRemove(to)) @panic("Unrecoverable error");
            try to_n.incoming.put(allocator, from, .{ .id = id });
            errdefer if (!to_n.outgoing.orderedRemove(from)) @panic("Unrecoverable error");

            return .{ .id = .{ .id = id }, .old = null };
        }

        /// Updates the data associated with the edge from the `from` node to the `to` node.
        pub fn updateEdge(
            self: *Self,
            edge: E,
            from: NodeId,
            to: NodeId,
        ) !struct { id: EdgeId, old: E } {
            const from_n = self.nodes.get(from.id) orelse return error.NodeNotFound;
            if (!self.nodes.contains(to.id)) return error.NodeNotFound;

            if (from_n.outgoing.get(to)) |id| {
                const e = self.edges.get(id.id) orelse unreachable;
                const old = e.data;
                e.data = edge;
                return .{ .id = id, .old = old };
            }
            return error.EdgeNotFound;
        }

        /// Access the data associated with an edge.
        pub fn edgePtr(self: *const Self, edge: EdgeId) ?*E {
            const e = self.edges.get(edge.id) orelse return null;
            return &e.data;
        }

        /// Returns the node entpoints of an edge.
        pub fn edgeEndpoints(self: *const Self, edge: EdgeId) !struct { from: NodeId, to: NodeId } {
            const e = self.edges.get(edge.id) orelse return error.EdgeNotFound;
            return .{ .from = e.from, .to = e.to };
        }

        /// Removes a node and all its edges from the graph.
        pub fn removeNode(self: *Self, allocator: Allocator, node: NodeId) !N {
            var n = self.nodes.fetchRemove(allocator, node.id) catch return error.NodeNotFound;
            var incoming = n.incoming.iterator();
            while (incoming.next()) |edge| {
                if (!self.edges.remove(allocator, edge.value_ptr.id)) @panic("Unrecoverable error");
                const start = self.nodes.get(edge.key_ptr.id) orelse unreachable;
                _ = start.outgoing.orderedRemove(node);
            }
            var outgoing = n.outgoing.iterator();
            while (outgoing.next()) |edge| {
                if (!self.edges.remove(allocator, edge.value_ptr.id)) @panic("Unrecoverable error");
                const end = self.nodes.get(edge.key_ptr.id) orelse unreachable;
                _ = end.incoming.orderedRemove(node);
            }
            n.incoming.deinit(allocator);
            n.outgoing.deinit(allocator);
            return n.data;
        }

        /// Removes an edge from the graph.
        pub fn removeEdge(self: *Self, allocator: Allocator, edge: EdgeId) !E {
            const e = self.edges.fetchRemove(allocator, edge.id) catch return error.EdgeNotFound;
            const from = self.nodes.get(e.from.id) orelse unreachable;
            const to = self.nodes.get(e.to.id) orelse unreachable;
            _ = from.outgoing.orderedRemove(e.to);
            _ = to.incoming.orderedRemove(e.from);
            return e.data;
        }

        /// Returns whether the graph contains an edge from the `from` node to the `to` node.
        pub fn containsEdge(
            self: *const Self,
            from: NodeId,
            to: NodeId,
        ) !bool {
            return self.findEdge(from, to) != null;
        }

        /// Finds the id of the edge from the `from` node to the `to` node.
        pub fn findEdge(
            self: *const Self,
            from: NodeId,
            to: NodeId,
        ) !?EdgeId {
            const from_n = self.nodes.get(from.id) orelse return error.NodeNotFound;
            if (!self.nodes.contains(to.id)) return error.NodeNotFound;
            return from_n.outgoing.get(to);
        }

        /// Constructs a new iterator over the nodes of the graph.
        pub fn nodesIterator(self: *const Self) NodesIterator {
            return .{ .inner = self.nodes.iterator() };
        }

        /// Constructs a new iterator over the edges of the graph.
        pub fn edgesIterator(self: *const Self) EdgesIterator {
            return .{ .inner = self.edges.iterator() };
        }

        /// Constructs a new iterator over the external nodes of the graph.
        pub fn externalsIterator(self: *const Self, direction: EdgeDirection) ExternalsIterator {
            return .{ .inner = self.nodes.iterator(), .direction = direction };
        }

        /// Constructs a new iterator over the neighbor nodes of another node.
        pub fn neighborsIterator(
            self: *const Self,
            node: NodeId,
            direction: EdgeDirection,
        ) !NeighborsIterator {
            const n = self.nodes.get(node.id) orelse return error.NodeNotFound;
            const it = switch (direction) {
                .incoming => n.incoming.iterator(),
                .outgoing => n.outgoing.iterator(),
            };
            return .{ .inner = it };
        }

        /// Removes all nodes and edges from the graph.
        pub fn clear(self: *Self, allocator: Allocator) void {
            self.nodes.clear(allocator);
            self.edges.clear(allocator);
        }

        /// Removes all edges from the graph.
        pub fn clearEdges(self: *Self, allocator: Allocator) void {
            var it = self.nodes.iterator();
            while (it.next()) |node| {
                node.value_ptr.incoming.clearAndFree(allocator);
                node.value_ptr.outgoing.clearAndFree(allocator);
            }
            self.edges.clear(allocator);
        }

        /// Reverses the direction of all edges contained in the graph.
        pub fn reverseDirection(self: *Self) void {
            var nodes = self.nodes.iterator();
            while (nodes.next()) |node| {
                const tmp = node.value_ptr.incoming;
                node.value_ptr.incoming = node.value_ptr.outgoing;
                node.value_ptr.outgoing = tmp;
            }

            var edges = self.edges.iterator();
            while (edges.next()) |edge| {
                const tmp = edge.value_ptr.from;
                edge.value_ptr.from = edge.value_ptr.to;
                edge.value_ptr.to = tmp;
            }
        }

        /// Clones the contents of the graph into a new graph.
        pub fn clone(
            self: *const Self,
            allocator: Allocator,
            comptime clone_node: ?fn (allocator: Allocator, value: *const N) anyerror!N,
            comptime clone_edge: ?fn (allocator: Allocator, value: *const E) anyerror!E,
        ) !Self {
            const clone_n = struct {
                fn f(al: Allocator, value: *const Node(N)) !Node(N) {
                    const incoming = try value.incoming.clone(al);
                    errdefer incoming.deinit(al);
                    const outgoing = try value.outgoing.clone(al);
                    errdefer outgoing.deinit(al);
                    const data = if (comptime clone_node) |cl| cl(
                        al,
                        &value.data,
                    ) else value.data;
                    return Node(N){
                        .data = data,
                        .incoming = incoming,
                        .outgoing = outgoing,
                    };
                }
            }.f;
            const clone_e = struct {
                fn f(al: Allocator, value: *const Edge(E)) !Edge(E) {
                    const data = if (comptime clone_edge) |cl| cl(
                        al,
                        &value.data,
                    ) else value.data;
                    return Edge(E){
                        .data = data,
                        .from = value.from,
                        .to = value.to,
                    };
                }
            };

            const nodes = try self.nodes.clone(allocator, clone_n);
            errdefer nodes.deinit(allocator);
            const edges = try self.edges.clone(allocator, clone_e);

            return Self{
                .nodes = nodes,
                .edges = edges,
            };
        }

        /// Checks whether there is a path from to `from` node to the `to` node.
        pub fn pathExists(
            self: *const Self,
            allocator: Allocator,
            from: NodeId,
            to: NodeId,
        ) (Allocator.Error || error{NodeNotFound})!bool {
            const start = self.nodes.get(from.id) orelse return error.NodeNotFound;
            if (!self.nodes.contains(to.id)) return error.NodeNotFound;

            // Fast path without allocating.
            if (start.outgoing.contains(to)) return true;

            var stack: std.ArrayListUnmanaged(NodeId) = .{};
            defer stack.deinit(allocator);
            var visited: std.AutoArrayHashMapUnmanaged(NodeId, void) = .{};
            defer visited.deinit(allocator);

            try stack.append(allocator, from);
            try visited.put(allocator, from, {});
            while (stack.popOrNull()) |current| {
                const node = self.nodes.get(current.id) orelse unreachable;
                if (node.outgoing.contains(to)) return true;

                var it = node.outgoing.iterator();
                while (it.next()) |next| {
                    if (visited.contains(next.key_ptr.*)) continue;
                    try stack.append(allocator, next.key_ptr.*);
                    try visited.put(allocator, next.key_ptr.*, {});
                }
            }

            return false;
        }

        /// Checks whether the graph contains a cycle.
        pub fn isCyclic(
            self: *const Self,
            allocator: Allocator,
        ) !bool {
            const VisitedState = enum { discovered, finished };
            var visited: std.AutoArrayHashMapUnmanaged(NodeId, VisitedState) = .{};
            defer visited.deinit(allocator);

            const isCyclicInner = struct {
                fn f(
                    self_: *const Self,
                    allocator_: Allocator,
                    visited_: *std.AutoArrayHashMapUnmanaged(NodeId, VisitedState),
                    node_id: NodeId,
                    node: *const Node(N),
                ) !bool {
                    const state = visited_.get(node_id);
                    if (state != null) return true;
                    try visited_.put(allocator_, node_id, .discovered);

                    var neighbors = node.outgoing.iterator();
                    while (neighbors.next()) |neighbor| {
                        const neighbor_id = neighbor.key_ptr.*;
                        const neighbor_state = visited_.get(neighbor_id);
                        if (neighbor_state) |st| {
                            if (st == .discovered) return true;
                            continue;
                        }
                        const neighbor_node = self_.nodes.get(neighbor_id.id) orelse unreachable;
                        if (try f(
                            self_,
                            allocator_,
                            visited_,
                            neighbor_id,
                            neighbor_node,
                        )) return true;
                    }

                    try visited_.put(allocator_, node_id, .finished);
                    return false;
                }
            }.f;

            var nodes = self.nodes.iterator();
            while (nodes.next()) |node| {
                const node_id = NodeId{ .id = node.key_ptr.* };
                const node_cyclic = try isCyclicInner(
                    self,
                    allocator,
                    &visited,
                    node_id,
                    node.value_ptr,
                );
                if (node_cyclic) return true;
            }

            return false;
        }

        /// Performs a topological sort of the graph, i.e., all nodes of
        /// the output slice appear before their neighbor nodes.
        pub fn sortTopological(
            self: *const Self,
            allocator: Allocator,
            direction: EdgeDirection,
        ) (Allocator.Error || error{GraphIsCyclic})![]NodeId {
            const Marker = enum { temp, perm };
            const Inner = struct {
                fn f(
                    self_: *const Self,
                    allocator_: Allocator,
                    markers: *std.AutoArrayHashMapUnmanaged(NodeId, Marker),
                    nodes: *std.ArrayListUnmanaged(NodeId),
                    direction_: EdgeDirection,
                    id: NodeId,
                    node: *const Node(N),
                ) !void {
                    if (markers.get(id)) |state| {
                        if (state == .temp) return error.GraphIsCyclic;
                        return;
                    }
                    try markers.put(allocator_, id, .temp);
                    const neighbors = switch (direction_) {
                        .incoming => &node.incoming,
                        .outgoing => &node.outgoing,
                    };
                    var iterator = neighbors.iterator();
                    while (iterator.next()) |neighbor| {
                        const neigh_id = neighbor.key_ptr.*;
                        const neigh = self_.nodes.get(neigh_id.id) orelse unreachable;
                        try f(
                            self_,
                            allocator_,
                            markers,
                            nodes,
                            direction_,
                            neigh_id,
                            neigh,
                        );
                    }
                    try markers.put(allocator_, id, .perm);
                    nodes.appendAssumeCapacity(id);
                }
            };

            var markers = std.AutoArrayHashMapUnmanaged(NodeId, Marker){};
            defer markers.deinit(allocator);

            var nodes = try std.ArrayListUnmanaged(NodeId).initCapacity(
                allocator,
                self.nodeCount(),
            );
            var iterator = self.nodes.iterator();
            while (iterator.next()) |node| {
                const id = NodeId{ .id = node.key_ptr.* };
                if (markers.get(id) != null) continue;
                try Inner.f(
                    self,
                    allocator,
                    &markers,
                    &nodes,
                    direction,
                    id,
                    node.value_ptr,
                );
            }
            std.mem.reverse(NodeId, nodes.items);
            return nodes.toOwnedSlice(allocator);
        }
    };
}

test "Zero sized nodes" {
    var graph = GraphUnmanaged(void, void).init(null, null);
    defer graph.deinit(testing.allocator);

    _ = try graph.addNode(testing.allocator, {});
    _ = try graph.addNode(testing.allocator, {});

    try std.testing.expect(graph.nodeCount() == 2);
}

test "Sized nodes" {
    var graph = GraphUnmanaged(usize, void).init(null, null);
    defer graph.deinit(testing.allocator);

    const a = try graph.addNode(testing.allocator, 5);
    const b = try graph.addNode(testing.allocator, 10);

    try std.testing.expect(graph.nodePtr(a).?.* == 5);
    try std.testing.expect(graph.nodePtr(b).?.* == 10);
}

test "Zero sized edges" {
    var graph = GraphUnmanaged(void, void).init(null, null);
    defer graph.deinit(testing.allocator);

    const a = try graph.addNode(testing.allocator, {});
    const b = try graph.addNode(testing.allocator, {});
    const c = try graph.addNode(testing.allocator, {});

    const ab = try graph.addEdge(testing.allocator, {}, a, b);
    try std.testing.expect(graph.edgeCount() == 1);
    try std.testing.expect(ab.old == null);
    const ab_new = try graph.addEdge(testing.allocator, {}, a, b);
    try std.testing.expect(graph.edgeCount() == 1);
    try std.testing.expect(ab_new.old != null);
    const bc = try graph.addEdge(testing.allocator, {}, b, c);
    try std.testing.expect(graph.edgeCount() == 2);
    try std.testing.expect(bc.old == null);
}
