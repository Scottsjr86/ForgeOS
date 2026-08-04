//! Thin composition adapter between editor state and project-owned file access.
//!
//! `forge-editor` owns buffer identity and state transitions. `forge-project`
//! owns repository boundaries and atomic file replacement. This module joins
//! those public contracts without moving either authority into the other crate.

use forge_editor::buffers::{
    BufferError, BufferId, BufferRegistry, ContentVersion, DiscardConfirmation, DiskBaseline,
    DiskVersion, DocumentKey, EditorBuffer, OpenBufferResult, SaveFailure, SynchronizationState,
};
use forge_project::files::{
    FileExpectation, FileRevision, ProjectFileAccess, ProjectFileError, WriteDurability,
};
use forge_protocol::paths::{RepositoryPathError, RepositoryPathRequest};
use std::collections::BTreeMap;
use std::fmt;

/// Result of one committed editor save.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EditorSaveResult {
    buffer_id: BufferId,
    content_version: ContentVersion,
    disk: DiskVersion,
    durability: WriteDurability,
}

impl EditorSaveResult {
    pub const fn buffer_id(self) -> BufferId {
        self.buffer_id
    }

    pub const fn content_version(self) -> ContentVersion {
        self.content_version
    }

    pub const fn disk(self) -> DiskVersion {
        self.disk
    }

    pub const fn durability(self) -> WriteDurability {
        self.durability
    }
}

/// Product-facing editor workspace bound to one validated project repository.
#[derive(Debug)]
pub struct EditorWorkspace {
    files: ProjectFileAccess,
    buffers: BufferRegistry,
    file_expectations: BTreeMap<BufferId, FileExpectation>,
}

impl EditorWorkspace {
    pub fn new(files: ProjectFileAccess) -> Self {
        Self {
            files,
            buffers: BufferRegistry::new(),
            file_expectations: BTreeMap::new(),
        }
    }

    pub fn buffers(&self) -> &BufferRegistry {
        &self.buffers
    }

    pub fn buffer(&self, buffer_id: BufferId) -> Option<&EditorBuffer> {
        self.buffers.get(buffer_id)
    }

    pub fn buffer_mut(&mut self, buffer_id: BufferId) -> Option<&mut EditorBuffer> {
        self.buffers.get_mut(buffer_id)
    }

    /// Opens the exact current repository bytes into one canonical buffer.
    pub fn open_existing(
        &mut self,
        buffer_id: BufferId,
        request: &RepositoryPathRequest,
    ) -> Result<OpenBufferResult, EditorWorkspaceError> {
        let snapshot = self.files.read(request)?;
        let revision = snapshot.revision();
        let document = DocumentKey::new(snapshot.repository_id(), snapshot.relative_path().clone());
        let disk = disk_version(revision);
        let bytes = snapshot.into_bytes();
        let result = self
            .buffers
            .open_existing(buffer_id, document, disk, bytes)?;
        self.refresh_exact_expectation(result, revision, disk);
        Ok(result)
    }

    /// Opens an existing file or creates a dirty empty buffer when the file is absent.
    pub fn open_or_create(
        &mut self,
        buffer_id: BufferId,
        request: &RepositoryPathRequest,
    ) -> Result<OpenBufferResult, EditorWorkspaceError> {
        match self.files.read(request) {
            Ok(snapshot) => {
                let revision = snapshot.revision();
                let document =
                    DocumentKey::new(snapshot.repository_id(), snapshot.relative_path().clone());
                let disk = disk_version(revision);
                let bytes = snapshot.into_bytes();
                let result = self
                    .buffers
                    .open_existing(buffer_id, document, disk, bytes)?;
                self.refresh_exact_expectation(result, revision, disk);
                Ok(result)
            }
            Err(ProjectFileError::Missing { .. }) => {
                let document =
                    DocumentKey::new(request.repository_id(), request.relative_path().clone());
                let result = self.buffers.open_new(buffer_id, document)?;
                let actual_id = opened_buffer_id(result);
                self.file_expectations
                    .entry(actual_id)
                    .or_insert(FileExpectation::Missing);
                Ok(result)
            }
            Err(error) => Err(EditorWorkspaceError::File(error)),
        }
    }

    /// Rechecks current disk identity without replacing local bytes.
    pub fn refresh(
        &mut self,
        buffer_id: BufferId,
    ) -> Result<SynchronizationState, EditorWorkspaceError> {
        let request = self.request_for(buffer_id)?;
        let (observed, exact) = match self.files.read(&request) {
            Ok(snapshot) => (
                DiskBaseline::Existing(disk_version(snapshot.revision())),
                FileExpectation::Exact(snapshot.revision()),
            ),
            Err(ProjectFileError::Missing { .. }) => {
                (DiskBaseline::Missing, FileExpectation::Missing)
            }
            Err(error) => return Err(EditorWorkspaceError::File(error)),
        };
        let buffer = self
            .buffers
            .get_mut(buffer_id)
            .ok_or(EditorWorkspaceError::Buffer(BufferError::UnknownBuffer(
                buffer_id,
            )))?;
        buffer.observe_disk(observed);
        let state = buffer.synchronization();
        if !matches!(state, SynchronizationState::Conflict { .. })
            && state.expected_disk() == observed
        {
            self.file_expectations.insert(buffer_id, exact);
        }
        Ok(state)
    }

    /// Atomically saves one exact buffer generation through `forge-project`.
    pub fn save(&mut self, buffer_id: BufferId) -> Result<EditorSaveResult, EditorWorkspaceError> {
        let intent = self
            .buffers
            .get_mut(buffer_id)
            .ok_or(EditorWorkspaceError::Buffer(BufferError::UnknownBuffer(
                buffer_id,
            )))?
            .prepare_save()?;
        let request = RepositoryPathRequest::new(
            intent.document().repository_id(),
            intent.document().relative_path().as_path(),
        )
        .map_err(EditorWorkspaceError::InvalidDocumentPath)?;
        let expected = self
            .file_expectations
            .get(&buffer_id)
            .copied()
            .ok_or(EditorWorkspaceError::MissingFileExpectation(buffer_id))?;
        if !expectation_matches_baseline(expected, intent.expected()) {
            self.buffers
                .get_mut(buffer_id)
                .expect("prepared buffer remains registered")
                .record_save_failure(intent.content_version(), SaveFailure::Io)?;
            return Err(EditorWorkspaceError::FileExpectationMismatch {
                buffer_id,
                editor: intent.expected(),
                file: expected,
            });
        }

        match self.files.write_atomic(&request, expected, intent.bytes()) {
            Ok(result) => {
                let disk = disk_version(result.revision());
                self.buffers
                    .get_mut(buffer_id)
                    .expect("prepared buffer remains registered")
                    .record_save_success(intent.content_version(), disk)?;
                self.file_expectations
                    .insert(buffer_id, FileExpectation::Exact(result.revision()));
                Ok(EditorSaveResult {
                    buffer_id,
                    content_version: intent.content_version(),
                    disk,
                    durability: result.durability(),
                })
            }
            Err(ProjectFileError::Conflict { found, .. }) => {
                let observed = disk_baseline(found);
                self.buffers
                    .get_mut(buffer_id)
                    .expect("prepared buffer remains registered")
                    .record_save_conflict(intent.content_version(), observed)?;
                Err(EditorWorkspaceError::SaveConflict {
                    buffer_id,
                    observed,
                })
            }
            Err(error) => {
                let failure = classify_save_failure(&error);
                self.buffers
                    .get_mut(buffer_id)
                    .expect("prepared buffer remains registered")
                    .record_save_failure(intent.content_version(), failure)?;
                Err(EditorWorkspaceError::File(error))
            }
        }
    }

    /// Closes only a clean buffer and removes its exact file precondition.
    pub fn close_clean(
        &mut self,
        buffer_id: BufferId,
    ) -> Result<EditorBuffer, EditorWorkspaceError> {
        let removed = self.buffers.remove_clean(buffer_id)?;
        self.file_expectations.remove(&buffer_id);
        Ok(removed)
    }

    /// Explicitly discards the confirmed local generation and closes the buffer.
    pub fn discard_and_close(
        &mut self,
        confirmation: DiscardConfirmation,
    ) -> Result<EditorBuffer, EditorWorkspaceError> {
        let removed = self.buffers.remove_discarding(confirmation)?;
        self.file_expectations.remove(&confirmation.buffer_id());
        Ok(removed)
    }

    /// Explicitly discards the confirmed local generation and reopens current disk bytes.
    pub fn discard_and_reopen(
        &mut self,
        confirmation: DiscardConfirmation,
    ) -> Result<OpenBufferResult, EditorWorkspaceError> {
        let request = self.request_for(confirmation.buffer_id())?;
        self.buffers.remove_discarding(confirmation)?;
        self.file_expectations.remove(&confirmation.buffer_id());
        self.open_or_create(confirmation.buffer_id(), &request)
    }

    fn refresh_exact_expectation(
        &mut self,
        result: OpenBufferResult,
        revision: FileRevision,
        observed: DiskVersion,
    ) {
        let buffer_id = opened_buffer_id(result);
        let buffer = self
            .buffers
            .get(buffer_id)
            .expect("opened buffer remains registered");
        if !matches!(
            buffer.synchronization(),
            SynchronizationState::Conflict { .. }
        ) && buffer.synchronization().expected_disk() == DiskBaseline::Existing(observed)
        {
            self.file_expectations
                .insert(buffer_id, FileExpectation::Exact(revision));
        }
    }

    fn request_for(
        &self,
        buffer_id: BufferId,
    ) -> Result<RepositoryPathRequest, EditorWorkspaceError> {
        let buffer = self
            .buffers
            .get(buffer_id)
            .ok_or(EditorWorkspaceError::Buffer(BufferError::UnknownBuffer(
                buffer_id,
            )))?;
        RepositoryPathRequest::new(
            buffer.document().repository_id(),
            buffer.document().relative_path().as_path(),
        )
        .map_err(EditorWorkspaceError::InvalidDocumentPath)
    }
}

/// Failure from the editor/project composition boundary.
#[derive(Debug)]
pub enum EditorWorkspaceError {
    Buffer(BufferError),
    File(ProjectFileError),
    InvalidDocumentPath(RepositoryPathError),
    MissingFileExpectation(BufferId),
    FileExpectationMismatch {
        buffer_id: BufferId,
        editor: DiskBaseline,
        file: FileExpectation,
    },
    SaveConflict {
        buffer_id: BufferId,
        observed: DiskBaseline,
    },
}

impl fmt::Display for EditorWorkspaceError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Buffer(error) => write!(formatter, "editor buffer error: {error}"),
            Self::File(error) => write!(formatter, "project file error: {error}"),
            Self::InvalidDocumentPath(error) => {
                write!(formatter, "editor document path is invalid: {error}")
            }
            Self::MissingFileExpectation(buffer_id) => {
                write!(
                    formatter,
                    "buffer {buffer_id} has no exact file precondition"
                )
            }
            Self::FileExpectationMismatch { buffer_id, .. } => write!(
                formatter,
                "buffer {buffer_id} editor baseline disagrees with its exact file precondition"
            ),
            Self::SaveConflict { buffer_id, .. } => {
                write!(
                    formatter,
                    "buffer {buffer_id} conflicts with current disk state"
                )
            }
        }
    }
}

impl std::error::Error for EditorWorkspaceError {}

impl From<BufferError> for EditorWorkspaceError {
    fn from(error: BufferError) -> Self {
        Self::Buffer(error)
    }
}

impl From<ProjectFileError> for EditorWorkspaceError {
    fn from(error: ProjectFileError) -> Self {
        Self::File(error)
    }
}

fn opened_buffer_id(result: OpenBufferResult) -> BufferId {
    match result {
        OpenBufferResult::Opened(buffer_id) | OpenBufferResult::Existing(buffer_id) => buffer_id,
    }
}

fn disk_version(revision: FileRevision) -> DiskVersion {
    DiskVersion::new(revision.content_hash(), revision.length())
}

fn disk_baseline(expectation: FileExpectation) -> DiskBaseline {
    match expectation {
        FileExpectation::Missing => DiskBaseline::Missing,
        FileExpectation::Exact(revision) => DiskBaseline::Existing(disk_version(revision)),
    }
}

fn expectation_matches_baseline(expectation: FileExpectation, baseline: DiskBaseline) -> bool {
    match (expectation, baseline) {
        (FileExpectation::Missing, DiskBaseline::Missing) => true,
        (FileExpectation::Exact(revision), DiskBaseline::Existing(disk)) => {
            disk_version(revision) == disk
        }
        _ => false,
    }
}

fn classify_save_failure(error: &ProjectFileError) -> SaveFailure {
    match error {
        ProjectFileError::RepositoryMismatch { .. }
        | ProjectFileError::PathNotAllowed { .. }
        | ProjectFileError::InvalidTarget { .. }
        | ProjectFileError::SymlinkRejected { .. }
        | ProjectFileError::ParentNotDirectory { .. }
        | ProjectFileError::NotRegularFile { .. }
        | ProjectFileError::Boundary(_) => SaveFailure::InvalidPath,
        ProjectFileError::Io { kind, .. } if *kind == std::io::ErrorKind::PermissionDenied => {
            SaveFailure::AccessDenied
        }
        ProjectFileError::InterruptedWrite { .. } => SaveFailure::DurabilityUncertain,
        ProjectFileError::UnsupportedPlatform
        | ProjectFileError::Missing { .. }
        | ProjectFileError::ParentIdentityChanged { .. }
        | ProjectFileError::ChangedDuringRead { .. }
        | ProjectFileError::FileTooLarge { .. }
        | ProjectFileError::Io { .. }
        | ProjectFileError::InjectedFailure
        | ProjectFileError::Conflict { .. } => SaveFailure::Io,
    }
}
