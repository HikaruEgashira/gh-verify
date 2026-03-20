use super::*;
use crate::evidence::EvidenceGap;

// --- ControlFinding constructors ---

#[test]
fn finding_satisfied_has_correct_status_and_no_gaps() {
    let f = ControlFinding::satisfied(
        ControlId::ReviewIndependence,
        "ok",
        vec!["s1".into()],
    );
    assert_eq!(f.status, ControlStatus::Satisfied);
    assert_eq!(f.control_id, ControlId::ReviewIndependence);
    assert_eq!(f.subjects, vec!["s1"]);
    assert!(f.evidence_gaps.is_empty());
}

#[test]
fn finding_violated_has_correct_status_and_no_gaps() {
    let f = ControlFinding::violated(
        ControlId::SourceAuthenticity,
        "bad",
        vec!["s1".into()],
    );
    assert_eq!(f.status, ControlStatus::Violated);
    assert!(f.evidence_gaps.is_empty());
}

#[test]
fn finding_indeterminate_preserves_gaps() {
    let gaps = vec![EvidenceGap::DiffUnavailable {
        subject: "x".into(),
    }];
    let f = ControlFinding::indeterminate(
        ControlId::ReviewIndependence,
        "partial",
        vec!["s1".into()],
        gaps,
    );
    assert_eq!(f.status, ControlStatus::Indeterminate);
    assert_eq!(f.evidence_gaps.len(), 1);
}

#[test]
fn finding_not_applicable_has_empty_subjects_and_gaps() {
    let f = ControlFinding::not_applicable(
        ControlId::SourceAuthenticity,
        "n/a",
    );
    assert_eq!(f.status, ControlStatus::NotApplicable);
    assert!(f.subjects.is_empty());
    assert!(f.evidence_gaps.is_empty());
}

// --- evaluate_all ---

#[test]
fn evaluate_all_collects_from_all_controls() {
    // Kills: only evaluating first control or returning early
    use crate::controls::slsa_foundation_controls;
    let evidence = EvidenceBundle::default();
    let controls = slsa_foundation_controls();
    let findings = evaluate_all(&controls, &evidence);
    // Both controls should produce at least one finding each
    assert!(findings.iter().any(|f| f.control_id == ControlId::ReviewIndependence));
    assert!(findings.iter().any(|f| f.control_id == ControlId::SourceAuthenticity));
}

#[test]
fn evaluate_all_empty_controls_returns_empty() {
    let evidence = EvidenceBundle::default();
    let controls: Vec<Box<dyn Control>> = vec![];
    let findings = evaluate_all(&controls, &evidence);
    assert!(findings.is_empty());
}
