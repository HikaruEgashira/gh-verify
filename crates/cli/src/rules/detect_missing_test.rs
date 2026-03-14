use anyhow::Result;
use gh_verify_core::scope::{classify_file_role, is_non_code_file, FileRole};
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
        let pr_files = match ctx {
            RuleContext::Pr { pr_files, .. } => pr_files,
            RuleContext::Release { .. } => return Ok(vec![pass_result()]),
        };

        let all_filenames: Vec<&str> = pr_files.iter().map(|f| f.filename.as_str()).collect();

        // Identify source files (code files that are not tests/fixtures/non-code)
        let source_files: Vec<&str> = all_filenames
            .iter()
            .filter(|&&f| !is_non_code_file(f) && classify_file_role(f) == FileRole::Source)
            .copied()
            .collect();

        if source_files.is_empty() {
            return Ok(vec![pass_result()]);
        }

        let uncovered = has_test_coverage(&source_files, &all_filenames);

        if uncovered.is_empty() {
            return Ok(vec![pass_result()]);
        }

        let affected: Vec<String> = uncovered.iter().map(|u| u.path.clone()).collect();
        let file_list = affected
            .iter()
            .map(|p| format!("  - {p}"))
            .collect::<Vec<_>>()
            .join("\n");

        Ok(vec![RuleResult {
            rule_id: RULE_ID.to_string(),
            severity: Severity::Warning,
            message: format!(
                "{} source file(s) changed without corresponding test changes",
                uncovered.len()
            ),
            affected_files: affected,
            suggestion: Some(format!(
                "Consider adding tests for:\n{file_list}"
            )),
        }])
    }
}

fn pass_result() -> RuleResult {
    RuleResult::pass(RULE_ID, "All changed source files have corresponding test changes")
}
