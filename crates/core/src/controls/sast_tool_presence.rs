use crate::control::{Control, ControlFinding, ControlId};
use crate::evidence::{CheckConclusion, EvidenceBundle, EvidenceState};

/// Known SAST tool name patterns (case-insensitive substring match).
const SAST_PATTERNS: &[&str] = &[
    "codeql",
    "semgrep",
    "snyk",
    "sonar",
    "fortify",
    "checkmarx",
    "veracode",
    "bandit",
    "brakeman",
    "gosec",
    "clippy",
    "rust-clippy",
    "eslint-security",
    "safety",
    "trivy",
    "grype",
];

/// Verifies that at least one SAST (Static Application Security Testing) tool
/// ran as part of CI checks.
///
/// Maps to:
/// - NIST SSDF PW.7: Review and test code for vulnerabilities
/// - OpenSSF Scorecard: SAST check
pub struct SastToolPresenceControl;

impl Control for SastToolPresenceControl {
    fn id(&self) -> ControlId {
        ControlId::SastToolPresence
    }

    fn evaluate(&self, evidence: &EvidenceBundle) -> Vec<ControlFinding> {
        let id = self.id();

        let runs = match &evidence.check_runs {
            EvidenceState::NotApplicable => {
                return vec![ControlFinding::not_applicable(
                    id,
                    "Check runs evidence is not applicable",
                )];
            }
            EvidenceState::Missing { gaps } => {
                return vec![ControlFinding::indeterminate(
                    id,
                    "Check runs evidence is unavailable",
                    vec!["commit".to_string()],
                    gaps.clone(),
                )];
            }
            EvidenceState::Complete { value } | EvidenceState::Partial { value, .. } => value,
        };

        if runs.is_empty() {
            return vec![ControlFinding::indeterminate(
                id,
                "No check runs found on the HEAD commit",
                vec!["commit".to_string()],
                vec![],
            )];
        }

        let sast_runs: Vec<&str> = runs
            .iter()
            .filter(|r| is_sast_tool(&r.name))
            .filter(|r| r.conclusion != CheckConclusion::Pending)
            .map(|r| r.name.as_str())
            .collect();

        if sast_runs.is_empty() {
            vec![ControlFinding::violated(
                id,
                format!(
                    "No SAST tool detected among {} check run(s)",
                    runs.len()
                ),
                vec!["commit".to_string()],
            )]
        } else {
            vec![ControlFinding::satisfied(
                id,
                format!(
                    "{} SAST tool(s) detected: {}",
                    sast_runs.len(),
                    sast_runs.join(", ")
                ),
                sast_runs.into_iter().map(String::from).collect(),
            )]
        }
    }
}

fn is_sast_tool(name: &str) -> bool {
    let lower = name.to_lowercase();
    SAST_PATTERNS.iter().any(|p| lower.contains(p))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::control::ControlStatus;
    use crate::evidence::{CheckConclusion, CheckRunEvidence, EvidenceGap};

    fn run(name: &str, conclusion: CheckConclusion) -> CheckRunEvidence {
        CheckRunEvidence {
            name: name.to_string(),
            conclusion,
        }
    }

    fn bundle_with_runs(runs: EvidenceState<Vec<CheckRunEvidence>>) -> EvidenceBundle {
        EvidenceBundle {
            check_runs: runs,
            ..Default::default()
        }
    }

    #[test]
    fn satisfied_when_codeql_present() {
        let bundle = bundle_with_runs(EvidenceState::complete(vec![
            run("ci/build", CheckConclusion::Success),
            run("CodeQL", CheckConclusion::Success),
        ]));
        let findings = SastToolPresenceControl.evaluate(&bundle);
        assert_eq!(findings[0].status, ControlStatus::Satisfied);
        assert!(findings[0].rationale.contains("CodeQL"));
    }

    #[test]
    fn satisfied_when_semgrep_present() {
        let bundle = bundle_with_runs(EvidenceState::complete(vec![
            run("Semgrep Scan", CheckConclusion::Success),
        ]));
        let findings = SastToolPresenceControl.evaluate(&bundle);
        assert_eq!(findings[0].status, ControlStatus::Satisfied);
    }

    #[test]
    fn violated_when_no_sast_tool() {
        let bundle = bundle_with_runs(EvidenceState::complete(vec![
            run("ci/build", CheckConclusion::Success),
            run("ci/test", CheckConclusion::Success),
        ]));
        let findings = SastToolPresenceControl.evaluate(&bundle);
        assert_eq!(findings[0].status, ControlStatus::Violated);
    }

    #[test]
    fn indeterminate_when_no_runs() {
        let bundle = bundle_with_runs(EvidenceState::complete(vec![]));
        let findings = SastToolPresenceControl.evaluate(&bundle);
        assert_eq!(findings[0].status, ControlStatus::Indeterminate);
    }

    #[test]
    fn indeterminate_when_evidence_missing() {
        let bundle = bundle_with_runs(EvidenceState::missing(vec![
            EvidenceGap::CollectionFailed {
                source: "github".to_string(),
                subject: "check_runs".to_string(),
                detail: "403".to_string(),
            },
        ]));
        let findings = SastToolPresenceControl.evaluate(&bundle);
        assert_eq!(findings[0].status, ControlStatus::Indeterminate);
    }

    #[test]
    fn not_applicable_when_not_applicable() {
        let bundle = bundle_with_runs(EvidenceState::not_applicable());
        let findings = SastToolPresenceControl.evaluate(&bundle);
        assert_eq!(findings[0].status, ControlStatus::NotApplicable);
    }

    #[test]
    fn ignores_pending_sast_runs() {
        let bundle = bundle_with_runs(EvidenceState::complete(vec![
            run("CodeQL", CheckConclusion::Pending),
        ]));
        let findings = SastToolPresenceControl.evaluate(&bundle);
        assert_eq!(findings[0].status, ControlStatus::Violated);
    }

    #[test]
    fn case_insensitive_matching() {
        let bundle = bundle_with_runs(EvidenceState::complete(vec![
            run("CODEQL-ANALYSIS", CheckConclusion::Success),
        ]));
        let findings = SastToolPresenceControl.evaluate(&bundle);
        assert_eq!(findings[0].status, ControlStatus::Satisfied);
    }
}
