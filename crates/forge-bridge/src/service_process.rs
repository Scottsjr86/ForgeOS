//! Long-lived process adapter for externally owned managed services.
//!
//! This adapter owns only native process mechanics. Service policy, readiness,
//! restart decisions, and canonical session state remain outside `forge-bridge`.

use crate::processes::{
    configure_managed_process, terminate_managed_process,
    terminate_remaining_managed_process_group, ProcessExecutionContext,
};
use forge_protocol::identities::ProcessId;
use forge_protocol::processes::{ProcessExit, ProcessSpawnRequest};
use std::fmt;
use std::process::{Child, Command, Stdio};
use std::time::Duration;

const STOP_POLL_INTERVAL: Duration = Duration::from_millis(5);
const STOP_GRACE: Duration = Duration::from_millis(500);

/// One externally owned service process isolated in its own native process group.
#[derive(Debug)]
pub struct ManagedServiceProcess {
    process_id: ProcessId,
    child: Child,
    system_pid: u32,
    terminal: Option<ProcessExit>,
}

impl ManagedServiceProcess {
    /// Starts the exact shell-free request in the supplied launch context.
    pub fn spawn(
        request: &ProcessSpawnRequest,
        context: &ProcessExecutionContext,
    ) -> Result<Self, ManagedServiceProcessError> {
        let mut command = Command::new(request.program());
        command
            .args(request.arguments())
            .current_dir(context.working_directory())
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null());
        if context.clears_parent_environment() {
            command.env_clear();
        }
        command.envs(context.environment().iter().cloned());
        configure_managed_process(&mut command);

        let child = command
            .spawn()
            .map_err(|error| ManagedServiceProcessError::Spawn(error.to_string()))?;
        let system_pid = child.id();
        Ok(Self {
            process_id: request.process_id(),
            child,
            system_pid,
            terminal: None,
        })
    }

    /// Stable ForgeOS process identity chosen before native spawn.
    pub const fn process_id(&self) -> ProcessId {
        self.process_id
    }

    /// Native PID exposed only as observed runtime metadata.
    pub const fn system_pid(&self) -> u32 {
        self.system_pid
    }

    /// Polls native exit without inferring readiness from process presence.
    pub fn try_wait(&mut self) -> Result<Option<ProcessExit>, ManagedServiceProcessError> {
        if let Some(exit) = self.terminal {
            return Ok(Some(exit));
        }
        let Some(status) = self
            .child
            .try_wait()
            .map_err(|error| ManagedServiceProcessError::Wait(error.to_string()))?
        else {
            return Ok(None);
        };
        terminate_remaining_managed_process_group(self.system_pid);
        let exit = ProcessExit::new(status.code(), status.success());
        self.terminal = Some(exit);
        Ok(Some(exit))
    }

    /// Stops the exact process group and returns the observed native exit.
    pub fn stop(&mut self) -> Result<ProcessExit, ManagedServiceProcessError> {
        if let Some(exit) = self.try_wait()? {
            return Ok(exit);
        }
        terminate_managed_process(
            &mut self.child,
            self.system_pid,
            STOP_POLL_INTERVAL,
            STOP_GRACE,
        )
        .map_err(|error| ManagedServiceProcessError::Terminate(error.to_string()))?;
        let status = self
            .child
            .try_wait()
            .map_err(|error| ManagedServiceProcessError::Wait(error.to_string()))?
            .ok_or(ManagedServiceProcessError::MissingTerminalStatus)?;
        terminate_remaining_managed_process_group(self.system_pid);
        let exit = ProcessExit::new(status.code(), status.success());
        self.terminal = Some(exit);
        Ok(exit)
    }
}

impl Drop for ManagedServiceProcess {
    fn drop(&mut self) {
        if self.terminal.is_none() {
            let _ = terminate_managed_process(
                &mut self.child,
                self.system_pid,
                STOP_POLL_INTERVAL,
                STOP_GRACE,
            );
            terminate_remaining_managed_process_group(self.system_pid);
        }
    }
}

/// Native process failure preserved without converting it into service health.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ManagedServiceProcessError {
    Spawn(String),
    Wait(String),
    Terminate(String),
    MissingTerminalStatus,
}

impl fmt::Display for ManagedServiceProcessError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Spawn(message) => write!(formatter, "managed service spawn failed: {message}"),
            Self::Wait(message) => write!(formatter, "managed service wait failed: {message}"),
            Self::Terminate(message) => {
                write!(formatter, "managed service termination failed: {message}")
            }
            Self::MissingTerminalStatus => {
                formatter.write_str("managed service terminated without an observable exit status")
            }
        }
    }
}

impl std::error::Error for ManagedServiceProcessError {}
