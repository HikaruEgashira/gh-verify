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

/// Core predicate for PR coverage.
/// Verified by Creusot in `gh-verify-verif` crate.
pub fn is_uncovered_commit(is_merge: bool, has_pr: bool) -> bool {
    !is_merge && !has_pr
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

/// Core predicate for PR coverage severity.
/// Verified by Creusot in `gh-verify-verif` crate.
pub fn pr_coverage_severity(uncovered_count: usize) -> Severity {
    if uncovered_count == 0 {
        Severity::Pass
    } else {
        Severity::Error
    }
}

/// Core predicate for branch protection severity.
/// Pass iff both stale review dismissal and admin enforcement are enabled.
///
/// Verified by Creusot in `gh-verify-verif` crate.
pub fn branch_protection_severity(dismiss_stale: bool, enforce_admins: bool) -> Severity {
    if dismiss_stale && enforce_admins {
        Severity::Pass
    } else {
        Severity::Error
    }
}

/// Core predicate for required reviewers severity.
/// Pass iff at least one reviewer is required.
///
/// Verified by Creusot in `gh-verify-verif` crate.
pub fn required_reviewers_severity(required_reviews: u32) -> Severity {
    if required_reviews >= 1 {
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
    fn is_uncovered_commit_exhaustive_equivalence() {
        for &merge in &[false, true] {
            for &has_pr in &[false, true] {
                let result = is_uncovered_commit(merge, has_pr);
                let spec = !merge && !has_pr;
                assert_eq!(
                    result, spec,
                    "is_uncovered_commit({merge}, {has_pr}): got {result}, spec {spec}"
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

    #[test]
    fn pr_coverage_severity_equivalence() {
        assert_eq!(pr_coverage_severity(0), Severity::Pass);
        for count in 1..=100 {
            assert_eq!(
                pr_coverage_severity(count),
                Severity::Error,
                "pr_coverage_severity({count}) should be Error"
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
    fn branch_protection_severity_exhaustive() {
        for &ds in &[false, true] {
            for &ea in &[false, true] {
                let result = branch_protection_severity(ds, ea);
                let spec = if ds && ea { Severity::Pass } else { Severity::Error };
                assert_eq!(
                    result, spec,
                    "branch_protection_severity({ds}, {ea}): got {result:?}, spec {spec:?}"
                );
            }
        }
    }

    #[test]
    fn required_reviewers_severity_equivalence() {
        assert_eq!(required_reviewers_severity(0), Severity::Error);
        for count in 1..=10 {
            assert_eq!(
                required_reviewers_severity(count),
                Severity::Pass,
                "required_reviewers_severity({count}) should be Pass"
            );
        }
    }
}
