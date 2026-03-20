pub mod build_provenance;
pub mod review_independence;
pub mod source_authenticity;

use crate::control::Control;

use self::build_provenance::BuildProvenanceControl;
use self::review_independence::ReviewIndependenceControl;
use self::source_authenticity::SourceAuthenticityControl;

/// Returns the default set of controls for the SLSA foundation profile.
///
/// Branch protection and required reviewer settings are intentionally excluded:
/// these are admin-mutable repository settings that can be changed or bypassed
/// at any time. Instead, we verify the actual PR-level evidence (reviews,
/// signatures) which cannot be retroactively altered.
pub fn slsa_foundation_controls() -> Vec<Box<dyn Control>> {
    vec![
        Box::new(ReviewIndependenceControl),
        Box::new(SourceAuthenticityControl),
        Box::new(BuildProvenanceControl),
    ]
}
