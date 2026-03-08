const std = @import("std");
const rule = @import("rule.zig");

const rule_id = "verify-release-integrity";

pub fn run(alloc: std.mem.Allocator, ctx: rule.RuleContext) ![]rule.RuleResult {
    const rel = switch (ctx.payload) {
        .release => |r| r,
        .pr => return &[_]rule.RuleResult{},
    };

    var results: std.ArrayList(rule.RuleResult) = .empty;

    try checkCommitSignatures(alloc, rel, &results);
    try checkMutualApproval(alloc, rel, &results);
    try checkPrCoverage(alloc, rel, &results);

    // If no issues found, emit a single pass result
    if (results.items.len == 0) {
        try results.append(alloc, .{
            .rule_id = rule_id,
            .severity = .pass,
            .message = "All release integrity checks passed",
            .affected_files = &[_][]const u8{},
            .suggestion = null,
        });
    }

    return results.toOwnedSlice(alloc);
}

/// Check that all commits in the range are signed (verified).
fn checkCommitSignatures(
    alloc: std.mem.Allocator,
    rel: rule.ReleaseContext,
    results: *std.ArrayList(rule.RuleResult),
) !void {
    var unsigned: std.ArrayList([]const u8) = .empty;

    for (rel.commits) |c| {
        if (!c.commit.verification.verified) {
            // Use first 7 chars of SHA as identifier
            const short_sha = if (c.sha.len >= 7) c.sha[0..7] else c.sha;
            try unsigned.append(alloc, try alloc.dupe(u8, short_sha));
        }
    }

    if (unsigned.items.len > 0) {
        const message = try std.fmt.allocPrint(
            alloc,
            "{d} of {d} commits are unsigned",
            .{ unsigned.items.len, rel.commits.len },
        );

        var detail_buf: std.ArrayList(u8) = .empty;
        const writer = detail_buf.writer(alloc);
        try writer.print("Unsigned commits:\n", .{});
        for (unsigned.items) |sha| {
            try writer.print("  {s}\n", .{sha});
        }
        try writer.print("Enable commit signing: git config commit.gpgsign true", .{});

        try results.append(alloc, .{
            .rule_id = rule_id,
            .severity = .@"error",
            .message = message,
            .affected_files = try unsigned.toOwnedSlice(alloc),
            .suggestion = try detail_buf.toOwnedSlice(alloc),
        });
    }
}

/// Check that commit author and PR approver are different people.
fn checkMutualApproval(
    alloc: std.mem.Allocator,
    rel: rule.ReleaseContext,
    results: *std.ArrayList(rule.RuleResult),
) !void {
    var violations: std.ArrayList([]const u8) = .empty;
    var detail_buf: std.ArrayList(u8) = .empty;
    const writer = detail_buf.writer(alloc);

    for (rel.pr_reviews) |pr_rev| {
        // Collect commit authors for this PR
        var commit_authors = std.StringHashMap(void).init(alloc);
        for (rel.commit_prs) |assoc| {
            for (assoc.prs) |pr| {
                if (pr.number == pr_rev.pr_number) {
                    // Find the commit's author
                    for (rel.commits) |c| {
                        if (std.mem.eql(u8, c.sha, assoc.commit_sha)) {
                            if (c.author) |a| {
                                try commit_authors.put(a.login, {});
                            }
                        }
                    }
                }
            }
        }

        // Check if any approver is also a commit author
        var has_independent_approval = false;
        for (pr_rev.reviews) |review| {
            if (!std.mem.eql(u8, review.state, "APPROVED")) continue;
            if (!commit_authors.contains(review.user.login)) {
                has_independent_approval = true;
                break;
            }
        }

        // Also check: PR author != sole approver (fallback for squash merges)
        if (!has_independent_approval) {
            for (pr_rev.reviews) |review| {
                if (!std.mem.eql(u8, review.state, "APPROVED")) continue;
                if (!std.mem.eql(u8, review.user.login, pr_rev.pr_author)) {
                    has_independent_approval = true;
                    break;
                }
            }
        }

        if (!has_independent_approval) {
            const pr_label = try std.fmt.allocPrint(alloc, "PR #{d}", .{pr_rev.pr_number});
            try violations.append(alloc, pr_label);
            try writer.print("  PR #{d}: author={s}, no independent approver\n", .{
                pr_rev.pr_number,
                pr_rev.pr_author,
            });
        }
    }

    if (violations.items.len > 0) {
        const message = try std.fmt.allocPrint(
            alloc,
            "{d} PRs lack independent approval (commit author != approver)",
            .{violations.items.len},
        );

        try results.append(alloc, .{
            .rule_id = rule_id,
            .severity = .@"error",
            .message = message,
            .affected_files = try violations.toOwnedSlice(alloc),
            .suggestion = try detail_buf.toOwnedSlice(alloc),
        });
    }
}

/// Check that all non-merge commits are associated with a PR.
fn checkPrCoverage(
    alloc: std.mem.Allocator,
    rel: rule.ReleaseContext,
    results: *std.ArrayList(rule.RuleResult),
) !void {
    var uncovered: std.ArrayList([]const u8) = .empty;

    for (rel.commit_prs) |assoc| {
        // Skip merge commits
        for (rel.commits) |c| {
            if (std.mem.eql(u8, c.sha, assoc.commit_sha)) {
                if (std.mem.startsWith(u8, c.commit.message, "Merge ")) break;
                if (assoc.prs.len == 0) {
                    const short_sha = if (c.sha.len >= 7) c.sha[0..7] else c.sha;
                    try uncovered.append(alloc, try alloc.dupe(u8, short_sha));
                }
                break;
            }
        }
    }

    if (uncovered.items.len > 0) {
        const message = try std.fmt.allocPrint(
            alloc,
            "{d} commits have no associated PR (direct pushes)",
            .{uncovered.items.len},
        );

        try results.append(alloc, .{
            .rule_id = rule_id,
            .severity = .warning,
            .message = message,
            .affected_files = try uncovered.toOwnedSlice(alloc),
            .suggestion = "All changes should go through pull requests for proper review.",
        });
    }
}
