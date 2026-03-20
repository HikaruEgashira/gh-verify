use super::*;

// --- EvidenceState::value() mutations ---

#[test]
fn complete_state_returns_value() {
    let state = EvidenceState::complete(42);
    assert_eq!(state.value(), Some(&42));
}

#[test]
fn partial_state_returns_value() {
    let state = EvidenceState::partial(
        42,
        vec![EvidenceGap::DiffUnavailable {
            subject: "x".into(),
        }],
    );
    assert_eq!(state.value(), Some(&42));
}

#[test]
fn missing_state_returns_none() {
    let state: EvidenceState<i32> = EvidenceState::missing(vec![]);
    assert_eq!(state.value(), None);
}

#[test]
fn not_applicable_state_returns_none() {
    let state: EvidenceState<i32> = EvidenceState::not_applicable();
    assert_eq!(state.value(), None);
}

// --- EvidenceState::gaps() mutations ---

#[test]
fn complete_state_has_no_gaps() {
    let state = EvidenceState::complete(42);
    assert!(state.gaps().is_empty());
}

#[test]
fn missing_state_returns_gaps() {
    let gap = EvidenceGap::DiffUnavailable {
        subject: "x".into(),
    };
    let state: EvidenceState<i32> = EvidenceState::missing(vec![gap]);
    assert_eq!(state.gaps().len(), 1);
}

#[test]
fn not_applicable_state_has_no_gaps() {
    let state: EvidenceState<i32> = EvidenceState::not_applicable();
    assert!(state.gaps().is_empty());
}

// --- EvidenceState::has_gaps() mutations ---

#[test]
fn partial_with_empty_gaps_no_gaps() {
    let state = EvidenceState::partial(1, vec![]);
    assert!(!state.has_gaps());
}
