//! Native pseudo-terminal adapter for ForgeOS terminal sessions.
//!
//! This module owns the operating-system PTY effect behind a narrow byte-preserving
//! interface. Stable ForgeOS terminal identity and session policy remain in
//! `forge-terminal`.

use portable_pty::{CommandBuilder, ExitStatus, MasterPty, PtySize, native_pty_system};
use std::ffi::{OsStr, OsString};
use std::fmt;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::thread::{self, JoinHandle};

/// Exact native PTY size sent to the kernel.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NativePtySize {
    pub rows: u16,
    pub columns: u16,
    pub pixel_width: u16,
    pub pixel_height: u16,
}

impl NativePtySize {
    fn into_portable(self) -> PtySize {
        PtySize {
            rows: self.rows,
            cols: self.columns,
            pixel_width: self.pixel_width,
            pixel_height: self.pixel_height,
        }
    }
}

/// Shell-free native PTY launch payload.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativePtyLaunch {
    program: OsString,
    arguments: Vec<OsString>,
    working_directory: PathBuf,
    size: NativePtySize,
}

impl NativePtyLaunch {
    pub fn new(
        program: impl Into<OsString>,
        arguments: Vec<OsString>,
        working_directory: impl Into<PathBuf>,
        size: NativePtySize,
    ) -> Self {
        Self {
            program: program.into(),
            arguments,
            working_directory: working_directory.into(),
            size,
        }
    }

    pub fn program(&self) -> &OsStr {
        &self.program
    }

    pub fn arguments(&self) -> &[OsString] {
        &self.arguments
    }

    pub fn working_directory(&self) -> &Path {
        &self.working_directory
    }

    pub const fn size(&self) -> NativePtySize {
        self.size
    }
}

/// Stage at which the native adapter failed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PtyAdapterStage {
    Open,
    Reader,
    Writer,
    Spawn,
    Input,
    Resize,
    Wait,
    Terminate,
}

impl PtyAdapterStage {
    const fn label(self) -> &'static str {
        match self {
            Self::Open => "open PTY",
            Self::Reader => "open PTY reader",
            Self::Writer => "open PTY writer",
            Self::Spawn => "spawn PTY child",
            Self::Input => "write PTY input",
            Self::Resize => "resize PTY",
            Self::Wait => "observe PTY child",
            Self::Terminate => "terminate PTY child",
        }
    }
}

/// Typed native PTY adapter failure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PtyAdapterError {
    stage: PtyAdapterStage,
    kind: Option<io::ErrorKind>,
    message: String,
}

impl PtyAdapterError {
    fn from_anyhow(stage: PtyAdapterStage, error: impl fmt::Display) -> Self {
        Self {
            stage,
            kind: None,
            message: error.to_string(),
        }
    }

    fn from_io(stage: PtyAdapterStage, error: io::Error) -> Self {
        Self {
            stage,
            kind: Some(error.kind()),
            message: error.to_string(),
        }
    }

    pub const fn stage(&self) -> PtyAdapterStage {
        self.stage
    }

    pub const fn kind(&self) -> Option<io::ErrorKind> {
        self.kind
    }

    pub fn message(&self) -> &str {
        &self.message
    }
}

impl fmt::Display for PtyAdapterError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{} failed: {}", self.stage.label(), self.message)
    }
}

impl std::error::Error for PtyAdapterError {}

/// Native child exit information without ForgeOS policy classification.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativePtyExit {
    code: u32,
    signal: Option<String>,
}

impl NativePtyExit {
    fn from_status(status: ExitStatus) -> Self {
        Self {
            code: status.exit_code(),
            signal: status.signal().map(str::to_owned),
        }
    }

    pub const fn code(&self) -> u32 {
        self.code
    }

    pub fn signal(&self) -> Option<&str> {
        self.signal.as_deref()
    }

    pub fn success(&self) -> bool {
        self.code == 0 && self.signal.is_none()
    }
}

/// Native termination result, including whether the adapter actually sent a kill.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativePtyTermination {
    exit: NativePtyExit,
    signal_sent: bool,
}

impl NativePtyTermination {
    pub fn exit(&self) -> &NativePtyExit {
        &self.exit
    }

    pub const fn signal_sent(&self) -> bool {
        self.signal_sent
    }
}

#[derive(Debug)]
enum ReaderEvent {
    Bytes(Vec<u8>),
    Eof,
    Failed(io::ErrorKind, String),
}

/// Result of draining bytes produced by the PTY slave.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativePtyDrain {
    chunks: Vec<Vec<u8>>,
    eof: bool,
}

impl NativePtyDrain {
    pub fn chunks(&self) -> &[Vec<u8>] {
        &self.chunks
    }

    pub const fn eof(&self) -> bool {
        self.eof
    }
}

/// One real native PTY process and its master-side byte streams.
pub struct NativePtyProcess {
    master: Box<dyn MasterPty + Send>,
    writer: Option<Box<dyn Write + Send>>,
    child: Box<dyn portable_pty::Child + Send + Sync>,
    output: Receiver<ReaderEvent>,
    reader: Option<JoinHandle<()>>,
    output_eof: bool,
    exit: Option<NativePtyExit>,
}

impl NativePtyProcess {
    /// Opens a PTY and starts one child process without invoking a shell.
    pub fn spawn(request: &NativePtyLaunch) -> Result<Self, PtyAdapterError> {
        let pair = native_pty_system()
            .openpty(request.size().into_portable())
            .map_err(|error| PtyAdapterError::from_anyhow(PtyAdapterStage::Open, error))?;
        let reader = pair
            .master
            .try_clone_reader()
            .map_err(|error| PtyAdapterError::from_anyhow(PtyAdapterStage::Reader, error))?;
        let writer = pair
            .master
            .take_writer()
            .map_err(|error| PtyAdapterError::from_anyhow(PtyAdapterStage::Writer, error))?;

        let mut command = CommandBuilder::new(request.program());
        command.args(request.arguments());
        command.cwd(request.working_directory());
        let child = pair
            .slave
            .spawn_command(command)
            .map_err(|error| PtyAdapterError::from_anyhow(PtyAdapterStage::Spawn, error))?;
        drop(pair.slave);

        let (sender, output) = mpsc::channel();
        let reader = Some(thread::spawn(move || {
            let mut reader = reader;
            let mut buffer = [0_u8; 8192];
            loop {
                match reader.read(&mut buffer) {
                    Ok(0) => {
                        let _ = sender.send(ReaderEvent::Eof);
                        break;
                    }
                    Ok(count) => {
                        if sender
                            .send(ReaderEvent::Bytes(buffer[..count].to_vec()))
                            .is_err()
                        {
                            break;
                        }
                    }
                    Err(error) if error.kind() == io::ErrorKind::Interrupted => {}
                    // Linux reports EIO when the PTY slave has closed. For the
                    // master byte stream that is terminal EOF, not corrupted output.
                    Err(error) if error.raw_os_error() == Some(5) => {
                        let _ = sender.send(ReaderEvent::Eof);
                        break;
                    }
                    Err(error) => {
                        let _ = sender.send(ReaderEvent::Failed(error.kind(), error.to_string()));
                        break;
                    }
                }
            }
        }));

        Ok(Self {
            master: pair.master,
            writer: Some(writer),
            child,
            output,
            reader,
            output_eof: false,
            exit: None,
        })
    }

    /// Native child process identifier, when supplied by the platform.
    pub fn system_pid(&self) -> Option<u32> {
        self.child.process_id()
    }

    /// Drains all bytes currently available without blocking.
    pub fn drain_output(&mut self) -> Result<NativePtyDrain, PtyAdapterError> {
        let mut chunks = Vec::new();
        loop {
            match self.output.try_recv() {
                Ok(ReaderEvent::Bytes(bytes)) => chunks.push(bytes),
                Ok(ReaderEvent::Eof) => self.output_eof = true,
                Ok(ReaderEvent::Failed(kind, message)) => {
                    return Err(PtyAdapterError {
                        stage: PtyAdapterStage::Reader,
                        kind: Some(kind),
                        message,
                    });
                }
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => {
                    self.output_eof = true;
                    break;
                }
            }
        }
        Ok(NativePtyDrain {
            chunks,
            eof: self.output_eof,
        })
    }

    /// Writes exact bytes to the PTY master and flushes them.
    pub fn write_input(&mut self, bytes: &[u8]) -> Result<(), PtyAdapterError> {
        let writer = self.writer.as_mut().ok_or_else(|| PtyAdapterError {
            stage: PtyAdapterStage::Input,
            kind: Some(io::ErrorKind::BrokenPipe),
            message: "PTY input is closed".to_owned(),
        })?;
        writer
            .write_all(bytes)
            .and_then(|()| writer.flush())
            .map_err(|error| PtyAdapterError::from_io(PtyAdapterStage::Input, error))
    }

    /// Closes the input channel. The child observes terminal EOF when supported.
    pub fn close_input(&mut self) {
        self.writer.take();
    }

    /// Applies a real kernel PTY resize.
    pub fn resize(&self, size: NativePtySize) -> Result<(), PtyAdapterError> {
        self.master
            .resize(size.into_portable())
            .map_err(|error| PtyAdapterError::from_anyhow(PtyAdapterStage::Resize, error))
    }

    /// Polls the native child without blocking.
    pub fn try_wait(&mut self) -> Result<Option<NativePtyExit>, PtyAdapterError> {
        if let Some(exit) = &self.exit {
            return Ok(Some(exit.clone()));
        }
        let status = self
            .child
            .try_wait()
            .map_err(|error| PtyAdapterError::from_io(PtyAdapterStage::Wait, error))?;
        Ok(status.map(|status| {
            let exit = NativePtyExit::from_status(status);
            self.exit = Some(exit.clone());
            exit
        }))
    }

    /// Terminates the child and waits for its native exit status.
    pub fn terminate(&mut self) -> Result<NativePtyTermination, PtyAdapterError> {
        if let Some(exit) = self.try_wait()? {
            return Ok(NativePtyTermination {
                exit,
                signal_sent: false,
            });
        }
        if let Err(kill_error) = self.child.kill() {
            match self.child.try_wait() {
                Ok(Some(status)) => {
                    let exit = NativePtyExit::from_status(status);
                    self.exit = Some(exit.clone());
                    return Ok(NativePtyTermination {
                        exit,
                        signal_sent: false,
                    });
                }
                Ok(None) => {
                    return Err(PtyAdapterError::from_io(
                        PtyAdapterStage::Terminate,
                        kill_error,
                    ));
                }
                Err(wait_error) => {
                    return Err(PtyAdapterError {
                        stage: PtyAdapterStage::Terminate,
                        kind: Some(kill_error.kind()),
                        message: format!(
                            "kill failed: {kill_error}; exit recheck failed: {wait_error}"
                        ),
                    });
                }
            }
        }
        let status = self
            .child
            .wait()
            .map_err(|error| PtyAdapterError::from_io(PtyAdapterStage::Wait, error))?;
        let exit = NativePtyExit::from_status(status);
        self.exit = Some(exit.clone());
        Ok(NativePtyTermination {
            exit,
            signal_sent: true,
        })
    }
}

impl Drop for NativePtyProcess {
    fn drop(&mut self) {
        if self.exit.is_none() && self.child.kill().is_ok() {
            let _ = self.child.wait();
        }
        self.writer.take();
        // Dropping the join handle detaches the reader. The master is dropped
        // immediately after this method returns, which unblocks the read and lets
        // the thread exit without deadlocking `Drop` on its own still-open master.
        self.reader.take();
    }
}
