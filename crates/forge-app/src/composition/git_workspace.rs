//! Project composition for read-only native Git inspection.
//!
//! `forge-project` owns the registered repository boundary. `forge-git` owns
//! native Git parsing and inspection truth. This adapter binds those contracts to
//! one project without caching, mutating, or synthesizing repository state.

use forge_core::projects::ProjectManifest;
use forge_git::inspection::{GitInspectionSnapshot, GitRepositoryInspectionError};
use forge_git::repository::{GitInspectError, GitRepositoryInspector};
use forge_project::paths::{RepositoryBoundary, RepositoryBoundaryError};
use forge_protocol::identities::{ProjectId, RepositoryId};
use std::fmt;

/// One immutable Git view attached to the project that requested it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectGitSnapshot {
    project_id: ProjectId,
    inspection: GitInspectionSnapshot,
}

impl ProjectGitSnapshot {
    pub const fn project_id(&self) -> ProjectId {
        self.project_id
    }

    pub const fn repository_id(&self) -> RepositoryId {
        self.inspection.repository_id()
    }

    pub fn inspection(&self) -> &GitInspectionSnapshot {
        &self.inspection
    }
}

/// Read-only Git workspace bound to one exact registered project repository.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectGitWorkspace {
    project_id: ProjectId,
    repository_id: RepositoryId,
    boundary: RepositoryBoundary,
    inspector: GitRepositoryInspector,
}

impl ProjectGitWorkspace {
    pub fn new(
        manifest: &ProjectManifest,
        boundary: RepositoryBoundary,
    ) -> Result<Self, ProjectGitWorkspaceError> {
        if manifest.repository_id() != boundary.repository_id() {
            return Err(ProjectGitWorkspaceError::RepositoryMismatch {
                manifest: manifest.repository_id(),
                boundary: boundary.repository_id(),
            });
        }
        boundary.revalidate()?;
        let inspector =
            GitRepositoryInspector::open(manifest.repository_id(), boundary.canonical_root())?;
        Ok(Self {
            project_id: manifest.project_id(),
            repository_id: manifest.repository_id(),
            boundary,
            inspector,
        })
    }

    pub const fn project_id(&self) -> ProjectId {
        self.project_id
    }

    pub const fn repository_id(&self) -> RepositoryId {
        self.repository_id
    }

    /// Reads one consistency-checked native Git view from the registered repository.
    pub fn inspect(&self) -> Result<ProjectGitSnapshot, ProjectGitWorkspaceError> {
        self.boundary.revalidate()?;
        if self.inspector.repository_id() != self.repository_id {
            return Err(ProjectGitWorkspaceError::InspectorRepositoryMismatch {
                expected: self.repository_id,
                found: self.inspector.repository_id(),
            });
        }
        let inspection = self.inspector.inspect_consistent()?;
        if inspection.repository_id() != self.repository_id {
            return Err(ProjectGitWorkspaceError::InspectionRepositoryMismatch {
                expected: self.repository_id,
                found: inspection.repository_id(),
            });
        }
        Ok(ProjectGitSnapshot {
            project_id: self.project_id,
            inspection,
        })
    }
}

/// Exact reason project-bound Git inspection was rejected.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProjectGitWorkspaceError {
    RepositoryMismatch {
        manifest: RepositoryId,
        boundary: RepositoryId,
    },
    InspectorRepositoryMismatch {
        expected: RepositoryId,
        found: RepositoryId,
    },
    InspectionRepositoryMismatch {
        expected: RepositoryId,
        found: RepositoryId,
    },
    Boundary(RepositoryBoundaryError),
    GitOpen(GitInspectError),
    GitInspection(GitRepositoryInspectionError),
}

impl fmt::Display for ProjectGitWorkspaceError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::RepositoryMismatch { manifest, boundary } => write!(
                formatter,
                "project manifest repository {manifest} does not match boundary {boundary}"
            ),
            Self::InspectorRepositoryMismatch { expected, found } => write!(
                formatter,
                "Git inspector repository {found} does not match project repository {expected}"
            ),
            Self::InspectionRepositoryMismatch { expected, found } => write!(
                formatter,
                "Git inspection repository {found} does not match project repository {expected}"
            ),
            Self::Boundary(error) => error.fmt(formatter),
            Self::GitOpen(error) => error.fmt(formatter),
            Self::GitInspection(error) => error.fmt(formatter),
        }
    }
}

impl std::error::Error for ProjectGitWorkspaceError {}

impl From<RepositoryBoundaryError> for ProjectGitWorkspaceError {
    fn from(error: RepositoryBoundaryError) -> Self {
        Self::Boundary(error)
    }
}

impl From<GitInspectError> for ProjectGitWorkspaceError {
    fn from(error: GitInspectError) -> Self {
        Self::GitOpen(error)
    }
}

impl From<GitRepositoryInspectionError> for ProjectGitWorkspaceError {
    fn from(error: GitRepositoryInspectionError) -> Self {
        Self::GitInspection(error)
    }
}
