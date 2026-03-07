/// GitHub API response type definitions. Data-only, append-only.

pub const PrFile = struct {
    filename: []const u8,
    status: []const u8,
    additions: u32,
    deletions: u32,
    changes: u32,
    patch: ?[]const u8 = null,
};

pub const PrMetadata = struct {
    number: u32,
    title: []const u8,
    body: ?[]const u8,
};
