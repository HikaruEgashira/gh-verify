use std::collections::HashSet;

use anyhow::Result;
use streaming_iterator::StreamingIterator;
use tree_sitter::{Language, Parser, Query, QueryCursor};

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct FileSymbols {
    pub filename: String,
    pub definitions: Vec<String>,
    pub calls: Vec<String>,
    pub imports: Vec<String>,
    pub identifiers: Vec<String>,
}

#[derive(Debug, Clone, Copy)]
pub enum Lang {
    TypeScript,
    Python,
    Go,
}

pub fn detect_language(filename: &str) -> Option<Lang> {
    if filename.ends_with(".ts")
        || filename.ends_with(".tsx")
        || filename.ends_with(".js")
        || filename.ends_with(".jsx")
    {
        Some(Lang::TypeScript)
    } else if filename.ends_with(".py") {
        Some(Lang::Python)
    } else if filename.ends_with(".go") {
        Some(Lang::Go)
    } else {
        None
    }
}

fn get_language(lang: Lang) -> Language {
    match lang {
        Lang::TypeScript => tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into(),
        Lang::Python => tree_sitter_python::LANGUAGE.into(),
        Lang::Go => tree_sitter_go::LANGUAGE.into(),
    }
}

fn def_query(lang: Lang) -> &'static str {
    match lang {
        Lang::TypeScript => {
            r#"(function_declaration name: (identifier) @name)
(method_definition name: (property_identifier) @name)
(lexical_declaration (variable_declarator name: (identifier) @name value: (arrow_function)))"#
        }
        Lang::Python => "(function_definition name: (identifier) @name)",
        Lang::Go => {
            r#"(function_declaration name: (identifier) @name)
(method_declaration name: (field_identifier) @name)"#
        }
    }
}

fn call_query(lang: Lang) -> &'static str {
    match lang {
        Lang::TypeScript => {
            r#"(call_expression function: (identifier) @name)
(call_expression function: (member_expression property: (property_identifier) @name))"#
        }
        Lang::Python => {
            r#"(call function: (identifier) @name)
(call function: (attribute attribute: (identifier) @name))"#
        }
        Lang::Go => {
            r#"(call_expression function: (identifier) @name)
(call_expression function: (selector_expression field: (field_identifier) @name))"#
        }
    }
}

fn import_query(lang: Lang) -> &'static str {
    match lang {
        Lang::TypeScript => {
            r#"(import_statement source: (string (string_fragment) @source))
(call_expression function: (identifier) @_req (#eq? @_req "require") arguments: (arguments (string (string_fragment) @source)))"#
        }
        Lang::Python => {
            r#"(import_from_statement module_name: (dotted_name) @source)
(import_statement name: (dotted_name) @source)"#
        }
        Lang::Go => r#"(import_spec path: (interpreted_string_literal) @source)"#,
    }
}

fn run_query(
    language: &Language,
    query_src: &str,
    source: &[u8],
    tree: &tree_sitter::Tree,
) -> Result<Vec<String>> {
    let query = Query::new(language, query_src)?;
    let mut cursor = QueryCursor::new();
    let mut matches = cursor.matches(&query, tree.root_node(), source);

    let mut results = Vec::new();
    let mut seen = HashSet::new();

    while let Some(m) = matches.next() {
        for cap in m.captures {
            let start = cap.node.start_byte();
            let end = cap.node.end_byte();
            if end > start && end <= source.len() {
                let name = std::str::from_utf8(&source[start..end]).unwrap_or("");
                if !name.is_empty() && seen.insert(name.to_string()) {
                    results.push(name.to_string());
                }
            }
        }
    }

    Ok(results)
}

fn extract_semantic_tokens(source: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut buf = String::new();
    for ch in source.chars() {
        if ch.is_ascii_alphanumeric() || ch == '_' {
            buf.push(ch.to_ascii_lowercase());
        } else if !buf.is_empty() {
            push_semantic_token(&buf, &mut out);
            buf.clear();
        }
    }
    if !buf.is_empty() {
        push_semantic_token(&buf, &mut out);
    }

    out.sort();
    out.dedup();
    out
}

fn push_semantic_token(token: &str, out: &mut Vec<String>) {
    if token.len() < 6 {
        return;
    }
    if token.bytes().all(|b| b.is_ascii_digit()) {
        return;
    }
    if is_semantic_stopword(token) {
        return;
    }
    out.push(token.to_string());
}

fn is_semantic_stopword(token: &str) -> bool {
    matches!(
        token,
        "import"
            | "export"
            | "default"
            | "const"
            | "let"
            | "function"
            | "return"
            | "class"
            | "interface"
            | "extends"
            | "implements"
            | "public"
            | "private"
            | "protected"
            | "static"
            | "async"
            | "await"
            | "throws"
            | "new"
            | "super"
            | "object"
            | "string"
            | "number"
            | "boolean"
            | "result"
            | "params"
            | "values"
            | "option"
            | "options"
            | "config"
            | "component"
            | "components"
    )
}

/// Extract source code from a unified diff patch.
pub fn extract_source(patch: &str) -> String {
    let mut output = String::new();
    for line in patch.lines() {
        if line.starts_with("+++") || line.starts_with("---") || line.starts_with("@@") {
            continue;
        }
        if line.is_empty() {
            output.push('\n');
            continue;
        }
        match line.as_bytes()[0] {
            b'+' | b' ' => {
                output.push_str(&line[1..]);
                output.push('\n');
            }
            _ => {}
        }
    }
    output
}

/// Extract symbols from a file's patch using tree-sitter or lexical fallback.
pub fn extract_symbols(filename: &str, patch: &str) -> Result<FileSymbols> {
    let source = extract_source(patch);
    if source.is_empty() {
        return Ok(lexical_fallback(filename, patch));
    }

    if let Some(lang) = detect_language(filename) {
        match extract_with_tree_sitter(filename, &source, lang) {
            Ok(symbols) => return Ok(symbols),
            Err(_) => return Ok(lexical_fallback(filename, patch)),
        }
    }
    Ok(lexical_fallback(filename, patch))
}

fn extract_with_tree_sitter(filename: &str, source: &str, lang: Lang) -> Result<FileSymbols> {
    let language = get_language(lang);
    let mut parser = Parser::new();
    parser.set_language(&language)?;

    let tree = parser
        .parse(source.as_bytes(), None)
        .ok_or_else(|| anyhow::anyhow!("parse failed"))?;

    let source_bytes = source.as_bytes();
    let definitions = run_query(&language, def_query(lang), source_bytes, &tree)?;
    let calls = run_query(&language, call_query(lang), source_bytes, &tree)?;
    let imports = run_query(&language, import_query(lang), source_bytes, &tree)?;
    let identifiers = extract_semantic_tokens(source);

    Ok(FileSymbols {
        filename: filename.to_string(),
        definitions,
        calls,
        imports,
        identifiers,
    })
}

fn lexical_fallback(filename: &str, patch: &str) -> FileSymbols {
    let source = extract_source(patch);
    let mut defs = Vec::new();
    let mut calls = Vec::new();
    let mut seen_defs = HashSet::new();
    let mut seen_calls = HashSet::new();
    let identifiers = extract_semantic_tokens(&source);

    let keywords = ["function ", "def ", "func ", "fn "];
    for line in source.lines() {
        let trimmed = line.trim();
        for kw in &keywords {
            if let Some(rest) = trimmed.strip_prefix(kw) {
                if let Some(name) = extract_identifier(rest) {
                    if seen_defs.insert(name.to_string()) {
                        defs.push(name.to_string());
                    }
                }
            }
        }

        // Simple call detection: identifier followed by '('
        let bytes = trimmed.as_bytes();
        let mut i = 0;
        while i < bytes.len() {
            if is_ident_start(bytes[i]) {
                let start = i;
                while i < bytes.len() && is_ident_char(bytes[i]) {
                    i += 1;
                }
                if i < bytes.len() && bytes[i] == b'(' {
                    let name = &trimmed[start..i];
                    if seen_calls.insert(name.to_string()) {
                        calls.push(name.to_string());
                    }
                }
            } else {
                i += 1;
            }
        }
    }

    FileSymbols {
        filename: filename.to_string(),
        definitions: defs,
        calls,
        imports: vec![],
        identifiers,
    }
}

fn extract_identifier(s: &str) -> Option<&str> {
    let s = s.trim_start();
    if s.is_empty() || !is_ident_start(s.as_bytes()[0]) {
        return None;
    }
    let end = s.bytes().position(|b| !is_ident_char(b)).unwrap_or(s.len());
    Some(&s[..end])
}

fn is_ident_start(b: u8) -> bool {
    b.is_ascii_alphabetic() || b == b'_'
}

fn is_ident_char(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_'
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_source_from_patch() {
        let patch = "@@ -1,3 +1,3 @@\n-old line\n+new line\n context line";
        let source = extract_source(patch);
        assert!(source.contains("new line"));
        assert!(source.contains("context line"));
        assert!(!source.contains("old line"));
    }

    #[test]
    fn detect_typescript() {
        assert!(matches!(detect_language("app.ts"), Some(Lang::TypeScript)));
        assert!(matches!(detect_language("app.tsx"), Some(Lang::TypeScript)));
    }

    #[test]
    fn detect_python() {
        assert!(matches!(detect_language("app.py"), Some(Lang::Python)));
    }

    #[test]
    fn detect_go() {
        assert!(matches!(detect_language("main.go"), Some(Lang::Go)));
    }

    #[test]
    fn detect_unknown() {
        assert!(detect_language("main.rs").is_none());
        assert!(detect_language("Makefile").is_none());
    }

    #[test]
    fn lexical_fallback_finds_definitions() {
        let patch = "@@ -0,0 +1,3 @@\n+function myFunc() {\n+  return 42;\n+}";
        let symbols = lexical_fallback("test.js", patch);
        assert!(symbols.definitions.contains(&"myFunc".to_string()));
    }

    #[test]
    fn lexical_fallback_finds_calls() {
        let patch = "@@ -0,0 +1,1 @@\n+  doSomething(arg);";
        let symbols = lexical_fallback("test.js", patch);
        assert!(symbols.calls.contains(&"doSomething".to_string()));
    }
}
