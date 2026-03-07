const std = @import("std");
const rule = @import("../rules/rule.zig");

const RESET = "\x1b[0m";
const RED = "\x1b[31m";
const YELLOW = "\x1b[33m";
const GREEN = "\x1b[32m";
const BOLD = "\x1b[1m";

/// RuleResult のリストを人間が読みやすい形式で stdout に出力する。
pub fn print(results: []const rule.RuleResult) !void {
    const stdout = std.fs.File.stdout().deprecatedWriter();

    for (results) |r| {
        switch (r.severity) {
            .pass => {
                try stdout.print("{s}[{s}]{s} {s}pass{s}: {s}\n", .{
                    BOLD, r.rule_id, RESET, GREEN, RESET, r.message,
                });
            },
            .warning => {
                try stdout.print("{s}[{s}]{s} {s}warning{s}: {s}\n", .{
                    BOLD, r.rule_id, RESET, YELLOW, RESET, r.message,
                });
                if (r.suggestion) |s| {
                    try stdout.print("{s}", .{s});
                }
                try stdout.print("  Suggestion: Consider splitting into separate PRs by domain.\n", .{});
            },
            .@"error" => {
                try stdout.print("{s}[{s}]{s} {s}error{s}: {s}\n", .{
                    BOLD, r.rule_id, RESET, RED, RESET, r.message,
                });
                if (r.suggestion) |s| {
                    try stdout.print("{s}", .{s});
                }
                try stdout.print("  Suggestion: Consider splitting into separate PRs by domain.\n", .{});
            },
        }
    }

}
