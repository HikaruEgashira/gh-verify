//! Test coverage heuristics for PR change analysis.
//!
//! Given a set of changed files, identifies source files that lack
//! corresponding test file changes — a signal that tests may be missing.

use crate::scope::{classify_file_role, is_non_code_file, FileRole};

/// A source file that has no corresponding test file in the changeset.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UncoveredSource {
    pub path: String,
}

/// Generate candidate test file paths for a given source file.
///
/// Produces language-independent naming patterns such as:
/// - `tests/foo_test.rs` (sibling tests/ dir with `_test` suffix)
/// - `src/foo_test.rs` (colocated with `_test` suffix)
/// - `tests/test_foo.rs` (sibling tests/ dir with `test_` prefix)
/// - `src/tests/foo.rs` (nested tests/ subdir)
pub fn find_test_pairs(source_path: &str) -> Vec<String> {
    let mut candidates = Vec::new();

    // Extract directory, stem, and extension
    let (dir, filename) = match source_path.rsplit_once('/') {
        Some((d, f)) => (d, f),
        None => ("", source_path),
    };

    let (stem, ext) = match filename.rsplit_once('.') {
        Some((s, e)) => (s, format!(".{e}")),
        None => (filename, String::new()),
    };

    // Parent of source dir (e.g. "crates/cli/src" -> "crates/cli")
    let parent_dir = dir.rsplit_once('/').map(|(p, _)| p).unwrap_or("");

    // 1. tests/<stem>_test<ext> — sibling tests/ directory
    if !parent_dir.is_empty() {
        candidates.push(format!("{parent_dir}/tests/{stem}_test{ext}"));
    } else {
        candidates.push(format!("tests/{stem}_test{ext}"));
    }

    // 2. <dir>/<stem>_test<ext> — colocated _test suffix
    if !dir.is_empty() {
        candidates.push(format!("{dir}/{stem}_test{ext}"));
    } else {
        candidates.push(format!("{stem}_test{ext}"));
    }

    // 3. tests/test_<stem><ext> — sibling tests/ with test_ prefix
    if !parent_dir.is_empty() {
        candidates.push(format!("{parent_dir}/tests/test_{stem}{ext}"));
    } else {
        candidates.push(format!("tests/test_{stem}{ext}"));
    }

    // 4. <dir>/tests/<stem><ext> — nested tests/ subdir
    if !dir.is_empty() {
        candidates.push(format!("{dir}/tests/{stem}{ext}"));
    } else {
        candidates.push(format!("tests/{stem}{ext}"));
    }

    // 5. Common JS/TS patterns: <stem>.test<ext>, <stem>.spec<ext>
    if !dir.is_empty() {
        candidates.push(format!("{dir}/{stem}.test{ext}"));
        candidates.push(format!("{dir}/{stem}.spec{ext}"));
    } else {
        candidates.push(format!("{stem}.test{ext}"));
        candidates.push(format!("{stem}.spec{ext}"));
    }

    // 6. __tests__/<stem><ext> pattern (JS/TS convention)
    if !parent_dir.is_empty() {
        candidates.push(format!("{parent_dir}/__tests__/{stem}{ext}"));
    }

    candidates
}

/// Extensions for languages that support inline tests (`#[cfg(test)]`, `if __name__`).
const INLINE_TEST_EXTENSIONS: &[&str] = &[".rs", ".py"];

/// Check whether `ext` supports inline tests.
fn supports_inline_tests(path: &str) -> bool {
    INLINE_TEST_EXTENSIONS.iter().any(|ext| path.ends_with(ext))
}

/// Check whether the test file stem starts or ends with the source stem
/// at a word boundary (delimited by `_`, `-`, or string edge).
///
/// This prevents `api` from matching `github_api_test` (embedded match),
/// while allowing `api` to match `api_test` or `test_api`.
fn stem_matches_at_word_boundary(test_file_stem: &str, source_stem: &str) -> bool {
    if source_stem.is_empty() {
        return false;
    }
    // Exact match
    if test_file_stem == source_stem {
        return true;
    }
    // Prefix match: test stem starts with source_stem followed by separator
    if let Some(rest) = test_file_stem.strip_prefix(source_stem) {
        if rest.starts_with('_') || rest.starts_with('-') || rest.starts_with('.') {
            return true;
        }
    }
    // Suffix match: test stem ends with source_stem preceded by separator
    if let Some(rest) = test_file_stem.strip_suffix(source_stem) {
        if rest.ends_with('_') || rest.ends_with('-') || rest.ends_with('.') {
            return true;
        }
    }
    false
}

/// Extract the file stem (filename without extension) from a path.
fn file_stem(path: &str) -> &str {
    let filename = path.rsplit('/').next().unwrap_or(path);
    filename.split('.').next().unwrap_or(filename)
}

/// Check which source files in a changeset lack corresponding test file changes.
///
/// For each file classified as `Source` (and not non-code), checks whether any
/// of its test pair candidates appear in `all_changed_files`. Returns the list
/// of source files with no matching test.
pub fn has_test_coverage(
    source_files: &[&str],
    all_changed_files: &[&str],
) -> Vec<UncoveredSource> {
    let mut uncovered = Vec::new();

    for &src in source_files {
        // Skip non-code files
        if is_non_code_file(src) {
            continue;
        }
        // Only check Source files
        if classify_file_role(src) != FileRole::Source {
            continue;
        }

        // Languages with inline test support (Rust #[cfg(test)], Python
        // if __name__) are self-tested when the source file itself is changed.
        if supports_inline_tests(src) && all_changed_files.contains(&src) {
            continue;
        }

        let candidates = find_test_pairs(src);
        let covered = candidates
            .iter()
            .any(|candidate| all_changed_files.iter().any(|&f| f == candidate));

        // Also check if any changed test file contains the source stem
        // at a word boundary (stricter heuristic for non-standard layouts)
        let stem = file_stem(src);

        let has_related_test = all_changed_files.iter().any(|&f| {
            if classify_file_role(f) != FileRole::Test || stem.len() < 3 {
                return false;
            }
            let test_stem = file_stem(f);
            stem_matches_at_word_boundary(test_stem, stem)
        });

        if !covered && !has_related_test {
            uncovered.push(UncoveredSource {
                path: src.to_string(),
            });
        }
    }

    uncovered
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn find_test_pairs_generates_expected_candidates() {
        let pairs = find_test_pairs("src/foo.rs");
        assert!(pairs.contains(&"tests/foo_test.rs".to_string()));
        assert!(pairs.contains(&"src/foo_test.rs".to_string()));
        assert!(pairs.contains(&"tests/test_foo.rs".to_string()));
        assert!(pairs.contains(&"src/tests/foo.rs".to_string()));
    }

    #[test]
    fn find_test_pairs_nested_path() {
        let pairs = find_test_pairs("crates/cli/src/rules/engine.rs");
        assert!(pairs.contains(&"crates/cli/src/rules/engine_test.rs".to_string()));
        assert!(pairs.contains(&"crates/cli/src/rules/engine.test.rs".to_string()));
        assert!(pairs.contains(&"crates/cli/src/rules/tests/engine.rs".to_string()));
    }

    #[test]
    fn source_with_test_pair_is_covered() {
        let all = &["src/foo.rs", "tests/foo_test.rs"];
        let sources = &["src/foo.rs"];
        let uncovered = has_test_coverage(sources, all);
        assert!(uncovered.is_empty());
    }

    #[test]
    fn source_without_test_is_uncovered() {
        let all = &["src/foo.ts"];
        let sources = &["src/foo.ts"];
        let uncovered = has_test_coverage(sources, all);
        assert_eq!(uncovered.len(), 1);
        assert_eq!(uncovered[0].path, "src/foo.ts");
    }

    #[test]
    fn partial_coverage_reports_uncovered_only() {
        let all = &["src/foo.ts", "src/bar.ts", "src/foo.test.ts"];
        let sources = &["src/foo.ts", "src/bar.ts"];
        let uncovered = has_test_coverage(sources, all);
        assert_eq!(uncovered.len(), 1);
        assert_eq!(uncovered[0].path, "src/bar.ts");
    }

    #[test]
    fn non_code_files_are_skipped() {
        let all = &["README.md"];
        let sources = &["README.md"];
        let uncovered = has_test_coverage(sources, all);
        assert!(uncovered.is_empty());
    }

    #[test]
    fn test_only_changes_produce_no_uncovered() {
        let all = &["tests/foo_test.rs"];
        let sources: &[&str] = &[];
        let uncovered = has_test_coverage(sources, all);
        assert!(uncovered.is_empty());
    }

    #[test]
    fn config_files_are_skipped() {
        let all = &[".github/workflows/ci.yml"];
        let sources = &[".github/workflows/ci.yml"];
        let uncovered = has_test_coverage(sources, all);
        assert!(uncovered.is_empty());
    }

    #[test]
    fn related_test_with_stem_match_counts_as_covered() {
        // A test file that contains the source stem in its path
        let all = &["src/parser.ts", "src/parser_test.ts"];
        let sources = &["src/parser.ts"];
        // "parser" stem appears in "src/parser_test.ts" which is classified as Test
        let uncovered = has_test_coverage(sources, all);
        assert!(uncovered.is_empty());
    }

    #[test]
    fn rust_inline_test_is_self_covered() {
        // Rust files with #[cfg(test)] inline tests are self-tested
        let all = &["src/lib.rs"];
        let sources = &["src/lib.rs"];
        let uncovered = has_test_coverage(sources, all);
        assert!(uncovered.is_empty(), "Rust files should be self-tested");
    }

    #[test]
    fn python_inline_test_is_self_covered() {
        // Python files can have `if __name__` inline tests
        let all = &["src/main.py"];
        let sources = &["src/main.py"];
        let uncovered = has_test_coverage(sources, all);
        assert!(uncovered.is_empty(), "Python files should be self-tested");
    }

    #[test]
    fn substring_stem_match_does_not_false_positive() {
        // "api" should NOT match "github_api_test" — different semantic scope
        let all = &["src/api.ts", "tests/github_api_test.ts"];
        let sources = &["src/api.ts"];
        let uncovered = has_test_coverage(sources, all);
        assert_eq!(uncovered.len(), 1, "api should not match github_api_test");
        assert_eq!(uncovered[0].path, "src/api.ts");
    }

    #[test]
    fn exact_stem_boundary_match_works() {
        // "api" should match "api_test" (word boundary match)
        let all = &["src/api.ts", "tests/api_test.ts"];
        let sources = &["src/api.ts"];
        let uncovered = has_test_coverage(sources, all);
        assert!(uncovered.is_empty(), "api should match api_test");
    }

    #[test]
    fn stem_boundary_function() {
        // Prefix: source stem at the start of test stem
        assert!(stem_matches_at_word_boundary("api_test", "api"));
        // Suffix: source stem at the end of test stem
        assert!(stem_matches_at_word_boundary("test_api", "api"));
        // Exact match
        assert!(stem_matches_at_word_boundary("api", "api"));
        // Embedded: api in the middle — should NOT match
        assert!(!stem_matches_at_word_boundary("github_api_test", "api"));
        // Prefix of compound stem
        assert!(stem_matches_at_word_boundary("github_api_test", "github_api"));
    }
}
