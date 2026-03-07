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

/// パッチ内容を Tree-sitter で AST 解析してセマンティックシグナルを抽出する。
/// patch には unified diff 形式の文字列を渡す（+ で始まる追加行 + コンテキスト行）。
pub fn analyzeSemantics(filename: []const u8, patch: []const u8) SemanticHints {
    const lang = detectLanguage(filename) orelse return .{};

    // パッチから表示可能な行を抽出（+ 行とコンテキスト行、--- / +++ ヘッダーを除く）
    var buf: [65536]u8 = undefined;
    const source = extractSource(patch, &buf);
    if (source.len == 0) return .{};

    const grammar = selectGrammar(lang) orelse return .{};
    const query_src = selectQuery(lang);

    // Tree-sitter でパース
    const parser = c.ts_parser_new() orelse return .{};
    defer c.ts_parser_delete(parser);

    _ = c.ts_parser_set_language(parser, grammar);

    const tree = c.ts_parser_parse_string(parser, null, source.ptr, @intCast(source.len)) orelse return .{};
    defer c.ts_tree_delete(tree);

    const root = c.ts_tree_root_node(tree);

    // クエリを実行してインポートモジュール名を収集
    var error_offset: u32 = 0;
    var error_type: c.TSQueryError = c.TSQueryErrorNone;
    const query = c.ts_query_new(grammar, query_src.ptr, @intCast(query_src.len), &error_offset, &error_type) orelse return .{};
    defer c.ts_query_delete(query);

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
                const text = source[start..end];
                classifyImport(lang, text, &signals);
            }
        }
    }

    return hintsFromSignals(signals);
}

// --- 内部型 ---

const Language = enum { typescript, python, go };

const ImportSignals = struct {
    has_react: bool = false,
    has_vue: bool = false,
    has_svelte: bool = false,
    has_zod: bool = false,
    has_yup: bool = false,
    has_joi: bool = false,
    has_prisma: bool = false,
    has_sqlalchemy: bool = false,
    has_sql_db: bool = false,
    has_fastapi: bool = false,
    has_express: bool = false,
    has_net_http: bool = false,
    has_pytest: bool = false,
    has_testing: bool = false,
    has_jwt: bool = false,
    has_passport: bool = false,
    has_nextauth: bool = false,
};

// --- ファイル言語判定 ---

fn detectLanguage(filename: []const u8) ?Language {
    if (std.mem.endsWith(u8, filename, ".ts") or
        std.mem.endsWith(u8, filename, ".tsx") or
        std.mem.endsWith(u8, filename, ".js") or
        std.mem.endsWith(u8, filename, ".jsx")) return .typescript;
    if (std.mem.endsWith(u8, filename, ".py")) return .python;
    if (std.mem.endsWith(u8, filename, ".go")) return .go;
    return null;
}

// --- グラマー選択 ---

fn selectGrammar(lang: Language) ?*const c.TSLanguage {
    return switch (lang) {
        .typescript => c.tree_sitter_typescript(),
        .python => c.tree_sitter_python(),
        .go => c.tree_sitter_go(),
    };
}

// --- 言語ごとの Tree-sitter クエリ ---
// インポート文からモジュール名（文字列リテラル）を @source キャプチャする。

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

// --- パッチからソースコード断片を抽出 ---

fn extractSource(patch: []const u8, buf: []u8) []const u8 {
    var out_len: usize = 0;
    var lines = std.mem.splitScalar(u8, patch, '\n');
    while (lines.next()) |line| {
        const content: []const u8 = blk: {
            if (line.len == 0) break :blk "\n";
            if (std.mem.startsWith(u8, line, "+++") or std.mem.startsWith(u8, line, "---") or std.mem.startsWith(u8, line, "@@")) continue;
            // 追加行: '+' プレフィックスを除去
            if (line[0] == '+') break :blk line[1..];
            // コンテキスト行: そのまま（先頭スペース除去）
            if (line[0] == ' ') break :blk line[1..];
            // 削除行: スキップ
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

// --- インポートパスからシグナルを検出 ---

fn classifyImport(lang: Language, raw: []const u8, signals: *ImportSignals) void {
    // 文字列リテラルのクォートを除去（Go は含む場合がある）
    const text = if (raw.len >= 2 and (raw[0] == '"' or raw[0] == '\''))
        raw[1 .. raw.len - 1]
    else
        raw;

    switch (lang) {
        .typescript => classifyTsImport(text, signals),
        .python => classifyPyImport(text, signals), // routes to classifyPyImport
        .go => classifyGoImport(text, signals),
    }
}

fn classifyTsImport(mod: []const u8, s: *ImportSignals) void {
    if (std.mem.eql(u8, mod, "react") or std.mem.eql(u8, mod, "react-dom")) s.has_react = true;
    if (std.mem.eql(u8, mod, "vue")) s.has_vue = true;
    if (std.mem.eql(u8, mod, "svelte")) s.has_svelte = true;
    if (std.mem.eql(u8, mod, "zod")) s.has_zod = true;
    if (std.mem.eql(u8, mod, "yup")) s.has_yup = true;
    if (std.mem.eql(u8, mod, "joi")) s.has_joi = true;
    if (std.mem.startsWith(u8, mod, "@prisma/")) s.has_prisma = true;
    if (std.mem.eql(u8, mod, "express") or std.mem.startsWith(u8, mod, "express/")) s.has_express = true;
    if (std.mem.startsWith(u8, mod, "jsonwebtoken") or std.mem.eql(u8, mod, "jose")) s.has_jwt = true;
    if (std.mem.startsWith(u8, mod, "passport")) s.has_passport = true;
    if (std.mem.startsWith(u8, mod, "next-auth") or std.mem.eql(u8, mod, "@auth/core")) s.has_nextauth = true;
}

fn classifyPyImport(mod: []const u8, s: *ImportSignals) void {
    if (std.mem.startsWith(u8, mod, "sqlalchemy") or std.mem.startsWith(u8, mod, "SQLAlchemy")) s.has_sqlalchemy = true;
    if (std.mem.startsWith(u8, mod, "fastapi") or std.mem.startsWith(u8, mod, "FastAPI")) s.has_fastapi = true;
    if (std.mem.startsWith(u8, mod, "pytest")) s.has_pytest = true;
    if (std.mem.eql(u8, mod, "flask") or std.mem.startsWith(u8, mod, "flask.")) s.has_fastapi = true; // Flask も api として扱う
    if (std.mem.startsWith(u8, mod, "django")) s.has_fastapi = true;
    if (std.mem.eql(u8, mod, "pydantic") or std.mem.startsWith(u8, mod, "pydantic.")) s.has_zod = true; // validation
}

fn classifyGoImport(mod: []const u8, s: *ImportSignals) void {
    if (std.mem.eql(u8, mod, "net/http")) s.has_net_http = true;
    if (std.mem.eql(u8, mod, "database/sql")) s.has_sql_db = true;
    if (std.mem.eql(u8, mod, "testing")) s.has_testing = true;
    if (std.mem.startsWith(u8, mod, "github.com/golang-jwt/") or std.mem.startsWith(u8, mod, "github.com/dgrijalva/jwt")) s.has_jwt = true;
    if (std.mem.startsWith(u8, mod, "gorm.io/") or std.mem.startsWith(u8, mod, "github.com/lib/pq")) s.has_sql_db = true;
}

// --- シグナルからヒント生成 ---

fn hintsFromSignals(s: ImportSignals) SemanticHints {
    // 検証スキーマライブラリ → UI/validation (データベースではない)
    if (s.has_zod or s.has_yup or s.has_joi) return .{
        .domain_suppress = .database,
        .domain_boost = .ui,
        .confidence = 0.9,
    };
    // React/Vue/Svelte → UI (auth ではない)
    if (s.has_react or s.has_vue or s.has_svelte) return .{
        .domain_suppress = .auth,
        .domain_boost = .ui,
        .confidence = 0.85,
    };
    // 認証専用ライブラリ → auth domain を強化
    if (s.has_jwt or s.has_passport or s.has_nextauth) return .{
        .domain_boost = .auth,
        .confidence = 0.85,
    };
    // ORM / DB ドライバー → database
    if (s.has_prisma or s.has_sqlalchemy or s.has_sql_db) return .{
        .domain_boost = .database,
        .confidence = 0.85,
    };
    // Web フレームワーク → api
    if (s.has_fastapi or s.has_express or s.has_net_http) return .{
        .domain_boost = .api,
        .confidence = 0.8,
    };
    // テストフレームワーク → test
    if (s.has_pytest or s.has_testing) return .{
        .domain_boost = .@"test",
        .confidence = 0.9,
    };
    return .{};
}
