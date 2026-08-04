//! Atomic workspace recovery-image storage and explicit crash assessment.
//!
//! The adapter reuses `AtomicStateStore`; it never auto-restores, never replays
//! journaled actions, and refuses to replace a valid image without an exact
//! generation match.

use crate::persistence::{AtomicStateStore, StateStoreError};
use forge_core::recovery::{WorkspaceRecoveryError, WorkspaceRecoveryRecord};
use forge_core::state::StateRecord;
use forge_protocol::hashes::ContentHash;
use std::fmt;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

/// A decoded current recovery image plus visible interrupted publication state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OpenedWorkspaceRecovery {
    record: WorkspaceRecoveryRecord,
    interrupted_write_present: bool,
}

impl OpenedWorkspaceRecovery {
    pub const fn record(&self) -> &WorkspaceRecoveryRecord {
        &self.record
    }

    pub fn into_record(self) -> WorkspaceRecoveryRecord {
        self.record
    }

    pub const fn interrupted_write_present(&self) -> bool {
        self.interrupted_write_present
    }
}

/// Read-only status of one current or previous recovery image.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecoveryImageStatus {
    Missing,
    Valid {
        generation: u64,
        identity: ContentHash,
    },
    Invalid,
}

impl RecoveryImageStatus {
    pub const fn is_valid(self) -> bool {
        matches!(self, Self::Valid { .. })
    }
}

/// Explicit operator choices derived without modifying recovery data.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecoveryChoice {
    KeepCurrent,
    DiscardInterruptedWrite,
    RestorePrevious,
}

/// One non-mutating inspection of current, previous, and staged recovery state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecoveryAssessment {
    current: RecoveryImageStatus,
    previous: RecoveryImageStatus,
    interrupted_write_present: bool,
    choices: Vec<RecoveryChoice>,
}

impl RecoveryAssessment {
    pub const fn current(&self) -> RecoveryImageStatus {
        self.current
    }

    pub const fn previous(&self) -> RecoveryImageStatus {
        self.previous
    }

    pub const fn interrupted_write_present(&self) -> bool {
        self.interrupted_write_present
    }

    pub fn choices(&self) -> &[RecoveryChoice] {
        &self.choices
    }
}

/// Durable recovery-image store with generation-guarded replacement.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceRecoveryStore {
    store: AtomicStateStore,
}

impl WorkspaceRecoveryStore {
    pub fn new(target: impl AsRef<Path>) -> Result<Self, WorkspaceRecoveryStoreError> {
        Ok(Self {
            store: AtomicStateStore::new(target)?,
        })
    }

    pub fn target_path(&self) -> &Path {
        self.store.target_path()
    }

    pub fn staged_path(&self) -> &Path {
        self.store.staged_path()
    }

    pub fn previous_path(&self) -> &Path {
        self.store.previous_path()
    }

    /// Creates the first recovery image without overwriting existing data.
    pub fn create(
        &self,
        record: &WorkspaceRecoveryRecord,
    ) -> Result<(), WorkspaceRecoveryStoreError> {
        if record.generation() != 1 {
            return Err(WorkspaceRecoveryStoreError::InitialGenerationMustBeOne {
                found: record.generation(),
            });
        }
        self.store.create(&record.to_state_record()?)?;
        Ok(())
    }

    /// Reopens only a valid current workspace recovery image.
    pub fn open_current(&self) -> Result<OpenedWorkspaceRecovery, WorkspaceRecoveryStoreError> {
        let opened = self.store.open_current()?;
        let record = WorkspaceRecoveryRecord::from_state_record(opened.record())?;
        Ok(OpenedWorkspaceRecovery {
            record,
            interrupted_write_present: opened.interrupted_write_present(),
        })
    }

    /// Atomically publishes exactly the next generation after checking current state.
    pub fn publish_next(
        &self,
        expected_generation: u64,
        next: &WorkspaceRecoveryRecord,
    ) -> Result<(), WorkspaceRecoveryStoreError> {
        let expected_next = expected_generation
            .checked_add(1)
            .ok_or(WorkspaceRecoveryStoreError::GenerationOverflow)?;
        if next.generation() != expected_next {
            return Err(WorkspaceRecoveryStoreError::NonSequentialGeneration {
                expected: expected_next,
                found: next.generation(),
            });
        }
        let current = self.open_current()?;
        if current.interrupted_write_present() {
            return Err(WorkspaceRecoveryStoreError::InterruptedWriteRequiresResolution);
        }
        if current.record().project_id() != next.project_id() {
            return Err(WorkspaceRecoveryStoreError::ProjectMismatch);
        }
        if current.record().generation() != expected_generation {
            return Err(WorkspaceRecoveryStoreError::GenerationMismatch {
                expected: expected_generation,
                found: current.record().generation(),
            });
        }
        self.store.replace(&next.to_state_record()?)?;
        Ok(())
    }

    /// Inspects all visible recovery images without choosing or mutating one.
    pub fn assess(&self) -> Result<RecoveryAssessment, WorkspaceRecoveryStoreError> {
        let current = inspect_image(self.store.target_path())?;
        let previous = inspect_image(self.store.previous_path())?;
        let interrupted_write_present = inspect_staged(self.store.staged_path())?;
        let mut choices = Vec::new();
        match current {
            RecoveryImageStatus::Valid { .. } => {
                choices.push(RecoveryChoice::KeepCurrent);
                if interrupted_write_present {
                    choices.push(RecoveryChoice::DiscardInterruptedWrite);
                }
            }
            RecoveryImageStatus::Missing | RecoveryImageStatus::Invalid => {
                if previous.is_valid() {
                    choices.push(RecoveryChoice::RestorePrevious);
                }
            }
        }
        Ok(RecoveryAssessment {
            current,
            previous,
            interrupted_write_present,
            choices,
        })
    }

    /// Discards staging residue only when the current image is valid.
    pub fn discard_interrupted_write(&self) -> Result<bool, WorkspaceRecoveryStoreError> {
        let current = self.open_current()?;
        let _ = current;
        Ok(self.store.discard_interrupted_write()?)
    }

    /// Restores a validated previous image only when current data is unusable.
    pub fn restore_previous_if_current_unusable(
        &self,
    ) -> Result<WorkspaceRecoveryRecord, WorkspaceRecoveryStoreError> {
        let assessment = self.assess()?;
        if assessment.current().is_valid() {
            return Err(WorkspaceRecoveryStoreError::ValidCurrentWouldBeOverwritten);
        }
        if !assessment.previous().is_valid() {
            return Err(WorkspaceRecoveryStoreError::NoValidPreviousImage);
        }
        let restored = self.store.recover_previous()?;
        Ok(WorkspaceRecoveryRecord::from_state_record(&restored)?)
    }
}

fn inspect_image(path: &Path) -> Result<RecoveryImageStatus, WorkspaceRecoveryStoreError> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_symlink() => Err(
            WorkspaceRecoveryStoreError::SymlinkRejected(path.to_path_buf()),
        ),
        Ok(metadata) if !metadata.is_file() => {
            Err(WorkspaceRecoveryStoreError::NotFile(path.to_path_buf()))
        }
        Ok(_) => {
            let bytes = fs::read(path).map_err(|source| WorkspaceRecoveryStoreError::Io {
                path: path.to_path_buf(),
                kind: source.kind(),
            })?;
            let record = match StateRecord::decode(&bytes) {
                Ok(record) => record,
                Err(_) => return Ok(RecoveryImageStatus::Invalid),
            };
            let recovery = match WorkspaceRecoveryRecord::from_state_record(&record) {
                Ok(recovery) => recovery,
                Err(_) => return Ok(RecoveryImageStatus::Invalid),
            };
            Ok(RecoveryImageStatus::Valid {
                generation: recovery.generation(),
                identity: recovery.identity()?,
            })
        }
        Err(source) if source.kind() == io::ErrorKind::NotFound => Ok(RecoveryImageStatus::Missing),
        Err(source) => Err(WorkspaceRecoveryStoreError::Io {
            path: path.to_path_buf(),
            kind: source.kind(),
        }),
    }
}

fn inspect_staged(path: &Path) -> Result<bool, WorkspaceRecoveryStoreError> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_symlink() => Err(
            WorkspaceRecoveryStoreError::SymlinkRejected(path.to_path_buf()),
        ),
        Ok(metadata) if metadata.is_file() => Ok(true),
        Ok(_) => Err(WorkspaceRecoveryStoreError::NotFile(path.to_path_buf())),
        Err(source) if source.kind() == io::ErrorKind::NotFound => Ok(false),
        Err(source) => Err(WorkspaceRecoveryStoreError::Io {
            path: path.to_path_buf(),
            kind: source.kind(),
        }),
    }
}

/// Exact reason workspace recovery storage failed.
#[derive(Debug)]
pub enum WorkspaceRecoveryStoreError {
    StateStore(StateStoreError),
    Recovery(WorkspaceRecoveryError),
    GenerationOverflow,
    InitialGenerationMustBeOne { found: u64 },
    NonSequentialGeneration { expected: u64, found: u64 },
    InterruptedWriteRequiresResolution,
    ProjectMismatch,
    GenerationMismatch { expected: u64, found: u64 },
    ValidCurrentWouldBeOverwritten,
    NoValidPreviousImage,
    SymlinkRejected(PathBuf),
    NotFile(PathBuf),
    Io { path: PathBuf, kind: io::ErrorKind },
}

impl fmt::Display for WorkspaceRecoveryStoreError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::StateStore(source) => {
                write!(formatter, "workspace recovery storage failed: {source}")
            }
            Self::Recovery(source) => {
                write!(formatter, "workspace recovery image is invalid: {source}")
            }
            Self::GenerationOverflow => {
                formatter.write_str("workspace recovery generation overflow")
            }
            Self::InitialGenerationMustBeOne { found } => write!(
                formatter,
                "initial workspace recovery generation must be 1, found {found}"
            ),
            Self::NonSequentialGeneration { expected, found } => write!(
                formatter,
                "next recovery generation must be {expected}, found {found}"
            ),
            Self::GenerationMismatch { expected, found } => write!(
                formatter,
                "current recovery generation mismatch: expected {expected}, found {found}"
            ),
            Self::InterruptedWriteRequiresResolution => formatter.write_str(
                "interrupted recovery publication must be assessed before publishing again",
            ),
            Self::ProjectMismatch => {
                formatter.write_str("next recovery image belongs to a different project")
            }
            Self::ValidCurrentWouldBeOverwritten => {
                formatter.write_str("previous recovery image cannot replace a valid current image")
            }
            Self::NoValidPreviousImage => {
                formatter.write_str("no valid previous recovery image exists")
            }
            Self::SymlinkRejected(path) => {
                write!(formatter, "recovery path is a symlink: {}", path.display())
            }
            Self::NotFile(path) => write!(
                formatter,
                "recovery path is not a regular file: {}",
                path.display()
            ),
            Self::Io { path, kind } => write!(
                formatter,
                "failed to inspect recovery path {}: {kind:?}",
                path.display()
            ),
        }
    }
}

impl std::error::Error for WorkspaceRecoveryStoreError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::StateStore(source) => Some(source),
            Self::Recovery(source) => Some(source),
            _ => None,
        }
    }
}

impl From<StateStoreError> for WorkspaceRecoveryStoreError {
    fn from(source: StateStoreError) -> Self {
        Self::StateStore(source)
    }
}

impl From<WorkspaceRecoveryError> for WorkspaceRecoveryStoreError {
    fn from(source: WorkspaceRecoveryError) -> Self {
        Self::Recovery(source)
    }
}
