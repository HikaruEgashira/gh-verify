const std = @import("std");
const rule = @import("rule.zig");
const detect_unscoped_change = @import("detect_unscoped_change.zig");

const Rule = struct {
    id: []const u8,
    run: rule.RuleFn,
};

/// 全ルールの登録。新しいルール追加時はここに1行追加するだけ。
const rules = [_]Rule{
    .{ .id = "detect-unscoped-change", .run = detect_unscoped_change.run },
    // 将来の追加例:
    // .{ .id = "commit-message", .run = commit_message.run },
};

/// 全登録ルールを実行し、結果を集約して返す。
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

/// 登録されている全ルールの ID リストを返す。rules 配列から自動導出する。
pub fn listRuleIds() []const []const u8 {
    return &rule_ids;
}
