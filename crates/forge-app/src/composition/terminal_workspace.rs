//! Thin composition adapter between project boundaries and managed PTYs.
//!
//! `forge-project` owns repository boundary resolution. `forge-terminal` owns PTY
//! process, byte, lifecycle, and transcript truth. This module only prepares a
//! project-bound launch from those public contracts.

use forge_core::projects::{AllowedProjectRoot, ProjectManifest};
use forge_project::paths::{RepositoryBoundary, RepositoryBoundaryError};
use forge_protocol::identities::{ProjectId, RepositoryId, TerminalId};
use forge_protocol::paths::{RepositoryPathError, RepositoryPathRequest, RepositoryRelativePath};
use forge_terminal::managed::{
    ManagedTerminalError, ManagedTerminalHandle, ManagedTerminalRegistry,
    ManagedTerminalSpawnRequest, ManagedTerminalView,
};
use forge_terminal::pty::{PtyDimensions, PtyRequestError, PtySpawnRequest};
use std::ffi::OsString;
use std::fmt;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

/// A working directory selected from the project's declared repository scope.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TerminalWorkingDirectory {
    RepositoryRoot,
    Relative(RepositoryRelativePath),
}

impl TerminalWorkingDirectory {
    pub const fn repository_root() -> Self {
        Self::RepositoryRoot
    }

    pub fn relative(path: impl AsRef<Path>) -> Result<Self, RepositoryPathError> {
        RepositoryRelativePath::new(path).map(Self::Relative)
    }
}

/// Shell-free terminal request before repository boundary resolution.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectTerminalLaunch {
    terminal_id: TerminalId,
    program: OsString,
    arguments: Vec<OsString>,
    working_directory: TerminalWorkingDirectory,
    dimensions: PtyDimensions,
}

impl ProjectTerminalLaunch {
    pub fn new(
        terminal_id: TerminalId,
        program: impl Into<OsString>,
        arguments: Vec<OsString>,
        working_directory: TerminalWorkingDirectory,
        dimensions: PtyDimensions,
    ) -> Self {
        Self {
            terminal_id,
            program: program.into(),
            arguments,
            working_directory,
            dimensions,
        }
    }
}

/// Product-facing terminal workspace bound to one validated project manifest.
pub struct ProjectTerminalWorkspace {
    project_id: ProjectId,
    repository_id: RepositoryId,
    allowed_roots: Vec<AllowedProjectRoot>,
    boundary: RepositoryBoundary,
    terminals: ManagedTerminalRegistry,
}

impl ProjectTerminalWorkspace {
    pub fn new(
        manifest: &ProjectManifest,
        boundary: RepositoryBoundary,
    ) -> Result<Self, ProjectTerminalWorkspaceError> {
        if manifest.repository_id() != boundary.repository_id() {
            return Err(ProjectTerminalWorkspaceError::RepositoryMismatch {
                manifest: manifest.repository_id(),
                boundary: boundary.repository_id(),
            });
        }
        Ok(Self {
            project_id: manifest.project_id(),
            repository_id: manifest.repository_id(),
            allowed_roots: manifest.allowed_roots().to_vec(),
            boundary,
            terminals: ManagedTerminalRegistry::new(),
        })
    }

    pub const fn project_id(&self) -> ProjectId {
        self.project_id
    }

    pub const fn repository_id(&self) -> RepositoryId {
        self.repository_id
    }

    pub fn spawn(
        &mut self,
        launch: ProjectTerminalLaunch,
    ) -> Result<ManagedTerminalHandle, ProjectTerminalWorkspaceError> {
        let working_directory = self.resolve_working_directory(&launch.working_directory)?;
        let pty = PtySpawnRequest::new(
            launch.terminal_id,
            launch.program,
            launch.arguments,
            working_directory,
            launch.dimensions,
        )?;
        let request = ManagedTerminalSpawnRequest::new(self.project_id, self.repository_id, pty)?;
        self.terminals.spawn(request).map_err(Into::into)
    }

    pub fn handles(&self) -> impl Iterator<Item = ManagedTerminalHandle> + '_ {
        self.terminals.handles()
    }

    pub fn view(
        &self,
        handle: ManagedTerminalHandle,
    ) -> Result<ManagedTerminalView, ProjectTerminalWorkspaceError> {
        self.terminals.view(handle).map_err(Into::into)
    }

    pub fn read_available(
        &mut self,
        handle: ManagedTerminalHandle,
    ) -> Result<(), ProjectTerminalWorkspaceError> {
        self.terminals.read_available(handle)?;
        Ok(())
    }

    pub fn write_input(
        &mut self,
        handle: ManagedTerminalHandle,
        bytes: &[u8],
    ) -> Result<(), ProjectTerminalWorkspaceError> {
        self.terminals
            .write_input(handle, bytes)
            .map_err(Into::into)
    }

    pub fn close_input(
        &mut self,
        handle: ManagedTerminalHandle,
    ) -> Result<(), ProjectTerminalWorkspaceError> {
        self.terminals.close_input(handle).map_err(Into::into)
    }

    pub fn resize(
        &mut self,
        handle: ManagedTerminalHandle,
        dimensions: PtyDimensions,
    ) -> Result<(), ProjectTerminalWorkspaceError> {
        self.terminals
            .resize(handle, dimensions)
            .map_err(Into::into)
    }

    pub fn poll_exit(
        &mut self,
        handle: ManagedTerminalHandle,
    ) -> Result<Option<forge_terminal::pty::PtyExit>, ProjectTerminalWorkspaceError> {
        self.terminals.poll_exit(handle).map_err(Into::into)
    }

    pub fn terminate(
        &mut self,
        handle: ManagedTerminalHandle,
    ) -> Result<forge_terminal::pty::PtyExit, ProjectTerminalWorkspaceError> {
        self.terminals.terminate(handle).map_err(Into::into)
    }

    pub fn remove_exited(
        &mut self,
        handle: ManagedTerminalHandle,
    ) -> Result<ManagedTerminalView, ProjectTerminalWorkspaceError> {
        self.terminals.remove_exited(handle).map_err(Into::into)
    }

    fn resolve_working_directory(
        &self,
        requested: &TerminalWorkingDirectory,
    ) -> Result<PathBuf, ProjectTerminalWorkspaceError> {
        self.boundary.revalidate()?;
        let path = match requested {
            TerminalWorkingDirectory::RepositoryRoot => {
                if !self
                    .allowed_roots
                    .iter()
                    .any(|root| matches!(root, AllowedProjectRoot::RepositoryRoot))
                {
                    return Err(
                        ProjectTerminalWorkspaceError::WorkingDirectoryOutsideAllowedRoots(
                            PathBuf::from("<repository-root>"),
                        ),
                    );
                }
                self.boundary.canonical_root().to_path_buf()
            }
            TerminalWorkingDirectory::Relative(relative) => {
                if !is_allowed_relative(relative, &self.allowed_roots) {
                    return Err(
                        ProjectTerminalWorkspaceError::WorkingDirectoryOutsideAllowedRoots(
                            relative.as_path().to_path_buf(),
                        ),
                    );
                }
                let request = RepositoryPathRequest::new(self.repository_id, relative.as_path())?;
                self.boundary
                    .resolve_existing(&request)?
                    .canonical_path()
                    .to_path_buf()
            }
        };

        let metadata = fs::symlink_metadata(&path).map_err(|error| {
            ProjectTerminalWorkspaceError::WorkingDirectoryIo {
                path: path.clone(),
                kind: error.kind(),
            }
        })?;
        if !metadata.is_dir() {
            return Err(ProjectTerminalWorkspaceError::WorkingDirectoryNotDirectory(
                path,
            ));
        }
        Ok(path)
    }
}

fn is_allowed_relative(
    requested: &RepositoryRelativePath,
    allowed_roots: &[AllowedProjectRoot],
) -> bool {
    allowed_roots.iter().any(|root| match root {
        AllowedProjectRoot::RepositoryRoot => true,
        AllowedProjectRoot::Relative(allowed) => requested.as_path().starts_with(allowed.as_path()),
    })
}

/// Failure from project/terminal composition.
#[derive(Debug)]
pub enum ProjectTerminalWorkspaceError {
    RepositoryMismatch {
        manifest: RepositoryId,
        boundary: RepositoryId,
    },
    InvalidPath(RepositoryPathError),
    Boundary(RepositoryBoundaryError),
    PtyRequest(PtyRequestError),
    Terminal(ManagedTerminalError),
    WorkingDirectoryOutsideAllowedRoots(PathBuf),
    WorkingDirectoryNotDirectory(PathBuf),
    WorkingDirectoryIo {
        path: PathBuf,
        kind: io::ErrorKind,
    },
}

impl fmt::Display for ProjectTerminalWorkspaceError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::RepositoryMismatch { manifest, boundary } => write!(
                formatter,
                "project manifest repository {manifest} does not match boundary {boundary}"
            ),
            Self::InvalidPath(error) => error.fmt(formatter),
            Self::Boundary(error) => error.fmt(formatter),
            Self::PtyRequest(error) => error.fmt(formatter),
            Self::Terminal(error) => error.fmt(formatter),
            Self::WorkingDirectoryOutsideAllowedRoots(path) => write!(
                formatter,
                "terminal working directory is outside declared project roots: {}",
                path.display()
            ),
            Self::WorkingDirectoryNotDirectory(path) => write!(
                formatter,
                "terminal working directory is not a directory: {}",
                path.display()
            ),
            Self::WorkingDirectoryIo { path, kind } => write!(
                formatter,
                "cannot inspect terminal working directory {}: {kind:?}",
                path.display()
            ),
        }
    }
}

impl std::error::Error for ProjectTerminalWorkspaceError {}

impl From<RepositoryPathError> for ProjectTerminalWorkspaceError {
    fn from(error: RepositoryPathError) -> Self {
        Self::InvalidPath(error)
    }
}

impl From<RepositoryBoundaryError> for ProjectTerminalWorkspaceError {
    fn from(error: RepositoryBoundaryError) -> Self {
        Self::Boundary(error)
    }
}

impl From<PtyRequestError> for ProjectTerminalWorkspaceError {
    fn from(error: PtyRequestError) -> Self {
        Self::PtyRequest(error)
    }
}

impl From<ManagedTerminalError> for ProjectTerminalWorkspaceError {
    fn from(error: ManagedTerminalError) -> Self {
        Self::Terminal(error)
    }
}
