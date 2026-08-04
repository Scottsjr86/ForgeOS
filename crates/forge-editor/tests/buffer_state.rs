use forge_editor::buffers::{
    BufferError, BufferId, BufferRegistry, CloseDisposition, ContentVersion, CursorState,
    DiskBaseline, DiskVersion, DocumentKey, OpenBufferResult, SaveFailure, SaveOutcome,
    SynchronizationState,
};
use forge_protocol::identities::RepositoryId;
use forge_protocol::paths::RepositoryRelativePath;

fn buffer_id(seed: u8) -> BufferId {
    BufferId::from_bytes([seed; 16])
}

fn repository_id(seed: u8) -> RepositoryId {
    RepositoryId::from_bytes([seed; 16])
}

fn document(repository_seed: u8, path: &str) -> DocumentKey {
    DocumentKey::new(
        repository_id(repository_seed),
        RepositoryRelativePath::new(path).expect("canonical fixture path"),
    )
}

#[test]
fn reopening_the_same_document_reuses_one_buffer_authority() {
    let mut registry = BufferRegistry::new();
    let key = document(1, "src/lib.rs");
    let bytes = b"pub fn one() {}\n".to_vec();
    let disk = DiskVersion::for_bytes(&bytes);

    assert_eq!(
        registry.open_existing(buffer_id(1), key.clone(), disk, bytes.clone()),
        Ok(OpenBufferResult::Opened(buffer_id(1)))
    );
    assert_eq!(
        registry.open_existing(buffer_id(2), key.clone(), disk, bytes),
        Ok(OpenBufferResult::Existing(buffer_id(1)))
    );
    assert_eq!(registry.len(), 1);
    assert_eq!(registry.buffer_for_document(&key), Some(buffer_id(1)));
}

#[test]
fn one_buffer_identity_cannot_name_two_documents() {
    let mut registry = BufferRegistry::new();
    let first = document(1, "src/lib.rs");
    let second = document(1, "src/main.rs");
    registry
        .open_new(buffer_id(1), first.clone())
        .expect("first buffer opens");

    assert!(matches!(
        registry.open_new(buffer_id(1), second.clone()),
        Err(BufferError::DuplicateBufferId {
            id,
            existing,
            requested
        }) if id == buffer_id(1) && existing == first && requested == second
    ));
}

#[test]
fn opening_existing_bytes_requires_the_exact_disk_version() {
    let mut registry = BufferRegistry::new();
    let wrong = DiskVersion::for_bytes(b"different");
    assert_eq!(
        registry.open_existing(
            buffer_id(1),
            document(1, "src/lib.rs"),
            wrong,
            b"actual".to_vec(),
        ),
        Err(BufferError::DiskVersionMismatch)
    );
    assert!(registry.is_empty());
}

#[test]
fn edits_advance_content_version_cursor_and_dirty_state() {
    let mut registry = BufferRegistry::new();
    let original = b"abc".to_vec();
    let disk = DiskVersion::for_bytes(&original);
    registry
        .open_existing(buffer_id(1), document(1, "note.txt"), disk, original)
        .expect("buffer opens");
    let buffer = registry.get_mut(buffer_id(1)).expect("buffer exists");

    let version = buffer.replace_range(1..2, b"XYZ").expect("edit succeeds");
    assert_eq!(version.get(), ContentVersion::initial().get() + 1);
    assert_eq!(buffer.bytes(), b"aXYZc");
    assert_eq!(buffer.cursor(), CursorState::collapsed(4));
    assert_eq!(
        buffer.synchronization(),
        SynchronizationState::Dirty {
            base: DiskBaseline::Existing(disk)
        }
    );
    assert_eq!(
        buffer.close_disposition(),
        CloseDisposition::ConfirmationRequired
    );
}

#[test]
fn editing_back_to_the_disk_bytes_restores_clean_state() {
    let mut registry = BufferRegistry::new();
    let original = b"abc".to_vec();
    let disk = DiskVersion::for_bytes(&original);
    registry
        .open_existing(buffer_id(1), document(1, "note.txt"), disk, original)
        .expect("buffer opens");
    let buffer = registry.get_mut(buffer_id(1)).expect("buffer exists");

    buffer.replace_range(1..2, b"Z").expect("first edit");
    buffer.replace_range(1..2, b"b").expect("revert edit");
    assert_eq!(
        buffer.synchronization(),
        SynchronizationState::Clean { disk }
    );
    assert_eq!(buffer.close_disposition(), CloseDisposition::Safe);
}

#[test]
fn external_disk_change_creates_conflict_without_replacing_local_bytes() {
    let mut registry = BufferRegistry::new();
    let original = b"abc".to_vec();
    let disk = DiskVersion::for_bytes(&original);
    registry
        .open_existing(
            buffer_id(1),
            document(1, "note.txt"),
            disk,
            original.clone(),
        )
        .expect("buffer opens");
    let buffer = registry.get_mut(buffer_id(1)).expect("buffer exists");
    let observed = DiskBaseline::Existing(DiskVersion::for_bytes(b"external"));

    buffer.observe_disk(observed);
    assert_eq!(buffer.bytes(), original);
    assert_eq!(
        buffer.synchronization(),
        SynchronizationState::Conflict {
            base: DiskBaseline::Existing(disk),
            observed,
        }
    );
    assert_eq!(buffer.prepare_save(), Err(BufferError::ConflictUnresolved));
    assert_eq!(
        buffer.close_disposition(),
        CloseDisposition::ConflictResolutionRequired
    );
}

#[test]
fn matching_save_success_marks_the_current_generation_clean() {
    let mut registry = BufferRegistry::new();
    registry
        .open_new(buffer_id(1), document(1, "new.txt"))
        .expect("new buffer opens");
    let buffer = registry.get_mut(buffer_id(1)).expect("buffer exists");
    buffer
        .replace_range(0..0, b"saved")
        .expect("content is added");

    let intent = buffer.prepare_save().expect("save intent");
    let disk = DiskVersion::for_bytes(intent.bytes());
    buffer
        .record_save_success(intent.content_version(), disk)
        .expect("save result applies");

    assert_eq!(
        buffer.synchronization(),
        SynchronizationState::Clean { disk }
    );
    assert_eq!(
        buffer.last_save(),
        SaveOutcome::Succeeded {
            content_version: intent.content_version(),
            disk,
        }
    );
}

#[test]
fn late_save_success_does_not_erase_newer_dirty_edits() {
    let mut registry = BufferRegistry::new();
    let original = b"abc".to_vec();
    let original_disk = DiskVersion::for_bytes(&original);
    registry
        .open_existing(
            buffer_id(1),
            document(1, "note.txt"),
            original_disk,
            original,
        )
        .expect("buffer opens");
    let buffer = registry.get_mut(buffer_id(1)).expect("buffer exists");
    buffer.replace_range(3..3, b"d").expect("first edit");
    let intent = buffer.prepare_save().expect("save intent");
    let saved_disk = DiskVersion::for_bytes(intent.bytes());
    buffer.replace_range(4..4, b"e").expect("newer edit");

    buffer
        .record_save_success(intent.content_version(), saved_disk)
        .expect("older save result applies");
    assert_eq!(buffer.bytes(), b"abcde");
    assert_eq!(
        buffer.synchronization(),
        SynchronizationState::Dirty {
            base: DiskBaseline::Existing(saved_disk)
        }
    );
}

#[test]
fn save_conflicts_and_failures_are_explicit_terminal_outcomes() {
    let mut registry = BufferRegistry::new();
    registry
        .open_new(buffer_id(1), document(1, "new.txt"))
        .expect("new buffer opens");
    let buffer = registry.get_mut(buffer_id(1)).expect("buffer exists");
    let first = buffer.prepare_save().expect("first save intent");
    let observed = DiskBaseline::Existing(DiskVersion::for_bytes(b"someone else"));
    buffer
        .record_save_conflict(first.content_version(), observed)
        .expect("conflict recorded");
    assert_eq!(
        buffer.last_save(),
        SaveOutcome::Conflict {
            content_version: first.content_version(),
            observed,
        }
    );

    let mut second_registry = BufferRegistry::new();
    second_registry
        .open_new(buffer_id(2), document(1, "other.txt"))
        .expect("second buffer opens");
    let second_buffer = second_registry
        .get_mut(buffer_id(2))
        .expect("second buffer exists");
    let second = second_buffer.prepare_save().expect("second save intent");
    second_buffer
        .record_save_failure(second.content_version(), SaveFailure::Io)
        .expect("failure recorded");
    assert_eq!(
        second_buffer.last_save(),
        SaveOutcome::Failed {
            content_version: second.content_version(),
            failure: SaveFailure::Io,
        }
    );
    assert!(second_buffer.synchronization().is_dirty());
}

#[test]
fn invalid_ranges_cursors_and_destructive_close_are_rejected() {
    let mut registry = BufferRegistry::new();
    registry
        .open_new(buffer_id(1), document(1, "new.txt"))
        .expect("new buffer opens");
    let buffer = registry.get_mut(buffer_id(1)).expect("buffer exists");
    assert!(matches!(
        buffer.replace_range(1..0, b"x"),
        Err(BufferError::InvalidEditRange { .. })
    ));
    assert!(matches!(
        buffer.set_cursor(CursorState::collapsed(1)),
        Err(BufferError::InvalidCursor { .. })
    ));
    assert_eq!(
        registry.remove_clean(buffer_id(1)),
        Err(BufferError::DestructiveCloseBlocked(
            CloseDisposition::ConfirmationRequired
        ))
    );
}

#[test]
fn destructive_discard_requires_the_exact_current_generation() {
    let mut registry = BufferRegistry::new();
    registry
        .open_new(buffer_id(9), document(1, "discard.txt"))
        .expect("buffer opens");
    registry
        .get_mut(buffer_id(9))
        .unwrap()
        .replace_range(0..0, b"first")
        .expect("first edit");
    let stale = registry.get(buffer_id(9)).unwrap().discard_confirmation();
    registry
        .get_mut(buffer_id(9))
        .unwrap()
        .replace_range(0..5, b"second")
        .expect("second edit");

    assert!(matches!(
        registry.remove_discarding(stale),
        Err(BufferError::StaleDiscardConfirmation { .. })
    ));
    assert_eq!(registry.get(buffer_id(9)).unwrap().bytes(), b"second");

    let fresh = registry.get(buffer_id(9)).unwrap().discard_confirmation();
    let removed = registry
        .remove_discarding(fresh)
        .expect("fresh confirmation removes buffer");
    assert_eq!(removed.bytes(), b"second");
    assert!(registry.is_empty());
}
