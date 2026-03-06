/// GitHub API レスポンス型定義。ロジックなし、追加専用。

pub const PrFile = struct {
    filename: []const u8,
    status: []const u8,
    additions: u32,
    deletions: u32,
    changes: u32,
};

pub const PrMetadata = struct {
    number: u32,
    title: []const u8,
    body: ?[]const u8,
};
