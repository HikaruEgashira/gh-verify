use crate::control::{Control, ControlFinding, ControlId};
use crate::evidence::{EvidenceBundle, EvidenceState, GovernedChange};

/// Verifies that GitHub Actions dependencies use pinned SHA references
/// rather than mutable tags.
///
/// Maps to:
/// - OpenSSF Scorecard: Pinned-Dependencies check
/// - NIST SSDF PS.1: Protect code from tampering (tag-based refs are mutable)
///
/// Only analyzes `.github/workflows/` files. Checks `uses:` directives
/// for SHA pinning (`@<40-hex-chars>`) vs tag pinning (`@v1`, `@main`).
pub struct DependencyPinningControl;

impl Control for DependencyPinningControl {
    fn id(&self) -> ControlId {
        ControlId::DependencyPinning
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

    // Filter to workflow files with diffs
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

    let mut unpinned: Vec<String> = Vec::new();
    let mut total_uses = 0usize;

    for asset in &workflow_assets {
        if let Some(diff) = &asset.diff {
            for line in diff.lines() {
                // Only check added lines
                if !line.starts_with('+') || line.starts_with("+++") {
                    continue;
                }
                if let Some(reference) = extract_uses_ref(line) {
                    total_uses += 1;
                    if !is_sha_pinned(reference) {
                        unpinned.push(format!("{}:{}", asset.path, reference));
                    }
                }
            }
        }
    }

    if total_uses == 0 {
        return ControlFinding::not_applicable(
            id,
            format!("{cr_subject}: no `uses:` directives found in added lines"),
        );
    }

    if unpinned.is_empty() {
        ControlFinding::satisfied(
            id,
            format!("{cr_subject}: all {total_uses} action reference(s) are SHA-pinned"),
            vec![cr_subject],
        )
    } else {
        ControlFinding::violated(
            id,
            format!(
                "{cr_subject}: {} of {} action reference(s) not SHA-pinned: {}",
                unpinned.len(),
                total_uses,
                unpinned.join(", ")
            ),
            unpinned,
        )
    }
}

fn is_workflow_path(path: &str) -> bool {
    path.starts_with(".github/workflows/") || path.starts_with(".github/actions/")
}

/// Extracts the action reference from a `uses:` line.
/// e.g. `uses: actions/checkout@v4` → `actions/checkout@v4`
fn extract_uses_ref(line: &str) -> Option<&str> {
    let trimmed = line.trim_start_matches('+').trim();
    if let Some(rest) = trimmed.strip_prefix("uses:") {
        let reference = rest.trim();
        // Skip Docker and local path references
        if reference.starts_with("docker://") || reference.starts_with("./") {
            return None;
        }
        if reference.contains('@') {
            return Some(reference);
        }
    }
    None
}

/// Returns true if the reference uses a full SHA (40 hex chars after @).
fn is_sha_pinned(reference: &str) -> bool {
    if let Some(at_pos) = reference.rfind('@') {
        let version = &reference[at_pos + 1..];
        // Strip trailing comment
        let version = version.split_whitespace().next().unwrap_or(version);
        version.len() >= 40 && version.chars().take(40).all(|c| c.is_ascii_hexdigit())
    } else {
        false
    }
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
    fn satisfied_when_sha_pinned() {
        let diff = "+      uses: actions/checkout@b4ffde65f46336ab88eb53be808477a3936bae11";
        let cr = make_change(vec![workflow_asset(".github/workflows/ci.yml", diff)]);
        let findings = DependencyPinningControl.evaluate(&bundle(vec![cr]));
        assert_eq!(findings[0].status, ControlStatus::Satisfied);
    }

    #[test]
    fn violated_when_tag_ref() {
        let diff = "+      uses: actions/checkout@v4";
        let cr = make_change(vec![workflow_asset(".github/workflows/ci.yml", diff)]);
        let findings = DependencyPinningControl.evaluate(&bundle(vec![cr]));
        assert_eq!(findings[0].status, ControlStatus::Violated);
        assert!(findings[0].rationale.contains("not SHA-pinned"));
    }

    #[test]
    fn ignores_removed_lines() {
        let diff = "-      uses: actions/checkout@v3\n+      uses: actions/checkout@b4ffde65f46336ab88eb53be808477a3936bae11";
        let cr = make_change(vec![workflow_asset(".github/workflows/ci.yml", diff)]);
        let findings = DependencyPinningControl.evaluate(&bundle(vec![cr]));
        assert_eq!(findings[0].status, ControlStatus::Satisfied);
    }

    #[test]
    fn not_applicable_when_no_workflows() {
        let cr = make_change(vec![ChangedAsset {
            path: "src/main.rs".to_string(),
            diff_available: true,
            additions: 1,
            deletions: 0,
            status: "modified".to_string(),
            diff: Some("+fn main() {}".to_string()),
        }]);
        let findings = DependencyPinningControl.evaluate(&bundle(vec![cr]));
        assert_eq!(findings[0].status, ControlStatus::NotApplicable);
    }

    #[test]
    fn not_applicable_when_no_uses() {
        let diff = "+    runs-on: ubuntu-latest";
        let cr = make_change(vec![workflow_asset(".github/workflows/ci.yml", diff)]);
        let findings = DependencyPinningControl.evaluate(&bundle(vec![cr]));
        assert_eq!(findings[0].status, ControlStatus::NotApplicable);
    }

    #[test]
    fn ignores_docker_refs() {
        let diff = "+      uses: docker://alpine:3.18";
        let cr = make_change(vec![workflow_asset(".github/workflows/ci.yml", diff)]);
        let findings = DependencyPinningControl.evaluate(&bundle(vec![cr]));
        assert_eq!(findings[0].status, ControlStatus::NotApplicable);
    }

    #[test]
    fn ignores_local_refs() {
        let diff = "+      uses: ./.github/actions/my-action";
        let cr = make_change(vec![workflow_asset(".github/workflows/ci.yml", diff)]);
        let findings = DependencyPinningControl.evaluate(&bundle(vec![cr]));
        assert_eq!(findings[0].status, ControlStatus::NotApplicable);
    }

    #[test]
    fn mixed_pinned_and_unpinned() {
        let diff = "+      uses: actions/checkout@b4ffde65f46336ab88eb53be808477a3936bae11\n+      uses: actions/setup-node@v4";
        let cr = make_change(vec![workflow_asset(".github/workflows/ci.yml", diff)]);
        let findings = DependencyPinningControl.evaluate(&bundle(vec![cr]));
        assert_eq!(findings[0].status, ControlStatus::Violated);
        assert!(findings[0].rationale.contains("1 of 2"));
    }
}
