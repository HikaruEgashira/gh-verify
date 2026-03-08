const std = @import("std");
const rule = @import("rule.zig");
const detect_unscoped_change = @import("detect_unscoped_change.zig");
const verify_release_integrity = @import("verify_release_integrity.zig");

const Rule = struct {
    id: []const u8,
    run: rule.RuleFn,
};

/// Rule registry. Add new rules here.
const rules = [_]Rule{
    .{ .id = "detect-unscoped-change", .run = detect_unscoped_change.run },
    .{ .id = "verify-release-integrity", .run = verify_release_integrity.run },
};

/// Run all registered rules and return aggregated results.
pub fn runAll(alloc: std.mem.Allocator, ctx: rule.RuleContext) ![]rule.RuleResult {
    var results: std.ArrayList(rule.RuleResult) = .empty;
    for (rules) |r| {
        const rule_results = try r.run(alloc, ctx);
        try results.appendSlice(alloc, rule_results);
    }
    return results.toOwnedSlice(alloc);
}

const rule_ids: [rules.len][]const u8 = blk: {
    var ids: [rules.len][]const u8 = undefined;
    for (rules, 0..) |r, i| {
        ids[i] = r.id;
    }
    break :blk ids;
};

/// Return IDs of all registered rules. Derived automatically from the rules array.
pub fn listRuleIds() []const []const u8 {
    return &rule_ids;
}
