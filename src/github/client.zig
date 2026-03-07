const std = @import("std");
const Config = @import("../main.zig").Config;

const max_body_size = 10 * 1024 * 1024; // 10MB

const RawResponse = struct {
    body: []const u8,
    link_header: ?[]const u8,
};

/// HTTP GET を実行してボディと Link ヘッダを返す内部関数。
fn doGet(alloc: std.mem.Allocator, cfg: Config, path: []const u8, accept: ?[]const u8) !RawResponse {
    var http_client = std.http.Client{ .allocator = alloc };
    defer http_client.deinit();

    const url_str = try std.fmt.allocPrint(alloc, "https://{s}{s}", .{ cfg.host, path });
    defer alloc.free(url_str);

    const uri = try std.Uri.parse(url_str);

    const auth_header = try std.fmt.allocPrint(alloc, "Bearer {s}", .{cfg.token});
    defer alloc.free(auth_header);

    const accept_header = accept orelse "application/vnd.github.v3+json";

    var req = try http_client.request(.GET, uri, .{
        .headers = .{ .accept_encoding = .{ .override = "identity" } },
        .extra_headers = &[_]std.http.Header{
            .{ .name = "Authorization", .value = auth_header },
            .{ .name = "Accept", .value = accept_header },
            .{ .name = "X-GitHub-Api-Version", .value = "2022-11-28" },
        },
    });
    defer req.deinit();

    try req.sendBodiless();

    var redirect_buf: [4096]u8 = undefined;
    var response = try req.receiveHead(&redirect_buf);

    if (response.head.status != .ok) {
        return switch (response.head.status) {
            .unauthorized => error.Unauthorized,
            .forbidden => error.Forbidden,
            .not_found => error.NotFound,
            .too_many_requests => error.RateLimited,
            else => error.HttpError,
        };
    }

    var link_header: ?[]const u8 = null;
    var header_it = response.head.iterateHeaders();
    while (header_it.next()) |header| {
        if (std.ascii.eqlIgnoreCase(header.name, "link")) {
            link_header = try alloc.dupe(u8, header.value);
            break;
        }
    }

    var transfer_buf: [8192]u8 = undefined;
    const reader = response.reader(&transfer_buf);

    var body: std.ArrayList(u8) = .empty;
    try reader.appendRemainingUnlimited(alloc, &body);

    if (body.items.len > max_body_size) {
        return error.ResponseTooLarge;
    }

    return RawResponse{
        .body = try body.toOwnedSlice(alloc),
        .link_header = link_header,
    };
}

/// GitHub REST API への GET リクエスト。
/// レスポンスボディを caller-owned スライスで返す（JSON 解析はしない）。
pub fn get(alloc: std.mem.Allocator, cfg: Config, path: []const u8, accept: ?[]const u8) ![]const u8 {
    const raw = try doGet(alloc, cfg, path, accept);
    if (raw.link_header) |h| alloc.free(h);
    return raw.body;
}

pub const GetResult = struct {
    body: []const u8,
    next_path: ?[]const u8,
};

/// GitHub REST API への GET リクエスト（ページネーション対応）。
/// Link ヘッダから next URL を抽出して返す。
pub fn getWithLink(alloc: std.mem.Allocator, cfg: Config, path: []const u8, accept: ?[]const u8) !GetResult {
    const raw = try doGet(alloc, cfg, path, accept);

    var next_path: ?[]const u8 = null;
    if (raw.link_header) |link| {
        defer alloc.free(link);
        const base_prefix = try std.fmt.allocPrint(alloc, "https://{s}", .{cfg.host});
        defer alloc.free(base_prefix);
        next_path = parseLinkNext(alloc, link, base_prefix) catch null;
    }

    return GetResult{
        .body = raw.body,
        .next_path = next_path,
    };
}

/// Link ヘッダから rel="next" の URL パス部分を抽出する。
fn parseLinkNext(alloc: std.mem.Allocator, link_header: []const u8, base_prefix: []const u8) !?[]const u8 {
    // Format: <URL>; rel="next", <URL>; rel="last"
    var rest = link_header;
    while (rest.len > 0) {
        const lt = std.mem.indexOf(u8, rest, "<") orelse break;
        const gt = std.mem.indexOf(u8, rest[lt..], ">") orelse break;
        const url = rest[lt + 1 .. lt + gt];
        const after = rest[lt + gt + 1 ..];

        if (std.mem.indexOf(u8, after[0..@min(after.len, 20)], "rel=\"next\"") != null) {
            // Strip base prefix to get path
            if (std.mem.startsWith(u8, url, base_prefix)) {
                return try alloc.dupe(u8, url[base_prefix.len..]);
            }
            return try alloc.dupe(u8, url);
        }
        rest = after;
    }
    return null;
}
