//! Registered-command execution and exact project-bound history.
//!
//! Only immutable [`RegisteredCommand`] definitions may enter this runner. The
//! operating-system process adapter owns native execution; this layer binds the
//! result to the exact project, repository, revision, definition, and process IDs.

use crate::commands::{
    CommandDirectoryBinding, CommandLaunchError, CommandLaunchPayload,
    ResolvedCommandEnvironmentVariable,
};
use forge_bridge::processes::{CancellationToken, ProcessExecutionContext, ProcessRunner};
use forge_core::commands::{CommandAuthorityClass, CommandCancellationPolicy, RegisteredCommand};
use forge_protocol::hashes::ContentHash;
use forge_protocol::identities::{CommandId, ProcessId, ProjectId, RepositoryId};
use forge_protocol::processes::{ProcessExecution, ProcessOutputChunk, ProcessSpawnRequest};
use std::collections::{BTreeMap, btree_map::Entry};
use std::fmt;
use std::path::Path;

/// Source binding retained for every registered command execution.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CommandSourceBinding {
    project_id: ProjectId,
    repository_id: RepositoryId,
    revision: ContentHash,
}

impl CommandSourceBinding {
    pub const fn new(
        project_id: ProjectId,
        repository_id: RepositoryId,
        revision: ContentHash,
    ) -> Self {
        Self {
            project_id,
            repository_id,
            revision,
        }
    }

    pub const fn project_id(self) -> ProjectId {
        self.project_id
    }

    pub const fn repository_id(self) -> RepositoryId {
        self.repository_id
    }

    pub const fn revision(self) -> ContentHash {
        self.revision
    }
}

/// Complete immutable history record for one command process.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandRunRecord {
    source: CommandSourceBinding,
    command_id: CommandId,
    command_definition: ContentHash,
    process_id: ProcessId,
    process_request: ProcessSpawnRequest,
    working_directory: std::path::PathBuf,
    environment: Vec<ResolvedCommandEnvironmentVariable>,
    authority: CommandAuthorityClass,
    cancellation: CommandCancellationPolicy,
    execution: ProcessExecution,
}

impl CommandRunRecord {
    pub const fn source(&self) -> CommandSourceBinding {
        self.source
    }

    pub const fn command_id(&self) -> CommandId {
        self.command_id
    }

    pub const fn command_definition(&self) -> ContentHash {
        self.command_definition
    }

    pub const fn process_id(&self) -> ProcessId {
        self.process_id
    }

    pub fn process_request(&self) -> &ProcessSpawnRequest {
        &self.process_request
    }

    pub fn working_directory(&self) -> &Path {
        &self.working_directory
    }

    pub fn environment(&self) -> &[ResolvedCommandEnvironmentVariable] {
        &self.environment
    }

    pub const fn clears_parent_environment(&self) -> bool {
        true
    }

    pub const fn authority(&self) -> CommandAuthorityClass {
        self.authority
    }

    pub const fn cancellation(&self) -> CommandCancellationPolicy {
        self.cancellation
    }

    pub fn execution(&self) -> &ProcessExecution {
        &self.execution
    }
}

/// Typed command-run failure before or after native execution.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CommandRunError {
    RepositoryMismatch {
        source: RepositoryId,
        command: RepositoryId,
    },
    DuplicateProcess(ProcessId),
    Launch(CommandLaunchError),
}

impl fmt::Display for CommandRunError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "registered command execution rejected: {self:?}")
    }
}

impl std::error::Error for CommandRunError {}

impl From<CommandLaunchError> for CommandRunError {
    fn from(error: CommandLaunchError) -> Self {
        Self::Launch(error)
    }
}

/// In-memory exact command history for one product composition owner.
#[derive(Debug, Clone, Default)]
pub struct CommandRunRegistry {
    runner: ProcessRunner,
    records: BTreeMap<ProcessId, CommandRunRecord>,
}

impl CommandRunRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_runner(runner: ProcessRunner) -> Self {
        Self {
            runner,
            records: BTreeMap::new(),
        }
    }

    /// Runs one registered command and records its terminal outcome and exact output.
    pub fn run(
        &mut self,
        source: CommandSourceBinding,
        process_id: ProcessId,
        command: &RegisteredCommand,
        directory: &CommandDirectoryBinding,
        declared_environment: &BTreeMap<String, String>,
        cancellation: &CancellationToken,
    ) -> Result<&CommandRunRecord, CommandRunError> {
        self.run_with_output(
            source,
            process_id,
            command,
            directory,
            declared_environment,
            cancellation,
            |_| {},
        )
    }

    /// Runs one command while forwarding live native output chunks to the caller.
    #[allow(clippy::too_many_arguments)]
    pub fn run_with_output<F>(
        &mut self,
        source: CommandSourceBinding,
        process_id: ProcessId,
        command: &RegisteredCommand,
        directory: &CommandDirectoryBinding,
        declared_environment: &BTreeMap<String, String>,
        cancellation: &CancellationToken,
        observe: F,
    ) -> Result<&CommandRunRecord, CommandRunError>
    where
        F: FnMut(&ProcessOutputChunk),
    {
        if source.repository_id() != command.repository_id() {
            return Err(CommandRunError::RepositoryMismatch {
                source: source.repository_id(),
                command: command.repository_id(),
            });
        }
        if self.records.contains_key(&process_id) {
            return Err(CommandRunError::DuplicateProcess(process_id));
        }

        let payload =
            CommandLaunchPayload::prepare(process_id, command, directory, declared_environment)?;
        let context = ProcessExecutionContext::new(payload.working_directory()).with_environment(
            payload
                .environment()
                .iter()
                .map(|variable| (variable.name(), variable.value())),
        );
        let process_request = payload.process_request().clone();
        let execution = self.runner.run_configured_with_output(
            process_request.clone(),
            &context,
            cancellation,
            observe,
        );
        let record = CommandRunRecord {
            source,
            command_id: payload.command_id(),
            command_definition: payload.command_definition(),
            process_id,
            process_request,
            working_directory: payload.working_directory().to_path_buf(),
            environment: payload.environment().to_vec(),
            authority: payload.authority(),
            cancellation: payload.cancellation(),
            execution,
        };
        match self.records.entry(process_id) {
            Entry::Vacant(entry) => Ok(entry.insert(record)),
            Entry::Occupied(_) => Err(CommandRunError::DuplicateProcess(process_id)),
        }
    }

    pub fn get(&self, process_id: ProcessId) -> Option<&CommandRunRecord> {
        self.records.get(&process_id)
    }

    pub fn iter(&self) -> impl ExactSizeIterator<Item = &CommandRunRecord> {
        self.records.values()
    }

    pub fn len(&self) -> usize {
        self.records.len()
    }

    pub fn is_empty(&self) -> bool {
        self.records.is_empty()
    }
}
