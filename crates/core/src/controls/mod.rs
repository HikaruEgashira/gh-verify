pub mod build_provenance;
pub mod issue_linkage;
pub mod pr_size;
pub mod required_status_checks;
pub mod review_independence;
pub mod scoped_change;
pub mod source_authenticity;
pub mod test_coverage;

use crate::control::Control;

use self::build_provenance::BuildProvenanceControl;
use self::issue_linkage::IssueLinkageControl;
use self::pr_size::PrSizeControl;
use self::required_status_checks::RequiredStatusChecksControl;
use self::review_independence::ReviewIndependenceControl;
use self::scoped_change::ScopedChangeControl;
use self::source_authenticity::SourceAuthenticityControl;
use self::test_coverage::TestCoverageControl;

/// Returns the default set of controls for the SLSA foundation profile.
pub fn slsa_foundation_controls() -> Vec<Box<dyn Control>> {
    vec![
        Box::new(ReviewIndependenceControl),
        Box::new(SourceAuthenticityControl),
        Box::new(BuildProvenanceControl),
        Box::new(RequiredStatusChecksControl),
    ]
}

/// Returns controls for development quality (non-SLSA).
pub fn development_quality_controls() -> Vec<Box<dyn Control>> {
    vec![
        Box::new(PrSizeControl),
        Box::new(TestCoverageControl),
        Box::new(ScopedChangeControl),
        Box::new(IssueLinkageControl),
    ]
}

/// Returns all controls (SLSA foundation + development quality).
pub fn all_controls() -> Vec<Box<dyn Control>> {
    let mut controls = slsa_foundation_controls();
    controls.extend(development_quality_controls());
    controls
}
