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

        let candidates = find_test_pairs(src);
        let covered = candidates
            .iter()
            .any(|candidate| all_changed_files.iter().any(|&f| f == candidate));

        // Also check if any changed test file contains the source stem
        // (looser heuristic for non-standard layouts)
        let stem = src
            .rsplit('/')
            .next()
            .unwrap_or(src)
            .split('.')
            .next()
            .unwrap_or(src);

        let has_related_test = all_changed_files.iter().any(|&f| {
            classify_file_role(f) == FileRole::Test && f.contains(stem) && stem.len() >= 3
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
        let all = &["src/foo.rs"];
        let sources = &["src/foo.rs"];
        let uncovered = has_test_coverage(sources, all);
        assert_eq!(uncovered.len(), 1);
        assert_eq!(uncovered[0].path, "src/foo.rs");
    }

    #[test]
    fn partial_coverage_reports_uncovered_only() {
        let all = &["src/foo.rs", "src/bar.rs", "tests/foo_test.rs"];
        let sources = &["src/foo.rs", "src/bar.rs"];
        let uncovered = has_test_coverage(sources, all);
        assert_eq!(uncovered.len(), 1);
        assert_eq!(uncovered[0].path, "src/bar.rs");
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
        let all = &["src/parser.rs", "src/parser_test.rs"];
        let sources = &["src/parser.rs"];
        // "parser" stem appears in "src/parser_test.rs" which is classified as Test
        let uncovered = has_test_coverage(sources, all);
        assert!(uncovered.is_empty());
    }
}
