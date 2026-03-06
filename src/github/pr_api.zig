const std = @import("std");
const client = @import("client.zig");
const types = @import("types.zig");
const Config = @import("../main.zig").Config;

pub const PrFile = types.PrFile;
pub const PrMetadata = types.PrMetadata;

/// PR の変更ファイル一覧を取得する。
/// GET /repos/{owner}/{repo}/pulls/{pr_number}/files
pub fn getPrFiles(
    alloc: std.mem.Allocator,
    cfg: Config,
    owner: []const u8,
    repo: []const u8,
    pr_number: u32,
) ![]PrFile {
    var all_files: std.ArrayList(PrFile) = .empty;

    var current_path: []const u8 = try std.fmt.allocPrint(
        alloc,
        "/repos/{s}/{s}/pulls/{d}/files?per_page=100",
        .{ owner, repo, pr_number },
    );

    const max_pages = 10;
    var page: usize = 0;

    while (page < max_pages) : (page += 1) {
        const res = try client.getWithLink(alloc, cfg, current_path, null);
        defer alloc.free(res.body);

        const parsed = try std.json.parseFromSlice(
            []PrFile,
            alloc,
            res.body,
            .{ .ignore_unknown_fields = true },
        );
        defer parsed.deinit();

        for (parsed.value) |f| {
            try all_files.append(alloc, PrFile{
                .filename = try alloc.dupe(u8, f.filename),
                .status = try alloc.dupe(u8, f.status),
                .additions = f.additions,
                .deletions = f.deletions,
                .changes = f.changes,
                .patch = if (f.patch) |p| try alloc.dupe(u8, p) else null,
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
    return all_files.toOwnedSlice(alloc);
}

/// PR のメタデータを取得する。
/// GET /repos/{owner}/{repo}/pulls/{pr_number}
pub fn getPrMetadata(
    alloc: std.mem.Allocator,
    cfg: Config,
    owner: []const u8,
    repo: []const u8,
    pr_number: u32,
) !PrMetadata {
    const path = try std.fmt.allocPrint(
        alloc,
        "/repos/{s}/{s}/pulls/{d}",
        .{ owner, repo, pr_number },
    );
    defer alloc.free(path);

    const body = try client.get(alloc, cfg, path, null);
    defer alloc.free(body);

    const RawPr = struct {
        number: u32,
        title: []const u8,
        body: ?[]const u8,
    };

    const parsed = try std.json.parseFromSlice(
        RawPr,
        alloc,
        body,
        .{ .ignore_unknown_fields = true },
    );
    defer parsed.deinit();

    return PrMetadata{
        .number = parsed.value.number,
        .title = try alloc.dupe(u8, parsed.value.title),
        .body = if (parsed.value.body) |b| try alloc.dupe(u8, b) else null,
    };
}
