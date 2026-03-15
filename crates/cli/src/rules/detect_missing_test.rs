use anyhow::Result;
use gh_verify_core::scope::{FileRole, classify_file_role, is_non_code_file};
use gh_verify_core::test_coverage::{SourceFile, has_test_coverage};
use gh_verify_core::verdict::{RuleResult, Severity};

use super::{Rule, RuleContext};

const RULE_ID: &str = "detect-missing-test";

pub struct DetectMissingTest;

/// Inspect unified diff patch content for test markers.
///
/// Only added lines (`+` prefix) count — removing `#[test]` functions
/// reduces coverage and should NOT be treated as self-testing.
fn patch_contains_test_markers(patch: &str) -> bool {
    for line in patch.lines() {
        // Only inspect added lines ('+' prefix in unified diff)
        if !line.starts_with('+') {
            continue;
        }
        let trimmed = line[1..].trim();
        // Rust
        if trimmed.contains("#[test]")
            || trimmed.contains("#[cfg(test)]")
            || trimmed.starts_with("mod tests")
        {
            return true;
        }
        // Python
        if trimmed.starts_with("def test_")
            || trimmed.contains("import pytest")
            || trimmed.contains("import unittest")
        {
            return true;
        }
        // JS/TS
        if trimmed.starts_with("describe(")
            || trimmed.starts_with("it(")
            || trimmed.starts_with("test(")
        {
            return true;
        }
        // Go
        if trimmed.starts_with("func Test") {
            return true;
        }
    }
    false
}

impl Rule for DetectMissingTest {
    fn id(&self) -> &'static str {
        RULE_ID
    }

    fn run(&self, ctx: &RuleContext) -> Result<Vec<RuleResult>> {
        let (pr_files, options) = match ctx {
            RuleContext::Pr {
                pr_files, options, ..
            } => (pr_files, options),
            RuleContext::Release { .. } => return Ok(vec![]),
        };

        if !options.detect_missing_test {
            return Ok(vec![]);
        }

        // Collect all changed code files (excluding removed and non-code)
        let changed_code_files: Vec<&str> = pr_files
            .iter()
            .filter(|f| f.patch.is_some())
            .filter(|f| f.status != "removed")
            .filter(|f| !is_non_code_file(&f.filename))
            .map(|f| f.filename.as_str())
            .collect();

        // Build SourceFile structs with patch_contains_test pre-computed
        let source_files: Vec<SourceFile> = pr_files
            .iter()
            .filter(|f| f.patch.is_some())
            .filter(|f| f.status != "removed")
            .filter(|f| !is_non_code_file(&f.filename))
            .filter(|f| classify_file_role(&f.filename) == FileRole::Source)
            .map(|f| SourceFile {
                path: f.filename.clone(),
                patch_contains_test: f
                    .patch
                    .as_deref()
                    .map(patch_contains_test_markers)
                    .unwrap_or(false),
            })
            .collect();

        if source_files.is_empty() {
            return Ok(vec![pass_result()]);
        }

        let mut uncovered = has_test_coverage(&source_files, &changed_code_files);
        if !options.test_patterns.is_empty() {
            let test_files: Vec<&str> = changed_code_files
                .iter()
                .copied()
                .filter(|p| classify_file_role(p) == FileRole::Test)
                .collect();
            uncovered.retain(|entry| {
                !is_covered_by_custom_patterns(
                    &entry.source_path,
                    &test_files,
                    &options.test_patterns,
                )
            });
        }

        if uncovered.is_empty() {
            return Ok(vec![pass_result()]);
        }

        let affected_files: Vec<String> = uncovered.iter().map(|u| u.source_path.clone()).collect();
        let mut suggestion_lines = Vec::new();
        for entry in &uncovered {
            let first_candidate = entry
                .suggested_test_paths
                .first()
                .cloned()
                .unwrap_or_else(|| "(no candidate)".to_string());
            suggestion_lines.push(format!("{} -> {}", entry.source_path, first_candidate));
        }

        Ok(vec![RuleResult {
            rule_id: RULE_ID.to_string(),
            severity: Severity::Warning,
            message: format!(
                "{} source file(s) changed without matching test updates",
                uncovered.len()
            ),
            affected_files,
            suggestion: Some(format!(
                "Consider updating tests for:\n{}",
                suggestion_lines.join("\n")
            )),
        }])
    }
}

fn pass_result() -> RuleResult {
    RuleResult::pass(RULE_ID, "Source changes include matching test updates")
}

fn is_covered_by_custom_patterns(
    source_path: &str,
    test_files: &[&str],
    patterns: &[String],
) -> bool {
    let source_stem = normalized_file_stem(source_path);
    if source_stem.is_empty() {
        return false;
    }

    patterns.iter().any(|pattern| {
        let candidate = pattern.replace('*', &source_stem);
        test_files.iter().any(|test| test.ends_with(&candidate))
    })
}

fn normalized_file_stem(path: &str) -> String {
    let file = path.rsplit('/').next().unwrap_or(path);
    let stem = file.split('.').next().unwrap_or(file);
    stem.chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::github::types::{PrFile, PrMetadata};
    use crate::rules::{PrRuleOptions, RuleContext};

    // --- patch_contains_test_markers ---

    /// WHY: Only added lines in the patch should count as test markers.
    /// Removing #[test] functions reduces coverage and should NOT be treated as self-testing.
    #[test]
    fn removed_test_lines_do_not_count() {
        assert!(!patch_contains_test_markers("-    #[test]"));
        assert!(!patch_contains_test_markers("-#[cfg(test)]"));
        assert!(!patch_contains_test_markers("-def test_something():"));
    }

    /// WHY: patch_contains_test_markers should detect common test patterns
    /// across Rust, Python, JS/TS, and Go.
    #[test]
    fn test_marker_detection() {
        // Rust
        assert!(patch_contains_test_markers("+    #[test]"));
        assert!(patch_contains_test_markers("+#[cfg(test)]"));
        assert!(patch_contains_test_markers("+mod tests {"));
        // Python
        assert!(patch_contains_test_markers("+def test_something():"));
        assert!(patch_contains_test_markers("+import pytest"));
        assert!(patch_contains_test_markers("+import unittest"));
        // JS/TS
        assert!(patch_contains_test_markers("+describe('foo', () => {"));
        assert!(patch_contains_test_markers("+it('should work', () => {"));
        assert!(patch_contains_test_markers("+test('addition', () => {"));
        // Go
        assert!(patch_contains_test_markers("+func TestFoo(t *testing.T) {"));
        // Non-test lines
        assert!(!patch_contains_test_markers("+fn regular_function() {}"));
        assert!(!patch_contains_test_markers("+let x = 42;"));
        // Context lines (no + prefix)
        assert!(!patch_contains_test_markers(" #[test]"));
    }

    /// WHY: Files with status "removed" should be excluded from the check entirely.
    /// Deleted files cannot have missing tests.
    #[test]
    fn deleted_file_excluded() {
        let ctx = RuleContext::Pr {
            pr_files: vec![removed_file("src/foo.rs")],
            pr_metadata: metadata(),
            options: PrRuleOptions::default(),
        };

        let results = DetectMissingTest.run(&ctx).expect("rule should run");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].severity, Severity::Pass);
    }

    /// WHY: Source files whose patch contains #[test] should be treated as self-tested.
    #[test]
    fn inline_test_in_patch_passes() {
        let ctx = RuleContext::Pr {
            pr_files: vec![file_with_patch(
                "src/foo.rs",
                "@@ -1,3 +1,10 @@\n+fn foo() {}\n+#[test]\n+fn test_foo() {}",
            )],
            pr_metadata: metadata(),
            options: PrRuleOptions::default(),
        };

        let results = DetectMissingTest.run(&ctx).expect("rule should run");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].severity, Severity::Pass);
    }

    #[test]
    fn warns_when_source_has_no_test_update() {
        let ctx = RuleContext::Pr {
            pr_files: vec![file("src/foo.rs"), file("README.md")],
            pr_metadata: metadata(),
            options: PrRuleOptions::default(),
        };

        let results = DetectMissingTest.run(&ctx).expect("rule should run");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].severity, Severity::Warning);
        assert_eq!(results[0].affected_files, vec!["src/foo.rs".to_string()]);
    }

    #[test]
    fn passes_when_test_pair_is_changed() {
        let ctx = RuleContext::Pr {
            pr_files: vec![file("src/foo.rs"), file("tests/foo_test.rs")],
            pr_metadata: metadata(),
            options: PrRuleOptions::default(),
        };

        let results = DetectMissingTest.run(&ctx).expect("rule should run");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].severity, Severity::Pass);
    }

    #[test]
    fn can_disable_rule_with_option() {
        let ctx = RuleContext::Pr {
            pr_files: vec![file("src/foo.rs")],
            pr_metadata: metadata(),
            options: PrRuleOptions {
                detect_missing_test: false,
                test_patterns: vec![],
            },
        };

        let results = DetectMissingTest.run(&ctx).expect("rule should run");
        assert!(results.is_empty());
    }

    #[test]
    fn custom_pattern_covers_source() {
        let ctx = RuleContext::Pr {
            pr_files: vec![file("src/foo.rs"), file("spec/foo.spec.rs")],
            pr_metadata: metadata(),
            options: PrRuleOptions {
                detect_missing_test: true,
                test_patterns: vec!["*.spec.rs".to_string()],
            },
        };

        let results = DetectMissingTest.run(&ctx).expect("rule should run");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].severity, Severity::Pass);
    }

    fn file(path: &str) -> PrFile {
        PrFile {
            filename: path.to_string(),
            status: "modified".to_string(),
            additions: 1,
            deletions: 1,
            changes: 2,
            patch: Some("@@ -1 +1 @@\n+some change".to_string()),
        }
    }

    fn file_with_patch(path: &str, patch: &str) -> PrFile {
        PrFile {
            filename: path.to_string(),
            status: "modified".to_string(),
            additions: 1,
            deletions: 1,
            changes: 2,
            patch: Some(patch.to_string()),
        }
    }

    fn removed_file(path: &str) -> PrFile {
        PrFile {
            filename: path.to_string(),
            status: "removed".to_string(),
            additions: 0,
            deletions: 10,
            changes: 10,
            patch: Some("@@ -1,10 +0,0 @@\n-fn foo() {}".to_string()),
        }
    }

    fn metadata() -> PrMetadata {
        PrMetadata {
            number: 1,
            title: "test".to_string(),
            body: None,
        }
    }
}
