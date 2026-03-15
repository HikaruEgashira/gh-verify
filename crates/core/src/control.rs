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
