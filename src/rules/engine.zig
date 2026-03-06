const std = @import("std");
const rule = @import("rule.zig");
const detect_unscoped_change = @import("detect_unscoped_change.zig");

/// 全ルールの登録。新しいルール追加時はここに1行追加するだけ。
const rules = [_]rule.RuleFn{
    detect_unscoped_change.run,
    // 将来の追加例:
    // commit_message.run,
    // branch_protection.run,
};

/// 全登録ルールを実行し、結果を集約して返す。
pub fn runAll(alloc: std.mem.Allocator, ctx: rule.RuleContext) ![]rule.RuleResult {
    var results: std.ArrayList(rule.RuleResult) = .empty;
    for (rules) |run_fn| {
        const rule_results = try run_fn(alloc, ctx);
        try results.appendSlice(alloc, rule_results);
    }
    return results.toOwnedSlice(alloc);
}

/// 登録されている全ルールの ID リストを返す。
pub fn listRuleIds() []const []const u8 {
    return &[_][]const u8{
        "detect-unscoped-change",
    };
}
