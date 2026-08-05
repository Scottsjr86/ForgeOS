//! ForgeOS supervision of the separate Nyx server process.
//!
//! Nyx owns server behavior and health truth. `forge-bridge` owns native process
//! mechanics. `forge-session` owns canonical lifecycle state. This adapter only
//! joins those contracts and never embeds or recreates Nyx runtime behavior.

use forge_bridge::processes::ProcessExecutionContext;
use forge_bridge::service_process::{ManagedServiceProcess, ManagedServiceProcessError};
use forge_nyx_client::transport::{
    probe_nyx, NyxClientConfig, NyxProbeOutcome, NyxUnavailableReason,
};
use forge_protocol::identities::{ProcessId, SessionId};
use forge_protocol::processes::{ProcessExit, ProcessSpawnRequest};
use forge_session::lifecycle::{ServiceFailure, ServiceFailureStage};
use forge_session::service_runtime::{
    ManagedServiceReadiness, ManagedServiceRuntime, ManagedServiceRuntimeError,
    ManagedServiceRuntimeState,
};
use forge_session::services::{ServiceName, StartupRestartPolicy};
use std::fmt;
use std::thread;
use std::time::Duration;

/// Exact shell-free Nyx launch and bounded readiness policy.
#[derive(Debug, Clone)]
pub struct NyxServiceConfig {
    program: String,
    arguments: Vec<String>,
    context: ProcessExecutionContext,
    client: NyxClientConfig,
    restart_policy: StartupRestartPolicy,
    readiness_attempts: u16,
    readiness_interval: Duration,
}

impl NyxServiceConfig {
    pub fn new<I, S>(
        program: impl Into<String>,
        arguments: I,
        context: ProcessExecutionContext,
        client: NyxClientConfig,
        restart_policy: StartupRestartPolicy,
    ) -> Result<Self, NyxServiceConfigError>
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        let program = program.into();
        ProcessSpawnRequest::new(ProcessId::from_bytes([1; 16]), program.clone(), arguments)
            .map_err(|error| NyxServiceConfigError::InvalidProcess(error.to_string()))
            .map(|request| Self {
                program,
                arguments: request.arguments().to_vec(),
                context,
                client,
                restart_policy,
                readiness_attempts: 40,
                readiness_interval: Duration::from_millis(25),
            })
    }

    pub fn with_readiness_policy(
        mut self,
        attempts: u16,
        interval: Duration,
    ) -> Result<Self, NyxServiceConfigError> {
        if attempts == 0 {
            return Err(NyxServiceConfigError::ZeroReadinessAttempts);
        }
        self.readiness_attempts = attempts;
        self.readiness_interval = interval;
        Ok(self)
    }
}

/// Successful start bound to the exact process attempt and Nyx-owned report.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NyxServiceStart {
    session_id: SessionId,
    attempt: u16,
    process_id: ProcessId,
    outcome: NyxProbeOutcome,
}

impl NyxServiceStart {
    pub const fn session_id(&self) -> SessionId {
        self.session_id
    }

    pub const fn attempt(&self) -> u16 {
        self.attempt
    }

    pub const fn process_id(&self) -> ProcessId {
        self.process_id
    }

    pub fn outcome(&self) -> &NyxProbeOutcome {
        &self.outcome
    }
}

/// Product-level manager for one Nyx process owned by the current ForgeOS session.
#[derive(Debug)]
pub struct ManagedNyxService {
    session_id: SessionId,
    config: NyxServiceConfig,
    runtime: ManagedServiceRuntime,
    process: Option<ManagedServiceProcess>,
}

impl ManagedNyxService {
    pub fn new(session_id: SessionId, config: NyxServiceConfig) -> Self {
        let runtime = ManagedServiceRuntime::new(
            ServiceName::new("nyx-server").expect("static Nyx service name is canonical"),
            config.restart_policy,
        );
        Self {
            session_id,
            config,
            runtime,
            process: None,
        }
    }

    pub const fn session_id(&self) -> SessionId {
        self.session_id
    }

    pub fn state(&self) -> &ManagedServiceRuntimeState {
        self.runtime.state()
    }

    /// Starts one exact Nyx process after proving the endpoint is not already served.
    pub fn start(
        &mut self,
        process_id: ProcessId,
    ) -> Result<NyxServiceStart, ManagedNyxServiceError> {
        if self.runtime.has_used_process_id(process_id) {
            return Err(ManagedServiceRuntimeError::ReusedProcessIdentity(process_id).into());
        }
        self.ensure_endpoint_free()?;
        let attempt = self.runtime.request_start()?;
        let request = ProcessSpawnRequest::new(
            process_id,
            self.config.program.clone(),
            self.config.arguments.clone(),
        )
        .map_err(|error| ManagedNyxServiceError::InvalidProcess(error.to_string()))?;
        let process = match ManagedServiceProcess::spawn(&request, &self.config.context) {
            Ok(process) => process,
            Err(error) => {
                self.runtime.record_attempt_failed(
                    attempt,
                    ServiceFailure::new(ServiceFailureStage::Start, error.to_string()),
                )?;
                return Err(ManagedNyxServiceError::Process(error));
            }
        };
        self.runtime.record_started(attempt, process_id)?;
        self.process = Some(process);

        for probe_index in 0..self.config.readiness_attempts {
            if let Some(exit) = self.poll_native_exit()? {
                return Err(ManagedNyxServiceError::ExitedBeforeReadiness { exit });
            }
            let outcome = probe_nyx(&self.config.client);
            if let Some(readiness) = service_readiness(&outcome) {
                self.runtime
                    .record_readiness(attempt, process_id, readiness)?;
                return Ok(NyxServiceStart {
                    session_id: self.session_id,
                    attempt,
                    process_id,
                    outcome,
                });
            }
            if outcome_proves_occupied_endpoint(&outcome) {
                self.fail_readiness(attempt, format!("Nyx readiness rejected: {outcome:?}"))?;
                return Err(ManagedNyxServiceError::Readiness(outcome));
            }
            if probe_index + 1 < self.config.readiness_attempts {
                thread::sleep(self.config.readiness_interval);
            }
        }

        let final_outcome = probe_nyx(&self.config.client);
        self.fail_readiness(
            attempt,
            format!("Nyx did not expose a live control plane: {final_outcome:?}"),
        )?;
        Err(ManagedNyxServiceError::Readiness(final_outcome))
    }

    /// Returns current Nyx-owned health without treating process presence as readiness.
    pub fn probe(&mut self) -> Result<NyxProbeOutcome, ManagedNyxServiceError> {
        if let Some(exit) = self.poll_native_exit()? {
            return Err(ManagedNyxServiceError::UnexpectedExit { exit });
        }
        if self.process.is_none() {
            return Err(ManagedNyxServiceError::NotRunning);
        }
        Ok(probe_nyx(&self.config.client))
    }

    /// Polls for a crash and advances the bounded restart ledger when one occurred.
    pub fn poll_native_exit(&mut self) -> Result<Option<ProcessExit>, ManagedNyxServiceError> {
        let Some(process) = self.process.as_mut() else {
            return Ok(None);
        };
        let Some(exit) = process.try_wait()? else {
            return Ok(None);
        };
        let process_id = process.process_id();
        self.process = None;
        self.runtime.record_runtime_exit(
            process_id,
            ServiceFailure::new(
                ServiceFailureStage::Runtime,
                format!("Nyx process exited with code {:?}", exit.code()),
            ),
        )?;
        Ok(Some(exit))
    }

    /// Stops the exact managed Nyx process during logout.
    pub fn stop(&mut self) -> Result<ProcessExit, ManagedNyxServiceError> {
        let process_id = self.runtime.request_stop()?;
        let mut process = self
            .process
            .take()
            .ok_or(ManagedNyxServiceError::MissingOwnedProcess(process_id))?;
        if process.process_id() != process_id {
            return Err(ManagedNyxServiceError::ProcessIdentityMismatch {
                expected: process_id,
                actual: process.process_id(),
            });
        }
        let exit = process.stop()?;
        self.runtime.record_stopped(process_id)?;
        Ok(exit)
    }

    fn ensure_endpoint_free(&self) -> Result<(), ManagedNyxServiceError> {
        let outcome = probe_nyx(&self.config.client);
        if endpoint_is_unbound(&outcome) {
            Ok(())
        } else {
            Err(ManagedNyxServiceError::EndpointAlreadyServing(outcome))
        }
    }

    fn fail_readiness(
        &mut self,
        attempt: u16,
        message: String,
    ) -> Result<(), ManagedNyxServiceError> {
        if let Some(mut process) = self.process.take() {
            process.stop()?;
        }
        self.runtime.record_attempt_failed(
            attempt,
            ServiceFailure::new(ServiceFailureStage::Readiness, message),
        )?;
        Ok(())
    }
}

fn service_readiness(outcome: &NyxProbeOutcome) -> Option<ManagedServiceReadiness> {
    match outcome {
        NyxProbeOutcome::Ready { response } => response
            .live()
            .then_some(ManagedServiceReadiness::Ready)
            .filter(|_| response.control_plane_ready()),
        NyxProbeOutcome::Unhealthy { response } => response
            .live()
            .then_some(ManagedServiceReadiness::Degraded)
            .filter(|_| response.control_plane_ready()),
        NyxProbeOutcome::Unavailable { .. } | NyxProbeOutcome::Incompatible { .. } => None,
    }
}

fn endpoint_is_unbound(outcome: &NyxProbeOutcome) -> bool {
    matches!(
        outcome,
        NyxProbeOutcome::Unavailable {
            reason: NyxUnavailableReason::ConnectFailed(_)
        }
    )
}

fn outcome_proves_occupied_endpoint(outcome: &NyxProbeOutcome) -> bool {
    !endpoint_is_unbound(outcome)
        && !matches!(
            outcome,
            NyxProbeOutcome::Unavailable {
                reason: NyxUnavailableReason::ReadFailed(_) | NyxUnavailableReason::WriteFailed(_)
            }
        )
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NyxServiceConfigError {
    InvalidProcess(String),
    ZeroReadinessAttempts,
}

impl fmt::Display for NyxServiceConfigError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidProcess(message) => write!(formatter, "invalid Nyx process: {message}"),
            Self::ZeroReadinessAttempts => {
                formatter.write_str("Nyx readiness attempts must be nonzero")
            }
        }
    }
}

impl std::error::Error for NyxServiceConfigError {}

#[derive(Debug)]
pub enum ManagedNyxServiceError {
    InvalidProcess(String),
    Runtime(ManagedServiceRuntimeError),
    Process(ManagedServiceProcessError),
    EndpointAlreadyServing(NyxProbeOutcome),
    Readiness(NyxProbeOutcome),
    ExitedBeforeReadiness {
        exit: ProcessExit,
    },
    UnexpectedExit {
        exit: ProcessExit,
    },
    NotRunning,
    MissingOwnedProcess(ProcessId),
    ProcessIdentityMismatch {
        expected: ProcessId,
        actual: ProcessId,
    },
}

impl fmt::Display for ManagedNyxServiceError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidProcess(message) => write!(formatter, "invalid Nyx process: {message}"),
            Self::Runtime(error) => error.fmt(formatter),
            Self::Process(error) => error.fmt(formatter),
            Self::EndpointAlreadyServing(outcome) => {
                write!(formatter, "Nyx endpoint is already serving: {outcome:?}")
            }
            Self::Readiness(outcome) => write!(formatter, "Nyx readiness failed: {outcome:?}"),
            Self::ExitedBeforeReadiness { exit } => write!(
                formatter,
                "Nyx exited before readiness with code {:?}",
                exit.code()
            ),
            Self::UnexpectedExit { exit } => {
                write!(
                    formatter,
                    "Nyx exited unexpectedly with code {:?}",
                    exit.code()
                )
            }
            Self::NotRunning => formatter.write_str("Nyx service is not running"),
            Self::MissingOwnedProcess(process_id) => {
                write!(formatter, "Nyx runtime lost owned process {process_id}")
            }
            Self::ProcessIdentityMismatch { expected, actual } => write!(
                formatter,
                "Nyx process {actual} does not match expected process {expected}"
            ),
        }
    }
}

impl std::error::Error for ManagedNyxServiceError {}

impl From<ManagedServiceRuntimeError> for ManagedNyxServiceError {
    fn from(error: ManagedServiceRuntimeError) -> Self {
        Self::Runtime(error)
    }
}

impl From<ManagedServiceProcessError> for ManagedNyxServiceError {
    fn from(error: ManagedServiceProcessError) -> Self {
        Self::Process(error)
    }
}
