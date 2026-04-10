use anyhow::Result;
use colored::Colorize;
use libverify_core::assessment::{BatchReport, VerificationResult};
use crate::remediation_hint;
use libverify_core::profile::GateDecision;
use libverify_output::utc_now_rfc3339;

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
            && let Some(hint) = remediation_hint(outcome.control_id.as_str())
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
    let timestamp = utc_now_rfc3339();
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
    let timestamp = utc_now_rfc3339();
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
