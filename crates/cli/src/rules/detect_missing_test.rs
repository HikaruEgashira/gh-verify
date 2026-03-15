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
            RuleContext::Release { .. } => return Ok(vec![]),
        };

        if !options.detect_missing_test {
            return Ok(vec![]);
        }

        let changed_code_files: Vec<&str> = pr_files
            .iter()
            .filter(|f| f.patch.is_some())
            .filter(|f| f.status != "removed")
            .filter(|f| !is_non_code_file(&f.filename))
            .map(|f| f.filename.as_str())
            .collect();

        let source_files: Vec<&str> = changed_code_files
            .iter()
            .copied()
            .filter(|p| classify_file_role(p) == FileRole::Source)
            .collect();
        let test_files: Vec<&str> = changed_code_files
            .iter()
            .copied()
            .filter(|p| classify_file_role(p) == FileRole::Test)
            .collect();

        if source_files.is_empty() {
            return Ok(vec![pass_result()]);
        }

        let mut uncovered = has_test_coverage(&source_files, &test_files);
        if !options.test_patterns.is_empty() {
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
                .unwrap_or_else(|| "(候補なし)".to_string());
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

    fn make_ctx(files: Vec<PrFile>, options: PrRuleOptions) -> RuleContext {
        RuleContext::Pr {
            pr_files: files,
            pr_metadata: metadata(),
            pr_reviews: vec![],
            pr_commits: vec![],
            options,
        }
    }

    #[test]
    fn warns_when_source_has_no_test_update() {
        let ctx = make_ctx(vec![file("src/foo.rs"), file("README.md")], PrRuleOptions::default());

        let results = DetectMissingTest.run(&ctx).expect("rule should run");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].severity, Severity::Warning);
        assert_eq!(results[0].affected_files, vec!["src/foo.rs".to_string()]);
    }

    #[test]
    fn passes_when_test_pair_is_changed() {
        let ctx = make_ctx(vec![file("src/foo.rs"), file("tests/foo_test.rs")], PrRuleOptions::default());

        let results = DetectMissingTest.run(&ctx).expect("rule should run");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].severity, Severity::Pass);
    }

    #[test]
    fn can_disable_rule_with_option() {
        let ctx = make_ctx(vec![file("src/foo.rs")], PrRuleOptions {
            detect_missing_test: false,
            test_patterns: vec![],
        });

        let results = DetectMissingTest.run(&ctx).expect("rule should run");
        assert!(results.is_empty());
    }

    #[test]
    fn custom_pattern_covers_source() {
        let ctx = make_ctx(vec![file("src/foo.rs"), file("spec/foo.spec.rs")], PrRuleOptions {
            detect_missing_test: true,
            test_patterns: vec!["*.spec.rs".to_string()],
        });

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

    fn metadata() -> PrMetadata {
        PrMetadata {
            number: 1,
            title: "test".to_string(),
            body: None,
            user: None,
        }
    }
}
