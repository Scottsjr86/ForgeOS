//! Canonical repository roots and boundary-safe existing-child resolution.
//!
//! A [`RepositoryBoundary`] binds one stable [`RepositoryId`] to a real directory
//! object. The operator-facing display path is preserved separately from the
//! canonical root. Every child resolution revalidates the root, walks each path
//! component without following symlinks, rejects filesystem-device changes, and
//! confirms that the final canonical object remains inside the root.

use forge_protocol::identities::RepositoryId;
use forge_protocol::paths::{RepositoryPathRequest, RepositoryRelativePath};
use std::fmt;
use std::fs::{self, Metadata};
use std::io;
use std::path::{Path, PathBuf};

/// Stable operating-system identity for one filesystem object.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FileSystemObjectId {
    device: u64,
    object: u64,
}

impl FileSystemObjectId {
    pub(crate) const fn from_raw(device: u64, object: u64) -> Self {
        Self { device, object }
    }

    /// Filesystem or mount identity.
    pub const fn device(self) -> u64 {
        self.device
    }

    /// Object identity within the filesystem.
    pub const fn object(self) -> u64 {
        self.object
    }
}

/// One verified repository root with separate display and canonical locations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepositoryBoundary {
    repository_id: RepositoryId,
    display_root: PathBuf,
    canonical_root: PathBuf,
    root_object: FileSystemObjectId,
}

impl RepositoryBoundary {
    /// Opens and verifies a real repository root.
    pub fn open(
        repository_id: RepositoryId,
        display_root: impl AsRef<Path>,
    ) -> Result<Self, RepositoryBoundaryError> {
        let display_root = display_root.as_ref().to_path_buf();
        let inspected = inspect_root(&display_root)?;
        Ok(Self {
            repository_id,
            display_root,
            canonical_root: inspected.canonical,
            root_object: inspected.object,
        })
    }

    /// Stable repository identity independent from filesystem location.
    pub const fn repository_id(&self) -> RepositoryId {
        self.repository_id
    }

    /// Exact path supplied for operator display.
    pub fn display_root(&self) -> &Path {
        &self.display_root
    }

    /// Canonical absolute repository root used for enforcement.
    pub fn canonical_root(&self) -> &Path {
        &self.canonical_root
    }

    /// Filesystem object identity captured when the root was opened.
    pub const fn root_object(&self) -> FileSystemObjectId {
        self.root_object
    }

    /// Confirms that the displayed root still names the same canonical object.
    pub fn revalidate(&self) -> Result<(), RepositoryBoundaryError> {
        let inspected = inspect_root(&self.display_root)?;
        if inspected.object != self.root_object {
            return Err(RepositoryBoundaryError::RootIdentityChanged {
                expected: self.root_object,
                found: inspected.object,
            });
        }
        if inspected.canonical != self.canonical_root {
            return Err(RepositoryBoundaryError::RootLocationChanged {
                expected: self.canonical_root.clone(),
                found: inspected.canonical,
            });
        }
        Ok(())
    }

    /// Rebinds the display and canonical location after the same directory object
    /// has moved. A copied, replaced, or cross-filesystem root is rejected.
    pub fn relocate(
        &self,
        new_display_root: impl AsRef<Path>,
    ) -> Result<Self, RepositoryBoundaryError> {
        let new_display_root = new_display_root.as_ref().to_path_buf();
        let inspected = inspect_root(&new_display_root)?;
        if inspected.object != self.root_object {
            return Err(RepositoryBoundaryError::RootIdentityChanged {
                expected: self.root_object,
                found: inspected.object,
            });
        }
        Ok(Self {
            repository_id: self.repository_id,
            display_root: new_display_root,
            canonical_root: inspected.canonical,
            root_object: inspected.object,
        })
    }

    /// Resolves one existing child through the verified repository boundary.
    pub fn resolve_existing(
        &self,
        request: &RepositoryPathRequest,
    ) -> Result<ResolvedRepositoryPath, RepositoryBoundaryError> {
        if request.repository_id() != self.repository_id {
            return Err(RepositoryBoundaryError::RepositoryMismatch {
                expected: self.repository_id,
                found: request.repository_id(),
            });
        }
        self.revalidate()?;

        let relative = request.relative_path();
        let components: Vec<_> = relative.as_path().components().collect();
        let mut current = self.canonical_root.clone();
        let mut final_metadata = None;

        for (index, component) in components.iter().enumerate() {
            current.push(component.as_os_str());
            let metadata = symlink_metadata(&current, BoundaryOperation::InspectChild)?;
            if metadata.file_type().is_symlink() {
                return Err(RepositoryBoundaryError::SymlinkRejected { path: current });
            }
            if index + 1 < components.len() && !metadata.is_dir() {
                return Err(RepositoryBoundaryError::IntermediateNotDirectory { path: current });
            }
            let object = filesystem_object(&metadata)?;
            ensure_same_filesystem(self.root_object, object, &current)?;
            final_metadata = Some(metadata);
        }

        let before = filesystem_object(
            final_metadata
                .as_ref()
                .expect("validated relative paths contain at least one component"),
        )?;
        let canonical = canonicalize(&current, BoundaryOperation::CanonicalizeChild)?;
        if !canonical.starts_with(&self.canonical_root) {
            return Err(RepositoryBoundaryError::OutsideRepository { path: canonical });
        }

        let after_metadata = symlink_metadata(&canonical, BoundaryOperation::InspectCanonical)?;
        if after_metadata.file_type().is_symlink() {
            return Err(RepositoryBoundaryError::SymlinkRejected { path: canonical });
        }
        let after = filesystem_object(&after_metadata)?;
        if before != after {
            return Err(RepositoryBoundaryError::ObjectChangedDuringResolution {
                path: current,
                before,
                after,
            });
        }
        ensure_same_filesystem(self.root_object, after, &canonical)?;

        Ok(ResolvedRepositoryPath {
            repository_id: self.repository_id,
            relative_path: relative.clone(),
            display_path: self.display_root.join(relative.as_path()),
            canonical_path: canonical,
            object: after,
        })
    }
}

/// Verified result for one existing path inside a repository boundary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedRepositoryPath {
    repository_id: RepositoryId,
    relative_path: RepositoryRelativePath,
    display_path: PathBuf,
    canonical_path: PathBuf,
    object: FileSystemObjectId,
}

impl ResolvedRepositoryPath {
    pub const fn repository_id(&self) -> RepositoryId {
        self.repository_id
    }

    pub fn relative_path(&self) -> &RepositoryRelativePath {
        &self.relative_path
    }

    pub fn display_path(&self) -> &Path {
        &self.display_path
    }

    pub fn canonical_path(&self) -> &Path {
        &self.canonical_path
    }

    pub const fn object(&self) -> FileSystemObjectId {
        self.object
    }
}

/// Filesystem operation that produced a typed I/O failure.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BoundaryOperation {
    InspectRoot,
    CanonicalizeRoot,
    InspectCanonicalRoot,
    InspectChild,
    CanonicalizeChild,
    InspectCanonical,
}

impl BoundaryOperation {
    const fn label(self) -> &'static str {
        match self {
            Self::InspectRoot => "inspect repository root",
            Self::CanonicalizeRoot => "canonicalize repository root",
            Self::InspectCanonicalRoot => "inspect canonical repository root",
            Self::InspectChild => "inspect repository child",
            Self::CanonicalizeChild => "canonicalize repository child",
            Self::InspectCanonical => "inspect canonical repository child",
        }
    }
}

/// Exact reason a repository boundary or child resolution was rejected.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RepositoryBoundaryError {
    UnsupportedPlatform,
    Io {
        operation: BoundaryOperation,
        path: PathBuf,
        kind: io::ErrorKind,
    },
    RootSymlink {
        path: PathBuf,
    },
    RootNotDirectory {
        path: PathBuf,
    },
    RepositoryMismatch {
        expected: RepositoryId,
        found: RepositoryId,
    },
    SymlinkRejected {
        path: PathBuf,
    },
    IntermediateNotDirectory {
        path: PathBuf,
    },
    OutsideRepository {
        path: PathBuf,
    },
    UnexpectedMount {
        path: PathBuf,
        root_device: u64,
        found_device: u64,
    },
    RootIdentityChanged {
        expected: FileSystemObjectId,
        found: FileSystemObjectId,
    },
    RootLocationChanged {
        expected: PathBuf,
        found: PathBuf,
    },
    ObjectChangedDuringResolution {
        path: PathBuf,
        before: FileSystemObjectId,
        after: FileSystemObjectId,
    },
}

impl fmt::Display for RepositoryBoundaryError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedPlatform => {
                formatter.write_str("filesystem object identity is unsupported on this platform")
            }
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
            Self::RootSymlink { path } => {
                write!(
                    formatter,
                    "repository root is a symlink: {}",
                    path.display()
                )
            }
            Self::RootNotDirectory { path } => write!(
                formatter,
                "repository root is not a directory: {}",
                path.display()
            ),
            Self::RepositoryMismatch { expected, found } => write!(
                formatter,
                "repository path request targets {found}, but boundary owns {expected}"
            ),
            Self::SymlinkRejected { path } => write!(
                formatter,
                "repository child contains a symlink boundary: {}",
                path.display()
            ),
            Self::IntermediateNotDirectory { path } => write!(
                formatter,
                "repository child has a non-directory intermediate component: {}",
                path.display()
            ),
            Self::OutsideRepository { path } => write!(
                formatter,
                "canonical child escaped the repository boundary: {}",
                path.display()
            ),
            Self::UnexpectedMount {
                path,
                root_device,
                found_device,
            } => write!(
                formatter,
                "repository child crossed an unexpected mount at {}: root device {root_device}, found {found_device}",
                path.display()
            ),
            Self::RootIdentityChanged { expected, found } => write!(
                formatter,
                "repository root object changed from {expected:?} to {found:?}"
            ),
            Self::RootLocationChanged { expected, found } => write!(
                formatter,
                "repository canonical root changed from {} to {}",
                expected.display(),
                found.display()
            ),
            Self::ObjectChangedDuringResolution {
                path,
                before,
                after,
            } => write!(
                formatter,
                "repository child changed during resolution at {}: {before:?} -> {after:?}",
                path.display()
            ),
        }
    }
}

impl std::error::Error for RepositoryBoundaryError {}

struct InspectedRoot {
    canonical: PathBuf,
    object: FileSystemObjectId,
}

fn inspect_root(display_root: &Path) -> Result<InspectedRoot, RepositoryBoundaryError> {
    let display_metadata = symlink_metadata(display_root, BoundaryOperation::InspectRoot)?;
    if display_metadata.file_type().is_symlink() {
        return Err(RepositoryBoundaryError::RootSymlink {
            path: display_root.to_path_buf(),
        });
    }
    if !display_metadata.is_dir() {
        return Err(RepositoryBoundaryError::RootNotDirectory {
            path: display_root.to_path_buf(),
        });
    }
    let before = filesystem_object(&display_metadata)?;
    let canonical = canonicalize(display_root, BoundaryOperation::CanonicalizeRoot)?;
    let canonical_metadata = symlink_metadata(&canonical, BoundaryOperation::InspectCanonicalRoot)?;
    let after = filesystem_object(&canonical_metadata)?;
    if before != after {
        return Err(RepositoryBoundaryError::RootIdentityChanged {
            expected: before,
            found: after,
        });
    }
    Ok(InspectedRoot {
        canonical,
        object: after,
    })
}

fn symlink_metadata(
    path: &Path,
    operation: BoundaryOperation,
) -> Result<Metadata, RepositoryBoundaryError> {
    fs::symlink_metadata(path).map_err(|error| RepositoryBoundaryError::Io {
        operation,
        path: path.to_path_buf(),
        kind: error.kind(),
    })
}

fn canonicalize(
    path: &Path,
    operation: BoundaryOperation,
) -> Result<PathBuf, RepositoryBoundaryError> {
    fs::canonicalize(path).map_err(|error| RepositoryBoundaryError::Io {
        operation,
        path: path.to_path_buf(),
        kind: error.kind(),
    })
}

#[cfg(unix)]
fn filesystem_object(metadata: &Metadata) -> Result<FileSystemObjectId, RepositoryBoundaryError> {
    use std::os::unix::fs::MetadataExt;
    Ok(FileSystemObjectId {
        device: metadata.dev(),
        object: metadata.ino(),
    })
}

#[cfg(not(unix))]
fn filesystem_object(_metadata: &Metadata) -> Result<FileSystemObjectId, RepositoryBoundaryError> {
    Err(RepositoryBoundaryError::UnsupportedPlatform)
}

fn ensure_same_filesystem(
    root: FileSystemObjectId,
    child: FileSystemObjectId,
    path: &Path,
) -> Result<(), RepositoryBoundaryError> {
    if root.device != child.device {
        return Err(RepositoryBoundaryError::UnexpectedMount {
            path: path.to_path_buf(),
            root_device: root.device,
            found_device: child.device,
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn different_device_is_rejected_as_unexpected_mount() {
        let root = FileSystemObjectId {
            device: 1,
            object: 10,
        };
        let child = FileSystemObjectId {
            device: 2,
            object: 20,
        };
        assert_eq!(
            ensure_same_filesystem(root, child, Path::new("/repo/mounted")),
            Err(RepositoryBoundaryError::UnexpectedMount {
                path: PathBuf::from("/repo/mounted"),
                root_device: 1,
                found_device: 2,
            })
        );
    }
}
