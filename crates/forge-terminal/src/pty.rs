//! Stable ForgeOS terminal sessions backed by real native pseudo-terminals.

use forge_bridge::pty::{
    NativePtyExit, NativePtyLaunch, NativePtyProcess, NativePtySize, PtyAdapterError,
};
use forge_protocol::identities::TerminalId;
use std::collections::BTreeMap;
use std::ffi::{OsStr, OsString};
use std::fmt;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

/// Validated PTY dimensions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PtyDimensions {
    rows: u16,
    columns: u16,
    pixel_width: u16,
    pixel_height: u16,
}

impl PtyDimensions {
    pub fn new(rows: u16, columns: u16) -> Result<Self, PtyRequestError> {
        Self::with_pixels(rows, columns, 0, 0)
    }

    pub fn with_pixels(
        rows: u16,
        columns: u16,
        pixel_width: u16,
        pixel_height: u16,
    ) -> Result<Self, PtyRequestError> {
        if rows == 0 {
            return Err(PtyRequestError::ZeroRows);
        }
        if columns == 0 {
            return Err(PtyRequestError::ZeroColumns);
        }
        Ok(Self {
            rows,
            columns,
            pixel_width,
            pixel_height,
        })
    }

    pub const fn rows(self) -> u16 {
        self.rows
    }

    pub const fn columns(self) -> u16 {
        self.columns
    }

    pub const fn pixel_width(self) -> u16 {
        self.pixel_width
    }

    pub const fn pixel_height(self) -> u16 {
        self.pixel_height
    }

    fn into_native(self) -> NativePtySize {
        NativePtySize {
            rows: self.rows,
            columns: self.columns,
            pixel_width: self.pixel_width,
            pixel_height: self.pixel_height,
        }
    }
}

/// Exact reason a terminal launch payload is invalid.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PtyRequestError {
    EmptyProgram,
    ZeroRows,
    ZeroColumns,
    WorkingDirectoryNotAbsolute(PathBuf),
    WorkingDirectorySymlink(PathBuf),
    WorkingDirectoryNotDirectory(PathBuf),
    WorkingDirectoryNotCanonical {
        supplied: PathBuf,
        canonical: PathBuf,
    },
    WorkingDirectoryIo {
        path: PathBuf,
        kind: io::ErrorKind,
    },
}

impl fmt::Display for PtyRequestError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyProgram => formatter.write_str("PTY program must not be empty"),
            Self::ZeroRows => formatter.write_str("PTY rows must be nonzero"),
            Self::ZeroColumns => formatter.write_str("PTY columns must be nonzero"),
            Self::WorkingDirectoryNotAbsolute(path) => {
                write!(formatter, "PTY working directory is not absolute: {}", path.display())
            }
            Self::WorkingDirectorySymlink(path) => {
                write!(formatter, "PTY working directory is a symlink: {}", path.display())
            }
            Self::WorkingDirectoryNotDirectory(path) => {
                write!(formatter, "PTY working directory is not a directory: {}", path.display())
            }
            Self::WorkingDirectoryNotCanonical { supplied, canonical } => write!(
                formatter,
                "PTY working directory is not canonical: {} resolves to {}",
                supplied.display(),
                canonical.display()
            ),
            Self::WorkingDirectoryIo { path, kind } => write!(
                formatter,
                "cannot inspect PTY working directory {}: {kind:?}",
                path.display()
            ),
        }
    }
}

impl std::error::Error for PtyRequestError {}

/// Shell-free terminal launch request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PtySpawnRequest {
    terminal_id: TerminalId,
    program: OsString,
    arguments: Vec<OsString>,
    working_directory: PathBuf,
    dimensions: PtyDimensions,
}

impl PtySpawnRequest {
    pub fn new(
        terminal_id: TerminalId,
        program: impl Into<OsString>,
        arguments: Vec<OsString>,
        working_directory: impl Into<PathBuf>,
        dimensions: PtyDimensions,
    ) -> Result<Self, PtyRequestError> {
        let program = program.into();
        if program.is_empty() {
            return Err(PtyRequestError::EmptyProgram);
        }
        let working_directory = working_directory.into();
        validate_working_directory(&working_directory)?;
        Ok(Self {
            terminal_id,
            program,
            arguments,
            working_directory,
            dimensions,
        })
    }

    pub const fn terminal_id(&self) -> TerminalId {
        self.terminal_id
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

    pub const fn dimensions(&self) -> PtyDimensions {
        self.dimensions
    }

    fn native_launch(&self) -> NativePtyLaunch {
        NativePtyLaunch::new(
            self.program.clone(),
            self.arguments.clone(),
            self.working_directory.clone(),
            self.dimensions.into_native(),
        )
    }
}

fn validate_working_directory(path: &Path) -> Result<(), PtyRequestError> {
    if !path.is_absolute() {
        return Err(PtyRequestError::WorkingDirectoryNotAbsolute(
            path.to_path_buf(),
        ));
    }
    let metadata = fs::symlink_metadata(path).map_err(|error| PtyRequestError::WorkingDirectoryIo {
        path: path.to_path_buf(),
        kind: error.kind(),
    })?;
    if metadata.file_type().is_symlink() {
        return Err(PtyRequestError::WorkingDirectorySymlink(path.to_path_buf()));
    }
    if !metadata.is_dir() {
        return Err(PtyRequestError::WorkingDirectoryNotDirectory(
            path.to_path_buf(),
        ));
    }
    let canonical = fs::canonicalize(path).map_err(|error| PtyRequestError::WorkingDirectoryIo {
        path: path.to_path_buf(),
        kind: error.kind(),
    })?;
    if canonical != path {
        return Err(PtyRequestError::WorkingDirectoryNotCanonical {
            supplied: path.to_path_buf(),
            canonical,
        });
    }
    Ok(())
}

/// One raw PTY output chunk. Sequence is local to one terminal session.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PtyOutputChunk {
    terminal_id: TerminalId,
    sequence: u64,
    bytes: Vec<u8>,
}

impl PtyOutputChunk {
    pub const fn terminal_id(&self) -> TerminalId {
        self.terminal_id
    }

    pub const fn sequence(&self) -> u64 {
        self.sequence
    }

    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }
}

/// Native terminal exit with explicit operator-termination classification.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PtyExit {
    code: u32,
    signal: Option<String>,
    terminated_by_operator: bool,
}

impl PtyExit {
    fn native(exit: NativePtyExit, terminated_by_operator: bool) -> Self {
        Self {
            code: exit.code(),
            signal: exit.signal().map(str::to_owned),
            terminated_by_operator,
        }
    }

    pub const fn code(&self) -> u32 {
        self.code
    }

    pub fn signal(&self) -> Option<&str> {
        self.signal.as_deref()
    }

    pub const fn terminated_by_operator(&self) -> bool {
        self.terminated_by_operator
    }

    pub fn success(&self) -> bool {
        self.code == 0 && self.signal.is_none() && !self.terminated_by_operator
    }
}

/// Explicit terminal lifecycle.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PtyLifecycle {
    Running {
        system_pid: Option<u32>,
        dimensions: PtyDimensions,
    },
    Exited(PtyExit),
}

/// Typed terminal-session failure.
#[derive(Debug)]
pub enum PtyError {
    InvalidRequest(PtyRequestError),
    DuplicateTerminal(TerminalId),
    UnknownTerminal(TerminalId),
    NotRunning(TerminalId),
    StillRunning(TerminalId),
    Adapter(PtyAdapterError),
}

impl fmt::Display for PtyError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidRequest(error) => error.fmt(formatter),
            Self::DuplicateTerminal(id) => write!(formatter, "terminal {id} already exists"),
            Self::UnknownTerminal(id) => write!(formatter, "terminal {id} is not registered"),
            Self::NotRunning(id) => write!(formatter, "terminal {id} is not running"),
            Self::StillRunning(id) => write!(formatter, "terminal {id} is still running"),
            Self::Adapter(error) => error.fmt(formatter),
        }
    }
}

impl std::error::Error for PtyError {}

impl From<PtyRequestError> for PtyError {
    fn from(error: PtyRequestError) -> Self {
        Self::InvalidRequest(error)
    }
}

impl From<PtyAdapterError> for PtyError {
    fn from(error: PtyAdapterError) -> Self {
        Self::Adapter(error)
    }
}

/// One stable ForgeOS terminal session.
pub struct PtySession {
    terminal_id: TerminalId,
    native: NativePtyProcess,
    lifecycle: PtyLifecycle,
    next_output_sequence: u64,
    output_eof: bool,
}

impl PtySession {
    fn spawn(request: PtySpawnRequest) -> Result<Self, PtyError> {
        let terminal_id = request.terminal_id();
        let dimensions = request.dimensions();
        let native = NativePtyProcess::spawn(&request.native_launch())?;
        let system_pid = native.system_pid();
        Ok(Self {
            terminal_id,
            native,
            lifecycle: PtyLifecycle::Running {
                system_pid,
                dimensions,
            },
            next_output_sequence: 0,
            output_eof: false,
        })
    }

    pub const fn terminal_id(&self) -> TerminalId {
        self.terminal_id
    }

    pub fn lifecycle(&self) -> &PtyLifecycle {
        &self.lifecycle
    }

    pub const fn output_eof(&self) -> bool {
        self.output_eof
    }

    pub fn read_available(&mut self) -> Result<Vec<PtyOutputChunk>, PtyError> {
        let drained = self.native.drain_output()?;
        self.output_eof |= drained.eof();
        let mut chunks = Vec::with_capacity(drained.chunks().len());
        for bytes in drained.chunks() {
            let sequence = self.next_output_sequence;
            self.next_output_sequence = self
                .next_output_sequence
                .checked_add(1)
                .expect("a single V1 PTY cannot produce u64::MAX output chunks");
            chunks.push(PtyOutputChunk {
                terminal_id: self.terminal_id,
                sequence,
                bytes: bytes.clone(),
            });
        }
        Ok(chunks)
    }

    pub fn write_input(&mut self, bytes: &[u8]) -> Result<(), PtyError> {
        self.ensure_running()?;
        self.native.write_input(bytes)?;
        Ok(())
    }

    pub fn close_input(&mut self) -> Result<(), PtyError> {
        self.ensure_running()?;
        self.native.close_input();
        Ok(())
    }

    pub fn resize(&mut self, dimensions: PtyDimensions) -> Result<(), PtyError> {
        self.ensure_running()?;
        self.native.resize(dimensions.into_native())?;
        let system_pid = self.system_pid();
        self.lifecycle = PtyLifecycle::Running {
            system_pid,
            dimensions,
        };
        Ok(())
    }

    pub fn poll_exit(&mut self) -> Result<Option<PtyExit>, PtyError> {
        if let PtyLifecycle::Exited(exit) = &self.lifecycle {
            return Ok(Some(exit.clone()));
        }
        let Some(native) = self.native.try_wait()? else {
            return Ok(None);
        };
        let exit = PtyExit::native(native, false);
        self.lifecycle = PtyLifecycle::Exited(exit.clone());
        Ok(Some(exit))
    }

    pub fn terminate(&mut self) -> Result<PtyExit, PtyError> {
        if let PtyLifecycle::Exited(exit) = &self.lifecycle {
            return Ok(exit.clone());
        }
        let termination = self.native.terminate()?;
        let exit = PtyExit::native(
            termination.exit().clone(),
            termination.signal_sent(),
        );
        self.lifecycle = PtyLifecycle::Exited(exit.clone());
        Ok(exit)
    }

    fn system_pid(&self) -> Option<u32> {
        match &self.lifecycle {
            PtyLifecycle::Running { system_pid, .. } => *system_pid,
            PtyLifecycle::Exited(_) => None,
        }
    }

    fn ensure_running(&self) -> Result<(), PtyError> {
        match &self.lifecycle {
            PtyLifecycle::Running { .. } => Ok(()),
            PtyLifecycle::Exited(_) => Err(PtyError::NotRunning(self.terminal_id)),
        }
    }
}

/// Stable registry preventing output or mutation from crossing terminal identities.
#[derive(Default)]
pub struct PtyRegistry {
    sessions: BTreeMap<TerminalId, PtySession>,
}

impl PtyRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn spawn(&mut self, request: PtySpawnRequest) -> Result<TerminalId, PtyError> {
        let terminal_id = request.terminal_id();
        if self.sessions.contains_key(&terminal_id) {
            return Err(PtyError::DuplicateTerminal(terminal_id));
        }
        let session = PtySession::spawn(request)?;
        self.sessions.insert(terminal_id, session);
        Ok(terminal_id)
    }

    pub fn session(&self, terminal_id: TerminalId) -> Result<&PtySession, PtyError> {
        self.sessions
            .get(&terminal_id)
            .ok_or(PtyError::UnknownTerminal(terminal_id))
    }

    pub fn session_mut(&mut self, terminal_id: TerminalId) -> Result<&mut PtySession, PtyError> {
        self.sessions
            .get_mut(&terminal_id)
            .ok_or(PtyError::UnknownTerminal(terminal_id))
    }

    pub fn remove_exited(&mut self, terminal_id: TerminalId) -> Result<PtySession, PtyError> {
        let session = self.session(terminal_id)?;
        if matches!(session.lifecycle(), PtyLifecycle::Running { .. }) {
            return Err(PtyError::StillRunning(terminal_id));
        }
        self.sessions
            .remove(&terminal_id)
            .ok_or(PtyError::UnknownTerminal(terminal_id))
    }

    pub fn terminal_ids(&self) -> impl Iterator<Item = TerminalId> + '_ {
        self.sessions.keys().copied()
    }
}
