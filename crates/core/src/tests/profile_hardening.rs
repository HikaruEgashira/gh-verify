use super::*;

#[test]
fn profile_name_is_correct() {
    // Kills: returning wrong profile name
    assert_eq!(SlsaFoundationProfile.name(), "slsa-foundation");
}
