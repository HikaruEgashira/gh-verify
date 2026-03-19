use std::fmt;

use serde::{Deserialize, Serialize};

/// Represents the completeness of a collected evidence value.
///
/// Controls use this to distinguish between a verified absence and an
/// evidence-collection failure, which maps to different control statuses.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum EvidenceState<T> {
    /// All expected data was collected successfully.
    Complete { value: T },
    /// Data was collected but some aspects are missing or degraded.
    Partial { value: T, gaps: Vec<EvidenceGap> },
    /// No usable data could be collected; only gap descriptions remain.
    Missing { gaps: Vec<EvidenceGap> },
    /// The evidence category does not apply to this context.
    NotApplicable,
}

impl<T> EvidenceState<T> {
    /// Wraps a fully-collected value.
    pub fn complete(value: T) -> Self {
        Self::Complete { value }
    }

    /// Wraps a value that was collected with known gaps.
    pub fn partial(value: T, gaps: Vec<EvidenceGap>) -> Self {
        Self::Partial { value, gaps }
    }

    /// Creates a state where no value could be obtained at all.
    pub fn missing(gaps: Vec<EvidenceGap>) -> Self {
        Self::Missing { gaps }
    }

    /// Creates a state indicating the evidence category is irrelevant.
    pub fn not_applicable() -> Self {
        Self::NotApplicable
    }

    /// Returns the inner value if present (Complete or Partial).
    pub fn value(&self) -> Option<&T> {
        match self {
            Self::Complete { value } | Self::Partial { value, .. } => Some(value),
            Self::Missing { .. } | Self::NotApplicable => None,
        }
    }

    /// Returns the recorded evidence gaps, empty for Complete and NotApplicable.
    pub fn gaps(&self) -> &[EvidenceGap] {
        match self {
            Self::Partial { gaps, .. } | Self::Missing { gaps } => gaps,
            Self::Complete { .. } | Self::NotApplicable => &[],
        }
    }

    /// Returns true when at least one evidence gap exists.
    pub fn has_gaps(&self) -> bool {
        !self.gaps().is_empty()
    }
}

/// Describes why a piece of evidence is incomplete or absent.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum EvidenceGap {
    /// The adapter attempted collection but encountered an error.
    CollectionFailed {
        source: String,
        subject: String,
        detail: String,
    },
    /// The data was truncated by the upstream API (e.g. pagination limit).
    Truncated { source: String, subject: String },
    /// A required field was absent in the upstream response.
    MissingField {
        source: String,
        subject: String,
        field: String,
    },
    /// The diff content for a changed asset could not be retrieved.
    DiffUnavailable { subject: String },
    /// The upstream source does not support the requested capability.
    Unsupported { source: String, capability: String },
}

/// Platform-independent identifier for a change request (e.g. a pull request).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChangeRequestId {
    pub system: String,
    pub value: String,
}

impl ChangeRequestId {
    /// Creates a new identifier with the given system name and value.
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

/// Reference to an external work item (issue, Jira ticket, etc.).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkItemRef {
    pub system: String,
    pub value: String,
}

/// A file or artifact that was modified in a change request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChangedAsset {
    pub path: String,
    pub diff_available: bool,
}

/// Normalized outcome of a review action, independent of platform terminology.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ApprovalDisposition {
    /// The reviewer explicitly approved the change.
    Approved,
    /// The reviewer requested changes (blocks merge on some platforms).
    Rejected,
    /// The reviewer left comments without a disposition.
    Commented,
    /// The review was dismissed by a maintainer.
    Dismissed,
    /// The platform returned an unrecognized review state.
    Unknown,
}

/// A single review decision recorded against a change request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApprovalDecision {
    pub actor: String,
    pub disposition: ApprovalDisposition,
    pub submitted_at: Option<String>,
}

/// Cryptographic verification state for a source revision (e.g. GPG signature).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuthenticityEvidence {
    pub verified: bool,
    pub mechanism: Option<String>,
}

impl AuthenticityEvidence {
    /// Creates a new authenticity evidence record.
    pub fn new(verified: bool, mechanism: Option<String>) -> Self {
        Self {
            verified,
            mechanism,
        }
    }
}

/// A single commit or source revision associated with a change request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceRevision {
    pub id: String,
    pub authored_by: Option<String>,
    pub committed_at: Option<String>,
    pub merge: bool,
    pub authenticity: EvidenceState<AuthenticityEvidence>,
}

/// Normalized representation of a governed change request (e.g. a pull request).
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

/// A release or deployment batch that promotes one or more source revisions.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PromotionBatch {
    pub id: String,
    pub source_revisions: EvidenceState<Vec<SourceRevision>>,
    pub linked_change_requests: EvidenceState<Vec<ChangeRequestId>>,
}

/// Top-level container for all evidence collected from adapters.
///
/// Passed to controls for evaluation; should be platform-agnostic.
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

    // ================================================================
    // Mutation-hardening tests
    // ================================================================

    // --- EvidenceState::value() mutations ---

    #[test]
    fn complete_state_returns_value() {
        let state = EvidenceState::complete(42);
        assert_eq!(state.value(), Some(&42));
    }

    #[test]
    fn partial_state_returns_value() {
        let state = EvidenceState::partial(
            42,
            vec![EvidenceGap::DiffUnavailable {
                subject: "x".into(),
            }],
        );
        assert_eq!(state.value(), Some(&42));
    }

    #[test]
    fn missing_state_returns_none() {
        let state: EvidenceState<i32> = EvidenceState::missing(vec![]);
        assert_eq!(state.value(), None);
    }

    #[test]
    fn not_applicable_state_returns_none() {
        let state: EvidenceState<i32> = EvidenceState::not_applicable();
        assert_eq!(state.value(), None);
    }

    // --- EvidenceState::gaps() mutations ---

    #[test]
    fn complete_state_has_no_gaps() {
        let state = EvidenceState::complete(42);
        assert!(state.gaps().is_empty());
    }

    #[test]
    fn missing_state_returns_gaps() {
        let gap = EvidenceGap::DiffUnavailable {
            subject: "x".into(),
        };
        let state: EvidenceState<i32> = EvidenceState::missing(vec![gap]);
        assert_eq!(state.gaps().len(), 1);
    }

    #[test]
    fn not_applicable_state_has_no_gaps() {
        let state: EvidenceState<i32> = EvidenceState::not_applicable();
        assert!(state.gaps().is_empty());
    }

    // --- EvidenceState::has_gaps() mutations ---

    #[test]
    fn partial_with_empty_gaps_no_gaps() {
        let state = EvidenceState::partial(1, vec![]);
        assert!(!state.has_gaps());
    }

}
