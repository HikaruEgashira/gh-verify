pub mod branch_history_integrity;
pub mod branch_protection_enforcement;
pub mod build_isolation;
pub mod build_provenance;
pub mod conventional_title;
pub mod description_quality;
pub mod hosted_build_platform;
pub mod issue_linkage;
pub mod merge_commit_policy;
pub mod pr_size;
pub mod release_traceability;
pub mod provenance_authenticity;
pub mod required_status_checks;
pub mod review_independence;
pub mod scoped_change;
pub mod security_file_change;
pub mod source_authenticity;
pub mod stale_review;
pub mod test_coverage;
pub mod two_party_review;

use crate::control::Control;
use crate::slsa::{SlsaLevel, SlsaTrack};

use self::branch_history_integrity::BranchHistoryIntegrityControl;
use self::branch_protection_enforcement::BranchProtectionEnforcementControl;
use self::build_isolation::BuildIsolationControl;
use self::build_provenance::BuildProvenanceControl;
use self::conventional_title::ConventionalTitleControl;
use self::description_quality::DescriptionQualityControl;
use self::hosted_build_platform::HostedBuildPlatformControl;
use self::issue_linkage::IssueLinkageControl;
use self::merge_commit_policy::MergeCommitPolicyControl;
use self::pr_size::PrSizeControl;
use self::release_traceability::ReleaseTraceabilityControl;
use self::provenance_authenticity::ProvenanceAuthenticityControl;
use self::required_status_checks::RequiredStatusChecksControl;
use self::review_independence::ReviewIndependenceControl;
use self::scoped_change::ScopedChangeControl;
use self::security_file_change::SecurityFileChangeControl;
use self::source_authenticity::SourceAuthenticityControl;
use self::stale_review::StaleReviewControl;
use self::test_coverage::TestCoverageControl;
use self::two_party_review::TwoPartyReviewControl;

/// Instantiates a control by its ID.
fn instantiate(id: crate::control::ControlId) -> Box<dyn Control> {
    use crate::control::ControlId;
    match id {
        ControlId::SourceAuthenticity => Box::new(SourceAuthenticityControl),
        ControlId::ReviewIndependence => Box::new(ReviewIndependenceControl),
        ControlId::BranchHistoryIntegrity => Box::new(BranchHistoryIntegrityControl),
        ControlId::BranchProtectionEnforcement => Box::new(BranchProtectionEnforcementControl),
        ControlId::TwoPartyReview => Box::new(TwoPartyReviewControl),
        ControlId::BuildProvenance => Box::new(BuildProvenanceControl),
        ControlId::RequiredStatusChecks => Box::new(RequiredStatusChecksControl),
        ControlId::HostedBuildPlatform => Box::new(HostedBuildPlatformControl),
        ControlId::ProvenanceAuthenticity => Box::new(ProvenanceAuthenticityControl),
        ControlId::BuildIsolation => Box::new(BuildIsolationControl),
        ControlId::PrSize => Box::new(PrSizeControl),
        ControlId::TestCoverage => Box::new(TestCoverageControl),
        ControlId::ScopedChange => Box::new(ScopedChangeControl),
        ControlId::IssueLinkage => Box::new(IssueLinkageControl),
        ControlId::StaleReview => Box::new(StaleReviewControl),
        ControlId::DescriptionQuality => Box::new(DescriptionQualityControl),
        ControlId::MergeCommitPolicy => Box::new(MergeCommitPolicyControl),
        ControlId::ConventionalTitle => Box::new(ConventionalTitleControl),
        ControlId::SecurityFileChange => Box::new(SecurityFileChangeControl),
        ControlId::ReleaseTraceability => Box::new(ReleaseTraceabilityControl),
    }
}

/// Returns all SLSA controls required for the given track up to the given level.
pub fn slsa_controls_for_level(track: SlsaTrack, level: SlsaLevel) -> Vec<Box<dyn Control>> {
    crate::slsa::controls_for_level(track, level)
        .into_iter()
        .map(instantiate)
        .collect()
}

/// Returns all SLSA controls across both tracks up to the given levels.
pub fn slsa_controls(source_level: SlsaLevel, build_level: SlsaLevel) -> Vec<Box<dyn Control>> {
    let mut controls = slsa_controls_for_level(SlsaTrack::Source, source_level);
    controls.extend(slsa_controls_for_level(SlsaTrack::Build, build_level));
    controls
}

/// Returns all SLSA controls (Source L4 + Build L3).
pub fn all_slsa_controls() -> Vec<Box<dyn Control>> {
    slsa_controls(SlsaLevel::L4, SlsaLevel::L3)
}

/// Returns compliance controls (non-SLSA, SOC2 CC7/CC8 mapped).
pub fn compliance_controls() -> Vec<Box<dyn Control>> {
    vec![
        Box::new(PrSizeControl),
        Box::new(TestCoverageControl),
        Box::new(ScopedChangeControl),
        Box::new(IssueLinkageControl),
        Box::new(StaleReviewControl),
        Box::new(DescriptionQualityControl),
        Box::new(MergeCommitPolicyControl),
        Box::new(ConventionalTitleControl),
        Box::new(SecurityFileChangeControl),
        Box::new(ReleaseTraceabilityControl),
    ]
}

/// Returns all controls (all SLSA + compliance).
pub fn all_controls() -> Vec<Box<dyn Control>> {
    let mut controls = all_slsa_controls();
    controls.extend(compliance_controls());
    controls
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::slsa::control_slsa_mapping;

    #[test]
    fn slsa_l1_returns_l1_controls_only() {
        let controls = slsa_controls(SlsaLevel::L1, SlsaLevel::L1);
        for c in &controls {
            let mapping = control_slsa_mapping(c.id()).expect("should be SLSA-mapped");
            assert!(
                mapping.level <= SlsaLevel::L1,
                "{:?} is L{:?} but should be L1 or below",
                c.id(),
                mapping.level
            );
        }
    }

    #[test]
    fn all_slsa_includes_l3_build_and_l4_source() {
        let controls = all_slsa_controls();
        let ids: Vec<_> = controls.iter().map(|c| c.id()).collect();
        assert!(ids.contains(&crate::control::ControlId::TwoPartyReview));
        assert!(ids.contains(&crate::control::ControlId::BuildIsolation));
    }

    #[test]
    fn all_controls_includes_compliance() {
        let controls = all_controls();
        let ids: Vec<_> = controls.iter().map(|c| c.id()).collect();
        assert!(ids.contains(&crate::control::ControlId::PrSize));
        assert!(ids.contains(&crate::control::ControlId::IssueLinkage));
    }

    #[test]
    fn compliance_controls_count() {
        let controls = compliance_controls();
        assert_eq!(
            controls.len(),
            10,
            "compliance_controls() should return exactly 10 controls"
        );
    }

    #[test]
    fn compliance_controls_are_not_slsa_mapped() {
        use crate::slsa::control_slsa_mapping;
        let controls = compliance_controls();
        for c in &controls {
            assert!(
                control_slsa_mapping(c.id()).is_none(),
                "{:?} should not be SLSA-mapped",
                c.id()
            );
        }
    }

    #[test]
    fn compliance_controls_have_unique_ids() {
        let controls = compliance_controls();
        let mut ids: Vec<_> = controls.iter().map(|c| c.id()).collect();
        let original_len = ids.len();
        ids.sort_by_key(|id| id.as_str());
        ids.dedup();
        assert_eq!(ids.len(), original_len, "all compliance control IDs must be unique");
    }

    #[test]
    fn all_controls_count() {
        let slsa = all_slsa_controls();
        let compliance = compliance_controls();
        let all = all_controls();
        assert_eq!(
            all.len(),
            slsa.len() + compliance.len(),
            "all_controls = SLSA + compliance"
        );
    }

    #[test]
    fn slsa_controls_for_level_source_l2() {
        let controls = slsa_controls_for_level(SlsaTrack::Source, SlsaLevel::L2);
        let ids: Vec<_> = controls.iter().map(|c| c.id()).collect();
        assert!(ids.contains(&crate::control::ControlId::BranchHistoryIntegrity));
        assert!(!ids.contains(&crate::control::ControlId::BranchProtectionEnforcement));
    }

    #[test]
    fn slsa_controls_for_level_build_l2() {
        let controls = slsa_controls_for_level(SlsaTrack::Build, SlsaLevel::L2);
        let ids: Vec<_> = controls.iter().map(|c| c.id()).collect();
        assert!(ids.contains(&crate::control::ControlId::HostedBuildPlatform));
        assert!(ids.contains(&crate::control::ControlId::ProvenanceAuthenticity));
        assert!(!ids.contains(&crate::control::ControlId::BuildIsolation));
    }
}
