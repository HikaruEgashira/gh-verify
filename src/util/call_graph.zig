const std = @import("std");

pub const NodeId = u32;

pub const NodeKind = enum { file, function };

pub const NodeDescriptor = struct {
    file_index: u16,
    name: []const u8,
    kind: NodeKind,
};

pub const CallGraph = struct {
    nodes: std.ArrayList(NodeDescriptor),
    parent: std.ArrayList(NodeId),
    rank: std.ArrayList(u8),

    pub const empty: CallGraph = .{
        .nodes = .empty,
        .parent = .empty,
        .rank = .empty,
    };

    /// Add a node. Returns existing ID if (file_index, name) already present.
    pub fn addNode(self: *CallGraph, alloc: std.mem.Allocator, file_index: u16, name: []const u8, kind: NodeKind) !NodeId {
        // Deduplicate by (file_index, name)
        for (self.nodes.items, 0..) |n, i| {
            if (n.file_index == file_index and std.mem.eql(u8, n.name, name)) {
                return @intCast(i);
            }
        }
        const id: NodeId = @intCast(self.nodes.items.len);
        try self.nodes.append(alloc, .{ .file_index = file_index, .name = name, .kind = kind });
        try self.parent.append(alloc, id); // self-parent
        try self.rank.append(alloc, 0);
        return id;
    }

    /// Find root with path compression.
    pub fn find(self: *CallGraph, x: NodeId) NodeId {
        var current = x;
        while (self.parent.items[current] != current) {
            self.parent.items[current] = self.parent.items[self.parent.items[current]];
            current = self.parent.items[current];
        }
        return current;
    }

    /// Union by rank.
    pub fn merge(self: *CallGraph, a: NodeId, b: NodeId) void {
        const ra = self.find(a);
        const rb = self.find(b);
        if (ra == rb) return;
        if (self.rank.items[ra] < self.rank.items[rb]) {
            self.parent.items[ra] = rb;
        } else if (self.rank.items[ra] > self.rank.items[rb]) {
            self.parent.items[rb] = ra;
        } else {
            self.parent.items[rb] = ra;
            self.rank.items[ra] += 1;
        }
    }

    /// Count distinct connected components among file-kind nodes only.
    pub fn componentCount(self: *CallGraph) u32 {
        if (self.nodes.items.len == 0) return 0;
        var seen: std.AutoHashMap(NodeId, void) = .init(std.heap.page_allocator);
        defer seen.deinit();
        for (self.nodes.items, 0..) |n, i| {
            if (n.kind != .file) continue;
            const root = self.find(@intCast(i));
            seen.put(root, {}) catch continue;
        }
        return @intCast(seen.count());
    }

    /// Return file indices grouped by component.
    pub fn getComponents(self: *CallGraph, alloc: std.mem.Allocator) ![][]u16 {
        // Map root -> list of file indices
        var comp_map: std.AutoHashMap(NodeId, std.ArrayList(u16)) = .init(alloc);
        defer {
            var it = comp_map.iterator();
            while (it.next()) |entry| {
                entry.value_ptr.deinit(alloc);
            }
            comp_map.deinit();
        }

        for (self.nodes.items, 0..) |n, i| {
            if (n.kind != .file) continue;
            const root = self.find(@intCast(i));
            const gop = try comp_map.getOrPut(root);
            if (!gop.found_existing) {
                gop.value_ptr.* = .empty;
            }
            try gop.value_ptr.append(alloc, n.file_index);
        }

        var result: std.ArrayList([]u16) = .empty;
        var it = comp_map.iterator();
        while (it.next()) |entry| {
            try result.append(alloc, try entry.value_ptr.toOwnedSlice(alloc));
        }
        return try result.toOwnedSlice(alloc);
    }
};
