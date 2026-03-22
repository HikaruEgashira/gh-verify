pub mod branch_history_integrity;
pub mod branch_protection_enforcement;
pub mod build_isolation;
pub mod build_provenance;
pub mod hosted_build_platform;
pub mod issue_linkage;
pub mod pr_size;
pub mod provenance_authenticity;
pub mod required_status_checks;
pub mod review_independence;
pub mod scoped_change;
pub mod source_authenticity;
pub mod test_coverage;
pub mod two_party_review;

use crate::control::Control;
use crate::slsa::{SlsaLevel, SlsaTrack};

use self::branch_history_integrity::BranchHistoryIntegrityControl;
use self::branch_protection_enforcement::BranchProtectionEnforcementControl;
use self::build_isolation::BuildIsolationControl;
use self::build_provenance::BuildProvenanceControl;
use self::hosted_build_platform::HostedBuildPlatformControl;
use self::issue_linkage::IssueLinkageControl;
use self::pr_size::PrSizeControl;
use self::provenance_authenticity::ProvenanceAuthenticityControl;
use self::required_status_checks::RequiredStatusChecksControl;
use self::review_independence::ReviewIndependenceControl;
use self::scoped_change::ScopedChangeControl;
use self::source_authenticity::SourceAuthenticityControl;
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
