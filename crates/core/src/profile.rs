use serde::{Deserialize, Serialize};

use crate::control::{ControlFinding, ControlId, ControlStatus};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FindingSeverity {
    Info,
    Warning,
    Error,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GateDecision {
    Pass,
    Review,
    Fail,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProfileOutcome {
    pub control_id: ControlId,
    pub severity: FindingSeverity,
    pub decision: GateDecision,
    pub rationale: String,
}

pub trait ControlProfile {
    fn name(&self) -> &'static str;
    fn map(&self, finding: &ControlFinding) -> ProfileOutcome;
}

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
            ControlStatus::Indeterminate => (FindingSeverity::Warning, GateDecision::Review),
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
    fn indeterminate_finding_maps_to_review_gate() {
        let outcome = SlsaFoundationProfile.map(&crate::control::ControlFinding::indeterminate(
            ControlId::ReviewIndependence,
            "Evidence is partial",
            vec!["github_pr:owner/repo#10".to_string()],
            vec![],
        ));

        assert_eq!(outcome.severity, FindingSeverity::Warning);
        assert_eq!(outcome.decision, GateDecision::Review);
    }
}
