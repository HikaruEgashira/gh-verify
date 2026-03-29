use anyhow::Result;
use colored::Colorize;
use libverify_core::assessment::{BatchReport, VerificationResult};
use libverify_core::profile::GateDecision;

fn remediation_hint(control_id: &str) -> Option<&'static str> {
    match control_id {
        "source-authenticity" => Some("Sign commits: git config commit.gpgsign true"),
        "review-independence" => Some("Ensure PRs are reviewed by someone other than the author"),
        "branch-history-integrity" => {
            Some("Use linear history (rebase/squash, avoid merge commits)")
        }
        "branch-protection-enforcement" => {
            Some("Enable branch protection rules at Settings > Branches")
        }
        "two-party-review" => Some("Require at least 2 reviewers in branch protection rules"),
        "required-status-checks" => Some("Add required status checks in branch protection rules"),
        "build-provenance" => {
            Some("Generate SLSA provenance with slsa-framework/slsa-github-generator")
        }
        "hosted-build-platform" => Some("Use GitHub-hosted runners instead of self-hosted"),
        "provenance-authenticity" => {
            Some("Verify build provenance signatures with cosign/slsa-verifier")
        }
        "build-isolation" => Some("Ensure builds run in ephemeral, isolated environments"),
        "dependency-signature" => Some("Use signed dependencies; verify with cosign or sigstore"),
        "dependency-provenance" => Some("Ensure dependencies publish SLSA provenance attestations"),
        "dependency-signer-verified" => Some("Verify dependency signers against a trusted list"),
        "dependency-completeness" => Some("Ensure all transitive dependencies have provenance"),
        "change-request-size" => Some("Keep PRs small and focused; split large changes"),
        "test-coverage" => Some("Add or update tests for changed source files"),
        "scoped-change" => Some("Limit PR to a single logical change; split unrelated changes"),
        "issue-linkage" => Some("Reference an issue in the PR body: Fixes #123 or Closes #456"),
        "description-quality" => Some("Add a meaningful PR description explaining the change"),
        "merge-commit-policy" => {
            Some("Use squash or rebase merge strategy instead of merge commits")
        }
        "conventional-title" => Some("Use Conventional Commits format: type(scope): description"),
        "stale-review" => Some("Re-request review if changes were pushed after approval"),
        "security-file-change" => Some("Security-sensitive file changes require additional review"),
        "release-traceability" => Some("Link release to merged PRs and resolved issues"),
        "codeowners-coverage" => Some("Add a CODEOWNERS file to define code ownership"),
        "secret-scanning" => {
            Some("Enable secret scanning at Settings > Code security and analysis")
        }
        "vulnerability-scanning" => {
            Some("Enable Dependabot alerts at Settings > Code security and analysis")
        }
        "security-policy" => {
            Some("Add a SECURITY.md file with vulnerability reporting instructions")
        }
        _ => None,
    }
}

pub fn print(
    result: &VerificationResult,
    only_failures: bool,
    policy: Option<&str>,
    excluded: &[String],
) -> Result<()> {
    let report = &result.report;

    for outcome in &report.outcomes {
        if only_failures && outcome.decision != GateDecision::Fail {
            continue;
        }

        let decision_str = match outcome.decision {
            GateDecision::Pass => "PASS".green(),
            GateDecision::Review => "REVIEW".yellow(),
            GateDecision::Fail => "FAIL".red(),
        };

        let severity_str = report.severity_labels.label_for(outcome.severity);

        println!(
            "[{}] {} ({}): {}",
            outcome.control_id.to_string().bold(),
            decision_str,
            severity_str,
            outcome.rationale
        );

        if outcome.decision == GateDecision::Fail
            && let Some(hint) = remediation_hint(&outcome.control_id.to_string())
        {
            println!("  -> {} {}", "Hint:".cyan(), hint);
        }
    }

    let (mut pass_count, mut review_count, mut fail_count) = (0usize, 0usize, 0usize);
    for outcome in &report.outcomes {
        match outcome.decision {
            GateDecision::Pass => pass_count += 1,
            GateDecision::Review => review_count += 1,
            GateDecision::Fail => fail_count += 1,
        }
    }

    println!();
    if only_failures {
        let hidden: Vec<String> = [(pass_count, "pass"), (review_count, "review")]
            .iter()
            .filter(|(c, _)| *c > 0)
            .map(|(c, label)| format!("{c} {label}"))
            .collect();

        if hidden.is_empty() {
            println!("Summary: {fail_count} fail");
        } else {
            println!("Summary: {fail_count} fail ({} hidden)", hidden.join(", "));
        }
    } else {
        let total = pass_count + review_count + fail_count;
        let pass_rate = if total > 0 {
            (pass_count as f64 / total as f64) * 100.0
        } else {
            0.0
        };
        if excluded.is_empty() {
            println!(
                "Summary: {pass_count} pass, {review_count} review, {fail_count} fail ({pass_rate:.0}% pass rate)"
            );
        } else {
            println!(
                "Summary: {pass_count} pass, {review_count} review, {fail_count} fail ({pass_rate:.0}% pass rate, {} excluded)",
                excluded.len()
            );
        }
    }

    let policy_name = policy.unwrap_or("default");
    let version = env!("GH_VERIFY_VERSION");
    print!("Policy: {policy_name} | gh-verify {version}");
    if !excluded.is_empty() {
        print!(" | Excluded: {}", excluded.join(", "));
    }
    println!();

    Ok(())
}

pub fn print_batch(
    batch: &BatchReport,
    only_failures: bool,
    policy: Option<&str>,
    excluded: &[String],
) -> Result<()> {
    for entry in &batch.reports {
        println!("{}", format!("--- {} ---", entry.subject_id).bold());
        print(&entry.result, only_failures, policy, excluded)?;
        println!();
    }

    for skipped in &batch.skipped {
        println!(
            "{} {}: {}",
            "SKIPPED".yellow(),
            skipped.subject_id,
            skipped.reason
        );
    }

    println!(
        "{}",
        format!(
            "Batch summary: {} pass, {} review, {} fail, {} skipped",
            batch.total_pass,
            batch.total_review,
            batch.total_fail,
            batch.skipped.len()
        )
        .bold()
    );

    let failed_entries: Vec<&str> = batch
        .reports
        .iter()
        .filter(|entry| {
            entry
                .result
                .report
                .outcomes
                .iter()
                .any(|o| o.decision == GateDecision::Fail)
        })
        .map(|entry| entry.subject_id.as_str())
        .collect();

    if !failed_entries.is_empty() {
        let list = failed_entries.join(", ");
        println!("{}", format!("Failed: {list}").red().bold());
    }

    Ok(())
}
