//! Boundary-safe repository tree projection for registered projects.
//!
//! The browser exposes only manifest-approved roots. Every discovered child is
//! re-resolved through [`RepositoryBoundary`], symlinks are never followed,
//! filesystem-device changes are rejected, and file opening delegates to the
//! existing raw file-access authority.

use crate::files::{FileSnapshot, ProjectFileAccess, ProjectFileError};
use crate::paths::{FileSystemObjectId, RepositoryBoundary, RepositoryBoundaryError};
use forge_core::projects::{AllowedProjectRoot, ProjectManifest};
use forge_protocol::identities::{ProjectId, RepositoryId};
use forge_protocol::paths::{RepositoryPathError, RepositoryPathRequest, RepositoryRelativePath};
use std::cmp::Ordering;
use std::ffi::OsString;
use std::fmt;
use std::fs::{self, File, Metadata, OpenOptions};
use std::io;
use std::path::{Path, PathBuf};

#[cfg(unix)]
use std::os::unix::ffi::OsStrExt;
#[cfg(unix)]
use std::os::unix::fs::{MetadataExt, OpenOptionsExt};
#[cfg(unix)]
use std::os::unix::io::AsRawFd;

const MAX_TREE_ENTRIES: usize = 100_000;
const MAX_TREE_DEPTH: usize = 128;

#[cfg(target_os = "linux")]
const O_DIRECTORY: i32 = 0o200000;
#[cfg(target_os = "linux")]
const O_NOFOLLOW: i32 = 0o400000;
#[cfg(target_os = "linux")]
const O_CLOEXEC: i32 = 0o2000000;

/// Approved portion of the repository to browse or search.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RepositoryBrowseScope {
    ApprovedRoots,
    Subtree(RepositoryRelativePath),
}

impl RepositoryBrowseScope {
    pub const fn approved_roots() -> Self {
        Self::ApprovedRoots
    }

    pub fn subtree(path: impl AsRef<Path>) -> Result<Self, RepositoryPathError> {
        RepositoryRelativePath::new(path).map(Self::Subtree)
    }
}

/// File type exposed by the safe repository tree.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RepositoryEntryKind {
    Directory,
    RegularFile,
}

/// One exact repository child included in a tree snapshot.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepositoryTreeEntry {
    relative_path: RepositoryRelativePath,
    display_path: PathBuf,
    kind: RepositoryEntryKind,
    object: FileSystemObjectId,
    byte_length: Option<u64>,
}

impl RepositoryTreeEntry {
    pub fn relative_path(&self) -> &RepositoryRelativePath {
        &self.relative_path
    }

    pub fn display_path(&self) -> &Path {
        &self.display_path
    }

    pub const fn kind(&self) -> RepositoryEntryKind {
        self.kind
    }

    pub const fn object(&self) -> FileSystemObjectId {
        self.object
    }

    pub const fn byte_length(&self) -> Option<u64> {
        self.byte_length
    }
}

/// Filesystem operation that failed while constructing a safe tree.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RepositoryScanOperation {
    ReadDirectory,
    ReadDirectoryEntry,
    InspectEntryType,
    InspectPinnedEntry,
    InspectResolvedEntry,
    OpenDirectory,
    InspectDirectory,
}

impl RepositoryScanOperation {
    const fn label(self) -> &'static str {
        match self {
            Self::ReadDirectory => "read repository directory",
            Self::ReadDirectoryEntry => "read repository directory entry",
            Self::InspectEntryType => "inspect repository entry type",
            Self::InspectPinnedEntry => "inspect pinned repository entry",
            Self::InspectResolvedEntry => "inspect resolved repository entry",
            Self::OpenDirectory => "open repository directory",
            Self::InspectDirectory => "inspect repository directory",
        }
    }
}

/// Explicit reason one discovered entry was omitted from the safe tree.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RepositoryScanIssueKind {
    Boundary(RepositoryBoundaryError),
    Io {
        operation: RepositoryScanOperation,
        kind: io::ErrorKind,
    },
    SymlinkRejected,
    UnsupportedFileType,
    ObjectChanged {
        expected: FileSystemObjectId,
        found: FileSystemObjectId,
    },
}

/// One rejected or unreadable repository entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepositoryScanIssue {
    relative_path: RepositoryRelativePath,
    kind: RepositoryScanIssueKind,
}

impl RepositoryScanIssue {
    pub fn relative_path(&self) -> &RepositoryRelativePath {
        &self.relative_path
    }

    pub fn kind(&self) -> &RepositoryScanIssueKind {
        &self.kind
    }
}

/// Deterministic read-only projection of one approved repository scope.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepositoryTreeSnapshot {
    project_id: ProjectId,
    repository_id: RepositoryId,
    entries: Vec<RepositoryTreeEntry>,
    issues: Vec<RepositoryScanIssue>,
}

impl RepositoryTreeSnapshot {
    pub const fn project_id(&self) -> ProjectId {
        self.project_id
    }

    pub const fn repository_id(&self) -> RepositoryId {
        self.repository_id
    }

    pub fn entries(&self) -> &[RepositoryTreeEntry] {
        &self.entries
    }

    pub fn issues(&self) -> &[RepositoryScanIssue] {
        &self.issues
    }
}

/// Manifest-bound tree browsing and exact file opening.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepositoryBrowser {
    project_id: ProjectId,
    repository_id: RepositoryId,
    boundary: RepositoryBoundary,
    allowed_roots: Vec<AllowedProjectRoot>,
    files: ProjectFileAccess,
}

impl RepositoryBrowser {
    pub fn new(
        manifest: &ProjectManifest,
        boundary: &RepositoryBoundary,
    ) -> Result<Self, RepositoryBrowseError> {
        let files = ProjectFileAccess::new(manifest, boundary)?;
        Ok(Self {
            project_id: manifest.project_id(),
            repository_id: manifest.repository_id(),
            boundary: boundary.clone(),
            allowed_roots: manifest.allowed_roots().to_vec(),
            files,
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

    /// Opens one exact regular file through the existing FILE-100 authority.
    pub fn open_file(
        &self,
        request: &RepositoryPathRequest,
    ) -> Result<FileSnapshot, ProjectFileError> {
        self.files.read(request)
    }

    /// Builds a deterministic tree without following rejected filesystem objects.
    pub fn tree(
        &self,
        scope: &RepositoryBrowseScope,
    ) -> Result<RepositoryTreeSnapshot, RepositoryBrowseError> {
        self.boundary.revalidate()?;
        let roots = self.scan_roots(scope)?;
        let mut entries = Vec::new();
        let mut issues = Vec::new();

        for root in roots {
            let directory =
                PinnedDirectory::open(&root.canonical_path, root.object).map_err(|kind| {
                    RepositoryBrowseError::ScopeUnavailable {
                        path: root
                            .relative_path
                            .as_ref()
                            .map(|path| path.as_path().to_path_buf()),
                        kind,
                    }
                })?;

            if let Some(relative_path) = root.relative_path.clone() {
                let metadata = directory.metadata().map_err(|kind| {
                    RepositoryBrowseError::ScopeUnavailable {
                        path: Some(relative_path.as_path().to_path_buf()),
                        kind,
                    }
                })?;
                push_entry(
                    &self.boundary,
                    relative_path,
                    &metadata,
                    root.object,
                    &mut entries,
                )?;
            }

            self.scan_directory(
                &directory,
                root.relative_path.as_ref(),
                0,
                &mut entries,
                &mut issues,
            )?;
        }

        Ok(RepositoryTreeSnapshot {
            project_id: self.project_id,
            repository_id: self.repository_id,
            entries,
            issues,
        })
    }

    pub(crate) fn safe_file_paths(
        &self,
        scope: &RepositoryBrowseScope,
    ) -> Result<(Vec<RepositoryRelativePath>, Vec<RepositoryScanIssue>), RepositoryBrowseError>
    {
        let tree = self.tree(scope)?;
        let files = tree
            .entries
            .iter()
            .filter(|entry| entry.kind == RepositoryEntryKind::RegularFile)
            .map(|entry| entry.relative_path.clone())
            .collect();
        Ok((files, tree.issues))
    }

    fn scan_roots(
        &self,
        scope: &RepositoryBrowseScope,
    ) -> Result<Vec<ScanRoot>, RepositoryBrowseError> {
        match scope {
            RepositoryBrowseScope::ApprovedRoots => self.approved_scan_roots(),
            RepositoryBrowseScope::Subtree(relative_path) => {
                if !self.path_is_allowed(relative_path) {
                    return Err(RepositoryBrowseError::PathNotAllowed {
                        path: relative_path.as_path().to_path_buf(),
                    });
                }
                Ok(vec![self.prepare_relative_root(relative_path.clone())?])
            }
        }
    }

    fn approved_scan_roots(&self) -> Result<Vec<ScanRoot>, RepositoryBrowseError> {
        if self
            .allowed_roots
            .iter()
            .any(|root| matches!(root, AllowedProjectRoot::RepositoryRoot))
        {
            return Ok(vec![ScanRoot {
                relative_path: None,
                canonical_path: self.boundary.canonical_root().to_path_buf(),
                object: self.boundary.root_object(),
            }]);
        }

        let mut relative_roots: Vec<_> = self
            .allowed_roots
            .iter()
            .filter_map(|root| root.relative_path().cloned())
            .collect();
        relative_roots.sort_by(|left, right| compare_paths(left.as_path(), right.as_path()));

        let mut pruned: Vec<RepositoryRelativePath> = Vec::new();
        for candidate in relative_roots {
            if pruned
                .iter()
                .any(|selected| candidate.as_path().starts_with(selected.as_path()))
            {
                continue;
            }
            pruned.push(candidate);
        }

        pruned
            .into_iter()
            .map(|relative_path| self.prepare_relative_root(relative_path))
            .collect()
    }

    fn prepare_relative_root(
        &self,
        relative_path: RepositoryRelativePath,
    ) -> Result<ScanRoot, RepositoryBrowseError> {
        let request = RepositoryPathRequest::new(self.repository_id, relative_path.as_path())
            .expect("manifest and browse paths are already lexically valid");
        let resolved = self.boundary.resolve_existing(&request)?;
        let metadata = inspect_metadata(resolved.canonical_path()).map_err(|kind| {
            RepositoryBrowseError::ScopeUnavailable {
                path: Some(relative_path.as_path().to_path_buf()),
                kind,
            }
        })?;
        let found = object_from_metadata(&metadata)?;
        if found != resolved.object() {
            return Err(RepositoryBrowseError::ScopeUnavailable {
                path: Some(relative_path.as_path().to_path_buf()),
                kind: RepositoryScanIssueKind::ObjectChanged {
                    expected: resolved.object(),
                    found,
                },
            });
        }
        if !metadata.is_dir() {
            return Err(RepositoryBrowseError::ScopeNotDirectory {
                path: relative_path.as_path().to_path_buf(),
            });
        }
        Ok(ScanRoot {
            relative_path: Some(relative_path),
            canonical_path: resolved.canonical_path().to_path_buf(),
            object: resolved.object(),
        })
    }

    fn path_is_allowed(&self, relative_path: &RepositoryRelativePath) -> bool {
        self.allowed_roots.iter().any(|root| match root {
            AllowedProjectRoot::RepositoryRoot => true,
            AllowedProjectRoot::Relative(allowed) => {
                relative_path.as_path().starts_with(allowed.as_path())
            }
        })
    }

    fn scan_directory(
        &self,
        directory: &PinnedDirectory,
        relative_parent: Option<&RepositoryRelativePath>,
        depth: usize,
        entries: &mut Vec<RepositoryTreeEntry>,
        issues: &mut Vec<RepositoryScanIssue>,
    ) -> Result<(), RepositoryBrowseError> {
        if depth > MAX_TREE_DEPTH {
            return Err(RepositoryBrowseError::TreeDepthExceeded {
                maximum: MAX_TREE_DEPTH,
            });
        }

        let mut children = Vec::new();
        let iterator = fs::read_dir(&directory.proc_path).map_err(|source| {
            RepositoryBrowseError::ScopeUnavailable {
                path: relative_parent.map(|path| path.as_path().to_path_buf()),
                kind: RepositoryScanIssueKind::Io {
                    operation: RepositoryScanOperation::ReadDirectory,
                    kind: source.kind(),
                },
            }
        })?;
        for child in iterator {
            children.push(
                child.map_err(|source| RepositoryBrowseError::ScopeUnavailable {
                    path: relative_parent.map(|path| path.as_path().to_path_buf()),
                    kind: RepositoryScanIssueKind::Io {
                        operation: RepositoryScanOperation::ReadDirectoryEntry,
                        kind: source.kind(),
                    },
                })?,
            );
        }
        children.sort_by(|left, right| compare_names(&left.file_name(), &right.file_name()));

        for child in children {
            if entries.len() >= MAX_TREE_ENTRIES {
                return Err(RepositoryBrowseError::TreeEntryLimitExceeded {
                    maximum: MAX_TREE_ENTRIES,
                });
            }

            let relative_path = child_relative_path(relative_parent, child.file_name())?;
            let file_type = match child.file_type() {
                Ok(file_type) => file_type,
                Err(source) => {
                    issues.push(RepositoryScanIssue {
                        relative_path,
                        kind: RepositoryScanIssueKind::Io {
                            operation: RepositoryScanOperation::InspectEntryType,
                            kind: source.kind(),
                        },
                    });
                    continue;
                }
            };
            if file_type.is_symlink() {
                issues.push(RepositoryScanIssue {
                    relative_path,
                    kind: RepositoryScanIssueKind::SymlinkRejected,
                });
                continue;
            }
            let pinned_metadata = match fs::symlink_metadata(child.path()) {
                Ok(metadata) => metadata,
                Err(source) => {
                    issues.push(RepositoryScanIssue {
                        relative_path,
                        kind: RepositoryScanIssueKind::Io {
                            operation: RepositoryScanOperation::InspectPinnedEntry,
                            kind: source.kind(),
                        },
                    });
                    continue;
                }
            };
            if pinned_metadata.file_type().is_symlink() {
                issues.push(RepositoryScanIssue {
                    relative_path,
                    kind: RepositoryScanIssueKind::SymlinkRejected,
                });
                continue;
            }
            let pinned_object = object_from_metadata(&pinned_metadata)?;

            let request = RepositoryPathRequest::new(self.repository_id, relative_path.as_path())
                .expect("filesystem child produced a canonical repository path");
            let resolved = match self.boundary.resolve_existing(&request) {
                Ok(resolved) => resolved,
                Err(source) => {
                    issues.push(RepositoryScanIssue {
                        relative_path,
                        kind: RepositoryScanIssueKind::Boundary(source),
                    });
                    continue;
                }
            };
            let metadata = match inspect_metadata(resolved.canonical_path()) {
                Ok(metadata) => metadata,
                Err(kind) => {
                    issues.push(RepositoryScanIssue {
                        relative_path,
                        kind,
                    });
                    continue;
                }
            };
            let found = object_from_metadata(&metadata)?;
            if found != resolved.object() {
                issues.push(RepositoryScanIssue {
                    relative_path,
                    kind: RepositoryScanIssueKind::ObjectChanged {
                        expected: resolved.object(),
                        found,
                    },
                });
                continue;
            }
            if found != pinned_object {
                issues.push(RepositoryScanIssue {
                    relative_path,
                    kind: RepositoryScanIssueKind::ObjectChanged {
                        expected: pinned_object,
                        found,
                    },
                });
                continue;
            }

            if metadata.is_dir() {
                push_entry(
                    &self.boundary,
                    relative_path.clone(),
                    &metadata,
                    found,
                    entries,
                )?;
                match PinnedDirectory::open(resolved.canonical_path(), found) {
                    Ok(child_directory) => self.scan_directory(
                        &child_directory,
                        Some(&relative_path),
                        depth + 1,
                        entries,
                        issues,
                    )?,
                    Err(kind) => issues.push(RepositoryScanIssue {
                        relative_path,
                        kind,
                    }),
                }
            } else if metadata.is_file() {
                push_entry(&self.boundary, relative_path, &metadata, found, entries)?;
            } else {
                issues.push(RepositoryScanIssue {
                    relative_path,
                    kind: RepositoryScanIssueKind::UnsupportedFileType,
                });
            }
        }
        Ok(())
    }
}

#[derive(Debug)]
struct ScanRoot {
    relative_path: Option<RepositoryRelativePath>,
    canonical_path: PathBuf,
    object: FileSystemObjectId,
}

#[derive(Debug)]
struct PinnedDirectory {
    _directory: File,
    proc_path: PathBuf,
}

impl PinnedDirectory {
    fn metadata(&self) -> Result<Metadata, RepositoryScanIssueKind> {
        self._directory
            .metadata()
            .map_err(|source| RepositoryScanIssueKind::Io {
                operation: RepositoryScanOperation::InspectDirectory,
                kind: source.kind(),
            })
    }

    fn open(path: &Path, expected: FileSystemObjectId) -> Result<Self, RepositoryScanIssueKind> {
        #[cfg(not(target_os = "linux"))]
        {
            let _ = (path, expected);
            return Err(RepositoryScanIssueKind::Io {
                operation: RepositoryScanOperation::OpenDirectory,
                kind: io::ErrorKind::Unsupported,
            });
        }

        #[cfg(target_os = "linux")]
        {
            let mut options = OpenOptions::new();
            options
                .read(true)
                .custom_flags(O_DIRECTORY | O_NOFOLLOW | O_CLOEXEC);
            let directory = options
                .open(path)
                .map_err(|source| RepositoryScanIssueKind::Io {
                    operation: RepositoryScanOperation::OpenDirectory,
                    kind: source.kind(),
                })?;
            let metadata = directory
                .metadata()
                .map_err(|source| RepositoryScanIssueKind::Io {
                    operation: RepositoryScanOperation::InspectDirectory,
                    kind: source.kind(),
                })?;
            if !metadata.is_dir() {
                return Err(RepositoryScanIssueKind::UnsupportedFileType);
            }
            let found =
                object_from_metadata(&metadata).map_err(|_| RepositoryScanIssueKind::Io {
                    operation: RepositoryScanOperation::InspectDirectory,
                    kind: io::ErrorKind::Unsupported,
                })?;
            if found != expected {
                return Err(RepositoryScanIssueKind::ObjectChanged { expected, found });
            }
            let proc_path = PathBuf::from(format!("/proc/self/fd/{}", directory.as_raw_fd()));
            Ok(Self {
                _directory: directory,
                proc_path,
            })
        }
    }
}

fn child_relative_path(
    parent: Option<&RepositoryRelativePath>,
    name: OsString,
) -> Result<RepositoryRelativePath, RepositoryBrowseError> {
    let path = match parent {
        Some(parent) => parent.as_path().join(name),
        None => PathBuf::from(name),
    };
    RepositoryRelativePath::new(&path)
        .map_err(|source| RepositoryBrowseError::InvalidDiscoveredPath { path, source })
}

fn push_entry(
    boundary: &RepositoryBoundary,
    relative_path: RepositoryRelativePath,
    metadata: &Metadata,
    object: FileSystemObjectId,
    entries: &mut Vec<RepositoryTreeEntry>,
) -> Result<(), RepositoryBrowseError> {
    if entries.len() >= MAX_TREE_ENTRIES {
        return Err(RepositoryBrowseError::TreeEntryLimitExceeded {
            maximum: MAX_TREE_ENTRIES,
        });
    }
    let kind = if metadata.is_dir() {
        RepositoryEntryKind::Directory
    } else if metadata.is_file() {
        RepositoryEntryKind::RegularFile
    } else {
        return Ok(());
    };
    entries.push(RepositoryTreeEntry {
        display_path: boundary.display_root().join(relative_path.as_path()),
        relative_path,
        kind,
        object,
        byte_length: metadata.is_file().then_some(metadata.len()),
    });
    Ok(())
}

fn inspect_metadata(path: &Path) -> Result<Metadata, RepositoryScanIssueKind> {
    fs::symlink_metadata(path).map_err(|source| RepositoryScanIssueKind::Io {
        operation: RepositoryScanOperation::InspectResolvedEntry,
        kind: source.kind(),
    })
}

#[cfg(unix)]
fn object_from_metadata(metadata: &Metadata) -> Result<FileSystemObjectId, RepositoryBrowseError> {
    Ok(FileSystemObjectId::from_raw(metadata.dev(), metadata.ino()))
}

#[cfg(not(unix))]
fn object_from_metadata(_metadata: &Metadata) -> Result<FileSystemObjectId, RepositoryBrowseError> {
    Err(RepositoryBrowseError::UnsupportedPlatform)
}

#[cfg(unix)]
fn compare_names(left: &OsString, right: &OsString) -> Ordering {
    left.as_os_str()
        .as_bytes()
        .cmp(right.as_os_str().as_bytes())
}

#[cfg(not(unix))]
fn compare_names(left: &OsString, right: &OsString) -> Ordering {
    left.cmp(right)
}

#[cfg(unix)]
fn compare_paths(left: &Path, right: &Path) -> Ordering {
    left.as_os_str()
        .as_bytes()
        .cmp(right.as_os_str().as_bytes())
}

#[cfg(not(unix))]
fn compare_paths(left: &Path, right: &Path) -> Ordering {
    left.cmp(right)
}

/// Exact reason tree construction or scope validation failed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RepositoryBrowseError {
    UnsupportedPlatform,
    Boundary(RepositoryBoundaryError),
    File(ProjectFileError),
    PathNotAllowed {
        path: PathBuf,
    },
    ScopeNotDirectory {
        path: PathBuf,
    },
    ScopeUnavailable {
        path: Option<PathBuf>,
        kind: RepositoryScanIssueKind,
    },
    InvalidDiscoveredPath {
        path: PathBuf,
        source: RepositoryPathError,
    },
    TreeEntryLimitExceeded {
        maximum: usize,
    },
    TreeDepthExceeded {
        maximum: usize,
    },
}

impl fmt::Display for RepositoryBrowseError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedPlatform => {
                formatter.write_str("boundary-safe repository browsing requires Linux")
            }
            Self::Boundary(source) => write!(formatter, "repository boundary rejected: {source}"),
            Self::File(source) => write!(formatter, "repository file access rejected: {source}"),
            Self::PathNotAllowed { path } => write!(
                formatter,
                "repository browse scope is outside approved roots: {}",
                path.display()
            ),
            Self::ScopeNotDirectory { path } => write!(
                formatter,
                "repository browse scope is not a directory: {}",
                path.display()
            ),
            Self::ScopeUnavailable { path, kind } => {
                if let Some(path) = path {
                    write!(
                        formatter,
                        "repository browse scope {} is unavailable: ",
                        path.display()
                    )?;
                } else {
                    formatter.write_str("repository root browse scope is unavailable: ")?;
                }
                match kind {
                    RepositoryScanIssueKind::Boundary(source) => write!(formatter, "{source}"),
                    RepositoryScanIssueKind::Io { operation, kind } => {
                        write!(formatter, "{} failed: {kind:?}", operation.label())
                    }
                    RepositoryScanIssueKind::SymlinkRejected => {
                        formatter.write_str("symlink is rejected")
                    }
                    RepositoryScanIssueKind::UnsupportedFileType => {
                        formatter.write_str("scope is not a supported directory")
                    }
                    RepositoryScanIssueKind::ObjectChanged { expected, found } => write!(
                        formatter,
                        "filesystem object changed during browse: expected {expected:?}, found {found:?}"
                    ),
                }
            }
            Self::InvalidDiscoveredPath { path, source } => write!(
                formatter,
                "filesystem entry produced an invalid repository path {}: {source}",
                path.display()
            ),
            Self::TreeEntryLimitExceeded { maximum } => write!(
                formatter,
                "repository tree exceeds the maximum of {maximum} entries"
            ),
            Self::TreeDepthExceeded { maximum } => write!(
                formatter,
                "repository tree exceeds the maximum depth of {maximum}"
            ),
        }
    }
}

impl std::error::Error for RepositoryBrowseError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Boundary(source) => Some(source),
            Self::File(source) => Some(source),
            Self::InvalidDiscoveredPath { source, .. } => Some(source),
            Self::UnsupportedPlatform
            | Self::PathNotAllowed { .. }
            | Self::ScopeNotDirectory { .. }
            | Self::ScopeUnavailable { .. }
            | Self::TreeEntryLimitExceeded { .. }
            | Self::TreeDepthExceeded { .. } => None,
        }
    }
}

impl From<RepositoryBoundaryError> for RepositoryBrowseError {
    fn from(source: RepositoryBoundaryError) -> Self {
        Self::Boundary(source)
    }
}

impl From<ProjectFileError> for RepositoryBrowseError {
    fn from(source: ProjectFileError) -> Self {
        Self::File(source)
    }
}
