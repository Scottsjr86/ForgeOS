//! Project-bound management for multiple real PTY sessions.
//!
//! The native PTY adapter remains the source of process and byte truth. This layer
//! binds every terminal to stable project and repository identities, preserves raw
//! output for rendering, and requires the same binding for every mutation.

use crate::pty::{
    PtyDimensions, PtyError, PtyExit, PtyLifecycle, PtyOutputChunk, PtyRegistry, PtySpawnRequest,
};
use forge_protocol::identities::{ProjectId, RepositoryId, TerminalId};
use std::collections::BTreeMap;
use std::fmt;
use std::path::{Path, PathBuf};

/// Stable project/repository/terminal identity required for every operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ManagedTerminalHandle {
    project_id: ProjectId,
    repository_id: RepositoryId,
    terminal_id: TerminalId,
}

impl ManagedTerminalHandle {
    pub const fn new(
        project_id: ProjectId,
        repository_id: RepositoryId,
        terminal_id: TerminalId,
    ) -> Self {
        Self {
            project_id,
            repository_id,
            terminal_id,
        }
    }

    pub const fn project_id(self) -> ProjectId {
        self.project_id
    }

    pub const fn repository_id(self) -> RepositoryId {
        self.repository_id
    }

    pub const fn terminal_id(self) -> TerminalId {
        self.terminal_id
    }
}

/// One project-bound native PTY launch request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ManagedTerminalSpawnRequest {
    handle: ManagedTerminalHandle,
    pty: PtySpawnRequest,
}

impl ManagedTerminalSpawnRequest {
    pub fn new(
        project_id: ProjectId,
        repository_id: RepositoryId,
        pty: PtySpawnRequest,
    ) -> Result<Self, ManagedTerminalError> {
        let handle = ManagedTerminalHandle::new(project_id, repository_id, pty.terminal_id());
        Ok(Self { handle, pty })
    }

    pub const fn handle(&self) -> ManagedTerminalHandle {
        self.handle
    }

    pub fn working_directory(&self) -> &Path {
        self.pty.working_directory()
    }

    fn into_parts(self) -> (ManagedTerminalHandle, PtySpawnRequest) {
        (self.handle, self.pty)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ManagedTerminalRecord {
    handle: ManagedTerminalHandle,
    working_directory: PathBuf,
    output: Vec<PtyOutputChunk>,
}

/// Immutable renderable view of one managed terminal.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ManagedTerminalView {
    handle: ManagedTerminalHandle,
    working_directory: PathBuf,
    lifecycle: PtyLifecycle,
    output: Vec<PtyOutputChunk>,
    output_eof: bool,
}

impl ManagedTerminalView {
    pub const fn handle(&self) -> ManagedTerminalHandle {
        self.handle
    }

    pub fn working_directory(&self) -> &Path {
        &self.working_directory
    }

    pub fn lifecycle(&self) -> &PtyLifecycle {
        &self.lifecycle
    }

    pub fn output_chunks(&self) -> &[PtyOutputChunk] {
        &self.output
    }

    pub const fn output_eof(&self) -> bool {
        self.output_eof
    }

    /// Concatenates the exact raw PTY bytes in sequence order for a renderer.
    pub fn output_bytes(&self) -> Vec<u8> {
        let capacity = self.output.iter().map(|chunk| chunk.bytes().len()).sum();
        let mut bytes = Vec::with_capacity(capacity);
        for chunk in &self.output {
            bytes.extend_from_slice(chunk.bytes());
        }
        bytes
    }
}

/// Typed failure from the project-bound terminal registry.
#[derive(Debug)]
pub enum ManagedTerminalError {
    Pty(PtyError),
    MissingRecord(TerminalId),
    BindingMismatch {
        terminal_id: TerminalId,
        expected_project: ProjectId,
        expected_repository: RepositoryId,
        found_project: ProjectId,
        found_repository: RepositoryId,
    },
    OutputSequenceGap {
        terminal_id: TerminalId,
        expected: u64,
        found: u64,
    },
}

impl fmt::Display for ManagedTerminalError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Pty(error) => error.fmt(formatter),
            Self::MissingRecord(terminal_id) => {
                write!(
                    formatter,
                    "managed terminal {terminal_id} has no metadata record"
                )
            }
            Self::BindingMismatch {
                terminal_id,
                expected_project,
                expected_repository,
                found_project,
                found_repository,
            } => write!(
                formatter,
                "terminal {terminal_id} belongs to project {expected_project} repository {expected_repository}, not project {found_project} repository {found_repository}"
            ),
            Self::OutputSequenceGap {
                terminal_id,
                expected,
                found,
            } => write!(
                formatter,
                "terminal {terminal_id} output sequence expected {expected} but received {found}"
            ),
        }
    }
}

impl std::error::Error for ManagedTerminalError {}

impl From<PtyError> for ManagedTerminalError {
    fn from(error: PtyError) -> Self {
        Self::Pty(error)
    }
}

/// Registry of isolated project-bound PTY sessions.
#[derive(Default)]
pub struct ManagedTerminalRegistry {
    ptys: PtyRegistry,
    records: BTreeMap<TerminalId, ManagedTerminalRecord>,
}

impl ManagedTerminalRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn spawn(
        &mut self,
        request: ManagedTerminalSpawnRequest,
    ) -> Result<ManagedTerminalHandle, ManagedTerminalError> {
        let working_directory = request.working_directory().to_path_buf();
        let (handle, pty) = request.into_parts();
        self.ptys.spawn(pty)?;
        let previous = self.records.insert(
            handle.terminal_id(),
            ManagedTerminalRecord {
                handle,
                working_directory,
                output: Vec::new(),
            },
        );
        debug_assert!(
            previous.is_none(),
            "PTY registry rejected duplicate identity"
        );
        Ok(handle)
    }

    pub fn handles(&self) -> impl Iterator<Item = ManagedTerminalHandle> + '_ {
        self.records.values().map(|record| record.handle)
    }

    pub fn view(
        &self,
        handle: ManagedTerminalHandle,
    ) -> Result<ManagedTerminalView, ManagedTerminalError> {
        let record = self.record(handle)?;
        let session = self.ptys.session(handle.terminal_id())?;
        Ok(ManagedTerminalView {
            handle,
            working_directory: record.working_directory.clone(),
            lifecycle: session.lifecycle().clone(),
            output: record.output.clone(),
            output_eof: session.output_eof(),
        })
    }

    /// Drains native output and appends it to the exact terminal transcript.
    pub fn read_available(
        &mut self,
        handle: ManagedTerminalHandle,
    ) -> Result<Vec<PtyOutputChunk>, ManagedTerminalError> {
        self.ensure_binding(handle)?;
        let chunks = self
            .ptys
            .session_mut(handle.terminal_id())?
            .read_available()?;
        let record = self
            .records
            .get_mut(&handle.terminal_id())
            .ok_or(ManagedTerminalError::MissingRecord(handle.terminal_id()))?;
        let mut expected = record
            .output
            .last()
            .map_or(0, |chunk| chunk.sequence().saturating_add(1));
        for chunk in &chunks {
            if chunk.sequence() != expected {
                return Err(ManagedTerminalError::OutputSequenceGap {
                    terminal_id: handle.terminal_id(),
                    expected,
                    found: chunk.sequence(),
                });
            }
            expected = expected.saturating_add(1);
        }
        record.output.extend(chunks.iter().cloned());
        Ok(chunks)
    }

    pub fn write_input(
        &mut self,
        handle: ManagedTerminalHandle,
        bytes: &[u8],
    ) -> Result<(), ManagedTerminalError> {
        self.ensure_binding(handle)?;
        self.ptys
            .session_mut(handle.terminal_id())?
            .write_input(bytes)?;
        Ok(())
    }

    pub fn close_input(
        &mut self,
        handle: ManagedTerminalHandle,
    ) -> Result<(), ManagedTerminalError> {
        self.ensure_binding(handle)?;
        self.ptys.session_mut(handle.terminal_id())?.close_input()?;
        Ok(())
    }

    pub fn resize(
        &mut self,
        handle: ManagedTerminalHandle,
        dimensions: PtyDimensions,
    ) -> Result<(), ManagedTerminalError> {
        self.ensure_binding(handle)?;
        self.ptys
            .session_mut(handle.terminal_id())?
            .resize(dimensions)?;
        Ok(())
    }

    pub fn poll_exit(
        &mut self,
        handle: ManagedTerminalHandle,
    ) -> Result<Option<PtyExit>, ManagedTerminalError> {
        self.ensure_binding(handle)?;
        Ok(self.ptys.session_mut(handle.terminal_id())?.poll_exit()?)
    }

    pub fn terminate(
        &mut self,
        handle: ManagedTerminalHandle,
    ) -> Result<PtyExit, ManagedTerminalError> {
        self.ensure_binding(handle)?;
        Ok(self.ptys.session_mut(handle.terminal_id())?.terminate()?)
    }

    pub fn remove_exited(
        &mut self,
        handle: ManagedTerminalHandle,
    ) -> Result<ManagedTerminalView, ManagedTerminalError> {
        self.ensure_binding(handle)?;
        let view = self.view(handle)?;
        self.ptys.remove_exited(handle.terminal_id())?;
        self.records
            .remove(&handle.terminal_id())
            .ok_or(ManagedTerminalError::MissingRecord(handle.terminal_id()))?;
        Ok(view)
    }

    fn record(
        &self,
        handle: ManagedTerminalHandle,
    ) -> Result<&ManagedTerminalRecord, ManagedTerminalError> {
        self.ensure_binding(handle)?;
        self.records
            .get(&handle.terminal_id())
            .ok_or(ManagedTerminalError::MissingRecord(handle.terminal_id()))
    }

    fn ensure_binding(&self, handle: ManagedTerminalHandle) -> Result<(), ManagedTerminalError> {
        let record = self.records.get(&handle.terminal_id()).ok_or_else(|| {
            ManagedTerminalError::Pty(PtyError::UnknownTerminal(handle.terminal_id()))
        })?;
        if record.handle.project_id() != handle.project_id()
            || record.handle.repository_id() != handle.repository_id()
        {
            return Err(ManagedTerminalError::BindingMismatch {
                terminal_id: handle.terminal_id(),
                expected_project: record.handle.project_id(),
                expected_repository: record.handle.repository_id(),
                found_project: handle.project_id(),
                found_repository: handle.repository_id(),
            });
        }
        Ok(())
    }
}
