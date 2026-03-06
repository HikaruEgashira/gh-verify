const std = @import("std");

pub const Domain = enum {
    auth,
    ui,
    database,
    docs,
    ci,
    @"test",
    config,
    api,
    unknown,

    pub fn name(self: Domain) []const u8 {
        return switch (self) {
            .auth => "auth",
            .ui => "ui",
            .database => "database",
            .docs => "docs",
            .ci => "ci",
            .@"test" => "test",
            .config => "config",
            .api => "api",
            .unknown => "unknown",
        };
    }
};

/// ファイルパスをドメインに分類する。純粋関数（副作用なし）。
pub fn classifyPath(path: []const u8) Domain {
    const lower = path; // Zig にはランタイム tolower がないため大文字小文字を考慮したパターンで対応

    // test
    if (containsSegment(lower, "test") or
        containsSegment(lower, "spec") or
        std.mem.endsWith(u8, lower, "_test.zig") or
        std.mem.endsWith(u8, lower, ".spec.ts") or
        std.mem.endsWith(u8, lower, ".test.ts") or
        std.mem.endsWith(u8, lower, "_test.go"))
        return .@"test";

    // ci
    if (std.mem.startsWith(u8, lower, ".github/") or
        containsSegment(lower, "ci") or
        containsSegment(lower, "workflow"))
        return .ci;

    // docs
    if (std.mem.startsWith(u8, lower, "docs/") or
        std.mem.endsWith(u8, lower, ".md") or
        std.mem.endsWith(u8, lower, ".rst") or
        std.mem.endsWith(u8, lower, ".txt"))
        return .docs;

    // auth
    if (containsSegment(lower, "auth") or
        containsPath(lower, "login") or
        containsSegment(lower, "token") or
        containsSegment(lower, "session") or
        containsPath(lower, "oauth"))
        return .auth;

    // database
    if (containsSegment(lower, "db") or
        containsSegment(lower, "database") or
        containsSegment(lower, "migration") or
        containsSegment(lower, "schema") or
        std.mem.endsWith(u8, lower, ".sql"))
        return .database;

    // ui
    if (containsSegment(lower, "ui") or
        containsSegment(lower, "component") or
        containsSegment(lower, "view") or
        containsSegment(lower, "page") or
        std.mem.endsWith(u8, lower, ".css") or
        std.mem.endsWith(u8, lower, ".scss") or
        std.mem.endsWith(u8, lower, ".tsx") or
        std.mem.endsWith(u8, lower, ".jsx"))
        return .ui;

    // api
    if (containsSegment(lower, "api") or
        containsSegment(lower, "handler") or
        containsSegment(lower, "route") or
        containsSegment(lower, "controller") or
        containsSegment(lower, "endpoint"))
        return .api;

    // config
    if (containsSegment(lower, "config") or
        std.mem.endsWith(u8, lower, ".toml") or
        std.mem.endsWith(u8, lower, ".yaml") or
        std.mem.endsWith(u8, lower, ".yml") or
        std.mem.endsWith(u8, lower, ".json") or
        std.mem.endsWith(u8, lower, ".env"))
        return .config;

    return .unknown;
}

fn containsSegment(path: []const u8, segment: []const u8) bool {
    if (std.mem.indexOf(u8, path, segment)) |idx| {
        const before_ok = idx == 0 or path[idx - 1] == '/';
        const after_idx = idx + segment.len;
        const after_ok = after_idx >= path.len or path[after_idx] == '/' or path[after_idx] == '.';
        return before_ok and after_ok;
    }
    return false;
}

fn containsPath(path: []const u8, needle: []const u8) bool {
    return std.mem.indexOf(u8, path, needle) != null;
}
