use forge_core::project_registry::SafeWorkspaceSnapshot;
use forge_core::recovery::{
    InterruptedAction, InterruptedEffectState, RecordedProcess, RecoveredProcessState,
    RecoveryActionKind, WorkspaceRecoveryError, WorkspaceRecoveryRecord,
};
use forge_core::state::StateRecord;
use forge_protocol::hashes::{HashDomain, hash_canonical_bytes};
use forge_protocol::identities::{IDENTITY_BYTES, ProcessId, ProjectId, SessionId};

fn project(byte: u8) -> ProjectId {
    ProjectId::from_bytes([byte; IDENTITY_BYTES])
}

fn session(byte: u8) -> SessionId {
    SessionId::from_bytes([byte; IDENTITY_BYTES])
}

fn process(byte: u8) -> ProcessId {
    ProcessId::from_bytes([byte; IDENTITY_BYTES])
}

fn action(label: &[u8], kind: RecoveryActionKind) -> InterruptedAction {
    InterruptedAction::new(
        hash_canonical_bytes(HashDomain::ToolRequest, label),
        kind,
        InterruptedEffectState::CommitUnknown,
    )
}

#[test]
fn recovery_record_round_trips_canonical_safe_and_crash_evidence() {
    let record = WorkspaceRecoveryRecord::new(
        7,
        project(1),
        Some(session(2)),
        SafeWorkspaceSnapshot::new(1, b"buffers=2;active=src/lib.rs".to_vec()).unwrap(),
        vec![
            action(b"second", RecoveryActionKind::GitMutation),
            action(b"first", RecoveryActionKind::FileWrite),
        ],
        vec![
            RecordedProcess::new(
                "nyx",
                Some(process(9)),
                RecoveredProcessState::RequiresRevalidation,
            )
            .unwrap(),
            RecordedProcess::new("forge-core", None, RecoveredProcessState::ConfirmedStopped)
                .unwrap(),
        ],
    )
    .unwrap();

    let reopened =
        WorkspaceRecoveryRecord::from_state_record(&record.to_state_record().unwrap()).unwrap();

    assert_eq!(reopened, record);
    assert_eq!(reopened.generation(), 7);
    assert_eq!(reopened.project_id(), project(1));
    assert_eq!(reopened.session_id(), Some(session(2)));
    assert_eq!(
        reopened.safe_snapshot().payload(),
        b"buffers=2;active=src/lib.rs"
    );
    assert!(
        reopened
            .interrupted_actions()
            .iter()
            .all(|entry| !entry.replay_allowed())
    );
    assert!(
        reopened
            .recorded_processes()
            .iter()
            .all(|entry| !entry.claims_alive())
    );
    assert_eq!(
        reopened.recorded_processes()[0].service_name(),
        "forge-core"
    );
    assert_eq!(reopened.recorded_processes()[1].service_name(), "nyx");
    assert_eq!(record.identity().unwrap(), reopened.identity().unwrap());
}

#[test]
fn malformed_snapshot_identity_is_rejected_inside_valid_state_envelope() {
    let record = WorkspaceRecoveryRecord::new(
        1,
        project(3),
        None,
        SafeWorkspaceSnapshot::new(1, b"safe".to_vec()).unwrap(),
        Vec::new(),
        Vec::new(),
    )
    .unwrap();
    let state = record.to_state_record().unwrap();
    let mut payload = state.payload().to_vec();
    let expected = record.safe_snapshot().identity();
    let position = payload
        .windows(expected.as_bytes().len())
        .position(|window| window == expected.as_bytes())
        .expect("snapshot identity bytes are present");
    payload[position] ^= 0x80;
    let malformed = StateRecord::new(state.record_type(), payload).unwrap();

    assert!(matches!(
        WorkspaceRecoveryRecord::from_state_record(&malformed),
        Err(WorkspaceRecoveryError::SnapshotIdentityMismatch { .. })
    ));
}

#[test]
fn invalid_process_claims_and_duplicate_actions_fail_closed() {
    assert!(matches!(
        RecordedProcess::new("nyx", None, RecoveredProcessState::RequiresRevalidation),
        Err(WorkspaceRecoveryError::MissingPriorProcessId)
    ));
    assert!(matches!(
        RecordedProcess::new(
            "nyx",
            Some(process(4)),
            RecoveredProcessState::ConfirmedStopped
        ),
        Err(WorkspaceRecoveryError::StoppedProcessRetainsIdentity)
    ));

    let duplicate = action(b"same", RecoveryActionKind::Command);
    assert!(matches!(
        WorkspaceRecoveryRecord::new(
            1,
            project(4),
            None,
            SafeWorkspaceSnapshot::new(1, b"safe".to_vec()).unwrap(),
            vec![duplicate.clone(), duplicate],
            Vec::new(),
        ),
        Err(WorkspaceRecoveryError::DuplicateInterruptedAction(_))
    ));
}
