use forge_core::state::StateRecord;
use forge_nyx_client::permissions::{
    NyxAuthorityScope, NyxPermissionCheckpoint, NyxPermissionCheckpointStatus,
    NyxPermissionDecisionKind, NyxPermissionError, NyxToolName, NyxToolRequest,
};
use forge_protocol::identities::{CommandId, RepositoryId, TaskId, IDENTITY_BYTES};
use forge_protocol::paths::RepositoryRelativePath;

fn task(byte: u8) -> TaskId {
    TaskId::from_bytes([byte; IDENTITY_BYTES])
}

fn repository(byte: u8) -> RepositoryId {
    RepositoryId::from_bytes([byte; IDENTITY_BYTES])
}

fn command(byte: u8) -> CommandId {
    CommandId::from_bytes([byte; IDENTITY_BYTES])
}

fn request(payload: &[u8], expiry: u64) -> NyxToolRequest {
    let scope = NyxAuthorityScope::new(
        NyxToolName::new("repository.read").unwrap(),
        repository(2),
        [
            RepositoryRelativePath::new("src/lib.rs").unwrap(),
            RepositoryRelativePath::new("Cargo.toml").unwrap(),
        ],
        Some(command(3)),
    )
    .unwrap();
    NyxToolRequest::new(task(1), scope, payload.to_vec(), expiry).unwrap()
}

#[test]
fn scope_order_is_canonical_and_request_payload_is_identity_bound() {
    let left_scope = NyxAuthorityScope::new(
        NyxToolName::new("repository.read").unwrap(),
        repository(2),
        [
            RepositoryRelativePath::new("src/lib.rs").unwrap(),
            RepositoryRelativePath::new("Cargo.toml").unwrap(),
        ],
        Some(command(3)),
    )
    .unwrap();
    let right_scope = NyxAuthorityScope::new(
        NyxToolName::new("repository.read").unwrap(),
        repository(2),
        [
            RepositoryRelativePath::new("Cargo.toml").unwrap(),
            RepositoryRelativePath::new("src/lib.rs").unwrap(),
        ],
        Some(command(3)),
    )
    .unwrap();
    let left = NyxToolRequest::new(task(1), left_scope, b"exact payload".to_vec(), 500).unwrap();
    let right = NyxToolRequest::new(task(1), right_scope, b"exact payload".to_vec(), 500).unwrap();
    let altered = request(b"altered payload", 500);

    assert_eq!(left.identity(), right.identity());
    assert_ne!(left.identity(), altered.identity());
    assert_eq!(
        left.scope().paths()[0].as_path().to_str(),
        Some("Cargo.toml")
    );
}

#[test]
fn approval_releases_only_the_exact_request_and_consumes_the_token() {
    let exact = request(b"cargo test", 500);
    let mut checkpoint = NyxPermissionCheckpoint::pending(exact.clone());
    let token = checkpoint
        .decide(exact.identity(), NyxPermissionDecisionKind::Approve, 100)
        .unwrap()
        .expect("approval token");

    let unrelated_token = {
        let other = request(b"other", 500);
        let mut other_checkpoint = NyxPermissionCheckpoint::pending(other.clone());
        other_checkpoint
            .decide(other.identity(), NyxPermissionDecisionKind::Approve, 100)
            .unwrap()
            .unwrap()
    };
    assert!(matches!(
        checkpoint.authorize(&exact, unrelated_token, 200),
        Err(NyxPermissionError::ResumeTokenMismatch { .. })
    ));

    let authorized = checkpoint.authorize(&exact, token, 200).unwrap();
    assert_eq!(authorized.request(), &exact);
    assert_eq!(authorized.resume_token(), token);
    assert_eq!(checkpoint.status(), NyxPermissionCheckpointStatus::Consumed);
    assert_eq!(
        checkpoint.authorize(&exact, token, 201),
        Err(NyxPermissionError::ResumeTokenConsumed)
    );
}

#[test]
fn payload_mutation_cannot_reuse_an_approval() {
    let exact = request(b"read src/lib.rs", 500);
    let altered = request(b"read src/main.rs", 500);
    let mut checkpoint = NyxPermissionCheckpoint::pending(exact.clone());
    let token = checkpoint
        .decide(exact.identity(), NyxPermissionDecisionKind::Approve, 100)
        .unwrap()
        .unwrap();

    assert!(matches!(
        checkpoint.authorize(&altered, token, 200),
        Err(NyxPermissionError::RequestIdentityMismatch { .. })
    ));
    assert_eq!(checkpoint.status(), NyxPermissionCheckpointStatus::Approved);
}

#[test]
fn denial_and_expiration_never_release_a_request() {
    let denied_request = request(b"shell", 500);
    let mut denied = NyxPermissionCheckpoint::pending(denied_request.clone());
    assert_eq!(
        denied
            .decide(
                denied_request.identity(),
                NyxPermissionDecisionKind::Deny,
                100,
            )
            .unwrap(),
        None
    );
    let unrelated_token = {
        let approved_request = request(b"other", 500);
        let mut approved = NyxPermissionCheckpoint::pending(approved_request.clone());
        approved
            .decide(
                approved_request.identity(),
                NyxPermissionDecisionKind::Approve,
                100,
            )
            .unwrap()
            .unwrap()
    };
    assert_eq!(
        denied.authorize(&denied_request, unrelated_token, 200),
        Err(NyxPermissionError::RequestDenied)
    );

    let expiring = request(b"read", 120);
    let mut checkpoint = NyxPermissionCheckpoint::pending(expiring.clone());
    let token = checkpoint
        .decide(expiring.identity(), NyxPermissionDecisionKind::Approve, 100)
        .unwrap()
        .unwrap();
    assert_eq!(
        checkpoint.authorize(&expiring, token, 121),
        Err(NyxPermissionError::RequestExpired {
            expired_at: 120,
            observed_at: 121,
        })
    );
}

#[test]
fn approval_after_expiration_is_rejected_without_recording_a_decision() {
    let exact = request(b"read", 120);
    let mut checkpoint = NyxPermissionCheckpoint::pending(exact.clone());
    assert_eq!(
        checkpoint.decide(exact.identity(), NyxPermissionDecisionKind::Approve, 121,),
        Err(NyxPermissionError::RequestExpired {
            expired_at: 120,
            observed_at: 121,
        })
    );
    assert_eq!(checkpoint.status(), NyxPermissionCheckpointStatus::Pending);
}

#[test]
fn checkpoint_round_trip_preserves_exact_token_and_consumed_state() {
    let exact = request(b"cargo test", 500);
    let mut checkpoint = NyxPermissionCheckpoint::pending(exact.clone());
    let token = checkpoint
        .decide(exact.identity(), NyxPermissionDecisionKind::Approve, 100)
        .unwrap()
        .unwrap();

    let record = checkpoint.to_state_record().unwrap();
    let encoded = record.encode();
    let state = StateRecord::decode(&encoded).unwrap();
    let mut altered_payload = state.payload().to_vec();
    let payload_offset = altered_payload
        .windows(b"cargo test".len())
        .position(|window| window == b"cargo test")
        .expect("fixture payload is encoded exactly");
    altered_payload[payload_offset] ^= 0x01;
    let rechecksummed = StateRecord::new(state.record_type(), altered_payload).unwrap();
    assert!(matches!(
        NyxPermissionCheckpoint::from_state_record(&rechecksummed),
        Err(NyxPermissionError::StoredRequestIdentityMismatch { .. })
    ));

    let mut restored = NyxPermissionCheckpoint::from_state_record(&state).unwrap();
    assert_eq!(restored, checkpoint);
    restored.authorize(&exact, token, 200).unwrap();

    let consumed_bytes = restored.to_state_record().unwrap().encode();
    let consumed_state = StateRecord::decode(&consumed_bytes).unwrap();
    let mut consumed = NyxPermissionCheckpoint::from_state_record(&consumed_state).unwrap();
    assert_eq!(consumed.status(), NyxPermissionCheckpointStatus::Consumed);
    assert_eq!(
        consumed.authorize(&exact, token, 201),
        Err(NyxPermissionError::ResumeTokenConsumed)
    );
}

#[test]
fn corrupted_or_wrong_type_checkpoint_is_rejected() {
    let exact = request(b"read", 500);
    let checkpoint = NyxPermissionCheckpoint::pending(exact);
    let encoded = checkpoint.to_state_record().unwrap().encode();
    let mut corrupt = encoded.clone();
    let index = corrupt.len() - 5;
    corrupt[index] ^= 0x01;
    assert!(StateRecord::decode(&corrupt).is_err());

    let wrong = StateRecord::new(7, vec![1, 2, 3]).unwrap();
    assert_eq!(
        NyxPermissionCheckpoint::from_state_record(&wrong),
        Err(NyxPermissionError::WrongStateRecordType {
            expected: 0x4e59,
            actual: 7,
        })
    );
}

#[test]
fn malformed_names_duplicate_paths_and_wrong_request_identity_fail_closed() {
    assert!(matches!(
        NyxToolName::new("Repository.Read"),
        Err(NyxPermissionError::InvalidToolNameByte { .. })
    ));
    let path = RepositoryRelativePath::new("src/lib.rs").unwrap();
    assert!(matches!(
        NyxAuthorityScope::new(
            NyxToolName::new("repository.read").unwrap(),
            repository(2),
            [path.clone(), path],
            None,
        ),
        Err(NyxPermissionError::DuplicateScopePath(_))
    ));

    let exact = request(b"read", 500);
    let other = request(b"other", 500);
    let mut checkpoint = NyxPermissionCheckpoint::pending(exact);
    assert!(matches!(
        checkpoint.decide(other.identity(), NyxPermissionDecisionKind::Approve, 100,),
        Err(NyxPermissionError::RequestIdentityMismatch { .. })
    ));
}
