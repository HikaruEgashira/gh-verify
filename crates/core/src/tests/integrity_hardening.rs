use super::*;

fn make_commit(sha: &str, verified: bool, author: &str) -> Commit {
    Commit {
        sha: sha.to_string(),
        message: "feat: something".to_string(),
        verified,
        author_login: Some(author.to_string()),
        parent_count: Some(1),
    }
}

// --- short_sha boundary mutations ---

#[test]
fn short_sha_exactly_7_chars() {
    // Kills: >= 7 → > 7
    assert_eq!(short_sha("abcdefg"), "abcdefg");
}

#[test]
fn short_sha_empty_string() {
    assert_eq!(short_sha(""), "");
}

// --- is_merge fallback mutation ---

#[test]
fn is_merge_fallback_non_merge_prefix() {
    // Kills: starts_with("Merge ") → contains("Merge")
    let c = Commit {
        sha: "a".into(),
        message: "Merged something".into(),
        verified: true,
        author_login: None,
        parent_count: None,
    };
    assert!(!c.is_merge());
}

// --- check_commit_signatures boundary ---

#[test]
fn signatures_empty_commits_is_pass() {
    let result = check_commit_signatures(&[]);
    assert_eq!(result.severity, Severity::Pass);
}

// --- check_mutual_approval mutations ---

#[test]
fn mutual_approval_empty_prs_is_pass() {
    let result = check_mutual_approval(&[]);
    assert_eq!(result.severity, Severity::Pass);
}

#[test]
fn mutual_approval_mixed_one_self_one_independent() {
    // Kills: returning Pass when ANY PR has independent (should require ALL)
    let prs = vec![
        PrWithReviews {
            pr_number: 1,
            pr_author: "alice".into(),
            commit_authors: vec!["alice".into()],
            approvers: vec!["bob".into()],
        },
        PrWithReviews {
            pr_number: 2,
            pr_author: "carol".into(),
            commit_authors: vec!["carol".into()],
            approvers: vec!["carol".into()],
        },
    ];
    let result = check_mutual_approval(&prs);
    assert_eq!(result.severity, Severity::Error);
}

#[test]
fn mutual_approval_multiple_approvers_one_independent() {
    // Kills: requiring ALL approvers to be independent (only need one)
    let prs = vec![PrWithReviews {
        pr_number: 1,
        pr_author: "alice".into(),
        commit_authors: vec!["alice".into()],
        approvers: vec!["alice".into(), "bob".into()],
    }];
    let result = check_mutual_approval(&prs);
    assert_eq!(result.severity, Severity::Pass);
}

// --- check_pr_coverage boundary ---

#[test]
fn pr_coverage_empty_assocs_is_pass() {
    let result = check_pr_coverage(&[]);
    assert_eq!(result.severity, Severity::Pass);
}

// --- verify_release_integrity mutations ---

#[test]
fn verify_integrity_single_sig_failure_only() {
    let commits = vec![make_commit("aaa1234567", false, "alice")];
    let prs = vec![PrWithReviews {
        pr_number: 1, pr_author: "alice".into(),
        commit_authors: vec!["alice".into()], approvers: vec!["bob".into()],
    }];
    let assocs = vec![CommitPrAssoc {
        commit_sha: "aaa1234567".into(), pr_numbers: vec![1], is_merge: false,
    }];
    let results = verify_release_integrity(&commits, &prs, &assocs);
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].severity, Severity::Error);
}

#[test]
fn verify_integrity_all_three_fail() {
    let commits = vec![make_commit("aaa1234567", false, "alice")];
    let prs = vec![PrWithReviews {
        pr_number: 1, pr_author: "alice".into(),
        commit_authors: vec!["alice".into()], approvers: vec!["alice".into()],
    }];
    let assocs = vec![CommitPrAssoc {
        commit_sha: "aaa1234567".into(), pr_numbers: vec![], is_merge: false,
    }];
    let results = verify_release_integrity(&commits, &prs, &assocs);
    assert_eq!(results.len(), 3);
    assert!(results.iter().all(|r| r.severity == Severity::Error));
}

#[test]
fn verify_integrity_empty_inputs_single_pass() {
    let results = verify_release_integrity(&[], &[], &[]);
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].severity, Severity::Pass);
}

// --- has_independent_approver edge cases ---

#[test]
fn has_independent_approver_multiple_commit_authors() {
    // Kills: only checking first commit author
    let prs = vec![PrWithReviews {
        pr_number: 1,
        pr_author: "alice".into(),
        commit_authors: vec!["alice".into(), "bob".into()],
        approvers: vec!["bob".into()],
    }];
    let result = check_mutual_approval(&prs);
    assert_eq!(result.severity, Severity::Error, "bob is a commit author");
}

#[test]
fn has_independent_approver_third_party() {
    let prs = vec![PrWithReviews {
        pr_number: 1,
        pr_author: "alice".into(),
        commit_authors: vec!["alice".into(), "bob".into()],
        approvers: vec!["carol".into()],
    }];
    let result = check_mutual_approval(&prs);
    assert_eq!(result.severity, Severity::Pass);
}
