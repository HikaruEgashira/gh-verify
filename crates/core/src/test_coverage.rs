//! Test coverage heuristics for PR change analysis.
//!
//! Given a set of changed files, identifies source files that lack
//! corresponding test file changes — a signal that tests may be missing.
//!
//! Two coverage signals:
//! 1. **Inline test markers**: the file's own patch contains test constructs
//!    (`#[test]`, `def test_`, etc.) — set by the CLI layer.
//! 2. **External test file**: a companion test file was also changed.

use std::collections::HashSet;

use crate::scope::{FileRole, classify_file_role, semantic_path_tokens};

/// A source file with pre-computed patch analysis.
/// The CLI layer inspects patch content and sets `patch_contains_test`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceFile {
    pub path: String,
    /// Whether the unified diff patch for this file contains test markers
    /// (e.g. `#[test]`, `#[cfg(test)]`, `def test_`).
    pub patch_contains_test: bool,
}

/// A source file that has no corresponding test file in the changeset.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UncoveredSource {
    pub source_path: String,
    pub suggested_test_paths: Vec<String>,
}

/// Core predicate: a source file has test coverage iff either
/// its own patch contains test markers OR an external test file matches.
///
/// Verified by Creusot in `gh-verify-verif` crate.
pub fn has_coverage(patch_has_test: bool, external_test_found: bool) -> bool {
    patch_has_test || external_test_found
}

/// Generate likely companion test file paths for a source file.
pub fn find_test_pair(source_path: &str) -> Vec<String> {
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

    // Colocated test files: foo_test.rs, test_foo.rs
    push_unique(
        &mut out,
        join_path(source_parent, &format!("{stem}_test{ext_suffix}")),
    );
    push_unique(
        &mut out,
        join_path(source_parent, &format!("test_{stem}{ext_suffix}")),
    );

    // JS/TS conventions: foo.test.ts, foo.spec.ts
    push_unique(
        &mut out,
        join_path(source_parent, &format!("{stem}.test{ext_suffix}")),
    );
    push_unique(
        &mut out,
        join_path(source_parent, &format!("{stem}.spec{ext_suffix}")),
    );

    // __tests__/ directory
    push_unique(
        &mut out,
        join_path(
            &join_path(source_parent, "__tests__"),
            &format!("{stem}{ext_suffix}"),
        ),
    );

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

    out
}

/// Check whether a test file stem matches a source file stem at a word boundary.
///
/// Prevents "api" from matching "github_api_test" (embedded match).
/// Only exact, prefix-with-separator, or suffix-with-separator matches count.
pub fn stem_matches_at_word_boundary(test_stem: &str, source_stem: &str) -> bool {
    if test_stem == source_stem {
        return true;
    }
    if source_stem.is_empty() || test_stem.is_empty() {
        return false;
    }

    // Check if source_stem appears in test_stem at a word boundary
    let separators = ['_', '-', '.'];

    for (pos, _) in test_stem.match_indices(source_stem) {
        let at_start = pos == 0;
        let at_end = pos + source_stem.len() == test_stem.len();
        let preceded_by_sep =
            pos > 0 && separators.contains(&(test_stem.as_bytes()[pos - 1] as char));
        let followed_by_sep = pos + source_stem.len() < test_stem.len()
            && separators.contains(&(test_stem.as_bytes()[pos + source_stem.len()] as char));

        if (at_start || preceded_by_sep) && (at_end || followed_by_sep) {
            return true;
        }
    }

    false
}

/// Return uncovered source files that lack both inline tests and external test files.
///
/// A source is covered if:
/// 1. Its patch contains test markers (`source.patch_contains_test == true`), OR
/// 2. A companion test file exists in the changeset (by convention or semantics).
pub fn has_test_coverage(
    sources: &[SourceFile],
    all_changed_files: &[&str],
) -> Vec<UncoveredSource> {
    let test_files: Vec<&str> = all_changed_files
        .iter()
        .copied()
        .filter(|p| classify_file_role(p) == FileRole::Test)
        .collect();

    let normalized_tests: HashSet<String> = test_files
        .iter()
        .map(|p| normalize_path_for_match(p))
        .collect();

    let mut uncovered = Vec::new();

    for source in sources {
        if classify_file_role(&source.path) != FileRole::Source {
            continue;
        }

        // Signal 1: patch itself contains test markers
        if has_coverage(source.patch_contains_test, false) {
            continue;
        }

        // Signal 2: external test file matches by convention
        let suggestions = find_test_pair(&source.path);
        let covered_by_convention = suggestions
            .iter()
            .any(|candidate| normalized_tests.contains(&normalize_path_for_match(candidate)));

        if has_coverage(false, covered_by_convention) {
            continue;
        }

        // Signal 2b: semantic stem matching with word-boundary guard
        let covered_by_semantics = test_files
            .iter()
            .any(|test| is_semantically_matching_test(&source.path, test));

        if has_coverage(false, covered_by_semantics) {
            continue;
        }

        uncovered.push(UncoveredSource {
            source_path: source.path.clone(),
            suggested_test_paths: suggestions,
        });
    }

    uncovered
}

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

fn normalize_path_for_match(path: &str) -> String {
    path.to_ascii_lowercase()
}

fn normalized_file_stem(path: &str) -> String {
    let file = path.rsplit('/').next().unwrap_or(path);
    let (stem, _) = split_stem_ext(file);
    stem.chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect()
}

fn is_semantically_matching_test(source_path: &str, test_path: &str) -> bool {
    if classify_file_role(test_path) != FileRole::Test {
        return false;
    }

    let source_stem = normalized_file_stem(source_path);
    let test_stem = normalized_file_stem(test_path);

    // Word-boundary stem matching prevents false coverage
    if source_stem.len() >= 5
        && !is_generic_token(&source_stem)
        && stem_matches_at_word_boundary(&test_stem, &source_stem)
    {
        return true;
    }

    let source_tokens = semantic_path_tokens(source_path);
    let test_tokens: HashSet<String> = semantic_path_tokens(test_path).into_iter().collect();

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

    fn make_source(path: &str, patch_has_test: bool) -> SourceFile {
        SourceFile {
            path: path.to_string(),
            patch_contains_test: patch_has_test,
        }
    }

    // --- has_coverage predicate ---

    /// Property: has_coverage returns true iff at least one coverage source exists.
    /// This directly tests the biconditional in the Creusot spec.
    #[test]
    fn coverage_biconditional() {
        assert!(has_coverage(true, false));
        assert!(has_coverage(false, true));
        assert!(has_coverage(true, true));
        assert!(!has_coverage(false, false));
    }

    // --- has_test_coverage integration ---

    /// WHY: A file whose patch touches #[cfg(test)] is self-tested — the developer
    /// modified both production and test code in the same file.
    #[test]
    fn self_tested_source_is_covered() {
        let sources = vec![make_source("src/foo.rs", true)];
        let all_files: Vec<&str> = vec!["src/foo.rs"];
        let uncovered = has_test_coverage(&sources, &all_files);
        assert!(
            uncovered.is_empty(),
            "self-tested source should not be flagged"
        );
    }

    /// WHY: Modified production code with no test changes should be flagged.
    /// This is the primary use case for the rule.
    #[test]
    fn source_without_test_changes_is_uncovered() {
        let sources = vec![make_source("src/foo.rs", false)];
        let all_files: Vec<&str> = vec!["src/foo.rs"];
        let uncovered = has_test_coverage(&sources, &all_files);
        assert_eq!(uncovered.len(), 1);
        assert_eq!(uncovered[0].source_path, "src/foo.rs");
    }

    /// WHY: This repo's own scope.rs has inline tests. If the patch adds a new
    /// #[test] fn, it should be considered self-tested.
    #[test]
    fn real_world_this_repo_scope_rs() {
        let sources = vec![make_source("crates/core/src/scope.rs", true)];
        let all_files: Vec<&str> = vec!["crates/core/src/scope.rs"];
        let uncovered = has_test_coverage(&sources, &all_files);
        assert!(
            uncovered.is_empty(),
            "scope.rs with inline test changes is self-tested"
        );
    }

    /// WHY: When an external test file pairs with the source, the source is covered
    /// even without inline test markers.
    #[test]
    fn external_test_file_covers_source() {
        let sources = vec![make_source("src/foo.rs", false)];
        let all_files: Vec<&str> = vec!["src/foo.rs", "tests/foo_test.rs"];
        let uncovered = has_test_coverage(&sources, &all_files);
        assert!(
            uncovered.is_empty(),
            "source with companion test file should be covered"
        );
    }

    /// WHY: Test-only changes should never produce warnings.
    /// If no source files are in the changeset, the rule has nothing to check.
    #[test]
    fn test_only_changes_pass() {
        let sources: Vec<SourceFile> = vec![];
        let all_files: Vec<&str> = vec!["tests/foo_test.rs"];
        let uncovered = has_test_coverage(&sources, &all_files);
        assert!(uncovered.is_empty());
    }

    /// WHY: Non-code files (markdown, config) should be excluded from coverage checks.
    /// Flagging them would produce noise on documentation PRs.
    #[test]
    fn non_source_files_are_skipped() {
        // classify_file_role classifies test files as Test, not Source
        let sources = vec![make_source("tests/foo_test.rs", false)];
        let all_files: Vec<&str> = vec!["tests/foo_test.rs"];
        let uncovered = has_test_coverage(&sources, &all_files);
        assert!(
            uncovered.is_empty(),
            "test files should not be flagged as uncovered sources"
        );
    }

    // --- stem_matches_at_word_boundary ---

    /// WHY: "api" should NOT match "githubapi" — different semantic scope.
    /// Only word-boundary matches (api_test, test_api) should count.
    #[test]
    fn substring_stem_match_rejected() {
        assert!(
            !stem_matches_at_word_boundary("githubapi", "api"),
            "embedded match without separator should be rejected"
        );
        assert!(
            !stem_matches_at_word_boundary("myapihandler", "api"),
            "middle embedded match should be rejected"
        );
    }

    #[test]
    fn word_boundary_match_accepted() {
        assert!(stem_matches_at_word_boundary("api_test", "api"));
        assert!(stem_matches_at_word_boundary("test_api", "api"));
        assert!(stem_matches_at_word_boundary("api", "api"));
        assert!(stem_matches_at_word_boundary("api-handler", "api"));
    }

    // --- find_test_pair ---

    #[test]
    fn find_test_pair_for_src_file() {
        let pairs = find_test_pair("src/foo.rs");
        assert!(pairs.contains(&"tests/foo_test.rs".to_string()));
        assert!(pairs.contains(&"src/foo_test.rs".to_string()));
        assert!(pairs.contains(&"tests/test_foo.rs".to_string()));
        assert!(pairs.contains(&"src/tests/foo.rs".to_string()));
    }

    #[test]
    fn find_test_pair_includes_js_conventions() {
        let pairs = find_test_pair("src/utils.ts");
        assert!(pairs.contains(&"src/utils.test.ts".to_string()));
        assert!(pairs.contains(&"src/utils.spec.ts".to_string()));
        assert!(pairs.contains(&"src/__tests__/utils.ts".to_string()));
    }

    #[test]
    fn find_test_pair_for_nested_workspace_source() {
        let pairs = find_test_pair("crates/core/src/scope.rs");
        assert!(pairs.contains(&"crates/core/tests/scope_test.rs".to_string()));
        assert!(pairs.contains(&"crates/core/src/scope_test.rs".to_string()));
        assert!(pairs.contains(&"crates/core/tests/test_scope.rs".to_string()));
        assert!(pairs.contains(&"crates/core/src/tests/scope.rs".to_string()));
    }

    // --- semantic matching ---

    #[test]
    fn semantic_fallback_matches_named_test() {
        let sources = vec![make_source(
            "packages/runtime-core/src/apiDefineComponent.ts",
            false,
        )];
        let all_files: Vec<&str> = vec![
            "packages/runtime-core/src/apiDefineComponent.ts",
            "packages/runtime-core/__tests__/apiDefineComponent.spec.ts",
        ];
        let uncovered = has_test_coverage(&sources, &all_files);
        assert!(uncovered.is_empty());
    }

    #[test]
    fn semantic_fallback_rejects_generic_test_name() {
        let sources = vec![make_source("src/auth.rs", false)];
        let all_files: Vec<&str> = vec!["src/auth.rs", "tests/index_test.rs"];
        let uncovered = has_test_coverage(&sources, &all_files);
        assert_eq!(uncovered.len(), 1);
    }
}
