use anyhow::Result;
use colored::Colorize;
use libverify_core::assessment::{BatchReport, VerificationResult};
use libverify_core::profile::GateDecision;

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
    for entry in &batch.reports {
        println!("{}", format!("--- {} ---", entry.subject_id).bold());
        print(&entry.result, only_failures)?;
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
