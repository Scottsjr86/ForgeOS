//! Real managed process execution behind the ForgeOS process contract.
//!
//! The runner uses a stable ForgeOS [`ProcessId`] supplied before spawn, keeps the
//! native PID as metadata only, captures stdout and stderr independently, observes
//! actual exit before cancellation or timeout, and commits exactly one terminal
//! outcome. On Unix, each managed process starts in its own process group so
//! cancellation and timeout terminate descendants instead of orphaning them.

use forge_protocol::identities::ProcessId;
use forge_protocol::processes::{
    ProcessExecution, ProcessExit, ProcessFailure, ProcessFailureStage, ProcessLifecycle,
    ProcessOutcome, ProcessOutput, ProcessOutputChunk, ProcessSpawnRequest, ProcessStream,
};
use std::io::{self, Read};
use std::process::{Child, Command, ExitStatus, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc};
use std::thread;
use std::time::{Duration, Instant};

/// Thread-safe cancellation signal shared with one managed execution.
#[derive(Debug, Clone, Default)]
pub struct CancellationToken {
    cancelled: Arc<AtomicBool>,
}

impl CancellationToken {
    /// Creates a live, not-yet-cancelled token.
    pub fn new() -> Self {
        Self::default()
    }

    /// Requests cancellation. Repeated calls are idempotent.
    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::Release);
    }

    /// Whether cancellation has been requested.
    pub fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::Acquire)
    }
}

/// Configuration error for process polling and termination behavior.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProcessRunnerConfigError {
    ZeroPollInterval,
}

impl std::fmt::Display for ProcessRunnerConfigError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self {
            Self::ZeroPollInterval => "process poll interval must be nonzero",
        })
    }
}

impl std::error::Error for ProcessRunnerConfigError {}

/// Synchronous real-process runner with bounded polling and tree termination.
#[derive(Debug, Clone)]
pub struct ProcessRunner {
    poll_interval: Duration,
    termination_grace: Duration,
}

impl Default for ProcessRunner {
    fn default() -> Self {
        Self {
            poll_interval: Duration::from_millis(10),
            termination_grace: Duration::from_millis(250),
        }
    }
}

impl ProcessRunner {
    /// Creates a runner with explicit testable timing controls.
    pub fn with_timings(
        poll_interval: Duration,
        termination_grace: Duration,
    ) -> Result<Self, ProcessRunnerConfigError> {
        if poll_interval.is_zero() {
            return Err(ProcessRunnerConfigError::ZeroPollInterval);
        }
        Ok(Self {
            poll_interval,
            termination_grace,
        })
    }

    /// Runs one process and returns final channel-separated output.
    pub fn run(
        &self,
        request: ProcessSpawnRequest,
        cancellation: &CancellationToken,
    ) -> ProcessExecution {
        self.run_with_output(request, cancellation, |_| {})
    }

    /// Runs one process while observing ordered chunks from each native output channel.
    ///
    /// Sequence numbers are monotonic within each channel. Relative arrival order
    /// between stdout and stderr is deliberately not treated as canonical truth.
    pub fn run_with_output<F>(
        &self,
        request: ProcessSpawnRequest,
        cancellation: &CancellationToken,
        mut observe: F,
    ) -> ProcessExecution
    where
        F: FnMut(&ProcessOutputChunk),
    {
        let process_id = request.process_id();
        let mut lifecycle = ProcessLifecycle::requested(process_id);
        let mut command = Command::new(request.program());
        command
            .args(request.arguments())
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        configure_process_group(&mut command);

        let mut child = match command.spawn() {
            Ok(child) => child,
            Err(error) => {
                let outcome = ProcessOutcome::Failed(ProcessFailure::new(
                    ProcessFailureStage::Spawn,
                    error.to_string(),
                ));
                lifecycle
                    .finish(outcome.clone())
                    .expect("spawn failure is a valid requested-state terminal outcome");
                return ProcessExecution::new(process_id, None, outcome, ProcessOutput::default());
            }
        };

        let system_pid = child.id();
        lifecycle
            .mark_running(system_pid)
            .expect("operating-system child IDs are nonzero and lifecycle is requested");

        let stdout = child
            .stdout
            .take()
            .expect("stdout was configured as a pipe before spawn");
        let stderr = child
            .stderr
            .take()
            .expect("stderr was configured as a pipe before spawn");
        let (sender, receiver) = mpsc::channel();
        let stdout_reader =
            spawn_output_reader(process_id, ProcessStream::Stdout, stdout, sender.clone());
        let stderr_reader =
            spawn_output_reader(process_id, ProcessStream::Stderr, stderr, sender.clone());
        drop(sender);

        let started_at = Instant::now();
        let (mut outcome, collect_output) = loop {
            drain_output(&receiver, &mut observe);

            match child.try_wait() {
                Ok(Some(status)) => {
                    terminate_remaining_process_group(system_pid);
                    break (ProcessOutcome::Exited(exit_from_status(status)), true);
                }
                Ok(None) => {}
                Err(error) => {
                    let cleanup = terminate_child_tree(
                        &mut child,
                        system_pid,
                        self.poll_interval,
                        self.termination_grace,
                    );
                    let cleanup_succeeded = cleanup.is_ok();
                    let message = match &cleanup {
                        Ok(_) => error.to_string(),
                        Err(cleanup_error) => {
                            format!("wait failed: {error}; cleanup failed: {cleanup_error}")
                        }
                    };
                    break (
                        ProcessOutcome::Failed(ProcessFailure::new(
                            ProcessFailureStage::Wait,
                            message,
                        )),
                        cleanup_succeeded,
                    );
                }
            }

            if cancellation.is_cancelled() {
                match terminate_child_tree(
                    &mut child,
                    system_pid,
                    self.poll_interval,
                    self.termination_grace,
                ) {
                    Ok(termination) if termination.signal_sent => {
                        break (ProcessOutcome::Cancelled, true);
                    }
                    Ok(termination) => {
                        break (
                            ProcessOutcome::Exited(exit_from_status(termination.status)),
                            true,
                        );
                    }
                    Err(error) => {
                        break (
                            ProcessOutcome::Failed(ProcessFailure::new(
                                ProcessFailureStage::Termination,
                                format!("cancellation could not terminate process tree: {error}"),
                            )),
                            false,
                        );
                    }
                }
            }

            if matches!(
                request.timeout(),
                Some(timeout) if started_at.elapsed() >= timeout
            ) {
                match terminate_child_tree(
                    &mut child,
                    system_pid,
                    self.poll_interval,
                    self.termination_grace,
                ) {
                    Ok(termination) if termination.signal_sent => {
                        break (ProcessOutcome::TimedOut, true);
                    }
                    Ok(termination) => {
                        break (
                            ProcessOutcome::Exited(exit_from_status(termination.status)),
                            true,
                        );
                    }
                    Err(error) => {
                        break (
                            ProcessOutcome::Failed(ProcessFailure::new(
                                ProcessFailureStage::Termination,
                                format!("timeout could not terminate process tree: {error}"),
                            )),
                            false,
                        );
                    }
                }
            }

            thread::sleep(self.poll_interval);
        };

        let output = if collect_output {
            let stdout_result = join_output_reader(stdout_reader, ProcessStream::Stdout);
            let stderr_result = join_output_reader(stderr_reader, ProcessStream::Stderr);
            drain_output(&receiver, &mut observe);
            match (stdout_result, stderr_result) {
                (Ok(stdout), Ok(stderr)) => ProcessOutput::new(stdout, stderr),
                (stdout, stderr) => {
                    let mut messages = Vec::new();
                    if let Err(error) = stdout {
                        messages.push(error);
                    }
                    if let Err(error) = stderr {
                        messages.push(error);
                    }
                    outcome = ProcessOutcome::Failed(ProcessFailure::new(
                        ProcessFailureStage::Output,
                        messages.join("; "),
                    ));
                    ProcessOutput::default()
                }
            }
        } else {
            drain_output(&receiver, &mut observe);
            drop(stdout_reader);
            drop(stderr_reader);
            ProcessOutput::default()
        };

        lifecycle
            .finish(outcome.clone())
            .expect("runner owns the only terminal lifecycle commit");
        ProcessExecution::new(process_id, Some(system_pid), outcome, output)
    }
}

fn spawn_output_reader<R>(
    process_id: ProcessId,
    stream: ProcessStream,
    mut reader: R,
    sender: mpsc::Sender<ProcessOutputChunk>,
) -> thread::JoinHandle<io::Result<Vec<u8>>>
where
    R: Read + Send + 'static,
{
    thread::spawn(move || {
        let mut complete = Vec::new();
        let mut buffer = [0_u8; 8192];
        let mut sequence = 0_u64;
        loop {
            let read = reader.read(&mut buffer)?;
            if read == 0 {
                return Ok(complete);
            }
            let bytes = buffer[..read].to_vec();
            complete.extend_from_slice(&bytes);
            if sender
                .send(ProcessOutputChunk::new(process_id, stream, sequence, bytes))
                .is_err()
            {
                return Ok(complete);
            }
            sequence = sequence.saturating_add(1);
        }
    })
}

fn join_output_reader(
    handle: thread::JoinHandle<io::Result<Vec<u8>>>,
    stream: ProcessStream,
) -> Result<Vec<u8>, String> {
    match handle.join() {
        Ok(Ok(bytes)) => Ok(bytes),
        Ok(Err(error)) => Err(format!("{} read failed: {error}", stream.label())),
        Err(_) => Err(format!("{} reader thread panicked", stream.label())),
    }
}

fn drain_output<F>(receiver: &mpsc::Receiver<ProcessOutputChunk>, observe: &mut F)
where
    F: FnMut(&ProcessOutputChunk),
{
    while let Ok(chunk) = receiver.try_recv() {
        observe(&chunk);
    }
}

fn exit_from_status(status: ExitStatus) -> ProcessExit {
    ProcessExit::new(status.code(), status.success())
}

#[cfg(unix)]
fn configure_process_group(command: &mut Command) {
    use std::os::unix::process::CommandExt;
    command.process_group(0);
}

#[cfg(not(unix))]
fn configure_process_group(_command: &mut Command) {}

struct TreeTermination {
    status: ExitStatus,
    signal_sent: bool,
}

#[cfg(unix)]
fn terminate_child_tree(
    child: &mut Child,
    process_group: u32,
    poll_interval: Duration,
    termination_grace: Duration,
) -> io::Result<TreeTermination> {
    if let Err(signal_error) = signal_process_group(process_group, "-TERM") {
        if let Some(status) = child.try_wait()? {
            return Ok(TreeTermination {
                status,
                signal_sent: false,
            });
        }
        return Err(signal_error);
    }
    let started = Instant::now();
    loop {
        if let Some(status) = child.try_wait()? {
            terminate_remaining_process_group(process_group);
            return Ok(TreeTermination {
                status,
                signal_sent: true,
            });
        }
        if started.elapsed() >= termination_grace {
            break;
        }
        thread::sleep(poll_interval);
    }

    if let Err(signal_error) = signal_process_group(process_group, "-KILL") {
        if let Some(status) = child.try_wait()? {
            return Ok(TreeTermination {
                status,
                signal_sent: true,
            });
        }
        return Err(signal_error);
    }
    child.wait().map(|status| TreeTermination {
        status,
        signal_sent: true,
    })
}

#[cfg(not(unix))]
fn terminate_child_tree(
    child: &mut Child,
    _process_group: u32,
    _poll_interval: Duration,
    _termination_grace: Duration,
) -> io::Result<TreeTermination> {
    child.kill()?;
    child.wait().map(|status| TreeTermination {
        status,
        signal_sent: true,
    })
}

#[cfg(unix)]
fn signal_process_group(process_group: u32, signal: &str) -> io::Result<()> {
    let target = format!("-{process_group}");
    let status = Command::new("kill")
        .arg(signal)
        .arg("--")
        .arg(target)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()?;
    if status.success() {
        Ok(())
    } else {
        Err(io::Error::new(
            io::ErrorKind::Other,
            format!("kill {signal} process group {process_group} failed with {status}"),
        ))
    }
}

#[cfg(unix)]
fn terminate_remaining_process_group(process_group: u32) {
    let _ = signal_process_group(process_group, "-KILL");
}

#[cfg(not(unix))]
fn terminate_remaining_process_group(_process_group: u32) {}

#[cfg(test)]
mod tests {
    use super::{CancellationToken, ProcessRunner};
    use forge_protocol::identities::ProcessId;
    use forge_protocol::processes::{
        ProcessExit, ProcessOutcome, ProcessOutputChunk, ProcessSpawnRequest, ProcessStream,
    };
    use std::fs;
    use std::io;
    use std::path::{Path, PathBuf};
    use std::sync::atomic::{AtomicU64, Ordering as AtomicOrdering};
    use std::thread;
    use std::time::{Duration, Instant};

    static TEMP_COUNTER: AtomicU64 = AtomicU64::new(0);

    fn process_id(byte: u8) -> ProcessId {
        ProcessId::from_bytes([byte; 16])
    }

    fn request(byte: u8, script: &str) -> ProcessSpawnRequest {
        ProcessSpawnRequest::new(process_id(byte), "/bin/sh", ["-c", script])
            .expect("fixture request is valid")
    }

    fn runner() -> ProcessRunner {
        ProcessRunner::with_timings(Duration::from_millis(5), Duration::from_millis(100))
            .expect("fixture timings are valid")
    }

    fn temp_path(label: &str) -> PathBuf {
        let count = TEMP_COUNTER.fetch_add(1, AtomicOrdering::Relaxed);
        std::env::temp_dir().join(format!(
            "forge-process-{label}-{}-{count}",
            std::process::id()
        ))
    }

    #[test]
    fn captures_fast_success_and_channel_local_sequences() {
        let mut chunks = Vec::new();
        let execution = runner().run_with_output(
            request(1, "printf 'out'; printf 'err' >&2"),
            &CancellationToken::new(),
            |chunk| chunks.push(chunk.clone()),
        );

        assert_eq!(execution.process_id(), process_id(1));
        assert!(execution.system_pid().is_some());
        assert_eq!(
            execution.outcome(),
            &ProcessOutcome::Exited(ProcessExit::new(Some(0), true))
        );
        assert_eq!(execution.output().stdout(), b"out");
        assert_eq!(execution.output().stderr(), b"err");
        assert!(chunks
            .iter()
            .all(|chunk| chunk.process_id() == process_id(1)));
        for stream in [ProcessStream::Stdout, ProcessStream::Stderr] {
            let sequences = chunks
                .iter()
                .filter(|chunk| chunk.stream() == stream)
                .map(ProcessOutputChunk::sequence)
                .collect::<Vec<_>>();
            assert_eq!(sequences, (0..sequences.len() as u64).collect::<Vec<_>>());
        }
    }

    #[test]
    fn nonzero_exit_is_explicit_exit_not_adapter_failure() {
        let execution = runner().run(
            request(2, "printf 'nope' >&2; exit 7"),
            &CancellationToken::new(),
        );
        assert_eq!(
            execution.outcome(),
            &ProcessOutcome::Exited(ProcessExit::new(Some(7), false))
        );
        assert_eq!(execution.output().stderr(), b"nope");
    }

    #[test]
    fn timeout_is_not_reported_as_normal_exit() {
        let execution = runner().run(
            request(3, "sleep 5").with_timeout(Duration::from_millis(40)),
            &CancellationToken::new(),
        );
        assert_eq!(execution.outcome(), &ProcessOutcome::TimedOut);
    }

    #[test]
    fn cancellation_is_not_reported_as_success() {
        let token = CancellationToken::new();
        let worker_token = token.clone();
        let worker = thread::spawn(move || {
            runner().run(
                request(4, "sleep 5").with_timeout(Duration::from_secs(2)),
                &worker_token,
            )
        });
        thread::sleep(Duration::from_millis(40));
        token.cancel();
        let execution = worker.join().expect("runner thread must not panic");
        assert_eq!(execution.outcome(), &ProcessOutcome::Cancelled);
    }

    #[test]
    fn concurrent_outcomes_keep_stable_process_identity() {
        let cancelled_token = CancellationToken::new();
        let worker_token = cancelled_token.clone();
        let cancelled = thread::spawn(move || {
            runner().run(
                request(5, "sleep 5").with_timeout(Duration::from_secs(2)),
                &worker_token,
            )
        });
        let fast =
            thread::spawn(move || runner().run(request(6, "exit 9"), &CancellationToken::new()));
        thread::sleep(Duration::from_millis(40));
        cancelled_token.cancel();

        let cancelled = cancelled.join().expect("cancelled fixture thread");
        let fast = fast.join().expect("fast fixture thread");
        assert_eq!(cancelled.process_id(), process_id(5));
        assert_eq!(cancelled.outcome(), &ProcessOutcome::Cancelled);
        assert_eq!(fast.process_id(), process_id(6));
        assert_eq!(
            fast.outcome(),
            &ProcessOutcome::Exited(ProcessExit::new(Some(9), false))
        );
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn cancellation_terminates_descendant_process_group() {
        let marker = temp_path("child-pid");
        let script = "sleep 5 & child=$!; printf '%s' \"$child\" > \"$1\"; wait";
        let request = ProcessSpawnRequest::new(
            process_id(7),
            "/bin/sh",
            [
                "-c",
                script,
                "forge-child-fixture",
                marker.to_str().expect("UTF-8 temp path"),
            ],
        )
        .expect("child fixture request")
        .with_timeout(Duration::from_secs(3));
        let token = CancellationToken::new();
        let worker_token = token.clone();
        let worker = thread::spawn(move || runner().run(request, &worker_token));

        let child_pid = wait_for_child_pid(&marker, Duration::from_secs(2));
        token.cancel();
        let execution = worker.join().expect("runner thread");
        assert_eq!(execution.outcome(), &ProcessOutcome::Cancelled);
        assert!(wait_until_process_absent(child_pid, Duration::from_secs(2)));
        let _ = fs::remove_file(marker);
    }

    #[cfg(target_os = "linux")]
    fn wait_for_child_pid(path: &Path, timeout: Duration) -> u32 {
        let started = Instant::now();
        loop {
            if let Ok(contents) = fs::read_to_string(path) {
                if let Ok(pid) = contents.parse::<u32>() {
                    return pid;
                }
            }
            assert!(
                started.elapsed() < timeout,
                "fixture child PID marker was not completed"
            );
            thread::sleep(Duration::from_millis(5));
        }
    }

    #[cfg(target_os = "linux")]
    fn wait_until_process_absent(pid: u32, timeout: Duration) -> bool {
        let started = Instant::now();
        loop {
            let stat_path = PathBuf::from(format!("/proc/{pid}/stat"));
            match fs::read_to_string(stat_path) {
                Ok(stat) => {
                    let state = stat
                        .rsplit_once(") ")
                        .and_then(|(_, rest)| rest.chars().next());
                    if state == Some('Z') {
                        return true;
                    }
                }
                Err(error) if error.kind() == io::ErrorKind::NotFound => return true,
                Err(error) => panic!("failed to inspect child process: {error}"),
            }
            if started.elapsed() >= timeout {
                return false;
            }
            thread::sleep(Duration::from_millis(10));
        }
    }
}
