//! Static site generator for gh-verify rule specifications.
//!
//! Extracts documentation from:
//!   1. `#[ensures(...)]` specs in the Creusot verification crate
//!   2. `#[test]` functions (name, doc comment, full body, assertions)
//!   3. Rule metadata (ID, module doc, context)
//!
//! Generates `site/index.html` — a self-contained production-quality page.

use regex::Regex;
use std::collections::BTreeMap;
use std::fmt::Write as FmtWrite;
use std::fs;
use std::path::{Path, PathBuf};

// ---------------------------------------------------------------------------
// Data model
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
struct VerifSpec {
    fn_name: String,
    signature: String,
    ensures: Vec<String>,
    doc: String,
    body: String,
}

#[derive(Debug, Clone)]
struct TestCase {
    name: String,
    doc: String,
    body: String,
    assertions: Vec<String>,
    severity: Option<Severity>,
    source_file: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Severity {
    Pass,
    Warning,
    Error,
}

impl Severity {
    fn css_class(self) -> &'static str {
        match self {
            Self::Pass => "sev-pass",
            Self::Warning => "sev-warn",
            Self::Error => "sev-error",
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::Pass => "Pass",
            Self::Warning => "Warning",
            Self::Error => "Error",
        }
    }
}

#[derive(Debug)]
struct RuleInfo {
    rule_id: String,
    description: String,
    context: String,
    source_file: String,
    specs: Vec<VerifSpec>,
    tests: Vec<TestCase>,
}

// ---------------------------------------------------------------------------
// Parsers
// ---------------------------------------------------------------------------

fn extract_block(text: &str, open_pos: usize) -> &str {
    let bytes = text.as_bytes();
    let mut depth = 0;
    let mut pos = open_pos;
    while pos < bytes.len() {
        match bytes[pos] {
            b'{' => depth += 1,
            b'}' => {
                depth -= 1;
                if depth == 0 {
                    return &text[open_pos + 1..pos];
                }
            }
            _ => {}
        }
        pos += 1;
    }
    &text[open_pos + 1..]
}

fn collect_doc_comment(text: &str, end_pos: usize) -> String {
    let before = &text[..end_pos];
    let mut lines: Vec<&str> = Vec::new();
    for line in before.lines().rev() {
        let trimmed = line.trim();
        if trimmed.starts_with("///") {
            let content = trimmed.strip_prefix("///").unwrap().trim();
            lines.push(content);
        } else if trimmed.starts_with("#[") || trimmed.is_empty() {
            if !lines.is_empty() {
                break;
            }
        } else {
            break;
        }
    }
    lines.reverse();
    lines.join(" ")
}

fn parse_verif_specs(path: &Path) -> Vec<VerifSpec> {
    let text = match fs::read_to_string(path) {
        Ok(t) => t,
        Err(_) => return Vec::new(),
    };

    let mut specs = Vec::new();
    let ensures_re = Regex::new(r"#\[ensures\((.+?)\)\]").unwrap();
    let fn_re = Regex::new(r"pub fn (\w+)\(([^)]*)\)\s*->\s*([^\{]+)\{").unwrap();

    for fn_match in fn_re.find_iter(&text) {
        let fn_start = fn_match.start();
        let fn_cap = fn_re.captures(&text[fn_start..]).unwrap();
        let fn_name = fn_cap[1].to_string();
        let params = fn_cap[2].trim().to_string();
        let ret_type = fn_cap[3].trim().to_string();
        let signature = format!("fn {fn_name}({params}) -> {ret_type}");

        let prefix = &text[..fn_start];
        let mut ensures = Vec::new();
        for line in prefix.lines().rev() {
            let trimmed = line.trim();
            if trimmed.starts_with("#[ensures(") {
                for cap in ensures_re.captures_iter(trimmed) {
                    ensures.push(cap[1].to_string());
                }
            } else if trimmed.starts_with("///") || trimmed.is_empty() {
                continue;
            } else {
                break;
            }
        }
        if ensures.is_empty() {
            continue;
        }
        ensures.reverse();

        let doc = collect_doc_comment(&text, fn_start);
        let open_brace = fn_start + fn_match.end() - fn_match.start() - 1;
        let body = extract_block(&text, open_brace).trim().to_string();

        specs.push(VerifSpec {
            fn_name,
            signature,
            ensures,
            doc,
            body,
        });
    }

    specs
}

fn detect_severity(body: &str) -> Option<Severity> {
    let has_pass = body.contains("Severity::Pass");
    let has_warn = body.contains("Severity::Warning");
    let has_error = body.contains("Severity::Error");
    let has_is_empty = body.contains("is_empty()");
    let has_ne = body.contains("assert_ne!");

    if has_ne && (has_pass || has_error) {
        return None; // property test
    }
    if has_error && !has_pass && !has_warn {
        return Some(Severity::Error);
    }
    if has_warn && !has_error && !has_pass {
        return Some(Severity::Warning);
    }
    if has_pass && !has_error && !has_warn {
        return Some(Severity::Pass);
    }
    if has_is_empty && !has_error && !has_warn {
        return Some(Severity::Pass);
    }
    None
}

fn parse_tests(path: &Path, root: &Path) -> Vec<TestCase> {
    let text = match fs::read_to_string(path) {
        Ok(t) => t,
        Err(_) => return Vec::new(),
    };

    let rel_path = path.strip_prefix(root).unwrap_or(path);
    let source_file = rel_path.to_string_lossy().to_string();

    let test_mod_idx = match text.find("#[cfg(test)]") {
        Some(idx) => idx,
        None => return Vec::new(),
    };
    let test_section = &text[test_mod_idx..];

    let test_attr_re = Regex::new(r"#\[test\]\s*\n\s*fn (\w+)\(\)").unwrap();
    let mut tests = Vec::new();

    for m in test_attr_re.find_iter(test_section) {
        let cap = test_attr_re.captures(&test_section[m.start()..]).unwrap();
        let name = cap[1].to_string();
        let doc = collect_doc_comment(test_section, m.start());

        let after_sig = m.end();
        let remaining = &test_section[after_sig..];
        let brace_offset = match remaining.find('{') {
            Some(o) => o,
            None => continue,
        };

        let body = extract_block(test_section, after_sig + brace_offset)
            .trim()
            .to_string();

        let assertions: Vec<String> = body
            .lines()
            .filter(|l| l.trim().starts_with("assert"))
            .map(|l| l.trim().to_string())
            .collect();

        let severity = detect_severity(&body);

        tests.push(TestCase {
            name,
            doc,
            body,
            assertions,
            severity,
            source_file: source_file.clone(),
        });
    }

    tests
}

fn parse_rule_metadata(path: &Path) -> (String, String, String) {
    let text = match fs::read_to_string(path) {
        Ok(t) => t,
        Err(_) => return (String::new(), String::new(), String::new()),
    };

    let id_re = Regex::new(r#"const RULE_ID:\s*&str\s*=\s*"([^"]+)""#).unwrap();
    let rule_id = id_re
        .captures(&text)
        .map(|c| c[1].to_string())
        .unwrap_or_default();

    let mut doc_lines = Vec::new();
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("//!") {
            let content = trimmed.strip_prefix("//!").unwrap().trim();
            if !content.is_empty() {
                doc_lines.push(content.to_string());
            }
        } else if trimmed.is_empty() {
            continue;
        } else {
            break;
        }
    }
    let description = doc_lines.join(" ");

    let context = if text.contains("RuleContext::Pr") && text.contains("RuleContext::Release") {
        "PR + Release".to_string()
    } else if text.contains("RuleContext::Pr") {
        "PR".to_string()
    } else if text.contains("RuleContext::Release") {
        "Release".to_string()
    } else {
        "Unknown".to_string()
    };

    (rule_id, description, context)
}

// ---------------------------------------------------------------------------
// Auto-discovery: spec → rule mapping via function usage in core/cli sources
// ---------------------------------------------------------------------------

/// Build two maps by scanning CLI rule files for `use` statements:
///   1. core module name → rule ID  (e.g. "scope" → "detect-unscoped-change")
///   2. function name → rule ID     (e.g. "classify_scope" → "detect-unscoped-change")
/// Returns:
///   - module_map: core module name → Vec<rule_id> (for test file mapping)
///   - fn_map: function name → rule_id (only unique mappings, for spec mapping)
fn build_module_rule_maps(
    root: &Path,
) -> (BTreeMap<String, Vec<String>>, BTreeMap<String, String>) {
    let mut module_to_rules: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let mut fn_to_rules: BTreeMap<String, Vec<String>> = BTreeMap::new();

    let rule_id_re = Regex::new(r#"const RULE_ID:\s*&str\s*=\s*"([^"]+)""#).unwrap();
    let use_re = Regex::new(r"use (?:gh_verify_core|crate)::(\w+)").unwrap();
    let fn_call_re = Regex::new(r"\b([a-z_]\w+)\s*\(").unwrap();

    let dirs = [root.join("crates/cli/src/rules"), root.join("crates/core/src")];

    for dir in &dirs {
        for entry in walkdir(dir) {
            let text = match fs::read_to_string(&entry) {
                Ok(t) => t,
                Err(_) => continue,
            };
            let rule_id = match rule_id_re.captures(&text) {
                Some(cap) => cap[1].to_string(),
                None => continue,
            };

            for cap in use_re.captures_iter(&text) {
                let module_name = cap[1].to_string();
                let entry = module_to_rules.entry(module_name).or_default();
                if !entry.contains(&rule_id) {
                    entry.push(rule_id.clone());
                }
            }

            // Only scan non-test code for function calls (exclude #[cfg(test)] blocks)
            let non_test_text = text
                .find("#[cfg(test)]")
                .map(|idx| &text[..idx])
                .unwrap_or(&text);
            for cap in fn_call_re.captures_iter(non_test_text) {
                let fn_name = cap[1].to_string();
                let entry = fn_to_rules.entry(fn_name).or_default();
                if !entry.contains(&rule_id) {
                    entry.push(rule_id.clone());
                }
            }
        }
    }

    // fn_map: only keep functions uniquely mapped to ONE rule (for precise spec mapping)
    let fn_map: BTreeMap<String, String> = fn_to_rules
        .into_iter()
        .filter(|(_, rules)| rules.len() == 1)
        .map(|(k, mut v)| (k, v.remove(0)))
        .collect();

    (module_to_rules, fn_map)
}

/// Map a test source file to rule IDs.
/// Priority: RULE_ID constant > core module name match > filename match.
fn infer_rule_ids_from_file(
    path: &Path,
    rules: &BTreeMap<String, RuleInfo>,
    module_map: &BTreeMap<String, Vec<String>>,
) -> Vec<String> {
    let text = match fs::read_to_string(path) {
        Ok(t) => t,
        Err(_) => return vec![],
    };

    // Direct: file has RULE_ID
    let id_re = Regex::new(r#"const RULE_ID:\s*&str\s*=\s*"([^"]+)""#).unwrap();
    if let Some(cap) = id_re.captures(&text) {
        return vec![cap[1].to_string()];
    }

    // Indirect: the file's stem matches a core module mapped to rule(s).
    // Only use this if the module maps to exactly 1 rule (not shared infra).
    let file_stem = path
        .file_stem()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();
    if let Some(rule_ids) = module_map.get(&file_stem) {
        let valid: Vec<String> = rule_ids
            .iter()
            .filter(|id| rules.contains_key(*id))
            .cloned()
            .collect();
        if valid.len() == 1 {
            return valid;
        }
    }

    // Fallback: filename contains a rule's snake_case name
    let mut matched = Vec::new();
    let filename = path.file_name().unwrap_or_default().to_string_lossy();
    for rule in rules.values() {
        let rule_snake = rule.rule_id.replace('-', "_");
        if filename.contains(&rule_snake) {
            matched.push(rule.rule_id.clone());
        }
    }

    matched
}

/// Simple recursive directory walk (avoids adding walkdir crate).
fn walkdir(dir: &Path) -> Vec<PathBuf> {
    let mut result = Vec::new();
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                result.extend(walkdir(&path));
            } else if path.extension().map(|e| e == "rs").unwrap_or(false) {
                result.push(path);
            }
        }
    }
    result
}

// ---------------------------------------------------------------------------
// Collector
// ---------------------------------------------------------------------------

fn collect_rules(root: &Path) -> BTreeMap<String, RuleInfo> {
    let mut rules = BTreeMap::new();

    // 1. Auto-discover all rules from cli/src/rules/*.rs and core/src/*.rs
    let rule_dirs = [
        root.join("crates/cli/src/rules"),
        root.join("crates/core/src"),
    ];

    for dir in &rule_dirs {
        for path in walkdir(dir) {
            let filename = path.file_name().unwrap().to_string_lossy();
            if filename == "mod.rs" || filename == "engine.rs" || filename == "lib.rs" {
                continue;
            }
            let (rule_id, description, context) = parse_rule_metadata(&path);
            if rule_id.is_empty() {
                continue;
            }
            let rel_path = path
                .strip_prefix(root)
                .unwrap_or(&path)
                .to_string_lossy()
                .to_string();

            rules.entry(rule_id.clone()).or_insert_with(|| RuleInfo {
                rule_id,
                description,
                context,
                source_file: rel_path,
                specs: Vec::new(),
                tests: Vec::new(),
            });
        }
    }

    // 2. Build module→rule and fn→rule maps from use statements and calls
    let (module_map, fn_map) = build_module_rule_maps(root);

    // 3. Collect verif specs and map to rules via fn_map
    let verif_path = root.join("crates/verif/src/lib.rs");
    for spec in parse_verif_specs(&verif_path) {
        let rule_id = fn_map.get(&spec.fn_name).cloned().unwrap_or_default();
        if let Some(rule) = rules.get_mut(&rule_id) {
            rule.specs.push(spec);
        }
    }

    // 4. Collect tests from all .rs files with #[cfg(test)]
    let test_dirs = [
        root.join("crates/core/src"),
        root.join("crates/cli/src"),
    ];

    for dir in &test_dirs {
        for path in walkdir(dir) {
            let tests = parse_tests(&path, root);
            if tests.is_empty() {
                continue;
            }
            let rule_ids = infer_rule_ids_from_file(&path, &rules, &module_map);
            for rid in &rule_ids {
                if let Some(rule) = rules.get_mut(rid) {
                    rule.tests.extend(tests.clone());
                }
            }
        }
    }

    rules
}

// ---------------------------------------------------------------------------
// HTML renderer
// ---------------------------------------------------------------------------

fn esc(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

fn render_inline_md(s: &str) -> String {
    let s = esc(s);
    let bold_re = Regex::new(r"\*\*(.+?)\*\*").unwrap();
    let code_re = Regex::new(r"`(.+?)`").unwrap();
    let s = bold_re.replace_all(&s, "<strong>$1</strong>");
    let s = code_re.replace_all(&s, "<code>$1</code>");
    s.to_string()
}

fn render_html(rules: &BTreeMap<String, RuleInfo>) -> String {
    let total_specs: usize = rules.values().map(|r| r.specs.len()).sum();
    let total_tests: usize = rules.values().map(|r| r.tests.len()).sum();
    let timestamp = chrono_now();

    let mut html = String::with_capacity(64 * 1024);

    write!(
        html,
        r#"<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>gh-verify Rule Specification</title>
<style>{CSS}</style>
</head>
<body>
<div class="container">
"#
    )
    .unwrap();

    // Header
    write!(
        html,
        r#"<header>
<h1>gh-verify Rule Specification</h1>
<p class="subtitle">Derived from <strong>{total_tests}</strong> test cases and
<strong>{total_specs}</strong> <a href="https://github.com/creusot-rs/creusot">Creusot</a>
formal verification specs. Source of truth is code.</p>
<p class="timestamp">Generated: {timestamp}</p>
</header>
"#
    )
    .unwrap();

    // Stats
    write!(
        html,
        r#"<div class="stats">
<div class="stat"><span class="stat-value">{}</span><span class="stat-label">Rules</span></div>
<div class="stat"><span class="stat-value">{total_specs}</span><span class="stat-label">Formal Specs</span></div>
<div class="stat"><span class="stat-value">{total_tests}</span><span class="stat-label">Test Cases</span></div>
</div>
"#,
        rules.len()
    )
    .unwrap();

    // TOC
    html.push_str("<nav class=\"toc\"><h2>Rules</h2><ul>\n");
    for rule in rules.values() {
        let n_specs = rule.specs.len();
        let n_tests = rule.tests.len();
        let ctx = esc(&rule.context);
        let id = esc(&rule.rule_id);
        html.push_str(&format!(
            "<li>\
             <a href=\"#{id}\">{id}</a> \
             <span class=\"badge badge-ctx\">{ctx}</span> \
             <span class=\"badge badge-spec\">{n_specs} specs</span> \
             <span class=\"badge badge-test\">{n_tests} tests</span>\
             </li>\n"
        ));
    }
    html.push_str("</ul></nav>\n");

    for rule in rules.values() {
        render_rule(&mut html, rule);
    }

    // Footer
    write!(
        html,
        r#"<footer>
<p>Generated by <code>gen-docs</code> from test code and Creusot verification specs.
Regenerate: <code>devenv tasks run ghverify:docs</code></p>
</footer>
</div>
</body>
</html>"#
    )
    .unwrap();

    html
}

fn render_rule(html: &mut String, rule: &RuleInfo) {
    let id = esc(&rule.rule_id);
    let ctx = esc(&rule.context);
    let src = esc(&rule.source_file);
    html.push_str(&format!(
        "<section class=\"rule\" id=\"{id}\">\n\
         <h2><code>{id}</code></h2>\n\
         <div class=\"rule-meta\">\
         <span class=\"badge badge-ctx\">{ctx}</span> \
         <span class=\"source-link\">{src}</span>\
         </div>\n"
    ));

    if !rule.description.is_empty() {
        write!(
            html,
            "<p class=\"rule-desc\">{}</p>\n",
            render_inline_md(&rule.description)
        )
        .unwrap();
    }

    if !rule.specs.is_empty() {
        html.push_str(
            "<h3>Formal Specification <span class=\"badge badge-spec\">Creusot + SMT</span></h3>\n",
        );
        for spec in &rule.specs {
            render_spec(html, spec);
        }
    }

    if !rule.tests.is_empty() {
        write!(
            html,
            "<h3>Behavioral Specification <span class=\"badge badge-test\">{} tests</span></h3>\n",
            rule.tests.len()
        )
        .unwrap();

        render_decision_table(html, &rule.tests);

        html.push_str("<h4>Test Details</h4>\n");
        for test in &rule.tests {
            render_test(html, test);
        }
    }

    html.push_str("</section>\n");
}

fn render_spec(html: &mut String, spec: &VerifSpec) {
    write!(
        html,
        "<div class=\"spec-block\">\n<h4><code>{sig}</code></h4>\n",
        sig = esc(&spec.signature),
    )
    .unwrap();

    if !spec.doc.is_empty() {
        write!(
            html,
            "<p class=\"spec-doc\">{}</p>\n",
            render_inline_md(&spec.doc)
        )
        .unwrap();
    }

    html.push_str("<div class=\"ensures-list\">\n");
    for e in &spec.ensures {
        write!(
            html,
            "<pre class=\"ensures\"><code>#[ensures({e})]</code></pre>\n",
            e = esc(e)
        )
        .unwrap();
    }
    html.push_str("</div>\n");

    write!(
        html,
        "<details><summary>Implementation</summary><pre><code>{body}</code></pre></details>\n",
        body = esc(&spec.body)
    )
    .unwrap();

    html.push_str("</div>\n");
}

fn render_decision_table(html: &mut String, tests: &[TestCase]) {
    let categorized: Vec<_> = tests.iter().filter(|t| t.severity.is_some()).collect();
    if categorized.is_empty() {
        return;
    }

    html.push_str(
        "<table class=\"decision-table\">\n\
         <thead><tr><th>Scenario</th><th>Verdict</th><th>Key Assertion</th></tr></thead>\n\
         <tbody>\n",
    );

    for test in &categorized {
        let sev = test.severity.unwrap();
        let name = test.name.replace('_', " ");
        let key_assert = test
            .assertions
            .first()
            .map(|a| esc(a))
            .unwrap_or_default();

        write!(
            html,
            "<tr>\
             <td>{name}</td>\
             <td><span class=\"sev-badge {cls}\">{label}</span></td>\
             <td><code>{assert}</code></td>\
             </tr>\n",
            name = esc(&name),
            cls = sev.css_class(),
            label = sev.label(),
            assert = key_assert,
        )
        .unwrap();
    }

    html.push_str("</tbody>\n</table>\n");
}

fn render_test(html: &mut String, test: &TestCase) {
    let sev_badge = match test.severity {
        Some(s) => format!(
            " <span class=\"sev-badge {}\">{}</span>",
            s.css_class(),
            s.label()
        ),
        None => " <span class=\"sev-badge sev-prop\">Property</span>".to_string(),
    };

    let doc_span = if test.doc.is_empty() {
        String::new()
    } else {
        format!(
            " <span class=\"test-doc\">&mdash; {}</span>",
            render_inline_md(&test.doc)
        )
    };

    write!(
        html,
        "<details class=\"test-card\">\n\
         <summary><code>{name}</code>{sev}{doc}</summary>\n",
        name = esc(&test.name),
        sev = sev_badge,
        doc = doc_span,
    )
    .unwrap();

    write!(
        html,
        "<pre class=\"test-body\"><code>{body}</code></pre>\n",
        body = esc(&test.body)
    )
    .unwrap();

    write!(
        html,
        "<p class=\"source-link\">{src}</p>\n</details>\n",
        src = esc(&test.source_file)
    )
    .unwrap();
}

fn chrono_now() -> String {
    std::process::Command::new("date")
        .args(["+%Y-%m-%d %H:%M:%S %Z"])
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|| "unknown".to_string())
}

// ---------------------------------------------------------------------------
// CSS
// ---------------------------------------------------------------------------

const CSS: &str = r#"
:root {
  --bg: #0d1117; --fg: #e6edf3; --muted: #8b949e;
  --border: #30363d; --surface: #161b22; --accent: #58a6ff;
  --green: #3fb950; --yellow: #d29922; --red: #f85149;
  --code-bg: #1c2128;
}
*, *::before, *::after { margin: 0; padding: 0; box-sizing: border-box; }
body {
  font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Helvetica, Arial, sans-serif;
  background: var(--bg); color: var(--fg); line-height: 1.6;
}
.container { max-width: 960px; margin: 0 auto; padding: 2rem 1.5rem; }
a { color: var(--accent); text-decoration: none; }
a:hover { text-decoration: underline; }

header { margin-bottom: 1.5rem; }
h1 { font-size: 1.8rem; border-bottom: 1px solid var(--border); padding-bottom: 0.5rem; }
.subtitle { color: var(--muted); margin-top: 0.5rem; }
.subtitle strong { color: var(--fg); }
.timestamp { color: var(--muted); font-size: 0.8rem; margin-top: 0.3rem; }

.stats { display: flex; gap: 1rem; margin: 1.5rem 0; flex-wrap: wrap; }
.stat {
  background: var(--surface); border: 1px solid var(--border); border-radius: 8px;
  padding: 0.8rem 1.2rem; text-align: center; min-width: 130px; flex: 1;
}
.stat-value { font-size: 2rem; font-weight: 700; display: block; }
.stat-label { font-size: 0.8rem; color: var(--muted); }

.toc {
  background: var(--surface); border: 1px solid var(--border); border-radius: 8px;
  padding: 1rem 1.5rem; margin: 1.5rem 0;
}
.toc h2 { font-size: 1rem; color: var(--muted); border: none; margin-bottom: 0.5rem; }
.toc ul { list-style: none; }
.toc li { margin: 0.4rem 0; display: flex; align-items: center; gap: 0.5rem; flex-wrap: wrap; }

.badge {
  display: inline-block; font-size: 0.7rem; padding: 0.15rem 0.5rem; border-radius: 12px;
  font-weight: 600; white-space: nowrap;
}
.badge-spec { background: rgba(88,166,255,0.15); color: var(--accent); }
.badge-test { background: rgba(63,185,80,0.15); color: var(--green); }
.badge-ctx { background: rgba(139,148,158,0.15); color: var(--muted); }

.sev-badge {
  display: inline-block; font-size: 0.7rem; padding: 0.1rem 0.45rem; border-radius: 4px;
  font-weight: 700; text-transform: uppercase; letter-spacing: 0.03em;
}
.sev-pass { background: rgba(63,185,80,0.15); color: var(--green); }
.sev-warn { background: rgba(210,153,34,0.15); color: var(--yellow); }
.sev-error { background: rgba(248,81,73,0.15); color: var(--red); }
.sev-prop { background: rgba(88,166,255,0.15); color: var(--accent); }

.rule {
  margin: 2.5rem 0; padding-top: 1rem;
  border-top: 2px solid var(--border);
}
.rule h2 {
  font-size: 1.4rem; margin-bottom: 0.3rem;
  display: flex; align-items: center; gap: 0.5rem;
}
.rule-meta {
  display: flex; align-items: center; gap: 0.8rem;
  margin-bottom: 0.5rem;
}
.rule-desc { color: var(--muted); margin-bottom: 1rem; }
h3 {
  font-size: 1.1rem; margin: 1.5rem 0 0.5rem;
  display: flex; align-items: center; gap: 0.5rem;
}
h4 { font-size: 0.95rem; margin: 1rem 0 0.3rem; }
.source-link { color: var(--muted); font-size: 0.75rem; }

pre, code {
  font-family: "SFMono-Regular", Consolas, "Liberation Mono", Menlo, monospace;
}
pre {
  background: var(--code-bg); border: 1px solid var(--border); border-radius: 6px;
  padding: 0.7rem 1rem; overflow-x: auto; font-size: 0.82rem; line-height: 1.5;
}

.spec-block {
  border-left: 3px solid var(--accent); padding: 0.8rem 1rem; margin: 0.8rem 0;
  background: rgba(88,166,255,0.03); border-radius: 0 6px 6px 0;
}
.spec-block h4 { margin: 0 0 0.3rem; font-size: 0.9rem; }
.spec-doc { color: var(--muted); font-size: 0.85rem; margin: 0.3rem 0 0.5rem; }
.ensures-list { display: flex; flex-direction: column; gap: 0.3rem; }
.ensures { margin: 0; font-size: 0.8rem; border-color: var(--accent); }
.spec-block details { margin-top: 0.5rem; }
.spec-block details summary {
  font-size: 0.8rem; color: var(--muted); cursor: pointer;
}

.decision-table {
  width: 100%; border-collapse: collapse; margin: 0.8rem 0;
  font-size: 0.85rem;
}
.decision-table th {
  text-align: left; padding: 0.5rem 0.8rem;
  border-bottom: 2px solid var(--border); color: var(--muted);
  font-weight: 600; font-size: 0.8rem; text-transform: uppercase;
  letter-spacing: 0.03em;
}
.decision-table td {
  padding: 0.4rem 0.8rem; border-bottom: 1px solid var(--border);
}
.decision-table tr:hover { background: rgba(255,255,255,0.02); }
.decision-table code { font-size: 0.78rem; }

.test-card {
  background: var(--surface); border: 1px solid var(--border); border-radius: 6px;
  padding: 0.6rem 1rem; margin: 0.4rem 0;
}
.test-card summary {
  cursor: pointer; font-size: 0.85rem;
  display: flex; align-items: center; gap: 0.5rem; flex-wrap: wrap;
}
.test-card summary:hover { color: var(--accent); }
.test-card[open] summary { margin-bottom: 0.5rem; }
.test-doc { color: var(--muted); font-style: italic; font-size: 0.8rem; }
.test-body { margin: 0.5rem 0; font-size: 0.78rem; }

footer {
  margin-top: 3rem; padding-top: 1rem;
  border-top: 1px solid var(--border);
  color: var(--muted); font-size: 0.8rem;
}

@media (max-width: 640px) {
  .container { padding: 1rem; }
  .stats { flex-direction: column; }
  .decision-table { font-size: 0.78rem; }
  .decision-table th, .decision-table td { padding: 0.3rem 0.5rem; }
}
"#;

// ---------------------------------------------------------------------------
// Main
// ---------------------------------------------------------------------------

fn find_workspace_root() -> PathBuf {
    let mut dir = std::env::current_dir().expect("cannot get cwd");
    loop {
        if dir.join("Cargo.toml").exists() {
            let content = fs::read_to_string(dir.join("Cargo.toml")).unwrap_or_default();
            if content.contains("[workspace]") {
                return dir;
            }
        }
        if !dir.pop() {
            eprintln!("error: cannot find workspace root (Cargo.toml with [workspace])");
            std::process::exit(1);
        }
    }
}

fn main() {
    let root = find_workspace_root();
    let out_dir = root.join("site");
    fs::create_dir_all(&out_dir).expect("cannot create site/");

    let rules = collect_rules(&root);
    let html_content = render_html(&rules);

    let out_path = out_dir.join("index.html");
    fs::write(&out_path, &html_content).expect("cannot write index.html");

    println!(
        "Generated {} ({} bytes)",
        out_path.display(),
        html_content.len()
    );
    for rule in rules.values() {
        println!(
            "  {}: {} specs, {} tests",
            rule.rule_id,
            rule.specs.len(),
            rule.tests.len()
        );
    }
}
