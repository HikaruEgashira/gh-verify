const std = @import("std");
const types = @import("../github/types.zig");
const release_types = @import("../github/release_types.zig");

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

/// Per-commit PR association for release context.
pub const CommitPrAssociation = struct {
    commit_sha: []const u8,
    prs: []const release_types.PullRequestSummary,
};

/// Per-PR review set for release context.
pub const PrReviewSet = struct {
    pr_number: u32,
    pr_author: []const u8,
    reviews: []const release_types.Review,
};

pub const ReleaseContext = struct {
    base_tag: []const u8,
    head_tag: []const u8,
    commits: []const release_types.CompareCommit,
    commit_prs: []const CommitPrAssociation,
    pr_reviews: []const PrReviewSet,
};

pub const ContextPayload = union(enum) {
    pr: struct {
        pr_files: []const PrFile,
        pr_metadata: PrMetadata,
    },
    release: ReleaseContext,
};

pub const RuleContext = struct {
    payload: ContextPayload,
};

/// Signature that all rule functions must implement
pub const RuleFn = *const fn (std.mem.Allocator, RuleContext) anyerror![]RuleResult;
