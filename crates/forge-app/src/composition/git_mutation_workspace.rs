//! Project-bound Git mutation composition.
//!
//! `forge-git` owns native mutation semantics. `forge-project` owns the exact
//! registered repository boundary. This adapter binds one user-selected,
//! consistency-checked inspection snapshot to explicit Git mutations and returns
//! a new accepted snapshot after the operation. It owns no Git implementation.

use crate::composition::git_workspace::{
    ProjectGitSnapshot, ProjectGitWorkspace, ProjectGitWorkspaceError,
};
use forge_core::projects::ProjectManifest;
use forge_git::inspection::GitInspectionSnapshot;
use forge_git::mutation::{
    expected_file_state, CommitRequest, CreateWorktreeRequest, GitBranchName, GitCommitIdentity,
    GitMutationError, GitMutationOutcome, GitPathExpectation, GitRepositoryMutator,
    RemoveWorktreeConfirmation, RemoveWorktreeRequest, RestoreConfirmation, RestoreRequest,
    StageRequest, UnstageRequest,
};
use forge_git::status::{GitStatusEntry, GitStatusEntryKind};
use forge_git::types::GitObjectId;
use forge_project::paths::{RepositoryBoundary, RepositoryBoundaryError};
use forge_protocol::hashes::{hash_canonical_bytes, ContentHash, HashDomain};
use forge_protocol::identities::{ProjectId, RepositoryId};
use forge_protocol::paths::RepositoryRelativePath;
use std::collections::BTreeSet;
use std::fmt;
use std::path::PathBuf;

/// One native Git mutation plus the exact accepted state before and after it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectGitMutationResult {
    project_id: ProjectId,
    before_identity: ContentHash,
    outcome: GitMutationOutcome,
    after: ProjectGitSnapshot,
}

impl ProjectGitMutationResult {
    pub const fn project_id(&self) -> ProjectId {
        self.project_id
    }

    pub const fn repository_id(&self) -> RepositoryId {
        self.outcome.repository_id()
    }

    pub const fn before_identity(&self) -> ContentHash {
        self.before_identity
    }

    pub fn outcome(&self) -> &GitMutationOutcome {
        &self.outcome
    }

    pub fn after(&self) -> &ProjectGitSnapshot {
        &self.after
    }
}

/// Explicit mutation surface for one registered project repository.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectGitMutationWorkspace {
    project_id: ProjectId,
    repository_id: RepositoryId,
    boundary: RepositoryBoundary,
    inspection: ProjectGitWorkspace,
    mutator: GitRepositoryMutator,
}

impl ProjectGitMutationWorkspace {
    pub fn new(
        manifest: &ProjectManifest,
        boundary: RepositoryBoundary,
    ) -> Result<Self, ProjectGitMutationWorkspaceError> {
        if manifest.repository_id() != boundary.repository_id() {
            return Err(ProjectGitMutationWorkspaceError::RepositoryMismatch {
                manifest: manifest.repository_id(),
                boundary: boundary.repository_id(),
            });
        }
        boundary.revalidate()?;
        let inspection = ProjectGitWorkspace::new(manifest, boundary.clone())?;
        let mutator =
            GitRepositoryMutator::open(manifest.repository_id(), boundary.canonical_root())?;
        Ok(Self {
            project_id: manifest.project_id(),
            repository_id: manifest.repository_id(),
            boundary,
            inspection,
            mutator,
        })
    }

    pub const fn project_id(&self) -> ProjectId {
        self.project_id
    }

    pub const fn repository_id(&self) -> RepositoryId {
        self.repository_id
    }

    pub fn inspect(&self) -> Result<ProjectGitSnapshot, ProjectGitMutationWorkspaceError> {
        Ok(self.inspection.inspect()?)
    }

    /// Stages only paths that were explicitly selected from the accepted snapshot.
    pub fn stage(
        &self,
        before: &ProjectGitSnapshot,
        paths: Vec<RepositoryRelativePath>,
    ) -> Result<ProjectGitMutationResult, ProjectGitMutationWorkspaceError> {
        self.validate_before(before)?;
        let paths = self.validate_selected_paths(before.inspection(), paths)?;
        let mut expectations = Vec::with_capacity(paths.len());
        for path in paths {
            let state = expected_file_state(self.mutator.root().join(path.as_path()))?;
            expectations.push(GitPathExpectation::new(path, state));
        }
        let request = StageRequest::new(
            self.repository_id,
            before.inspection().status().head().revision().cloned(),
            expectations,
        )?;
        let outcome = self.mutator.stage(request)?;
        self.finish(before, outcome)
    }

    /// Removes only explicitly selected staged paths from the index.
    pub fn unstage(
        &self,
        before: &ProjectGitSnapshot,
        paths: Vec<RepositoryRelativePath>,
    ) -> Result<ProjectGitMutationResult, ProjectGitMutationWorkspaceError> {
        self.validate_before(before)?;
        let paths = self.validate_selected_paths(before.inspection(), paths)?;
        for path in &paths {
            let entry = selected_entry(before.inspection(), path)
                .expect("validated selected path must have one status entry");
            if !entry.status().is_some_and(|status| status.index() != b'.') {
                return Err(ProjectGitMutationWorkspaceError::PathNotStaged(
                    path.clone(),
                ));
            }
        }
        let head = required_head(before.inspection())?;
        let request = UnstageRequest::new(
            self.repository_id,
            head,
            staged_patch_identity(before.inspection()),
            paths,
        )?;
        let outcome = self.mutator.unstage(request)?;
        self.finish(before, outcome)
    }

    /// Discards only explicitly selected tracked worktree changes after confirmation.
    pub fn restore(
        &self,
        before: &ProjectGitSnapshot,
        paths: Vec<RepositoryRelativePath>,
        confirmation: RestoreConfirmation,
    ) -> Result<ProjectGitMutationResult, ProjectGitMutationWorkspaceError> {
        self.validate_before(before)?;
        let paths = self.validate_selected_paths(before.inspection(), paths)?;
        let mut expectations = Vec::with_capacity(paths.len());
        for path in paths {
            let entry = selected_entry(before.inspection(), &path)
                .expect("validated selected path must have one status entry");
            let restorable = !matches!(
                entry.kind(),
                GitStatusEntryKind::Untracked | GitStatusEntryKind::Ignored
            ) && entry
                .status()
                .is_some_and(|status| status.worktree() != b'.');
            if !restorable {
                return Err(ProjectGitMutationWorkspaceError::PathNotRestorable(path));
            }
            let state = expected_file_state(self.mutator.root().join(path.as_path()))?;
            expectations.push(GitPathExpectation::new(path, state));
        }
        let request = RestoreRequest::new(
            self.repository_id,
            required_head(before.inspection())?,
            expectations,
            confirmation,
        )?;
        let outcome = self.mutator.restore(request)?;
        self.finish(before, outcome)
    }

    /// Commits the exact staged patch visible in the accepted snapshot.
    pub fn commit(
        &self,
        before: &ProjectGitSnapshot,
        message: impl Into<Vec<u8>>,
        identity: GitCommitIdentity,
    ) -> Result<ProjectGitMutationResult, ProjectGitMutationWorkspaceError> {
        self.validate_before(before)?;
        let request = CommitRequest::new(
            self.repository_id,
            before.inspection().status().head().revision().cloned(),
            staged_patch_identity(before.inspection()),
            message,
            identity,
        )?;
        let outcome = self.mutator.commit(request)?;
        self.finish(before, outcome)
    }

    /// Creates one linked worktree from the exact selected primary HEAD.
    pub fn create_worktree(
        &self,
        before: &ProjectGitSnapshot,
        target: impl Into<PathBuf>,
        branch: GitBranchName,
    ) -> Result<ProjectGitMutationResult, ProjectGitMutationWorkspaceError> {
        self.validate_before(before)?;
        let head = required_head(before.inspection())?;
        let request = CreateWorktreeRequest::new(
            self.repository_id,
            Some(head.clone()),
            target,
            branch,
            head,
        )?;
        let outcome = self.mutator.create_worktree(request)?;
        self.finish(before, outcome)
    }

    /// Removes one exact clean linked worktree after explicit confirmation.
    pub fn remove_worktree(
        &self,
        before: &ProjectGitSnapshot,
        target: impl Into<PathBuf>,
        expected_head: GitObjectId,
        confirmation: RemoveWorktreeConfirmation,
    ) -> Result<ProjectGitMutationResult, ProjectGitMutationWorkspaceError> {
        self.validate_before(before)?;
        let request =
            RemoveWorktreeRequest::new(self.repository_id, target, expected_head, confirmation)?;
        let outcome = self.mutator.remove_worktree(request)?;
        self.finish(before, outcome)
    }

    fn validate_before(
        &self,
        before: &ProjectGitSnapshot,
    ) -> Result<(), ProjectGitMutationWorkspaceError> {
        self.boundary.revalidate()?;
        if before.project_id() != self.project_id {
            return Err(ProjectGitMutationWorkspaceError::ProjectMismatch {
                expected: self.project_id,
                actual: before.project_id(),
            });
        }
        if before.repository_id() != self.repository_id {
            return Err(
                ProjectGitMutationWorkspaceError::SnapshotRepositoryMismatch {
                    expected: self.repository_id,
                    actual: before.repository_id(),
                },
            );
        }
        let current = self.inspection.inspect()?;
        if current.inspection().identity() != before.inspection().identity() {
            return Err(ProjectGitMutationWorkspaceError::StaleSnapshot {
                expected: before.inspection().identity(),
                actual: current.inspection().identity(),
            });
        }
        Ok(())
    }

    fn validate_selected_paths(
        &self,
        snapshot: &GitInspectionSnapshot,
        paths: Vec<RepositoryRelativePath>,
    ) -> Result<Vec<RepositoryRelativePath>, ProjectGitMutationWorkspaceError> {
        if paths.is_empty() {
            return Err(ProjectGitMutationWorkspaceError::EmptySelection);
        }
        let mut seen = BTreeSet::new();
        for path in &paths {
            if !seen.insert(path.clone()) {
                return Err(ProjectGitMutationWorkspaceError::DuplicateSelection(
                    path.clone(),
                ));
            }
            if selected_entry(snapshot, path).is_none() {
                return Err(ProjectGitMutationWorkspaceError::PathNotInSnapshot(
                    path.clone(),
                ));
            }
        }
        Ok(paths)
    }

    fn finish(
        &self,
        before: &ProjectGitSnapshot,
        outcome: GitMutationOutcome,
    ) -> Result<ProjectGitMutationResult, ProjectGitMutationWorkspaceError> {
        if outcome.repository_id() != self.repository_id {
            return Err(
                ProjectGitMutationWorkspaceError::OutcomeRepositoryMismatch {
                    expected: self.repository_id,
                    actual: outcome.repository_id(),
                },
            );
        }
        let after = self.inspection.inspect()?;
        Ok(ProjectGitMutationResult {
            project_id: self.project_id,
            before_identity: before.inspection().identity(),
            outcome,
            after,
        })
    }
}

fn selected_entry<'a>(
    snapshot: &'a GitInspectionSnapshot,
    path: &RepositoryRelativePath,
) -> Option<&'a GitStatusEntry> {
    let selected = path.as_path().as_os_str().as_encoded_bytes();
    snapshot
        .status()
        .entries()
        .iter()
        .find(|entry| entry.path().as_bytes() == selected)
}

fn required_head(
    snapshot: &GitInspectionSnapshot,
) -> Result<GitObjectId, ProjectGitMutationWorkspaceError> {
    snapshot
        .status()
        .head()
        .revision()
        .cloned()
        .ok_or(ProjectGitMutationWorkspaceError::HeadUnavailable)
}

fn staged_patch_identity(snapshot: &GitInspectionSnapshot) -> ContentHash {
    hash_canonical_bytes(HashDomain::Patch, snapshot.staged_diff().patch_bytes())
}

/// Exact reason a project-bound Git mutation was rejected.
#[derive(Debug)]
pub enum ProjectGitMutationWorkspaceError {
    RepositoryMismatch {
        manifest: RepositoryId,
        boundary: RepositoryId,
    },
    ProjectMismatch {
        expected: ProjectId,
        actual: ProjectId,
    },
    SnapshotRepositoryMismatch {
        expected: RepositoryId,
        actual: RepositoryId,
    },
    OutcomeRepositoryMismatch {
        expected: RepositoryId,
        actual: RepositoryId,
    },
    StaleSnapshot {
        expected: ContentHash,
        actual: ContentHash,
    },
    EmptySelection,
    DuplicateSelection(RepositoryRelativePath),
    PathNotInSnapshot(RepositoryRelativePath),
    PathNotStaged(RepositoryRelativePath),
    PathNotRestorable(RepositoryRelativePath),
    HeadUnavailable,
    Boundary(RepositoryBoundaryError),
    Inspection(ProjectGitWorkspaceError),
    Mutation(GitMutationError),
}

impl fmt::Display for ProjectGitMutationWorkspaceError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::RepositoryMismatch { manifest, boundary } => write!(
                formatter,
                "project manifest repository {manifest} does not match boundary {boundary}"
            ),
            Self::ProjectMismatch { expected, actual } => write!(
                formatter,
                "Git mutation snapshot project {actual} does not match workspace project {expected}"
            ),
            Self::SnapshotRepositoryMismatch { expected, actual } => write!(
                formatter,
                "Git mutation snapshot repository {actual} does not match workspace repository {expected}"
            ),
            Self::OutcomeRepositoryMismatch { expected, actual } => write!(
                formatter,
                "Git mutation outcome repository {actual} does not match workspace repository {expected}"
            ),
            Self::StaleSnapshot { expected, actual } => write!(
                formatter,
                "Git mutation selection is stale: selected view {expected}, current view {actual}"
            ),
            Self::EmptySelection => formatter.write_str("Git mutation selection is empty"),
            Self::DuplicateSelection(path) => write!(
                formatter,
                "Git mutation selection contains duplicate path {}",
                path.as_path().display()
            ),
            Self::PathNotInSnapshot(path) => write!(
                formatter,
                "Git mutation path {} is not present in the selected status snapshot",
                path.as_path().display()
            ),
            Self::PathNotStaged(path) => write!(
                formatter,
                "Git mutation path {} is not staged in the selected snapshot",
                path.as_path().display()
            ),
            Self::PathNotRestorable(path) => write!(
                formatter,
                "Git mutation path {} is not a tracked worktree change in the selected snapshot",
                path.as_path().display()
            ),
            Self::HeadUnavailable => {
                formatter.write_str("Git mutation requires an existing exact HEAD revision")
            }
            Self::Boundary(error) => error.fmt(formatter),
            Self::Inspection(error) => error.fmt(formatter),
            Self::Mutation(error) => error.fmt(formatter),
        }
    }
}

impl std::error::Error for ProjectGitMutationWorkspaceError {}

impl From<RepositoryBoundaryError> for ProjectGitMutationWorkspaceError {
    fn from(error: RepositoryBoundaryError) -> Self {
        Self::Boundary(error)
    }
}

impl From<ProjectGitWorkspaceError> for ProjectGitMutationWorkspaceError {
    fn from(error: ProjectGitWorkspaceError) -> Self {
        Self::Inspection(error)
    }
}

impl From<GitMutationError> for ProjectGitMutationWorkspaceError {
    fn from(error: GitMutationError) -> Self {
        Self::Mutation(error)
    }
}
