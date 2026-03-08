const std = @import("std");
const Config = @import("../main.zig").Config;
const release_api = @import("../github/release_api.zig");
const engine = @import("../rules/engine.zig");
const rule = @import("../rules/rule.zig");
const formatter = @import("../output/formatter.zig");

/// Entry point for the `gh lint release` subcommand.
pub fn run(alloc: std.mem.Allocator, cfg: Config, args: []const []const u8) !void {
    const stderr = std.fs.File.stderr().deprecatedWriter();

    if (args.len == 0) {
        try stderr.print(
            \\Usage: gh lint release <tag> [--repo OWNER/REPO] [--format human|json]
            \\       gh lint release <base>..<head> [--repo OWNER/REPO] [--format human|json]
            \\
        , .{});
        std.process.exit(1);
    }

    // Parse tag argument: single tag or base..head range
    var base_tag: []const u8 = undefined;
    var head_tag: []const u8 = undefined;
    var auto_detect_base = false;

    if (std.mem.indexOf(u8, args[0], "..")) |sep_idx| {
        base_tag = args[0][0..sep_idx];
        head_tag = args[0][sep_idx + 2 ..];
    } else {
        head_tag = args[0];
        auto_detect_base = true;
    }

    // Parse flags
    var format: formatter.Format = .human;
    var repo_override: ?[]const u8 = null;
    var i: usize = 1;
    while (i < args.len) : (i += 1) {
        if (std.mem.eql(u8, args[i], "--format") and i + 1 < args.len) {
            i += 1;
            format = formatter.parseFormat(args[i]) catch {
                try stderr.print("Invalid format: {s} (use 'human' or 'json')\n", .{args[i]});
                std.process.exit(1);
            };
        } else if (std.mem.eql(u8, args[i], "--repo") and i + 1 < args.len) {
            i += 1;
            repo_override = args[i];
        }
    }

    // Resolve OWNER/REPO
    const repo_str = repo_override orelse cfg.repo;
    const slash_idx = std.mem.indexOf(u8, repo_str, "/") orelse {
        try stderr.print("Could not resolve repo. Use --repo OWNER/REPO or set GH_REPO env var.\n", .{});
        std.process.exit(1);
    };
    const owner = repo_str[0..slash_idx];
    const repo_name = repo_str[slash_idx + 1 ..];

    // Auto-detect previous tag if needed
    if (auto_detect_base) {
        const tags = release_api.getTags(alloc, cfg, owner, repo_name) catch |err| {
            try stderr.print("Failed to fetch tags: {}\n", .{err});
            std.process.exit(1);
        };

        var found_head = false;
        for (tags, 0..) |t, idx| {
            if (std.mem.eql(u8, t.name, head_tag)) {
                found_head = true;
                if (idx + 1 < tags.len) {
                    base_tag = tags[idx + 1].name;
                } else {
                    try stderr.print("No previous tag found before {s}\n", .{head_tag});
                    std.process.exit(1);
                }
                break;
            }
        }
        if (!found_head) {
            try stderr.print("Tag not found: {s}\n", .{head_tag});
            std.process.exit(1);
        }
    }

    const stdout = std.fs.File.stdout().deprecatedWriter();
    try stdout.print("Checking release: {s}..{s}\n", .{ base_tag, head_tag });

    // Fetch commits between tags
    const commits = release_api.compareRefs(alloc, cfg, owner, repo_name, base_tag, head_tag) catch |err| {
        try stderr.print("Failed to compare refs: {}\n", .{err});
        std.process.exit(1);
    };

    if (commits.len == 0) {
        try stdout.print("No commits found between {s} and {s}\n", .{ base_tag, head_tag });
        return;
    }

    try stdout.print("Found {d} commits\n", .{commits.len});

    // Fetch PR associations for each commit
    var commit_prs: std.ArrayList(rule.CommitPrAssociation) = .empty;
    var seen_prs = std.AutoHashMap(u32, void).init(alloc);

    for (commits) |c| {
        const prs = release_api.getCommitPulls(alloc, cfg, owner, repo_name, c.sha) catch |err| {
            try stderr.print("Warning: failed to fetch PRs for commit {s}: {}\n", .{ c.sha[0..@min(c.sha.len, 7)], err });
            try commit_prs.append(alloc, .{
                .commit_sha = c.sha,
                .prs = &[_]@import("../github/release_types.zig").PullRequestSummary{},
            });
            continue;
        };
        try commit_prs.append(alloc, .{ .commit_sha = c.sha, .prs = prs });

        for (prs) |pr| {
            try seen_prs.put(pr.number, {});
        }
    }

    // Fetch reviews for each unique PR
    var pr_reviews: std.ArrayList(rule.PrReviewSet) = .empty;
    var pr_it = seen_prs.keyIterator();
    while (pr_it.next()) |pr_number_ptr| {
        const pr_number = pr_number_ptr.*;
        // Find PR author from commit_prs
        var pr_author: []const u8 = "unknown";
        outer: for (commit_prs.items) |assoc| {
            for (assoc.prs) |pr| {
                if (pr.number == pr_number) {
                    pr_author = pr.user.login;
                    break :outer;
                }
            }
        }

        const reviews = release_api.getPrReviews(alloc, cfg, owner, repo_name, pr_number) catch |err| {
            try stderr.print("Warning: failed to fetch reviews for PR #{d}: {}\n", .{ pr_number, err });
            continue;
        };

        try pr_reviews.append(alloc, .{
            .pr_number = pr_number,
            .pr_author = pr_author,
            .reviews = reviews,
        });
    }

    // Build release context and run rules
    const ctx = rule.RuleContext{
        .payload = .{ .release = .{
            .base_tag = base_tag,
            .head_tag = head_tag,
            .commits = commits,
            .commit_prs = try commit_prs.toOwnedSlice(alloc),
            .pr_reviews = try pr_reviews.toOwnedSlice(alloc),
        } },
    };

    const results = try engine.runAll(alloc, ctx);

    try formatter.print(alloc, format, results);

    for (results) |r| {
        if (r.severity == .@"error") std.process.exit(1);
    }
}
