const std = @import("std");
const Domain = @import("diff_parser.zig").Domain;

const c = @cImport({
    @cInclude("tree_sitter/api.h");
    @cInclude("tree_sitter/tree-sitter-go.h");
    @cInclude("tree_sitter/tree-sitter-python.h");
    @cInclude("tree_sitter/tree-sitter-typescript.h");
});

pub const SemanticHints = struct {
    domain_boost: ?Domain = null,
    domain_suppress: ?Domain = null,
    confidence: f32 = 0,
};

// --- Cache: reuse parser and compiled queries across calls ---

var cached_parser: ?*c.TSParser = null;
var cached_queries: [3]?*c.TSQuery = .{ null, null, null };

fn getParser() ?*c.TSParser {
    if (cached_parser) |p| return p;
    const p = c.ts_parser_new() orelse return null;
    cached_parser = p;
    return p;
}

fn getCachedQuery(lang: Language) ?*c.TSQuery {
    const idx = @intFromEnum(lang);
    if (cached_queries[idx]) |q| return q;
    const grammar = selectGrammar(lang) orelse return null;
    const src = selectQuery(lang);
    var err_offset: u32 = 0;
    var err_type: c.TSQueryError = c.TSQueryErrorNone;
    const q = c.ts_query_new(grammar, src.ptr, @intCast(src.len), &err_offset, &err_type) orelse return null;
    cached_queries[idx] = q;
    return q;
}

/// Extract semantic signals from patch content via Tree-sitter AST analysis.
/// `patch` is a unified diff string (added lines with '+' prefix + context lines).
pub fn analyzeSemantics(filename: []const u8, patch: []const u8) SemanticHints {
    const lang = detectLanguage(filename) orelse return .{};

    var buf: [65536]u8 = undefined;
    const source = extractSource(patch, &buf);
    if (source.len == 0) return .{};

    const parser = getParser() orelse return .{};
    const grammar = selectGrammar(lang) orelse return .{};
    _ = c.ts_parser_set_language(parser, grammar);

    const tree = c.ts_parser_parse_string(parser, null, source.ptr, @intCast(source.len)) orelse return .{};
    defer c.ts_tree_delete(tree);

    const root = c.ts_tree_root_node(tree);

    const query = getCachedQuery(lang) orelse return .{};

    const cursor = c.ts_query_cursor_new() orelse return .{};
    defer c.ts_query_cursor_delete(cursor);

    c.ts_query_cursor_exec(cursor, query, root);

    var signals = ImportSignals{};
    var match: c.TSQueryMatch = undefined;
    while (c.ts_query_cursor_next_match(cursor, &match)) {
        for (0..match.capture_count) |i| {
            const cap = match.captures[i];
            const start = c.ts_node_start_byte(cap.node);
            const end = c.ts_node_end_byte(cap.node);
            if (end > start and end <= source.len) {
                classifyImport(lang, source[start..end], &signals);
            }
        }
    }

    return hintsFromSignals(signals);
}

// --- Internal types ---

const Language = enum { typescript, python, go };

const ImportSignals = struct {
    has_react: bool = false,
    has_vue: bool = false,
    has_svelte: bool = false,
    has_validation: bool = false, // zod/yup/joi (TS), pydantic (Python)
    has_prisma: bool = false,
    has_sqlalchemy: bool = false,
    has_sql_db: bool = false,
    has_web_framework: bool = false, // express/fastapi/flask/django/net.http
    has_net_http: bool = false,
    has_pytest: bool = false,
    has_testing: bool = false,
    has_jwt: bool = false,
    has_passport: bool = false,
    has_nextauth: bool = false,
};

// --- Language detection ---

fn detectLanguage(filename: []const u8) ?Language {
    if (std.mem.endsWith(u8, filename, ".ts") or
        std.mem.endsWith(u8, filename, ".tsx") or
        std.mem.endsWith(u8, filename, ".js") or
        std.mem.endsWith(u8, filename, ".jsx")) return .typescript;
    if (std.mem.endsWith(u8, filename, ".py")) return .python;
    if (std.mem.endsWith(u8, filename, ".go")) return .go;
    return null;
}

// --- Grammar selection ---

fn selectGrammar(lang: Language) ?*const c.TSLanguage {
    return switch (lang) {
        .typescript => c.tree_sitter_typescript(),
        .python => c.tree_sitter_python(),
        .go => c.tree_sitter_go(),
    };
}

// --- Per-language Tree-sitter queries ---
// Capture module name (string literal) from import statements as @source.

fn selectQuery(lang: Language) []const u8 {
    return switch (lang) {
        // import ... from "module" / require("module")
        .typescript =>
        \\(import_statement source: (string (string_fragment) @source))
        \\(call_expression function: (identifier) @_req (#eq? @_req "require") arguments: (arguments (string (string_fragment) @source)))
        ,
        // from module import ... / import module
        .python =>
        \\(import_from_statement module_name: (dotted_name) @source)
        \\(import_statement name: (dotted_name) @source)
        \\(import_statement name: (aliased_import alias: (identifier) name: (dotted_name) @source))
        ,
        // import "module"
        .go =>
        \\(import_spec path: (interpreted_string_literal) @source)
        ,
    };
}

// --- Extract source fragment from patch ---

fn extractSource(patch: []const u8, buf: []u8) []const u8 {
    var out_len: usize = 0;
    var lines = std.mem.splitScalar(u8, patch, '\n');
    while (lines.next()) |line| {
        const content: []const u8 = blk: {
            if (line.len == 0) break :blk "\n";
            if (std.mem.startsWith(u8, line, "+++") or std.mem.startsWith(u8, line, "---") or std.mem.startsWith(u8, line, "@@")) continue;
            // Added line: strip '+' prefix
            if (line[0] == '+') break :blk line[1..];
            // Context line: strip leading space
            if (line[0] == ' ') break :blk line[1..];
            // Deleted line: skip
            continue;
        };
        if (out_len + content.len + 1 >= buf.len) break;
        @memcpy(buf[out_len .. out_len + content.len], content);
        out_len += content.len;
        buf[out_len] = '\n';
        out_len += 1;
    }
    return buf[0..out_len];
}

// --- Detect signals from import paths ---

fn classifyImport(lang: Language, raw: []const u8, signals: *ImportSignals) void {
    // Strip quotes from string literals (Go includes them)
    const text = if (raw.len >= 2 and (raw[0] == '"' or raw[0] == '\''))
        raw[1 .. raw.len - 1]
    else
        raw;

    switch (lang) {
        .typescript => classifyTsImport(text, signals),
        .python => classifyPyImport(text, signals),
        .go => classifyGoImport(text, signals),
    }
}

fn classifyTsImport(mod: []const u8, s: *ImportSignals) void {
    if (std.mem.eql(u8, mod, "react") or std.mem.eql(u8, mod, "react-dom")) s.has_react = true;
    if (std.mem.eql(u8, mod, "vue")) s.has_vue = true;
    if (std.mem.eql(u8, mod, "svelte")) s.has_svelte = true;
    if (std.mem.eql(u8, mod, "zod") or std.mem.eql(u8, mod, "yup") or std.mem.eql(u8, mod, "joi")) s.has_validation = true;
    if (std.mem.startsWith(u8, mod, "@prisma/")) s.has_prisma = true;
    if (std.mem.eql(u8, mod, "express") or std.mem.startsWith(u8, mod, "express/")) s.has_web_framework = true;
    if (std.mem.startsWith(u8, mod, "jsonwebtoken") or std.mem.eql(u8, mod, "jose")) s.has_jwt = true;
    if (std.mem.startsWith(u8, mod, "passport")) s.has_passport = true;
    if (std.mem.startsWith(u8, mod, "next-auth") or std.mem.eql(u8, mod, "@auth/core")) s.has_nextauth = true;
}

fn classifyPyImport(mod: []const u8, s: *ImportSignals) void {
    if (std.mem.startsWith(u8, mod, "sqlalchemy") or std.mem.startsWith(u8, mod, "SQLAlchemy")) s.has_sqlalchemy = true;
    if (std.mem.startsWith(u8, mod, "fastapi") or std.mem.startsWith(u8, mod, "FastAPI") or
        std.mem.eql(u8, mod, "flask") or std.mem.startsWith(u8, mod, "flask.") or
        std.mem.startsWith(u8, mod, "django")) s.has_web_framework = true;
    if (std.mem.startsWith(u8, mod, "pytest")) s.has_pytest = true;
    if (std.mem.eql(u8, mod, "pydantic") or std.mem.startsWith(u8, mod, "pydantic.")) s.has_validation = true;
}

fn classifyGoImport(mod: []const u8, s: *ImportSignals) void {
    if (std.mem.eql(u8, mod, "net/http")) s.has_net_http = true;
    if (std.mem.eql(u8, mod, "database/sql")) s.has_sql_db = true;
    if (std.mem.eql(u8, mod, "testing")) s.has_testing = true;
    if (std.mem.startsWith(u8, mod, "github.com/golang-jwt/") or std.mem.startsWith(u8, mod, "github.com/dgrijalva/jwt")) s.has_jwt = true;
    if (std.mem.startsWith(u8, mod, "gorm.io/") or std.mem.startsWith(u8, mod, "github.com/lib/pq")) s.has_sql_db = true;
}

// --- Generate hints from signals ---

fn hintsFromSignals(s: ImportSignals) SemanticHints {
    // Aggregate signals per category
    const is_ui = s.has_react or s.has_vue or s.has_svelte or s.has_validation;
    const is_auth = s.has_jwt or s.has_passport or s.has_nextauth;
    const is_db = s.has_prisma or s.has_sqlalchemy or s.has_sql_db;
    const is_api = s.has_web_framework or s.has_net_http;
    const is_test = s.has_pytest or s.has_testing;

    // Conflicting signals from multiple categories: return no hints (defer to path classification)
    const category_count = @as(u8, @intFromBool(is_ui)) +
        @as(u8, @intFromBool(is_auth)) +
        @as(u8, @intFromBool(is_db)) +
        @as(u8, @intFromBool(is_api)) +
        @as(u8, @intFromBool(is_test));
    if (category_count > 1) return .{};

    // Single category detected: high confidence
    if (s.has_validation) return .{
        .domain_suppress = .database,
        .domain_boost = .ui,
        .confidence = 0.9,
    };
    if (s.has_react or s.has_vue or s.has_svelte) return .{
        .domain_suppress = .auth,
        .domain_boost = .ui,
        .confidence = 0.85,
    };
    if (is_auth) return .{
        .domain_boost = .auth,
        .confidence = 0.85,
    };
    if (is_db) return .{
        .domain_boost = .database,
        .confidence = 0.85,
    };
    if (is_api) return .{
        .domain_boost = .api,
        .confidence = 0.8,
    };
    if (is_test) return .{
        .domain_boost = .@"test",
        .confidence = 0.9,
    };
    return .{};
}
