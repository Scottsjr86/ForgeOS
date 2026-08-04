//! Project composition for immutable registered commands and exact run history.
//!
//! Forge Core owns command definitions. Forge Project owns repository boundary
//! resolution. Forge Terminal owns native execution records. This adapter joins
//! those contracts without accepting shell strings or widening project scope.

use forge_bridge::processes::CancellationToken;
use forge_core::commands::{
    CommandRegistry, CommandRegistryError, CommandWorkingDirectory, RegisteredCommand,
};
use forge_core::projects::{AllowedProjectRoot, ProjectManifest};
use forge_project::paths::{RepositoryBoundary, RepositoryBoundaryError};
use forge_protocol::hashes::ContentHash;
use forge_protocol::identities::{CommandId, ProcessId, ProjectId, RepositoryId};
use forge_protocol::paths::{RepositoryPathError, RepositoryPathRequest, RepositoryRelativePath};
use forge_terminal::commands::CommandDirectoryBinding;
use forge_terminal::execution::{
    CommandRunError, CommandRunRecord, CommandRunRegistry, CommandSourceBinding,
};
use std::collections::BTreeMap;
use std::fmt;
use std::fs;
use std::io;
use std::path::PathBuf;

/// Product-facing registered-command workspace for one exact project revision.
pub struct ProjectCommandWorkspace {
    project_id: ProjectId,
    repository_id: RepositoryId,
    revision: ContentHash,
    allowed_roots: Vec<AllowedProjectRoot>,
    boundary: RepositoryBoundary,
    commands: CommandRegistry,
    declared_environment: BTreeMap<String, String>,
    runs: CommandRunRegistry,
}

impl ProjectCommandWorkspace {
    pub fn new(
        manifest: &ProjectManifest,
        boundary: RepositoryBoundary,
        revision: ContentHash,
        commands: impl IntoIterator<Item = RegisteredCommand>,
        declared_environment: BTreeMap<String, String>,
    ) -> Result<Self, ProjectCommandWorkspaceError> {
        if manifest.repository_id() != boundary.repository_id() {
            return Err(ProjectCommandWorkspaceError::RepositoryMismatch {
                manifest: manifest.repository_id(),
                boundary: boundary.repository_id(),
            });
        }
        let mut registry = CommandRegistry::new();
        for command in commands {
            if command.repository_id() != manifest.repository_id() {
                return Err(ProjectCommandWorkspaceError::CommandRepositoryMismatch {
                    command_id: command.command_id(),
                    command: command.repository_id(),
                    project: manifest.repository_id(),
                });
            }
            registry.register(command)?;
        }
        Ok(Self {
            project_id: manifest.project_id(),
            repository_id: manifest.repository_id(),
            revision,
            allowed_roots: manifest.allowed_roots().to_vec(),
            boundary,
            commands: registry,
            declared_environment,
            runs: CommandRunRegistry::new(),
        })
    }

    pub const fn project_id(&self) -> ProjectId {
        self.project_id
    }

    pub const fn repository_id(&self) -> RepositoryId {
        self.repository_id
    }

    pub const fn revision(&self) -> ContentHash {
        self.revision
    }

    pub fn command(&self, command_id: CommandId) -> Option<&RegisteredCommand> {
        self.commands.get(command_id)
    }

    /// Runs exactly the registered definition named by ID and expected identity.
    pub fn run(
        &mut self,
        command_id: CommandId,
        expected_definition: ContentHash,
        process_id: ProcessId,
        cancellation: &CancellationToken,
    ) -> Result<&CommandRunRecord, ProjectCommandWorkspaceError> {
        let command = self
            .commands
            .get(command_id)
            .cloned()
            .ok_or(ProjectCommandWorkspaceError::UnknownCommand(command_id))?;
        let actual = command.definition_identity();
        if actual != expected_definition {
            return Err(ProjectCommandWorkspaceError::StaleCommandDefinition {
                command_id,
                expected: expected_definition,
                actual,
            });
        }
        let directory = self.resolve_working_directory(command.working_directory())?;
        let source = CommandSourceBinding::new(self.project_id, self.repository_id, self.revision);
        self.runs
            .run(
                source,
                process_id,
                &command,
                &directory,
                &self.declared_environment,
                cancellation,
            )
            .map_err(Into::into)
    }

    pub fn history(&self) -> impl ExactSizeIterator<Item = &CommandRunRecord> {
        self.runs.iter()
    }

    pub fn run_record(&self, process_id: ProcessId) -> Option<&CommandRunRecord> {
        self.runs.get(process_id)
    }

    fn resolve_working_directory(
        &self,
        requested: &CommandWorkingDirectory,
    ) -> Result<CommandDirectoryBinding, ProjectCommandWorkspaceError> {
        self.boundary.revalidate()?;
        let path = match requested.relative_path() {
            None => {
                if !self
                    .allowed_roots
                    .iter()
                    .any(|root| matches!(root, AllowedProjectRoot::RepositoryRoot))
                {
                    return Err(
                        ProjectCommandWorkspaceError::WorkingDirectoryOutsideAllowedRoots(
                            PathBuf::from("<repository-root>"),
                        ),
                    );
                }
                self.boundary.canonical_root().to_path_buf()
            }
            Some(relative) => {
                if !is_allowed_relative(relative, &self.allowed_roots) {
                    return Err(
                        ProjectCommandWorkspaceError::WorkingDirectoryOutsideAllowedRoots(
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
            ProjectCommandWorkspaceError::WorkingDirectoryIo {
                path: path.clone(),
                kind: error.kind(),
            }
        })?;
        if !metadata.is_dir() {
            return Err(ProjectCommandWorkspaceError::WorkingDirectoryNotDirectory(
                path,
            ));
        }
        match requested.relative_path() {
            None => CommandDirectoryBinding::repository_root(self.repository_id, path),
            Some(relative) => {
                CommandDirectoryBinding::relative(self.repository_id, relative.clone(), path)
            }
        }
        .map_err(ProjectCommandWorkspaceError::CommandRunPreparation)
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

/// Failure from registered-command project composition.
#[derive(Debug)]
pub enum ProjectCommandWorkspaceError {
    RepositoryMismatch {
        manifest: RepositoryId,
        boundary: RepositoryId,
    },
    CommandRepositoryMismatch {
        command_id: CommandId,
        command: RepositoryId,
        project: RepositoryId,
    },
    CommandRegistry(CommandRegistryError),
    UnknownCommand(CommandId),
    StaleCommandDefinition {
        command_id: CommandId,
        expected: ContentHash,
        actual: ContentHash,
    },
    InvalidPath(RepositoryPathError),
    Boundary(RepositoryBoundaryError),
    CommandRunPreparation(forge_terminal::commands::CommandLaunchError),
    CommandRun(CommandRunError),
    WorkingDirectoryOutsideAllowedRoots(PathBuf),
    WorkingDirectoryNotDirectory(PathBuf),
    WorkingDirectoryIo {
        path: PathBuf,
        kind: io::ErrorKind,
    },
}

impl fmt::Display for ProjectCommandWorkspaceError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "project command workspace rejected operation: {self:?}"
        )
    }
}

impl std::error::Error for ProjectCommandWorkspaceError {}

impl From<CommandRegistryError> for ProjectCommandWorkspaceError {
    fn from(error: CommandRegistryError) -> Self {
        Self::CommandRegistry(error)
    }
}

impl From<RepositoryPathError> for ProjectCommandWorkspaceError {
    fn from(error: RepositoryPathError) -> Self {
        Self::InvalidPath(error)
    }
}

impl From<RepositoryBoundaryError> for ProjectCommandWorkspaceError {
    fn from(error: RepositoryBoundaryError) -> Self {
        Self::Boundary(error)
    }
}

impl From<CommandRunError> for ProjectCommandWorkspaceError {
    fn from(error: CommandRunError) -> Self {
        Self::CommandRun(error)
    }
}
