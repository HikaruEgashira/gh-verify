use anyhow::Result;
use gh_verify_core::assessment::VerificationResult;
use gh_verify_core::profile::GateDecision;

use crate::verify::BatchReport;

pub fn print(result: &VerificationResult, only_failures: bool) -> Result<()> {
    if only_failures {
        let filtered = filter_result(result);
        let json = serde_json::to_string_pretty(&filtered)?;
        println!("{json}");
    } else {
        let json = serde_json::to_string_pretty(result)?;
        println!("{json}");
    }
    Ok(())
}

pub fn print_batch(batch: &BatchReport, only_failures: bool) -> Result<()> {
    if only_failures {
        let filtered = filter_batch(batch);
        let json = serde_json::to_string_pretty(&filtered)?;
        println!("{json}");
    } else {
        let json = serde_json::to_string_pretty(batch)?;
        println!("{json}");
    }
    Ok(())
}

fn filter_result(result: &VerificationResult) -> VerificationResult {
    let report = &result.report;
    let mut filtered_findings = Vec::new();
    let mut filtered_outcomes = Vec::new();

    for (finding, outcome) in report.findings.iter().zip(report.outcomes.iter()) {
        if outcome.decision == GateDecision::Fail {
            filtered_findings.push(finding.clone());
            filtered_outcomes.push(outcome.clone());
        }
    }

    VerificationResult {
        report: gh_verify_core::assessment::AssessmentReport {
            profile_name: report.profile_name.clone(),
            findings: filtered_findings,
            outcomes: filtered_outcomes,
            severity_labels: report.severity_labels.clone(),
        },
        evidence: result.evidence.clone(),
    }
}

fn filter_batch(batch: &BatchReport) -> BatchReport {
    BatchReport {
        pr_reports: batch
            .pr_reports
            .iter()
            .map(|pr| crate::verify::PrReport {
                pr_number: pr.pr_number,
                result: filter_result(&pr.result),
            })
            .collect(),
        total_pass: batch.total_pass,
        total_review: batch.total_review,
        total_fail: batch.total_fail,
        skipped: batch.skipped.clone(),
    }
}
