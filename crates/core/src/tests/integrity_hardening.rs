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
fn short_sha_6_chars_returns_full() {
    // Kills: >= 7 → >= 6
    assert_eq!(short_sha("abcdef"), "abcdef");
}

#[test]
fn short_sha_8_chars_truncates() {
    assert_eq!(short_sha("abcdefgh"), "abcdefg");
}

#[test]
fn short_sha_empty_string() {
    assert_eq!(short_sha(""), "");
}

// --- is_merge boundary mutations ---

#[test]
fn is_merge_parent_count_exactly_2_is_merge() {
    // Kills: >= 2 → > 2
    let c = Commit {
        sha: "a".into(),
        message: "not merge".into(),
        verified: true,
        author_login: None,
        parent_count: Some(2),
    };
    assert!(c.is_merge());
}

#[test]
fn is_merge_parent_count_1_not_merge() {
    // Kills: >= 2 → >= 1
    let c = Commit {
        sha: "a".into(),
        message: "not merge".into(),
        verified: true,
        author_login: None,
        parent_count: Some(1),
    };
    assert!(!c.is_merge());
}

#[test]
fn is_merge_parent_count_0_not_merge() {
    let c = Commit {
        sha: "a".into(),
        message: "not merge".into(),
        verified: true,
        author_login: None,
        parent_count: Some(0),
    };
    assert!(!c.is_merge());
}

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

// --- check_commit_signatures mutations ---

#[test]
fn signatures_empty_commits_is_pass() {
    // Kills: returning Error for empty input
    let result = check_commit_signatures(&[]);
    assert_eq!(result.severity, Severity::Pass);
}

#[test]
fn signatures_single_unsigned_message_format() {
    // Kills: swapping unsigned.len() and commits.len() in message
    let commits = vec![make_commit("aaa1234567", false, "alice")];
    let result = check_commit_signatures(&commits);
    assert!(result.message.contains("1 of 1"));
}

#[test]
fn signatures_mixed_message_counts() {
    // Kills: off-by-one in count formatting
    let commits = vec![
        make_commit("aaa1234567", true, "alice"),
        make_commit("bbb1234567", false, "bob"),
        make_commit("ccc1234567", false, "carol"),
    ];
    let result = check_commit_signatures(&commits);
    assert!(result.message.contains("2 of 3"));
    assert_eq!(result.affected_files.len(), 2);
}

#[test]
fn signatures_suggestion_present_on_error() {
    // Kills: removing suggestion field
    let commits = vec![make_commit("aaa1234567", false, "alice")];
    let result = check_commit_signatures(&commits);
    assert!(result.suggestion.is_some());
    assert!(result.suggestion.unwrap().contains("gpgsign"));
}

// --- check_mutual_approval mutations ---

#[test]
fn mutual_approval_empty_prs_is_pass() {
    let result = check_mutual_approval(&[]);
    assert_eq!(result.severity, Severity::Pass);
}

#[test]
fn mutual_approval_mixed_one_self_one_independent() {
    // Kills: returning Pass when any PR has independent (should require ALL)
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
    assert!(result.message.contains("1 PR"));
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

#[test]
fn mutual_approval_affected_files_contains_pr_numbers() {
    // Kills: mutation of affected_files formatting
    let prs = vec![PrWithReviews {
        pr_number: 42,
        pr_author: "alice".into(),
        commit_authors: vec!["alice".into()],
        approvers: vec!["alice".into()],
    }];
    let result = check_mutual_approval(&prs);
    assert!(result.affected_files.iter().any(|f| f.contains("42")));
}

// --- check_pr_coverage: exhaustive truth table ---

#[test]
fn pr_coverage_empty_assocs_is_pass() {
    let result = check_pr_coverage(&[]);
    assert_eq!(result.severity, Severity::Pass);
}

#[test]
fn pr_coverage_truth_table() {
    // Kills: any negation flip on is_merge or has_pr
    // (is_merge=false, has_pr=true) → Pass
    assert_eq!(
        check_pr_coverage(&[CommitPrAssoc {
            commit_sha: "a".into(), pr_numbers: vec![1], is_merge: false,
        }]).severity,
        Severity::Pass,
    );
    // (is_merge=true, has_pr=false) → Pass
    assert_eq!(
        check_pr_coverage(&[CommitPrAssoc {
            commit_sha: "a".into(), pr_numbers: vec![], is_merge: true,
        }]).severity,
        Severity::Pass,
    );
    // (is_merge=true, has_pr=true) → Pass
    assert_eq!(
        check_pr_coverage(&[CommitPrAssoc {
            commit_sha: "a".into(), pr_numbers: vec![1], is_merge: true,
        }]).severity,
        Severity::Pass,
    );
    // (is_merge=false, has_pr=false) → Error
    assert_eq!(
        check_pr_coverage(&[CommitPrAssoc {
            commit_sha: "a".into(), pr_numbers: vec![], is_merge: false,
        }]).severity,
        Severity::Error,
    );
}

#[test]
fn pr_coverage_uncovered_count_in_message() {
    let assocs = vec![
        CommitPrAssoc { commit_sha: "aaa1234567".into(), pr_numbers: vec![], is_merge: false },
        CommitPrAssoc { commit_sha: "bbb1234567".into(), pr_numbers: vec![], is_merge: false },
    ];
    let result = check_pr_coverage(&assocs);
    assert!(result.message.contains("2 commits"));
    assert_eq!(result.affected_files.len(), 2);
}

// --- verify_release_integrity mutations ---

#[test]
fn verify_integrity_single_sig_failure_only() {
    // Kills: including pass results when failures exist
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

#[test]
fn signature_severity_usize_max() {
    assert_eq!(signature_severity(usize::MAX), Severity::Error);
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

#[test]
fn pass_result_has_correct_rule_id() {
    let commits = vec![make_commit("aaa", true, "alice")];
    let result = check_commit_signatures(&commits);
    assert_eq!(result.rule_id, "verify-release-integrity");
}
