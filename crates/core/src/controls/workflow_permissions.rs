use crate::control::{Control, ControlFinding, ControlId};
use crate::evidence::{EvidenceBundle, EvidenceState, GovernedChange};

/// Dangerous permission patterns in GitHub Actions workflows.
const DANGEROUS_PERMISSIONS: &[&str] = &[
    "permissions: write-all",
    "contents: write",
    "packages: write",
    "actions: write",
    "security-events: write",
    "id-token: write",
];

/// Verifies that GitHub Actions workflow files follow the principle of
/// least privilege for token permissions.
///
/// Maps to:
/// - OpenSSF Scorecard: Token-Permissions check
/// - NIST SSDF PS.1: Protect code from unauthorized access
///
/// Detects workflows that grant overly broad write permissions,
/// which could be exploited if a dependency or action is compromised.
pub struct WorkflowPermissionsControl;

impl Control for WorkflowPermissionsControl {
    fn id(&self) -> ControlId {
        ControlId::WorkflowPermissions
    }

    fn evaluate(&self, evidence: &EvidenceBundle) -> Vec<ControlFinding> {
        if evidence.change_requests.is_empty() {
            return vec![ControlFinding::not_applicable(
                self.id(),
                "No change requests found",
            )];
        }

        evidence
            .change_requests
            .iter()
            .map(|cr| evaluate_change(self.id(), cr))
            .collect()
    }
}

fn evaluate_change(id: ControlId, cr: &GovernedChange) -> ControlFinding {
    let cr_subject = cr.id.to_string();

    let assets = match &cr.changed_assets {
        EvidenceState::Complete { value } | EvidenceState::Partial { value, .. } => value,
        EvidenceState::Missing { gaps } => {
            return ControlFinding::indeterminate(
                id,
                format!("{cr_subject}: changed asset evidence could not be collected"),
                vec![cr_subject],
                gaps.clone(),
            );
        }
        EvidenceState::NotApplicable => {
            return ControlFinding::not_applicable(id, "Changed assets not applicable");
        }
    };

    let workflow_assets: Vec<_> = assets
        .iter()
        .filter(|a| is_workflow_path(&a.path))
        .filter(|a| a.diff.is_some())
        .collect();

    if workflow_assets.is_empty() {
        return ControlFinding::not_applicable(
            id,
            format!("{cr_subject}: no workflow files with diffs to analyze"),
        );
    }

    let mut violations: Vec<String> = Vec::new();

    for asset in &workflow_assets {
        if let Some(diff) = &asset.diff {
            for line in diff.lines() {
                if !line.starts_with('+') || line.starts_with("+++") {
                    continue;
                }
                let trimmed = line.trim_start_matches('+').trim().to_lowercase();
                for pattern in DANGEROUS_PERMISSIONS {
                    if trimmed.contains(pattern) {
                        violations.push(format!("{}:{}", asset.path, pattern));
                    }
                }
            }
        }
    }

    if violations.is_empty() {
        ControlFinding::satisfied(
            id,
            format!(
                "{cr_subject}: no excessive permissions detected in workflow files"
            ),
            vec![cr_subject],
        )
    } else {
        ControlFinding::violated(
            id,
            format!(
                "{cr_subject}: {} excessive permission(s) detected: {}",
                violations.len(),
                violations.join(", ")
            ),
            violations,
        )
    }
}

fn is_workflow_path(path: &str) -> bool {
    path.starts_with(".github/workflows/") || path.starts_with(".github/actions/")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::control::ControlStatus;
    use crate::evidence::{ChangeRequestId, ChangedAsset};

    fn workflow_asset(path: &str, diff: &str) -> ChangedAsset {
        ChangedAsset {
            path: path.to_string(),
            diff_available: true,
            additions: 1,
            deletions: 0,
            status: "modified".to_string(),
            diff: Some(diff.to_string()),
        }
    }

    fn make_change(assets: Vec<ChangedAsset>) -> GovernedChange {
        GovernedChange {
            id: ChangeRequestId::new("github_pr", "owner/repo#1"),
            title: "test".to_string(),
            summary: None,
            submitted_by: None,
            changed_assets: EvidenceState::complete(assets),
            approval_decisions: EvidenceState::not_applicable(),
            source_revisions: EvidenceState::not_applicable(),
            work_item_refs: EvidenceState::not_applicable(),
        }
    }

    fn bundle(changes: Vec<GovernedChange>) -> EvidenceBundle {
        EvidenceBundle {
            change_requests: changes,
            ..Default::default()
        }
    }

    #[test]
    fn satisfied_when_read_only() {
        let diff = "+permissions:\n+  contents: read";
        let cr = make_change(vec![workflow_asset(".github/workflows/ci.yml", diff)]);
        let findings = WorkflowPermissionsControl.evaluate(&bundle(vec![cr]));
        assert_eq!(findings[0].status, ControlStatus::Satisfied);
    }

    #[test]
    fn violated_when_write_all() {
        let diff = "+permissions: write-all";
        let cr = make_change(vec![workflow_asset(".github/workflows/ci.yml", diff)]);
        let findings = WorkflowPermissionsControl.evaluate(&bundle(vec![cr]));
        assert_eq!(findings[0].status, ControlStatus::Violated);
        assert!(findings[0].rationale.contains("write-all"));
    }

    #[test]
    fn violated_when_contents_write() {
        let diff = "+  contents: write";
        let cr = make_change(vec![workflow_asset(".github/workflows/release.yml", diff)]);
        let findings = WorkflowPermissionsControl.evaluate(&bundle(vec![cr]));
        assert_eq!(findings[0].status, ControlStatus::Violated);
    }

    #[test]
    fn ignores_removed_permissions() {
        let diff = "-permissions: write-all\n+permissions:\n+  contents: read";
        let cr = make_change(vec![workflow_asset(".github/workflows/ci.yml", diff)]);
        let findings = WorkflowPermissionsControl.evaluate(&bundle(vec![cr]));
        assert_eq!(findings[0].status, ControlStatus::Satisfied);
    }

    #[test]
    fn not_applicable_when_no_workflows() {
        let findings = WorkflowPermissionsControl.evaluate(&EvidenceBundle::default());
        assert_eq!(findings[0].status, ControlStatus::NotApplicable);
    }

    #[test]
    fn not_applicable_when_non_workflow_files() {
        let cr = make_change(vec![ChangedAsset {
            path: "src/main.rs".to_string(),
            diff_available: true,
            additions: 1,
            deletions: 0,
            status: "modified".to_string(),
            diff: Some("+fn main() {}".to_string()),
        }]);
        let findings = WorkflowPermissionsControl.evaluate(&bundle(vec![cr]));
        assert_eq!(findings[0].status, ControlStatus::NotApplicable);
    }
}
