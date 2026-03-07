const std = @import("std");
const rule = @import("../rules/rule.zig");
const human = @import("human.zig");
const json = @import("json.zig");

pub const Format = enum { human, json };

/// Dispatch output formatting based on the --format flag.
/// Add new formats here.
pub fn print(alloc: std.mem.Allocator, format: Format, results: []const rule.RuleResult) !void {
    switch (format) {
        .human => try human.print(results),
        .json => try json.print(alloc, results),
    }
}

pub fn parseFormat(s: []const u8) !Format {
    if (std.mem.eql(u8, s, "human")) return .human;
    if (std.mem.eql(u8, s, "json")) return .json;
    return error.InvalidFormat;
}
