const std = @import("std");
const types = @import("../github/types.zig");

pub const PrFile = types.PrFile;
pub const PrMetadata = types.PrMetadata;

pub const Severity = enum {
    pass,
    warning,
    @"error",
};

pub const RuleResult = struct {
    rule_id: []const u8,
    severity: Severity,
    message: []const u8,
    affected_files: [][]const u8,
    suggestion: ?[]const u8,
};

pub const RuleContext = struct {
    pr_files: []const PrFile,
    pr_metadata: PrMetadata,
};

/// 全ルール関数が実装するシグネチャ
pub const RuleFn = *const fn (std.mem.Allocator, RuleContext) anyerror![]RuleResult;
