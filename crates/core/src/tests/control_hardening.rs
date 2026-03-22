use super::*;
use crate::evidence::EvidenceGap;

// --- ControlStatus::as_str / Display ---

#[test]
fn control_status_as_str_returns_expected_values() {
    assert_eq!(ControlStatus::Satisfied.as_str(), "satisfied");
    assert_eq!(ControlStatus::Violated.as_str(), "violated");
    assert_eq!(ControlStatus::Indeterminate.as_str(), "indeterminate");
    assert_eq!(ControlStatus::NotApplicable.as_str(), "not_applicable");
}

#[test]
fn control_status_display_matches_as_str() {
    let variants = [
        ControlStatus::Satisfied,
        ControlStatus::Violated,
        ControlStatus::Indeterminate,
        ControlStatus::NotApplicable,
    ];
    for status in &variants {
        let displayed = format!("{status}");
        assert_eq!(
            displayed,
            status.as_str(),
            "Display output must equal as_str for {status:?}"
        );
        assert!(
            !displayed.is_empty(),
            "Display must not be empty for {status:?}"
        );
    }
}

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

#[test]
fn evaluate_all_compliance_controls_against_empty_evidence() {
    use crate::controls::compliance_controls;
    let evidence = EvidenceBundle::default();
    let controls = compliance_controls();
    let findings = evaluate_all(&controls, &evidence);
    // All 10 compliance controls should return NotApplicable for empty evidence
    assert_eq!(
        findings.len(),
        10,
        "each compliance control should produce exactly one finding for empty evidence"
    );
    for f in &findings {
        assert_eq!(
            f.status,
            ControlStatus::NotApplicable,
            "{:?} should be NotApplicable for empty evidence, got {:?}",
            f.control_id,
            f.status
        );
    }
}

#[test]
fn evaluate_all_compliance_controls_findings_have_valid_ids() {
    use crate::controls::compliance_controls;
    let controls = compliance_controls();
    let evidence = EvidenceBundle::default();
    let findings = evaluate_all(&controls, &evidence);
    for f in &findings {
        // Every finding's control_id should round-trip through as_str/parse
        let s = f.control_id.as_str();
        let parsed: ControlId = s.parse().expect("control_id should round-trip");
        assert_eq!(f.control_id, parsed);
    }
}
