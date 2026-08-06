//! Atomic filesystem persistence for Forge Core state records.
//!
//! Forge Core owns canonical bytes and schema meaning. This adapter owns the
//! Linux filesystem effect: synced staging files, atomic replacement, explicit
//! legacy migration, interrupted-write visibility, and previous-state recovery.

use forge_core::state::{StateRecord, StateRecordError, migrate_legacy_v0};
use std::ffi::{OsStr, OsString};
use std::fmt;
use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};

const STAGED_SUFFIX: &str = ".forgeos-staged";
const PREVIOUS_SUFFIX: &str = ".forgeos-previous";
const PREVIOUS_STAGED_SUFFIX: &str = ".forgeos-previous-staged";

/// One decoded current record plus visible interrupted-write state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OpenedStateRecord {
    record: StateRecord,
    interrupted_write_present: bool,
}

impl OpenedStateRecord {
    pub fn record(&self) -> &StateRecord {
        &self.record
    }

    pub fn into_record(self) -> StateRecord {
        self.record
    }

    pub const fn interrupted_write_present(&self) -> bool {
        self.interrupted_write_present
    }
}

/// Atomic local store for one canonical Forge Core state record.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AtomicStateStore {
    target: PathBuf,
    staged: PathBuf,
    previous: PathBuf,
    previous_staged: PathBuf,
}

impl AtomicStateStore {
    /// Creates a store descriptor without touching the filesystem.
    pub fn new(target: impl AsRef<Path>) -> Result<Self, StateStoreError> {
        let target = target.as_ref().to_path_buf();
        let file_name = target
            .file_name()
            .ok_or_else(|| StateStoreError::InvalidTarget(target.clone()))?;
        let parent = target
            .parent()
            .ok_or_else(|| StateStoreError::InvalidTarget(target.clone()))?;
        if file_name.is_empty() {
            return Err(StateStoreError::InvalidTarget(target));
        }

        Ok(Self {
            staged: parent.join(companion_name(file_name, STAGED_SUFFIX)),
            previous: parent.join(companion_name(file_name, PREVIOUS_SUFFIX)),
            previous_staged: parent.join(companion_name(file_name, PREVIOUS_STAGED_SUFFIX)),
            target,
        })
    }

    pub fn target_path(&self) -> &Path {
        &self.target
    }

    pub fn staged_path(&self) -> &Path {
        &self.staged
    }

    pub fn previous_path(&self) -> &Path {
        &self.previous
    }

    /// Creates the first record. Existing state is never overwritten.
    pub fn create(&self, record: &StateRecord) -> Result<(), StateStoreError> {
        self.commit(record, CommitMode::Create, FaultPoint::None)
    }

    /// Atomically replaces a valid current record and retains its bytes as the
    /// explicit previous recovery record.
    pub fn replace(&self, record: &StateRecord) -> Result<(), StateStoreError> {
        self.commit(record, CommitMode::Replace, FaultPoint::None)
    }

    /// Reopens only the current schema. Unsupported, legacy, corrupt, or missing
    /// data is returned as an error and is never replaced with defaults.
    pub fn open_current(&self) -> Result<OpenedStateRecord, StateStoreError> {
        ensure_supported_platform()?;
        self.validate_parent()?;
        reject_symlink_if_present(&self.target)?;
        reject_symlink_if_present(&self.staged)?;

        let bytes = read_required(&self.target, StateOperation::ReadCurrent)?;
        let record = decode_at(&self.target, &bytes)?;
        Ok(OpenedStateRecord {
            record,
            interrupted_write_present: self.staged.exists(),
        })
    }

    /// Explicitly migrates the reviewed V0 fixture and commits current V1 bytes.
    /// No other version is guessed or accepted.
    pub fn migrate_legacy_v0(&self) -> Result<StateRecord, StateStoreError> {
        ensure_supported_platform()?;
        self.validate_parent()?;
        reject_symlink_if_present(&self.target)?;
        let bytes = read_required(&self.target, StateOperation::ReadCurrent)?;
        let migrated = migrate_legacy_v0(&bytes).map_err(|source| StateStoreError::State {
            path: self.target.clone(),
            source,
        })?;
        let record = migrated.into_record();
        self.commit(&record, CommitMode::ReplaceLegacy, FaultPoint::None)?;
        Ok(record)
    }

    /// Restores the last explicitly retained valid record over a corrupt or
    /// otherwise unwanted current record. Recovery is never automatic.
    pub fn recover_previous(&self) -> Result<StateRecord, StateStoreError> {
        ensure_supported_platform()?;
        self.validate_parent()?;
        reject_symlink_if_present(&self.previous)?;
        reject_symlink_if_present(&self.target)?;
        reject_symlink_if_present(&self.staged)?;

        let previous_bytes = read_required(&self.previous, StateOperation::ReadPrevious)?;
        let record = decode_at(&self.previous, &previous_bytes)?;
        remove_regular_if_present(&self.staged, StateOperation::DiscardInterrupted)?;
        prepare_staged(&self.staged, &previous_bytes)?;
        rename_replace(&self.staged, &self.target, StateOperation::RestorePrevious)?;
        sync_parent(parent_of(&self.target)?)?;
        Ok(record)
    }

    /// Removes a visible interrupted staging file only after the current target
    /// has been proven valid. Returns whether a staged file was removed.
    pub fn discard_interrupted_write(&self) -> Result<bool, StateStoreError> {
        let opened = self.open_current()?;
        if !opened.interrupted_write_present() {
            return Ok(false);
        }
        reject_symlink_if_present(&self.staged)?;
        fs::remove_file(&self.staged).map_err(|source| StateStoreError::Io {
            operation: StateOperation::DiscardInterrupted,
            path: self.staged.clone(),
            kind: source.kind(),
        })?;
        sync_parent(parent_of(&self.target)?)?;
        Ok(true)
    }

    fn commit(
        &self,
        record: &StateRecord,
        mode: CommitMode,
        fault: FaultPoint,
    ) -> Result<(), StateStoreError> {
        ensure_supported_platform()?;
        self.validate_parent()?;
        reject_symlink_if_present(&self.target)?;
        reject_symlink_if_present(&self.staged)?;
        reject_symlink_if_present(&self.previous)?;
        reject_symlink_if_present(&self.previous_staged)?;

        match mode {
            CommitMode::Create if self.target.exists() => {
                return Err(StateStoreError::AlreadyExists(self.target.clone()));
            }
            CommitMode::Create if self.previous.exists() => {
                return Err(StateStoreError::PreviousStatePresent(self.previous.clone()));
            }
            CommitMode::Replace => {
                let current = read_required(&self.target, StateOperation::ReadCurrent)?;
                decode_at(&self.target, &current)?;
            }
            CommitMode::ReplaceLegacy => {
                if !self.target.exists() {
                    return Err(StateStoreError::Missing(self.target.clone()));
                }
            }
            CommitMode::Create => {}
        }

        remove_regular_if_present(&self.staged, StateOperation::DiscardInterrupted)?;
        remove_regular_if_present(&self.previous_staged, StateOperation::DiscardPreviousStaged)?;
        let encoded = record.encode();
        prepare_staged(&self.staged, &encoded)?;
        fault.trigger(FaultPoint::AfterStagedSync)?;

        if matches!(mode, CommitMode::Replace | CommitMode::ReplaceLegacy) {
            let current = read_required(&self.target, StateOperation::ReadCurrent)?;
            match mode {
                CommitMode::Replace => {
                    decode_at(&self.target, &current)?;
                }
                CommitMode::ReplaceLegacy => {
                    migrate_legacy_v0(&current).map_err(|source| StateStoreError::State {
                        path: self.target.clone(),
                        source,
                    })?;
                }
                CommitMode::Create => unreachable!("create mode has no previous state"),
            }
            prepare_staged(&self.previous_staged, &current)?;
            rename_replace(
                &self.previous_staged,
                &self.previous,
                StateOperation::PublishPrevious,
            )?;
            sync_parent(parent_of(&self.target)?)?;
            fault.trigger(FaultPoint::AfterPreviousSync)?;
        }

        rename_replace(&self.staged, &self.target, StateOperation::PublishCurrent)?;
        sync_parent(parent_of(&self.target)?)?;
        Ok(())
    }

    fn validate_parent(&self) -> Result<(), StateStoreError> {
        let parent = parent_of(&self.target)?;
        let metadata = fs::symlink_metadata(parent).map_err(|source| StateStoreError::Io {
            operation: StateOperation::InspectParent,
            path: parent.to_path_buf(),
            kind: source.kind(),
        })?;
        if metadata.file_type().is_symlink() {
            return Err(StateStoreError::SymlinkRejected(parent.to_path_buf()));
        }
        if !metadata.is_dir() {
            return Err(StateStoreError::ParentNotDirectory(parent.to_path_buf()));
        }
        Ok(())
    }

    #[cfg(test)]
    fn replace_with_fault(
        &self,
        record: &StateRecord,
        fault: FaultPoint,
    ) -> Result<(), StateStoreError> {
        self.commit(record, CommitMode::Replace, fault)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CommitMode {
    Create,
    Replace,
    ReplaceLegacy,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FaultPoint {
    None,
    AfterStagedSync,
    AfterPreviousSync,
}

impl FaultPoint {
    fn trigger(self, reached: Self) -> Result<(), StateStoreError> {
        if self == reached {
            Err(StateStoreError::InjectedFailure(reached.label()))
        } else {
            Ok(())
        }
    }

    const fn label(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::AfterStagedSync => "after staged state sync",
            Self::AfterPreviousSync => "after previous state sync",
        }
    }
}

fn prepare_staged(path: &Path, bytes: &[u8]) -> Result<(), StateStoreError> {
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|source| StateStoreError::Io {
            operation: StateOperation::CreateStaged,
            path: path.to_path_buf(),
            kind: source.kind(),
        })?;
    file.write_all(bytes)
        .map_err(|source| StateStoreError::Io {
            operation: StateOperation::WriteStaged,
            path: path.to_path_buf(),
            kind: source.kind(),
        })?;
    file.sync_all().map_err(|source| StateStoreError::Io {
        operation: StateOperation::SyncStaged,
        path: path.to_path_buf(),
        kind: source.kind(),
    })
}

fn rename_replace(
    from: &Path,
    to: &Path,
    operation: StateOperation,
) -> Result<(), StateStoreError> {
    fs::rename(from, to).map_err(|source| StateStoreError::Io {
        operation,
        path: to.to_path_buf(),
        kind: source.kind(),
    })
}

fn sync_parent(parent: &Path) -> Result<(), StateStoreError> {
    File::open(parent)
        .and_then(|directory| directory.sync_all())
        .map_err(|source| StateStoreError::Io {
            operation: StateOperation::SyncParent,
            path: parent.to_path_buf(),
            kind: source.kind(),
        })
}

fn read_required(path: &Path, operation: StateOperation) -> Result<Vec<u8>, StateStoreError> {
    fs::read(path).map_err(|source| {
        if source.kind() == io::ErrorKind::NotFound {
            StateStoreError::Missing(path.to_path_buf())
        } else {
            StateStoreError::Io {
                operation,
                path: path.to_path_buf(),
                kind: source.kind(),
            }
        }
    })
}

fn decode_at(path: &Path, bytes: &[u8]) -> Result<StateRecord, StateStoreError> {
    StateRecord::decode(bytes).map_err(|source| StateStoreError::State {
        path: path.to_path_buf(),
        source,
    })
}

fn reject_symlink_if_present(path: &Path) -> Result<(), StateStoreError> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_symlink() => {
            Err(StateStoreError::SymlinkRejected(path.to_path_buf()))
        }
        Ok(_) => Ok(()),
        Err(source) if source.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(source) => Err(StateStoreError::Io {
            operation: StateOperation::InspectPath,
            path: path.to_path_buf(),
            kind: source.kind(),
        }),
    }
}

fn remove_regular_if_present(
    path: &Path,
    operation: StateOperation,
) -> Result<(), StateStoreError> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_symlink() => {
            Err(StateStoreError::SymlinkRejected(path.to_path_buf()))
        }
        Ok(metadata) if metadata.is_file() => {
            fs::remove_file(path).map_err(|source| StateStoreError::Io {
                operation,
                path: path.to_path_buf(),
                kind: source.kind(),
            })
        }
        Ok(_) => Err(StateStoreError::CompanionNotFile(path.to_path_buf())),
        Err(source) if source.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(source) => Err(StateStoreError::Io {
            operation: StateOperation::InspectPath,
            path: path.to_path_buf(),
            kind: source.kind(),
        }),
    }
}

fn parent_of(path: &Path) -> Result<&Path, StateStoreError> {
    match path.parent() {
        Some(parent) if parent.as_os_str().is_empty() => Ok(Path::new(".")),
        Some(parent) => Ok(parent),
        None => Err(StateStoreError::InvalidTarget(path.to_path_buf())),
    }
}

fn companion_name(file_name: &OsStr, suffix: &str) -> OsString {
    let mut name = file_name.to_os_string();
    name.push(suffix);
    name
}

fn ensure_supported_platform() -> Result<(), StateStoreError> {
    if cfg!(unix) {
        Ok(())
    } else {
        Err(StateStoreError::UnsupportedPlatform)
    }
}

/// Filesystem operation attached to one typed I/O failure.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StateOperation {
    InspectParent,
    InspectPath,
    ReadCurrent,
    ReadPrevious,
    CreateStaged,
    WriteStaged,
    SyncStaged,
    PublishPrevious,
    PublishCurrent,
    RestorePrevious,
    SyncParent,
    DiscardInterrupted,
    DiscardPreviousStaged,
}

impl StateOperation {
    const fn label(self) -> &'static str {
        match self {
            Self::InspectParent => "inspect state parent",
            Self::InspectPath => "inspect state path",
            Self::ReadCurrent => "read current state",
            Self::ReadPrevious => "read previous state",
            Self::CreateStaged => "create staged state",
            Self::WriteStaged => "write staged state",
            Self::SyncStaged => "sync staged state",
            Self::PublishPrevious => "publish previous state",
            Self::PublishCurrent => "publish current state",
            Self::RestorePrevious => "restore previous state",
            Self::SyncParent => "sync state parent",
            Self::DiscardInterrupted => "discard interrupted state",
            Self::DiscardPreviousStaged => "discard interrupted previous state",
        }
    }
}

/// Exact reason local state persistence failed.
#[derive(Debug)]
pub enum StateStoreError {
    UnsupportedPlatform,
    InvalidTarget(PathBuf),
    ParentNotDirectory(PathBuf),
    CompanionNotFile(PathBuf),
    SymlinkRejected(PathBuf),
    AlreadyExists(PathBuf),
    PreviousStatePresent(PathBuf),
    Missing(PathBuf),
    State {
        path: PathBuf,
        source: StateRecordError,
    },
    Io {
        operation: StateOperation,
        path: PathBuf,
        kind: io::ErrorKind,
    },
    InjectedFailure(&'static str),
}

impl fmt::Display for StateStoreError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedPlatform => {
                write!(
                    formatter,
                    "atomic V1 state persistence requires Unix rename semantics"
                )
            }
            Self::InvalidTarget(path) => {
                write!(formatter, "state target is invalid: {}", path.display())
            }
            Self::ParentNotDirectory(path) => {
                write!(
                    formatter,
                    "state parent is not a directory: {}",
                    path.display()
                )
            }
            Self::CompanionNotFile(path) => {
                write!(
                    formatter,
                    "state companion is not a regular file: {}",
                    path.display()
                )
            }
            Self::SymlinkRejected(path) => {
                write!(formatter, "state path is a symlink: {}", path.display())
            }
            Self::AlreadyExists(path) => {
                write!(formatter, "state already exists: {}", path.display())
            }
            Self::PreviousStatePresent(path) => write!(
                formatter,
                "previous state exists and must be recovered or removed explicitly: {}",
                path.display()
            ),
            Self::Missing(path) => write!(formatter, "state is missing: {}", path.display()),
            Self::State { path, source } => {
                write!(
                    formatter,
                    "state bytes are invalid at {}: {source}",
                    path.display()
                )
            }
            Self::Io {
                operation,
                path,
                kind,
            } => write!(
                formatter,
                "failed to {} at {}: {kind:?}",
                operation.label(),
                path.display()
            ),
            Self::InjectedFailure(point) => {
                write!(formatter, "injected persistence failure {point}")
            }
        }
    }
}

impl std::error::Error for StateStoreError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::State { source, .. } => Some(source),
            _ => None,
        }
    }
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use forge_core::state::encode_legacy_v0_fixture;
    use std::sync::atomic::{AtomicU64, Ordering};

    static FIXTURE_COUNTER: AtomicU64 = AtomicU64::new(0);

    struct Fixture {
        root: PathBuf,
    }

    impl Fixture {
        fn new(label: &str) -> Self {
            let sequence = FIXTURE_COUNTER.fetch_add(1, Ordering::Relaxed);
            let root = std::env::temp_dir().join(format!(
                "forgeos-state-{label}-{}-{sequence}",
                std::process::id()
            ));
            if root.exists() {
                fs::remove_dir_all(&root).expect("remove stale fixture");
            }
            fs::create_dir_all(&root).expect("create fixture root");
            Self { root }
        }

        fn store(&self) -> AtomicStateStore {
            AtomicStateStore::new(self.root.join("forge.state")).unwrap()
        }
    }

    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.root);
        }
    }

    fn record(payload: &[u8]) -> StateRecord {
        StateRecord::new(1, payload.to_vec()).unwrap()
    }

    #[test]
    fn create_and_reopen_are_equivalent() {
        let fixture = Fixture::new("create");
        let store = fixture.store();
        let expected = record(b"first");

        store.create(&expected).unwrap();
        let opened = store.open_current().unwrap();

        assert_eq!(opened.record(), &expected);
        assert!(!opened.interrupted_write_present());
        assert_eq!(fs::read(store.target_path()).unwrap(), expected.encode());
    }

    #[test]
    fn replace_is_atomic_and_retains_previous_valid_bytes() {
        let fixture = Fixture::new("replace");
        let store = fixture.store();
        let first = record(b"first");
        let second = record(b"second");

        store.create(&first).unwrap();
        store.replace(&second).unwrap();

        assert_eq!(store.open_current().unwrap().record(), &second);
        assert_eq!(
            StateRecord::decode(&fs::read(store.previous_path()).unwrap()).unwrap(),
            first
        );
    }

    #[test]
    fn legacy_migration_is_explicit_and_rewrites_current_schema() {
        let fixture = Fixture::new("migration");
        let store = fixture.store();
        fs::write(
            store.target_path(),
            encode_legacy_v0_fixture(2, b"legacy").unwrap(),
        )
        .unwrap();

        assert!(matches!(
            store.open_current(),
            Err(StateStoreError::State {
                source: StateRecordError::MigrationRequired { .. },
                ..
            })
        ));

        let migrated = store.migrate_legacy_v0().unwrap();
        assert_eq!(migrated.record_type(), 2);
        assert_eq!(migrated.payload(), b"legacy");
        assert_eq!(store.open_current().unwrap().record(), &migrated);
    }

    #[test]
    fn corrupt_and_unknown_schema_records_are_rejected_without_defaults() {
        let fixture = Fixture::new("corrupt");
        let store = fixture.store();
        let valid = record(b"valid");
        store.create(&valid).unwrap();

        let mut corrupt = valid.encode();
        corrupt[18] ^= 0x20;
        fs::write(store.target_path(), corrupt).unwrap();
        assert!(matches!(
            store.open_current(),
            Err(StateStoreError::State {
                source: StateRecordError::ChecksumMismatch { .. },
                ..
            })
        ));

        let mut future = valid.encode();
        future[8..10].copy_from_slice(&77u16.to_be_bytes());
        fs::write(store.target_path(), future).unwrap();
        assert!(matches!(
            store.open_current(),
            Err(StateStoreError::State {
                source: StateRecordError::UnsupportedSchemaVersion { found: 77 },
                ..
            })
        ));
    }

    #[test]
    fn interrupted_stage_is_visible_and_discarded_only_after_valid_reopen() {
        let fixture = Fixture::new("interrupted");
        let store = fixture.store();
        let stable = record(b"stable");
        let uncommitted = record(b"uncommitted");
        store.create(&stable).unwrap();
        fs::write(store.staged_path(), uncommitted.encode()).unwrap();

        let opened = store.open_current().unwrap();
        assert_eq!(opened.record(), &stable);
        assert!(opened.interrupted_write_present());
        assert!(store.discard_interrupted_write().unwrap());
        assert!(!store.staged_path().exists());
        assert_eq!(store.open_current().unwrap().record(), &stable);
    }

    #[test]
    fn injected_failures_preserve_the_previous_valid_current_record() {
        let fixture = Fixture::new("failure");
        let store = fixture.store();
        let first = record(b"first");
        let second = record(b"second");
        store.create(&first).unwrap();

        assert!(matches!(
            store.replace_with_fault(&second, FaultPoint::AfterStagedSync),
            Err(StateStoreError::InjectedFailure(_))
        ));
        assert_eq!(store.open_current().unwrap().record(), &first);
        store.discard_interrupted_write().unwrap();

        assert!(matches!(
            store.replace_with_fault(&second, FaultPoint::AfterPreviousSync),
            Err(StateStoreError::InjectedFailure(_))
        ));
        assert_eq!(store.open_current().unwrap().record(), &first);
        assert_eq!(
            StateRecord::decode(&fs::read(store.previous_path()).unwrap()).unwrap(),
            first
        );
    }

    #[test]
    fn explicit_recovery_restores_previous_after_current_corruption() {
        let fixture = Fixture::new("recovery");
        let store = fixture.store();
        let first = record(b"recover-me");
        let second = record(b"newer");
        store.create(&first).unwrap();
        store.replace(&second).unwrap();

        fs::write(store.target_path(), b"broken-current").unwrap();
        assert!(store.open_current().is_err());

        let recovered = store.recover_previous().unwrap();
        assert_eq!(recovered, first);
        assert_eq!(store.open_current().unwrap().record(), &first);
    }

    #[test]
    fn missing_current_can_recover_previous_and_create_does_not_hide_it() {
        let fixture = Fixture::new("missing-recovery");
        let store = fixture.store();
        let first = record(b"first");
        let second = record(b"second");
        store.create(&first).unwrap();
        store.replace(&second).unwrap();
        fs::remove_file(store.target_path()).unwrap();

        assert!(matches!(
            store.create(&record(b"unrelated")),
            Err(StateStoreError::PreviousStatePresent(_))
        ));
        assert_eq!(store.recover_previous().unwrap(), first);
        assert_eq!(store.open_current().unwrap().record(), &first);
    }

    #[test]
    fn symlink_target_is_rejected() {
        use std::os::unix::fs::symlink;

        let fixture = Fixture::new("symlink");
        let real = fixture.root.join("real.state");
        fs::write(&real, record(b"real").encode()).unwrap();
        let target = fixture.root.join("link.state");
        symlink(&real, &target).unwrap();
        let store = AtomicStateStore::new(target).unwrap();

        assert!(matches!(
            store.open_current(),
            Err(StateStoreError::SymlinkRejected(_))
        ));
    }
}
