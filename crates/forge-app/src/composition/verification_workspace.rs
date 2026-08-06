//! Project-bound validation execution and immutable result history.
//!
//! Native Git supplies exact source state, Forge Terminal supplies exact process
//! execution, and Forge Core owns the canonical verification record. This adapter
//! joins those authorities without treating stale or interrupted runs as current.

use crate::composition::command_workspace::{
    ProjectCommandWorkspace, ProjectCommandWorkspaceError,
};
use crate::composition::git_workspace::{
    ProjectGitSnapshot, ProjectGitWorkspace, ProjectGitWorkspaceError,
};
use forge_bridge::processes::CancellationToken;
use forge_core::commands::RegisteredCommand;
use forge_core::projects::ProjectManifest;
use forge_core::state::StateRecord;
use forge_core::verification::{
    VerificationLedger, VerificationLedgerError, VerificationOutcome, VerificationOutputReference,
    VerificationRecord, VerificationRecordError, VerificationSourceState,
};
use forge_project::paths::RepositoryBoundary;
use forge_protocol::hashes::{ContentHash, HashDomain, hash_canonical_bytes};
use forge_protocol::identities::{CommandId, ProcessId, ProjectId, RepositoryId};
use forge_protocol::processes::ProcessOutcome;
use forge_terminal::execution::CommandRunRecord;
use std::collections::BTreeMap;
use std::fmt;

/// Whether one historical record can satisfy the repository state visible now.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VerificationApplicability {
    CurrentPass,
    CurrentNonPassing,
    StaleSource,
}

/// Product composition for version-bound command validation and immutable history.
pub struct ProjectVerificationWorkspace {
    git: ProjectGitWorkspace,
    commands: ProjectCommandWorkspace,
    records: VerificationLedger,
}

impl ProjectVerificationWorkspace {
    pub fn new(
        manifest: &ProjectManifest,
        boundary: RepositoryBoundary,
        commands: impl IntoIterator<Item = RegisteredCommand>,
        declared_environment: BTreeMap<String, String>,
    ) -> Result<Self, ProjectVerificationWorkspaceError> {
        let git = ProjectGitWorkspace::new(manifest, boundary.clone())?;
        let initial = source_state(&git.inspect()?)?;
        let commands = ProjectCommandWorkspace::new(
            manifest,
            boundary,
            initial.revision_identity(),
            commands,
            declared_environment,
        )?;
        Ok(Self {
            git,
            commands,
            records: VerificationLedger::new(),
        })
    }

    pub const fn project_id(&self) -> ProjectId {
        self.commands.project_id()
    }

    pub const fn repository_id(&self) -> RepositoryId {
        self.commands.repository_id()
    }

    pub fn records(&self) -> &VerificationLedger {
        &self.records
    }

    pub fn record(&self, identity: ContentHash) -> Option<&VerificationRecord> {
        self.records.get(identity)
    }

    /// Runs one exact registered command between two consistency-checked Git views.
    pub fn run(
        &mut self,
        command_id: CommandId,
        expected_definition: ContentHash,
        process_id: ProcessId,
        cancellation: &CancellationToken,
    ) -> Result<&VerificationRecord, ProjectVerificationWorkspaceError> {
        let start = source_state(&self.git.inspect()?)?;
        if self.commands.revision() != start.revision_identity() {
            return Err(ProjectVerificationWorkspaceError::StaleCommandRevision {
                configured: self.commands.revision(),
                current: start.revision_identity(),
            });
        }
        let run = self
            .commands
            .run(command_id, expected_definition, process_id, cancellation)?
            .clone();
        validate_run_source(&run, &start)?;
        let end = source_state(&self.git.inspect()?)?;
        let record = verification_record(&run, start, end)?;
        let identity = record.identity();
        self.records.record(record)?;
        self.records
            .get(identity)
            .ok_or(ProjectVerificationWorkspaceError::MissingInsertedRecord(
                identity,
            ))
    }

    /// Rechecks one historical record against the exact current project source state.
    pub fn applicability(
        &self,
        identity: ContentHash,
    ) -> Result<VerificationApplicability, ProjectVerificationWorkspaceError> {
        let record = self
            .records
            .get(identity)
            .ok_or(ProjectVerificationWorkspaceError::UnknownRecord(identity))?;
        let current = source_state(&self.git.inspect()?)?;
        if record.end_state() != &current {
            return Ok(VerificationApplicability::StaleSource);
        }
        if record.outcome().is_pass() {
            Ok(VerificationApplicability::CurrentPass)
        } else {
            Ok(VerificationApplicability::CurrentNonPassing)
        }
    }

    pub fn history_state_record(&self) -> Result<StateRecord, ProjectVerificationWorkspaceError> {
        self.records.state_record().map_err(Into::into)
    }

    /// Restores history only into an empty workspace and only for this project scope.
    pub fn restore_history(
        &mut self,
        state: &StateRecord,
    ) -> Result<(), ProjectVerificationWorkspaceError> {
        if !self.records.is_empty() {
            return Err(ProjectVerificationWorkspaceError::HistoryAlreadyInitialized);
        }
        let restored = VerificationLedger::from_state_record(state)?;
        for record in restored.iter() {
            let source = record.start_state();
            if source.project_id() != self.project_id()
                || source.repository_id() != self.repository_id()
            {
                return Err(ProjectVerificationWorkspaceError::HistoryScopeMismatch {
                    expected_project: self.project_id(),
                    found_project: source.project_id(),
                    expected_repository: self.repository_id(),
                    found_repository: source.repository_id(),
                });
            }
        }
        self.records = restored;
        Ok(())
    }
}

#[derive(Debug)]
pub enum ProjectVerificationWorkspaceError {
    Git(ProjectGitWorkspaceError),
    Command(ProjectCommandWorkspaceError),
    Record(VerificationRecordError),
    Ledger(VerificationLedgerError),
    MissingRevision,
    StaleCommandRevision {
        configured: ContentHash,
        current: ContentHash,
    },
    RunSourceMismatch,
    MissingInsertedRecord(ContentHash),
    UnknownRecord(ContentHash),
    HistoryAlreadyInitialized,
    HistoryScopeMismatch {
        expected_project: ProjectId,
        found_project: ProjectId,
        expected_repository: RepositoryId,
        found_repository: RepositoryId,
    },
}

impl fmt::Display for ProjectVerificationWorkspaceError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "project verification rejected: {self:?}")
    }
}

impl std::error::Error for ProjectVerificationWorkspaceError {}

impl From<ProjectGitWorkspaceError> for ProjectVerificationWorkspaceError {
    fn from(error: ProjectGitWorkspaceError) -> Self {
        Self::Git(error)
    }
}

impl From<ProjectCommandWorkspaceError> for ProjectVerificationWorkspaceError {
    fn from(error: ProjectCommandWorkspaceError) -> Self {
        Self::Command(error)
    }
}

impl From<VerificationRecordError> for ProjectVerificationWorkspaceError {
    fn from(error: VerificationRecordError) -> Self {
        Self::Record(error)
    }
}

impl From<VerificationLedgerError> for ProjectVerificationWorkspaceError {
    fn from(error: VerificationLedgerError) -> Self {
        Self::Ledger(error)
    }
}

fn source_state(
    snapshot: &ProjectGitSnapshot,
) -> Result<VerificationSourceState, ProjectVerificationWorkspaceError> {
    let revision = snapshot
        .inspection()
        .status()
        .head()
        .revision()
        .ok_or(ProjectVerificationWorkspaceError::MissingRevision)?;
    VerificationSourceState::new(
        snapshot.project_id(),
        snapshot.repository_id(),
        revision.as_str().as_bytes().to_vec(),
        snapshot.inspection().identity(),
    )
    .map_err(Into::into)
}

fn validate_run_source(
    run: &CommandRunRecord,
    start: &VerificationSourceState,
) -> Result<(), ProjectVerificationWorkspaceError> {
    let source = run.source();
    if source.project_id() != start.project_id()
        || source.repository_id() != start.repository_id()
        || source.revision() != start.revision_identity()
    {
        return Err(ProjectVerificationWorkspaceError::RunSourceMismatch);
    }
    Ok(())
}

fn verification_record(
    run: &CommandRunRecord,
    start: VerificationSourceState,
    end: VerificationSourceState,
) -> Result<VerificationRecord, VerificationRecordError> {
    let execution = run.execution();
    let outcome = match execution.outcome() {
        ProcessOutcome::Exited(exit) if exit.success() => VerificationOutcome::Passed {
            exit_code: exit.code(),
        },
        ProcessOutcome::Exited(exit) => VerificationOutcome::Failed {
            exit_code: exit.code(),
        },
        ProcessOutcome::TimedOut => VerificationOutcome::TimedOut,
        ProcessOutcome::Cancelled => VerificationOutcome::Cancelled,
        ProcessOutcome::Failed(failure) => VerificationOutcome::ExecutionFailed {
            stage: failure.stage(),
            message_identity: hash_canonical_bytes(
                HashDomain::ResultPayload,
                failure.message().as_bytes(),
            ),
        },
    };
    let output = VerificationOutputReference::from_output(
        execution.output().stdout(),
        execution.output().stderr(),
    );
    VerificationRecord::new(
        run.command_id(),
        run.command_definition(),
        run.process_id(),
        run.process_request().program().to_owned(),
        run.process_request().arguments().iter().cloned(),
        start,
        end,
        outcome,
        output,
    )
}
