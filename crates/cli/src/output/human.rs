use anyhow::Result;
use colored::Colorize;
use gh_verify_core::assessment::AssessmentReport;
use gh_verify_core::profile::{FindingSeverity, GateDecision};

pub fn print(report: &AssessmentReport) -> Result<()> {
    println!(
        "{}",
        format!("Assessment profile: {}", report.profile_name).bold()
    );
    println!();

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
