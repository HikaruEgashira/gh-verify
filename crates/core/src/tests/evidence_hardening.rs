use super::*;

// --- EvidenceState::value() variant coverage ---

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

// --- EvidenceState::gaps() mutation coverage ---

#[test]
fn missing_state_returns_gaps() {
    let gap = EvidenceGap::DiffUnavailable {
        subject: "x".into(),
    };
    let state: EvidenceState<i32> = EvidenceState::missing(vec![gap]);
    assert_eq!(state.gaps().len(), 1);
}
