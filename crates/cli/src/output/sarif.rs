use anyhow::Result;
use gh_verify_core::assessment::AssessmentReport;
use gh_verify_core::control::ControlId;
use gh_verify_core::profile::FindingSeverity;

const VERSION: &str = env!("GH_VERIFY_VERSION");

pub fn print(report: &AssessmentReport) -> Result<()> {
    let sarif = build_sarif(report);
    println!("{}", serde_json::to_string_pretty(&sarif)?);
    Ok(())
}

fn build_sarif(report: &AssessmentReport) -> serde_json::Value {
    let mut seen_rules = Vec::new();
    let rules: Vec<serde_json::Value> = report
        .outcomes
        .iter()
        .filter_map(|o| {
            if seen_rules.contains(&o.control_id) {
                return None;
            }
            seen_rules.push(o.control_id);
            Some(rule_descriptor(o.control_id))
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

fn rule_descriptor(id: ControlId) -> serde_json::Value {
    let desc = match id {
        ControlId::SourceAuthenticity => "All commits must carry verified signatures",
        ControlId::ReviewIndependence => "Four-eyes: approver must differ from author",
        ControlId::BranchHistoryIntegrity => {
            "Branch history must be continuous and protected from force-push"
        }
        ControlId::BranchProtectionEnforcement => {
            "Branch protection rules must be continuously enforced"
        }
        ControlId::TwoPartyReview => "At least two independent reviewers must approve changes",
        ControlId::BuildProvenance => "Artifacts must have verified SLSA provenance",
        ControlId::RequiredStatusChecks => "At least one required status check must be configured",
        ControlId::HostedBuildPlatform => {
            "Build must run on a hosted platform, not a developer workstation"
        }
        ControlId::ProvenanceAuthenticity => {
            "Provenance attestation must be cryptographically signed"
        }
        ControlId::BuildIsolation => "Build must run in an isolated, ephemeral environment",
        ControlId::PrSize => "PR size must be within acceptable limits",
        ControlId::TestCoverage => "Source changes must include matching test updates",
        ControlId::ScopedChange => "PR changes must be well-scoped (single logical unit)",
        ControlId::IssueLinkage => "PR must reference at least one issue or ticket",
        ControlId::StaleReview => "Approvals must postdate the latest source revision",
        ControlId::DescriptionQuality => "Change requests must include a meaningful description",
        ControlId::MergeCommitPolicy => {
            "Source revisions must follow linear history (no merge commits)"
        }
        ControlId::ConventionalTitle => {
            "Change request titles must follow Conventional Commits format"
        }
        ControlId::SecurityFileChange => {
            "Changes to security-sensitive files require heightened scrutiny"
        }
        ControlId::ReleaseTraceability => "Release batches must trace to governed change requests",
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
    use gh_verify_core::control::{ControlFinding, ControlId};
    use gh_verify_core::profile::{GateDecision, ProfileOutcome};

    fn sample_report() -> AssessmentReport {
        AssessmentReport {
            profile_name: "slsa-source-l1-build-l1".to_string(),
            findings: vec![
                ControlFinding::satisfied(
                    ControlId::ReviewIndependence,
                    "Independent reviewer approved",
                    vec!["github_pr:owner/repo#1".to_string()],
                ),
                ControlFinding::violated(
                    ControlId::SourceAuthenticity,
                    "1 unsigned commit",
                    vec!["github_pr:owner/repo#1".to_string()],
                ),
            ],
            outcomes: vec![
                ProfileOutcome {
                    control_id: ControlId::ReviewIndependence,
                    severity: FindingSeverity::Info,
                    decision: GateDecision::Pass,
                    rationale: "Independent reviewer approved".to_string(),
                },
                ProfileOutcome {
                    control_id: ControlId::SourceAuthenticity,
                    severity: FindingSeverity::Error,
                    decision: GateDecision::Fail,
                    rationale: "1 unsigned commit".to_string(),
                },
            ],
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
