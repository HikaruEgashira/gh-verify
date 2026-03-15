//! Scope classification and semantic connectivity logic for PR change analysis.
//!
//! Determines whether a PR's changes are well-scoped (single logical unit)
//! or spread across disconnected domains.

use crate::verdict::Severity;

/// Classify the scope of a PR based on the number of connected components
/// among its changed code files.
/// Verified by Creusot in `gh-verify-verif` crate.
pub fn classify_scope(code_files_count: usize, components: usize) -> Severity {
    if code_files_count <= 1 {
        return Severity::Pass;
    }
    match components {
        0 | 1 => Severity::Pass,
        2 => Severity::Warning,
        _ => Severity::Error, // 3+
    }
}

/// Known non-code file extensions that should be excluded from scope analysis.
pub const NON_CODE_EXTENSIONS: &[&str] = &[
    ".md", ".rst", ".txt", ".json", ".yaml", ".yml", ".toml", ".lock", ".env", ".cfg", ".ini",
    ".css", ".scss", ".svg", ".png", ".jpg", ".gif", ".ico", ".woff", ".woff2",
];

/// Known non-code path prefixes that should be excluded from scope analysis.
pub const NON_CODE_PREFIXES: &[&str] = &[".github/", "docs/"];

/// Coarse role of a changed file for weak semantic connectivity.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileRole {
    Source,
    Test,
    Fixture,
}

/// Determine whether a file path refers to a non-code file.
pub fn is_non_code_file(filename: &str) -> bool {
    for prefix in NON_CODE_PREFIXES {
        if filename.starts_with(prefix) {
            return true;
        }
    }
    for ext in NON_CODE_EXTENSIONS {
        if filename.ends_with(ext) {
            return true;
        }
    }
    false
}

/// Resolve an import path against a set of changed file paths.
/// Returns the index of the matched file, if any.
pub fn resolve_import(import_path: &str, filenames: &[&str]) -> Option<usize> {
    let mut path = import_path;

    // Strip quotes (Go imports include them)
    if path.len() >= 2 && (path.starts_with('"') || path.starts_with('\'')) {
        path = &path[1..path.len() - 1];
    }

    // Strip relative prefixes
    if let Some(stripped) = path.strip_prefix("./") {
        path = stripped;
    } else if let Some(stripped) = path.strip_prefix("../") {
        path = stripped;
    } else if let Some(stripped) = path.strip_prefix("@/") {
        path = stripped;
    }

    // Convert Python dotted notation to path
    let converted: String;
    if path.contains('.') && !path.contains('/') {
        converted = path.replace('.', "/");
        path = &converted;
    } else {
        converted = String::new();
        let _ = &converted; // suppress unused warning
    }

    // Match against changed file names (suffix match)
    for (idx, fname) in filenames.iter().enumerate() {
        // Exact suffix match
        if fname.ends_with(path) {
            return Some(idx);
        }
        // Try with common extensions
        for ext in &[
            ".ts",
            ".tsx",
            ".js",
            ".jsx",
            ".py",
            ".go",
            "/index.ts",
            "/index.js",
        ] {
            let with_ext = format!("{path}{ext}");
            if fname.ends_with(&with_ext) {
                return Some(idx);
            }
        }
    }
    None
}

/// Classify file role from path shape and filename conventions.
pub fn classify_file_role(path: &str) -> FileRole {
    let normalized = path.to_ascii_lowercase();

    if has_fixture_marker(&normalized) {
        return FileRole::Fixture;
    }
    if has_test_marker(&normalized) {
        return FileRole::Test;
    }
    FileRole::Source
}

/// Extract semantic tokens from path for weak matching.
pub fn semantic_path_tokens(path: &str) -> Vec<String> {
    let mut out = Vec::new();

    for segment in path.split('/') {
        for dot_part in segment.split('.') {
            extend_split_tokens(dot_part, &mut out);
        }
    }

    out.sort();
    out.dedup();
    out
}

/// Source-source weak bridge used for colocated feature files.
/// Guarded by strict long-stem overlap to avoid short-name over-merging.
pub fn should_bridge_colocated_sources(path_a: &str, path_b: &str) -> bool {
    if classify_file_role(path_a) != FileRole::Source
        || classify_file_role(path_b) != FileRole::Source
    {
        return false;
    }
    if parent_dir(path_a) != parent_dir(path_b) {
        return false;
    }

    let stem_a = normalized_file_stem(path_a);
    let stem_b = normalized_file_stem(path_b);
    if common_prefix_len(&stem_a, &stem_b) >= 8 {
        return true;
    }

    let tokens_a = filename_tokens(path_a);
    let tokens_b = filename_tokens(path_b);
    has_token_overlap(&tokens_a, &tokens_b, 8, true)
}

/// Bridge test/fixture file to a source file with semantic token overlap.
/// Guards: role check, parent_dir difference, same package root, and
/// token overlap (≥5 chars, non-generic).
pub fn should_bridge_aux_to_source(source_path: &str, aux_path: &str) -> bool {
    if classify_file_role(source_path) != FileRole::Source {
        return false;
    }

    let aux_role = classify_file_role(aux_path);
    if aux_role != FileRole::Test && aux_role != FileRole::Fixture {
        return false;
    }

    // Do not collapse same-parent unit test pairs (can hide real split concerns).
    if parent_dir(source_path) == parent_dir(aux_path) {
        return false;
    }

    // Source and aux must be in the same package (share a package root).
    // This prevents cross-package bridging via coincidental token overlap.
    if package_root(source_path) != package_root(aux_path) {
        return false;
    }

    let source_tokens = semantic_path_tokens(source_path);
    let aux_tokens = semantic_path_tokens(aux_path);
    has_token_overlap(&source_tokens, &aux_tokens, 5, true)
}

/// Bridge between test and fixture files that target the same behavior.
pub fn should_bridge_test_fixture_pair(path_a: &str, path_b: &str) -> bool {
    let role_a = classify_file_role(path_a);
    let role_b = classify_file_role(path_b);
    let is_test_fixture = (role_a == FileRole::Test && role_b == FileRole::Fixture)
        || (role_a == FileRole::Fixture && role_b == FileRole::Test);

    if !is_test_fixture {
        return false;
    }

    let tokens_a = filename_tokens(path_a);
    let tokens_b = filename_tokens(path_b);
    has_token_overlap(&tokens_a, &tokens_b, 5, true)
}

/// Bridge build-fork variants that share one canonical feature surface.
pub fn should_bridge_fork_variants(path_a: &str, path_b: &str) -> bool {
    if classify_file_role(path_a) != FileRole::Source
        || classify_file_role(path_b) != FileRole::Source
    {
        return false;
    }

    if !is_fork_variant_path(path_a) && !is_fork_variant_path(path_b) {
        return false;
    }

    let family_a = fork_family_root(path_a);
    let family_b = fork_family_root(path_b);
    if family_a.is_empty() || family_a != family_b {
        return false;
    }

    let stem_a = normalized_file_stem(path_a);
    let stem_b = normalized_file_stem(path_b);
    if stem_a != stem_b {
        return false;
    }
    if stem_a.len() < 8 || is_generic_token(&stem_a) {
        return false;
    }

    true
}

fn has_fixture_marker(path: &str) -> bool {
    path.contains("/__fixtures__/")
        || path.contains("/fixtures/")
        || path.contains("/fixture/")
        || path.contains("/fixtures-")
        || (path.contains("/cases/") && (path.contains("test") || path.contains("e2e")))
}

fn has_test_marker(path: &str) -> bool {
    path.contains("/__tests__/")
        || path.contains("/tests/")
        || path.contains("/test/")
        || path.contains("/e2e/")
        || path.contains(".test.")
        || path.contains("_test.")
        || path.contains(".spec.")
        || path.contains("-test.")
        || path.contains("-spec.")
        || path.contains("test-d.ts")
}

fn extend_split_tokens(input: &str, out: &mut Vec<String>) {
    let mut buf = String::new();
    let mut prev_is_lower = false;

    for ch in input.chars() {
        if ch.is_ascii_alphanumeric() {
            let is_upper = ch.is_ascii_uppercase();
            if is_upper && prev_is_lower && !buf.is_empty() {
                push_token(&buf, out);
                buf.clear();
            }
            buf.push(ch.to_ascii_lowercase());
            prev_is_lower = ch.is_ascii_lowercase();
        } else {
            if !buf.is_empty() {
                push_token(&buf, out);
                buf.clear();
            }
            prev_is_lower = false;
        }
    }

    if !buf.is_empty() {
        push_token(&buf, out);
    }
}

fn push_token(token: &str, out: &mut Vec<String>) {
    if token.len() >= 3 {
        out.push(token.to_string());
    }
}

fn normalized_file_stem(path: &str) -> String {
    let file = path.rsplit('/').next().unwrap_or(path);
    let stem = file.split('.').next().unwrap_or(file);
    stem.chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect::<String>()
}

fn filename_tokens(path: &str) -> Vec<String> {
    let file = path.rsplit('/').next().unwrap_or(path);
    let stem = file.split('.').next().unwrap_or(file);
    let mut out = Vec::new();
    extend_split_tokens(stem, &mut out);
    out.sort();
    out.dedup();
    out
}

fn parent_dir(path: &str) -> &str {
    path.rsplit_once('/').map(|(p, _)| p).unwrap_or("")
}

/// Detect the package root by finding the prefix before the first conventional
/// boundary directory (src, lib, test, tests, __tests__, e2e).
/// Falls back to parent_dir when no boundary is found.
fn package_root(path: &str) -> &str {
    const BOUNDARIES: &[&str] = &[
        "/src/", "/lib/", "/test/", "/tests/", "/__tests__/", "/e2e/",
    ];
    for boundary in BOUNDARIES {
        if let Some(idx) = path.find(boundary) {
            return &path[..idx];
        }
    }
    parent_dir(path)
}

fn is_fork_variant_path(path: &str) -> bool {
    path.contains("/forks/")
}

fn fork_family_root(path: &str) -> String {
    if let Some((prefix, _)) = path.split_once("/forks/") {
        return prefix.to_string();
    }
    parent_dir(path).to_string()
}

fn common_prefix_len(a: &str, b: &str) -> usize {
    a.bytes().zip(b.bytes()).take_while(|(x, y)| x == y).count()
}

fn has_token_overlap(
    tokens_a: &[String],
    tokens_b: &[String],
    min_len: usize,
    require_non_generic: bool,
) -> bool {
    tokens_a.iter().any(|a| {
        if a.len() < min_len {
            return false;
        }
        if require_non_generic && is_generic_token(a) {
            return false;
        }
        tokens_b.iter().any(|b| b == a)
    })
}

fn is_generic_token(token: &str) -> bool {
    matches!(
        token,
        "test"
            | "tests"
            | "spec"
            | "fixture"
            | "fixtures"
            | "runtime"
            | "source"
            | "types"
            | "type"
            | "index"
            | "core"
            | "src"
            | "lib"
            | "util"
            | "utils"
            | "package"
            | "packages"
            | "private"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zero_or_one_file_is_pass() {
        assert_eq!(classify_scope(0, 0), Severity::Pass);
        assert_eq!(classify_scope(1, 1), Severity::Pass);
        assert_eq!(classify_scope(1, 5), Severity::Pass);
    }

    #[test]
    fn single_component_is_pass() {
        assert_eq!(classify_scope(5, 1), Severity::Pass);
    }

    #[test]
    fn two_components_is_warning() {
        assert_eq!(classify_scope(5, 2), Severity::Warning);
    }

    #[test]
    fn three_or_more_components_is_error() {
        assert_eq!(classify_scope(5, 3), Severity::Error);
        assert_eq!(classify_scope(10, 7), Severity::Error);
    }

    #[test]
    fn markdown_is_non_code() {
        assert!(is_non_code_file("README.md"));
        assert!(is_non_code_file("docs/guide.md"));
    }

    #[test]
    fn github_dir_is_non_code() {
        assert!(is_non_code_file(".github/workflows/ci.yml"));
    }

    #[test]
    fn source_files_are_code() {
        assert!(!is_non_code_file("src/main.rs"));
        assert!(!is_non_code_file("lib/utils.ts"));
        assert!(!is_non_code_file("app.py"));
    }

    #[test]
    fn resolve_relative_import() {
        let files = vec!["src/utils/helper.ts"];
        assert_eq!(resolve_import("./helper", &files), Some(0));
    }

    #[test]
    fn resolve_python_dotted() {
        let files = vec!["src/foo/bar.py"];
        assert_eq!(resolve_import("foo.bar", &files), Some(0));
    }

    #[test]
    fn resolve_go_quoted() {
        let files = vec!["internal/handler.go"];
        assert_eq!(resolve_import("\"internal/handler\"", &files), Some(0));
    }

    #[test]
    fn no_match_returns_none() {
        let files = vec!["src/main.rs"];
        assert_eq!(resolve_import("nonexistent", &files), None);
    }

    #[test]
    fn classify_scope_exhaustive_for_small_inputs() {
        for files in 0..=10 {
            for comps in 0..=10 {
                let result = classify_scope(files, comps);
                if files <= 1 {
                    assert_eq!(result, Severity::Pass, "files={files}, comps={comps}");
                } else {
                    match comps {
                        0 | 1 => assert_eq!(result, Severity::Pass, "files={files}, comps={comps}"),
                        2 => {
                            assert_eq!(result, Severity::Warning, "files={files}, comps={comps}")
                        }
                        _ => assert_eq!(result, Severity::Error, "files={files}, comps={comps}"),
                    }
                }
            }
        }
    }

    #[test]
    fn classify_file_roles() {
        assert_eq!(
            classify_file_role("packages/runtime-core/src/foo.ts"),
            FileRole::Source
        );
        assert_eq!(
            classify_file_role("packages/runtime-core/__tests__/foo.spec.ts"),
            FileRole::Test
        );
        assert_eq!(
            classify_file_role("packages/runtime-core/__tests__/fixtures/foo.ts"),
            FileRole::Fixture
        );
        assert_eq!(
            classify_file_role("packages-private/vapor-e2e-test/transition/cases/mode/sample.vue"),
            FileRole::Fixture
        );
    }

    #[test]
    fn colocated_source_bridge_requires_long_stem() {
        assert!(should_bridge_colocated_sources(
            "packages/devtools/src/ContextMenu.tsx",
            "packages/devtools/src/ContextMenuItem.tsx"
        ));
        assert!(!should_bridge_colocated_sources(
            "packages/prisma/src/auth.ts",
            "packages/prisma/src/auth-client.ts"
        ));
    }

    #[test]
    fn aux_bridge_with_token_overlap() {
        // Same-dir unit test must NOT bridge
        assert!(!should_bridge_aux_to_source(
            "packages/client/src/mariadb.ts",
            "packages/client/src/mariadb.test.ts",
        ));

        // Same package, different dirs, token overlap → bridge
        assert!(should_bridge_aux_to_source(
            "packages/compiler-vapor/src/generators/expression.ts",
            "packages/compiler-vapor/__tests__/transforms/expression.spec.ts",
        ));

        // Scoped packages in monorepo → bridge
        assert!(should_bridge_aux_to_source(
            "packages/@ember/-internals/glimmer/lib/components/link-to.ts",
            "packages/@ember/-internals/glimmer/tests/integration/components/link-to/routing-angle-test.js",
        ));

        // Same package (compiler), src/ vs test/ → bridge
        assert!(should_bridge_aux_to_source(
            "packages/compiler/src/ml_parser/parser.ts",
            "packages/compiler/test/ml_parser/html_parser_spec.ts",
        ));

        // Cross-package must NOT bridge (different package roots)
        assert!(!should_bridge_aux_to_source(
            "packages/client-engine-runtime/src/query-interpreter.ts",
            "packages/client/tests/functional/issue/tests.ts",
        ));

        // No semantic overlap must NOT bridge
        assert!(!should_bridge_aux_to_source(
            "packages/compiler/src/parser.ts",
            "packages/compiler/test/scheduler.spec.ts",
        ));

        // Cross-package (runtime-core vs packages-private) must NOT bridge
        assert!(!should_bridge_aux_to_source(
            "packages/runtime-core/src/apiDefineComponent.ts",
            "packages-private/dts-test/defineComponent.test-d.ts",
        ));
    }

    #[test]
    fn test_fixture_bridge_uses_semantic_overlap() {
        assert!(should_bridge_test_fixture_pair(
            "packages/vue/__tests__/transition.spec.ts",
            "packages/vue/__tests__/fixtures/transition.html"
        ));
        assert!(!should_bridge_test_fixture_pair(
            "packages/vue/__tests__/alpha.spec.ts",
            "packages/vue/__tests__/fixtures/beta.html"
        ));
    }

    #[test]
    fn fork_variant_bridge_requires_same_family_and_stem() {
        assert!(should_bridge_fork_variants(
            "packages/shared/ReactFeatureFlags.js",
            "packages/shared/forks/ReactFeatureFlags.native-oss.js"
        ));
        assert!(should_bridge_fork_variants(
            "packages/shared/forks/ReactFeatureFlags.test-renderer.js",
            "packages/shared/forks/ReactFeatureFlags.test-renderer.www.js"
        ));
    }

    #[test]
    fn fork_variant_bridge_rejects_broad_over_merge() {
        assert!(!should_bridge_fork_variants(
            "packages/shared/index.js",
            "packages/shared/forks/index.www.js"
        ));
        assert!(!should_bridge_fork_variants(
            "packages/shared/ReactFeatureFlags.js",
            "packages/other/forks/ReactFeatureFlags.native-oss.js"
        ));
        assert!(!should_bridge_fork_variants(
            "packages/shared/ReactFeatureFlags.js",
            "packages/shared/ReactFeatureFlags.native-oss.js"
        ));
    }
}
