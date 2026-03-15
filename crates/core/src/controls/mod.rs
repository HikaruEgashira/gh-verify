pub mod review_independence;
pub mod source_authenticity;

use crate::control::Control;

use self::review_independence::ReviewIndependenceControl;
use self::source_authenticity::SourceAuthenticityControl;

pub fn slsa_foundation_controls() -> Vec<Box<dyn Control>> {
    vec![
        Box::new(ReviewIndependenceControl),
        Box::new(SourceAuthenticityControl),
    ]
}
