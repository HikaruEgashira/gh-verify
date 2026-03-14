use anyhow::Result;
use gh_verify_core::verdict::RuleResult;

use super::detect_unscoped_change::DetectUnscopedChange;
use super::verify_branch_protection::VerifyBranchProtection;
use super::verify_release_integrity::VerifyReleaseIntegrity;
use super::{Rule, RuleContext};

/// Run all registered rules and return aggregated results.
pub fn run_all(ctx: &RuleContext) -> Result<Vec<RuleResult>> {
    let rules: Vec<Box<dyn Rule>> = vec![
        Box::new(DetectUnscopedChange),
        Box::new(VerifyReleaseIntegrity),
        Box::new(VerifyBranchProtection),
    ];

    let mut results = Vec::new();
    for rule in &rules {
        let rule_results = rule.run(ctx)?;
        results.extend(rule_results);
    }
    Ok(results)
}

/// Return IDs of all registered rules.
pub fn list_rule_ids() -> Vec<&'static str> {
    vec![
        DetectUnscopedChange.id(),
        VerifyReleaseIntegrity.id(),
        VerifyBranchProtection.id(),
    ]
}
