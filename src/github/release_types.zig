/// GitHub API response types for release verification. Data-only.

pub const Tag = struct {
    name: []const u8,
    commit: struct { sha: []const u8 },
};

pub const CommitVerification = struct {
    verified: bool,
    reason: []const u8,
};

pub const CommitAuthor = struct {
    login: []const u8,
};

pub const CompareCommitInner = struct {
    message: []const u8,
    verification: CommitVerification,
};

pub const CompareCommit = struct {
    sha: []const u8,
    commit: CompareCommitInner,
    author: ?CommitAuthor = null,
};

pub const CompareResponse = struct {
    commits: []CompareCommit,
    total_commits: u32,
};

pub const PullRequestSummary = struct {
    number: u32,
    state: []const u8,
    merged_at: ?[]const u8 = null,
    user: struct { login: []const u8 },
};

pub const Review = struct {
    user: struct { login: []const u8 },
    state: []const u8,
};
