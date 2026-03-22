use super::*;
use crate::evidence::EvidenceGap;

// --- ControlFinding gap preservation ---

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

// --- evaluate_all ---

#[test]
fn evaluate_all_collects_from_all_controls() {
    use crate::controls::slsa_controls;
    use crate::slsa::SlsaLevel;
    let evidence = EvidenceBundle::default();
    let controls = slsa_controls(SlsaLevel::L1, SlsaLevel::L1);
    let findings = evaluate_all(&controls, &evidence);
    assert!(
        findings
            .iter()
            .any(|f| f.control_id == ControlId::ReviewIndependence)
    );
    assert!(
        findings
            .iter()
            .any(|f| f.control_id == ControlId::SourceAuthenticity)
    );
}

#[test]
fn evaluate_all_empty_controls_returns_empty() {
    let evidence = EvidenceBundle::default();
    let controls: Vec<Box<dyn Control>> = vec![];
    let findings = evaluate_all(&controls, &evidence);
    assert!(findings.is_empty());
}
