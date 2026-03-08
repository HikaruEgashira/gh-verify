const std = @import("std");
const client = @import("client.zig");
const release_types = @import("release_types.zig");
const Config = @import("../main.zig").Config;

/// Fetch repository tags (reverse chronological).
/// GET /repos/{owner}/{repo}/tags
pub fn getTags(
    alloc: std.mem.Allocator,
    cfg: Config,
    owner: []const u8,
    repo: []const u8,
) ![]release_types.Tag {
    var all_tags: std.ArrayList(release_types.Tag) = .empty;

    var current_path: []const u8 = try std.fmt.allocPrint(
        alloc,
        "/repos/{s}/{s}/tags?per_page=100",
        .{ owner, repo },
    );

    const max_pages = 10;
    var page: usize = 0;

    while (page < max_pages) : (page += 1) {
        const res = try client.getWithLink(alloc, cfg, current_path, null);
        defer alloc.free(res.body);

        const parsed = try std.json.parseFromSlice(
            []release_types.Tag,
            alloc,
            res.body,
            .{ .ignore_unknown_fields = true },
        );
        defer parsed.deinit();

        for (parsed.value) |t| {
            try all_tags.append(alloc, .{
                .name = try alloc.dupe(u8, t.name),
                .commit = .{ .sha = try alloc.dupe(u8, t.commit.sha) },
            });
        }

        if (res.next_path) |next| {
            alloc.free(current_path);
            current_path = next;
        } else {
            break;
        }
    }

    alloc.free(current_path);
    return all_tags.toOwnedSlice(alloc);
}

/// Compare two refs and return commits between them.
/// GET /repos/{owner}/{repo}/compare/{base}...{head}
pub fn compareRefs(
    alloc: std.mem.Allocator,
    cfg: Config,
    owner: []const u8,
    repo: []const u8,
    base: []const u8,
    head: []const u8,
) ![]release_types.CompareCommit {
    const path = try std.fmt.allocPrint(
        alloc,
        "/repos/{s}/{s}/compare/{s}...{s}",
        .{ owner, repo, base, head },
    );
    defer alloc.free(path);

    const body = try client.get(alloc, cfg, path, null);
    defer alloc.free(body);

    const parsed = try std.json.parseFromSlice(
        release_types.CompareResponse,
        alloc,
        body,
        .{ .ignore_unknown_fields = true },
    );
    defer parsed.deinit();

    var commits: std.ArrayList(release_types.CompareCommit) = .empty;
    for (parsed.value.commits) |c| {
        try commits.append(alloc, .{
            .sha = try alloc.dupe(u8, c.sha),
            .commit = .{
                .message = try alloc.dupe(u8, c.commit.message),
                .verification = .{
                    .verified = c.commit.verification.verified,
                    .reason = try alloc.dupe(u8, c.commit.verification.reason),
                },
            },
            .author = if (c.author) |a| .{ .login = try alloc.dupe(u8, a.login) } else null,
        });
    }

    return commits.toOwnedSlice(alloc);
}

/// Fetch PRs associated with a commit.
/// GET /repos/{owner}/{repo}/commits/{sha}/pulls
pub fn getCommitPulls(
    alloc: std.mem.Allocator,
    cfg: Config,
    owner: []const u8,
    repo: []const u8,
    sha: []const u8,
) ![]release_types.PullRequestSummary {
    const path = try std.fmt.allocPrint(
        alloc,
        "/repos/{s}/{s}/commits/{s}/pulls",
        .{ owner, repo, sha },
    );
    defer alloc.free(path);

    const body = try client.get(alloc, cfg, path, "application/vnd.github.v3+json");
    defer alloc.free(body);

    const parsed = try std.json.parseFromSlice(
        []release_types.PullRequestSummary,
        alloc,
        body,
        .{ .ignore_unknown_fields = true },
    );
    defer parsed.deinit();

    var prs: std.ArrayList(release_types.PullRequestSummary) = .empty;
    for (parsed.value) |p| {
        try prs.append(alloc, .{
            .number = p.number,
            .state = try alloc.dupe(u8, p.state),
            .merged_at = if (p.merged_at) |m| try alloc.dupe(u8, m) else null,
            .user = .{ .login = try alloc.dupe(u8, p.user.login) },
        });
    }

    return prs.toOwnedSlice(alloc);
}

/// Fetch reviews for a PR.
/// GET /repos/{owner}/{repo}/pulls/{number}/reviews
pub fn getPrReviews(
    alloc: std.mem.Allocator,
    cfg: Config,
    owner: []const u8,
    repo: []const u8,
    pr_number: u32,
) ![]release_types.Review {
    const path = try std.fmt.allocPrint(
        alloc,
        "/repos/{s}/{s}/pulls/{d}/reviews",
        .{ owner, repo, pr_number },
    );
    defer alloc.free(path);

    const body = try client.get(alloc, cfg, path, null);
    defer alloc.free(body);

    const parsed = try std.json.parseFromSlice(
        []release_types.Review,
        alloc,
        body,
        .{ .ignore_unknown_fields = true },
    );
    defer parsed.deinit();

    var reviews: std.ArrayList(release_types.Review) = .empty;
    for (parsed.value) |r| {
        try reviews.append(alloc, .{
            .user = .{ .login = try alloc.dupe(u8, r.user.login) },
            .state = try alloc.dupe(u8, r.state),
        });
    }

    return reviews.toOwnedSlice(alloc);
}
