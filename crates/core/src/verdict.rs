use serde::{Deserialize, Serialize};

/// Severity levels for rule results.
///
/// Invariant: the ordering Pass < Warning < Error corresponds to
/// increasing severity. This is used by the CLI to determine the
/// exit code (exit 1 if any Error).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Pass,
    Warning,
    Error,
}

impl Severity {
    /// Returns true if this severity should cause a non-zero exit code.
    pub fn is_failing(&self) -> bool {
        matches!(self, Severity::Error)
    }
}

/// The result of evaluating a single rule.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuleResult {
    pub rule_id: String,
    pub severity: Severity,
    pub message: String,
    pub affected_files: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub suggestion: Option<String>,
}

impl RuleResult {
    pub fn pass(rule_id: &str, message: &str) -> Self {
        Self {
            rule_id: rule_id.to_string(),
            severity: Severity::Pass,
            message: message.to_string(),
            affected_files: vec![],
            suggestion: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn severity_ordering() {
        assert!(Severity::Pass < Severity::Warning);
        assert!(Severity::Warning < Severity::Error);
    }

    #[test]
    fn severity_is_failing() {
        assert!(!Severity::Pass.is_failing());
        assert!(!Severity::Warning.is_failing());
        assert!(Severity::Error.is_failing());
    }

    #[test]
    fn pass_result_construction() {
        let r = RuleResult::pass("test-rule", "all good");
        assert_eq!(r.severity, Severity::Pass);
        assert!(r.affected_files.is_empty());
        assert!(r.suggestion.is_none());
    }

    // ================================================================
    // Mutation-hardening tests
    // ================================================================

    #[test]
    fn severity_ordering_all_pairs() {
        // Kills: any reordering of enum variants
        assert!(Severity::Pass < Severity::Warning);
        assert!(Severity::Pass < Severity::Error);
        assert!(Severity::Warning < Severity::Error);
        assert!(Severity::Warning > Severity::Pass);
        assert!(Severity::Error > Severity::Warning);
        assert!(Severity::Error > Severity::Pass);
    }

    #[test]
    fn severity_equality() {
        assert_eq!(Severity::Pass, Severity::Pass);
        assert_eq!(Severity::Warning, Severity::Warning);
        assert_eq!(Severity::Error, Severity::Error);
        assert_ne!(Severity::Pass, Severity::Warning);
        assert_ne!(Severity::Pass, Severity::Error);
        assert_ne!(Severity::Warning, Severity::Error);
    }

    #[test]
    fn is_failing_only_error() {
        // Kills: matching Warning as failing or not matching Error
        assert!(!Severity::Pass.is_failing());
        assert!(!Severity::Warning.is_failing());
        assert!(Severity::Error.is_failing());
    }

    #[test]
    fn pass_result_fields() {
        let r = RuleResult::pass("my-rule", "message");
        assert_eq!(r.rule_id, "my-rule");
        assert_eq!(r.message, "message");
        assert_eq!(r.severity, Severity::Pass);
        assert!(r.affected_files.is_empty());
        assert!(r.suggestion.is_none());
    }
}
