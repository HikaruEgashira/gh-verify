const std = @import("std");
const rule = @import("../rules/rule.zig");

const JsonResult = struct {
    rule_id: []const u8,
    severity: []const u8,
    message: []const u8,
    affected_files: [][]const u8,
    suggestion: ?[]const u8,
};

/// Print a list of RuleResults to stdout in JSON format.
pub fn print(alloc: std.mem.Allocator, results: []const rule.RuleResult) !void {
    var json_results = try alloc.alloc(JsonResult, results.len);
    for (results, 0..) |r, i| {
        json_results[i] = JsonResult{
            .rule_id = r.rule_id,
            .severity = switch (r.severity) {
                .pass => "pass",
                .warning => "warning",
                .@"error" => "error",
            },
            .message = r.message,
            .affected_files = r.affected_files,
            .suggestion = r.suggestion,
        };
    }

    const json_str = try std.json.Stringify.valueAlloc(alloc, json_results, .{ .whitespace = .indent_2 });
    defer alloc.free(json_str);
    const stdout = std.fs.File.stdout();
    try stdout.writeAll(json_str);
    try stdout.writeAll("\n");
}
