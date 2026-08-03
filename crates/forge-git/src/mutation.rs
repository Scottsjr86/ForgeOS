use crate::diff::DiffScope;
use crate::repository::{GitInspectError, GitRepositoryInspector};
use crate::status::GitStatusSnapshot;
use crate::types::GitObjectId;
use crate::worktree::{GitWorktreeSnapshot, GitWorktreeState};
use forge_bridge::git::NativeGitExit;
use forge_bridge::git_mutation::{
    GitMutationArgumentError, GitMutationRequest, NativeGitMutationAdapter,
    NativeGitMutationInvocationError, NativeGitMutationOutput,
};
use forge_protocol::hashes::{hash_canonical_bytes, ContentHash, HashDomain};
use forge_protocol::identities::RepositoryId;
use forge_protocol::paths::RepositoryRelativePath;
use std::collections::BTreeSet;
use std::ffi::{OsStr, OsString};
use std::fmt;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
#[cfg(unix)]
use std::os::unix::ffi::OsStrExt;
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExpectedWorktreeState {
    Missing,
    File(ContentHash),
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GitPathExpectation {
    path: RepositoryRelativePath,
    state: ExpectedWorktreeState,
}
impl GitPathExpectation {
    pub const fn new(path: RepositoryRelativePath, state: ExpectedWorktreeState) -> Self {
        Self { path, state }
    }
    pub fn path(&self) -> &RepositoryRelativePath {
        &self.path
    }
    pub const fn state(&self) -> ExpectedWorktreeState {
        self.state
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RestoreConfirmation {
    DiscardExactPaths,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RemoveWorktreeConfirmation {
    RemoveCleanLinkedWorktree,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GitCommitIdentity {
    name: String,
    email: String,
}
impl GitCommitIdentity {
    pub fn new(name: impl Into<String>, email: impl Into<String>) -> Result<Self, GitMutationError> {
        let name = name.into();
        let email = email.into();
        validate_identity("name", &name)?;
        validate_identity("email", &email)?;
        if !email.contains('@') {
            return Err(GitMutationError::InvalidCommitIdentity {
                field: "email",
                message: "email must contain @".to_owned(),
            });
        }
        Ok(Self { name, email })
    }
    pub fn name(&self) -> &str {
        &self.name
    }
    pub fn email(&self) -> &str {
        &self.email
    }
}
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct GitBranchName(String);
impl GitBranchName {
    pub fn new(value: impl Into<String>) -> Result<Self, GitMutationError> {
        let value = value.into();
        let valid = !value.is_empty()
            && !value.starts_with('-')
            && !value.starts_with('/')
            && !value.ends_with('/')
            && !value.ends_with('.')
            && !value.ends_with(".lock")
            && !value.contains("..")
            && !value.contains("@{")
            && !value.contains("//")
            && value != "@"
            && value.split('/').all(|part| !part.starts_with('.') && !part.ends_with('.'))
            && value.bytes().all(|byte| {
                byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b'/')
            });
        if valid {
            Ok(Self(value))
        } else {
            Err(GitMutationError::InvalidBranchName(value))
        }
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StageRequest {
    repository_id: RepositoryId,
    expected_head: Option<GitObjectId>,
    paths: Vec<GitPathExpectation>,
}
impl StageRequest {
    pub fn new(
        repository_id: RepositoryId,
        expected_head: Option<GitObjectId>,
        paths: Vec<GitPathExpectation>,
    ) -> Result<Self, GitMutationError> {
        validate_expectations(&paths)?;
        Ok(Self {
            repository_id,
            expected_head,
            paths,
        })
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnstageRequest {
    repository_id: RepositoryId,
    expected_head: GitObjectId,
    expected_staged_patch: ContentHash,
    paths: Vec<RepositoryRelativePath>,
}
impl UnstageRequest {
    pub fn new(
        repository_id: RepositoryId,
        expected_head: GitObjectId,
        expected_staged_patch: ContentHash,
        paths: Vec<RepositoryRelativePath>,
    ) -> Result<Self, GitMutationError> {
        validate_paths(&paths)?;
        Ok(Self {
            repository_id,
            expected_head,
            expected_staged_patch,
            paths,
        })
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RestoreRequest {
    repository_id: RepositoryId,
    expected_head: GitObjectId,
    paths: Vec<GitPathExpectation>,
    confirmation: RestoreConfirmation,
}
impl RestoreRequest {
    pub fn new(
        repository_id: RepositoryId,
        expected_head: GitObjectId,
        paths: Vec<GitPathExpectation>,
        confirmation: RestoreConfirmation,
    ) -> Result<Self, GitMutationError> {
        validate_expectations(&paths)?;
        Ok(Self {
            repository_id,
            expected_head,
            paths,
            confirmation,
        })
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommitRequest {
    repository_id: RepositoryId,
    expected_head: Option<GitObjectId>,
    expected_staged_patch: ContentHash,
    message: Vec<u8>,
    identity: GitCommitIdentity,
}
impl CommitRequest {
    pub fn new(
        repository_id: RepositoryId,
        expected_head: Option<GitObjectId>,
        expected_staged_patch: ContentHash,
        message: impl Into<Vec<u8>>,
        identity: GitCommitIdentity,
    ) -> Result<Self, GitMutationError> {
        let message = message.into();
        if message.is_empty() {
            return Err(GitMutationError::EmptyCommitMessage);
        }
        if message.contains(&0) {
            return Err(GitMutationError::CommitMessageContainsNul);
        }
        Ok(Self {
            repository_id,
            expected_head,
            expected_staged_patch,
            message,
            identity,
        })
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreateWorktreeRequest {
    repository_id: RepositoryId,
    expected_head: Option<GitObjectId>,
    target: PathBuf,
    branch: GitBranchName,
    start: GitObjectId,
}
impl CreateWorktreeRequest {
    pub fn new(
        repository_id: RepositoryId,
        expected_head: Option<GitObjectId>,
        target: impl Into<PathBuf>,
        branch: GitBranchName,
        start: GitObjectId,
    ) -> Result<Self, GitMutationError> {
        let target = target.into();
        if !target.is_absolute() {
            return Err(GitMutationError::WorktreeTargetNotAbsolute(target));
        }
        Ok(Self {
            repository_id,
            expected_head,
            target,
            branch,
            start,
        })
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemoveWorktreeRequest {
    repository_id: RepositoryId,
    target: PathBuf,
    expected_head: GitObjectId,
    confirmation: RemoveWorktreeConfirmation,
}
impl RemoveWorktreeRequest {
    pub fn new(
        repository_id: RepositoryId,
        target: impl Into<PathBuf>,
        expected_head: GitObjectId,
        confirmation: RemoveWorktreeConfirmation,
    ) -> Result<Self, GitMutationError> {
        let target = target.into();
        if !target.is_absolute() {
            return Err(GitMutationError::WorktreeTargetNotAbsolute(target));
        }
        Ok(Self {
            repository_id,
            target,
            expected_head,
            confirmation,
        })
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GitMutationOperation {
    Stage,
    Unstage,
    RestoreWorktree,
    Commit,
    CreateWorktree,
    RemoveWorktree,
}
impl GitMutationOperation {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Stage => "stage",
            Self::Unstage => "unstage",
            Self::RestoreWorktree => "restore_worktree",
            Self::Commit => "commit",
            Self::CreateWorktree => "create_worktree",
            Self::RemoveWorktree => "remove_worktree",
        }
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GitNativeMutationFailure {
    operation: GitMutationOperation,
    exit: NativeGitExit,
    stdout: Vec<u8>,
    stderr: Vec<u8>,
}
impl GitNativeMutationFailure {
    pub const fn operation(&self) -> GitMutationOperation {
        self.operation
    }
    pub const fn exit(&self) -> NativeGitExit {
        self.exit
    }
    pub fn stdout(&self) -> &[u8] {
        &self.stdout
    }
    pub fn stderr(&self) -> &[u8] {
        &self.stderr
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GitMutationOutcome {
    repository_id: RepositoryId,
    operation: GitMutationOperation,
    native_exit: NativeGitExit,
    native_stdout: Vec<u8>,
    native_stderr: Vec<u8>,
    status: GitStatusSnapshot,
    worktrees: GitWorktreeSnapshot,
}
impl GitMutationOutcome {
    pub const fn repository_id(&self) -> RepositoryId {
        self.repository_id
    }
    pub const fn operation(&self) -> GitMutationOperation {
        self.operation
    }
    pub const fn native_exit(&self) -> NativeGitExit {
        self.native_exit
    }
    pub fn native_stdout(&self) -> &[u8] {
        &self.native_stdout
    }
    pub fn native_stderr(&self) -> &[u8] {
        &self.native_stderr
    }
    pub fn status(&self) -> &GitStatusSnapshot {
        &self.status
    }
    pub fn worktrees(&self) -> &GitWorktreeSnapshot {
        &self.worktrees
    }
}
#[derive(Debug)]
pub enum GitMutationError {
    Inspect(GitInspectError),
    NativeInvocation(NativeGitMutationInvocationError),
    NativeFailure(GitNativeMutationFailure),
    InvalidNativeRequest(GitMutationArgumentError),
    RepositoryMismatch {
        expected: RepositoryId,
        actual: RepositoryId,
    },
    HeadChanged {
        expected: Option<GitObjectId>,
        actual: Option<GitObjectId>,
    },
    StagedStateChanged {
        expected: ContentHash,
        actual: ContentHash,
    },
    EmptyStagedState,
    DuplicatePath(PathBuf),
    WorktreePathChanged {
        path: PathBuf,
        expected: ExpectedWorktreeState,
        actual: ExpectedWorktreeState,
    },
    WorktreePathIo {
        path: PathBuf,
        kind: io::ErrorKind,
        message: String,
    },
    WorktreePathNotFile(PathBuf),
    WorktreePathSymlink(PathBuf),
    InvalidCommitIdentity {
        field: &'static str,
        message: String,
    },
    EmptyCommitMessage,
    CommitMessageContainsNul,
    InvalidBranchName(String),
    WorktreeTargetNotAbsolute(PathBuf),
    WorktreeTargetExists(PathBuf),
    WorktreeTargetParentInvalid(PathBuf),
    WorktreeTargetCrossesPrimary(PathBuf),
    WorktreeNotRegistered(PathBuf),
    WorktreeIsPrimary(PathBuf),
    WorktreeHeadChanged {
        expected: GitObjectId,
        actual: GitObjectId,
    },
    WorktreeNotClean(PathBuf),
    UnsupportedPlatform,
    AppliedButInspectionFailed {
        operation: GitMutationOperation,
        source: GitInspectError,
    },
}
impl fmt::Display for GitMutationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Inspect(error) => error.fmt(formatter),
            Self::NativeInvocation(error) => error.fmt(formatter),
            Self::NativeFailure(failure) => write!(
                formatter,
                "native Git {} failed with code {:?} signal {:?}",
                failure.operation.label(),
                failure.exit.code(),
                failure.exit.signal()
            ),
            Self::InvalidNativeRequest(error) => error.fmt(formatter),
            Self::RepositoryMismatch { expected, actual } => write!(
                formatter,
                "Git mutation repository identity mismatch: expected {expected}, got {actual}"
            ),
            Self::HeadChanged { expected, actual } => {
                write!(formatter, "Git HEAD changed: expected {expected:?}, got {actual:?}")
            }
            Self::StagedStateChanged { expected, actual } => write!(
                formatter,
                "Git staged state changed: expected {expected}, got {actual}"
            ),
            Self::EmptyStagedState => formatter.write_str("Git staged state is empty"),
            Self::DuplicatePath(path) => {
                write!(formatter, "Git mutation path is duplicated: {}", path.display())
            }
            Self::WorktreePathChanged {
                path,
                expected,
                actual,
            } => write!(
                formatter,
                "worktree path {} changed: expected {expected:?}, got {actual:?}",
                path.display()
            ),
            Self::WorktreePathIo { path, message, .. } => {
                write!(formatter, "failed to inspect {}: {message}", path.display())
            }
            Self::WorktreePathNotFile(path) => {
                write!(formatter, "Git mutation path is not a regular file: {}", path.display())
            }
            Self::WorktreePathSymlink(path) => {
                write!(formatter, "Git mutation path may not be a symlink: {}", path.display())
            }
            Self::InvalidCommitIdentity { field, message } => {
                write!(formatter, "invalid Git commit identity {field}: {message}")
            }
            Self::EmptyCommitMessage => formatter.write_str("Git commit message is empty"),
            Self::CommitMessageContainsNul => {
                formatter.write_str("Git commit message contains NUL")
            }
            Self::InvalidBranchName(value) => write!(formatter, "invalid Git branch name: {value}"),
            Self::WorktreeTargetNotAbsolute(path) => write!(
                formatter,
                "linked-worktree target must be absolute: {}",
                path.display()
            ),
            Self::WorktreeTargetExists(path) => {
                write!(formatter, "linked-worktree target already exists: {}", path.display())
            }
            Self::WorktreeTargetParentInvalid(path) => write!(
                formatter,
                "linked-worktree target parent is invalid or noncanonical: {}",
                path.display()
            ),
            Self::WorktreeTargetCrossesPrimary(path) => write!(
                formatter,
                "linked-worktree target overlaps the primary worktree: {}",
                path.display()
            ),
            Self::WorktreeNotRegistered(path) => write!(
                formatter,
                "linked worktree is not registered: {}",
                path.display()
            ),
            Self::WorktreeIsPrimary(path) => write!(
                formatter,
                "primary worktree may not be removed: {}",
                path.display()
            ),
            Self::WorktreeHeadChanged { expected, actual } => write!(
                formatter,
                "linked-worktree HEAD changed: expected {expected}, got {actual}"
            ),
            Self::WorktreeNotClean(path) => write!(
                formatter,
                "linked worktree is not clean and cannot be removed: {}",
                path.display()
            ),
            Self::UnsupportedPlatform => {
                formatter.write_str("raw Git worktree paths require Unix path bytes")
            }
            Self::AppliedButInspectionFailed { operation, source } => write!(
                formatter,
                "Git {} completed but resulting state inspection failed: {source}",
                operation.label()
            ),
        }
    }
}
impl std::error::Error for GitMutationError {}
impl From<GitInspectError> for GitMutationError {
    fn from(error: GitInspectError) -> Self {
        Self::Inspect(error)
    }
}
impl From<NativeGitMutationInvocationError> for GitMutationError {
    fn from(error: NativeGitMutationInvocationError) -> Self {
        Self::NativeInvocation(error)
    }
}
impl From<GitMutationArgumentError> for GitMutationError {
    fn from(error: GitMutationArgumentError) -> Self {
        Self::InvalidNativeRequest(error)
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GitRepositoryMutator {
    inspector: GitRepositoryInspector,
    adapter: NativeGitMutationAdapter,
}
impl GitRepositoryMutator {
    pub fn open(
        repository_id: RepositoryId,
        root: impl AsRef<Path>,
    ) -> Result<Self, GitMutationError> {
        Ok(Self {
            inspector: GitRepositoryInspector::open(repository_id, root)?,
            adapter: NativeGitMutationAdapter::default(),
        })
    }
    pub fn from_inspector(inspector: GitRepositoryInspector) -> Self {
        Self {
            inspector,
            adapter: NativeGitMutationAdapter::default(),
        }
    }
    pub fn from_inspector_with_program(
        inspector: GitRepositoryInspector,
        program: impl AsRef<OsStr>,
    ) -> Self {
        Self {
            inspector,
            adapter: NativeGitMutationAdapter::with_program(program),
        }
    }
    pub const fn repository_id(&self) -> RepositoryId {
        self.inspector.repository_id()
    }
    pub fn root(&self) -> &Path {
        self.inspector.root()
    }
    pub fn stage(&self, request: StageRequest) -> Result<GitMutationOutcome, GitMutationError> {
        self.validate_repository(request.repository_id)?;
        self.validate_head(request.expected_head.as_ref())?;
        self.validate_expectations(&request.paths)?;
        let native = GitMutationRequest::stage(expectation_paths(&request.paths))?;
        self.run(GitMutationOperation::Stage, native)
    }
    pub fn unstage(
        &self,
        request: UnstageRequest,
    ) -> Result<GitMutationOutcome, GitMutationError> {
        self.validate_repository(request.repository_id)?;
        self.validate_head(Some(&request.expected_head))?;
        self.validate_staged_hash(request.expected_staged_patch, false)?;
        let native = GitMutationRequest::unstage(relative_paths(&request.paths))?;
        self.run(GitMutationOperation::Unstage, native)
    }
    pub fn restore(
        &self,
        request: RestoreRequest,
    ) -> Result<GitMutationOutcome, GitMutationError> {
        self.validate_repository(request.repository_id)?;
        self.validate_head(Some(&request.expected_head))?;
        self.validate_expectations(&request.paths)?;
        match request.confirmation {
            RestoreConfirmation::DiscardExactPaths => {}
        }
        let native = GitMutationRequest::restore_worktree(
            request.expected_head.as_str(),
            expectation_paths(&request.paths),
        )?;
        self.run(GitMutationOperation::RestoreWorktree, native)
    }
    pub fn commit(
        &self,
        request: CommitRequest,
    ) -> Result<GitMutationOutcome, GitMutationError> {
        self.validate_repository(request.repository_id)?;
        self.validate_head(request.expected_head.as_ref())?;
        self.validate_staged_hash(request.expected_staged_patch, true)?;
        let native = GitMutationRequest::commit(
            request.message,
            request.identity.name,
            request.identity.email,
        )?;
        self.run(GitMutationOperation::Commit, native)
    }
    pub fn create_worktree(
        &self,
        request: CreateWorktreeRequest,
    ) -> Result<GitMutationOutcome, GitMutationError> {
        self.validate_repository(request.repository_id)?;
        self.validate_head(request.expected_head.as_ref())?;
        let target = self.validate_new_worktree_target(&request.target)?;
        let native = GitMutationRequest::create_worktree(
            target.as_os_str(),
            request.branch.as_str(),
            request.start.as_str(),
        )?;
        self.run(GitMutationOperation::CreateWorktree, native)
    }
    pub fn remove_worktree(
        &self,
        request: RemoveWorktreeRequest,
    ) -> Result<GitMutationOutcome, GitMutationError> {
        self.validate_repository(request.repository_id)?;
        match request.confirmation {
            RemoveWorktreeConfirmation::RemoveCleanLinkedWorktree => {}
        }
        let target = self.validate_removable_worktree(&request.target, &request.expected_head)?;
        let native = GitMutationRequest::remove_worktree(target.as_os_str())?;
        self.run(GitMutationOperation::RemoveWorktree, native)
    }
    fn validate_repository(&self, actual: RepositoryId) -> Result<(), GitMutationError> {
        let expected = self.repository_id();
        if actual == expected {
            Ok(())
        } else {
            Err(GitMutationError::RepositoryMismatch { expected, actual })
        }
    }
    fn validate_head(&self, expected: Option<&GitObjectId>) -> Result<(), GitMutationError> {
        let actual = self.inspector.inspect_head()?.revision().cloned();
        if actual.as_ref() == expected {
            Ok(())
        } else {
            Err(GitMutationError::HeadChanged {
                expected: expected.cloned(),
                actual,
            })
        }
    }
    fn validate_staged_hash(
        &self,
        expected: ContentHash,
        require_nonempty: bool,
    ) -> Result<(), GitMutationError> {
        let staged = self.inspector.inspect_diff(DiffScope::Staged)?;
        if require_nonempty && staged.is_empty() {
            return Err(GitMutationError::EmptyStagedState);
        }
        let actual = hash_canonical_bytes(HashDomain::Patch, staged.patch_bytes());
        if actual == expected {
            Ok(())
        } else {
            Err(GitMutationError::StagedStateChanged { expected, actual })
        }
    }
    fn validate_expectations(
        &self,
        expectations: &[GitPathExpectation],
    ) -> Result<(), GitMutationError> {
        for expectation in expectations {
            let path = self.root().join(expectation.path.as_path());
            let actual = inspect_file_state(&path)?;
            if actual != expectation.state {
                return Err(GitMutationError::WorktreePathChanged {
                    path,
                    expected: expectation.state,
                    actual,
                });
            }
        }
        Ok(())
    }
    fn validate_new_worktree_target(&self, target: &Path) -> Result<PathBuf, GitMutationError> {
        match fs::symlink_metadata(target) {
            Ok(_) => return Err(GitMutationError::WorktreeTargetExists(target.to_path_buf())),
            Err(error) if error.kind() == io::ErrorKind::NotFound => {}
            Err(error) => {
                return Err(GitMutationError::WorktreePathIo {
                    path: target.to_path_buf(),
                    kind: error.kind(),
                    message: error.to_string(),
                })
            }
        }
        let Some(parent) = target.parent() else {
            return Err(GitMutationError::WorktreeTargetParentInvalid(
                target.to_path_buf(),
            ));
        };
        let Some(name) = target.file_name() else {
            return Err(GitMutationError::WorktreeTargetParentInvalid(
                target.to_path_buf(),
            ));
        };
        let metadata = fs::symlink_metadata(parent).map_err(|error| {
            GitMutationError::WorktreePathIo {
                path: parent.to_path_buf(),
                kind: error.kind(),
                message: error.to_string(),
            }
        })?;
        if metadata.file_type().is_symlink() || !metadata.is_dir() {
            return Err(GitMutationError::WorktreeTargetParentInvalid(
                parent.to_path_buf(),
            ));
        }
        let canonical_parent = fs::canonicalize(parent).map_err(|error| {
            GitMutationError::WorktreePathIo {
                path: parent.to_path_buf(),
                kind: error.kind(),
                message: error.to_string(),
            }
        })?;
        if canonical_parent != parent {
            return Err(GitMutationError::WorktreeTargetParentInvalid(
                parent.to_path_buf(),
            ));
        }
        let canonical_target = canonical_parent.join(name);
        if canonical_target.starts_with(self.root()) || self.root().starts_with(&canonical_target) {
            return Err(GitMutationError::WorktreeTargetCrossesPrimary(
                canonical_target,
            ));
        }
        Ok(canonical_target)
    }
    fn validate_removable_worktree(
        &self,
        target: &Path,
        expected_head: &GitObjectId,
    ) -> Result<PathBuf, GitMutationError> {
        let canonical = fs::canonicalize(target).map_err(|error| {
            GitMutationError::WorktreePathIo {
                path: target.to_path_buf(),
                kind: error.kind(),
                message: error.to_string(),
            }
        })?;
        if canonical == self.root() {
            return Err(GitMutationError::WorktreeIsPrimary(canonical));
        }
        let worktrees = self.inspector.inspect_worktrees()?;
        let listed = worktrees
            .worktrees()
            .iter()
            .find(|worktree| git_path_matches(worktree.path().as_bytes(), &canonical))
            .ok_or_else(|| GitMutationError::WorktreeNotRegistered(canonical.clone()))?;
        if listed.head() != expected_head {
            return Err(GitMutationError::WorktreeHeadChanged {
                expected: expected_head.clone(),
                actual: listed.head().clone(),
            });
        }
        if matches!(listed.state(), GitWorktreeState::Bare) {
            return Err(GitMutationError::WorktreeNotClean(canonical));
        }
        let linked = GitRepositoryInspector::open(self.repository_id(), &canonical)?;
        let status = linked.inspect_status()?;
        let actual_head = status.head().revision().cloned().ok_or_else(|| {
            GitMutationError::WorktreeNotClean(canonical.clone())
        })?;
        if &actual_head != expected_head {
            return Err(GitMutationError::WorktreeHeadChanged {
                expected: expected_head.clone(),
                actual: actual_head,
            });
        }
        if !status.is_clean() {
            return Err(GitMutationError::WorktreeNotClean(canonical));
        }
        Ok(canonical)
    }
    fn run(
        &self,
        operation: GitMutationOperation,
        request: GitMutationRequest,
    ) -> Result<GitMutationOutcome, GitMutationError> {
        self.inspector.revalidate_root()?;
        let output = self.adapter.invoke(self.root(), &request)?;
        if !output.exit().success() {
            return Err(GitMutationError::NativeFailure(native_failure(
                operation, output,
            )));
        }
        let status = self.inspector.inspect_status().map_err(|source| {
            GitMutationError::AppliedButInspectionFailed { operation, source }
        })?;
        let worktrees = self.inspector.inspect_worktrees().map_err(|source| {
            GitMutationError::AppliedButInspectionFailed { operation, source }
        })?;
        Ok(GitMutationOutcome {
            repository_id: self.repository_id(),
            operation,
            native_exit: output.exit(),
            native_stdout: output.stdout().to_vec(),
            native_stderr: output.stderr().to_vec(),
            status,
            worktrees,
        })
    }
}
pub fn staged_patch_identity(
    inspector: &GitRepositoryInspector,
) -> Result<ContentHash, GitMutationError> {
    let staged = inspector.inspect_diff(DiffScope::Staged)?;
    Ok(hash_canonical_bytes(
        HashDomain::Patch,
        staged.patch_bytes(),
    ))
}
pub fn expected_file_state(path: impl AsRef<Path>) -> Result<ExpectedWorktreeState, GitMutationError> {
    inspect_file_state(path.as_ref())
}
fn native_failure(
    operation: GitMutationOperation,
    output: NativeGitMutationOutput,
) -> GitNativeMutationFailure {
    GitNativeMutationFailure {
        operation,
        exit: output.exit(),
        stdout: output.stdout().to_vec(),
        stderr: output.stderr().to_vec(),
    }
}
fn inspect_file_state(path: &Path) -> Result<ExpectedWorktreeState, GitMutationError> {
    let metadata = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            return Ok(ExpectedWorktreeState::Missing)
        }
        Err(error) => {
            return Err(GitMutationError::WorktreePathIo {
                path: path.to_path_buf(),
                kind: error.kind(),
                message: error.to_string(),
            })
        }
    };
    if metadata.file_type().is_symlink() {
        return Err(GitMutationError::WorktreePathSymlink(path.to_path_buf()));
    }
    if !metadata.is_file() {
        return Err(GitMutationError::WorktreePathNotFile(path.to_path_buf()));
    }
    let bytes = fs::read(path).map_err(|error| GitMutationError::WorktreePathIo {
        path: path.to_path_buf(),
        kind: error.kind(),
        message: error.to_string(),
    })?;
    Ok(ExpectedWorktreeState::File(hash_canonical_bytes(
        HashDomain::File,
        &bytes,
    )))
}
fn validate_expectations(paths: &[GitPathExpectation]) -> Result<(), GitMutationError> {
    if paths.is_empty() {
        return Err(GitMutationError::InvalidNativeRequest(
            GitMutationArgumentError::EmptyPathSet,
        ));
    }
    let mut seen = BTreeSet::new();
    for expectation in paths {
        let path = expectation.path.as_path().to_path_buf();
        if !seen.insert(path.clone()) {
            return Err(GitMutationError::DuplicatePath(path));
        }
    }
    Ok(())
}
fn validate_paths(paths: &[RepositoryRelativePath]) -> Result<(), GitMutationError> {
    if paths.is_empty() {
        return Err(GitMutationError::InvalidNativeRequest(
            GitMutationArgumentError::EmptyPathSet,
        ));
    }
    let mut seen = BTreeSet::new();
    for path in paths {
        let path = path.as_path().to_path_buf();
        if !seen.insert(path.clone()) {
            return Err(GitMutationError::DuplicatePath(path));
        }
    }
    Ok(())
}
fn expectation_paths(paths: &[GitPathExpectation]) -> Vec<OsString> {
    paths
        .iter()
        .map(|expectation| expectation.path.as_path().as_os_str().to_os_string())
        .collect()
}
fn relative_paths(paths: &[RepositoryRelativePath]) -> Vec<OsString> {
    paths
        .iter()
        .map(|path| path.as_path().as_os_str().to_os_string())
        .collect()
}
fn validate_identity(field: &'static str, value: &str) -> Result<(), GitMutationError> {
    if value.is_empty() {
        return Err(GitMutationError::InvalidCommitIdentity {
            field,
            message: "value is empty".to_owned(),
        });
    }
    if value.bytes().any(|byte| matches!(byte, 0 | b'\n' | b'\r')) {
        return Err(GitMutationError::InvalidCommitIdentity {
            field,
            message: "value contains NUL or a line break".to_owned(),
        });
    }
    Ok(())
}
#[cfg(unix)]
fn git_path_matches(raw: &[u8], path: &Path) -> bool {
    path.as_os_str().as_bytes() == raw
}
#[cfg(not(unix))]
fn git_path_matches(_raw: &[u8], _path: &Path) -> bool {
    false
}
