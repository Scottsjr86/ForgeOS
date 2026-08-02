//! In-memory editor buffer identity, content versioning, and disk-conflict state.
//!
//! Buffers never own repository files. They track exact document identity, local
//! bytes, cursor state, the disk baseline against which edits were made, and the
//! outcome of save attempts. File reads and atomic writes remain owned by
//! `forge-project`; this module only models the state transitions required to
//! drive those operations without creating duplicate authorities.

use forge_protocol::hashes::{hash_canonical_bytes, ContentHash, HashDomain};
use forge_protocol::identities::RepositoryId;
use forge_protocol::paths::RepositoryRelativePath;
use std::collections::BTreeMap;
use std::fmt;
use std::ops::Range;

const BUFFER_ID_BYTES: usize = 16;

/// Stable identity for one in-memory editor buffer.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct BufferId([u8; BUFFER_ID_BYTES]);

impl BufferId {
    pub const fn from_bytes(bytes: [u8; BUFFER_ID_BYTES]) -> Self {
        Self(bytes)
    }

    pub const fn as_bytes(&self) -> &[u8; BUFFER_ID_BYTES] {
        &self.0
    }
}

impl fmt::Debug for BufferId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_tuple("BufferId")
            .field(&self.to_string())
            .finish()
    }
}

impl fmt::Display for BufferId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        for byte in self.0 {
            write!(formatter, "{byte:02x}")?;
        }
        Ok(())
    }
}

/// Canonical repository document identity.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DocumentKey {
    repository_id: RepositoryId,
    relative_path: RepositoryRelativePath,
}

impl DocumentKey {
    pub fn new(repository_id: RepositoryId, relative_path: RepositoryRelativePath) -> Self {
        Self {
            repository_id,
            relative_path,
        }
    }

    pub const fn repository_id(&self) -> RepositoryId {
        self.repository_id
    }

    pub fn relative_path(&self) -> &RepositoryRelativePath {
        &self.relative_path
    }
}

/// Exact content identity observed on disk.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DiskVersion {
    content_hash: ContentHash,
    length: u64,
}

impl DiskVersion {
    pub const fn new(content_hash: ContentHash, length: u64) -> Self {
        Self {
            content_hash,
            length,
        }
    }

    pub fn for_bytes(bytes: &[u8]) -> Self {
        Self {
            content_hash: hash_canonical_bytes(HashDomain::File, bytes),
            length: bytes.len() as u64,
        }
    }

    pub const fn content_hash(self) -> ContentHash {
        self.content_hash
    }

    pub const fn length(self) -> u64 {
        self.length
    }

    pub fn matches(self, bytes: &[u8]) -> bool {
        self == Self::for_bytes(bytes)
    }
}

/// Expected disk state associated with local buffer bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiskBaseline {
    Missing,
    Existing(DiskVersion),
}

/// Monotonic local content generation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ContentVersion(u64);

impl ContentVersion {
    pub const fn initial() -> Self {
        Self(1)
    }

    pub const fn get(self) -> u64 {
        self.0
    }

    fn next(self) -> Result<Self, BufferError> {
        self.0
            .checked_add(1)
            .map(Self)
            .ok_or(BufferError::ContentVersionExhausted)
    }
}

/// Byte-offset cursor and selection state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CursorState {
    anchor: usize,
    head: usize,
}

impl CursorState {
    pub const fn new(anchor: usize, head: usize) -> Self {
        Self { anchor, head }
    }

    pub const fn collapsed(offset: usize) -> Self {
        Self {
            anchor: offset,
            head: offset,
        }
    }

    pub const fn anchor(self) -> usize {
        self.anchor
    }

    pub const fn head(self) -> usize {
        self.head
    }
}

/// Relationship between local bytes and the last known disk state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SynchronizationState {
    Clean {
        disk: DiskVersion,
    },
    Dirty {
        base: DiskBaseline,
    },
    Conflict {
        base: DiskBaseline,
        observed: DiskBaseline,
    },
}

impl SynchronizationState {
    pub const fn is_dirty(self) -> bool {
        !matches!(self, Self::Clean { .. })
    }

    pub const fn expected_disk(self) -> DiskBaseline {
        match self {
            Self::Clean { disk } => DiskBaseline::Existing(disk),
            Self::Dirty { base } | Self::Conflict { base, .. } => base,
        }
    }
}

/// Stable save-failure classes reported by the eventual file adapter.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SaveFailure {
    AccessDenied,
    InvalidPath,
    Io,
    DurabilityUncertain,
    Cancelled,
}

/// Last completed or pending save transition.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SaveOutcome {
    NotRequested,
    Pending {
        content_version: ContentVersion,
        expected: DiskBaseline,
    },
    Succeeded {
        content_version: ContentVersion,
        disk: DiskVersion,
    },
    Conflict {
        content_version: ContentVersion,
        observed: DiskBaseline,
    },
    Failed {
        content_version: ContentVersion,
        failure: SaveFailure,
    },
}

/// Whether closing the buffer would discard unsaved or conflicted work.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CloseDisposition {
    Safe,
    ConfirmationRequired,
    ConflictResolutionRequired,
}

/// Immutable bytes and precondition handed to the file-owning save path.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SaveIntent {
    buffer_id: BufferId,
    document: DocumentKey,
    content_version: ContentVersion,
    expected: DiskBaseline,
    content_hash: ContentHash,
    bytes: Vec<u8>,
}

impl SaveIntent {
    pub const fn buffer_id(&self) -> BufferId {
        self.buffer_id
    }

    pub fn document(&self) -> &DocumentKey {
        &self.document
    }

    pub const fn content_version(&self) -> ContentVersion {
        self.content_version
    }

    pub const fn expected(&self) -> DiskBaseline {
        self.expected
    }

    pub const fn content_hash(&self) -> ContentHash {
        self.content_hash
    }

    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct PendingSave {
    content_version: ContentVersion,
    expected: DiskBaseline,
    content_hash: ContentHash,
    length: u64,
}

/// One authoritative in-memory buffer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EditorBuffer {
    id: BufferId,
    document: DocumentKey,
    bytes: Vec<u8>,
    content_version: ContentVersion,
    cursor: CursorState,
    synchronization: SynchronizationState,
    pending_save: Option<PendingSave>,
    last_save: SaveOutcome,
}

impl EditorBuffer {
    fn existing(id: BufferId, document: DocumentKey, disk: DiskVersion, bytes: Vec<u8>) -> Self {
        Self {
            id,
            document,
            bytes,
            content_version: ContentVersion::initial(),
            cursor: CursorState::collapsed(0),
            synchronization: SynchronizationState::Clean { disk },
            pending_save: None,
            last_save: SaveOutcome::NotRequested,
        }
    }

    fn new_missing(id: BufferId, document: DocumentKey) -> Self {
        Self {
            id,
            document,
            bytes: Vec::new(),
            content_version: ContentVersion::initial(),
            cursor: CursorState::collapsed(0),
            synchronization: SynchronizationState::Dirty {
                base: DiskBaseline::Missing,
            },
            pending_save: None,
            last_save: SaveOutcome::NotRequested,
        }
    }

    pub const fn id(&self) -> BufferId {
        self.id
    }

    pub fn document(&self) -> &DocumentKey {
        &self.document
    }

    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }

    pub const fn content_version(&self) -> ContentVersion {
        self.content_version
    }

    pub const fn cursor(&self) -> CursorState {
        self.cursor
    }

    pub const fn synchronization(&self) -> SynchronizationState {
        self.synchronization
    }

    pub const fn last_save(&self) -> SaveOutcome {
        self.last_save
    }

    pub const fn close_disposition(&self) -> CloseDisposition {
        match self.synchronization {
            SynchronizationState::Clean { .. } => CloseDisposition::Safe,
            SynchronizationState::Dirty { .. } => CloseDisposition::ConfirmationRequired,
            SynchronizationState::Conflict { .. } => CloseDisposition::ConflictResolutionRequired,
        }
    }

    pub fn set_cursor(&mut self, cursor: CursorState) -> Result<(), BufferError> {
        validate_cursor(cursor, self.bytes.len())?;
        self.cursor = cursor;
        Ok(())
    }

    pub fn replace_range(
        &mut self,
        range: Range<usize>,
        replacement: &[u8],
    ) -> Result<ContentVersion, BufferError> {
        validate_range(&range, self.bytes.len())?;
        let mut updated =
            Vec::with_capacity(self.bytes.len() - (range.end - range.start) + replacement.len());
        updated.extend_from_slice(&self.bytes[..range.start]);
        updated.extend_from_slice(replacement);
        updated.extend_from_slice(&self.bytes[range.end..]);

        if updated == self.bytes {
            return Ok(self.content_version);
        }

        self.content_version = self.content_version.next()?;
        self.bytes = updated;
        let cursor = range.start + replacement.len();
        self.cursor = CursorState::collapsed(cursor);
        self.refresh_dirty_state_after_edit();
        Ok(self.content_version)
    }

    /// Records the latest disk state without replacing local bytes.
    pub fn observe_disk(&mut self, observed: DiskBaseline) {
        let previous = self.synchronization;
        let base = previous.expected_disk();
        if observed == base {
            if matches!(previous, SynchronizationState::Conflict { .. }) {
                self.synchronization = match base {
                    DiskBaseline::Existing(disk) if disk.matches(&self.bytes) => {
                        SynchronizationState::Clean { disk }
                    }
                    _ => SynchronizationState::Dirty { base },
                };
            }
            return;
        }
        self.synchronization = SynchronizationState::Conflict { base, observed };
    }

    /// Captures one exact save intent. At most one save may be in flight.
    pub fn prepare_save(&mut self) -> Result<SaveIntent, BufferError> {
        if self.pending_save.is_some() {
            return Err(BufferError::SaveAlreadyPending);
        }
        if matches!(self.synchronization, SynchronizationState::Clean { .. }) {
            return Err(BufferError::NothingToSave);
        }
        if matches!(self.synchronization, SynchronizationState::Conflict { .. }) {
            return Err(BufferError::ConflictUnresolved);
        }

        let expected = self.synchronization.expected_disk();
        let content_hash = hash_canonical_bytes(HashDomain::File, &self.bytes);
        let pending = PendingSave {
            content_version: self.content_version,
            expected,
            content_hash,
            length: self.bytes.len() as u64,
        };
        self.pending_save = Some(pending);
        self.last_save = SaveOutcome::Pending {
            content_version: pending.content_version,
            expected,
        };
        Ok(SaveIntent {
            buffer_id: self.id,
            document: self.document.clone(),
            content_version: pending.content_version,
            expected,
            content_hash,
            bytes: self.bytes.clone(),
        })
    }

    /// Applies a successful save result only to the matching pending generation.
    pub fn record_save_success(
        &mut self,
        content_version: ContentVersion,
        disk: DiskVersion,
    ) -> Result<(), BufferError> {
        let pending = self.match_pending(content_version)?;
        if disk.content_hash() != pending.content_hash || disk.length() != pending.length {
            return Err(BufferError::SaveResultMismatch);
        }
        self.pending_save = None;
        self.last_save = SaveOutcome::Succeeded {
            content_version,
            disk,
        };

        let current_matches_saved =
            self.content_version == content_version && disk.matches(&self.bytes);
        self.synchronization = if current_matches_saved {
            SynchronizationState::Clean { disk }
        } else {
            SynchronizationState::Dirty {
                base: DiskBaseline::Existing(disk),
            }
        };
        Ok(())
    }

    pub fn record_save_conflict(
        &mut self,
        content_version: ContentVersion,
        observed: DiskBaseline,
    ) -> Result<(), BufferError> {
        let pending = self.match_pending(content_version)?;
        self.pending_save = None;
        self.last_save = SaveOutcome::Conflict {
            content_version,
            observed,
        };
        self.synchronization = SynchronizationState::Conflict {
            base: pending.expected,
            observed,
        };
        Ok(())
    }

    pub fn record_save_failure(
        &mut self,
        content_version: ContentVersion,
        failure: SaveFailure,
    ) -> Result<(), BufferError> {
        self.match_pending(content_version)?;
        self.pending_save = None;
        self.last_save = SaveOutcome::Failed {
            content_version,
            failure,
        };
        Ok(())
    }

    fn match_pending(&self, content_version: ContentVersion) -> Result<PendingSave, BufferError> {
        let pending = self.pending_save.ok_or(BufferError::NoSavePending)?;
        if pending.content_version != content_version {
            return Err(BufferError::StaleSaveResult {
                expected: pending.content_version,
                found: content_version,
            });
        }
        Ok(pending)
    }

    fn refresh_dirty_state_after_edit(&mut self) {
        let previous = self.synchronization;
        let base = previous.expected_disk();
        if matches!(previous, SynchronizationState::Conflict { .. }) {
            return;
        }

        self.synchronization = match base {
            DiskBaseline::Existing(disk) if disk.matches(&self.bytes) => {
                SynchronizationState::Clean { disk }
            }
            _ => SynchronizationState::Dirty { base },
        };
    }
}

/// Result of opening a document through the registry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OpenBufferResult {
    Opened(BufferId),
    Existing(BufferId),
}

/// Canonical registry that prevents duplicate buffer authorities for one document.
#[derive(Debug, Default)]
pub struct BufferRegistry {
    buffers: BTreeMap<BufferId, EditorBuffer>,
    documents: BTreeMap<DocumentKey, BufferId>,
}

impl BufferRegistry {
    pub fn new() -> Self {
        Self {
            buffers: BTreeMap::new(),
            documents: BTreeMap::new(),
        }
    }

    pub fn len(&self) -> usize {
        self.buffers.len()
    }

    pub fn is_empty(&self) -> bool {
        self.buffers.is_empty()
    }

    pub fn get(&self, id: BufferId) -> Option<&EditorBuffer> {
        self.buffers.get(&id)
    }

    pub fn get_mut(&mut self, id: BufferId) -> Option<&mut EditorBuffer> {
        self.buffers.get_mut(&id)
    }

    pub fn buffer_for_document(&self, document: &DocumentKey) -> Option<BufferId> {
        self.documents.get(document).copied()
    }

    pub fn open_existing(
        &mut self,
        id: BufferId,
        document: DocumentKey,
        disk: DiskVersion,
        bytes: Vec<u8>,
    ) -> Result<OpenBufferResult, BufferError> {
        if !disk.matches(&bytes) {
            return Err(BufferError::DiskVersionMismatch);
        }
        if let Some(existing_id) = self.documents.get(&document).copied() {
            let existing = self
                .buffers
                .get_mut(&existing_id)
                .expect("document index must resolve to a buffer");
            existing.observe_disk(DiskBaseline::Existing(disk));
            return Ok(OpenBufferResult::Existing(existing_id));
        }
        self.ensure_id_available(id, &document)?;
        self.documents.insert(document.clone(), id);
        self.buffers
            .insert(id, EditorBuffer::existing(id, document, disk, bytes));
        Ok(OpenBufferResult::Opened(id))
    }

    pub fn open_new(
        &mut self,
        id: BufferId,
        document: DocumentKey,
    ) -> Result<OpenBufferResult, BufferError> {
        if let Some(existing_id) = self.documents.get(&document).copied() {
            let existing = self
                .buffers
                .get_mut(&existing_id)
                .expect("document index must resolve to a buffer");
            existing.observe_disk(DiskBaseline::Missing);
            return Ok(OpenBufferResult::Existing(existing_id));
        }
        self.ensure_id_available(id, &document)?;
        self.documents.insert(document.clone(), id);
        self.buffers
            .insert(id, EditorBuffer::new_missing(id, document));
        Ok(OpenBufferResult::Opened(id))
    }

    pub fn remove_clean(&mut self, id: BufferId) -> Result<EditorBuffer, BufferError> {
        let disposition = self
            .buffers
            .get(&id)
            .ok_or(BufferError::UnknownBuffer(id))?
            .close_disposition();
        if disposition != CloseDisposition::Safe {
            return Err(BufferError::DestructiveCloseBlocked(disposition));
        }
        let removed = self
            .buffers
            .remove(&id)
            .expect("buffer existence was checked");
        self.documents.remove(removed.document());
        Ok(removed)
    }

    fn ensure_id_available(&self, id: BufferId, document: &DocumentKey) -> Result<(), BufferError> {
        if let Some(existing) = self.buffers.get(&id) {
            return Err(BufferError::DuplicateBufferId {
                id,
                existing: existing.document().clone(),
                requested: document.clone(),
            });
        }
        Ok(())
    }
}

/// Exact buffer-state transition failure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BufferError {
    DuplicateBufferId {
        id: BufferId,
        existing: DocumentKey,
        requested: DocumentKey,
    },
    UnknownBuffer(BufferId),
    DiskVersionMismatch,
    InvalidEditRange {
        start: usize,
        end: usize,
        length: usize,
    },
    InvalidCursor {
        anchor: usize,
        head: usize,
        length: usize,
    },
    ContentVersionExhausted,
    NothingToSave,
    ConflictUnresolved,
    SaveAlreadyPending,
    NoSavePending,
    StaleSaveResult {
        expected: ContentVersion,
        found: ContentVersion,
    },
    SaveResultMismatch,
    DestructiveCloseBlocked(CloseDisposition),
}

impl fmt::Display for BufferError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DuplicateBufferId { id, .. } => {
                write!(formatter, "buffer identity {id} is already in use")
            }
            Self::UnknownBuffer(id) => write!(formatter, "buffer {id} is not registered"),
            Self::DiskVersionMismatch => {
                formatter.write_str("disk version does not match supplied bytes")
            }
            Self::InvalidEditRange { start, end, length } => write!(
                formatter,
                "edit range {start}..{end} is invalid for {length} bytes"
            ),
            Self::InvalidCursor {
                anchor,
                head,
                length,
            } => write!(
                formatter,
                "cursor {anchor}..{head} is invalid for {length} bytes"
            ),
            Self::ContentVersionExhausted => {
                formatter.write_str("buffer content version is exhausted")
            }
            Self::NothingToSave => formatter.write_str("buffer has no unsaved content"),
            Self::ConflictUnresolved => {
                formatter.write_str("buffer disk conflict must be resolved before saving")
            }
            Self::SaveAlreadyPending => {
                formatter.write_str("a save is already pending for this buffer")
            }
            Self::NoSavePending => formatter.write_str("buffer has no pending save"),
            Self::StaleSaveResult { expected, found } => write!(
                formatter,
                "save result targets content version {}, expected {}",
                found.get(),
                expected.get()
            ),
            Self::SaveResultMismatch => {
                formatter.write_str("save result disk version does not match saved bytes")
            }
            Self::DestructiveCloseBlocked(disposition) => {
                write!(formatter, "buffer close is blocked by {disposition:?}")
            }
        }
    }
}

impl std::error::Error for BufferError {}

fn validate_range(range: &Range<usize>, length: usize) -> Result<(), BufferError> {
    if range.start > range.end || range.end > length {
        return Err(BufferError::InvalidEditRange {
            start: range.start,
            end: range.end,
            length,
        });
    }
    Ok(())
}

fn validate_cursor(cursor: CursorState, length: usize) -> Result<(), BufferError> {
    if cursor.anchor() > length || cursor.head() > length {
        return Err(BufferError::InvalidCursor {
            anchor: cursor.anchor(),
            head: cursor.head(),
            length,
        });
    }
    Ok(())
}
