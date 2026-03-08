const std = @import("std");

const c = @cImport({
    @cInclude("tree_sitter/api.h");
    @cInclude("tree_sitter/tree-sitter-go.h");
    @cInclude("tree_sitter/tree-sitter-python.h");
    @cInclude("tree_sitter/tree-sitter-typescript.h");
});

pub const FileSymbols = struct {
    filename: []const u8,
    definitions: [][]const u8,
    calls: [][]const u8,
    imports: [][]const u8,
};

pub const Language = enum { typescript, python, go };

// --- Cache: reuse parser and compiled queries across calls ---

var cached_parser: ?*c.TSParser = null;
var cached_def_queries: [3]?*c.TSQuery = .{ null, null, null };
var cached_call_queries: [3]?*c.TSQuery = .{ null, null, null };
var cached_import_queries: [3]?*c.TSQuery = .{ null, null, null };

fn getParser() ?*c.TSParser {
    if (cached_parser) |p| return p;
    const p = c.ts_parser_new() orelse return null;
    cached_parser = p;
    return p;
}

fn getCachedQuery(cache: *[3]?*c.TSQuery, lang: Language, src: []const u8) ?*c.TSQuery {
    const idx = @intFromEnum(lang);
    if (cache[idx]) |q| return q;
    const grammar = selectGrammar(lang) orelse return null;
    var err_offset: u32 = 0;
    var err_type: c.TSQueryError = c.TSQueryErrorNone;
    const q = c.ts_query_new(grammar, src.ptr, @intCast(src.len), &err_offset, &err_type) orelse return null;
    cache[idx] = q;
    return q;
}

/// Extract function definitions, calls, and imports from a file's patch content.
pub fn extractSymbols(alloc: std.mem.Allocator, filename: []const u8, patch: []const u8) !FileSymbols {
    var buf: [65536]u8 = undefined;
    const source = extractSource(patch, &buf);
    if (source.len == 0) return lexicalFallback(alloc, filename, patch);

    if (detectLanguage(filename)) |lang| {
        return extractWithTreeSitter(alloc, filename, source, lang) catch
            lexicalFallback(alloc, filename, patch);
    }
    return lexicalFallback(alloc, filename, patch);
}

fn extractWithTreeSitter(alloc: std.mem.Allocator, filename: []const u8, source: []const u8, lang: Language) !FileSymbols {
    const parser = getParser() orelse return error.ParserInitFailed;
    const grammar = selectGrammar(lang) orelse return error.GrammarNotFound;
    _ = c.ts_parser_set_language(parser, grammar);

    const tree = c.ts_parser_parse_string(parser, null, source.ptr, @intCast(source.len)) orelse return error.ParseFailed;
    defer c.ts_tree_delete(tree);
    const root = c.ts_tree_root_node(tree);

    var defs: std.ArrayList([]const u8) = .empty;
    var calls_list: std.ArrayList([]const u8) = .empty;
    var imports: std.ArrayList([]const u8) = .empty;

    // Extract definitions
    if (getCachedQuery(&cached_def_queries, lang, selectDefQuery(lang))) |query| {
        try runQuery(alloc, query, root, source, &defs);
    }

    // Extract calls
    if (getCachedQuery(&cached_call_queries, lang, selectCallQuery(lang))) |query| {
        try runQuery(alloc, query, root, source, &calls_list);
    }

    // Extract imports
    if (getCachedQuery(&cached_import_queries, lang, selectImportQuery(lang))) |query| {
        try runQuery(alloc, query, root, source, &imports);
    }

    return .{
        .filename = filename,
        .definitions = try defs.toOwnedSlice(alloc),
        .calls = try calls_list.toOwnedSlice(alloc),
        .imports = try imports.toOwnedSlice(alloc),
    };
}

fn runQuery(alloc: std.mem.Allocator, query: *c.TSQuery, root: c.TSNode, source: []const u8, results: *std.ArrayList([]const u8)) !void {
    const cursor = c.ts_query_cursor_new() orelse return;
    defer c.ts_query_cursor_delete(cursor);
    c.ts_query_cursor_exec(cursor, query, root);

    var match: c.TSQueryMatch = undefined;
    while (c.ts_query_cursor_next_match(cursor, &match)) {
        for (0..match.capture_count) |i| {
            const cap = match.captures[i];
            const start = c.ts_node_start_byte(cap.node);
            const end = c.ts_node_end_byte(cap.node);
            if (end > start and end <= source.len) {
                const name = source[start..end];
                // Deduplicate
                var found = false;
                for (results.items) |existing| {
                    if (std.mem.eql(u8, existing, name)) {
                        found = true;
                        break;
                    }
                }
                if (!found) {
                    try results.append(alloc, try alloc.dupe(u8, name));
                }
            }
        }
    }
}

// --- Language detection ---

pub fn detectLanguage(filename: []const u8) ?Language {
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

// --- Tree-sitter queries ---

fn selectDefQuery(lang: Language) []const u8 {
    return switch (lang) {
        .typescript =>
        \\(function_declaration name: (identifier) @name)
        \\(method_definition name: (property_identifier) @name)
        \\(lexical_declaration (variable_declarator name: (identifier) @name value: (arrow_function)))
        ,
        .python =>
        \\(function_definition name: (identifier) @name)
        ,
        .go =>
        \\(function_declaration name: (identifier) @name)
        \\(method_declaration name: (field_identifier) @name)
        ,
    };
}

fn selectCallQuery(lang: Language) []const u8 {
    return switch (lang) {
        .typescript =>
        \\(call_expression function: (identifier) @name)
        \\(call_expression function: (member_expression property: (property_identifier) @name))
        ,
        .python =>
        \\(call function: (identifier) @name)
        \\(call function: (attribute attribute: (identifier) @name))
        ,
        .go =>
        \\(call_expression function: (identifier) @name)
        \\(call_expression function: (selector_expression field: (field_identifier) @name))
        ,
    };
}

fn selectImportQuery(lang: Language) []const u8 {
    return switch (lang) {
        .typescript =>
        \\(import_statement source: (string (string_fragment) @source))
        \\(call_expression function: (identifier) @_req (#eq? @_req "require") arguments: (arguments (string (string_fragment) @source)))
        ,
        .python =>
        \\(import_from_statement module_name: (dotted_name) @source)
        \\(import_statement name: (dotted_name) @source)
        ,
        .go =>
        \\(import_spec path: (interpreted_string_literal) @source)
        ,
    };
}

// --- Extract source from patch ---

pub fn extractSource(patch: []const u8, buf: []u8) []const u8 {
    var out_len: usize = 0;
    var lines = std.mem.splitScalar(u8, patch, '\n');
    while (lines.next()) |line| {
        const content: []const u8 = blk: {
            if (line.len == 0) break :blk "\n";
            if (std.mem.startsWith(u8, line, "+++") or std.mem.startsWith(u8, line, "---") or std.mem.startsWith(u8, line, "@@")) continue;
            if (line[0] == '+') break :blk line[1..];
            if (line[0] == ' ') break :blk line[1..];
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

// --- Lexical fallback for unsupported languages ---

fn lexicalFallback(alloc: std.mem.Allocator, filename: []const u8, patch: []const u8) !FileSymbols {
    var defs: std.ArrayList([]const u8) = .empty;
    var calls_list: std.ArrayList([]const u8) = .empty;

    var buf: [65536]u8 = undefined;
    const source = extractSource(patch, &buf);

    // Scan for function definitions: function/def/func/fn followed by identifier
    var i: usize = 0;
    while (i < source.len) : (i += 1) {
        if (matchKeyword(source, i, "function") orelse
            matchKeyword(source, i, "def") orelse
            matchKeyword(source, i, "func") orelse
            matchKeyword(source, i, "fn")) |end|
        {
            if (extractIdentifier(source, end)) |ident| {
                const name = try alloc.dupe(u8, ident);
                if (!contains(defs.items, name)) {
                    try defs.append(alloc, name);
                }
                i = end + ident.len;
                continue;
            }
        }

        // Scan for calls: identifier followed by '('
        if (isIdentStart(source[i])) {
            const start = i;
            while (i < source.len and isIdentChar(source[i])) : (i += 1) {}
            if (i < source.len and source[i] == '(') {
                const name = try alloc.dupe(u8, source[start..i]);
                if (!contains(calls_list.items, name)) {
                    try calls_list.append(alloc, name);
                }
            }
        }
    }

    return .{
        .filename = filename,
        .definitions = try defs.toOwnedSlice(alloc),
        .calls = try calls_list.toOwnedSlice(alloc),
        .imports = try alloc.alloc([]const u8, 0),
    };
}

fn matchKeyword(source: []const u8, pos: usize, keyword: []const u8) ?usize {
    if (pos + keyword.len >= source.len) return null;
    if (!std.mem.eql(u8, source[pos .. pos + keyword.len], keyword)) return null;
    // Must be preceded by newline or start of string
    if (pos > 0 and isIdentChar(source[pos - 1])) return null;
    const end = pos + keyword.len;
    // Must be followed by whitespace
    if (end >= source.len or (source[end] != ' ' and source[end] != '\t')) return null;
    return end;
}

fn extractIdentifier(source: []const u8, pos: usize) ?[]const u8 {
    var start = pos;
    // Skip whitespace
    while (start < source.len and (source[start] == ' ' or source[start] == '\t')) : (start += 1) {}
    if (start >= source.len or !isIdentStart(source[start])) return null;
    var end = start;
    while (end < source.len and isIdentChar(source[end])) : (end += 1) {}
    return source[start..end];
}

fn isIdentStart(ch: u8) bool {
    return (ch >= 'a' and ch <= 'z') or (ch >= 'A' and ch <= 'Z') or ch == '_';
}

fn isIdentChar(ch: u8) bool {
    return isIdentStart(ch) or (ch >= '0' and ch <= '9');
}

fn contains(items: [][]const u8, name: []const u8) bool {
    for (items) |item| {
        if (std.mem.eql(u8, item, name)) return true;
    }
    return false;
}
