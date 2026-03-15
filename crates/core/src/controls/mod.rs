pub mod review_independence;
pub mod source_authenticity;

use crate::control::Control;

use self::review_independence::ReviewIndependenceControl;
use self::source_authenticity::SourceAuthenticityControl;

/// Returns the default set of controls for the SLSA foundation profile.
pub fn slsa_foundation_controls() -> Vec<Box<dyn Control>> {
    vec![
        Box::new(ReviewIndependenceControl),
        Box::new(SourceAuthenticityControl),
    ]
}
