use forge_core::state::StateRecord;
use forge_core::verification::{
    VERIFICATION_LEDGER_STATE_RECORD_TYPE, VerificationLedger, VerificationLedgerError,
    VerificationOutcome, VerificationOutputReference, VerificationRecord, VerificationSourceState,
};
use forge_protocol::hashes::ContentHash;
use forge_protocol::identities::{CommandId, IDENTITY_BYTES, ProcessId, ProjectId, RepositoryId};

fn id<T>(byte: u8, constructor: fn([u8; IDENTITY_BYTES]) -> T) -> T {
    constructor([byte; IDENTITY_BYTES])
}

fn source(revision: &[u8], dirty: u8) -> VerificationSourceState {
    VerificationSourceState::new(
        id(1, ProjectId::from_bytes),
        id(2, RepositoryId::from_bytes),
        revision.to_vec(),
        ContentHash::from_bytes([dirty; 32]),
    )
    .unwrap()
}

fn record(process_byte: u8, argument: &str, outcome: VerificationOutcome) -> VerificationRecord {
    VerificationRecord::new(
        id(3, CommandId::from_bytes),
        ContentHash::from_bytes([4; 32]),
        id(process_byte, ProcessId::from_bytes),
        "/usr/bin/cargo",
        vec!["test".to_owned(), argument.to_owned()],
        source(b"0123456789abcdef", 5),
        source(b"0123456789abcdef", 6),
        outcome,
        VerificationOutputReference::from_output(b"stdout\n", b"stderr\n"),
    )
    .unwrap()
}

#[test]
fn record_identity_binds_exact_argv_source_outcome_and_output() {
    let first = record(
        7,
        "--locked",
        VerificationOutcome::Passed { exit_code: Some(0) },
    );
    let same = record(
        7,
        "--locked",
        VerificationOutcome::Passed { exit_code: Some(0) },
    );
    let different_argument = record(
        7,
        "--offline",
        VerificationOutcome::Passed { exit_code: Some(0) },
    );
    let failed = record(
        7,
        "--locked",
        VerificationOutcome::Failed { exit_code: Some(1) },
    );

    assert_eq!(first.identity(), same.identity());
    assert_ne!(first.identity(), different_argument.identity());
    assert_ne!(first.identity(), failed.identity());
    assert_eq!(first.program(), "/usr/bin/cargo");
    assert_eq!(
        first
            .arguments()
            .iter()
            .map(String::as_str)
            .collect::<Vec<_>>(),
        ["test", "--locked"]
    );
    assert_eq!(first.outcome().exit_code(), Some(0));
}

#[test]
fn record_round_trip_preserves_exact_canonical_bytes() {
    let original = record(8, "--workspace", VerificationOutcome::TimedOut);
    let bytes = original.canonical_bytes();
    let decoded = VerificationRecord::decode(&bytes).unwrap();

    assert_eq!(decoded, original);
    assert_eq!(decoded.canonical_bytes(), bytes);
    assert_eq!(decoded.identity(), original.identity());
}

#[test]
fn output_reference_preserves_channel_boundaries_and_lengths() {
    let reference = VerificationOutputReference::from_output(b"ab", b"c");
    let swapped = VerificationOutputReference::from_output(b"a", b"bc");

    assert!(reference.matches(b"ab", b"c"));
    assert!(!reference.matches(b"abc", b""));
    assert_ne!(reference.identity(), swapped.identity());
    assert_eq!(reference.stdout_bytes(), 2);
    assert_eq!(reference.stderr_bytes(), 1);
}

#[test]
fn only_a_pass_for_the_exact_end_state_satisfies_current_validation() {
    let pass = record(
        9,
        "--all",
        VerificationOutcome::Passed { exit_code: Some(0) },
    );
    let fail = record(
        10,
        "--all",
        VerificationOutcome::Failed { exit_code: Some(2) },
    );
    let current = source(b"0123456789abcdef", 6);
    let changed = source(b"0123456789abcdef", 99);

    assert!(pass.satisfies(&current));
    assert!(!pass.satisfies(&changed));
    assert!(!fail.satisfies(&current));
}

#[test]
fn ledger_is_append_only_deterministic_and_state_round_trips() {
    let first = record(
        11,
        "--first",
        VerificationOutcome::Passed { exit_code: Some(0) },
    );
    let second = record(12, "--second", VerificationOutcome::Cancelled);

    let mut left = VerificationLedger::new();
    left.record(second.clone()).unwrap();
    left.record(first.clone()).unwrap();
    left.record(first.clone()).unwrap();

    let mut right = VerificationLedger::new();
    right.record(first).unwrap();
    right.record(second).unwrap();

    let left_state = left.state_record().unwrap();
    let right_state = right.state_record().unwrap();
    assert_eq!(left_state.encode(), right_state.encode());

    let restored = VerificationLedger::from_state_record(&left_state).unwrap();
    assert_eq!(restored, left);
    assert_eq!(restored.len(), 2);
}

#[test]
fn ledger_rejects_tampered_stored_record_identity() {
    let mut ledger = VerificationLedger::new();
    ledger
        .record(record(
            13,
            "--tamper",
            VerificationOutcome::Passed { exit_code: Some(0) },
        ))
        .unwrap();
    let state = ledger.state_record().unwrap();
    let mut payload = state.payload().to_vec();
    payload[13] ^= 0xff;
    let tampered = StateRecord::new(VERIFICATION_LEDGER_STATE_RECORD_TYPE, payload).unwrap();

    assert!(matches!(
        VerificationLedger::from_state_record(&tampered),
        Err(VerificationLedgerError::StoredIdentityMismatch { .. })
    ));
}
