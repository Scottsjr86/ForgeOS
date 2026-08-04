//! Conservative crash evidence derived from the managed-session state machine.
//!
//! Historical process IDs are retained only for revalidation. Interrupted start
//! and stop requests become non-replayable journal entries.

use forge_core::recovery::{InterruptedAction, RecordedProcess};
use forge_protocol::identities::SessionId;

/// Recovery evidence exported from one session supervisor without side effects.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionRecoveryEvidence {
    session_id: SessionId,
    interrupted_actions: Vec<InterruptedAction>,
    recorded_processes: Vec<RecordedProcess>,
}

impl SessionRecoveryEvidence {
    pub(crate) fn new(
        session_id: SessionId,
        interrupted_actions: Vec<InterruptedAction>,
        recorded_processes: Vec<RecordedProcess>,
    ) -> Self {
        Self {
            session_id,
            interrupted_actions,
            recorded_processes,
        }
    }

    pub const fn session_id(&self) -> SessionId {
        self.session_id
    }

    pub fn interrupted_actions(&self) -> &[InterruptedAction] {
        &self.interrupted_actions
    }

    pub fn recorded_processes(&self) -> &[RecordedProcess] {
        &self.recorded_processes
    }
}
