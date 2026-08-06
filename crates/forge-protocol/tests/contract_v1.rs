use forge_protocol::envelopes::{
    CURRENT_PROTOCOL_VERSION, EnvelopeKind, ErrorEnvelope, RequestEnvelope, ResultEnvelope,
};
use forge_protocol::errors::{EnvelopeViolation, ProtocolError, ProtocolErrorCode};
use forge_protocol::events::EventRecord;
use forge_protocol::identities::{
    CommandId, EventId, IDENTITY_BYTES, IdentityKind, PatchId, ProcessId, ProjectId, RepositoryId,
    ResultId, SessionId, TaskId, TerminalId, ensure_unique,
};

#[test]
fn all_required_identity_types_round_trip_canonical_text() {
    assert_identity(ProjectId::from_bytes([1; IDENTITY_BYTES]));
    assert_identity(RepositoryId::from_bytes([2; IDENTITY_BYTES]));
    assert_identity(ProcessId::from_bytes([3; IDENTITY_BYTES]));
    assert_identity(TerminalId::from_bytes([4; IDENTITY_BYTES]));
    assert_identity(CommandId::from_bytes([5; IDENTITY_BYTES]));
    assert_identity(SessionId::from_bytes([6; IDENTITY_BYTES]));
    assert_identity(TaskId::from_bytes([7; IDENTITY_BYTES]));
    assert_identity(PatchId::from_bytes([8; IDENTITY_BYTES]));
    assert_identity(ResultId::from_bytes([9; IDENTITY_BYTES]));
    assert_identity(EventId::from_bytes([10; IDENTITY_BYTES]));
}

#[test]
fn request_result_and_error_envelopes_round_trip() {
    let task = TaskId::from_bytes([0x11; IDENTITY_BYTES]);
    let request = RequestEnvelope::new(
        task,
        CommandId::from_bytes([0x22; IDENTITY_BYTES]),
        b"build".to_vec(),
    )
    .expect("request should fit V1 payload bounds");
    assert_eq!(request.version(), CURRENT_PROTOCOL_VERSION);
    assert_eq!(RequestEnvelope::decode(&request.encode()), Ok(request));

    let result = ResultEnvelope::new(
        task,
        ResultId::from_bytes([0x33; IDENTITY_BYTES]),
        b"green".to_vec(),
    )
    .expect("result should fit V1 payload bounds");
    assert_eq!(ResultEnvelope::decode(&result.encode()), Ok(result));

    let typed_error = ProtocolError::UnsupportedVersion {
        envelope: EnvelopeKind::Request,
        found: 2,
        supported: CURRENT_PROTOCOL_VERSION,
    };
    let envelope = ErrorEnvelope::new(task, typed_error.clone());
    let decoded = ErrorEnvelope::decode(&envelope.encode()).expect("typed error should decode");
    assert_eq!(decoded.error(), &typed_error);
    assert_eq!(decoded, envelope);
}

#[test]
fn unknown_version_is_rejected_before_payload_decode() {
    let request = RequestEnvelope::new(
        TaskId::from_bytes([1; IDENTITY_BYTES]),
        CommandId::from_bytes([2; IDENTITY_BYTES]),
        Vec::new(),
    )
    .expect("empty request should be valid");
    let mut bytes = request.encode();
    bytes[4..6].copy_from_slice(&2_u16.to_be_bytes());

    assert_eq!(
        RequestEnvelope::decode(&bytes),
        Err(ProtocolError::UnsupportedVersion {
            envelope: EnvelopeKind::Request,
            found: 2,
            supported: CURRENT_PROTOCOL_VERSION,
        })
    );
}

#[test]
fn duplicate_ids_become_typed_protocol_errors() {
    let id = ProjectId::from_bytes([0x44; IDENTITY_BYTES]);
    let duplicate = ensure_unique([id, id]).expect_err("duplicate project ID must fail");
    let error = ProtocolError::from(duplicate.clone());

    assert_eq!(duplicate.kind(), IdentityKind::Project);
    assert_eq!(error.code(), ProtocolErrorCode::DuplicateIdentity);

    let envelope = ErrorEnvelope::new(TaskId::from_bytes([0x55; IDENTITY_BYTES]), error);
    let decoded = ErrorEnvelope::decode(&envelope.encode()).expect("duplicate error should decode");
    assert_eq!(decoded, envelope);
}

#[test]
fn malformed_wire_is_rejected_without_guessing() {
    let request = RequestEnvelope::new(
        TaskId::from_bytes([1; IDENTITY_BYTES]),
        CommandId::from_bytes([2; IDENTITY_BYTES]),
        b"x".to_vec(),
    )
    .expect("request should be valid");

    let mut wrong_magic = request.encode();
    wrong_magic[0] = b'X';
    assert!(matches!(
        RequestEnvelope::decode(&wrong_magic),
        Err(ProtocolError::MalformedEnvelope {
            envelope: EnvelopeKind::Request,
            violation: EnvelopeViolation::BadMagic,
        })
    ));

    let mut trailing = request.encode();
    trailing.push(0);
    assert!(matches!(
        RequestEnvelope::decode(&trailing),
        Err(ProtocolError::MalformedEnvelope {
            envelope: EnvelopeKind::Request,
            violation: EnvelopeViolation::TrailingBytes { remaining: 1 },
        })
    ));
}

#[test]
fn v1_wire_bytes_are_locked() {
    let request = RequestEnvelope::new(
        TaskId::from_bytes([1; IDENTITY_BYTES]),
        CommandId::from_bytes([2; IDENTITY_BYTES]),
        b"ok".to_vec(),
    )
    .expect("request should be valid");
    let mut expected_request = b"FGOS".to_vec();
    expected_request.extend_from_slice(&[0, 1, 1]);
    expected_request.extend_from_slice(&[1; IDENTITY_BYTES]);
    expected_request.extend_from_slice(&[2; IDENTITY_BYTES]);
    expected_request.extend_from_slice(&[0, 0, 0, 2, b'o', b'k']);
    assert_eq!(request.encode(), expected_request);

    let result = ResultEnvelope::new(
        TaskId::from_bytes([3; IDENTITY_BYTES]),
        ResultId::from_bytes([4; IDENTITY_BYTES]),
        vec![0xaa],
    )
    .expect("result should be valid");
    let mut expected_result = b"FGOS".to_vec();
    expected_result.extend_from_slice(&[0, 1, 2]);
    expected_result.extend_from_slice(&[3; IDENTITY_BYTES]);
    expected_result.extend_from_slice(&[4; IDENTITY_BYTES]);
    expected_result.extend_from_slice(&[0, 0, 0, 1, 0xaa]);
    assert_eq!(result.encode(), expected_result);

    let error = ErrorEnvelope::new(
        TaskId::from_bytes([5; IDENTITY_BYTES]),
        ProtocolError::UnsupportedVersion {
            envelope: EnvelopeKind::Request,
            found: 2,
            supported: 1,
        },
    );
    let mut expected_error = b"FGOS".to_vec();
    expected_error.extend_from_slice(&[0, 1, 3]);
    expected_error.extend_from_slice(&[5; IDENTITY_BYTES]);
    expected_error.extend_from_slice(&[0, 1, 1, 0, 2, 0, 1]);
    assert_eq!(error.encode(), expected_error);
}

#[test]
fn event_identity_is_independent_from_event_payload() {
    let event_id = EventId::from_bytes([0x66; IDENTITY_BYTES]);
    let event = EventRecord::new(event_id, b"display text may change".to_vec());
    assert_eq!(event.event_id(), event_id);
    assert_eq!(event.payload(), b"display text may change");
}

fn assert_identity<T>(identity: T)
where
    T: Copy + std::fmt::Display + std::fmt::Debug + PartialEq + std::str::FromStr,
    <T as std::str::FromStr>::Err: std::fmt::Debug + PartialEq,
{
    let text = identity.to_string();
    assert_eq!(text.len(), IDENTITY_BYTES * 2);
    assert_eq!(text.parse::<T>(), Ok(identity));
}
