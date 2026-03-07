const std = @import("std");
const Domain = @import("diff_parser.zig").Domain;

pub const SemanticHints = struct {
    domain_boost: ?Domain = null,
    domain_suppress: ?Domain = null,
    confidence: f32 = 0,
};

/// パッチ内容からセマンティックシグナルを抽出する。
/// 追加行（`+`プレフィックス）のみを解析し、import文やデコレータからドメインを推定する。
pub fn analyzeSemantics(filename: []const u8, patch: []const u8) SemanticHints {
    var hints = SemanticHints{};

    const lang = detectLanguage(filename);
    if (lang == .unknown) return hints;

    // パッチの追加行を走査
    var lines = std.mem.splitScalar(u8, patch, '\n');
    var import_signals = ImportSignals{};

    while (lines.next()) |line| {
        if (line.len == 0 or line[0] != '+') continue;
        if (line.len > 1 and line[1] == '+') continue; // +++ header

        const content = line[1..]; // skip '+' prefix

        switch (lang) {
            .typescript, .javascript => analyzeJsImport(content, &import_signals),
            .python => analyzePyImport(content, &import_signals),
            .go => analyzeGoImport(content, &import_signals),
            .unknown => {},
        }
    }

    // シグナルからヒントを生成
    if (import_signals.has_zod or import_signals.has_yup or import_signals.has_joi) {
        hints.domain_suppress = .database;
        hints.domain_boost = .ui;
        hints.confidence = 0.9;
    } else if (import_signals.has_react or import_signals.has_svelte or import_signals.has_vue) {
        hints.domain_suppress = .auth;
        hints.domain_boost = .ui;
        hints.confidence = 0.8;
    } else if (import_signals.has_prisma or import_signals.has_sqlalchemy or import_signals.has_sql_db) {
        hints.domain_boost = .database;
        hints.confidence = 0.8;
    } else if (import_signals.has_fastapi or import_signals.has_express_router or import_signals.has_net_http) {
        hints.domain_boost = .api;
        hints.confidence = 0.8;
    } else if (import_signals.has_pytest or import_signals.has_testing) {
        hints.domain_boost = .@"test";
        hints.confidence = 0.9;
    }

    return hints;
}

const Language = enum { typescript, javascript, python, go, unknown };

fn detectLanguage(filename: []const u8) Language {
    if (std.mem.endsWith(u8, filename, ".ts") or std.mem.endsWith(u8, filename, ".tsx")) return .typescript;
    if (std.mem.endsWith(u8, filename, ".js") or std.mem.endsWith(u8, filename, ".jsx")) return .javascript;
    if (std.mem.endsWith(u8, filename, ".py")) return .python;
    if (std.mem.endsWith(u8, filename, ".go")) return .go;
    return .unknown;
}

const ImportSignals = struct {
    has_react: bool = false,
    has_svelte: bool = false,
    has_vue: bool = false,
    has_zod: bool = false,
    has_yup: bool = false,
    has_joi: bool = false,
    has_prisma: bool = false,
    has_sqlalchemy: bool = false,
    has_sql_db: bool = false,
    has_fastapi: bool = false,
    has_express_router: bool = false,
    has_net_http: bool = false,
    has_pytest: bool = false,
    has_testing: bool = false,
};

fn analyzeJsImport(line: []const u8, signals: *ImportSignals) void {
    const trimmed = std.mem.trim(u8, line, " \t");

    // import ... from "xxx" or require("xxx")
    if (std.mem.indexOf(u8, trimmed, "from ") != null or std.mem.indexOf(u8, trimmed, "require(") != null) {
        if (containsAny(trimmed, &.{ "\"react\"", "'react'", "\"react-dom\"", "'react-dom'" })) signals.has_react = true;
        if (containsAny(trimmed, &.{ "\"svelte\"", "'svelte'" })) signals.has_svelte = true;
        if (containsAny(trimmed, &.{ "\"vue\"", "'vue'" })) signals.has_vue = true;
        if (containsAny(trimmed, &.{ "\"zod\"", "'zod'" })) signals.has_zod = true;
        if (containsAny(trimmed, &.{ "\"yup\"", "'yup'" })) signals.has_yup = true;
        if (containsAny(trimmed, &.{ "\"joi\"", "'joi'" })) signals.has_joi = true;
        if (containsAny(trimmed, &.{ "\"@prisma/client\"", "'@prisma/client'" })) signals.has_prisma = true;
        if (containsAny(trimmed, &.{ "\"express\"", "'express'" })) signals.has_express_router = true;
    }
}

fn analyzePyImport(line: []const u8, signals: *ImportSignals) void {
    const trimmed = std.mem.trim(u8, line, " \t");

    if (std.mem.startsWith(u8, trimmed, "import ") or std.mem.startsWith(u8, trimmed, "from ")) {
        if (std.mem.indexOf(u8, trimmed, "sqlalchemy") != null) signals.has_sqlalchemy = true;
        if (std.mem.indexOf(u8, trimmed, "fastapi") != null or std.mem.indexOf(u8, trimmed, "FastAPI") != null) signals.has_fastapi = true;
        if (std.mem.indexOf(u8, trimmed, "pytest") != null) signals.has_pytest = true;
        if (std.mem.indexOf(u8, trimmed, "react") != null) signals.has_react = true;
    }

    // @pytest.fixture, @app.route
    if (std.mem.startsWith(u8, trimmed, "@")) {
        if (std.mem.indexOf(u8, trimmed, "pytest") != null) signals.has_pytest = true;
        if (std.mem.indexOf(u8, trimmed, "app.route") != null or std.mem.indexOf(u8, trimmed, "app.get") != null or std.mem.indexOf(u8, trimmed, "app.post") != null) signals.has_fastapi = true;
    }
}

fn analyzeGoImport(line: []const u8, signals: *ImportSignals) void {
    const trimmed = std.mem.trim(u8, line, " \t");

    if (std.mem.indexOf(u8, trimmed, "\"net/http\"") != null) signals.has_net_http = true;
    if (std.mem.indexOf(u8, trimmed, "\"database/sql\"") != null) signals.has_sql_db = true;
    if (std.mem.indexOf(u8, trimmed, "\"testing\"") != null) signals.has_testing = true;
}

fn containsAny(haystack: []const u8, needles: []const []const u8) bool {
    for (needles) |needle| {
        if (std.mem.indexOf(u8, haystack, needle) != null) return true;
    }
    return false;
}
