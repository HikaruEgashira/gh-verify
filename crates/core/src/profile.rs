use std::fmt;

use serde::{Deserialize, Serialize};

use crate::control::{ControlFinding, ControlId, ControlStatus};
use crate::slsa::{SlsaLevel, SlsaTrack, control_slsa_mapping};

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

impl GateDecision {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Pass => "pass",
            Self::Review => "review",
            Self::Fail => "fail",
        }
    }
}

impl fmt::Display for GateDecision {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
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
    /// Returns the human-readable profile name (e.g. "slsa-source-l3-build-l2").
    fn name(&self) -> &'static str;
    /// Converts a control finding into a profile outcome with severity and gate decision.
    fn map(&self, finding: &ControlFinding) -> ProfileOutcome;
}

/// SLSA level-aware profile.
///
/// Controls at or below the configured level are required (Indeterminate → Fail).
/// Controls above the configured level are advisory (Indeterminate → Review).
/// Non-SLSA controls are always advisory.
pub struct SlsaLevelProfile {
    pub source_level: SlsaLevel,
    pub build_level: SlsaLevel,
    profile_name: &'static str,
}

impl SlsaLevelProfile {
    pub fn new(source_level: SlsaLevel, build_level: SlsaLevel) -> Self {
        let profile_name = match (source_level, build_level) {
            (SlsaLevel::L0, SlsaLevel::L0) => "slsa-source-l0-build-l0",
            (SlsaLevel::L1, SlsaLevel::L0) => "slsa-source-l1-build-l0",
            (SlsaLevel::L0, SlsaLevel::L1) => "slsa-source-l0-build-l1",
            (SlsaLevel::L1, SlsaLevel::L1) => "slsa-source-l1-build-l1",
            (SlsaLevel::L2, SlsaLevel::L1) => "slsa-source-l2-build-l1",
            (SlsaLevel::L2, SlsaLevel::L2) => "slsa-source-l2-build-l2",
            (SlsaLevel::L3, SlsaLevel::L2) => "slsa-source-l3-build-l2",
            (SlsaLevel::L3, SlsaLevel::L3) => "slsa-source-l3-build-l3",
            (SlsaLevel::L4, SlsaLevel::L3) => "slsa-source-l4-build-l3",
            _ => "slsa-custom",
        };
        Self {
            source_level,
            build_level,
            profile_name,
        }
    }

    /// Returns true if the given control is required at the configured levels.
    fn is_required(&self, control_id: ControlId) -> bool {
        match control_slsa_mapping(control_id) {
            Some(mapping) => {
                let target_level = match mapping.track {
                    SlsaTrack::Source => self.source_level,
                    SlsaTrack::Build => self.build_level,
                };
                mapping.level <= target_level
            }
            // Non-SLSA controls are never required
            None => false,
        }
    }
}

impl ControlProfile for SlsaLevelProfile {
    fn name(&self) -> &'static str {
        self.profile_name
    }

    fn map(&self, finding: &ControlFinding) -> ProfileOutcome {
        let required = self.is_required(finding.control_id);

        let (severity, decision) = match finding.status {
            ControlStatus::Satisfied | ControlStatus::NotApplicable => {
                (FindingSeverity::Info, GateDecision::Pass)
            }
            ControlStatus::Indeterminate => {
                if required {
                    (FindingSeverity::Error, GateDecision::Fail)
                } else {
                    (FindingSeverity::Warning, GateDecision::Review)
                }
            }
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

/// Parses a profile name into the corresponding profile instance.
///
/// Format: `slsa-source-l{N}-build-l{M}` where N is 0-4 and M is 0-3.
pub fn parse_profile(name: &str) -> Option<Box<dyn ControlProfile>> {
    if name.starts_with("slsa-source-l") && name.contains("-build-l") {
        parse_level_profile(name).map(|p| Box::new(p) as Box<dyn ControlProfile>)
    } else {
        None
    }
}

fn parse_level_profile(name: &str) -> Option<SlsaLevelProfile> {
    let rest = name.strip_prefix("slsa-source-l")?;
    let dash_pos = rest.find("-build-l")?;
    let source_str = &rest[..dash_pos];
    let build_str = &rest[dash_pos + 8..];

    let source_level = parse_level_num(source_str)?;
    let build_level = parse_level_num(build_str)?;

    if !build_level.is_valid_for_track(SlsaTrack::Build) {
        return None;
    }

    Some(SlsaLevelProfile::new(source_level, build_level))
}

fn parse_level_num(s: &str) -> Option<SlsaLevel> {
    match s {
        "0" => Some(SlsaLevel::L0),
        "1" => Some(SlsaLevel::L1),
        "2" => Some(SlsaLevel::L2),
        "3" => Some(SlsaLevel::L3),
        "4" => Some(SlsaLevel::L4),
        _ => None,
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

    fn l1_profile() -> SlsaLevelProfile {
        SlsaLevelProfile::new(SlsaLevel::L1, SlsaLevel::L1)
    }

    #[test]
    fn l1_indeterminate_required_control_fails() {
        let profile = l1_profile();
        let outcome = profile.map(&ControlFinding::indeterminate(
            ControlId::ReviewIndependence, // Source L1 → required
            "Evidence is partial",
            vec!["github_pr:owner/repo#10".to_string()],
            vec![],
        ));
        assert_eq!(outcome.severity, FindingSeverity::Error);
        assert_eq!(outcome.decision, GateDecision::Fail);
    }

    #[test]
    fn l1_satisfied_maps_to_pass() {
        let profile = l1_profile();
        let outcome = profile.map(&ControlFinding::satisfied(
            ControlId::ReviewIndependence,
            "Independent reviewer approved",
            vec!["github_pr:owner/repo#10".to_string()],
        ));
        assert_eq!(outcome.severity, FindingSeverity::Info);
        assert_eq!(outcome.decision, GateDecision::Pass);
        assert_eq!(outcome.control_id, ControlId::ReviewIndependence);
    }

    #[test]
    fn violated_always_fails() {
        let profile = l1_profile();
        let outcome = profile.map(&ControlFinding::violated(
            ControlId::SourceAuthenticity,
            "No valid signature found",
            vec!["github_release:owner/repo@v1.0".to_string()],
        ));
        assert_eq!(outcome.severity, FindingSeverity::Error);
        assert_eq!(outcome.decision, GateDecision::Fail);
    }

    #[test]
    fn not_applicable_maps_to_pass() {
        let profile = l1_profile();
        let outcome = profile.map(&ControlFinding::not_applicable(
            ControlId::SourceAuthenticity,
            "No release artifacts to verify",
        ));
        assert_eq!(outcome.severity, FindingSeverity::Info);
        assert_eq!(outcome.decision, GateDecision::Pass);
    }

    #[test]
    fn level_profile_required_control_indeterminate_fails() {
        let profile = SlsaLevelProfile::new(SlsaLevel::L3, SlsaLevel::L2);
        let outcome = profile.map(&ControlFinding::indeterminate(
            ControlId::BranchHistoryIntegrity, // Source L2, required at L3
            "Evidence incomplete",
            vec!["branch:main".to_string()],
            vec![],
        ));
        assert_eq!(outcome.decision, GateDecision::Fail);
        assert_eq!(outcome.severity, FindingSeverity::Error);
    }

    #[test]
    fn level_profile_above_level_indeterminate_reviews() {
        let profile = l1_profile();
        let outcome = profile.map(&ControlFinding::indeterminate(
            ControlId::BranchHistoryIntegrity, // Source L2, not required at L1
            "Evidence incomplete",
            vec!["branch:main".to_string()],
            vec![],
        ));
        assert_eq!(outcome.decision, GateDecision::Review);
        assert_eq!(outcome.severity, FindingSeverity::Warning);
    }

    #[test]
    fn level_profile_violated_above_level_still_fails() {
        let profile = l1_profile();
        let outcome = profile.map(&ControlFinding::violated(
            ControlId::TwoPartyReview, // Source L4, above L1 but violated
            "Only 1 reviewer",
            vec!["github_pr:owner/repo#5".to_string()],
        ));
        assert_eq!(outcome.decision, GateDecision::Fail);
        assert_eq!(outcome.severity, FindingSeverity::Error);
    }

    #[test]
    fn dev_quality_indeterminate_reviews() {
        let profile = SlsaLevelProfile::new(SlsaLevel::L4, SlsaLevel::L3);
        let outcome = profile.map(&ControlFinding::indeterminate(
            ControlId::PrSize,
            "Cannot determine PR size",
            vec!["github_pr:owner/repo#5".to_string()],
            vec![],
        ));
        assert_eq!(outcome.decision, GateDecision::Review);
        assert_eq!(outcome.severity, FindingSeverity::Warning);
    }

    #[test]
    fn build_l2_hosted_required() {
        let profile = SlsaLevelProfile::new(SlsaLevel::L1, SlsaLevel::L2);
        let outcome = profile.map(&ControlFinding::indeterminate(
            ControlId::HostedBuildPlatform,
            "Cannot determine if hosted",
            vec!["build:ci".to_string()],
            vec![],
        ));
        assert_eq!(outcome.decision, GateDecision::Fail);
    }

    #[test]
    fn build_l3_isolation_not_required_at_l2() {
        let profile = SlsaLevelProfile::new(SlsaLevel::L1, SlsaLevel::L2);
        let outcome = profile.map(&ControlFinding::indeterminate(
            ControlId::BuildIsolation,
            "Cannot determine isolation",
            vec!["build:ci".to_string()],
            vec![],
        ));
        assert_eq!(outcome.decision, GateDecision::Review);
    }

    #[test]
    fn parse_profile_level_based() {
        assert!(parse_profile("slsa-source-l1-build-l1").is_some());
        assert!(parse_profile("slsa-source-l3-build-l2").is_some());
        assert!(parse_profile("slsa-source-l4-build-l3").is_some());
        assert!(parse_profile("slsa-source-l4-build-l4").is_none());
        assert!(parse_profile("slsa-source-l5-build-l1").is_none());
        assert!(parse_profile("unknown").is_none());
    }

    #[test]
    fn parse_profile_level_names() {
        let p = parse_profile("slsa-source-l3-build-l2").unwrap();
        assert_eq!(p.name(), "slsa-source-l3-build-l2");
    }

    #[test]
    fn apply_profile_processes_all_findings() {
        let profile = l1_profile();
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

        let outcomes = apply_profile(&profile, &findings);

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
