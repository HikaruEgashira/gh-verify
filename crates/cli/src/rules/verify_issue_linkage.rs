use anyhow::Result;
use gh_verify_core::linkage::{extract_issue_references, has_issue_linkage};
use gh_verify_core::verdict::{RuleResult, Severity};

use super::{Rule, RuleContext};

const RULE_ID: &str = "verify-issue-linkage";

pub struct VerifyIssueLinkage;

impl Rule for VerifyIssueLinkage {
    fn id(&self) -> &'static str {
        RULE_ID
    }

    fn run(&self, ctx: &RuleContext) -> Result<Vec<RuleResult>> {
        let pr_metadata = match ctx {
            RuleContext::Pr { pr_metadata, .. } => pr_metadata,
            RuleContext::Release { .. } => return Ok(vec![]),
        };

        let body = pr_metadata.body.as_deref().unwrap_or("");
        let refs = extract_issue_references(body, &[]);

        if has_issue_linkage(&refs) {
            let ref_list: Vec<&str> = refs.iter().map(|r| r.value.as_str()).collect();
            Ok(vec![RuleResult::pass(
                RULE_ID,
                &format!("PR links to issue(s): {}", ref_list.join(", ")),
            )])
        } else {
            Ok(vec![RuleResult {
                rule_id: RULE_ID.to_string(),
                severity: Severity::Error,
                message: "PR body has no issue or ticket reference".to_string(),
                affected_files: vec![],
                suggestion: Some(
                    "Add a reference such as `fixes #123`, a Jira ticket like `PROJ-456`, \
                     or a URL to the related issue."
                        .to_string(),
                ),
            }])
        }
    }
}
