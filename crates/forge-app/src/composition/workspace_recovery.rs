//! Explicit durable workspace recovery composition.
//!
//! Forge Core owns the canonical payload, Forge Project owns atomic storage,
//! Forge Editor owns buffer state, Forge Terminal owns PTY truth, and Forge
//! Session owns service lifecycle. This adapter captures and restores only safe
//! state. It never replays an interrupted action or resurrects a process.

use crate::composition::editor_workspace::{EditorWorkspace, EditorWorkspaceError};
use crate::composition::nyx_service::ManagedNyxService;
use crate::composition::terminal_workspace::{
    ProjectTerminalWorkspace, ProjectTerminalWorkspaceError,
};
use forge_core::recovery::{
    InterruptedAction, RecordedProcess, RecoveredProcessState, WorkspaceRecoveryError,
    WorkspaceRecoveryRecord,
};
use forge_core::workspace_recovery::{
    DurableWorkspaceState, RecoveredService, RecoveredServiceState, RecoveredTerminal,
    RecoveredTerminalState, WorkspacePayloadError,
};
use forge_editor::buffers::{BufferId, SynchronizationState};
use forge_project::recovery_store::{
    RecoveryAssessment, RecoveryImageStatus, WorkspaceRecoveryStore, WorkspaceRecoveryStoreError,
};
use forge_protocol::identities::{ProjectId, RepositoryId, SessionId};
use std::fmt;

/// One explicit recovery result. Process-like entries are metadata only.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RestoredWorkspace {
    generation: u64,
    project_id: ProjectId,
    repository_id: RepositoryId,
    session_id: SessionId,
    buffer_states: Vec<(BufferId, SynchronizationState)>,
    terminals: Vec<RecoveredTerminal>,
    services: Vec<RecoveredService>,
    interrupted_actions: Vec<InterruptedAction>,
}

impl RestoredWorkspace {
    pub const fn generation(&self) -> u64 {
        self.generation
    }
    pub const fn project_id(&self) -> ProjectId {
        self.project_id
    }
    pub const fn repository_id(&self) -> RepositoryId {
        self.repository_id
    }
    pub const fn session_id(&self) -> SessionId {
        self.session_id
    }
    pub fn buffer_states(&self) -> &[(BufferId, SynchronizationState)] {
        &self.buffer_states
    }
    pub fn terminals(&self) -> &[RecoveredTerminal] {
        &self.terminals
    }
    pub fn services(&self) -> &[RecoveredService] {
        &self.services
    }
    pub fn interrupted_actions(&self) -> &[InterruptedAction] {
        &self.interrupted_actions
    }
    pub fn requires_operator_attention(&self) -> bool {
        !self.interrupted_actions.is_empty()
            || self
                .buffer_states
                .iter()
                .any(|(_, state)| matches!(state, SynchronizationState::Conflict { .. }))
            || self
                .terminals
                .iter()
                .any(|terminal| matches!(terminal.state(), RecoveredTerminalState::RequiresRestart))
            || self
                .services
                .iter()
                .any(|service| !matches!(service.state(), RecoveredServiceState::Stopped))
    }
}

/// Project/session-bound recovery coordinator.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceRecoveryCoordinator {
    project_id: ProjectId,
    repository_id: RepositoryId,
    session_id: SessionId,
    store: WorkspaceRecoveryStore,
}

impl WorkspaceRecoveryCoordinator {
    pub fn new(
        project_id: ProjectId,
        repository_id: RepositoryId,
        session_id: SessionId,
        store: WorkspaceRecoveryStore,
    ) -> Self {
        Self {
            project_id,
            repository_id,
            session_id,
            store,
        }
    }

    pub fn assess(&self) -> Result<RecoveryAssessment, WorkspaceRecoveryCoordinatorError> {
        self.store.assess().map_err(Into::into)
    }

    /// Explicitly discards only abandoned staged publication bytes. A valid
    /// current image must already exist, so this cannot erase the last safe state.
    pub fn discard_interrupted_write(&self) -> Result<bool, WorkspaceRecoveryCoordinatorError> {
        self.store.discard_interrupted_write().map_err(Into::into)
    }

    /// Explicitly promotes the validated previous image only when the current
    /// image is missing or invalid. The restored record is rechecked against
    /// this coordinator before it can be used.
    pub fn restore_previous_if_current_unusable(
        &self,
    ) -> Result<WorkspaceRecoveryRecord, WorkspaceRecoveryCoordinatorError> {
        let record = self.store.restore_previous_if_current_unusable()?;
        self.require_record_identity(&record)?;
        let durable = DurableWorkspaceState::from_safe_snapshot(record.safe_snapshot())?;
        self.require_payload_identity(&durable)?;
        Ok(record)
    }

    /// Publishes one exact next generation. Pending saves must already be
    /// represented as interrupted actions or capture is rejected by the editor.
    pub fn capture(
        &self,
        editor: &EditorWorkspace,
        terminals: &ProjectTerminalWorkspace,
        nyx: &ManagedNyxService,
        interrupted_actions: Vec<InterruptedAction>,
    ) -> Result<WorkspaceRecoveryRecord, WorkspaceRecoveryCoordinatorError> {
        self.require_bindings(editor, terminals, nyx)?;
        let services = vec![nyx.recovery_service()?];
        let durable = DurableWorkspaceState::new(
            self.project_id,
            self.repository_id,
            self.session_id,
            editor.recovery_buffers()?,
            terminals.recovery_terminals()?,
            services.clone(),
        )?;
        let recorded_processes = services
            .iter()
            .map(recorded_process)
            .collect::<Result<Vec<_>, _>>()?;
        let assessment = self.store.assess()?;
        let generation = match assessment.current() {
            RecoveryImageStatus::Missing => 1,
            RecoveryImageStatus::Valid { generation, .. } => generation
                .checked_add(1)
                .ok_or(WorkspaceRecoveryCoordinatorError::GenerationOverflow)?,
            RecoveryImageStatus::Invalid => {
                return Err(WorkspaceRecoveryCoordinatorError::CurrentImageInvalid);
            }
        };
        let record = WorkspaceRecoveryRecord::new(
            generation,
            self.project_id,
            Some(self.session_id),
            durable.to_safe_snapshot()?,
            interrupted_actions,
            recorded_processes,
        )?;
        if generation == 1 {
            self.store.create(&record)?;
        } else {
            self.store.publish_next(generation - 1, &record)?;
        }
        Ok(record)
    }

    /// Restores unsaved editor bytes and returns terminal/service metadata for
    /// explicit UI presentation. It does not spawn or replay anything.
    pub fn restore(
        &self,
        editor: &mut EditorWorkspace,
    ) -> Result<RestoredWorkspace, WorkspaceRecoveryCoordinatorError> {
        let opened = self.store.open_current()?;
        if opened.interrupted_write_present() {
            return Err(WorkspaceRecoveryCoordinatorError::InterruptedWritePresent);
        }
        let record = opened.into_record();
        self.require_record_identity(&record)?;
        let durable = DurableWorkspaceState::from_safe_snapshot(record.safe_snapshot())?;
        self.require_payload_identity(&durable)?;
        let buffer_states = editor.restore_recovered_buffers(durable.buffers())?;
        Ok(RestoredWorkspace {
            generation: record.generation(),
            project_id: self.project_id,
            repository_id: self.repository_id,
            session_id: self.session_id,
            buffer_states,
            terminals: durable.terminals().to_vec(),
            services: durable.services().to_vec(),
            interrupted_actions: record.interrupted_actions().to_vec(),
        })
    }

    fn require_record_identity(
        &self,
        record: &WorkspaceRecoveryRecord,
    ) -> Result<(), WorkspaceRecoveryCoordinatorError> {
        if record.project_id() != self.project_id {
            return Err(WorkspaceRecoveryCoordinatorError::ProjectMismatch {
                expected: self.project_id,
                found: record.project_id(),
            });
        }
        if record.session_id() != Some(self.session_id) {
            return Err(WorkspaceRecoveryCoordinatorError::SessionMismatch {
                expected: self.session_id,
                found: record.session_id(),
            });
        }
        Ok(())
    }

    fn require_payload_identity(
        &self,
        durable: &DurableWorkspaceState,
    ) -> Result<(), WorkspaceRecoveryCoordinatorError> {
        if durable.project_id() != self.project_id {
            return Err(WorkspaceRecoveryCoordinatorError::ProjectMismatch {
                expected: self.project_id,
                found: durable.project_id(),
            });
        }
        if durable.repository_id() != self.repository_id {
            return Err(WorkspaceRecoveryCoordinatorError::RepositoryMismatch {
                expected: self.repository_id,
                found: durable.repository_id(),
            });
        }
        if durable.session_id() != self.session_id {
            return Err(WorkspaceRecoveryCoordinatorError::SessionMismatch {
                expected: self.session_id,
                found: Some(durable.session_id()),
            });
        }
        Ok(())
    }

    fn require_bindings(
        &self,
        editor: &EditorWorkspace,
        terminals: &ProjectTerminalWorkspace,
        nyx: &ManagedNyxService,
    ) -> Result<(), WorkspaceRecoveryCoordinatorError> {
        if editor.project_id() != self.project_id {
            return Err(WorkspaceRecoveryCoordinatorError::ProjectMismatch {
                expected: self.project_id,
                found: editor.project_id(),
            });
        }
        if editor.repository_id() != self.repository_id {
            return Err(WorkspaceRecoveryCoordinatorError::RepositoryMismatch {
                expected: self.repository_id,
                found: editor.repository_id(),
            });
        }
        if terminals.project_id() != self.project_id {
            return Err(WorkspaceRecoveryCoordinatorError::ProjectMismatch {
                expected: self.project_id,
                found: terminals.project_id(),
            });
        }
        if terminals.repository_id() != self.repository_id {
            return Err(WorkspaceRecoveryCoordinatorError::RepositoryMismatch {
                expected: self.repository_id,
                found: terminals.repository_id(),
            });
        }
        if nyx.session_id() != self.session_id {
            return Err(WorkspaceRecoveryCoordinatorError::SessionMismatch {
                expected: self.session_id,
                found: Some(nyx.session_id()),
            });
        }
        Ok(())
    }
}

fn recorded_process(service: &RecoveredService) -> Result<RecordedProcess, WorkspaceRecoveryError> {
    match service.state() {
        RecoveredServiceState::RequiresRevalidation => RecordedProcess::new(
            service.name(),
            service.prior_process_id(),
            RecoveredProcessState::RequiresRevalidation,
        ),
        RecoveredServiceState::Stopped
        | RecoveredServiceState::RestartPending
        | RecoveredServiceState::Failed => RecordedProcess::new(
            service.name(),
            None,
            RecoveredProcessState::ConfirmedStopped,
        ),
    }
}

/// Exact recovery-composition failure.
#[derive(Debug)]
pub enum WorkspaceRecoveryCoordinatorError {
    Store(WorkspaceRecoveryStoreError),
    Record(WorkspaceRecoveryError),
    Payload(WorkspacePayloadError),
    Editor(EditorWorkspaceError),
    Terminal(ProjectTerminalWorkspaceError),
    GenerationOverflow,
    CurrentImageInvalid,
    InterruptedWritePresent,
    ProjectMismatch {
        expected: ProjectId,
        found: ProjectId,
    },
    RepositoryMismatch {
        expected: RepositoryId,
        found: RepositoryId,
    },
    SessionMismatch {
        expected: SessionId,
        found: Option<SessionId>,
    },
}

impl fmt::Display for WorkspaceRecoveryCoordinatorError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Store(error) => error.fmt(formatter),
            Self::Record(error) => error.fmt(formatter),
            Self::Payload(error) => error.fmt(formatter),
            Self::Editor(error) => error.fmt(formatter),
            Self::Terminal(error) => error.fmt(formatter),
            Self::GenerationOverflow => formatter.write_str("recovery generation exhausted"),
            Self::CurrentImageInvalid => formatter
                .write_str("current recovery image is invalid and requires explicit resolution"),
            Self::InterruptedWritePresent => {
                formatter.write_str("interrupted recovery publication must be resolved first")
            }
            Self::ProjectMismatch { expected, found } => {
                write!(
                    formatter,
                    "recovery project {found} does not match {expected}"
                )
            }
            Self::RepositoryMismatch { expected, found } => {
                write!(
                    formatter,
                    "recovery repository {found} does not match {expected}"
                )
            }
            Self::SessionMismatch { expected, found } => {
                write!(
                    formatter,
                    "recovery session {found:?} does not match {expected}"
                )
            }
        }
    }
}
impl std::error::Error for WorkspaceRecoveryCoordinatorError {}

impl From<WorkspaceRecoveryStoreError> for WorkspaceRecoveryCoordinatorError {
    fn from(error: WorkspaceRecoveryStoreError) -> Self {
        Self::Store(error)
    }
}
impl From<WorkspaceRecoveryError> for WorkspaceRecoveryCoordinatorError {
    fn from(error: WorkspaceRecoveryError) -> Self {
        Self::Record(error)
    }
}
impl From<WorkspacePayloadError> for WorkspaceRecoveryCoordinatorError {
    fn from(error: WorkspacePayloadError) -> Self {
        Self::Payload(error)
    }
}
impl From<EditorWorkspaceError> for WorkspaceRecoveryCoordinatorError {
    fn from(error: EditorWorkspaceError) -> Self {
        Self::Editor(error)
    }
}
impl From<ProjectTerminalWorkspaceError> for WorkspaceRecoveryCoordinatorError {
    fn from(error: ProjectTerminalWorkspaceError) -> Self {
        Self::Terminal(error)
    }
}
