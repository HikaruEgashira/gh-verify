use anyhow::Result;
use gh_verify_core::conventional::{classify_commit_compliance, is_conventional_commit};
use gh_verify_core::verdict::{RuleResult, Severity};

use super::{Rule, RuleContext};

const RULE_ID: &str = "verify-conventional-commit";

// TODO: Support per-rule CLI configuration (--commit-types, --no-require-conventional-commit).
// Currently all rules run unconditionally via engine::run_all().
// See: https://github.com/HikaruEgashira/gh-verify/issues/11

/// Empty slice means "accept any valid type matching `[a-z][a-z0-9]*`".
const ALLOWED_TYPES: &[&str] = &[];

const FORMAT_HINT: &str = "Use the format: <type>[optional scope]: <description>\n\
    Common types: feat, fix, docs, style, refactor, perf, test, build, ci, chore, revert";

pub struct VerifyConventionalCommit;

impl Rule for VerifyConventionalCommit {
    fn id(&self) -> &'static str {
        RULE_ID
    }

    fn run(&self, ctx: &RuleContext) -> Result<Vec<RuleResult>> {
        match ctx {
            RuleContext::Pr { pr_metadata, .. } => {
                let title = &pr_metadata.title;
                if is_conventional_commit(title, ALLOWED_TYPES) {
                    Ok(vec![RuleResult::pass(
                        RULE_ID,
                        "PR title follows Conventional Commits format",
                    )])
                } else {
                    Ok(vec![RuleResult {
                        rule_id: RULE_ID.to_string(),
                        severity: Severity::Warning,
                        message: format!(
                            "PR title does not follow Conventional Commits format: \"{}\"",
                            title
                        ),
                        affected_files: vec![],
                        suggestion: Some(FORMAT_HINT.to_string()),
                    }])
                }
            }
            RuleContext::Release { commits, .. } => {
                let messages: Vec<&str> =
                    commits.iter().map(|c| c.commit.message.as_str()).collect();
                let severity = classify_commit_compliance(&messages, ALLOWED_TYPES);

                let non_merge_msgs: Vec<&str> = messages
                    .iter()
                    .copied()
                    .filter(|m| !gh_verify_core::conventional::is_merge_commit(m))
                    .collect();
                let non_compliant: Vec<&&str> = non_merge_msgs
                    .iter()
                    .filter(|m| !is_conventional_commit(m, ALLOWED_TYPES))
                    .collect();

                if severity == Severity::Pass {
                    Ok(vec![RuleResult::pass(
                        RULE_ID,
                        "All commit messages follow Conventional Commits format",
                    )])
                } else {
                    let mut detail = String::from("Non-compliant commit messages:\n");
                    for msg in &non_compliant {
                        let subject = msg.lines().next().unwrap_or(msg);
                        detail.push_str(&format!("  - {}\n", subject));
                    }
                    detail.push('\n');
                    detail.push_str(FORMAT_HINT);

                    Ok(vec![RuleResult {
                        rule_id: RULE_ID.to_string(),
                        severity,
                        message: format!(
                            "{} of {} non-merge commits do not follow Conventional Commits format",
                            non_compliant.len(),
                            non_merge_msgs.len()
                        ),
                        affected_files: vec![],
                        suggestion: Some(detail),
                    }])
                }
            }
        }
    }
}
