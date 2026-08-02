//! Stable repository-relative path contracts shared across ForgeOS seams.
//!
//! Repository identity and filesystem location are deliberately separate. A
//! request names a stable [`RepositoryId`] plus one lexically canonical relative
//! path. Filesystem canonicalization and boundary enforcement remain owned by
//! `forge-project`.

use crate::identities::RepositoryId;
use std::fmt;
use std::path::{Component, Path, PathBuf};

/// A lexically canonical repository-relative path.
///
/// V1 accepts only one or more normal path components. Absolute paths, parent
/// traversal, current-directory aliases, platform prefixes, repeated separators,
/// and trailing separators are rejected rather than normalized silently.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RepositoryRelativePath(PathBuf);

impl RepositoryRelativePath {
    /// Validates a repository-relative path without touching the filesystem.
    pub fn new(path: impl AsRef<Path>) -> Result<Self, RepositoryPathError> {
        let path = path.as_ref();
        if path.as_os_str().is_empty() {
            return Err(RepositoryPathError::Empty);
        }
        if path.is_absolute() {
            return Err(RepositoryPathError::Absolute);
        }

        let mut normalized = PathBuf::new();
        for component in path.components() {
            match component {
                Component::Normal(value) => normalized.push(value),
                Component::ParentDir => return Err(RepositoryPathError::ParentTraversal),
                Component::CurDir => return Err(RepositoryPathError::NonCanonical),
                Component::RootDir => return Err(RepositoryPathError::Absolute),
                Component::Prefix(_) => return Err(RepositoryPathError::PlatformPrefix),
            }
        }

        if normalized.as_os_str().is_empty() {
            return Err(RepositoryPathError::Empty);
        }
        if normalized.as_os_str() != path.as_os_str() {
            return Err(RepositoryPathError::NonCanonical);
        }

        Ok(Self(normalized))
    }

    /// Exact validated relative path bytes as a platform path.
    pub fn as_path(&self) -> &Path {
        &self.0
    }

    /// Consumes the wrapper and returns the validated path.
    pub fn into_path_buf(self) -> PathBuf {
        self.0
    }
}

impl AsRef<Path> for RepositoryRelativePath {
    fn as_ref(&self) -> &Path {
        self.as_path()
    }
}

/// Stable repository identity paired with one validated relative path request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepositoryPathRequest {
    repository_id: RepositoryId,
    relative_path: RepositoryRelativePath,
}

impl RepositoryPathRequest {
    /// Creates a request after lexical path validation.
    pub fn new(
        repository_id: RepositoryId,
        relative_path: impl AsRef<Path>,
    ) -> Result<Self, RepositoryPathError> {
        Ok(Self {
            repository_id,
            relative_path: RepositoryRelativePath::new(relative_path)?,
        })
    }

    /// Stable repository identity that owns the requested child path.
    pub const fn repository_id(&self) -> RepositoryId {
        self.repository_id
    }

    /// Validated relative child path.
    pub fn relative_path(&self) -> &RepositoryRelativePath {
        &self.relative_path
    }
}

/// Exact lexical reason a repository-relative path was rejected.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RepositoryPathError {
    Empty,
    Absolute,
    ParentTraversal,
    PlatformPrefix,
    NonCanonical,
}

impl fmt::Display for RepositoryPathError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Empty => "repository-relative path must contain at least one component",
            Self::Absolute => "absolute paths are not valid repository-relative paths",
            Self::ParentTraversal => "parent traversal is forbidden in repository-relative paths",
            Self::PlatformPrefix => "platform path prefixes are forbidden in repository-relative paths",
            Self::NonCanonical => {
                "repository-relative path must already be lexically canonical"
            }
        })
    }
}

impl std::error::Error for RepositoryPathError {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::identities::IDENTITY_BYTES;

    fn repository_id(byte: u8) -> RepositoryId {
        RepositoryId::from_bytes([byte; IDENTITY_BYTES])
    }

    #[test]
    fn canonical_relative_paths_preserve_repository_identity() {
        let request = RepositoryPathRequest::new(repository_id(7), "src/lib.rs")
            .expect("canonical relative path");
        assert_eq!(request.repository_id(), repository_id(7));
        assert_eq!(request.relative_path().as_path(), Path::new("src/lib.rs"));
    }

    #[test]
    fn ambiguous_or_escaping_paths_are_rejected_without_normalization() {
        assert_eq!(
            RepositoryRelativePath::new(""),
            Err(RepositoryPathError::Empty)
        );
        assert_eq!(
            RepositoryRelativePath::new("/tmp/file"),
            Err(RepositoryPathError::Absolute)
        );
        assert_eq!(
            RepositoryRelativePath::new("../outside"),
            Err(RepositoryPathError::ParentTraversal)
        );
        assert_eq!(
            RepositoryRelativePath::new("src/../outside"),
            Err(RepositoryPathError::ParentTraversal)
        );
        assert_eq!(
            RepositoryRelativePath::new("./src/lib.rs"),
            Err(RepositoryPathError::NonCanonical)
        );
        assert_eq!(
            RepositoryRelativePath::new("src//lib.rs"),
            Err(RepositoryPathError::NonCanonical)
        );
        assert_eq!(
            RepositoryRelativePath::new("src/lib.rs/"),
            Err(RepositoryPathError::NonCanonical)
        );
    }

    #[cfg(unix)]
    #[test]
    fn non_utf8_path_bytes_are_not_replaced() {
        use std::ffi::OsString;
        use std::os::unix::ffi::{OsStrExt, OsStringExt};

        let raw = vec![b's', b'r', b'c', b'/', 0xff, b'.', b'r', b's'];
        let path = PathBuf::from(OsString::from_vec(raw.clone()));
        let validated = RepositoryRelativePath::new(&path).expect("non-UTF8 path is valid");
        assert_eq!(validated.as_path().as_os_str().as_bytes(), raw);
    }
}
