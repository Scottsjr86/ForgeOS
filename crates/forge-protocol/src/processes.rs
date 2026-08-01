//! Stable process lifecycle, output, and cancellation contracts.
//!
//! The contract separates durable ForgeOS process identity from an operating-system
//! PID. A PID is observable execution metadata only. Terminal outcomes are explicit,
//! mutually exclusive, and may be committed only once by [`ProcessLifecycle`].

use crate::identities::ProcessId;
use std::fmt;
use std::time::Duration;

/// One process output channel.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
pub enum ProcessStream {
    Stdout = 1,
    Stderr = 2,
}

impl ProcessStream {
    /// Stable V1 numeric code.
    pub const fn code(self) -> u8 {
        self as u8
    }

    /// Stable diagnostic label.
    pub const fn label(self) -> &'static str {
        match self {
            Self::Stdout => "stdout",
            Self::Stderr => "stderr",
        }
    }
}

/// One ordered byte chunk from one process output channel.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProcessOutputChunk {
    process_id: ProcessId,
    stream: ProcessStream,
    sequence: u64,
    bytes: Vec<u8>,
}

impl ProcessOutputChunk {
    /// Creates one channel-local ordered output record.
    pub fn new(
        process_id: ProcessId,
        stream: ProcessStream,
        sequence: u64,
        bytes: impl Into<Vec<u8>>,
    ) -> Self {
        Self {
            process_id,
            stream,
            sequence,
            bytes: bytes.into(),
        }
    }

    /// Stable ForgeOS process identity.
    pub const fn process_id(&self) -> ProcessId {
        self.process_id
    }

    /// Output channel that produced the bytes.
    pub const fn stream(&self) -> ProcessStream {
        self.stream
    }

    /// Monotonic sequence within this output channel.
    pub const fn sequence(&self) -> u64 {
        self.sequence
    }

    /// Exact unmodified bytes read from the channel.
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }
}

/// Final collected bytes kept separate by native output channel.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ProcessOutput {
    stdout: Vec<u8>,
    stderr: Vec<u8>,
}

impl ProcessOutput {
    /// Creates final channel-separated output.
    pub fn new(stdout: impl Into<Vec<u8>>, stderr: impl Into<Vec<u8>>) -> Self {
        Self {
            stdout: stdout.into(),
            stderr: stderr.into(),
        }
    }

    /// Exact stdout bytes without text normalization.
    pub fn stdout(&self) -> &[u8] {
        &self.stdout
    }

    /// Exact stderr bytes without text normalization.
    pub fn stderr(&self) -> &[u8] {
        &self.stderr
    }
}

/// Exact stage at which managed process execution failed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum ProcessFailureStage {
    Spawn = 1,
    Wait = 2,
    Termination = 3,
    Output = 4,
}

impl ProcessFailureStage {
    /// Stable V1 numeric code.
    pub const fn code(self) -> u8 {
        self as u8
    }

    /// Stable diagnostic label.
    pub const fn label(self) -> &'static str {
        match self {
            Self::Spawn => "spawn",
            Self::Wait => "wait",
            Self::Termination => "termination",
            Self::Output => "output",
        }
    }
}

/// Typed process execution failure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProcessFailure {
    stage: ProcessFailureStage,
    message: String,
}

impl ProcessFailure {
    /// Creates a typed failure with preserved native diagnostic text.
    pub fn new(stage: ProcessFailureStage, message: impl Into<String>) -> Self {
        Self {
            stage,
            message: message.into(),
        }
    }

    /// Stage that failed.
    pub const fn stage(&self) -> ProcessFailureStage {
        self.stage
    }

    /// Preserved native diagnostic text.
    pub fn message(&self) -> &str {
        &self.message
    }
}

/// Native exit information for a process that reached an exit status normally.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProcessExit {
    code: Option<i32>,
    success: bool,
}

impl ProcessExit {
    /// Creates native exit information.
    pub const fn new(code: Option<i32>, success: bool) -> Self {
        Self { code, success }
    }

    /// Native numeric exit code, or `None` when the platform reports no code.
    pub const fn code(&self) -> Option<i32> {
        self.code
    }

    /// Whether the native exit status represents success.
    pub const fn success(&self) -> bool {
        self.success
    }
}

/// One mutually exclusive terminal process outcome.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProcessOutcome {
    Exited(ProcessExit),
    TimedOut,
    Cancelled,
    Failed(ProcessFailure),
}

impl ProcessOutcome {
    /// Stable terminal-state label.
    pub const fn label(&self) -> &'static str {
        match self {
            Self::Exited(_) => "exited",
            Self::TimedOut => "timed_out",
            Self::Cancelled => "cancelled",
            Self::Failed(_) => "failed",
        }
    }
}

/// Explicit request to start one process without a shell or implicit path state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProcessSpawnRequest {
    process_id: ProcessId,
    program: String,
    arguments: Vec<String>,
    timeout: Option<Duration>,
}

impl ProcessSpawnRequest {
    /// Creates a validated spawn request.
    pub fn new<I, S>(
        process_id: ProcessId,
        program: impl Into<String>,
        arguments: I,
    ) -> Result<Self, ProcessRequestError>
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        let program = program.into();
        validate_process_text(ProcessRequestField::Program, None, &program)?;
        let arguments = arguments.into_iter().map(Into::into).collect::<Vec<_>>();
        for (index, argument) in arguments.iter().enumerate() {
            validate_process_text(ProcessRequestField::Argument, Some(index), argument)?;
        }
        Ok(Self {
            process_id,
            program,
            arguments,
            timeout: None,
        })
    }

    /// Adds an explicit execution timeout. A zero timeout is valid and immediate.
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = Some(timeout);
        self
    }

    /// Stable ForgeOS process identity chosen before spawn.
    pub const fn process_id(&self) -> ProcessId {
        self.process_id
    }

    /// Exact executable name or path passed to the operating system.
    pub fn program(&self) -> &str {
        &self.program
    }

    /// Exact argument vector passed without shell interpolation.
    pub fn arguments(&self) -> &[String] {
        &self.arguments
    }

    /// Optional explicit timeout.
    pub const fn timeout(&self) -> Option<Duration> {
        self.timeout
    }
}

/// Spawn-request field that failed validation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProcessRequestField {
    Program,
    Argument,
}

/// Exact structural reason a process request was rejected before spawn.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProcessRequestFailure {
    Empty,
    ContainsNul { byte_index: usize },
}

/// Typed invalid process request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProcessRequestError {
    field: ProcessRequestField,
    argument_index: Option<usize>,
    failure: ProcessRequestFailure,
}

impl ProcessRequestError {
    /// Invalid request field.
    pub const fn field(&self) -> ProcessRequestField {
        self.field
    }

    /// Argument index when the invalid field is an argument.
    pub const fn argument_index(&self) -> Option<usize> {
        self.argument_index
    }

    /// Exact structural failure.
    pub fn failure(&self) -> &ProcessRequestFailure {
        &self.failure
    }
}

impl fmt::Display for ProcessRequestError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let field = match (self.field, self.argument_index) {
            (ProcessRequestField::Program, _) => "program".to_owned(),
            (ProcessRequestField::Argument, Some(index)) => format!("argument[{index}]"),
            (ProcessRequestField::Argument, None) => "argument".to_owned(),
        };
        match &self.failure {
            ProcessRequestFailure::Empty => write!(formatter, "process {field} must not be empty"),
            ProcessRequestFailure::ContainsNul { byte_index } => write!(
                formatter,
                "process {field} contains a NUL byte at index {byte_index}"
            ),
        }
    }
}

impl std::error::Error for ProcessRequestError {}

/// One race-safe lifecycle state for one stable process identity.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProcessLifecycleState {
    Requested,
    Running { system_pid: u32 },
    Terminal(ProcessOutcome),
}

/// Single-writer lifecycle that rejects invalid and duplicate terminal commits.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProcessLifecycle {
    process_id: ProcessId,
    state: ProcessLifecycleState,
}

impl ProcessLifecycle {
    /// Starts in the requested state before any operating-system process exists.
    pub const fn requested(process_id: ProcessId) -> Self {
        Self {
            process_id,
            state: ProcessLifecycleState::Requested,
        }
    }

    /// Stable ForgeOS process identity.
    pub const fn process_id(&self) -> ProcessId {
        self.process_id
    }

    /// Current lifecycle state.
    pub fn state(&self) -> &ProcessLifecycleState {
        &self.state
    }

    /// Records the operating-system PID as non-canonical running metadata.
    pub fn mark_running(&mut self, system_pid: u32) -> Result<(), ProcessTransitionError> {
        if system_pid == 0 {
            return Err(ProcessTransitionError::InvalidSystemPid);
        }
        let transition_error = match &self.state {
            ProcessLifecycleState::Requested => None,
            ProcessLifecycleState::Running { .. } => Some(ProcessTransitionError::AlreadyRunning),
            ProcessLifecycleState::Terminal(_) => Some(ProcessTransitionError::AlreadyTerminal),
        };
        if let Some(error) = transition_error {
            return Err(error);
        }
        self.state = ProcessLifecycleState::Running { system_pid };
        Ok(())
    }

    /// Commits one terminal outcome exactly once.
    pub fn finish(&mut self, outcome: ProcessOutcome) -> Result<(), ProcessTransitionError> {
        let is_spawn_failure = matches!(
            &outcome,
            ProcessOutcome::Failed(failure) if failure.stage() == ProcessFailureStage::Spawn
        );
        let transition_error = match &self.state {
            ProcessLifecycleState::Requested if is_spawn_failure => None,
            ProcessLifecycleState::Requested => Some(ProcessTransitionError::NotRunningForOutcome),
            ProcessLifecycleState::Running { .. } => None,
            ProcessLifecycleState::Terminal(_) => Some(ProcessTransitionError::AlreadyTerminal),
        };
        if let Some(error) = transition_error {
            return Err(error);
        }
        self.state = ProcessLifecycleState::Terminal(outcome);
        Ok(())
    }
}

/// Invalid lifecycle transition.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProcessTransitionError {
    InvalidSystemPid,
    AlreadyRunning,
    NotRunningForOutcome,
    AlreadyTerminal,
}

impl fmt::Display for ProcessTransitionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::InvalidSystemPid => "system PID must be nonzero",
            Self::AlreadyRunning => "process lifecycle is already running",
            Self::NotRunningForOutcome => {
                "only a spawn failure may finish a process that never started"
            }
            Self::AlreadyTerminal => "process lifecycle already has a terminal outcome",
        })
    }
}

impl std::error::Error for ProcessTransitionError {}

/// Complete result of one managed process request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProcessExecution {
    process_id: ProcessId,
    system_pid: Option<u32>,
    outcome: ProcessOutcome,
    output: ProcessOutput,
}

impl ProcessExecution {
    /// Creates one complete execution record.
    pub fn new(
        process_id: ProcessId,
        system_pid: Option<u32>,
        outcome: ProcessOutcome,
        output: ProcessOutput,
    ) -> Self {
        Self {
            process_id,
            system_pid,
            outcome,
            output,
        }
    }

    /// Stable ForgeOS process identity.
    pub const fn process_id(&self) -> ProcessId {
        self.process_id
    }

    /// Non-canonical operating-system PID when spawn succeeded.
    pub const fn system_pid(&self) -> Option<u32> {
        self.system_pid
    }

    /// Exact terminal outcome.
    pub fn outcome(&self) -> &ProcessOutcome {
        &self.outcome
    }

    /// Final channel-separated bytes.
    pub fn output(&self) -> &ProcessOutput {
        &self.output
    }
}

fn validate_process_text(
    field: ProcessRequestField,
    argument_index: Option<usize>,
    value: &str,
) -> Result<(), ProcessRequestError> {
    if field == ProcessRequestField::Program && value.is_empty() {
        return Err(ProcessRequestError {
            field,
            argument_index,
            failure: ProcessRequestFailure::Empty,
        });
    }
    if let Some(byte_index) = value.as_bytes().iter().position(|byte| *byte == 0) {
        return Err(ProcessRequestError {
            field,
            argument_index,
            failure: ProcessRequestFailure::ContainsNul { byte_index },
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn process_id(byte: u8) -> ProcessId {
        ProcessId::from_bytes([byte; 16])
    }

    #[test]
    fn request_rejects_empty_program_and_nul_arguments() {
        let empty = ProcessSpawnRequest::new(process_id(1), "", [] as [&str; 0])
            .expect_err("empty program must fail");
        assert_eq!(empty.field(), ProcessRequestField::Program);
        assert_eq!(empty.failure(), &ProcessRequestFailure::Empty);

        let nul = ProcessSpawnRequest::new(process_id(1), "/bin/echo", ["bad\0arg"])
            .expect_err("NUL argument must fail");
        assert_eq!(nul.field(), ProcessRequestField::Argument);
        assert_eq!(nul.argument_index(), Some(0));
        assert_eq!(
            nul.failure(),
            &ProcessRequestFailure::ContainsNul { byte_index: 3 }
        );
    }

    #[test]
    fn lifecycle_accepts_one_terminal_outcome_only() {
        let mut lifecycle = ProcessLifecycle::requested(process_id(2));
        lifecycle.mark_running(42).expect("running transition");
        lifecycle
            .finish(ProcessOutcome::Cancelled)
            .expect("first terminal transition");
        assert_eq!(
            lifecycle.finish(ProcessOutcome::TimedOut),
            Err(ProcessTransitionError::AlreadyTerminal)
        );
        assert_eq!(lifecycle.process_id(), process_id(2));
    }

    #[test]
    fn only_spawn_failure_can_finish_before_running() {
        let mut lifecycle = ProcessLifecycle::requested(process_id(3));
        assert_eq!(
            lifecycle.finish(ProcessOutcome::Cancelled),
            Err(ProcessTransitionError::NotRunningForOutcome)
        );
        lifecycle
            .finish(ProcessOutcome::Failed(ProcessFailure::new(
                ProcessFailureStage::Spawn,
                "fixture",
            )))
            .expect("spawn failure is terminal without a PID");
    }
}
