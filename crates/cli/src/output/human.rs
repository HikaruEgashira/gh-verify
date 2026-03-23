use anyhow::Result;
use colored::Colorize;
use gh_verify_core::assessment::VerificationResult;
use gh_verify_core::profile::GateDecision;

use crate::verify::BatchReport;

pub fn print(result: &VerificationResult, only_failures: bool) -> Result<()> {
    let report = &result.report;

    for outcome in &report.outcomes {
        if only_failures && outcome.decision != GateDecision::Fail {
            continue;
        }

        let decision_str = match outcome.decision {
            GateDecision::Pass => "pass".green(),
            GateDecision::Review => "review".yellow(),
            GateDecision::Fail => "fail".red(),
        };

        let severity_str = report.severity_labels.label_for(outcome.severity);

        println!(
            "{} {} [{}]: {}",
            format!("[{}]", outcome.control_id).bold(),
            decision_str,
            severity_str,
            outcome.rationale
        );
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
    println!("Summary: {pass_count} pass, {review_count} review, {fail_count} fail");

    Ok(())
}

pub fn print_batch(batch: &BatchReport, only_failures: bool) -> Result<()> {
    for pr_report in &batch.pr_reports {
        println!("{}", format!("--- PR #{} ---", pr_report.pr_number).bold());
        print(&pr_report.result, only_failures)?;
        println!();
    }

    for skipped in &batch.skipped {
        println!(
            "{} PR #{}: {}",
            "SKIPPED".yellow(),
            skipped.pr_number,
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

    let failed_prs: Vec<u32> = batch
        .pr_reports
        .iter()
        .filter(|pr| {
            pr.result
                .report
                .outcomes
                .iter()
                .any(|o| o.decision == GateDecision::Fail)
        })
        .map(|pr| pr.pr_number)
        .collect();

    if !failed_prs.is_empty() {
        let pr_list = failed_prs
            .iter()
            .map(|n| format!("#{n}"))
            .collect::<Vec<_>>()
            .join(", ");
        println!("{}", format!("Failed PRs: {pr_list}").red().bold());
    }

    Ok(())
}
