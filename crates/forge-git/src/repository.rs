//! Read-only Git repository inspection bound to stable ForgeOS repository identity.
//!
//! The inspector accepts one exact repository root, captures the operating-system
//! directory identity, revalidates it before every command, invokes only the fixed
//! read operations exposed by `forge-bridge`, and preserves native Git failures.

use crate::diff::{parse_diff, DiffScope, GitDiff};
use crate::status::{parse_status, GitHead, GitStatusSnapshot};
use crate::worktree::{parse_worktrees, GitWorktreeSnapshot};
use forge_bridge::git::{
    GitDiffInvocation, GitReadRequest, NativeGitAdapter, NativeGitExit,
    NativeGitInvocationError, NativeGitOutput,
};
use forge_protocol::identities::RepositoryId;
use std::ffi::OsStr;
use std::fmt;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

#[cfg(unix)]
use std::os::unix::fs::MetadataExt;

/// Stable operating-system identity of the inspected repository directory.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct RepositoryObjectId {
    device: u64,
    inode: u64,
}

/// Exact read-only Git operation that failed or produced malformed output.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GitInspectOperation {
    OpenRepository,
    Status,
    Worktrees,
    DiffRaw,
    DiffPatch,
}

impl GitInspectOperation {
    pub const fn label(self) -> &'static str {
        match self {
            Self::OpenRepository => "open_repository",
            Self::Status => "status",
            Self::Worktrees => "worktrees",
            Self::DiffRaw => "diff_raw",
            Self::DiffPatch => "diff_patch",
        }
    }
}

/// Native Git command failure after the child process was successfully invoked.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GitNativeFailure {
    operation: GitInspectOperation,
    exit: NativeGitExit,
    stderr: Vec<u8>,
}

impl GitNativeFailure {
    pub const fn operation(&self) -> GitInspectOperation {
        self.operation
    }

    pub const fn exit(&self) -> NativeGitExit {
        self.exit
    }

    pub fn stderr(&self) -> &[u8] {
        &self.stderr
    }
}

/// Exact reason a read-only Git inspection could not be completed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GitInspectError {
    UnsupportedPlatform,
    RootIo {
        operation: &'static str,
        path: PathBuf,
        kind: io::ErrorKind,
        message: String,
    },
    RootSymlink(PathBuf),
    RootNotDirectory(PathBuf),
    RootIdentityChanged {
        expected_device: u64,
        expected_inode: u64,
        found_device: u64,
        found_inode: u64,
    },
    NotRepositoryRoot(PathBuf),
    NativeInvocation(NativeGitInvocationError),
    NativeFailure(GitNativeFailure),
    MalformedOutput {
        operation: GitInspectOperation,
        message: String,
    },
}

impl fmt::Display for GitInspectError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedPlatform => {
                formatter.write_str("Git repository identity requires Unix filesystem metadata")
            }
            Self::RootIo {
                operation,
                path,
                message,
                ..
            } => write!(formatter, "failed to {operation} {}: {message}", path.display()),
            Self::RootSymlink(path) => {
                write!(formatter, "Git repository root may not be a symlink: {}", path.display())
            }
            Self::RootNotDirectory(path) => {
                write!(formatter, "Git repository root is not a directory: {}", path.display())
            }
            Self::RootIdentityChanged {
                expected_device,
                expected_inode,
                found_device,
                found_inode,
            } => write!(
                formatter,
                "Git repository root identity changed from {expected_device}:{expected_inode} to {found_device}:{found_inode}"
            ),
            Self::NotRepositoryRoot(path) => write!(
                formatter,
                "path is inside a Git worktree but is not its root: {}",
                path.display()
            ),
            Self::NativeInvocation(error) => error.fmt(formatter),
            Self::NativeFailure(failure) => write!(
                formatter,
                "native Git {} failed with code {:?} signal {:?}",
                failure.operation.label(),
                failure.exit.code(),
                failure.exit.signal()
            ),
            Self::MalformedOutput { operation, message } => {
                write!(formatter, "native Git {} output is malformed: {message}", operation.label())
            }
        }
    }
}

impl std::error::Error for GitInspectError {}

impl From<NativeGitInvocationError> for GitInspectError {
    fn from(error: NativeGitInvocationError) -> Self {
        Self::NativeInvocation(error)
    }
}

/// One stable repository identity bound to a verified native Git worktree root.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GitRepositoryInspector {
    repository_id: RepositoryId,
    root: PathBuf,
    root_object: RepositoryObjectId,
    adapter: NativeGitAdapter,
}

impl GitRepositoryInspector {
    /// Opens a repository using the system `git` executable.
    pub fn open(
        repository_id: RepositoryId,
        root: impl AsRef<Path>,
    ) -> Result<Self, GitInspectError> {
        Self::open_with_adapter(repository_id, root, NativeGitAdapter::default())
    }

    /// Opens a repository with an explicit Git executable.
    pub fn with_program(
        repository_id: RepositoryId,
        root: impl AsRef<Path>,
        program: impl AsRef<OsStr>,
    ) -> Result<Self, GitInspectError> {
        Self::open_with_adapter(
            repository_id,
            root,
            NativeGitAdapter::with_program(program),
        )
    }

    fn open_with_adapter(
        repository_id: RepositoryId,
        root: impl AsRef<Path>,
        adapter: NativeGitAdapter,
    ) -> Result<Self, GitInspectError> {
        let display_root = root.as_ref();
        let (root, root_object) = inspect_root(display_root)?;
        let output = adapter.invoke(&root, &GitReadRequest::RepositoryRoot)?;
        expect_success(GitInspectOperation::OpenRepository, output).and_then(|output| {
            validate_repository_root_output(&root, output.stdout())?;
            Ok(Self {
                repository_id,
                root,
                root_object,
                adapter,
            })
        })
    }

    pub const fn repository_id(&self) -> RepositoryId {
        self.repository_id
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Reads branch, revision, and typed working-tree status in one native snapshot.
    pub fn inspect_status(&self) -> Result<GitStatusSnapshot, GitInspectError> {
        let output = self.invoke(GitInspectOperation::Status, GitReadRequest::Status)?;
        parse_status(self.repository_id, output.stdout()).map_err(|message| GitInspectError::MalformedOutput {
            operation: GitInspectOperation::Status,
            message,
        })
    }

    /// Reads only the current branch and revision from the same porcelain-v2 source.
    pub fn inspect_head(&self) -> Result<GitHead, GitInspectError> {
        self.inspect_status().map(GitStatusSnapshot::into_head)
    }

    /// Reads every native Git worktree through porcelain `-z` output.
    pub fn inspect_worktrees(&self) -> Result<GitWorktreeSnapshot, GitInspectError> {
        let output = self.invoke(GitInspectOperation::Worktrees, GitReadRequest::Worktrees)?;
        parse_worktrees(output.stdout())
            .map(|worktrees| GitWorktreeSnapshot::new(self.repository_id, worktrees))
            .map_err(|message| GitInspectError::MalformedOutput {
                operation: GitInspectOperation::Worktrees,
                message,
            })
    }

    /// Reads typed raw diff entries plus the exact native binary-safe patch bytes.
    pub fn inspect_diff(&self, scope: DiffScope) -> Result<GitDiff, GitInspectError> {
        let invocation = scope.to_invocation();
        let raw = self.invoke(
            GitInspectOperation::DiffRaw,
            GitReadRequest::DiffRaw(invocation.clone()),
        )?;
        let patch = self.invoke(
            GitInspectOperation::DiffPatch,
            GitReadRequest::DiffPatch(invocation),
        )?;
        parse_diff(self.repository_id, scope, raw.stdout(), patch.stdout()).map_err(|message| {
            GitInspectError::MalformedOutput {
                operation: GitInspectOperation::DiffRaw,
                message,
            }
        })
    }

    fn invoke(
        &self,
        operation: GitInspectOperation,
        request: GitReadRequest,
    ) -> Result<NativeGitOutput, GitInspectError> {
        self.revalidate_root()?;
        expect_success(operation, self.adapter.invoke(&self.root, &request)?)
    }

    fn revalidate_root(&self) -> Result<(), GitInspectError> {
        let (root, found) = inspect_root(&self.root)?;
        if root != self.root || found != self.root_object {
            return Err(GitInspectError::RootIdentityChanged {
                expected_device: self.root_object.device,
                expected_inode: self.root_object.inode,
                found_device: found.device,
                found_inode: found.inode,
            });
        }
        Ok(())
    }
}

fn expect_success(
    operation: GitInspectOperation,
    output: NativeGitOutput,
) -> Result<NativeGitOutput, GitInspectError> {
    if output.exit().success() {
        Ok(output)
    } else {
        Err(GitInspectError::NativeFailure(GitNativeFailure {
            operation,
            exit: output.exit(),
            stderr: output.stderr().to_vec(),
        }))
    }
}

fn validate_repository_root_output(root: &Path, output: &[u8]) -> Result<(), GitInspectError> {
    let mut lines = output.split(|byte| *byte == b'\n');
    let inside = lines.next().unwrap_or_default();
    let prefix = lines.next().unwrap_or_default();
    if inside != b"true" {
        return Err(GitInspectError::MalformedOutput {
            operation: GitInspectOperation::OpenRepository,
            message: "rev-parse did not report a worktree".to_owned(),
        });
    }
    if !prefix.is_empty() {
        return Err(GitInspectError::NotRepositoryRoot(root.to_path_buf()));
    }
    if lines.any(|line| !line.is_empty()) {
        return Err(GitInspectError::MalformedOutput {
            operation: GitInspectOperation::OpenRepository,
            message: "rev-parse returned unexpected trailing records".to_owned(),
        });
    }
    Ok(())
}

fn inspect_root(path: &Path) -> Result<(PathBuf, RepositoryObjectId), GitInspectError> {
    let metadata = fs::symlink_metadata(path).map_err(|error| root_io("inspect", path, error))?;
    if metadata.file_type().is_symlink() {
        return Err(GitInspectError::RootSymlink(path.to_path_buf()));
    }
    if !metadata.is_dir() {
        return Err(GitInspectError::RootNotDirectory(path.to_path_buf()));
    }
    let canonical = fs::canonicalize(path).map_err(|error| root_io("canonicalize", path, error))?;
    let canonical_metadata = fs::symlink_metadata(&canonical)
        .map_err(|error| root_io("inspect canonical", &canonical, error))?;
    Ok((canonical, filesystem_identity(&canonical_metadata)?))
}

fn root_io(operation: &'static str, path: &Path, error: io::Error) -> GitInspectError {
    GitInspectError::RootIo {
        operation,
        path: path.to_path_buf(),
        kind: error.kind(),
        message: error.to_string(),
    }
}

#[cfg(unix)]
fn filesystem_identity(metadata: &fs::Metadata) -> Result<RepositoryObjectId, GitInspectError> {
    Ok(RepositoryObjectId {
        device: metadata.dev(),
        inode: metadata.ino(),
    })
}

#[cfg(not(unix))]
fn filesystem_identity(_metadata: &fs::Metadata) -> Result<RepositoryObjectId, GitInspectError> {
    Err(GitInspectError::UnsupportedPlatform)
}

impl DiffScope {
    pub(crate) fn to_invocation(&self) -> GitDiffInvocation {
        match self {
            Self::Worktree => GitDiffInvocation::worktree(),
            Self::Staged => GitDiffInvocation::staged(),
            Self::Between { base, target } => GitDiffInvocation::between(
                base.as_str().to_owned(),
                target.as_str().to_owned(),
            )
            .expect("typed Git object IDs are valid diff arguments"),
        }
    }
}
