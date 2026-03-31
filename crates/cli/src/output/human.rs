use anyhow::Result;
use colored::Colorize;
use libverify_core::assessment::{BatchReport, VerificationResult};
use libverify_core::profile::GateDecision;

fn format_utc_now() -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    // Simple UTC timestamp without chrono dependency
    let days = secs / 86400;
    let time_secs = secs % 86400;
    let h = time_secs / 3600;
    let m = (time_secs % 3600) / 60;
    let s = time_secs % 60;
    // Days since 1970-01-01
    let mut y = 1970i64;
    let mut remaining = days as i64;
    loop {
        let year_days = if y % 4 == 0 && (y % 100 != 0 || y % 400 == 0) {
            366
        } else {
            365
        };
        if remaining < year_days {
            break;
        }
        remaining -= year_days;
        y += 1;
    }
    let leap = y % 4 == 0 && (y % 100 != 0 || y % 400 == 0);
    let month_days = [
        31,
        if leap { 29 } else { 28 },
        31,
        30,
        31,
        30,
        31,
        31,
        30,
        31,
        30,
        31,
    ];
    let mut month = 12; // fallback to December if loop exhausts
    for (i, &d) in month_days.iter().enumerate() {
        if remaining < d as i64 {
            month = i + 1;
            break;
        }
        remaining -= d as i64;
    }
    let day = remaining + 1;
    format!("{y:04}-{month:02}-{day:02}T{h:02}:{m:02}:{s:02}Z")
}

pub fn remediation_hint(control_id: &str) -> Option<&'static str> {
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
        "change-request-size" => Some(
            "Keep PRs small and focused; split large changes. Monorepo cross-package PRs may false-positive here -- use --exclude change-request-size",
        ),
        "test-coverage" => Some(
            "Add or update tests for changed source files. Dependency-only PRs may false-positive here -- use --exclude test-coverage",
        ),
        "scoped-change" => Some(
            "Limit PR to a single logical change; split unrelated changes. In monorepos, features spanning multiple packages are expected -- use --exclude scoped-change",
        ),
        "issue-linkage" => Some(
            "Reference an issue in the PR body: Fixes #123 or Closes #456. Bot PRs (Dependabot/Renovate) don't link issues -- use --exclude issue-linkage",
        ),
        "description-quality" => Some("Add a meaningful PR description explaining the change"),
        "merge-commit-policy" => {
            Some("Use squash or rebase merge strategy instead of merge commits")
        }
        "conventional-title" => Some(
            "Use Conventional Commits format: type(scope): description. Bot PRs use their own title format -- use --exclude conventional-title",
        ),
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
        "secret-scanning-push-protection" => {
            Some("Enable push protection at Settings > Code security > Secret scanning")
        }
        "branch-protection-admin-enforcement" => {
            Some("Enable 'Include administrators' in branch protection rules")
        }
        "dismiss-stale-reviews-on-push" => {
            Some("Enable 'Dismiss stale pull request approvals when new commits are pushed'")
        }
        "actions-pinned-dependencies" => {
            Some("Pin GitHub Actions to full commit SHAs instead of tags")
        }
        "environment-protection-rules" => {
            Some("Configure environment protection rules at Settings > Environments")
        }
        "code-scanning-alerts-resolved" => {
            Some("Resolve open code scanning alerts at Security > Code scanning alerts")
        }
        "dependency-license-compliance" => {
            Some("Review dependency licenses; remove or replace copyleft dependencies")
        }
        "sbom-attestation" => {
            Some("Generate SBOM with gh attestation or anchore/sbom-action in CI")
        }
        "release-asset-attestation" => {
            Some("Attest release assets with gh attestation or sigstore/cosign")
        }
        "privileged-workflow-detection" => {
            Some("Avoid pull_request_target with checkout of PR code in workflows")
        }
        "workflow-permissions-restricted" => {
            Some("Set default workflow permissions to 'Read' at Settings > Actions > General")
        }
        "dependency-update-tool" => {
            Some("Add .github/dependabot.yml or renovate.json to enable automated dependency updates")
        }
        "repository-permissions-audit" => {
            Some("Reduce admin count (<= 3), use team-based access instead of direct collaborators")
        }
        "default-branch-settings-baseline" => {
            Some("Enable branch protection, admin enforcement, and stale review dismissal on default branch")
        }
        "security-test-in-ci" => {
            Some("Add CodeQL or Semgrep to GitHub Actions: github/codeql-action/analyze")
        }
        "protected-tags" => {
            Some("Add tag protection rules at Settings > Tags to prevent unauthorized releases")
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

        if matches!(outcome.decision, GateDecision::Fail | GateDecision::Review)
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
    let timestamp = format_utc_now();
    print!("Policy: {policy_name} | gh-verify {version} | {timestamp}");
    if !excluded.is_empty() {
        print!(" | Excluded: {}", excluded.join(", "));
    }
    println!();

    Ok(())
}

pub fn print_fleet_matrix(matrix: &crate::FleetMatrix) -> Result<()> {
    let policies = &matrix.policies;

    // Calculate column widths
    let repo_col_width = matrix
        .rows
        .iter()
        .map(|r| r.repo_id.len())
        .max()
        .unwrap_or(10)
        .max(10);
    let policy_col_width = policies.iter().map(|p| p.len()).max().unwrap_or(8).max(14); // "Pass/Total  %" minimum

    // Header
    print!("{:<width$}", "", width = repo_col_width + 2);
    for policy in policies {
        print!("  {:<width$}", policy.bold(), width = policy_col_width);
    }
    println!();

    // Separator
    let total_width = repo_col_width + 2 + policies.len() * (policy_col_width + 2);
    println!("{}", "─".repeat(total_width));

    // Rows
    let mut any_fail = false;
    for row in &matrix.rows {
        print!("{:<width$}", row.repo_id, width = repo_col_width + 2);
        for policy in policies {
            let summary = row.results.get(policy.as_str());
            match summary {
                Some(s) => {
                    let total = s.total();
                    let rate = if total > 0 {
                        (s.pass as f64 / total as f64) * 100.0
                    } else {
                        0.0
                    };
                    let cell = format!("{}/{} {:>3.0}%", s.pass, total, rate);
                    let colored_cell = if s.fail > 0 {
                        any_fail = true;
                        cell.red()
                    } else if s.review > 0 {
                        cell.yellow()
                    } else {
                        cell.green()
                    };
                    print!("  {:<width$}", colored_cell, width = policy_col_width);
                }
                None => {
                    print!("  {:<width$}", "-", width = policy_col_width);
                }
            }
        }
        println!();
    }

    // Separator
    println!("{}", "─".repeat(total_width));

    // Fleet average
    print!(
        "{:<width$}",
        "Fleet average".bold(),
        width = repo_col_width + 2
    );
    for policy in policies {
        let (total_pass, total_all) = matrix.rows.iter().fold((0usize, 0usize), |(p, t), row| {
            match row.results.get(policy.as_str()) {
                Some(s) => (p + s.pass, t + s.total()),
                None => (p, t),
            }
        });
        let rate = if total_all > 0 {
            (total_pass as f64 / total_all as f64) * 100.0
        } else {
            0.0
        };
        print!(
            "  {:<width$}",
            format!("{rate:.0}%").bold(),
            width = policy_col_width
        );
    }
    println!();
    println!();

    // Hotspots (worst controls)
    if !matrix.control_hotspots.is_empty() && any_fail {
        println!("{}", "Worst controls across fleet:".bold());
        for hotspot in matrix.control_hotspots.iter().take(5) {
            let parts: Vec<String> = hotspot
                .fail_by_policy
                .iter()
                .map(|(policy, count)| {
                    format!("{count}/{} repos fail ({policy})", hotspot.total_repos)
                })
                .collect();
            println!("  {:<35} {}", hotspot.control_id, parts.join(", ").red());
        }
        println!();
    }

    let version = env!("GH_VERIFY_VERSION");
    let timestamp = format_utc_now();
    println!(
        "Policies: {} | gh-verify {version} | {timestamp}",
        policies.join(", ")
    );

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
