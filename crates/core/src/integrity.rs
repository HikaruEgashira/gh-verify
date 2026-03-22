//! Release integrity verification predicates.
//!
//! Pure functions amenable to formal verification with Creusot.
//! The critical decision logic is proven correct in the `gh-verify-verif` crate.

use crate::verdict::Severity;

/// Truncate a SHA to 7 characters for display.
pub fn short_sha(sha: &str) -> &str {
    if sha.len() >= 7 { &sha[..7] } else { sha }
}

// --- Creusot-verifiable core predicates ---
//
// These pure functions operate on primitive types only, making them
// directly verifiable by Creusot's SMT backend. Controls in the
// `controls/` module delegate to these predicates, ensuring the
// critical logic is proven correct.

/// Core predicate for the four-eyes principle.
/// An approver is independent iff they are neither a commit author nor the PR author.
///
/// Verified by Creusot in `gh-verify-verif` crate.
pub fn is_approver_independent(is_commit_author: bool, is_pr_author: bool) -> bool {
    !is_commit_author && !is_pr_author
}

/// Core predicate for signature verification result severity.
/// Verified by Creusot in `gh-verify-verif` crate.
pub fn signature_severity(unsigned_count: usize) -> Severity {
    if unsigned_count == 0 {
        Severity::Pass
    } else {
        Severity::Error
    }
}

/// Core predicate for build provenance verification severity.
/// Zero unverified attestations → Pass, any unverified → Error.
pub fn build_provenance_severity(unverified_count: usize) -> Severity {
    if unverified_count == 0 {
        Severity::Pass
    } else {
        Severity::Error
    }
}

/// Core predicate for required status checks severity.
/// Pass iff zero check runs have a failing conclusion.
///
/// Verified by Creusot in `gh-verify-verif` crate.
pub fn required_status_checks_severity(fail_count: usize) -> Severity {
    if fail_count == 0 {
        Severity::Pass
    } else {
        Severity::Error
    }
}

/// Core predicate for branch history integrity severity (Source L2).
/// Zero unprotected branches -> Pass, any unprotected -> Error.
///
/// Verified by Creusot in `gh-verify-verif` crate.
pub fn branch_history_severity(unprotected_count: usize) -> Severity {
    if unprotected_count == 0 {
        Severity::Pass
    } else {
        Severity::Error
    }
}

/// Core predicate for branch protection enforcement severity (Source L3).
/// Zero non-enforced rules -> Pass, any non-enforced -> Error.
///
/// Verified by Creusot in `gh-verify-verif` crate.
pub fn branch_protection_enforcement_severity(non_enforced_count: usize) -> Severity {
    if non_enforced_count == 0 {
        Severity::Pass
    } else {
        Severity::Error
    }
}

/// Core predicate for two-party review severity (Source L4).
/// At least 2 independent approvers -> Pass, fewer -> Error.
///
/// Verified by Creusot in `gh-verify-verif` crate.
pub fn two_party_review_severity(independent_count: usize) -> Severity {
    if independent_count >= 2 {
        Severity::Pass
    } else {
        Severity::Error
    }
}

/// Core predicate for hosted build platform severity (Build L2).
/// Zero non-hosted builds -> Pass, any non-hosted -> Error.
///
/// Verified by Creusot in `gh-verify-verif` crate.
pub fn hosted_build_severity(non_hosted_count: usize) -> Severity {
    if non_hosted_count == 0 {
        Severity::Pass
    } else {
        Severity::Error
    }
}

/// Core predicate for provenance authenticity severity (Build L2).
/// Zero unauthenticated attestations -> Pass, any unauthenticated -> Error.
///
/// Verified by Creusot in `gh-verify-verif` crate.
pub fn provenance_authenticity_severity(unauthenticated_count: usize) -> Severity {
    if unauthenticated_count == 0 {
        Severity::Pass
    } else {
        Severity::Error
    }
}

/// Core predicate for build isolation severity (Build L3).
/// Zero non-isolated builds -> Pass, any non-isolated -> Error.
///
/// Verified by Creusot in `gh-verify-verif` crate.
pub fn build_isolation_severity(non_isolated_count: usize) -> Severity {
    if non_isolated_count == 0 {
        Severity::Pass
    } else {
        Severity::Error
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- verif↔core equivalence tests (exhaustive for boolean predicates) ---

    #[test]
    fn is_approver_independent_exhaustive_equivalence() {
        // Exhaustive truth table: both inputs are bool, so 4 combinations
        for &ca in &[false, true] {
            for &pa in &[false, true] {
                let result = is_approver_independent(ca, pa);
                let spec = !ca && !pa;
                assert_eq!(
                    result, spec,
                    "is_approver_independent({ca}, {pa}): got {result}, spec {spec}"
                );
            }
        }
    }

    #[test]
    fn signature_severity_equivalence() {
        assert_eq!(signature_severity(0), Severity::Pass);
        for count in 1..=100 {
            assert_eq!(
                signature_severity(count),
                Severity::Error,
                "signature_severity({count}) should be Error"
            );
        }
    }

    #[test]
    fn build_provenance_severity_equivalence() {
        assert_eq!(build_provenance_severity(0), Severity::Pass);
        for count in 1..=100 {
            assert_eq!(
                build_provenance_severity(count),
                Severity::Error,
                "build_provenance_severity({count}) should be Error"
            );
        }
    }

    /// Exhaustive equivalence for classify_scope against Creusot spec.
    #[test]
    fn classify_scope_exhaustive_equivalence() {
        use crate::scope::classify_scope;
        for files in 0..=20usize {
            for comps in 0..=20usize {
                let result = classify_scope(files, comps);
                let spec = if files <= 1 {
                    Severity::Pass
                } else if comps <= 1 {
                    Severity::Pass
                } else if comps == 2 {
                    Severity::Warning
                } else {
                    Severity::Error
                };
                assert_eq!(
                    result, spec,
                    "classify_scope({files}, {comps}): got {result:?}, spec {spec:?}"
                );
            }
        }
    }

    #[test]
    fn required_status_checks_severity_equivalence() {
        assert_eq!(required_status_checks_severity(0), Severity::Pass);
        for count in 1..=10 {
            assert_eq!(
                required_status_checks_severity(count),
                Severity::Error,
                "required_status_checks_severity({count}) should be Error"
            );
        }
    }

    #[test]
    fn branch_history_severity_equivalence() {
        assert_eq!(branch_history_severity(0), Severity::Pass);
        for count in 1..=10 {
            assert_eq!(
                branch_history_severity(count),
                Severity::Error,
                "branch_history_severity({count}) should be Error"
            );
        }
    }

    #[test]
    fn branch_protection_enforcement_severity_equivalence() {
        assert_eq!(branch_protection_enforcement_severity(0), Severity::Pass);
        for count in 1..=10 {
            assert_eq!(
                branch_protection_enforcement_severity(count),
                Severity::Error,
                "branch_protection_enforcement_severity({count}) should be Error"
            );
        }
    }

    #[test]
    fn two_party_review_severity_equivalence() {
        assert_eq!(two_party_review_severity(0), Severity::Error);
        assert_eq!(two_party_review_severity(1), Severity::Error);
        assert_eq!(two_party_review_severity(2), Severity::Pass);
        for count in 3..=10 {
            assert_eq!(
                two_party_review_severity(count),
                Severity::Pass,
                "two_party_review_severity({count}) should be Pass"
            );
        }
    }

    #[test]
    fn hosted_build_severity_equivalence() {
        assert_eq!(hosted_build_severity(0), Severity::Pass);
        for count in 1..=10 {
            assert_eq!(
                hosted_build_severity(count),
                Severity::Error,
                "hosted_build_severity({count}) should be Error"
            );
        }
    }

    #[test]
    fn provenance_authenticity_severity_equivalence() {
        assert_eq!(provenance_authenticity_severity(0), Severity::Pass);
        for count in 1..=10 {
            assert_eq!(
                provenance_authenticity_severity(count),
                Severity::Error,
                "provenance_authenticity_severity({count}) should be Error"
            );
        }
    }

    #[test]
    fn build_isolation_severity_equivalence() {
        assert_eq!(build_isolation_severity(0), Severity::Pass);
        for count in 1..=10 {
            assert_eq!(
                build_isolation_severity(count),
                Severity::Error,
                "build_isolation_severity({count}) should be Error"
            );
        }
    }
}
