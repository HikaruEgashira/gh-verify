const std = @import("std");
const Config = @import("../main.zig").Config;

/// GitHub REST API への GET リクエスト。
/// レスポンスボディを caller-owned スライスで返す（JSON 解析はしない）。
pub fn get(alloc: std.mem.Allocator, cfg: Config, path: []const u8, accept: ?[]const u8) ![]const u8 {
    var client = std.http.Client{ .allocator = alloc };
    defer client.deinit();

    const url_str = try std.fmt.allocPrint(alloc, "https://{s}{s}", .{ cfg.host, path });
    defer alloc.free(url_str);

    const uri = try std.Uri.parse(url_str);

    const auth_header = try std.fmt.allocPrint(alloc, "Bearer {s}", .{cfg.token});
    defer alloc.free(auth_header);

    const accept_header = accept orelse "application/vnd.github.v3+json";

    var req = try client.request(.GET, uri, .{
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
        return error.HttpError;
    }

    var transfer_buf: [8192]u8 = undefined;
    const reader = response.reader(&transfer_buf);

    var body: std.ArrayList(u8) = .empty;
    try reader.appendRemainingUnlimited(alloc, &body);
    return body.toOwnedSlice(alloc);
}
