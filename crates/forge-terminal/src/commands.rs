//! Registered-command launch policy and exact inspectable payloads.
//!
//! This module does not execute commands. It resolves only the already-declared
//! environment policy and converts one immutable Core definition into an exact
//! shell-free launch payload. Real project-command execution and history remain
//! owned by `FORGEOS-V1-COMMAND-200`.

use forge_core::commands::{
    CommandAuthorityClass, CommandCancellationPolicy, CommandEnvironmentSource,
    CommandTimeout, CommandWorkingDirectory, RegisteredCommand,
};
use forge_protocol::hashes::ContentHash;
use forge_protocol::identities::{CommandId, ProcessId, RepositoryId};
use forge_protocol::paths::RepositoryRelativePath;
use forge_protocol::processes::{ProcessRequestError, ProcessSpawnRequest};
use std::collections::BTreeMap;
use std::fmt;
use std::path::{Path, PathBuf};
use std::time::Duration;

/// Caller-provided repository-bound directory proof used to prepare a launch.
///
/// The project subsystem remains responsible for canonical boundary resolution.
/// This type records the resolved result and lets command policy reject identity or
/// declaration crossing before execution.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandDirectoryBinding {
    repository_id: RepositoryId,
    declaration: CommandWorkingDirectory,
    canonical_path: PathBuf,
}

impl CommandDirectoryBinding {
    pub fn repository_root(
        repository_id: RepositoryId,
        canonical_path: impl Into<PathBuf>,
    ) -> Result<Self, CommandLaunchError> {
        Self::new(
            repository_id,
            CommandWorkingDirectory::repository_root(),
            canonical_path,
        )
    }

    pub fn relative(
        repository_id: RepositoryId,
        relative: RepositoryRelativePath,
        canonical_path: impl Into<PathBuf>,
    ) -> Result<Self, CommandLaunchError> {
        let declaration = CommandWorkingDirectory::relative(relative)
            .map_err(CommandLaunchError::InvalidDefinition)?;
        Self::new(repository_id, declaration, canonical_path)
    }

    fn new(
        repository_id: RepositoryId,
        declaration: CommandWorkingDirectory,
        canonical_path: impl Into<PathBuf>,
    ) -> Result<Self, CommandLaunchError> {
        let canonical_path = canonical_path.into();
        if !canonical_path.is_absolute() {
            return Err(CommandLaunchError::WorkingDirectoryNotAbsolute(
                canonical_path,
            ));
        }
        Ok(Self {
            repository_id,
            declaration,
            canonical_path,
        })
    }

    pub const fn repository_id(&self) -> RepositoryId {
        self.repository_id
    }

    pub fn declaration(&self) -> &CommandWorkingDirectory {
        &self.declaration
    }

    pub fn canonical_path(&self) -> &Path {
        &self.canonical_path
    }
}

/// One exact environment variable included in a launch payload.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct ResolvedCommandEnvironmentVariable {
    name: String,
    value: String,
}

impl ResolvedCommandEnvironmentVariable {
    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn value(&self) -> &str {
        &self.value
    }
}

/// Fully inspectable launch payload produced from one immutable command definition.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandLaunchPayload {
    command_id: CommandId,
    command_definition: ContentHash,
    repository_id: RepositoryId,
    process_request: ProcessSpawnRequest,
    working_directory: PathBuf,
    environment: Vec<ResolvedCommandEnvironmentVariable>,
    cancellation: CommandCancellationPolicy,
    authority: CommandAuthorityClass,
}

impl CommandLaunchPayload {
    pub fn prepare(
        process_id: ProcessId,
        command: &RegisteredCommand,
        directory: &CommandDirectoryBinding,
        declared_environment_source: &BTreeMap<String, String>,
    ) -> Result<Self, CommandLaunchError> {
        if directory.repository_id != command.repository_id() {
            return Err(CommandLaunchError::RepositoryMismatch {
                command: command.repository_id(),
                directory: directory.repository_id,
            });
        }
        if &directory.declaration != command.working_directory() {
            return Err(CommandLaunchError::WorkingDirectoryMismatch);
        }

        let mut environment = Vec::with_capacity(command.environment().variables().len());
        for variable in command.environment().variables() {
            let value = match variable.source() {
                CommandEnvironmentSource::Literal(value) => value.clone(),
                CommandEnvironmentSource::InheritDeclared => declared_environment_source
                    .get(variable.name())
                    .cloned()
                    .ok_or_else(|| {
                        CommandLaunchError::MissingDeclaredEnvironmentVariable(
                            variable.name().to_owned(),
                        )
                    })?,
            };
            variable
                .validate_resolved_value(&value)
                .map_err(CommandLaunchError::InvalidDefinition)?;
            environment.push(ResolvedCommandEnvironmentVariable {
                name: variable.name().to_owned(),
                value,
            });
        }

        let mut process_request = ProcessSpawnRequest::new(
            process_id,
            command.program(),
            command.arguments().iter().cloned(),
        )
        .map_err(CommandLaunchError::InvalidProcessRequest)?;
        if let CommandTimeout::Milliseconds(millis) = command.timeout() {
            process_request = process_request.with_timeout(Duration::from_millis(millis));
        }

        Ok(Self {
            command_id: command.command_id(),
            command_definition: command.definition_identity(),
            repository_id: command.repository_id(),
            process_request,
            working_directory: directory.canonical_path.clone(),
            environment,
            cancellation: command.cancellation(),
            authority: command.authority(),
        })
    }

    pub const fn command_id(&self) -> CommandId {
        self.command_id
    }

    pub const fn command_definition(&self) -> ContentHash {
        self.command_definition
    }

    pub const fn repository_id(&self) -> RepositoryId {
        self.repository_id
    }

    pub fn process_request(&self) -> &ProcessSpawnRequest {
        &self.process_request
    }

    pub fn working_directory(&self) -> &Path {
        &self.working_directory
    }

    /// Exact sorted environment supplied after clearing the parent environment.
    pub fn environment(&self) -> &[ResolvedCommandEnvironmentVariable] {
        &self.environment
    }

    pub const fn clears_parent_environment(&self) -> bool {
        true
    }

    pub const fn cancellation(&self) -> CommandCancellationPolicy {
        self.cancellation
    }

    pub const fn authority(&self) -> CommandAuthorityClass {
        self.authority
    }
}

/// Exact reason command launch preparation failed before any process was spawned.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CommandLaunchError {
    InvalidDefinition(forge_core::commands::CommandDefinitionError),
    InvalidProcessRequest(ProcessRequestError),
    WorkingDirectoryNotAbsolute(PathBuf),
    RepositoryMismatch {
        command: RepositoryId,
        directory: RepositoryId,
    },
    WorkingDirectoryMismatch,
    MissingDeclaredEnvironmentVariable(String),
}

impl fmt::Display for CommandLaunchError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "registered command launch rejected: {self:?}")
    }
}

impl std::error::Error for CommandLaunchError {}
