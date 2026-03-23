use anyhow::Result;
use colored::Colorize;
use gh_verify_core::assessment::VerificationResult;
use gh_verify_core::profile::{FindingSeverity, GateDecision};

use crate::verify::BatchReport;

pub fn print(result: &VerificationResult) -> Result<()> {
    let report = &result.report;

    for outcome in &report.outcomes {
        let decision_str = match outcome.decision {
            GateDecision::Pass => "pass".green(),
            GateDecision::Review => "review".yellow(),
            GateDecision::Fail => "fail".red(),
        };

        let severity_str = match outcome.severity {
            FindingSeverity::Info => "info",
            FindingSeverity::Warning => "warning",
            FindingSeverity::Error => "error",
        };

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

pub fn print_batch(batch: &BatchReport) -> Result<()> {
    for pr_report in &batch.pr_reports {
        println!("{}", format!("--- PR #{} ---", pr_report.pr_number).bold());
        print(&pr_report.result)?;
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

    Ok(())
}
