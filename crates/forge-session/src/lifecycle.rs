//! Explicit session and managed-service lifecycle state machine.
//!
//! The supervisor emits actions and accepts exact outcomes. It never guesses process
//! identity, polls by process name, sleeps for readiness, or silently reorders the
//! declared service plan.

use crate::services::{ServiceName, ServicePlan};
use forge_protocol::identities::{ProcessId, SessionId};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

/// Current high-level session lifecycle phase.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionPhase {
    Starting,
    Ready,
    Stopping,
    Stopped,
    Failed,
}

/// Exact failure stage reported by a managed-service adapter.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ServiceFailureStage {
    Start,
    Readiness,
    Runtime,
    Stop,
}

/// Preserved typed service failure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServiceFailure {
    stage: ServiceFailureStage,
    native_message: String,
}

impl ServiceFailure {
    /// Creates a typed failure while preserving native diagnostic text.
    pub fn new(stage: ServiceFailureStage, native_message: impl Into<String>) -> Self {
        Self {
            stage,
            native_message: native_message.into(),
        }
    }

    /// Failure stage.
    pub const fn stage(&self) -> ServiceFailureStage {
        self.stage
    }

    /// Preserved native diagnostic text.
    pub fn native_message(&self) -> &str {
        &self.native_message
    }
}

/// Why the supervisor requests a service stop.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StopReason {
    Shutdown,
    RestartCleanup,
    StartupRollback,
}

/// One exact action for the runtime adapter or operator harness.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LifecycleAction {
    Start {
        session_id: SessionId,
        service: ServiceName,
        attempt: u16,
    },
    Stop {
        session_id: SessionId,
        service: ServiceName,
        process_id: ProcessId,
        reason: StopReason,
    },
    SessionReady {
        session_id: SessionId,
    },
    SessionStopped {
        session_id: SessionId,
    },
    SessionFailed {
        session_id: SessionId,
        service: ServiceName,
        failure: ServiceFailure,
    },
}

/// Public view of one service's runtime state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ServiceStatus {
    Pending { next_attempt: u16 },
    StartRequested { attempt: u16 },
    Running { attempt: u16, process_id: ProcessId },
    Ready { attempt: u16, process_id: ProcessId },
    StopRequested {
        attempt: u16,
        process_id: ProcessId,
        reason: StopReason,
    },
    Stopped,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum StopContinuation {
    Complete,
    Retry { next_attempt: u16 },
    FailAfterStop { failure: ServiceFailure },
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum RuntimeState {
    Pending { next_attempt: u16 },
    StartRequested { attempt: u16 },
    Running { attempt: u16, process_id: ProcessId },
    Ready { attempt: u16, process_id: ProcessId },
    StopRequested {
        attempt: u16,
        process_id: ProcessId,
        reason: StopReason,
        continuation: StopContinuation,
    },
    Stopped,
    Failed { failure: ServiceFailure },
}

impl RuntimeState {
    fn public_status(&self) -> ServiceStatus {
        match self {
            Self::Pending { next_attempt } => ServiceStatus::Pending {
                next_attempt: *next_attempt,
            },
            Self::StartRequested { attempt } => ServiceStatus::StartRequested {
                attempt: *attempt,
            },
            Self::Running {
                attempt,
                process_id,
            } => ServiceStatus::Running {
                attempt: *attempt,
                process_id: *process_id,
            },
            Self::Ready {
                attempt,
                process_id,
            } => ServiceStatus::Ready {
                attempt: *attempt,
                process_id: *process_id,
            },
            Self::StopRequested {
                attempt,
                process_id,
                reason,
                ..
            } => ServiceStatus::StopRequested {
                attempt: *attempt,
                process_id: *process_id,
                reason: *reason,
            },
            Self::Stopped => ServiceStatus::Stopped,
            Self::Failed { .. } => ServiceStatus::Failed,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct PrimaryFailure {
    service: ServiceName,
    failure: ServiceFailure,
}

/// Deterministic managed-session supervisor.
#[derive(Debug)]
pub struct SessionSupervisor {
    session_id: SessionId,
    plan: ServicePlan,
    phase: SessionPhase,
    states: BTreeMap<ServiceName, RuntimeState>,
    pending_action: Option<LifecycleAction>,
    primary_failure: Option<PrimaryFailure>,
    stop_reason: StopReason,
}

impl SessionSupervisor {
    /// Creates a new supervisor in the `Starting` phase.
    pub fn new(session_id: SessionId, plan: ServicePlan) -> Self {
        let states = plan
            .startup_order()
            .iter()
            .cloned()
            .map(|name| (name, RuntimeState::Pending { next_attempt: 1 }))
            .collect();
        Self {
            session_id,
            plan,
            phase: SessionPhase::Starting,
            states,
            pending_action: None,
            primary_failure: None,
            stop_reason: StopReason::Shutdown,
        }
    }

    /// Stable session identity.
    pub const fn session_id(&self) -> SessionId {
        self.session_id
    }

    /// Validated immutable service plan.
    pub const fn plan(&self) -> &ServicePlan {
        &self.plan
    }

    /// Current high-level phase.
    pub const fn phase(&self) -> SessionPhase {
        self.phase
    }

    /// Current public service state.
    pub fn service_status(&self, service: &ServiceName) -> Option<ServiceStatus> {
        self.states.get(service).map(RuntimeState::public_status)
    }

    /// Preserved terminal failure for one failed service, when present.
    pub fn service_failure(&self, service: &ServiceName) -> Option<&ServiceFailure> {
        match self.states.get(service) {
            Some(RuntimeState::Failed { failure }) => Some(failure),
            _ => None,
        }
    }

    /// Emits the next exact lifecycle action, or `None` while awaiting an outcome.
    pub fn take_next_action(&mut self) -> Option<LifecycleAction> {
        if let Some(action) = self.pending_action.take() {
            return Some(action);
        }

        match self.phase {
            SessionPhase::Starting => self.next_start_action(),
            SessionPhase::Ready => None,
            SessionPhase::Stopping => self.next_stop_action(),
            SessionPhase::Stopped | SessionPhase::Failed => None,
        }
    }

    fn next_start_action(&mut self) -> Option<LifecycleAction> {
        if self.states.values().any(|state| {
            matches!(
                state,
                RuntimeState::StartRequested { .. }
                    | RuntimeState::Running { .. }
                    | RuntimeState::StopRequested { .. }
            )
        }) {
            return None;
        }

        let ready = self.ready_services();
        for service in self.plan.startup_order() {
            let next_attempt = match self.states.get(service) {
                Some(RuntimeState::Pending { next_attempt }) => *next_attempt,
                _ => continue,
            };
            if !self.plan.dependencies_ready(service, &ready) {
                continue;
            }
            self.states.insert(
                service.clone(),
                RuntimeState::StartRequested {
                    attempt: next_attempt,
                },
            );
            return Some(LifecycleAction::Start {
                session_id: self.session_id,
                service: service.clone(),
                attempt: next_attempt,
            });
        }

        if self
            .states
            .values()
            .all(|state| matches!(state, RuntimeState::Ready { .. }))
        {
            self.phase = SessionPhase::Ready;
            return Some(LifecycleAction::SessionReady {
                session_id: self.session_id,
            });
        }
        None
    }

    fn next_stop_action(&mut self) -> Option<LifecycleAction> {
        if self
            .states
            .values()
            .any(|state| matches!(state, RuntimeState::StopRequested { .. }))
        {
            return None;
        }

        for service in self.plan.shutdown_order() {
            let (attempt, process_id) = match self.states.get(service) {
                Some(RuntimeState::Ready {
                    attempt,
                    process_id,
                })
                | Some(RuntimeState::Running {
                    attempt,
                    process_id,
                }) => (*attempt, *process_id),
                _ => continue,
            };
            let reason = self.stop_reason;
            self.states.insert(
                service.clone(),
                RuntimeState::StopRequested {
                    attempt,
                    process_id,
                    reason,
                    continuation: StopContinuation::Complete,
                },
            );
            return Some(LifecycleAction::Stop {
                session_id: self.session_id,
                service: service.clone(),
                process_id,
                reason,
            });
        }

        if let Some(primary) = self.primary_failure.take() {
            self.phase = SessionPhase::Failed;
            return Some(LifecycleAction::SessionFailed {
                session_id: self.session_id,
                service: primary.service,
                failure: primary.failure,
            });
        }

        self.phase = SessionPhase::Stopped;
        Some(LifecycleAction::SessionStopped {
            session_id: self.session_id,
        })
    }

    /// Records successful native process start for the exact requested attempt.
    pub fn record_started(
        &mut self,
        service: &ServiceName,
        attempt: u16,
        process_id: ProcessId,
    ) -> Result<(), SupervisorError> {
        self.require_phase(SessionPhase::Starting)?;
        match self.states.get(service) {
            Some(RuntimeState::StartRequested {
                attempt: expected,
            }) if *expected == attempt => {
                self.states.insert(
                    service.clone(),
                    RuntimeState::Running {
                        attempt,
                        process_id,
                    },
                );
                Ok(())
            }
            Some(RuntimeState::StartRequested {
                attempt: expected,
            }) => Err(SupervisorError::AttemptMismatch {
                service: service.clone(),
                expected: *expected,
                actual: attempt,
            }),
            Some(state) => Err(SupervisorError::InvalidTransition {
                service: service.clone(),
                status: state.public_status(),
                operation: "record_started",
            }),
            None => Err(SupervisorError::UnknownService(service.clone())),
        }
    }

    /// Records explicit readiness for the exact running process.
    pub fn record_ready(
        &mut self,
        service: &ServiceName,
        process_id: ProcessId,
    ) -> Result<(), SupervisorError> {
        self.require_phase(SessionPhase::Starting)?;
        match self.states.get(service) {
            Some(RuntimeState::Running {
                attempt,
                process_id: expected,
            }) if *expected == process_id => {
                let attempt = *attempt;
                self.states.insert(
                    service.clone(),
                    RuntimeState::Ready {
                        attempt,
                        process_id,
                    },
                );
                Ok(())
            }
            Some(RuntimeState::Running {
                process_id: expected,
                ..
            }) => Err(SupervisorError::ProcessMismatch {
                service: service.clone(),
                expected: *expected,
                actual: process_id,
            }),
            Some(state) => Err(SupervisorError::InvalidTransition {
                service: service.clone(),
                status: state.public_status(),
                operation: "record_ready",
            }),
            None => Err(SupervisorError::UnknownService(service.clone())),
        }
    }

    /// Records a failed native start and applies the declared startup restart policy.
    pub fn record_start_failed(
        &mut self,
        service: &ServiceName,
        attempt: u16,
        failure: ServiceFailure,
    ) -> Result<(), SupervisorError> {
        self.require_phase(SessionPhase::Starting)?;
        let expected = match self.states.get(service) {
            Some(RuntimeState::StartRequested { attempt }) => *attempt,
            Some(state) => {
                return Err(SupervisorError::InvalidTransition {
                    service: service.clone(),
                    status: state.public_status(),
                    operation: "record_start_failed",
                })
            }
            None => return Err(SupervisorError::UnknownService(service.clone())),
        };
        if expected != attempt {
            return Err(SupervisorError::AttemptMismatch {
                service: service.clone(),
                expected,
                actual: attempt,
            });
        }
        if failure.stage() != ServiceFailureStage::Start {
            return Err(SupervisorError::FailureStageMismatch {
                expected: ServiceFailureStage::Start,
                actual: failure.stage(),
            });
        }

        let next_attempt = self
            .plan
            .service(service)
            .expect("supervisor state derives from validated plan")
            .restart_policy()
            .next_attempt(attempt);
        if let Some(next_attempt) = next_attempt {
            self.states
                .insert(service.clone(), RuntimeState::Pending { next_attempt });
        } else {
            self.fail_startup(service.clone(), failure);
        }
        Ok(())
    }

    /// Records failed readiness, requests exact process cleanup, then retries or rolls back.
    pub fn record_readiness_failed(
        &mut self,
        service: &ServiceName,
        process_id: ProcessId,
        failure: ServiceFailure,
    ) -> Result<(), SupervisorError> {
        self.require_phase(SessionPhase::Starting)?;
        if failure.stage() != ServiceFailureStage::Readiness {
            return Err(SupervisorError::FailureStageMismatch {
                expected: ServiceFailureStage::Readiness,
                actual: failure.stage(),
            });
        }

        let (attempt, expected_process) = match self.states.get(service) {
            Some(RuntimeState::Running {
                attempt,
                process_id,
            }) => (*attempt, *process_id),
            Some(state) => {
                return Err(SupervisorError::InvalidTransition {
                    service: service.clone(),
                    status: state.public_status(),
                    operation: "record_readiness_failed",
                })
            }
            None => return Err(SupervisorError::UnknownService(service.clone())),
        };
        if expected_process != process_id {
            return Err(SupervisorError::ProcessMismatch {
                service: service.clone(),
                expected: expected_process,
                actual: process_id,
            });
        }

        let next_attempt = self
            .plan
            .service(service)
            .expect("supervisor state derives from validated plan")
            .restart_policy()
            .next_attempt(attempt);
        let (reason, continuation) = match next_attempt {
            Some(next_attempt) => (
                StopReason::RestartCleanup,
                StopContinuation::Retry { next_attempt },
            ),
            None => (
                StopReason::StartupRollback,
                StopContinuation::FailAfterStop {
                    failure: failure.clone(),
                },
            ),
        };
        self.states.insert(
            service.clone(),
            RuntimeState::StopRequested {
                attempt,
                process_id,
                reason,
                continuation,
            },
        );
        self.pending_action = Some(LifecycleAction::Stop {
            session_id: self.session_id,
            service: service.clone(),
            process_id,
            reason,
        });
        Ok(())
    }

    /// Requests a clean reverse-order shutdown after the session reaches readiness.
    pub fn request_shutdown(&mut self) -> Result<(), SupervisorError> {
        self.require_phase(SessionPhase::Ready)?;
        self.phase = SessionPhase::Stopping;
        self.stop_reason = StopReason::Shutdown;
        Ok(())
    }

    /// Records successful completion of an exact previously requested stop.
    pub fn record_stopped(
        &mut self,
        service: &ServiceName,
        process_id: ProcessId,
    ) -> Result<(), SupervisorError> {
        let (expected_process, continuation) = match self.states.get(service) {
            Some(RuntimeState::StopRequested {
                process_id,
                continuation,
                ..
            }) => (*process_id, continuation.clone()),
            Some(state) => {
                return Err(SupervisorError::InvalidTransition {
                    service: service.clone(),
                    status: state.public_status(),
                    operation: "record_stopped",
                })
            }
            None => return Err(SupervisorError::UnknownService(service.clone())),
        };
        if expected_process != process_id {
            return Err(SupervisorError::ProcessMismatch {
                service: service.clone(),
                expected: expected_process,
                actual: process_id,
            });
        }

        match continuation {
            StopContinuation::Complete => {
                self.states.insert(service.clone(), RuntimeState::Stopped);
            }
            StopContinuation::Retry { next_attempt } => {
                self.states
                    .insert(service.clone(), RuntimeState::Pending { next_attempt });
            }
            StopContinuation::FailAfterStop { failure } => {
                self.states.insert(
                    service.clone(),
                    RuntimeState::Failed {
                        failure: failure.clone(),
                    },
                );
                self.primary_failure = Some(PrimaryFailure {
                    service: service.clone(),
                    failure,
                });
                self.phase = SessionPhase::Stopping;
                self.stop_reason = StopReason::StartupRollback;
            }
        }
        Ok(())
    }

    /// Records failed stop while preserving the failure and continuing rollback.
    pub fn record_stop_failed(
        &mut self,
        service: &ServiceName,
        process_id: ProcessId,
        failure: ServiceFailure,
    ) -> Result<(), SupervisorError> {
        if failure.stage() != ServiceFailureStage::Stop {
            return Err(SupervisorError::FailureStageMismatch {
                expected: ServiceFailureStage::Stop,
                actual: failure.stage(),
            });
        }
        let expected_process = match self.states.get(service) {
            Some(RuntimeState::StopRequested { process_id, .. }) => *process_id,
            Some(state) => {
                return Err(SupervisorError::InvalidTransition {
                    service: service.clone(),
                    status: state.public_status(),
                    operation: "record_stop_failed",
                })
            }
            None => return Err(SupervisorError::UnknownService(service.clone())),
        };
        if expected_process != process_id {
            return Err(SupervisorError::ProcessMismatch {
                service: service.clone(),
                expected: expected_process,
                actual: process_id,
            });
        }

        self.states.insert(
            service.clone(),
            RuntimeState::Failed {
                failure: failure.clone(),
            },
        );
        if self.primary_failure.is_none() {
            self.primary_failure = Some(PrimaryFailure {
                service: service.clone(),
                failure,
            });
        }
        self.phase = SessionPhase::Stopping;
        self.stop_reason = StopReason::StartupRollback;
        Ok(())
    }

    /// Records unexpected process exit. Runtime failure is terminal in this foundation.
    pub fn record_unexpected_exit(
        &mut self,
        service: &ServiceName,
        process_id: ProcessId,
        failure: ServiceFailure,
    ) -> Result<(), SupervisorError> {
        if failure.stage() != ServiceFailureStage::Runtime {
            return Err(SupervisorError::FailureStageMismatch {
                expected: ServiceFailureStage::Runtime,
                actual: failure.stage(),
            });
        }
        let expected_process = match self.states.get(service) {
            Some(RuntimeState::Running { process_id, .. })
            | Some(RuntimeState::Ready { process_id, .. }) => *process_id,
            Some(state) => {
                return Err(SupervisorError::InvalidTransition {
                    service: service.clone(),
                    status: state.public_status(),
                    operation: "record_unexpected_exit",
                })
            }
            None => return Err(SupervisorError::UnknownService(service.clone())),
        };
        if expected_process != process_id {
            return Err(SupervisorError::ProcessMismatch {
                service: service.clone(),
                expected: expected_process,
                actual: process_id,
            });
        }
        self.fail_startup(service.clone(), failure);
        Ok(())
    }

    fn fail_startup(&mut self, service: ServiceName, failure: ServiceFailure) {
        self.states.insert(
            service.clone(),
            RuntimeState::Failed {
                failure: failure.clone(),
            },
        );
        self.primary_failure = Some(PrimaryFailure { service, failure });
        self.phase = SessionPhase::Stopping;
        self.stop_reason = StopReason::StartupRollback;
    }

    fn ready_services(&self) -> BTreeSet<ServiceName> {
        self.states
            .iter()
            .filter_map(|(name, state)| {
                matches!(state, RuntimeState::Ready { .. }).then(|| name.clone())
            })
            .collect()
    }

    fn require_phase(&self, expected: SessionPhase) -> Result<(), SupervisorError> {
        if self.phase == expected {
            Ok(())
        } else {
            Err(SupervisorError::PhaseMismatch {
                expected,
                actual: self.phase,
            })
        }
    }
}

/// Invalid lifecycle event or identity crossing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SupervisorError {
    UnknownService(ServiceName),
    PhaseMismatch {
        expected: SessionPhase,
        actual: SessionPhase,
    },
    AttemptMismatch {
        service: ServiceName,
        expected: u16,
        actual: u16,
    },
    ProcessMismatch {
        service: ServiceName,
        expected: ProcessId,
        actual: ProcessId,
    },
    FailureStageMismatch {
        expected: ServiceFailureStage,
        actual: ServiceFailureStage,
    },
    InvalidTransition {
        service: ServiceName,
        status: ServiceStatus,
        operation: &'static str,
    },
}

impl fmt::Display for SupervisorError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownService(service) => write!(formatter, "unknown service: {service}"),
            Self::PhaseMismatch { expected, actual } => {
                write!(formatter, "session phase is {actual:?}; expected {expected:?}")
            }
            Self::AttemptMismatch {
                service,
                expected,
                actual,
            } => write!(
                formatter,
                "service {service} attempt is {actual}; expected {expected}"
            ),
            Self::ProcessMismatch {
                service,
                expected,
                actual,
            } => write!(
                formatter,
                "service {service} process is {actual}; expected {expected}"
            ),
            Self::FailureStageMismatch { expected, actual } => write!(
                formatter,
                "failure stage is {actual:?}; expected {expected:?}"
            ),
            Self::InvalidTransition {
                service,
                status,
                operation,
            } => write!(
                formatter,
                "cannot {operation} for service {service} while state is {status:?}"
            ),
        }
    }
}

impl std::error::Error for SupervisorError {}
