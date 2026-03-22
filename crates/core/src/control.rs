use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

use crate::evidence::{EvidenceBundle, EvidenceGap};

/// Identifies a specific SDLC control in the catalog.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ControlId {
    // --- Source Track ---
    /// Source L1: Commit signatures must be present and verified.
    SourceAuthenticity,
    /// Source L1: Four-eyes principle: approver must differ from author and requester.
    ReviewIndependence,
    /// Source L2: Branch history is continuous, immutable, and protected from force-push.
    BranchHistoryIntegrity,
    /// Source L3: Branch protection rules are continuously enforced (required reviews, status checks).
    BranchProtectionEnforcement,
    /// Source L4: At least two independent reviewers approved the change.
    TwoPartyReview,

    // --- Build Track ---
    /// Build L1: Artifact has verified SLSA provenance attestation.
    BuildProvenance,
    /// Build L1: At least one required status check is configured and passes.
    RequiredStatusChecks,
    /// Build L2: Build runs on a hosted platform (not a developer workstation).
    HostedBuildPlatform,
    /// Build L2: Provenance attestation is cryptographically signed and authenticated.
    ProvenanceAuthenticity,
    /// Build L3: Build runs in an isolated, ephemeral environment.
    BuildIsolation,

    // --- Compliance (non-SLSA) ---
    /// Compliance CC8.1: PR size is within acceptable limits.
    PrSize,
    /// Compliance CC8.1: source changes include matching test updates.
    TestCoverage,
    /// Compliance CC8.1: PR changes are well-scoped (single logical unit).
    ScopedChange,
    /// Compliance CC7.1: PR references at least one issue or ticket.
    IssueLinkage,
    /// Compliance CC7.2: approvals must postdate the latest source revision.
    StaleReview,
    /// Compliance CC8.1: change requests must include a meaningful description.
    DescriptionQuality,
    /// Compliance CC8.1: source revisions must follow linear history (no merge commits).
    MergeCommitPolicy,
    /// Compliance CC8.1: change request titles must follow Conventional Commits format.
    ConventionalTitle,
    /// Compliance CC7.2: changes to security-sensitive files require heightened scrutiny.
    SecurityFileChange,
    /// Compliance CC7.1: release batches must trace to governed change requests.
    ReleaseTraceability,

    // --- NIST SSDF / OpenSSF Scorecard ---
    /// NIST PW.7 / OpenSSF SAST: at least one SAST tool must run in CI.
    SastToolPresence,
    /// OpenSSF Binary-Artifacts: no binary artifacts added to source.
    BinaryArtifactCheck,
    /// OpenSSF Pinned-Dependencies: GitHub Actions uses SHA-pinned references.
    DependencyPinning,
    /// OpenSSF Token-Permissions: workflow tokens follow least privilege.
    WorkflowPermissions,
}

impl ControlId {
    /// Returns the kebab-case string representation used in serialized output.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::SourceAuthenticity => "source-authenticity",
            Self::ReviewIndependence => "review-independence",
            Self::BranchHistoryIntegrity => "branch-history-integrity",
            Self::BranchProtectionEnforcement => "branch-protection-enforcement",
            Self::TwoPartyReview => "two-party-review",
            Self::BuildProvenance => "build-provenance",
            Self::RequiredStatusChecks => "required-status-checks",
            Self::HostedBuildPlatform => "hosted-build-platform",
            Self::ProvenanceAuthenticity => "provenance-authenticity",
            Self::BuildIsolation => "build-isolation",
            Self::PrSize => "pr-size",
            Self::TestCoverage => "test-coverage",
            Self::ScopedChange => "scoped-change",
            Self::IssueLinkage => "issue-linkage",
            Self::StaleReview => "stale-review",
            Self::DescriptionQuality => "description-quality",
            Self::MergeCommitPolicy => "merge-commit-policy",
            Self::ConventionalTitle => "conventional-title",
            Self::SecurityFileChange => "security-file-change",
            Self::ReleaseTraceability => "release-traceability",
            Self::SastToolPresence => "sast-tool-presence",
            Self::BinaryArtifactCheck => "binary-artifact-check",
            Self::DependencyPinning => "dependency-pinning",
            Self::WorkflowPermissions => "workflow-permissions",
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
            "source-authenticity" => Ok(Self::SourceAuthenticity),
            "review-independence" => Ok(Self::ReviewIndependence),
            "branch-history-integrity" => Ok(Self::BranchHistoryIntegrity),
            "branch-protection-enforcement" => Ok(Self::BranchProtectionEnforcement),
            "two-party-review" => Ok(Self::TwoPartyReview),
            "build-provenance" => Ok(Self::BuildProvenance),
            "required-status-checks" => Ok(Self::RequiredStatusChecks),
            "hosted-build-platform" => Ok(Self::HostedBuildPlatform),
            "provenance-authenticity" => Ok(Self::ProvenanceAuthenticity),
            "build-isolation" => Ok(Self::BuildIsolation),
            "pr-size" => Ok(Self::PrSize),
            "test-coverage" => Ok(Self::TestCoverage),
            "scoped-change" => Ok(Self::ScopedChange),
            "issue-linkage" => Ok(Self::IssueLinkage),
            "stale-review" => Ok(Self::StaleReview),
            "description-quality" => Ok(Self::DescriptionQuality),
            "merge-commit-policy" => Ok(Self::MergeCommitPolicy),
            "conventional-title" => Ok(Self::ConventionalTitle),
            "security-file-change" => Ok(Self::SecurityFileChange),
            "release-traceability" => Ok(Self::ReleaseTraceability),
            "sast-tool-presence" => Ok(Self::SastToolPresence),
            "binary-artifact-check" => Ok(Self::BinaryArtifactCheck),
            "dependency-pinning" => Ok(Self::DependencyPinning),
            "workflow-permissions" => Ok(Self::WorkflowPermissions),
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

impl ControlStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Satisfied => "satisfied",
            Self::Violated => "violated",
            Self::Indeterminate => "indeterminate",
            Self::NotApplicable => "not_applicable",
        }
    }
}

impl fmt::Display for ControlStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
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
        let variants = [
            ControlId::SourceAuthenticity,
            ControlId::ReviewIndependence,
            ControlId::BranchHistoryIntegrity,
            ControlId::BranchProtectionEnforcement,
            ControlId::TwoPartyReview,
            ControlId::BuildProvenance,
            ControlId::RequiredStatusChecks,
            ControlId::HostedBuildPlatform,
            ControlId::ProvenanceAuthenticity,
            ControlId::BuildIsolation,
            ControlId::PrSize,
            ControlId::TestCoverage,
            ControlId::ScopedChange,
            ControlId::IssueLinkage,
            ControlId::StaleReview,
            ControlId::DescriptionQuality,
            ControlId::MergeCommitPolicy,
            ControlId::ConventionalTitle,
            ControlId::SecurityFileChange,
            ControlId::ReleaseTraceability,
            ControlId::SastToolPresence,
            ControlId::BinaryArtifactCheck,
            ControlId::DependencyPinning,
            ControlId::WorkflowPermissions,
        ];
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
}

#[cfg(test)]
#[path = "tests/control_hardening.rs"]
mod control_hardening;
