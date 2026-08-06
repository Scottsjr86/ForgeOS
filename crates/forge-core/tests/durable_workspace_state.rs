use forge_core::workspace_recovery::{
    DurableWorkspaceState, RecoveredBuffer, RecoveredDiskBaseline, RecoveredService,
    RecoveredServiceState, RecoveredTerminal, RecoveredTerminalState, WorkspacePayloadError,
};
use forge_protocol::hashes::{HashDomain, hash_canonical_bytes};
use forge_protocol::identities::{
    IDENTITY_BYTES, ProcessId, ProjectId, RepositoryId, SessionId, TerminalId,
};

fn baseline(bytes: &[u8]) -> RecoveredDiskBaseline {
    RecoveredDiskBaseline::Existing {
        content_hash: hash_canonical_bytes(HashDomain::File, bytes),
        length: bytes.len() as u64,
    }
}

#[test]
fn durable_workspace_round_trips_sorted_non_live_state() {
    let state = DurableWorkspaceState::new(
        ProjectId::from_bytes([1; IDENTITY_BYTES]),
        RepositoryId::from_bytes([2; IDENTITY_BYTES]),
        SessionId::from_bytes([3; IDENTITY_BYTES]),
        vec![
            RecoveredBuffer::new(
                [9; IDENTITY_BYTES],
                b"src/z.rs".to_vec(),
                4,
                1,
                1,
                baseline(b"disk"),
                None,
                b"local".to_vec(),
            )
            .unwrap(),
            RecoveredBuffer::new(
                [4; IDENTITY_BYTES],
                b"src/a.rs".to_vec(),
                2,
                0,
                0,
                RecoveredDiskBaseline::Missing,
                Some(baseline(b"external")),
                b"new".to_vec(),
            )
            .unwrap(),
        ],
        vec![
            RecoveredTerminal::new(
                TerminalId::from_bytes([8; IDENTITY_BYTES]),
                b"src".to_vec(),
                RecoveredTerminalState::RequiresRestart,
            )
            .unwrap(),
            RecoveredTerminal::new(
                TerminalId::from_bytes([5; IDENTITY_BYTES]),
                Vec::new(),
                RecoveredTerminalState::Exited {
                    code: 3,
                    terminated_by_operator: false,
                },
            )
            .unwrap(),
        ],
        vec![
            RecoveredService::new("zeta", None, RecoveredServiceState::Failed).unwrap(),
            RecoveredService::new(
                "nyx-server",
                Some(ProcessId::from_bytes([7; IDENTITY_BYTES])),
                RecoveredServiceState::RequiresRevalidation,
            )
            .unwrap(),
        ],
    )
    .unwrap();

    let restored = DurableWorkspaceState::from_safe_snapshot(&state.to_safe_snapshot().unwrap())
        .expect("round trip");
    assert_eq!(restored, state);
    assert_eq!(restored.buffers()[0].buffer_id(), &[4; IDENTITY_BYTES]);
    assert_eq!(
        restored.terminals()[0].terminal_id(),
        TerminalId::from_bytes([5; 16])
    );
    assert_eq!(restored.services()[0].name(), "nyx-server");
    assert!(
        restored
            .terminals()
            .iter()
            .all(|entry| !entry.claims_alive())
    );
    assert!(
        restored
            .services()
            .iter()
            .all(|entry| !entry.claims_alive())
    );
}

#[test]
fn malformed_recovery_state_fails_closed() {
    let buffer = RecoveredBuffer::new(
        [1; IDENTITY_BYTES],
        b"src/lib.rs".to_vec(),
        1,
        0,
        0,
        RecoveredDiskBaseline::Missing,
        None,
        b"dirty".to_vec(),
    )
    .unwrap();
    assert!(matches!(
        DurableWorkspaceState::new(
            ProjectId::from_bytes([1; 16]),
            RepositoryId::from_bytes([2; 16]),
            SessionId::from_bytes([3; 16]),
            vec![buffer.clone(), buffer],
            Vec::new(),
            Vec::new(),
        ),
        Err(WorkspacePayloadError::DuplicateBuffer(_))
    ));
    assert!(matches!(
        RecoveredService::new(
            "nyx-server",
            None,
            RecoveredServiceState::RequiresRevalidation,
        ),
        Err(WorkspacePayloadError::MissingPriorProcessId(_))
    ));
    assert!(matches!(
        RecoveredBuffer::new(
            [2; 16],
            b"../escape.rs".to_vec(),
            1,
            0,
            0,
            RecoveredDiskBaseline::Missing,
            None,
            b"tiny".to_vec(),
        ),
        Err(WorkspacePayloadError::InvalidRelativePath)
    ));
    assert!(matches!(
        RecoveredBuffer::new(
            [2; 16],
            b"src/lib.rs".to_vec(),
            1,
            99,
            99,
            RecoveredDiskBaseline::Missing,
            None,
            b"tiny".to_vec(),
        ),
        Err(WorkspacePayloadError::CursorOutsideBuffer { .. })
    ));
}
