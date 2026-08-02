//! Validated project-manifest import and repository registration.
//!
//! Forge Core owns manifest bytes. This adapter binds those bytes to a verified
//! repository object and rejects duplicate project or repository identities.

use crate::paths::{RepositoryBoundary, RepositoryBoundaryError};
use forge_core::projects::{AllowedProjectRoot, ProjectManifest, ProjectManifestError};
use forge_core::state::StateRecord;
use forge_protocol::identities::{ProjectId, RepositoryId};
use forge_protocol::paths::RepositoryPathRequest;
use std::collections::BTreeMap;
use std::fmt;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

/// One validated project manifest bound to a real repository directory object.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RegisteredProject {
    manifest: ProjectManifest,
    boundary: RepositoryBoundary,
}

impl RegisteredProject {
    pub fn manifest(&self) -> &ProjectManifest {
        &self.manifest
    }

    pub fn boundary(&self) -> &RepositoryBoundary {
        &self.boundary
    }

    /// Rebinds a moved repository only when the same filesystem directory object
    /// is found at the new display path.
    pub fn relocate(
        &mut self,
        new_display_root: impl AsRef<Path>,
    ) -> Result<(), ProjectRegistryError> {
        let relocated = self.boundary.relocate(new_display_root)?;
        validate_allowed_roots(&self.manifest, &relocated)?;
        self.boundary = relocated;
        Ok(())
    }
}

/// In-memory V1 registry enforcing unique project and repository identities.
#[derive(Debug, Default)]
pub struct ProjectRegistry {
    projects: BTreeMap<ProjectId, RegisteredProject>,
    repositories: BTreeMap<RepositoryId, ProjectId>,
}

impl ProjectRegistry {
    pub fn new() -> Self {
        Self {
            projects: BTreeMap::new(),
            repositories: BTreeMap::new(),
        }
    }

    pub fn len(&self) -> usize {
        self.projects.len()
    }

    pub fn is_empty(&self) -> bool {
        self.projects.is_empty()
    }

    pub fn get(&self, project_id: ProjectId) -> Option<&RegisteredProject> {
        self.projects.get(&project_id)
    }

    pub fn get_mut(&mut self, project_id: ProjectId) -> Option<&mut RegisteredProject> {
        self.projects.get_mut(&project_id)
    }

    pub fn import_bytes(
        &mut self,
        manifest_bytes: &[u8],
        display_root: impl AsRef<Path>,
    ) -> Result<&RegisteredProject, ProjectRegistryError> {
        let manifest = ProjectManifest::decode(manifest_bytes)?;
        self.import_manifest(manifest, display_root)
    }

    pub fn import_state_record(
        &mut self,
        record: &StateRecord,
        display_root: impl AsRef<Path>,
    ) -> Result<&RegisteredProject, ProjectRegistryError> {
        let manifest = ProjectManifest::from_state_record(record)?;
        self.import_manifest(manifest, display_root)
    }

    pub fn import_manifest(
        &mut self,
        manifest: ProjectManifest,
        display_root: impl AsRef<Path>,
    ) -> Result<&RegisteredProject, ProjectRegistryError> {
        let project_id = manifest.project_id();
        let repository_id = manifest.repository_id();
        if self.projects.contains_key(&project_id) {
            return Err(ProjectRegistryError::DuplicateProjectId(project_id));
        }
        if let Some(existing_project) = self.repositories.get(&repository_id) {
            return Err(ProjectRegistryError::DuplicateRepositoryId {
                repository_id,
                existing_project: *existing_project,
            });
        }

        let boundary = RepositoryBoundary::open(repository_id, display_root)?;
        validate_allowed_roots(&manifest, &boundary)?;
        self.repositories.insert(repository_id, project_id);
        self.projects.insert(
            project_id,
            RegisteredProject {
                manifest,
                boundary,
            },
        );
        Ok(self
            .projects
            .get(&project_id)
            .expect("project was inserted immediately before lookup"))
    }
}

fn validate_allowed_roots(
    manifest: &ProjectManifest,
    boundary: &RepositoryBoundary,
) -> Result<(), ProjectRegistryError> {
    for root in manifest.allowed_roots() {
        let AllowedProjectRoot::Relative(relative) = root else {
            boundary.revalidate()?;
            continue;
        };
        let request = RepositoryPathRequest::new(manifest.repository_id(), relative.as_path())
            .expect("manifest roots were already lexically validated");
        let resolved = boundary.resolve_existing(&request)?;
        let metadata = fs::symlink_metadata(resolved.canonical_path()).map_err(|source| {
            ProjectRegistryError::Io {
                path: resolved.canonical_path().to_path_buf(),
                kind: source.kind(),
            }
        })?;
        if metadata.file_type().is_symlink() || !metadata.is_dir() {
            return Err(ProjectRegistryError::AllowedRootNotDirectory {
                path: relative.as_path().to_path_buf(),
            });
        }
    }
    Ok(())
}

/// Exact reason project import or registration failed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProjectRegistryError {
    Manifest(ProjectManifestError),
    Boundary(RepositoryBoundaryError),
    DuplicateProjectId(ProjectId),
    DuplicateRepositoryId {
        repository_id: RepositoryId,
        existing_project: ProjectId,
    },
    AllowedRootNotDirectory { path: PathBuf },
    Io { path: PathBuf, kind: io::ErrorKind },
}

impl fmt::Display for ProjectRegistryError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Manifest(source) => write!(formatter, "project manifest rejected: {source}"),
            Self::Boundary(source) => write!(formatter, "repository boundary rejected: {source}"),
            Self::DuplicateProjectId(project_id) => {
                write!(formatter, "project ID is already registered: {project_id}")
            }
            Self::DuplicateRepositoryId {
                repository_id,
                existing_project,
            } => write!(
                formatter,
                "repository ID {repository_id} is already owned by project {existing_project}"
            ),
            Self::AllowedRootNotDirectory { path } => {
                write!(formatter, "allowed project root is not a directory: {}", path.display())
            }
            Self::Io { path, kind } => {
                write!(formatter, "project registration I/O failed for {}: {kind:?}", path.display())
            }
        }
    }
}

impl std::error::Error for ProjectRegistryError {}

impl From<ProjectManifestError> for ProjectRegistryError {
    fn from(source: ProjectManifestError) -> Self {
        Self::Manifest(source)
    }
}

impl From<RepositoryBoundaryError> for ProjectRegistryError {
    fn from(source: RepositoryBoundaryError) -> Self {
        Self::Boundary(source)
    }
}
