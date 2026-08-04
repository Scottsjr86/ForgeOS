use forge_core::project_registry::SafeWorkspaceSnapshot;
use forge_core::recovery::WorkspaceRecoveryRecord;
use forge_project::recovery_store::{
    RecoveryChoice, RecoveryImageStatus, WorkspaceRecoveryStore, WorkspaceRecoveryStoreError,
};
use forge_protocol::identities::{ProjectId, IDENTITY_BYTES};
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_FIXTURE: AtomicU64 = AtomicU64::new(1);

struct Fixture {
    root: PathBuf,
}

impl Fixture {
    fn new(label: &str) -> Self {
        let sequence = NEXT_FIXTURE.fetch_add(1, Ordering::Relaxed);
        let root = std::env::temp_dir().join(format!(
            "forgeos-recovery-{label}-{}-{sequence}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        Self { root }
    }

    fn store(&self) -> WorkspaceRecoveryStore {
        WorkspaceRecoveryStore::new(self.root.join("workspace.recovery")).unwrap()
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

fn record(generation: u64, payload: &[u8]) -> WorkspaceRecoveryRecord {
    WorkspaceRecoveryRecord::new(
        generation,
        ProjectId::from_bytes([7; IDENTITY_BYTES]),
        None,
        SafeWorkspaceSnapshot::new(1, payload.to_vec()).unwrap(),
        Vec::new(),
        Vec::new(),
    )
    .unwrap()
}

#[test]
fn generation_guarded_publication_retains_previous_valid_image() {
    let fixture = Fixture::new("publish");
    let store = fixture.store();
    let first = record(1, b"first");
    let second = record(2, b"second");

    store.create(&first).unwrap();
    store.publish_next(1, &second).unwrap();

    assert_eq!(store.open_current().unwrap().record(), &second);
    assert!(matches!(
        store.publish_next(0, &first),
        Err(WorkspaceRecoveryStoreError::GenerationMismatch {
            expected: 0,
            found: 2
        })
    ));
    assert!(matches!(
        store.publish_next(2, &record(4, b"skipped")),
        Err(WorkspaceRecoveryStoreError::NonSequentialGeneration {
            expected: 3,
            found: 4
        })
    ));
}

#[test]
fn interrupted_publication_is_visible_but_current_remains_authoritative() {
    let fixture = Fixture::new("interrupted");
    let store = fixture.store();
    let first = record(1, b"safe");
    store.create(&first).unwrap();
    fs::write(
        store.staged_path(),
        record(2, b"uncommitted")
            .to_state_record()
            .unwrap()
            .encode(),
    )
    .unwrap();

    let assessment = store.assess().unwrap();
    assert!(matches!(
        assessment.current(),
        RecoveryImageStatus::Valid { generation: 1, .. }
    ));
    assert!(assessment.interrupted_write_present());
    assert_eq!(
        assessment.choices(),
        &[
            RecoveryChoice::KeepCurrent,
            RecoveryChoice::DiscardInterruptedWrite
        ]
    );
    assert!(matches!(
        store.publish_next(1, &record(2, b"must-not-hide-stage")),
        Err(WorkspaceRecoveryStoreError::InterruptedWriteRequiresResolution)
    ));
    assert!(store.discard_interrupted_write().unwrap());
    assert!(!store.assess().unwrap().interrupted_write_present());
    assert_eq!(store.open_current().unwrap().record(), &first);
}

#[test]
fn corrupt_current_offers_explicit_previous_restore() {
    let fixture = Fixture::new("corrupt");
    let store = fixture.store();
    let first = record(1, b"first");
    let second = record(2, b"second");
    store.create(&first).unwrap();
    store.publish_next(1, &second).unwrap();
    fs::write(store.target_path(), b"corrupt-current").unwrap();

    let assessment = store.assess().unwrap();
    assert_eq!(assessment.current(), RecoveryImageStatus::Invalid);
    assert!(matches!(
        assessment.previous(),
        RecoveryImageStatus::Valid { generation: 1, .. }
    ));
    assert_eq!(assessment.choices(), &[RecoveryChoice::RestorePrevious]);

    let restored = store.restore_previous_if_current_unusable().unwrap();
    assert_eq!(restored, first);
    assert_eq!(store.open_current().unwrap().record(), &first);
}

#[test]
fn valid_current_can_never_be_silently_replaced_by_older_previous() {
    let fixture = Fixture::new("protect-current");
    let store = fixture.store();
    store.create(&record(1, b"old")).unwrap();
    store.publish_next(1, &record(2, b"new")).unwrap();

    let assessment = store.assess().unwrap();
    assert_eq!(assessment.choices(), &[RecoveryChoice::KeepCurrent]);
    assert!(matches!(
        store.restore_previous_if_current_unusable(),
        Err(WorkspaceRecoveryStoreError::ValidCurrentWouldBeOverwritten)
    ));
    assert_eq!(store.open_current().unwrap().record().generation(), 2);
}
