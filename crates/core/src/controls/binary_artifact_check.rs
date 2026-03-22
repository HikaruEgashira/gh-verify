use crate::control::{Control, ControlFinding, ControlId};
use crate::evidence::{EvidenceBundle, EvidenceState, GovernedChange};

/// File extensions considered binary artifacts.
const BINARY_EXTENSIONS: &[&str] = &[
    // Compiled binaries
    ".exe", ".dll", ".so", ".dylib", ".a", ".lib", ".o", ".obj",
    // JVM
    ".jar", ".war", ".ear", ".class",
    // .NET
    ".nupkg",
    // Python
    ".pyc", ".pyo", ".whl",
    // WebAssembly
    ".wasm",
    // Archives
    ".zip", ".tar", ".gz", ".tgz", ".bz2", ".xz", ".7z", ".rar",
    // Images/media (not source)
    ".png", ".jpg", ".jpeg", ".gif", ".ico", ".bmp", ".svg",
    // Database
    ".db", ".sqlite", ".sqlite3",
];

/// Detects addition of binary artifacts in change requests.
///
/// Maps to:
/// - OpenSSF Scorecard: Binary-Artifacts check
/// - NIST SSDF PS.1: Protect code from tampering (binary blobs bypass review)
pub struct BinaryArtifactCheckControl;

impl Control for BinaryArtifactCheckControl {
    fn id(&self) -> ControlId {
        ControlId::BinaryArtifactCheck
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

    // Only flag added or modified binary files
    let binary_files: Vec<&str> = assets
        .iter()
        .filter(|a| a.status != "removed")
        .filter(|a| is_binary_path(&a.path))
        .map(|a| a.path.as_str())
        .collect();

    if binary_files.is_empty() {
        ControlFinding::satisfied(
            id,
            format!("{cr_subject}: no binary artifacts added or modified"),
            vec![cr_subject],
        )
    } else {
        ControlFinding::violated(
            id,
            format!(
                "{cr_subject}: {} binary artifact(s) added/modified: {}",
                binary_files.len(),
                binary_files.join(", ")
            ),
            binary_files.into_iter().map(String::from).collect(),
        )
    }
}

fn is_binary_path(path: &str) -> bool {
    let lower = path.to_lowercase();
    BINARY_EXTENSIONS.iter().any(|ext| lower.ends_with(ext))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::control::ControlStatus;
    use crate::evidence::{ChangeRequestId, ChangedAsset, EvidenceGap};

    fn asset(path: &str, status: &str) -> ChangedAsset {
        ChangedAsset {
            path: path.to_string(),
            diff_available: true,
            additions: 1,
            deletions: 0,
            status: status.to_string(),
            diff: None,
        }
    }

    fn make_change(assets: EvidenceState<Vec<ChangedAsset>>) -> GovernedChange {
        GovernedChange {
            id: ChangeRequestId::new("github_pr", "owner/repo#1"),
            title: "test".to_string(),
            summary: None,
            submitted_by: None,
            changed_assets: assets,
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
    fn satisfied_when_no_binaries() {
        let cr = make_change(EvidenceState::complete(vec![
            asset("src/main.rs", "modified"),
            asset("README.md", "modified"),
        ]));
        let findings = BinaryArtifactCheckControl.evaluate(&bundle(vec![cr]));
        assert_eq!(findings[0].status, ControlStatus::Satisfied);
    }

    #[test]
    fn violated_when_binary_added() {
        let cr = make_change(EvidenceState::complete(vec![
            asset("src/main.rs", "modified"),
            asset("bin/tool.exe", "added"),
        ]));
        let findings = BinaryArtifactCheckControl.evaluate(&bundle(vec![cr]));
        assert_eq!(findings[0].status, ControlStatus::Violated);
        assert!(findings[0].rationale.contains("tool.exe"));
    }

    #[test]
    fn satisfied_when_binary_removed() {
        let cr = make_change(EvidenceState::complete(vec![
            asset("bin/old.dll", "removed"),
        ]));
        let findings = BinaryArtifactCheckControl.evaluate(&bundle(vec![cr]));
        assert_eq!(findings[0].status, ControlStatus::Satisfied);
    }

    #[test]
    fn violated_when_jar_added() {
        let cr = make_change(EvidenceState::complete(vec![
            asset("libs/dependency.jar", "added"),
        ]));
        let findings = BinaryArtifactCheckControl.evaluate(&bundle(vec![cr]));
        assert_eq!(findings[0].status, ControlStatus::Violated);
    }

    #[test]
    fn not_applicable_when_no_changes() {
        let findings = BinaryArtifactCheckControl.evaluate(&EvidenceBundle::default());
        assert_eq!(findings[0].status, ControlStatus::NotApplicable);
    }

    #[test]
    fn indeterminate_when_assets_missing() {
        let cr = make_change(EvidenceState::missing(vec![
            EvidenceGap::CollectionFailed {
                source: "github".to_string(),
                subject: "files".to_string(),
                detail: "error".to_string(),
            },
        ]));
        let findings = BinaryArtifactCheckControl.evaluate(&bundle(vec![cr]));
        assert_eq!(findings[0].status, ControlStatus::Indeterminate);
    }

    #[test]
    fn case_insensitive() {
        let cr = make_change(EvidenceState::complete(vec![
            asset("bin/Tool.EXE", "added"),
        ]));
        let findings = BinaryArtifactCheckControl.evaluate(&bundle(vec![cr]));
        assert_eq!(findings[0].status, ControlStatus::Violated);
    }
}
