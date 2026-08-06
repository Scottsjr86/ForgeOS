//! Boundary-safe raw file access and atomic replacement for registered projects.
//!
//! File bytes remain authoritative on disk. This module resolves every operation
//! through a verified [`RepositoryBoundary`], enforces the manifest's allowed
//! roots, preserves raw bytes without encoding substitution, detects stale
//! revisions before commit, and replaces files through a synced same-directory
//! staging file.

use crate::paths::{FileSystemObjectId, RepositoryBoundary, RepositoryBoundaryError};
use forge_core::projects::{AllowedProjectRoot, ProjectManifest};
use forge_protocol::hashes::{ContentHash, HashDomain, hash_canonical_bytes};
use forge_protocol::identities::{ProjectId, RepositoryId};
use forge_protocol::paths::{RepositoryPathRequest, RepositoryRelativePath};
use std::ffi::{OsStr, OsString};
use std::fmt;
use std::fs::{self, File, Metadata, OpenOptions, Permissions};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};

#[cfg(unix)]
use std::os::unix::ffi::OsStrExt;
#[cfg(unix)]
use std::os::unix::fs::{MetadataExt, OpenOptionsExt, PermissionsExt};
#[cfg(unix)]
use std::os::unix::io::AsRawFd;

const MAX_FILE_BYTES: usize = 64 * 1024 * 1024;

#[cfg(target_os = "linux")]
const O_DIRECTORY: i32 = 0o200000;
#[cfg(target_os = "linux")]
const O_NOFOLLOW: i32 = 0o400000;
#[cfg(target_os = "linux")]
const O_CLOEXEC: i32 = 0o2000000;

/// Exact revision observed for one regular repository file.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FileRevision {
    object: FileSystemObjectId,
    content_hash: ContentHash,
    length: u64,
}

impl FileRevision {
    pub const fn object(self) -> FileSystemObjectId {
        self.object
    }

    pub const fn content_hash(self) -> ContentHash {
        self.content_hash
    }

    pub const fn length(self) -> u64 {
        self.length
    }
}

/// Expected state used as the optimistic-concurrency precondition for a write.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileExpectation {
    Missing,
    Exact(FileRevision),
}

/// Raw bytes and exact revision returned by one boundary-safe read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileSnapshot {
    repository_id: RepositoryId,
    relative_path: RepositoryRelativePath,
    display_path: PathBuf,
    revision: FileRevision,
    bytes: Vec<u8>,
}

impl FileSnapshot {
    pub const fn repository_id(&self) -> RepositoryId {
        self.repository_id
    }

    pub fn relative_path(&self) -> &RepositoryRelativePath {
        &self.relative_path
    }

    pub fn display_path(&self) -> &Path {
        &self.display_path
    }

    pub const fn revision(&self) -> FileRevision {
        self.revision
    }

    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }

    pub fn into_bytes(self) -> Vec<u8> {
        self.bytes
    }
}

/// Durability status after the atomic replacement itself has committed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WriteDurability {
    Confirmed,
    ParentSyncUncertain { kind: io::ErrorKind },
}

/// Result of one committed atomic file write.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FileWriteResult {
    revision: FileRevision,
    created: bool,
    durability: WriteDurability,
}

impl FileWriteResult {
    pub const fn revision(self) -> FileRevision {
        self.revision
    }

    pub const fn created(self) -> bool {
        self.created
    }

    pub const fn durability(self) -> WriteDurability {
        self.durability
    }
}

/// Manifest-bound repository file access.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectFileAccess {
    project_id: ProjectId,
    repository_id: RepositoryId,
    boundary: RepositoryBoundary,
    allowed_roots: Vec<AllowedProjectRoot>,
}

impl ProjectFileAccess {
    /// Binds file access to one validated project manifest and repository object.
    pub fn new(
        manifest: &ProjectManifest,
        boundary: &RepositoryBoundary,
    ) -> Result<Self, ProjectFileError> {
        if manifest.repository_id() != boundary.repository_id() {
            return Err(ProjectFileError::RepositoryMismatch {
                expected: manifest.repository_id(),
                found: boundary.repository_id(),
            });
        }
        boundary.revalidate()?;
        Ok(Self {
            project_id: manifest.project_id(),
            repository_id: manifest.repository_id(),
            boundary: boundary.clone(),
            allowed_roots: manifest.allowed_roots().to_vec(),
        })
    }

    pub const fn project_id(&self) -> ProjectId {
        self.project_id
    }

    pub const fn repository_id(&self) -> RepositoryId {
        self.repository_id
    }

    pub fn boundary(&self) -> &RepositoryBoundary {
        &self.boundary
    }

    /// Reads raw bytes without UTF-8 conversion or replacement.
    pub fn read(&self, request: &RepositoryPathRequest) -> Result<FileSnapshot, ProjectFileError> {
        self.validate_request(request)?;
        let parent = self.open_parent(request.relative_path())?;
        let leaf = target_leaf(request.relative_path())?;
        let snapshot = read_required_file(
            &parent,
            leaf,
            self.repository_id,
            request.relative_path(),
            self.boundary
                .display_root()
                .join(request.relative_path().as_path()),
        )?;
        Ok(snapshot.public)
    }

    /// Atomically creates or replaces one regular file after validating the exact
    /// expected on-disk revision.
    pub fn write_atomic(
        &self,
        request: &RepositoryPathRequest,
        expected: FileExpectation,
        bytes: &[u8],
    ) -> Result<FileWriteResult, ProjectFileError> {
        self.write_atomic_inner(request, expected, bytes, WriteFault::None)
    }

    fn write_atomic_inner(
        &self,
        request: &RepositoryPathRequest,
        expected: FileExpectation,
        bytes: &[u8],
        _fault: WriteFault,
    ) -> Result<FileWriteResult, ProjectFileError> {
        self.validate_request(request)?;
        if bytes.len() > MAX_FILE_BYTES {
            return Err(ProjectFileError::FileTooLarge {
                maximum: MAX_FILE_BYTES as u64,
                actual: bytes.len() as u64,
            });
        }

        let parent = self.open_parent(request.relative_path())?;
        let leaf = target_leaf(request.relative_path())?;
        let display_path = self
            .boundary
            .display_root()
            .join(request.relative_path().as_path());
        let current = read_optional_file(
            &parent,
            leaf,
            self.repository_id,
            request.relative_path(),
            display_path.clone(),
        )?;
        ensure_expectation(
            expected,
            current.as_ref().map(|value| value.public.revision),
        )?;

        let new_hash = hash_canonical_bytes(HashDomain::File, bytes);
        let stage_name = stage_name(request.relative_path(), new_hash)?;
        let stage_path = parent.proc_path.join(&stage_name);
        let target_path = parent.proc_path.join(leaf);
        let mode = current.as_ref().map(|value| value.mode).unwrap_or(0o666);

        let mut staged = StagedFile::create(stage_path.clone(), mode, current.is_some())?;
        staged.write_and_sync(bytes)?;
        let staged_metadata = staged.metadata()?;
        let staged_object = object_from_metadata(&staged_metadata)?;
        let staged_revision = FileRevision {
            object: staged_object,
            content_hash: new_hash,
            length: bytes.len() as u64,
        };

        #[cfg(test)]
        if _fault == WriteFault::BeforeConflictRecheck {
            return Err(ProjectFileError::InjectedFailure);
        }

        let rechecked = read_optional_file(
            &parent,
            leaf,
            self.repository_id,
            request.relative_path(),
            display_path,
        )?;
        ensure_expectation(
            expected,
            rechecked.as_ref().map(|value| value.public.revision),
        )?;

        #[cfg(test)]
        if _fault == WriteFault::BeforeReplace {
            return Err(ProjectFileError::InjectedFailure);
        }

        fs::rename(&stage_path, &target_path).map_err(|source| ProjectFileError::Io {
            operation: FileOperation::ReplaceTarget,
            path: request.relative_path().as_path().to_path_buf(),
            kind: source.kind(),
        })?;
        staged.commit();

        let durability = match parent.directory.sync_all() {
            Ok(()) => WriteDurability::Confirmed,
            Err(source) => WriteDurability::ParentSyncUncertain {
                kind: source.kind(),
            },
        };

        Ok(FileWriteResult {
            revision: staged_revision,
            created: current.is_none(),
            durability,
        })
    }

    fn validate_request(&self, request: &RepositoryPathRequest) -> Result<(), ProjectFileError> {
        if request.repository_id() != self.repository_id {
            return Err(ProjectFileError::RepositoryMismatch {
                expected: self.repository_id,
                found: request.repository_id(),
            });
        }
        if !self.path_is_allowed(request.relative_path()) {
            return Err(ProjectFileError::PathNotAllowed {
                path: request.relative_path().as_path().to_path_buf(),
            });
        }
        self.boundary.revalidate()?;
        Ok(())
    }

    fn path_is_allowed(&self, relative: &RepositoryRelativePath) -> bool {
        self.allowed_roots.iter().any(|root| match root {
            AllowedProjectRoot::RepositoryRoot => true,
            AllowedProjectRoot::Relative(allowed) => {
                relative.as_path().starts_with(allowed.as_path())
            }
        })
    }

    fn open_parent(
        &self,
        relative: &RepositoryRelativePath,
    ) -> Result<PinnedDirectory, ProjectFileError> {
        let parent_path = relative
            .as_path()
            .parent()
            .expect("validated repository path has a parent");
        if parent_path.as_os_str().is_empty() {
            return PinnedDirectory::open(
                self.boundary.canonical_root(),
                self.boundary.root_object(),
            );
        }

        let parent_relative = RepositoryRelativePath::new(parent_path)
            .expect("parent of a validated repository path remains valid");
        let parent_request =
            RepositoryPathRequest::new(self.repository_id, parent_relative.as_path())
                .expect("parent request was already validated");
        let resolved = self.boundary.resolve_existing(&parent_request)?;
        PinnedDirectory::open(resolved.canonical_path(), resolved.object())
    }
}

#[derive(Debug)]
struct PinnedDirectory {
    directory: File,
    proc_path: PathBuf,
}

impl PinnedDirectory {
    fn open(path: &Path, expected: FileSystemObjectId) -> Result<Self, ProjectFileError> {
        #[cfg(not(target_os = "linux"))]
        {
            let _ = (path, expected);
            return Err(ProjectFileError::UnsupportedPlatform);
        }

        #[cfg(target_os = "linux")]
        {
            let mut options = OpenOptions::new();
            options
                .read(true)
                .custom_flags(O_DIRECTORY | O_NOFOLLOW | O_CLOEXEC);
            let directory = options.open(path).map_err(|source| ProjectFileError::Io {
                operation: FileOperation::OpenParent,
                path: path.to_path_buf(),
                kind: source.kind(),
            })?;
            let metadata = directory
                .metadata()
                .map_err(|source| ProjectFileError::Io {
                    operation: FileOperation::InspectParent,
                    path: path.to_path_buf(),
                    kind: source.kind(),
                })?;
            if !metadata.is_dir() {
                return Err(ProjectFileError::ParentNotDirectory {
                    path: path.to_path_buf(),
                });
            }
            let found = object_from_metadata(&metadata)?;
            if found != expected {
                return Err(ProjectFileError::ParentIdentityChanged {
                    path: path.to_path_buf(),
                    expected,
                    found,
                });
            }
            let proc_path = PathBuf::from(format!("/proc/self/fd/{}", directory.as_raw_fd()));
            Ok(Self {
                directory,
                proc_path,
            })
        }
    }
}

#[derive(Debug)]
struct InternalSnapshot {
    public: FileSnapshot,
    mode: u32,
}

fn read_required_file(
    parent: &PinnedDirectory,
    leaf: &OsStr,
    repository_id: RepositoryId,
    relative: &RepositoryRelativePath,
    display_path: PathBuf,
) -> Result<InternalSnapshot, ProjectFileError> {
    read_optional_file(parent, leaf, repository_id, relative, display_path)?.ok_or_else(|| {
        ProjectFileError::Missing {
            path: relative.as_path().to_path_buf(),
        }
    })
}

fn read_optional_file(
    parent: &PinnedDirectory,
    leaf: &OsStr,
    repository_id: RepositoryId,
    relative: &RepositoryRelativePath,
    display_path: PathBuf,
) -> Result<Option<InternalSnapshot>, ProjectFileError> {
    #[cfg(not(target_os = "linux"))]
    {
        let _ = (parent, leaf, repository_id, relative, display_path);
        return Err(ProjectFileError::UnsupportedPlatform);
    }

    #[cfg(target_os = "linux")]
    {
        let path = parent.proc_path.join(leaf);
        let mut options = OpenOptions::new();
        options.read(true).custom_flags(O_NOFOLLOW | O_CLOEXEC);
        let mut file = match options.open(&path) {
            Ok(file) => file,
            Err(source) if source.kind() == io::ErrorKind::NotFound => return Ok(None),
            Err(source) => {
                if symlink_at(&path) {
                    return Err(ProjectFileError::SymlinkRejected {
                        path: relative.as_path().to_path_buf(),
                    });
                }
                return Err(ProjectFileError::Io {
                    operation: FileOperation::OpenFile,
                    path: relative.as_path().to_path_buf(),
                    kind: source.kind(),
                });
            }
        };
        let before = file.metadata().map_err(|source| ProjectFileError::Io {
            operation: FileOperation::InspectFile,
            path: relative.as_path().to_path_buf(),
            kind: source.kind(),
        })?;
        if !before.is_file() {
            return Err(ProjectFileError::NotRegularFile {
                path: relative.as_path().to_path_buf(),
            });
        }
        if before.len() > MAX_FILE_BYTES as u64 {
            return Err(ProjectFileError::FileTooLarge {
                maximum: MAX_FILE_BYTES as u64,
                actual: before.len(),
            });
        }
        let before_object = object_from_metadata(&before)?;
        let before_modified = before.modified().ok();
        let mut bytes = Vec::with_capacity(before.len() as usize);
        Read::by_ref(&mut file)
            .take((MAX_FILE_BYTES + 1) as u64)
            .read_to_end(&mut bytes)
            .map_err(|source| ProjectFileError::Io {
                operation: FileOperation::ReadFile,
                path: relative.as_path().to_path_buf(),
                kind: source.kind(),
            })?;
        if bytes.len() > MAX_FILE_BYTES {
            return Err(ProjectFileError::FileTooLarge {
                maximum: MAX_FILE_BYTES as u64,
                actual: bytes.len() as u64,
            });
        }
        let after = file.metadata().map_err(|source| ProjectFileError::Io {
            operation: FileOperation::InspectFile,
            path: relative.as_path().to_path_buf(),
            kind: source.kind(),
        })?;
        let after_object = object_from_metadata(&after)?;
        if before_object != after_object
            || before.len() != after.len()
            || before_modified != after.modified().ok()
        {
            return Err(ProjectFileError::ChangedDuringRead {
                path: relative.as_path().to_path_buf(),
            });
        }

        let revision = FileRevision {
            object: before_object,
            content_hash: hash_canonical_bytes(HashDomain::File, &bytes),
            length: bytes.len() as u64,
        };
        Ok(Some(InternalSnapshot {
            public: FileSnapshot {
                repository_id,
                relative_path: relative.clone(),
                display_path,
                revision,
                bytes,
            },
            mode: before.mode() & 0o7777,
        }))
    }
}

fn ensure_expectation(
    expected: FileExpectation,
    found: Option<FileRevision>,
) -> Result<(), ProjectFileError> {
    let found = found
        .map(FileExpectation::Exact)
        .unwrap_or(FileExpectation::Missing);
    if expected != found {
        return Err(ProjectFileError::Conflict { expected, found });
    }
    Ok(())
}

fn target_leaf(relative: &RepositoryRelativePath) -> Result<&OsStr, ProjectFileError> {
    relative
        .as_path()
        .file_name()
        .ok_or_else(|| ProjectFileError::InvalidTarget {
            path: relative.as_path().to_path_buf(),
        })
}

fn stage_name(
    relative: &RepositoryRelativePath,
    content_hash: ContentHash,
) -> Result<OsString, ProjectFileError> {
    #[cfg(not(unix))]
    {
        let _ = (relative, content_hash);
        Err(ProjectFileError::UnsupportedPlatform)
    }

    #[cfg(unix)]
    {
        let path_bytes = relative.as_path().as_os_str().as_bytes();
        let mut seed = Vec::with_capacity(8 + path_bytes.len() + 32);
        seed.extend_from_slice(&(path_bytes.len() as u64).to_be_bytes());
        seed.extend_from_slice(path_bytes);
        seed.extend_from_slice(content_hash.as_bytes());
        let identity = hash_canonical_bytes(HashDomain::File, &seed);
        Ok(OsString::from(format!(
            ".forgeos-write-{}.stage",
            identity.to_hex()
        )))
    }
}

#[derive(Debug)]
struct StagedFile {
    path: PathBuf,
    file: File,
    armed: bool,
}

impl StagedFile {
    fn create(path: PathBuf, mode: u32, preserve_mode: bool) -> Result<Self, ProjectFileError> {
        #[cfg(not(target_os = "linux"))]
        {
            let _ = (path, mode, preserve_mode);
            return Err(ProjectFileError::UnsupportedPlatform);
        }

        #[cfg(target_os = "linux")]
        {
            let mut options = OpenOptions::new();
            options
                .write(true)
                .create_new(true)
                .mode(mode)
                .custom_flags(O_NOFOLLOW | O_CLOEXEC);
            let file = options.open(&path).map_err(|source| {
                if source.kind() == io::ErrorKind::AlreadyExists {
                    ProjectFileError::InterruptedWrite { path: path.clone() }
                } else {
                    ProjectFileError::Io {
                        operation: FileOperation::CreateStage,
                        path: path.clone(),
                        kind: source.kind(),
                    }
                }
            })?;
            if preserve_mode {
                if let Err(source) = file.set_permissions(Permissions::from_mode(mode)) {
                    drop(file);
                    let _ = fs::remove_file(&path);
                    return Err(ProjectFileError::Io {
                        operation: FileOperation::SetStagePermissions,
                        path,
                        kind: source.kind(),
                    });
                }
            }
            Ok(Self {
                path,
                file,
                armed: true,
            })
        }
    }

    fn write_and_sync(&mut self, bytes: &[u8]) -> Result<(), ProjectFileError> {
        self.file
            .write_all(bytes)
            .map_err(|source| ProjectFileError::Io {
                operation: FileOperation::WriteStage,
                path: self.path.clone(),
                kind: source.kind(),
            })?;
        self.file.sync_all().map_err(|source| ProjectFileError::Io {
            operation: FileOperation::SyncStage,
            path: self.path.clone(),
            kind: source.kind(),
        })
    }

    fn metadata(&self) -> Result<Metadata, ProjectFileError> {
        self.file.metadata().map_err(|source| ProjectFileError::Io {
            operation: FileOperation::InspectStage,
            path: self.path.clone(),
            kind: source.kind(),
        })
    }

    fn commit(&mut self) {
        self.armed = false;
    }
}

impl Drop for StagedFile {
    fn drop(&mut self) {
        if self.armed {
            let _ = fs::remove_file(&self.path);
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum WriteFault {
    None,
    #[cfg(test)]
    BeforeConflictRecheck,
    #[cfg(test)]
    BeforeReplace,
}

/// Filesystem operation that produced a typed file-access failure.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileOperation {
    OpenParent,
    InspectParent,
    OpenFile,
    InspectFile,
    ReadFile,
    CreateStage,
    SetStagePermissions,
    WriteStage,
    SyncStage,
    InspectStage,
    ReplaceTarget,
}

impl FileOperation {
    const fn label(self) -> &'static str {
        match self {
            Self::OpenParent => "open parent directory",
            Self::InspectParent => "inspect parent directory",
            Self::OpenFile => "open repository file",
            Self::InspectFile => "inspect repository file",
            Self::ReadFile => "read repository file",
            Self::CreateStage => "create staged file",
            Self::SetStagePermissions => "set staged-file permissions",
            Self::WriteStage => "write staged file",
            Self::SyncStage => "sync staged file",
            Self::InspectStage => "inspect staged file",
            Self::ReplaceTarget => "replace repository file",
        }
    }
}

/// Exact reason a project file operation was rejected.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProjectFileError {
    UnsupportedPlatform,
    Boundary(RepositoryBoundaryError),
    RepositoryMismatch {
        expected: RepositoryId,
        found: RepositoryId,
    },
    PathNotAllowed {
        path: PathBuf,
    },
    InvalidTarget {
        path: PathBuf,
    },
    Missing {
        path: PathBuf,
    },
    SymlinkRejected {
        path: PathBuf,
    },
    ParentNotDirectory {
        path: PathBuf,
    },
    ParentIdentityChanged {
        path: PathBuf,
        expected: FileSystemObjectId,
        found: FileSystemObjectId,
    },
    NotRegularFile {
        path: PathBuf,
    },
    ChangedDuringRead {
        path: PathBuf,
    },
    FileTooLarge {
        maximum: u64,
        actual: u64,
    },
    Conflict {
        expected: FileExpectation,
        found: FileExpectation,
    },
    InterruptedWrite {
        path: PathBuf,
    },
    Io {
        operation: FileOperation,
        path: PathBuf,
        kind: io::ErrorKind,
    },
    InjectedFailure,
}

impl fmt::Display for ProjectFileError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedPlatform => {
                formatter.write_str("boundary-safe file access requires Linux")
            }
            Self::Boundary(source) => write!(formatter, "repository boundary rejected: {source}"),
            Self::RepositoryMismatch { expected, found } => write!(
                formatter,
                "file request repository mismatch: expected {expected}, found {found}"
            ),
            Self::PathNotAllowed { path } => {
                write!(
                    formatter,
                    "file path is outside approved project roots: {}",
                    path.display()
                )
            }
            Self::InvalidTarget { path } => {
                write!(
                    formatter,
                    "file target has no final path component: {}",
                    path.display()
                )
            }
            Self::Missing { path } => {
                write!(formatter, "repository file is missing: {}", path.display())
            }
            Self::SymlinkRejected { path } => {
                write!(
                    formatter,
                    "repository file symlink is rejected: {}",
                    path.display()
                )
            }
            Self::ParentNotDirectory { path } => {
                write!(
                    formatter,
                    "repository file parent is not a directory: {}",
                    path.display()
                )
            }
            Self::ParentIdentityChanged {
                path,
                expected,
                found,
            } => write!(
                formatter,
                "repository file parent changed during access at {}: expected {:?}, found {:?}",
                path.display(),
                expected,
                found
            ),
            Self::NotRegularFile { path } => {
                write!(
                    formatter,
                    "repository path is not a regular file: {}",
                    path.display()
                )
            }
            Self::ChangedDuringRead { path } => {
                write!(
                    formatter,
                    "repository file changed during read: {}",
                    path.display()
                )
            }
            Self::FileTooLarge { maximum, actual } => write!(
                formatter,
                "repository file exceeds maximum size: maximum {maximum} bytes, found {actual}"
            ),
            Self::Conflict { expected, found } => write!(
                formatter,
                "repository file revision conflict: expected {expected:?}, found {found:?}"
            ),
            Self::InterruptedWrite { path } => write!(
                formatter,
                "a staged repository file write already exists: {}",
                path.display()
            ),
            Self::Io {
                operation,
                path,
                kind,
            } => write!(
                formatter,
                "{} failed for {}: {kind:?}",
                operation.label(),
                path.display()
            ),
            Self::InjectedFailure => formatter.write_str("injected pre-commit file-write failure"),
        }
    }
}

impl std::error::Error for ProjectFileError {}

impl From<RepositoryBoundaryError> for ProjectFileError {
    fn from(source: RepositoryBoundaryError) -> Self {
        Self::Boundary(source)
    }
}

#[cfg(unix)]
fn object_from_metadata(metadata: &Metadata) -> Result<FileSystemObjectId, ProjectFileError> {
    Ok(FileSystemObjectId::from_raw(metadata.dev(), metadata.ino()))
}

#[cfg(not(unix))]
fn object_from_metadata(_metadata: &Metadata) -> Result<FileSystemObjectId, ProjectFileError> {
    Err(ProjectFileError::UnsupportedPlatform)
}

fn symlink_at(path: &Path) -> bool {
    fs::symlink_metadata(path)
        .map(|metadata| metadata.file_type().is_symlink())
        .unwrap_or(false)
}

#[cfg(all(test, target_os = "linux"))]
mod tests;
