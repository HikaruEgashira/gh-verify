use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

use crate::evidence::{EvidenceBundle, EvidenceGap};

/// Identifies a specific SDLC control in the catalog.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ControlId {
    /// Four-eyes principle: approver must differ from author and requester.
    ReviewIndependence,
    /// Commit signatures must be present and verified.
    SourceAuthenticity,
}

impl ControlId {
    /// Returns the kebab-case string representation used in serialized output.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::ReviewIndependence => "review-independence",
            Self::SourceAuthenticity => "source-authenticity",
        }
    }
}

impl fmt::Display for ControlId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnknownControlId(String);

impl fmt::Display for UnknownControlId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "unknown control id: {}", self.0)
    }
}

impl std::error::Error for UnknownControlId {}

impl FromStr for ControlId {
    type Err = UnknownControlId;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "review-independence" => Ok(Self::ReviewIndependence),
            "source-authenticity" => Ok(Self::SourceAuthenticity),
            _ => Err(UnknownControlId(s.to_string())),
        }
    }
}

/// Outcome of evaluating a single control against evidence.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ControlStatus {
    /// The control requirement is fully met.
    Satisfied,
    /// The control requirement is demonstrably not met.
    Violated,
    /// Evidence is insufficient to determine compliance.
    Indeterminate,
    /// The control does not apply to the supplied evidence.
    NotApplicable,
}

/// Result of a single control evaluation, including status and supporting detail.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ControlFinding {
    pub control_id: ControlId,
    pub status: ControlStatus,
    pub rationale: String,
    pub subjects: Vec<String>,
    pub evidence_gaps: Vec<EvidenceGap>,
}

impl ControlFinding {
    /// Creates a finding indicating the control is fully satisfied.
    pub fn satisfied(
        control_id: ControlId,
        rationale: impl Into<String>,
        subjects: Vec<String>,
    ) -> Self {
        Self {
            control_id,
            status: ControlStatus::Satisfied,
            rationale: rationale.into(),
            subjects,
            evidence_gaps: Vec::new(),
        }
    }

    /// Creates a finding indicating the control requirement was not met.
    pub fn violated(
        control_id: ControlId,
        rationale: impl Into<String>,
        subjects: Vec<String>,
    ) -> Self {
        Self {
            control_id,
            status: ControlStatus::Violated,
            rationale: rationale.into(),
            subjects,
            evidence_gaps: Vec::new(),
        }
    }

    /// Creates a finding when evidence is too incomplete to decide.
    pub fn indeterminate(
        control_id: ControlId,
        rationale: impl Into<String>,
        subjects: Vec<String>,
        evidence_gaps: Vec<EvidenceGap>,
    ) -> Self {
        Self {
            control_id,
            status: ControlStatus::Indeterminate,
            rationale: rationale.into(),
            subjects,
            evidence_gaps,
        }
    }

    /// Creates a finding when the control does not apply to the context.
    pub fn not_applicable(control_id: ControlId, rationale: impl Into<String>) -> Self {
        Self {
            control_id,
            status: ControlStatus::NotApplicable,
            rationale: rationale.into(),
            subjects: Vec::new(),
            evidence_gaps: Vec::new(),
        }
    }
}

/// A verifiable SDLC control that produces findings from evidence.
pub trait Control {
    /// Returns the unique identifier for this control.
    fn id(&self) -> ControlId;
    /// Evaluates the evidence bundle and returns one finding per subject.
    fn evaluate(&self, evidence: &EvidenceBundle) -> Vec<ControlFinding>;
}

/// Runs every control against the evidence bundle and collects all findings.
pub fn evaluate_all(
    controls: &[Box<dyn Control>],
    evidence: &EvidenceBundle,
) -> Vec<ControlFinding> {
    let mut findings = Vec::new();
    for control in controls {
        findings.extend(control.evaluate(evidence));
    }
    findings
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn control_id_display_round_trip() {
        let variants = [ControlId::ReviewIndependence, ControlId::SourceAuthenticity];
        for id in &variants {
            let s = id.to_string();
            let parsed: ControlId = s.parse().unwrap();
            assert_eq!(*id, parsed, "round-trip failed for {s}");
        }
    }

    #[test]
    fn control_id_from_str_unknown() {
        let result = "nonexistent-control".parse::<ControlId>();
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err().to_string(),
            "unknown control id: nonexistent-control"
        );
    }

    // ================================================================
    // Mutation-hardening tests
    // ================================================================

    // --- ControlFinding constructors ---

    #[test]
    fn finding_satisfied_has_correct_status_and_no_gaps() {
        let f = ControlFinding::satisfied(
            ControlId::ReviewIndependence,
            "ok",
            vec!["s1".into()],
        );
        assert_eq!(f.status, ControlStatus::Satisfied);
        assert_eq!(f.control_id, ControlId::ReviewIndependence);
        assert_eq!(f.subjects, vec!["s1"]);
        assert!(f.evidence_gaps.is_empty());
    }

    #[test]
    fn finding_violated_has_correct_status_and_no_gaps() {
        let f = ControlFinding::violated(
            ControlId::SourceAuthenticity,
            "bad",
            vec!["s1".into()],
        );
        assert_eq!(f.status, ControlStatus::Violated);
        assert!(f.evidence_gaps.is_empty());
    }

    #[test]
    fn finding_indeterminate_preserves_gaps() {
        let gaps = vec![EvidenceGap::DiffUnavailable {
            subject: "x".into(),
        }];
        let f = ControlFinding::indeterminate(
            ControlId::ReviewIndependence,
            "partial",
            vec!["s1".into()],
            gaps,
        );
        assert_eq!(f.status, ControlStatus::Indeterminate);
        assert_eq!(f.evidence_gaps.len(), 1);
    }

    #[test]
    fn finding_not_applicable_has_empty_subjects_and_gaps() {
        let f = ControlFinding::not_applicable(
            ControlId::SourceAuthenticity,
            "n/a",
        );
        assert_eq!(f.status, ControlStatus::NotApplicable);
        assert!(f.subjects.is_empty());
        assert!(f.evidence_gaps.is_empty());
    }

    // --- evaluate_all ---

    #[test]
    fn evaluate_all_collects_from_all_controls() {
        // Kills: only evaluating first control or returning early
        use crate::controls::slsa_foundation_controls;
        let evidence = EvidenceBundle::default();
        let controls = slsa_foundation_controls();
        let findings = evaluate_all(&controls, &evidence);
        // Both controls should produce at least one finding each
        assert!(findings.iter().any(|f| f.control_id == ControlId::ReviewIndependence));
        assert!(findings.iter().any(|f| f.control_id == ControlId::SourceAuthenticity));
    }

    #[test]
    fn evaluate_all_empty_controls_returns_empty() {
        let evidence = EvidenceBundle::default();
        let controls: Vec<Box<dyn Control>> = vec![];
        let findings = evaluate_all(&controls, &evidence);
        assert!(findings.is_empty());
    }
}
