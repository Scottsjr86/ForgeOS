//! Stable patch validation and all-or-nothing worktree application.
//!
//! The patch envelope owns identity and declared metadata. This module verifies
//! the exact native repository base, parses a deliberately narrow text-patch
//! surface, pins every touched file, runs Git's non-rejecting dry-run and apply
//! paths, verifies the declared result, and restores all touched files if apply
//! or postcondition verification fails.

use crate::repository::{GitInspectError, GitRepositoryInspector};
use crate::types::GitObjectId;
use forge_bridge::patch::{
    NativePatchAdapter, NativePatchInvocationError, NativePatchOperation, NativePatchOutput,
};
use forge_protocol::hashes::{ContentHash, HashDomain, hash_canonical_bytes};
use forge_protocol::identities::{PatchId, RepositoryId};
use forge_protocol::patches::{PatchEnvelope, PatchFileAction, PatchFileRecord};
use forge_protocol::paths::RepositoryRelativePath;
use std::ffi::OsStr;
use std::fmt;
use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::{Component, Path, PathBuf};

#[cfg(unix)]
use std::os::unix::fs::{MetadataExt, PermissionsExt};

const APPLY_LOCK_NAME: &str = ".forgeos-patch-apply.lock";
const MAX_PATCH_FILES: usize = 4096;

/// Successful validation before any worktree mutation occurs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PatchValidationResult {
    patch_id: PatchId,
    repository_id: RepositoryId,
    base_revision: GitObjectId,
    patch_identity: ContentHash,
    payload_hash: ContentHash,
    files: Vec<PatchFileRecord>,
}

impl PatchValidationResult {
    pub const fn patch_id(&self) -> PatchId {
        self.patch_id
    }

    pub const fn repository_id(&self) -> RepositoryId {
        self.repository_id
    }

    pub fn base_revision(&self) -> &GitObjectId {
        &self.base_revision
    }

    pub const fn patch_identity(&self) -> ContentHash {
        self.patch_identity
    }

    pub const fn payload_hash(&self) -> ContentHash {
        self.payload_hash
    }

    pub fn files(&self) -> &[PatchFileRecord] {
        &self.files
    }
}

/// Completed worktree application with exact native outputs retained.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PatchApplyOutcome {
    validation: PatchValidationResult,
    check_output: NativePatchOutput,
    apply_output: NativePatchOutput,
}

impl PatchApplyOutcome {
    pub fn validation(&self) -> &PatchValidationResult {
        &self.validation
    }

    pub fn check_output(&self) -> &NativePatchOutput {
        &self.check_output
    }

    pub fn apply_output(&self) -> &NativePatchOutput {
        &self.apply_output
    }
}

/// Exact failure class for validation or application.
#[derive(Debug)]
pub enum PatchApplyError {
    UnsupportedPlatform,
    Inspect(GitInspectError),
    RepositoryMismatch {
        expected: RepositoryId,
        actual: RepositoryId,
    },
    UnbornRepository,
    BaseRevisionChanged {
        expected: String,
        actual: String,
    },
    TooManyFiles {
        maximum: usize,
        actual: usize,
    },
    EmptyFileTable,
    MalformedPatch(String),
    HiddenBinaryPatch,
    FileTableMismatch {
        message: String,
    },
    PathIo {
        operation: &'static str,
        path: PathBuf,
        kind: io::ErrorKind,
        message: String,
    },
    PathSymlink(PathBuf),
    PathNotRegular(PathBuf),
    MissingParent(PathBuf),
    FileStateMismatch {
        path: PathBuf,
        expected: Option<ContentHash>,
        actual: Option<ContentHash>,
    },
    FileMetadataChanged(PathBuf),
    ApplyAlreadyInProgress(PathBuf),
    NativeInvocation(NativePatchInvocationError),
    NativeApplyInvocationFailed {
        error: NativePatchInvocationError,
        rolled_back: bool,
    },
    NativeCheckFailed {
        exit_code: Option<i32>,
        signal: Option<i32>,
        stderr: Vec<u8>,
    },
    NativeApplyFailed {
        exit_code: Option<i32>,
        signal: Option<i32>,
        stderr: Vec<u8>,
        rolled_back: bool,
    },
    PostApplyVerificationFailed {
        message: String,
        rolled_back: bool,
    },
    RollbackFailed {
        original: String,
        rollback: String,
    },
}

impl fmt::Display for PatchApplyError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedPlatform => {
                formatter.write_str("patch application requires Unix filesystem semantics")
            }
            Self::Inspect(error) => error.fmt(formatter),
            Self::RepositoryMismatch { expected, actual } => write!(
                formatter,
                "patch repository mismatch: expected {expected}, got {actual}"
            ),
            Self::UnbornRepository => {
                formatter.write_str("patch base validation requires a committed repository")
            }
            Self::BaseRevisionChanged { expected, actual } => write!(
                formatter,
                "patch base revision changed: expected {expected}, got {actual}"
            ),
            Self::TooManyFiles { maximum, actual } => write!(
                formatter,
                "patch file table exceeds {maximum} files; found {actual}"
            ),
            Self::EmptyFileTable => formatter.write_str("patch file table may not be empty"),
            Self::MalformedPatch(message) => write!(formatter, "patch is malformed: {message}"),
            Self::HiddenBinaryPatch => formatter.write_str(
                "binary or NUL-bearing patches require a later reviewed binary-patch contract",
            ),
            Self::FileTableMismatch { message } => {
                write!(
                    formatter,
                    "patch file table does not match payload: {message}"
                )
            }
            Self::PathIo {
                operation,
                path,
                message,
                ..
            } => write!(
                formatter,
                "failed to {operation} {}: {message}",
                path.display()
            ),
            Self::PathSymlink(path) => {
                write!(
                    formatter,
                    "patch path may not traverse a symlink: {}",
                    path.display()
                )
            }
            Self::PathNotRegular(path) => write!(
                formatter,
                "patch path is not a regular file: {}",
                path.display()
            ),
            Self::MissingParent(path) => write!(
                formatter,
                "patch parent directory must already exist: {}",
                path.display()
            ),
            Self::FileStateMismatch {
                path,
                expected,
                actual,
            } => write!(
                formatter,
                "patch file state changed for {}: expected {expected:?}, got {actual:?}",
                path.display()
            ),
            Self::FileMetadataChanged(path) => write!(
                formatter,
                "patch file metadata changed after validation for {}",
                path.display()
            ),
            Self::ApplyAlreadyInProgress(path) => write!(
                formatter,
                "another ForgeOS patch apply owns {}",
                path.display()
            ),
            Self::NativeInvocation(error) => error.fmt(formatter),
            Self::NativeApplyInvocationFailed { error, rolled_back } => write!(
                formatter,
                "native Git patch apply invocation failed: {error}; rolled_back={rolled_back}"
            ),
            Self::NativeCheckFailed {
                exit_code, signal, ..
            } => write!(
                formatter,
                "native Git patch check failed with code {exit_code:?} signal {signal:?}"
            ),
            Self::NativeApplyFailed {
                exit_code,
                signal,
                rolled_back,
                ..
            } => write!(
                formatter,
                "native Git patch apply failed with code {exit_code:?} signal {signal:?}; rolled_back={rolled_back}"
            ),
            Self::PostApplyVerificationFailed {
                message,
                rolled_back,
            } => write!(
                formatter,
                "patch postcondition failed: {message}; rolled_back={rolled_back}"
            ),
            Self::RollbackFailed { original, rollback } => write!(
                formatter,
                "patch failed ({original}) and rollback also failed ({rollback})"
            ),
        }
    }
}

impl std::error::Error for PatchApplyError {}

impl From<GitInspectError> for PatchApplyError {
    fn from(error: GitInspectError) -> Self {
        Self::Inspect(error)
    }
}

impl From<NativePatchInvocationError> for PatchApplyError {
    fn from(error: NativePatchInvocationError) -> Self {
        Self::NativeInvocation(error)
    }
}

/// One repository-bound patch validator and applier.
#[derive(Debug, Clone)]
pub struct GitPatchApplier {
    inspector: GitRepositoryInspector,
    adapter: NativePatchAdapter,
}

impl GitPatchApplier {
    pub fn from_inspector(inspector: GitRepositoryInspector) -> Self {
        Self {
            inspector,
            adapter: NativePatchAdapter::default(),
        }
    }

    pub fn with_program(inspector: GitRepositoryInspector, program: impl AsRef<OsStr>) -> Self {
        Self {
            inspector,
            adapter: NativePatchAdapter::with_program(program),
        }
    }

    /// Validates identity, base, payload structure, file table, and current files.
    pub fn validate(
        &self,
        envelope: &PatchEnvelope,
    ) -> Result<PatchValidationResult, PatchApplyError> {
        let prepared = self.prepare(envelope)?;
        let _lock = ApplyLock::acquire(self.inspector.root())?;
        revalidate_snapshots(&prepared.snapshots)?;
        let output = self.adapter.invoke(
            self.inspector.root(),
            NativePatchOperation::Check,
            envelope.bytes(),
        )?;
        if !output.exit().success() {
            return Err(PatchApplyError::NativeCheckFailed {
                exit_code: output.exit().code(),
                signal: output.exit().signal(),
                stderr: output.stderr().to_vec(),
            });
        }
        revalidate_snapshots(&prepared.snapshots)?;
        Ok(prepared.validation)
    }

    /// Applies one validated patch or restores every touched file before returning failure.
    pub fn apply(&self, envelope: &PatchEnvelope) -> Result<PatchApplyOutcome, PatchApplyError> {
        let prepared = self.prepare(envelope)?;
        let _lock = ApplyLock::acquire(self.inspector.root())?;
        revalidate_snapshots(&prepared.snapshots)?;

        let check_output = self.adapter.invoke(
            self.inspector.root(),
            NativePatchOperation::Check,
            envelope.bytes(),
        )?;
        if !check_output.exit().success() {
            return Err(PatchApplyError::NativeCheckFailed {
                exit_code: check_output.exit().code(),
                signal: check_output.exit().signal(),
                stderr: check_output.stderr().to_vec(),
            });
        }

        revalidate_snapshots(&prepared.snapshots)?;
        let apply_output = match self.adapter.invoke(
            self.inspector.root(),
            NativePatchOperation::Apply,
            envelope.bytes(),
        ) {
            Ok(output) => output,
            Err(error) => {
                let original = error.to_string();
                return match rollback(&prepared.snapshots) {
                    Ok(()) => Err(PatchApplyError::NativeApplyInvocationFailed {
                        error,
                        rolled_back: true,
                    }),
                    Err(rollback) => Err(PatchApplyError::RollbackFailed { original, rollback }),
                };
            }
        };
        if !apply_output.exit().success() {
            let original = format!(
                "native apply code {:?} signal {:?}",
                apply_output.exit().code(),
                apply_output.exit().signal()
            );
            return match rollback(&prepared.snapshots) {
                Ok(()) => Err(PatchApplyError::NativeApplyFailed {
                    exit_code: apply_output.exit().code(),
                    signal: apply_output.exit().signal(),
                    stderr: apply_output.stderr().to_vec(),
                    rolled_back: true,
                }),
                Err(rollback) => Err(PatchApplyError::RollbackFailed { original, rollback }),
            };
        }

        if let Err(error) = verify_after(envelope.files(), self.inspector.root()) {
            let original = error.to_string();
            return match rollback(&prepared.snapshots) {
                Ok(()) => Err(PatchApplyError::PostApplyVerificationFailed {
                    message: original,
                    rolled_back: true,
                }),
                Err(rollback) => Err(PatchApplyError::RollbackFailed { original, rollback }),
            };
        }

        Ok(PatchApplyOutcome {
            validation: prepared.validation,
            check_output,
            apply_output,
        })
    }

    fn prepare(&self, envelope: &PatchEnvelope) -> Result<PreparedPatch, PatchApplyError> {
        if envelope.repository_id() != self.inspector.repository_id() {
            return Err(PatchApplyError::RepositoryMismatch {
                expected: self.inspector.repository_id(),
                actual: envelope.repository_id(),
            });
        }
        if envelope.files().is_empty() {
            return Err(PatchApplyError::EmptyFileTable);
        }
        if envelope.files().len() > MAX_PATCH_FILES {
            return Err(PatchApplyError::TooManyFiles {
                maximum: MAX_PATCH_FILES,
                actual: envelope.files().len(),
            });
        }
        if envelope
            .files()
            .iter()
            .any(|file| file.path().as_path() == Path::new(APPLY_LOCK_NAME))
        {
            return Err(PatchApplyError::MalformedPatch(
                "patch may not target the ForgeOS apply lock".to_owned(),
            ));
        }
        let head = self.inspector.inspect_head()?;
        let actual = head
            .revision()
            .ok_or(PatchApplyError::UnbornRepository)?
            .clone();
        if actual.as_str() != envelope.base_revision().as_str() {
            return Err(PatchApplyError::BaseRevisionChanged {
                expected: envelope.base_revision().as_str().to_owned(),
                actual: actual.as_str().to_owned(),
            });
        }

        crate::patch_format::validate_file_table(envelope.files(), envelope.bytes())?;
        let snapshots = snapshot_before(envelope.files(), self.inspector.root())?;
        Ok(PreparedPatch {
            validation: PatchValidationResult {
                patch_id: envelope.patch_id(),
                repository_id: envelope.repository_id(),
                base_revision: actual,
                patch_identity: envelope.identity(),
                payload_hash: envelope.payload_hash(),
                files: envelope.files().to_vec(),
            },
            snapshots,
        })
    }
}

#[derive(Debug)]
struct PreparedPatch {
    validation: PatchValidationResult,
    snapshots: Vec<FileSnapshot>,
}

#[derive(Debug, Clone)]
struct FileSnapshot {
    absolute: PathBuf,
    expected_before: Option<ContentHash>,
    original: OriginalFile,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum OriginalFile {
    Missing,
    Regular { bytes: Vec<u8>, mode: u32 },
}

fn snapshot_before(
    files: &[PatchFileRecord],
    root: &Path,
) -> Result<Vec<FileSnapshot>, PatchApplyError> {
    let mut snapshots = Vec::with_capacity(files.len());
    for file in files {
        let absolute = secure_path(root, file.path(), file.action() == PatchFileAction::Add)?;
        let original = read_original(&absolute)?;
        let actual = original_hash(&original);
        if actual != file.before() {
            return Err(PatchApplyError::FileStateMismatch {
                path: file.path().as_path().to_path_buf(),
                expected: file.before(),
                actual,
            });
        }
        snapshots.push(FileSnapshot {
            absolute,
            expected_before: file.before(),
            original,
        });
    }
    Ok(snapshots)
}

fn secure_path(
    root: &Path,
    relative: &RepositoryRelativePath,
    allow_missing_final: bool,
) -> Result<PathBuf, PatchApplyError> {
    #[cfg(not(unix))]
    {
        let _ = (root, relative, allow_missing_final);
        return Err(PatchApplyError::UnsupportedPlatform);
    }
    #[cfg(unix)]
    {
        let mut current = root.to_path_buf();
        let components: Vec<_> = relative.as_path().components().collect();
        for (index, component) in components.iter().enumerate() {
            let Component::Normal(value) = component else {
                return Err(PatchApplyError::MalformedPatch(
                    "patch path is not lexically canonical".to_owned(),
                ));
            };
            current.push(value);
            let final_component = index + 1 == components.len();
            match fs::symlink_metadata(&current) {
                Ok(metadata) => {
                    if metadata.file_type().is_symlink() {
                        return Err(PatchApplyError::PathSymlink(current));
                    }
                    if !final_component && !metadata.is_dir() {
                        return Err(PatchApplyError::PathNotRegular(current));
                    }
                }
                Err(error) if error.kind() == io::ErrorKind::NotFound => {
                    if final_component && allow_missing_final {
                        let parent = current.parent().expect("relative path has a parent");
                        let parent_metadata = fs::symlink_metadata(parent)
                            .map_err(|error| path_io("inspect patch parent", parent, error))?;
                        if parent_metadata.file_type().is_symlink() || !parent_metadata.is_dir() {
                            return Err(PatchApplyError::MissingParent(parent.to_path_buf()));
                        }
                    } else {
                        return Err(PatchApplyError::PathIo {
                            operation: "inspect patch path",
                            path: current,
                            kind: error.kind(),
                            message: error.to_string(),
                        });
                    }
                }
                Err(error) => return Err(path_io("inspect patch path", &current, error)),
            }
        }
        Ok(current)
    }
}

fn read_original(path: &Path) -> Result<OriginalFile, PatchApplyError> {
    #[cfg(not(unix))]
    {
        let _ = path;
        return Err(PatchApplyError::UnsupportedPlatform);
    }
    #[cfg(unix)]
    {
        match fs::symlink_metadata(path) {
            Ok(metadata) => {
                if metadata.file_type().is_symlink() {
                    return Err(PatchApplyError::PathSymlink(path.to_path_buf()));
                }
                if !metadata.is_file() {
                    return Err(PatchApplyError::PathNotRegular(path.to_path_buf()));
                }
                let bytes =
                    fs::read(path).map_err(|error| path_io("read patch path", path, error))?;
                Ok(OriginalFile::Regular {
                    bytes,
                    mode: metadata.mode(),
                })
            }
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(OriginalFile::Missing),
            Err(error) => Err(path_io("inspect patch path", path, error)),
        }
    }
}

fn original_hash(original: &OriginalFile) -> Option<ContentHash> {
    match original {
        OriginalFile::Missing => None,
        OriginalFile::Regular { bytes, .. } => Some(hash_canonical_bytes(HashDomain::File, bytes)),
    }
}

fn revalidate_snapshots(snapshots: &[FileSnapshot]) -> Result<(), PatchApplyError> {
    for snapshot in snapshots {
        let current = read_original(&snapshot.absolute)?;
        if current != snapshot.original {
            let actual = original_hash(&current);
            if actual != snapshot.expected_before {
                return Err(PatchApplyError::FileStateMismatch {
                    path: snapshot.absolute.clone(),
                    expected: snapshot.expected_before,
                    actual,
                });
            }
            return Err(PatchApplyError::FileMetadataChanged(
                snapshot.absolute.clone(),
            ));
        }
    }
    Ok(())
}

fn verify_after(files: &[PatchFileRecord], root: &Path) -> Result<(), PatchApplyError> {
    for file in files {
        let absolute = secure_path(root, file.path(), file.action() == PatchFileAction::Delete)?;
        let current = read_original(&absolute)?;
        let actual = original_hash(&current);
        if actual != file.after() {
            return Err(PatchApplyError::FileStateMismatch {
                path: file.path().as_path().to_path_buf(),
                expected: file.after(),
                actual,
            });
        }
    }
    Ok(())
}

fn rollback(snapshots: &[FileSnapshot]) -> Result<(), String> {
    let mut failures = Vec::new();
    for (index, snapshot) in snapshots.iter().enumerate() {
        if let Err(error) = restore_snapshot(snapshot, index) {
            failures.push(format!("{}: {error}", snapshot.absolute.display()));
        }
    }
    if failures.is_empty() {
        Ok(())
    } else {
        Err(failures.join("; "))
    }
}

fn restore_snapshot(snapshot: &FileSnapshot, index: usize) -> Result<(), io::Error> {
    match &snapshot.original {
        OriginalFile::Missing => match fs::symlink_metadata(&snapshot.absolute) {
            Ok(metadata) if metadata.file_type().is_symlink() || metadata.is_file() => {
                fs::remove_file(&snapshot.absolute)
            }
            Ok(_) => Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "rollback target became a non-file object",
            )),
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
            Err(error) => Err(error),
        },
        OriginalFile::Regular { bytes, mode } => {
            let parent = snapshot
                .absolute
                .parent()
                .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "path has no parent"))?;
            let (temp, mut file) = create_rollback_temp(parent, index)?;
            let result = (|| {
                file.write_all(bytes)?;
                file.sync_all()?;
                #[cfg(unix)]
                fs::set_permissions(&temp, fs::Permissions::from_mode(*mode))?;
                drop(file);
                fs::rename(&temp, &snapshot.absolute)?;
                sync_directory(parent)
            })();
            if result.is_err() {
                let _ = fs::remove_file(&temp);
            }
            result
        }
    }
}

fn create_rollback_temp(parent: &Path, index: usize) -> Result<(PathBuf, fs::File), io::Error> {
    for attempt in 0..128u16 {
        let path = parent.join(format!(
            ".forgeos-patch-rollback-{}-{index}-{attempt}",
            std::process::id()
        ));
        match OpenOptions::new().write(true).create_new(true).open(&path) {
            Ok(file) => return Ok((path, file)),
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(error),
        }
    }
    Err(io::Error::new(
        io::ErrorKind::AlreadyExists,
        "could not allocate a unique patch rollback file",
    ))
}

fn sync_directory(path: &Path) -> Result<(), io::Error> {
    let directory = OpenOptions::new().read(true).open(path)?;
    directory.sync_all()
}

#[derive(Debug)]
struct ApplyLock {
    path: PathBuf,
}

impl ApplyLock {
    fn acquire(root: &Path) -> Result<Self, PatchApplyError> {
        let path = root.join(APPLY_LOCK_NAME);
        let mut options = OpenOptions::new();
        options.write(true).create_new(true);
        match options.open(&path) {
            Ok(mut file) => {
                if let Err(error) =
                    writeln!(file, "{}", std::process::id()).and_then(|_| file.sync_all())
                {
                    drop(file);
                    let _ = fs::remove_file(&path);
                    return Err(path_io("write and sync patch lock", &path, error));
                }
                Ok(Self { path })
            }
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {
                Err(PatchApplyError::ApplyAlreadyInProgress(path))
            }
            Err(error) => Err(path_io("create patch lock", &path, error)),
        }
    }
}

impl Drop for ApplyLock {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}

fn path_io(operation: &'static str, path: &Path, error: io::Error) -> PatchApplyError {
    PatchApplyError::PathIo {
        operation,
        path: path.to_path_buf(),
        kind: error.kind(),
        message: error.to_string(),
    }
}
