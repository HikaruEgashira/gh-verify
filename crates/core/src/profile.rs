use serde::{Deserialize, Serialize};

use crate::control::{ControlFinding, ControlId, ControlStatus};

/// Severity level assigned to a control finding by a profile.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FindingSeverity {
    /// Informational; no action required.
    Info,
    /// Warrants human review but does not block.
    Warning,
    /// Blocks the gate; must be resolved.
    Error,
}

/// Gate outcome that determines whether a pipeline stage may proceed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GateDecision {
    /// The control is satisfied; proceed without intervention.
    Pass,
    /// Human review is required before proceeding.
    Review,
    /// The control is violated; the gate must not pass.
    Fail,
}

/// The profile-mapped result for a single control finding.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProfileOutcome {
    pub control_id: ControlId,
    pub severity: FindingSeverity,
    pub decision: GateDecision,
    pub rationale: String,
}

/// Maps raw control findings to severity and gate decisions for a given policy.
pub trait ControlProfile {
    /// Returns the human-readable profile name (e.g. "slsa-foundation").
    fn name(&self) -> &'static str;
    /// Converts a control finding into a profile outcome with severity and gate decision.
    fn map(&self, finding: &ControlFinding) -> ProfileOutcome;
}

/// Default profile implementing SLSA Build L1 / Source L1 gate policy.
pub struct SlsaFoundationProfile;

impl ControlProfile for SlsaFoundationProfile {
    fn name(&self) -> &'static str {
        "slsa-foundation"
    }

    fn map(&self, finding: &ControlFinding) -> ProfileOutcome {
        let (severity, decision) = match finding.status {
            ControlStatus::Satisfied | ControlStatus::NotApplicable => {
                (FindingSeverity::Info, GateDecision::Pass)
            }
            // Intentionally separate from Violated: other profiles may map
            // Indeterminate → Review. SLSA Foundation treats insufficient evidence as failure
            // to align with Creusot's four_eyes_gate proof.
            ControlStatus::Indeterminate => (FindingSeverity::Error, GateDecision::Fail),
            ControlStatus::Violated => (FindingSeverity::Error, GateDecision::Fail),
        };

        ProfileOutcome {
            control_id: finding.control_id,
            severity,
            decision,
            rationale: finding.rationale.clone(),
        }
    }
}

/// Applies a profile to all findings and returns the mapped outcomes.
pub fn apply_profile(
    profile: &dyn ControlProfile,
    findings: &[ControlFinding],
) -> Vec<ProfileOutcome> {
    findings
        .iter()
        .map(|finding| profile.map(finding))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn indeterminate_finding_maps_to_fail_gate() {
        let outcome = SlsaFoundationProfile.map(&ControlFinding::indeterminate(
            ControlId::ReviewIndependence,
            "Evidence is partial",
            vec!["github_pr:owner/repo#10".to_string()],
            vec![],
        ));

        assert_eq!(outcome.severity, FindingSeverity::Error);
        assert_eq!(outcome.decision, GateDecision::Fail);
    }

    #[test]
    fn satisfied_finding_maps_to_pass_info() {
        let outcome = SlsaFoundationProfile.map(&ControlFinding::satisfied(
            ControlId::ReviewIndependence,
            "Independent reviewer approved",
            vec!["github_pr:owner/repo#10".to_string()],
        ));

        assert_eq!(outcome.severity, FindingSeverity::Info);
        assert_eq!(outcome.decision, GateDecision::Pass);
        assert_eq!(outcome.control_id, ControlId::ReviewIndependence);
    }

    #[test]
    fn violated_finding_maps_to_fail_error() {
        let outcome = SlsaFoundationProfile.map(&ControlFinding::violated(
            ControlId::SourceAuthenticity,
            "No valid signature found",
            vec!["github_release:owner/repo@v1.0".to_string()],
        ));

        assert_eq!(outcome.severity, FindingSeverity::Error);
        assert_eq!(outcome.decision, GateDecision::Fail);
        assert_eq!(outcome.control_id, ControlId::SourceAuthenticity);
    }

    #[test]
    fn not_applicable_finding_maps_to_pass_info() {
        let outcome = SlsaFoundationProfile.map(&ControlFinding::not_applicable(
            ControlId::SourceAuthenticity,
            "No release artifacts to verify",
        ));

        assert_eq!(outcome.severity, FindingSeverity::Info);
        assert_eq!(outcome.decision, GateDecision::Pass);
        assert_eq!(outcome.control_id, ControlId::SourceAuthenticity);
    }

    #[test]
    fn apply_profile_processes_all_findings() {
        let findings = vec![
            ControlFinding::satisfied(
                ControlId::ReviewIndependence,
                "Approved",
                vec!["github_pr:owner/repo#1".to_string()],
            ),
            ControlFinding::violated(
                ControlId::SourceAuthenticity,
                "Unsigned",
                vec!["github_release:owner/repo@v1.0".to_string()],
            ),
            ControlFinding::not_applicable(ControlId::ReviewIndependence, "No PR context"),
            ControlFinding::indeterminate(
                ControlId::SourceAuthenticity,
                "Partial evidence",
                vec!["github_release:owner/repo@v2.0".to_string()],
                vec![],
            ),
        ];

        let outcomes = apply_profile(&SlsaFoundationProfile, &findings);

        assert_eq!(outcomes.len(), 4);
        assert_eq!(outcomes[0].decision, GateDecision::Pass);
        assert_eq!(outcomes[0].severity, FindingSeverity::Info);
        assert_eq!(outcomes[1].decision, GateDecision::Fail);
        assert_eq!(outcomes[1].severity, FindingSeverity::Error);
        assert_eq!(outcomes[2].decision, GateDecision::Pass);
        assert_eq!(outcomes[2].severity, FindingSeverity::Info);
        assert_eq!(outcomes[3].decision, GateDecision::Fail);
        assert_eq!(outcomes[3].severity, FindingSeverity::Error);
    }

}

#[cfg(test)]
#[path = "tests/profile_hardening.rs"]
mod profile_hardening;
