use anyhow::Result;
use gh_verify_core::scope::{FileRole, classify_file_role, is_non_code_file};
use gh_verify_core::test_coverage::has_test_coverage;
use gh_verify_core::verdict::{RuleResult, Severity};

use super::{Rule, RuleContext};

const RULE_ID: &str = "detect-missing-test";

pub struct DetectMissingTest;

impl Rule for DetectMissingTest {
    fn id(&self) -> &'static str {
        RULE_ID
    }

    fn run(&self, ctx: &RuleContext) -> Result<Vec<RuleResult>> {
        let (pr_files, options) = match ctx {
            RuleContext::Pr {
                pr_files, options, ..
            } => (pr_files, options),
            RuleContext::Release { .. } => return Ok(vec![pass_result()]),
        };

        if !options.detect_missing_test {
            return Ok(vec![]);
        }

        // 1. Exclude removed files
        let active_files: Vec<&str> = pr_files
            .iter()
            .filter(|f| f.status != "removed")
            .map(|f| f.filename.as_str())
            .collect();

        // 2. Classify: source files to check
        let source_files: Vec<&str> = active_files
            .iter()
            .copied()
            .filter(|p| !is_non_code_file(p))
            .filter(|p| classify_file_role(p) == FileRole::Source)
            .collect();

        if source_files.is_empty() {
            return Ok(vec![pass_result()]);
        }

        // 3. Check coverage (pass all changed paths for test matching)
        let mut uncovered = has_test_coverage(&source_files, &active_files);

        // 3b. Apply custom test patterns if configured
        if !options.test_patterns.is_empty() {
            let test_files: Vec<&str> = active_files
                .iter()
                .copied()
                .filter(|p| classify_file_role(p) == FileRole::Test)
                .collect();
            uncovered.retain(|entry| {
                !is_covered_by_custom_patterns(&entry.path, &test_files, &options.test_patterns)
            });
        }

        if uncovered.is_empty() {
            return Ok(vec![pass_result()]);
        }

        // 4. Warning with affected files
        let affected_files: Vec<String> = uncovered.iter().map(|u| u.path.clone()).collect();

        Ok(vec![RuleResult {
            rule_id: RULE_ID.to_string(),
            severity: Severity::Warning,
            message: format!(
                "{} source file(s) changed without matching test updates",
                uncovered.len()
            ),
            affected_files,
            suggestion: Some(
                "Consider adding or updating tests for the listed source files.".to_string(),
            ),
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

    /// WHY: Files with status "removed" must not be checked for test coverage.
    /// Flagging deleted files produces noise on cleanup PRs.
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

    /// WHY: A PR changing src/parser.ts with tests/parser_test.ts -> Pass
    #[test]
    fn source_with_test_pair_passes() {
        let ctx = RuleContext::Pr {
            pr_files: vec![file("src/parser.ts"), file("tests/parser_test.ts")],
            pr_metadata: metadata(),
            options: PrRuleOptions::default(),
        };

        let results = DetectMissingTest.run(&ctx).expect("rule should run");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].severity, Severity::Pass);
    }

    /// WHY: A PR changing only src/parser.ts without any test -> Warning
    #[test]
    fn source_without_test_warns() {
        let ctx = RuleContext::Pr {
            pr_files: vec![file("src/parser.ts")],
            pr_metadata: metadata(),
            options: PrRuleOptions::default(),
        };

        let results = DetectMissingTest.run(&ctx).expect("rule should run");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].severity, Severity::Warning);
        assert_eq!(
            results[0].affected_files,
            vec!["src/parser.ts".to_string()]
        );
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

    /// WHY: Release context has no PR files, so the rule is not applicable.
    #[test]
    fn release_context_returns_pass() {
        let ctx = RuleContext::Release {
            base_tag: "v0.1.0".to_string(),
            head_tag: "v0.2.0".to_string(),
            commits: vec![],
            commit_prs: vec![],
            pr_reviews: vec![],
        };

        let results = DetectMissingTest.run(&ctx).expect("rule should run");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].severity, Severity::Pass);
    }

    /// WHY: Config-only changes (.github/workflows/) should not trigger
    /// test coverage warnings.
    #[test]
    fn config_only_changes_pass() {
        let ctx = RuleContext::Pr {
            pr_files: vec![file(".github/workflows/ci.yml")],
            pr_metadata: metadata(),
            options: PrRuleOptions::default(),
        };

        let results = DetectMissingTest.run(&ctx).expect("rule should run");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].severity, Severity::Pass);
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
            patch: Some("@@ -1 +1 @@".to_string()),
        }
    }

    fn removed_file(path: &str) -> PrFile {
        PrFile {
            filename: path.to_string(),
            status: "removed".to_string(),
            additions: 0,
            deletions: 10,
            changes: 10,
            patch: Some("@@ -1,10 +0,0 @@".to_string()),
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
