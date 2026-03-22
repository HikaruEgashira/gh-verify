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

impl<T> Default for EvidenceState<T> {
    /// Defaults to `NotApplicable`: the evidence category was not collected.
    fn default() -> Self {
        Self::NotApplicable
    }
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

impl fmt::Display for EvidenceGap {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::CollectionFailed {
                source,
                subject,
                detail,
            } => {
                write!(f, "collection failed: {source}/{subject}: {detail}")
            }
            Self::Truncated { source, subject } => {
                write!(f, "truncated: {source}/{subject}")
            }
            Self::MissingField {
                source,
                subject,
                field,
            } => {
                write!(f, "missing field: {source}/{subject}.{field}")
            }
            Self::DiffUnavailable { subject } => {
                write!(f, "diff unavailable: {subject}")
            }
            Self::Unsupported { source, capability } => {
                write!(f, "unsupported: {source}/{capability}")
            }
        }
    }
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
    /// Number of lines added.
    #[serde(default)]
    pub additions: u32,
    /// Number of lines deleted.
    #[serde(default)]
    pub deletions: u32,
    /// File status: "added", "modified", "removed", "renamed", etc.
    #[serde(default)]
    pub status: String,
    /// Unified diff patch text, if available.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub diff: Option<String>,
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

/// Result of verifying an artifact's build provenance attestation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArtifactAttestation {
    /// Artifact path or OCI URI that was verified.
    pub subject: String,
    /// Attestation predicate type (e.g. "https://slsa.dev/provenance/v1").
    pub predicate_type: String,
    /// The workflow that signed the attestation.
    pub signer_workflow: Option<String>,
    /// The source repository associated with the attestation.
    pub source_repo: Option<String>,
    /// Whether the attestation passed cryptographic verification.
    pub verified: bool,
    /// Detail message from the verifier.
    pub verification_detail: Option<String>,
}

/// Conclusion of a CI check run, normalized across platforms.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CheckConclusion {
    Success,
    Failure,
    Neutral,
    Cancelled,
    Skipped,
    TimedOut,
    ActionRequired,
    /// The check is still running (no conclusion yet).
    Pending,
    /// The platform returned an unrecognized conclusion.
    Unknown,
}

/// Evidence for a single CI check run executed against a commit.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CheckRunEvidence {
    /// Name of the check (e.g. "ci/build", "lint").
    pub name: String,
    /// Conclusion of the check run.
    pub conclusion: CheckConclusion,
}

/// Build platform evidence for Build Track L2+.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BuildPlatformEvidence {
    /// Name of the build platform (e.g. "github-actions", "cloud-build").
    pub platform: String,
    /// Whether the build ran on hosted infrastructure (not a self-hosted runner).
    pub hosted: bool,
    /// Whether the build environment was ephemeral (fresh for each run).
    pub ephemeral: bool,
    /// Whether the build was isolated from other concurrent builds.
    pub isolated: bool,
    /// The runner labels/tags (e.g. ["ubuntu-latest", "self-hosted"]).
    pub runner_labels: Vec<String>,
    /// Whether the provenance signing key was inaccessible to user-defined build steps.
    pub signing_key_isolated: bool,
}

/// Top-level container for all evidence collected from adapters.
///
/// Passed to controls for evaluation; should be platform-agnostic.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct EvidenceBundle {
    /// Source Track: governed change requests (e.g. pull requests).
    pub change_requests: Vec<GovernedChange>,
    /// Source Track: release promotion batches.
    pub promotion_batches: Vec<PromotionBatch>,
    /// Build Track: artifact provenance attestations.
    pub artifact_attestations: EvidenceState<Vec<ArtifactAttestation>>,
    /// CI check runs executed against the PR HEAD commit.
    pub check_runs: EvidenceState<Vec<CheckRunEvidence>>,
    /// Build Track L2+: build platform information.
    pub build_platform: EvidenceState<Vec<BuildPlatformEvidence>>,
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
                additions: 0,
                deletions: 0,
                status: String::new(),
                diff: None,
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

#[cfg(test)]
#[path = "tests/evidence_hardening.rs"]
mod evidence_hardening;
