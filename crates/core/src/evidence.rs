use std::fmt;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum EvidenceState<T> {
    Complete { value: T },
    Partial { value: T, gaps: Vec<EvidenceGap> },
    Missing { gaps: Vec<EvidenceGap> },
    NotApplicable,
}

impl<T> EvidenceState<T> {
    pub fn complete(value: T) -> Self {
        Self::Complete { value }
    }

    pub fn partial(value: T, gaps: Vec<EvidenceGap>) -> Self {
        Self::Partial { value, gaps }
    }

    pub fn missing(gaps: Vec<EvidenceGap>) -> Self {
        Self::Missing { gaps }
    }

    pub fn not_applicable() -> Self {
        Self::NotApplicable
    }

    pub fn value(&self) -> Option<&T> {
        match self {
            Self::Complete { value } | Self::Partial { value, .. } => Some(value),
            Self::Missing { .. } | Self::NotApplicable => None,
        }
    }

    pub fn gaps(&self) -> &[EvidenceGap] {
        match self {
            Self::Partial { gaps, .. } | Self::Missing { gaps } => gaps,
            Self::Complete { .. } | Self::NotApplicable => &[],
        }
    }

    pub fn has_gaps(&self) -> bool {
        !self.gaps().is_empty()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum EvidenceGap {
    CollectionFailed {
        source: String,
        subject: String,
        detail: String,
    },
    Truncated {
        source: String,
        subject: String,
    },
    MissingField {
        source: String,
        subject: String,
        field: String,
    },
    DiffUnavailable {
        subject: String,
    },
    Unsupported {
        source: String,
        capability: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChangeRequestId {
    pub system: String,
    pub value: String,
}

impl ChangeRequestId {
    pub fn new(system: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            system: system.into(),
            value: value.into(),
        }
    }
}

impl fmt::Display for ChangeRequestId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.system, self.value)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkItemRef {
    pub system: String,
    pub value: String,
    pub url: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChangedAsset {
    pub path: String,
    pub diff_available: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ApprovalDisposition {
    Approved,
    Rejected,
    Commented,
    Dismissed,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApprovalDecision {
    pub actor: String,
    pub disposition: ApprovalDisposition,
    pub submitted_at: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuthenticityEvidence {
    pub verified: bool,
    pub mechanism: Option<String>,
}

impl AuthenticityEvidence {
    pub fn new(verified: bool, mechanism: Option<String>) -> Self {
        Self {
            verified,
            mechanism,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceRevision {
    pub id: String,
    pub authored_by: Option<String>,
    pub committed_at: Option<String>,
    pub merge: bool,
    pub authenticity: EvidenceState<AuthenticityEvidence>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GovernedChange {
    pub id: ChangeRequestId,
    pub title: String,
    pub summary: Option<String>,
    pub submitted_by: Option<String>,
    pub changed_assets: EvidenceState<Vec<ChangedAsset>>,
    pub approval_decisions: EvidenceState<Vec<ApprovalDecision>>,
    pub source_revisions: EvidenceState<Vec<SourceRevision>>,
    pub work_item_refs: EvidenceState<Vec<WorkItemRef>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PromotionBatch {
    pub id: String,
    pub source_revisions: EvidenceState<Vec<SourceRevision>>,
    pub linked_change_requests: EvidenceState<Vec<ChangeRequestId>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct EvidenceBundle {
    pub change_requests: Vec<GovernedChange>,
    pub promotion_batches: Vec<PromotionBatch>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn partial_state_reports_gaps() {
        let state = EvidenceState::partial(
            vec![ChangedAsset {
                path: "src/main.rs".to_string(),
                diff_available: false,
            }],
            vec![EvidenceGap::DiffUnavailable {
                subject: "src/main.rs".to_string(),
            }],
        );

        assert!(state.has_gaps());
        assert_eq!(state.gaps().len(), 1);
    }

    #[test]
    fn change_request_id_formats_as_stable_subject() {
        let id = ChangeRequestId::new("github_pr", "owner/repo#42");
        assert_eq!(id.to_string(), "github_pr:owner/repo#42");
    }
}
