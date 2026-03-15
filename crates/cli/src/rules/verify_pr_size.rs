use anyhow::Result;
use gh_verify_core::size::{classify_pr_size, is_generated_file};
use gh_verify_core::verdict::{RuleResult, Severity};

use super::{Rule, RuleContext};

const RULE_ID: &str = "verify-pr-size";

// Threshold rationale: validated against the benchmark PR corpus
// (ossinsight PRs in benchmarks/cases/). All pass/warning cases have
// <500 lines, confirming these defaults align with real-world PRs.
// This rule is not exercised by the benchmark harness (bench.rs filters
// by rule_id = "detect-unscoped-change"), so threshold validation is
// done via unit tests below.

const WARN_LINES: usize = 500;
const WARN_FILES: usize = 15;
const ERROR_LINES: usize = 1000;
const ERROR_FILES: usize = 30;

pub struct VerifyPrSize;

impl Rule for VerifyPrSize {
    fn id(&self) -> &'static str {
        RULE_ID
    }

    fn run(&self, ctx: &RuleContext) -> Result<Vec<RuleResult>> {
        let pr_files = match ctx {
            RuleContext::Pr { pr_files, .. } => pr_files,
            RuleContext::Release { .. } => return Ok(vec![pass_result()]),
        };

        // Filter out generated files
        let non_generated: Vec<_> = pr_files
            .iter()
            .filter(|f| !is_generated_file(&f.filename))
            .collect();

        let total_files = non_generated.len();
        let total_lines: usize = non_generated
            .iter()
            .map(|f| (f.additions + f.deletions) as usize)
            .sum();

        let severity = classify_pr_size(
            total_lines,
            total_files,
            WARN_LINES,
            WARN_FILES,
            ERROR_LINES,
            ERROR_FILES,
        );

        if severity == Severity::Pass {
            return Ok(vec![pass_result()]);
        }

        let affected: Vec<String> = non_generated.iter().map(|f| f.filename.clone()).collect();

        Ok(vec![RuleResult {
            rule_id: RULE_ID.to_string(),
            severity,
            message: format!(
                "PR touches {total_lines} lines across {total_files} files (excluding generated)"
            ),
            affected_files: affected,
            suggestion: Some(
                "Consider splitting into smaller, focused pull requests.".to_string(),
            ),
        }])
    }
}

fn pass_result() -> RuleResult {
    RuleResult::pass(RULE_ID, "PR size is within acceptable limits")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::github::types::{PrFile, PrMetadata};

    fn make_file(name: &str, additions: u32, deletions: u32) -> PrFile {
        PrFile {
            filename: name.to_string(),
            status: "modified".to_string(),
            additions,
            deletions,
            changes: additions + deletions,
            patch: Some(String::new()),
        }
    }

    fn pr_ctx(files: Vec<PrFile>) -> RuleContext {
        RuleContext::Pr {
            pr_files: files,
            pr_metadata: PrMetadata {
                number: 1,
                title: "test".to_string(),
                body: None,
            },
            options: crate::rules::PrRuleOptions::default(),
        }
    }

    #[test]
    fn small_pr_passes() {
        let files = vec![make_file("src/a.rs", 5, 5), make_file("src/b.rs", 0, 0)];
        let results = VerifyPrSize.run(&pr_ctx(files)).unwrap();
        assert_eq!(results[0].severity, Severity::Pass);
    }

    #[test]
    fn many_lines_warns() {
        let files: Vec<_> = (0..5)
            .map(|i| make_file(&format!("src/{i}.rs"), 60, 60))
            .collect();
        // 5 files * 120 lines = 600 lines
        let results = VerifyPrSize.run(&pr_ctx(files)).unwrap();
        assert_eq!(results[0].severity, Severity::Warning);
    }

    #[test]
    fn many_files_warns() {
        let files: Vec<_> = (0..20)
            .map(|i| make_file(&format!("src/{i}.rs"), 3, 2))
            .collect();
        // 20 files > 15 warn threshold
        let results = VerifyPrSize.run(&pr_ctx(files)).unwrap();
        assert_eq!(results[0].severity, Severity::Warning);
    }

    #[test]
    fn huge_pr_errors() {
        let files: Vec<_> = (0..40)
            .map(|i| make_file(&format!("src/{i}.rs"), 20, 17))
            .collect();
        // 40 files * 37 lines = 1480 lines, 40 files
        let results = VerifyPrSize.run(&pr_ctx(files)).unwrap();
        assert_eq!(results[0].severity, Severity::Error);
    }

    #[test]
    fn generated_files_excluded() {
        let files = vec![
            make_file("Cargo.lock", 1000, 1000),
            make_file("package-lock.json", 500, 500),
        ];
        let results = VerifyPrSize.run(&pr_ctx(files)).unwrap();
        assert_eq!(results[0].severity, Severity::Pass);
    }
}
