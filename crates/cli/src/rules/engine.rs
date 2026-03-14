use anyhow::Result;
use gh_verify_core::verdict::RuleResult;

use super::detect_stale_approval::DetectStaleApproval;
use super::detect_unscoped_change::DetectUnscopedChange;
use super::verify_release_integrity::VerifyReleaseIntegrity;
use super::{Rule, RuleContext};

/// Run all registered rules and return aggregated results.
pub fn run_all(ctx: &RuleContext) -> Result<Vec<RuleResult>> {
    let rules: Vec<Box<dyn Rule>> = vec![
        Box::new(DetectStaleApproval),
        Box::new(DetectUnscopedChange),
        Box::new(VerifyReleaseIntegrity),
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
        DetectStaleApproval.id(),
        DetectUnscopedChange.id(),
        VerifyReleaseIntegrity.id(),
    ]
}
