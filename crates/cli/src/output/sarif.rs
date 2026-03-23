use anyhow::Result;
use libverify_core::assessment::{AssessmentReport, BatchReport, VerificationResult};
use libverify_core::control::builtin;
use libverify_core::control::ControlId;
use libverify_core::profile::FindingSeverity;

const VERSION: &str = env!("GH_VERIFY_VERSION");

pub fn print(result: &VerificationResult, only_failures: bool) -> Result<()> {
    let mut sarif = build_sarif(&result.report);
    if only_failures {
        filter_sarif_runs(&mut sarif);
    }
    if let Some(evidence) = &result.evidence {
        if let Some(run) = sarif["runs"].as_array_mut().and_then(|a| a.first_mut()) {
            run["properties"]["evidence"] = serde_json::to_value(evidence)?;
        }
    }
    println!("{}", serde_json::to_string_pretty(&sarif)?);
    Ok(())
}

pub fn print_batch(batch: &BatchReport, only_failures: bool) -> Result<()> {
    let mut runs = Vec::new();
    for entry in &batch.reports {
        let mut sarif = build_sarif(&entry.result.report);
        if only_failures {
            filter_sarif_runs(&mut sarif);
        }
        if let Some(run) = sarif["runs"].as_array().and_then(|a| a.first()) {
            let mut run = run.clone();
            let mut props = serde_json::json!({ "subjectId": entry.subject_id });
            if let Some(evidence) = &entry.result.evidence {
                props["evidence"] = serde_json::to_value(evidence)?;
            }
            run["properties"] = props;
            runs.push(run);
        }
    }
    let sarif = serde_json::json!({
        "$schema": "https://raw.githubusercontent.com/oasis-tcs/sarif-spec/main/sarif-2.1/schema/sarif-schema-2.1.0.json",
        "version": "2.1.0",
        "runs": runs,
    });
    println!("{}", serde_json::to_string_pretty(&sarif)?);
    Ok(())
}

fn build_sarif(report: &AssessmentReport) -> serde_json::Value {
    let mut seen_rules: Vec<String> = Vec::new();
    let rules: Vec<serde_json::Value> = report
        .outcomes
        .iter()
        .filter_map(|o| {
            let id_str = o.control_id.as_str().to_string();
            if seen_rules.contains(&id_str) {
                return None;
            }
            seen_rules.push(id_str);
            Some(rule_descriptor(&o.control_id))
        })
        .collect();

    let results: Vec<serde_json::Value> = report
        .findings
        .iter()
        .zip(report.outcomes.iter())
        .map(|(finding, outcome)| {
            let mut result = serde_json::json!({
                "ruleId": outcome.control_id.as_str(),
                "level": severity_to_level(outcome.severity),
                "message": { "text": outcome.rationale },
                "properties": {
                    "decision": outcome.decision.as_str(),
                    "controlStatus": finding.status.as_str(),
                },
            });

            if !finding.subjects.is_empty() {
                let locations: Vec<serde_json::Value> = finding
                    .subjects
                    .iter()
                    .map(|s| {
                        serde_json::json!({
                            "logicalLocations": [{
                                "fullyQualifiedName": s,
                                "kind": "resource",
                            }]
                        })
                    })
                    .collect();
                result["locations"] = serde_json::Value::Array(locations);
            }

            if !finding.evidence_gaps.is_empty() {
                let gaps: Vec<String> = finding
                    .evidence_gaps
                    .iter()
                    .map(|g| format!("{g}"))
                    .collect();
                result["properties"]["evidenceGaps"] = serde_json::json!(gaps);
            }

            result
        })
        .collect();

    serde_json::json!({
        "$schema": "https://raw.githubusercontent.com/oasis-tcs/sarif-spec/main/sarif-2.1/schema/sarif-schema-2.1.0.json",
        "version": "2.1.0",
        "runs": [{
            "tool": {
                "driver": {
                    "name": "gh-verify",
                    "version": VERSION,
                    "informationUri": "https://github.com/HikaruEgashira/gh-verify",
                    "rules": rules,
                }
            },
            "results": results,
        }]
    })
}

fn filter_sarif_runs(sarif: &mut serde_json::Value) {
    if let Some(runs) = sarif["runs"].as_array_mut() {
        for run in runs.iter_mut() {
            if let Some(results) = run["results"].as_array() {
                let filtered: Vec<serde_json::Value> = results
                    .iter()
                    .filter(|r| r["level"].as_str() == Some("error"))
                    .cloned()
                    .collect();
                run["results"] = serde_json::Value::Array(filtered);
            }
        }
    }
}

fn rule_descriptor(id: &ControlId) -> serde_json::Value {
    let desc = match id.as_str() {
        builtin::SOURCE_AUTHENTICITY => "All commits must carry verified signatures",
        builtin::REVIEW_INDEPENDENCE => "Four-eyes: approver must differ from author",
        builtin::BRANCH_HISTORY_INTEGRITY => {
            "Branch history must be continuous and protected from force-push"
        }
        builtin::BRANCH_PROTECTION_ENFORCEMENT => {
            "Branch protection rules must be continuously enforced"
        }
        builtin::TWO_PARTY_REVIEW => "At least two independent reviewers must approve changes",
        builtin::BUILD_PROVENANCE => "Artifacts must have verified SLSA provenance",
        builtin::REQUIRED_STATUS_CHECKS => {
            "At least one required status check must be configured"
        }
        builtin::HOSTED_BUILD_PLATFORM => {
            "Build must run on a hosted platform, not a developer workstation"
        }
        builtin::PROVENANCE_AUTHENTICITY => {
            "Provenance attestation must be cryptographically signed"
        }
        builtin::BUILD_ISOLATION => "Build must run in an isolated, ephemeral environment",
        builtin::CHANGE_REQUEST_SIZE => {
            "Change request size must be within acceptable limits"
        }
        builtin::TEST_COVERAGE => "Source changes must include matching test updates",
        builtin::SCOPED_CHANGE => {
            "Change request changes must be well-scoped (single logical unit)"
        }
        builtin::ISSUE_LINKAGE => {
            "Change request must reference at least one issue or ticket"
        }
        builtin::STALE_REVIEW => "Approvals must postdate the latest source revision",
        builtin::DESCRIPTION_QUALITY => {
            "Change requests must include a meaningful description"
        }
        builtin::MERGE_COMMIT_POLICY => {
            "Source revisions must follow linear history (no merge commits)"
        }
        builtin::CONVENTIONAL_TITLE => {
            "Change request titles must follow Conventional Commits format"
        }
        builtin::SECURITY_FILE_CHANGE => {
            "Changes to security-sensitive files require heightened scrutiny"
        }
        builtin::RELEASE_TRACEABILITY => {
            "Release batches must trace to governed change requests"
        }
        _ => "Unknown control",
    };
    serde_json::json!({
        "id": id.as_str(),
        "shortDescription": { "text": desc },
    })
}

fn severity_to_level(severity: FindingSeverity) -> &'static str {
    match severity {
        FindingSeverity::Info => "note",
        FindingSeverity::Warning => "warning",
        FindingSeverity::Error => "error",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use libverify_core::control::{ControlFinding, builtin};
    use libverify_core::profile::{GateDecision, ProfileOutcome};

    fn sample_report() -> AssessmentReport {
        AssessmentReport {
            profile_name: "slsa-source-l1-build-l1".to_string(),
            findings: vec![
                ControlFinding::satisfied(
                    builtin::id(builtin::REVIEW_INDEPENDENCE),
                    "Independent reviewer approved",
                    vec!["github_pr:owner/repo#1".to_string()],
                ),
                ControlFinding::violated(
                    builtin::id(builtin::SOURCE_AUTHENTICITY),
                    "1 unsigned commit",
                    vec!["github_pr:owner/repo#1".to_string()],
                ),
            ],
            outcomes: vec![
                ProfileOutcome {
                    control_id: builtin::id(builtin::REVIEW_INDEPENDENCE),
                    severity: FindingSeverity::Info,
                    decision: GateDecision::Pass,
                    rationale: "Independent reviewer approved".to_string(),
                },
                ProfileOutcome {
                    control_id: builtin::id(builtin::SOURCE_AUTHENTICITY),
                    severity: FindingSeverity::Error,
                    decision: GateDecision::Fail,
                    rationale: "1 unsigned commit".to_string(),
                },
            ],
            severity_labels: Default::default(),
        }
    }

    #[test]
    fn sarif_version_is_2_1_0() {
        let sarif = build_sarif(&sample_report());
        assert_eq!(sarif["version"], "2.1.0");
    }

    #[test]
    fn sarif_results_length_matches_outcomes() {
        let sarif = build_sarif(&sample_report());
        let results = sarif["runs"][0]["results"].as_array().unwrap();
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn sarif_severity_mapping() {
        let sarif = build_sarif(&sample_report());
        let results = sarif["runs"][0]["results"].as_array().unwrap();
        assert_eq!(results[0]["level"], "note");
        assert_eq!(results[1]["level"], "error");
    }

    #[test]
    fn sarif_rule_ids() {
        let sarif = build_sarif(&sample_report());
        let rules = sarif["runs"][0]["tool"]["driver"]["rules"]
            .as_array()
            .unwrap();
        assert_eq!(rules.len(), 2);
        assert_eq!(rules[0]["id"], "review-independence");
        assert_eq!(rules[1]["id"], "source-authenticity");
    }

    #[test]
    fn sarif_logical_locations() {
        let sarif = build_sarif(&sample_report());
        let loc = &sarif["runs"][0]["results"][0]["locations"][0]["logicalLocations"][0];
        assert_eq!(loc["fullyQualifiedName"], "github_pr:owner/repo#1");
        assert_eq!(loc["kind"], "resource");
    }

    #[test]
    fn sarif_decision_in_properties() {
        let sarif = build_sarif(&sample_report());
        assert_eq!(
            sarif["runs"][0]["results"][0]["properties"]["decision"],
            "pass"
        );
        assert_eq!(
            sarif["runs"][0]["results"][1]["properties"]["decision"],
            "fail"
        );
    }
}
