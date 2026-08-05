//! Canonical lifecycle state for one externally owned managed service.
//!
//! Native process mechanics and health probes are adapters. This module records
//! exact attempts, process identity, readiness, bounded restart decisions, and
//! clean stop state without inferring health from PID presence.

use crate::lifecycle::{ServiceFailure, ServiceFailureStage};
use crate::services::{ServiceName, StartupRestartPolicy};
use forge_protocol::identities::ProcessId;
use std::collections::BTreeSet;
use std::fmt;

/// Public readiness of a live service after its owning health contract responds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ManagedServiceReadiness {
    Ready,
    Degraded,
}

/// Canonical state for one externally owned service.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ManagedServiceRuntimeState {
    Stopped,
    StartRequested {
        attempt: u16,
    },
    Running {
        attempt: u16,
        process_id: ProcessId,
    },
    Ready {
        attempt: u16,
        process_id: ProcessId,
        readiness: ManagedServiceReadiness,
    },
    StopRequested {
        attempt: u16,
        process_id: ProcessId,
    },
    RestartPending {
        next_attempt: u16,
        failure: ServiceFailure,
    },
    Failed {
        failure: ServiceFailure,
    },
}

impl ManagedServiceRuntimeState {
    /// Exact live process identity, when a process is still owned by this state.
    pub const fn process_id(&self) -> Option<ProcessId> {
        match self {
            Self::Running { process_id, .. }
            | Self::Ready { process_id, .. }
            | Self::StopRequested { process_id, .. } => Some(*process_id),
            Self::Stopped
            | Self::StartRequested { .. }
            | Self::RestartPending { .. }
            | Self::Failed { .. } => None,
        }
    }
}

/// Bounded restart ledger for one service process.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ManagedServiceRuntime {
    service: ServiceName,
    restart_policy: StartupRestartPolicy,
    state: ManagedServiceRuntimeState,
    used_process_ids: BTreeSet<ProcessId>,
}

impl ManagedServiceRuntime {
    pub fn new(service: ServiceName, restart_policy: StartupRestartPolicy) -> Self {
        Self {
            service,
            restart_policy,
            state: ManagedServiceRuntimeState::Stopped,
            used_process_ids: BTreeSet::new(),
        }
    }

    pub fn service(&self) -> &ServiceName {
        &self.service
    }

    pub fn state(&self) -> &ManagedServiceRuntimeState {
        &self.state
    }

    pub fn has_used_process_id(&self, process_id: ProcessId) -> bool {
        self.used_process_ids.contains(&process_id)
    }

    /// Requests the next exact attempt. Duplicate live starts are rejected.
    pub fn request_start(&mut self) -> Result<u16, ManagedServiceRuntimeError> {
        let attempt = match &self.state {
            ManagedServiceRuntimeState::Stopped => 1,
            ManagedServiceRuntimeState::RestartPending { next_attempt, .. } => *next_attempt,
            state => {
                return Err(ManagedServiceRuntimeError::InvalidTransition {
                    operation: "request_start",
                    state: state.clone(),
                })
            }
        };
        self.state = ManagedServiceRuntimeState::StartRequested { attempt };
        Ok(attempt)
    }

    pub fn record_started(
        &mut self,
        attempt: u16,
        process_id: ProcessId,
    ) -> Result<(), ManagedServiceRuntimeError> {
        self.require_attempt("record_started", attempt, false)?;
        if !self.used_process_ids.insert(process_id) {
            return Err(ManagedServiceRuntimeError::ReusedProcessIdentity(
                process_id,
            ));
        }
        self.state = ManagedServiceRuntimeState::Running {
            attempt,
            process_id,
        };
        Ok(())
    }

    pub fn record_readiness(
        &mut self,
        attempt: u16,
        process_id: ProcessId,
        readiness: ManagedServiceReadiness,
    ) -> Result<(), ManagedServiceRuntimeError> {
        match &self.state {
            ManagedServiceRuntimeState::Running {
                attempt: expected,
                process_id: expected_process,
            } if *expected == attempt && *expected_process == process_id => {
                self.state = ManagedServiceRuntimeState::Ready {
                    attempt,
                    process_id,
                    readiness,
                };
                Ok(())
            }
            ManagedServiceRuntimeState::Running {
                attempt: expected,
                process_id: expected_process,
            } if *expected != attempt => Err(ManagedServiceRuntimeError::AttemptMismatch {
                expected: *expected,
                actual: attempt,
            }),
            ManagedServiceRuntimeState::Running {
                process_id: expected,
                ..
            } => Err(ManagedServiceRuntimeError::ProcessMismatch {
                expected: *expected,
                actual: process_id,
            }),
            state => Err(ManagedServiceRuntimeError::InvalidTransition {
                operation: "record_readiness",
                state: state.clone(),
            }),
        }
    }

    /// Records a spawn or readiness attempt that has already stopped cleanly.
    pub fn record_attempt_failed(
        &mut self,
        attempt: u16,
        failure: ServiceFailure,
    ) -> Result<(), ManagedServiceRuntimeError> {
        if !matches!(
            failure.stage(),
            ServiceFailureStage::Start | ServiceFailureStage::Readiness
        ) {
            return Err(ManagedServiceRuntimeError::FailureStage {
                actual: failure.stage(),
            });
        }
        self.require_attempt("record_attempt_failed", attempt, true)?;
        self.transition_after_failure(attempt, failure);
        Ok(())
    }

    /// Records an observed unexpected exit and applies the bounded restart policy.
    pub fn record_runtime_exit(
        &mut self,
        process_id: ProcessId,
        failure: ServiceFailure,
    ) -> Result<(), ManagedServiceRuntimeError> {
        if failure.stage() != ServiceFailureStage::Runtime {
            return Err(ManagedServiceRuntimeError::FailureStage {
                actual: failure.stage(),
            });
        }
        let attempt = match &self.state {
            ManagedServiceRuntimeState::Running {
                attempt,
                process_id: expected,
            }
            | ManagedServiceRuntimeState::Ready {
                attempt,
                process_id: expected,
                ..
            } if *expected == process_id => *attempt,
            ManagedServiceRuntimeState::Running {
                process_id: expected,
                ..
            }
            | ManagedServiceRuntimeState::Ready {
                process_id: expected,
                ..
            } => {
                return Err(ManagedServiceRuntimeError::ProcessMismatch {
                    expected: *expected,
                    actual: process_id,
                })
            }
            state => {
                return Err(ManagedServiceRuntimeError::InvalidTransition {
                    operation: "record_runtime_exit",
                    state: state.clone(),
                })
            }
        };
        self.transition_after_failure(attempt, failure);
        Ok(())
    }

    /// Requests clean logout stop for the exact live process.
    pub fn request_stop(&mut self) -> Result<ProcessId, ManagedServiceRuntimeError> {
        let (attempt, process_id) = match &self.state {
            ManagedServiceRuntimeState::Running {
                attempt,
                process_id,
            }
            | ManagedServiceRuntimeState::Ready {
                attempt,
                process_id,
                ..
            } => (*attempt, *process_id),
            state => {
                return Err(ManagedServiceRuntimeError::InvalidTransition {
                    operation: "request_stop",
                    state: state.clone(),
                })
            }
        };
        self.state = ManagedServiceRuntimeState::StopRequested {
            attempt,
            process_id,
        };
        Ok(process_id)
    }

    pub fn record_stopped(
        &mut self,
        process_id: ProcessId,
    ) -> Result<(), ManagedServiceRuntimeError> {
        match &self.state {
            ManagedServiceRuntimeState::StopRequested {
                process_id: expected,
                ..
            } if *expected == process_id => {
                self.state = ManagedServiceRuntimeState::Stopped;
                Ok(())
            }
            ManagedServiceRuntimeState::StopRequested {
                process_id: expected,
                ..
            } => Err(ManagedServiceRuntimeError::ProcessMismatch {
                expected: *expected,
                actual: process_id,
            }),
            state => Err(ManagedServiceRuntimeError::InvalidTransition {
                operation: "record_stopped",
                state: state.clone(),
            }),
        }
    }

    fn require_attempt(
        &self,
        operation: &'static str,
        actual: u16,
        allow_running: bool,
    ) -> Result<(), ManagedServiceRuntimeError> {
        let expected = match &self.state {
            ManagedServiceRuntimeState::StartRequested { attempt } => *attempt,
            ManagedServiceRuntimeState::Running { attempt, .. } if allow_running => *attempt,
            state => {
                return Err(ManagedServiceRuntimeError::InvalidTransition {
                    operation,
                    state: state.clone(),
                })
            }
        };
        if expected == actual {
            Ok(())
        } else {
            Err(ManagedServiceRuntimeError::AttemptMismatch { expected, actual })
        }
    }

    fn transition_after_failure(&mut self, attempt: u16, failure: ServiceFailure) {
        self.state = match self.restart_policy.next_attempt(attempt) {
            Some(next_attempt) => ManagedServiceRuntimeState::RestartPending {
                next_attempt,
                failure,
            },
            None => ManagedServiceRuntimeState::Failed { failure },
        };
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ManagedServiceRuntimeError {
    InvalidTransition {
        operation: &'static str,
        state: ManagedServiceRuntimeState,
    },
    AttemptMismatch {
        expected: u16,
        actual: u16,
    },
    ProcessMismatch {
        expected: ProcessId,
        actual: ProcessId,
    },
    FailureStage {
        actual: ServiceFailureStage,
    },
    ReusedProcessIdentity(ProcessId),
}

impl fmt::Display for ManagedServiceRuntimeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidTransition { operation, state } => {
                write!(
                    formatter,
                    "cannot {operation} while service state is {state:?}"
                )
            }
            Self::AttemptMismatch { expected, actual } => {
                write!(
                    formatter,
                    "service attempt {actual} does not match expected {expected}"
                )
            }
            Self::ProcessMismatch { expected, actual } => {
                write!(
                    formatter,
                    "service process {actual} does not match expected {expected}"
                )
            }
            Self::FailureStage { actual } => {
                write!(
                    formatter,
                    "service failure stage {actual:?} is invalid for this transition"
                )
            }
            Self::ReusedProcessIdentity(process_id) => {
                write!(
                    formatter,
                    "service process identity {process_id} was already used"
                )
            }
        }
    }
}

impl std::error::Error for ManagedServiceRuntimeError {}
