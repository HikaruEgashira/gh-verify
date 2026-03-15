use serde::{Deserialize, Serialize};

use crate::evidence::{EvidenceBundle, EvidenceGap};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ControlId {
    ReviewIndependence,
    SourceAuthenticity,
}

impl ControlId {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::ReviewIndependence => "review-independence",
            Self::SourceAuthenticity => "source-authenticity",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ControlStatus {
    Satisfied,
    Violated,
    Indeterminate,
    NotApplicable,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ControlFinding {
    pub control_id: ControlId,
    pub status: ControlStatus,
    pub rationale: String,
    pub subjects: Vec<String>,
    pub evidence_gaps: Vec<EvidenceGap>,
}

impl ControlFinding {
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

pub trait Control {
    fn id(&self) -> ControlId;
    fn evaluate(&self, evidence: &EvidenceBundle) -> Vec<ControlFinding>;
}

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
