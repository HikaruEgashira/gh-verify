//! Test coverage heuristics for PR change analysis.
//!
//! Detects source file changes without corresponding test file changes.
//! Entirely language-agnostic: uses file path conventions only,
//! never inspects file content.

use crate::scope::{FileRole, classify_file_role, is_non_code_file, semantic_path_tokens};

/// A source file that appears to have no matching changed test file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UncoveredSource {
    pub path: String,
}

/// Generate candidate test file paths for a given source file.
///
/// Produces candidates using naming conventions:
/// - `_test` suffix, `test_` prefix (sibling directory)
/// - `tests/` sibling directory variants
/// - `src/tests/` subdirectory variant
///
/// All convention-based, not language-specific.
pub fn find_test_pairs(source_path: &str) -> Vec<String> {
    if classify_file_role(source_path) != FileRole::Source {
        return vec![];
    }

    let file = source_path.rsplit('/').next().unwrap_or(source_path);
    let (stem, ext) = split_stem_ext(file);
    if stem.is_empty() {
        return vec![];
    }

    let ext_suffix = if ext.is_empty() {
        String::new()
    } else {
        format!(".{ext}")
    };
    let source_parent = parent_dir(source_path);

    let mut out = Vec::new();

    // _test suffix in same directory
    push_unique(
        &mut out,
        join_path(source_parent, &format!("{stem}_test{ext_suffix}")),
    );
    // test_ prefix in same directory
    push_unique(
        &mut out,
        join_path(source_parent, &format!("test_{stem}{ext_suffix}")),
    );
    // .test. infix in same directory
    push_unique(
        &mut out,
        join_path(source_parent, &format!("{stem}.test{ext_suffix}")),
    );
    // .spec. infix in same directory
    push_unique(
        &mut out,
        join_path(source_parent, &format!("{stem}.spec{ext_suffix}")),
    );

    // tests/ sibling directory variants
    if let Some((prefix, rel)) = split_src_root(source_path) {
        let rel_parent = parent_dir(rel);
        let tests_root = if prefix.is_empty() {
            "tests".to_string()
        } else {
            format!("{prefix}/tests")
        };
        let src_tests_root = if prefix.is_empty() {
            "src/tests".to_string()
        } else {
            format!("{prefix}/src/tests")
        };

        push_unique(
            &mut out,
            join_path(
                &join_path(&tests_root, rel_parent),
                &format!("{stem}_test{ext_suffix}"),
            ),
        );
        push_unique(
            &mut out,
            join_path(
                &join_path(&tests_root, rel_parent),
                &format!("test_{stem}{ext_suffix}"),
            ),
        );
        push_unique(
            &mut out,
            join_path(
                &join_path(&src_tests_root, rel_parent),
                &format!("{stem}{ext_suffix}"),
            ),
        );
    }

    // __tests__/ sibling directory variant
    push_unique(
        &mut out,
        join_path(
            &join_path(source_parent, "__tests__"),
            &format!("{stem}.test{ext_suffix}"),
        ),
    );
    push_unique(
        &mut out,
        join_path(
            &join_path(source_parent, "__tests__"),
            &format!("{stem}.spec{ext_suffix}"),
        ),
    );

    out
}

/// Check if a test file's stem matches a source file's stem at a word boundary.
///
/// A word boundary means the source stem appears at the START or END of the
/// test stem, separated by `_` (or an exact match). This prevents false
/// coverage when a short stem like "api" appears embedded in "github_api_test".
///
/// # Examples
///
/// - `api_test` matches `api` (prefix match, separator `_`)
/// - `test_api` matches `api` (suffix match, separator `_`)
/// - `api` matches `api` (exact match)
/// - `github_api_test` does NOT match `api` (embedded, no boundary at start)
/// - `github_api_test` matches `github_api` (prefix match)
pub fn stem_matches_at_word_boundary(test_stem: &str, source_stem: &str) -> bool {
    if source_stem.is_empty() || test_stem.is_empty() {
        return false;
    }

    // Exact match
    if test_stem == source_stem {
        return true;
    }

    // Prefix match: test_stem starts with source_stem followed by separator
    if let Some(rest) = test_stem.strip_prefix(source_stem) {
        if rest.starts_with('_') || rest.starts_with('-') || rest.starts_with('.') {
            return true;
        }
    }

    // Suffix match: test_stem ends with source_stem preceded by separator
    if let Some(before) = test_stem.strip_suffix(source_stem) {
        if before.ends_with('_') || before.ends_with('-') || before.ends_with('.') {
            return true;
        }
    }

    false
}

/// Find uncovered source files among changed paths.
///
/// For each source path (caller pre-filters to `FileRole::Source`):
/// 1. Skip if `is_non_code_file()`
/// 2. Skip if `classify_file_role() != FileRole::Source`
/// 3. Check if any test pair candidate from `find_test_pairs()` exists in `all_changed_paths`
/// 4. Check if any changed Test file matches source stem at word boundary (stem >= 3 chars)
/// 5. If neither matched, the source is uncovered
pub fn has_test_coverage(source_paths: &[&str], all_changed_paths: &[&str]) -> Vec<UncoveredSource> {
    let test_paths: Vec<&str> = all_changed_paths
        .iter()
        .copied()
        .filter(|p| classify_file_role(p) == FileRole::Test)
        .collect();

    let mut uncovered = Vec::new();

    for &source in source_paths {
        if is_non_code_file(source) {
            continue;
        }
        if classify_file_role(source) != FileRole::Source {
            continue;
        }

        // Check 1: convention-based test pair candidates
        let candidates = find_test_pairs(source);
        let covered_by_convention = candidates.iter().any(|candidate| {
            let norm_candidate = candidate.to_ascii_lowercase();
            all_changed_paths
                .iter()
                .any(|p| p.to_ascii_lowercase() == norm_candidate)
        });
        if covered_by_convention {
            continue;
        }

        // Check 2: stem-based word boundary matching
        let source_stem = file_stem_lowered(source);
        let covered_by_stem = if source_stem.len() >= 3 {
            test_paths.iter().any(|test_path| {
                let test_stem = file_stem_lowered(test_path);
                stem_matches_at_word_boundary(&test_stem, &source_stem)
            })
        } else {
            false
        };
        if covered_by_stem {
            continue;
        }

        // Check 3: semantic token overlap (for deeply nested / renamed paths)
        let covered_by_semantics = test_paths
            .iter()
            .any(|test| is_semantically_matching_test(source, test));
        if covered_by_semantics {
            continue;
        }

        uncovered.push(UncoveredSource {
            path: source.to_string(),
        });
    }

    uncovered
}

/// Core predicate for coverage: a source is covered when at least one
/// matching test is found.
/// Verified by Creusot in `gh-verify-verif` crate.
pub fn is_covered(matching_test_count: usize) -> bool {
    matching_test_count > 0
}

// --- Internal helpers ---

fn split_stem_ext(file: &str) -> (&str, &str) {
    if let Some((stem, ext)) = file.rsplit_once('.') {
        (stem, ext)
    } else {
        (file, "")
    }
}

fn split_src_root(path: &str) -> Option<(String, &str)> {
    if let Some(rest) = path.strip_prefix("src/") {
        return Some((String::new(), rest));
    }
    path.split_once("/src/")
        .map(|(prefix, rest)| (prefix.to_string(), rest))
}

fn parent_dir(path: &str) -> &str {
    path.rsplit_once('/').map(|(p, _)| p).unwrap_or("")
}

fn join_path(parent: &str, child: &str) -> String {
    if parent.is_empty() {
        return child.to_string();
    }
    if child.is_empty() {
        return parent.to_string();
    }
    format!("{parent}/{child}")
}

fn push_unique(out: &mut Vec<String>, value: String) {
    if !out.contains(&value) {
        out.push(value);
    }
}

fn file_stem_lowered(path: &str) -> String {
    let file = path.rsplit('/').next().unwrap_or(path);
    let (stem, _) = split_stem_ext(file);
    // Strip test markers from stem to get bare name
    let bare = stem
        .strip_suffix("_test")
        .or_else(|| stem.strip_suffix(".test"))
        .or_else(|| stem.strip_suffix(".spec"))
        .or_else(|| stem.strip_prefix("test_"))
        .unwrap_or(stem);
    bare.to_ascii_lowercase()
}

fn is_semantically_matching_test(source_path: &str, test_path: &str) -> bool {
    if classify_file_role(test_path) != FileRole::Test {
        return false;
    }

    let source_tokens = semantic_path_tokens(source_path);
    let test_tokens: std::collections::HashSet<String> =
        semantic_path_tokens(test_path).into_iter().collect();

    source_tokens
        .iter()
        .any(|token| token.len() >= 5 && !is_generic_token(token) && test_tokens.contains(token))
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

    // --- stem_matches_at_word_boundary ---

    #[test]
    fn stem_boundary_matching() {
        // Prefix: source stem at start, followed by separator
        assert!(stem_matches_at_word_boundary("api_test", "api"));
        // Suffix: source stem at end, preceded by separator
        assert!(stem_matches_at_word_boundary("test_api", "api"));
        // Exact match
        assert!(stem_matches_at_word_boundary("api", "api"));
        // Compound prefix
        assert!(stem_matches_at_word_boundary("github_api_test", "github_api"));
    }

    /// WHY: "api" must NOT match "github_api_test" -- the source stem is embedded
    /// in the middle, not at a word boundary. This prevents false coverage signals
    /// for common short stems.
    #[test]
    fn substring_stem_rejected() {
        assert!(!stem_matches_at_word_boundary("github_api_test", "api"));
    }

    #[test]
    fn stem_boundary_empty_inputs() {
        assert!(!stem_matches_at_word_boundary("", "api"));
        assert!(!stem_matches_at_word_boundary("api", ""));
        assert!(!stem_matches_at_word_boundary("", ""));
    }

    // --- find_test_pairs ---

    #[test]
    fn find_test_pairs_for_src_file() {
        let pairs = find_test_pairs("src/foo.rs");
        assert!(pairs.contains(&"src/foo_test.rs".to_string()));
        assert!(pairs.contains(&"src/test_foo.rs".to_string()));
        assert!(pairs.contains(&"src/foo.test.rs".to_string()));
        assert!(pairs.contains(&"src/foo.spec.rs".to_string()));
        assert!(pairs.contains(&"tests/foo_test.rs".to_string()));
        assert!(pairs.contains(&"src/tests/foo.rs".to_string()));
    }

    /// Verify find_test_pairs generates expected candidates for nested paths.
    #[test]
    fn find_test_pairs_nested_path() {
        let pairs = find_test_pairs("crates/core/src/scope.rs");
        assert!(pairs.contains(&"crates/core/src/scope_test.rs".to_string()));
        assert!(pairs.contains(&"crates/core/tests/scope_test.rs".to_string()));
        assert!(pairs.contains(&"crates/core/src/tests/scope.rs".to_string()));
    }

    #[test]
    fn find_test_pairs_returns_empty_for_test_file() {
        let pairs = find_test_pairs("src/foo_test.rs");
        assert!(pairs.is_empty());
    }

    // --- has_test_coverage ---

    /// Property: has_test_coverage returns empty iff all source files have matching tests.
    #[test]
    fn coverage_biconditional() {
        // Forward: test pair exists -> covered
        let uncovered = has_test_coverage(&["src/foo.ts"], &["src/foo.ts", "src/foo.test.ts"]);
        assert!(uncovered.is_empty());

        // Backward (contrapositive): no test pair -> uncovered
        let uncovered = has_test_coverage(&["src/foo.ts"], &["src/foo.ts"]);
        assert_eq!(uncovered.len(), 1);
    }

    /// WHY: Config-only changes (.github/workflows/) should not trigger
    /// test coverage warnings. Non-code files are skipped even if passed
    /// as source_paths.
    #[test]
    fn non_code_files_skipped() {
        let uncovered = has_test_coverage(
            &[".github/workflows/ci.yml", "README.md"],
            &[".github/workflows/ci.yml", "README.md"],
        );
        assert!(uncovered.is_empty());
    }

    /// WHY: When only test files change, there is nothing to flag.
    #[test]
    fn test_only_changes_pass() {
        let uncovered = has_test_coverage(&[], &["tests/foo_test.rs", "src/bar.test.ts"]);
        assert!(uncovered.is_empty());
    }

    /// Real-world: partial coverage reports only the uncovered file.
    #[test]
    fn partial_coverage_reports_uncovered_only() {
        let uncovered = has_test_coverage(
            &["src/foo.rs", "src/bar.rs"],
            &["src/foo.rs", "src/bar.rs", "tests/foo_test.rs"],
        );
        assert_eq!(uncovered.len(), 1);
        assert_eq!(uncovered[0].path, "src/bar.rs");
    }

    /// WHY: Semantic matching covers deeply nested paths with shared tokens
    /// like apiDefineComponent.ts matched by apiDefineComponent.spec.ts
    #[test]
    fn semantic_fallback_matches_named_test() {
        let uncovered = has_test_coverage(
            &["packages/runtime-core/src/apiDefineComponent.ts"],
            &[
                "packages/runtime-core/src/apiDefineComponent.ts",
                "packages/runtime-core/__tests__/apiDefineComponent.spec.ts",
            ],
        );
        assert!(uncovered.is_empty());
    }

    /// WHY: Generic token overlap (index, utils) must not create false coverage.
    #[test]
    fn semantic_fallback_rejects_generic_test_name() {
        let uncovered = has_test_coverage(
            &["src/auth.rs"],
            &["src/auth.rs", "tests/index_test.rs"],
        );
        assert_eq!(uncovered.len(), 1);
    }

    // --- is_covered predicate ---

    #[test]
    fn is_covered_predicate() {
        assert!(!is_covered(0));
        assert!(is_covered(1));
        assert!(is_covered(42));
    }
}
