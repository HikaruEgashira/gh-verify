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
    const path = try std.fmt.allocPrint(
        alloc,
        "/repos/{s}/{s}/pulls/{d}/files?per_page=100",
        .{ owner, repo, pr_number },
    );
    defer alloc.free(path);

    const body = try client.get(alloc, cfg, path, null);
    defer alloc.free(body);

    const parsed = try std.json.parseFromSlice(
        []PrFile,
        alloc,
        body,
        .{ .ignore_unknown_fields = true },
    );
    defer parsed.deinit();

    // ArenaAllocator を使わないため、深コピーして返す
    var result = try alloc.alloc(PrFile, parsed.value.len);
    for (parsed.value, 0..) |f, i| {
        result[i] = PrFile{
            .filename = try alloc.dupe(u8, f.filename),
            .status = try alloc.dupe(u8, f.status),
            .additions = f.additions,
            .deletions = f.deletions,
            .changes = f.changes,
        };
    }
    return result;
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
