const std = @import("std");
const rule = @import("rule.zig");
const symbol_extractor = @import("../util/symbol_extractor.zig");
const call_graph = @import("../util/call_graph.zig");

const NON_CODE_EXTENSIONS = [_][]const u8{
    ".md", ".rst", ".txt", ".json", ".yaml", ".yml", ".toml",
    ".lock", ".env", ".cfg", ".ini", ".css", ".scss",
    ".svg", ".png", ".jpg", ".gif", ".ico", ".woff", ".woff2",
};

const NON_CODE_PREFIXES = [_][]const u8{
    ".github/", "docs/",
};

const CodeFileEntry = struct { index: u16, file: rule.PrFile };

pub fn run(alloc: std.mem.Allocator, ctx: rule.RuleContext) ![]rule.RuleResult {
    const pr = switch (ctx.payload) {
        .pr => |p| p,
        .release => return passResult(alloc),
    };

    // Filter to code files with patches
    var code_files: std.ArrayList(CodeFileEntry) = .empty;
    for (pr.pr_files, 0..) |f, i| {
        if (f.patch == null) continue;
        if (isNonCodeFile(f.filename)) continue;
        try code_files.append(alloc, .{ .index = @intCast(i), .file = f });
    }

    // 0-1 code files: always scoped
    if (code_files.items.len <= 1) {
        return passResult(alloc);
    }

    // Extract symbols from each file
    var all_symbols: std.ArrayList(symbol_extractor.FileSymbols) = .empty;
    for (code_files.items) |entry| {
        const symbols = try symbol_extractor.extractSymbols(alloc, entry.file.filename, entry.file.patch.?);
        try all_symbols.append(alloc, symbols);
    }

    // Build call graph
    var graph: call_graph.CallGraph = .empty;

    // Create file-level nodes
    var file_nodes: std.ArrayList(call_graph.NodeId) = .empty;
    for (code_files.items, 0..) |entry, idx| {
        const node = try graph.addNode(alloc, entry.index, entry.file.filename, .file);
        try file_nodes.append(alloc, node);

        // Create definition nodes and merge with file node
        const symbols = all_symbols.items[idx];
        for (symbols.definitions) |def_name| {
            const def_node = try graph.addNode(alloc, entry.index, def_name, .function);
            graph.merge(node, def_node);
        }
    }

    // Cross-file edges: call-to-definition matching
    for (all_symbols.items, 0..) |syms_a, idx_a| {
        for (syms_a.calls) |call_name| {
            for (all_symbols.items, 0..) |syms_b, idx_b| {
                if (idx_a == idx_b) continue;
                for (syms_b.definitions) |def_name| {
                    if (std.mem.eql(u8, call_name, def_name)) {
                        graph.merge(file_nodes.items[idx_a], file_nodes.items[idx_b]);
                    }
                }
            }
        }
    }

    // Import edges: resolve import paths to changed files
    for (all_symbols.items, 0..) |syms_a, idx_a| {
        for (syms_a.imports) |import_path| {
            const resolved = resolveImport(import_path, code_files.items);
            if (resolved) |idx_b| {
                if (idx_a != idx_b) {
                    graph.merge(file_nodes.items[idx_a], file_nodes.items[idx_b]);
                }
            }
        }
    }

    // Count connected components
    const components = graph.componentCount();

    if (components <= 1) {
        return passResult(alloc);
    }

    // Build result with component details
    const severity: rule.Severity = if (components >= 3) .@"error" else .warning;

    const comp_groups = try graph.getComponents(alloc);

    var affected: std.ArrayList([]const u8) = .empty;
    var detail_buf: std.ArrayList(u8) = .empty;
    const writer = detail_buf.writer(alloc);

    for (comp_groups, 0..) |group, comp_idx| {
        try writer.print("  Component {d}:", .{comp_idx + 1});
        for (group) |file_idx| {
            const filename = pr.pr_files[file_idx].filename;
            try writer.print(" {s}", .{filename});
            try affected.append(alloc, filename);
        }
        try writer.print("\n", .{});
    }

    const message = try std.fmt.allocPrint(
        alloc,
        "PR has {d} disconnected change clusters",
        .{components},
    );

    const results = try alloc.alloc(rule.RuleResult, 1);
    results[0] = .{
        .rule_id = "detect-unscoped-change",
        .severity = severity,
        .message = message,
        .affected_files = try affected.toOwnedSlice(alloc),
        .suggestion = try detail_buf.toOwnedSlice(alloc),
    };
    return results;
}

fn passResult(alloc: std.mem.Allocator) ![]rule.RuleResult {
    const results = try alloc.alloc(rule.RuleResult, 1);
    results[0] = .{
        .rule_id = "detect-unscoped-change",
        .severity = .pass,
        .message = "PR is well-scoped",
        .affected_files = &[_][]const u8{},
        .suggestion = null,
    };
    return results;
}

fn isNonCodeFile(filename: []const u8) bool {
    for (NON_CODE_PREFIXES) |prefix| {
        if (std.mem.startsWith(u8, filename, prefix)) return true;
    }
    for (NON_CODE_EXTENSIONS) |ext| {
        if (std.mem.endsWith(u8, filename, ext)) return true;
    }
    return false;
}

fn resolveImport(import_path: []const u8, code_files: []const CodeFileEntry) ?usize {
    // Strip quotes (Go imports include them)
    var path = import_path;
    if (path.len >= 2 and (path[0] == '"' or path[0] == '\'')) {
        path = path[1 .. path.len - 1];
    }

    // Strip relative prefixes
    if (std.mem.startsWith(u8, path, "./")) {
        path = path[2..];
    } else if (std.mem.startsWith(u8, path, "../")) {
        // Can't resolve ../ without knowing the importing file's directory
        // Still try suffix matching
        path = path[3..];
    } else if (std.mem.startsWith(u8, path, "@/")) {
        path = path[2..];
    }

    // Convert Python dotted notation to path
    var path_buf: [512]u8 = undefined;
    var converted_path = path;
    if (std.mem.indexOf(u8, path, ".") != null and
        std.mem.indexOf(u8, path, "/") == null)
    {
        // Likely Python dotted import: foo.bar -> foo/bar
        var out_len: usize = 0;
        for (path) |ch| {
            if (out_len >= path_buf.len) break;
            path_buf[out_len] = if (ch == '.') '/' else ch;
            out_len += 1;
        }
        converted_path = path_buf[0..out_len];
    }

    // Match against changed file names (suffix match)
    var ext_buf: [512]u8 = undefined;
    // Copy converted_path into ext_buf so we can safely append extensions
    if (converted_path.len > ext_buf.len) return null;
    @memcpy(ext_buf[0..converted_path.len], converted_path);

    for (code_files, 0..) |entry, idx| {
        const fname = entry.file.filename;
        // Exact suffix match
        if (std.mem.endsWith(u8, fname, converted_path)) return idx;
        // Try with common extensions
        const extensions = [_][]const u8{ ".ts", ".tsx", ".js", ".jsx", ".py", ".go", "/index.ts", "/index.js" };
        for (extensions) |ext| {
            if (converted_path.len + ext.len > ext_buf.len) continue;
            @memcpy(ext_buf[converted_path.len .. converted_path.len + ext.len], ext);
            const with_ext = ext_buf[0 .. converted_path.len + ext.len];
            if (std.mem.endsWith(u8, fname, with_ext)) return idx;
        }
    }
    return null;
}
