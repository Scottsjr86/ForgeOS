//! Canonical durable workspace payload used by V1 recovery.
//!
//! This payload stores only source-independent state that can be restored safely:
//! current project identity, unsaved editor bytes, terminal metadata, and service
//! lifecycle evidence. It never represents a terminal or service as live after a
//! restart and never contains an executable action.

use crate::project_registry::{ProjectRegistryStateError, SafeWorkspaceSnapshot};
use forge_protocol::hashes::{ContentHash, HashDomain, hash_canonical_bytes};
use forge_protocol::identities::{
    IDENTITY_BYTES, ProcessId, ProjectId, RepositoryId, SessionId, TerminalId,
};
use std::collections::BTreeSet;
use std::fmt;

const MAGIC: [u8; 8] = *b"FGWSPAY\0";
/// Version stored in [`SafeWorkspaceSnapshot::schema_version`].
pub const WORKSPACE_PAYLOAD_SCHEMA_VERSION: u16 = 1;
const MAX_BUFFERS: usize = 256;
const MAX_TERMINALS: usize = 128;
const MAX_SERVICES: usize = 64;
const MAX_PATH_BYTES: usize = 16 * 1024;
const MAX_BUFFER_BYTES: usize = 2 * 1024 * 1024;
const MAX_SERVICE_NAME_BYTES: usize = 64;

/// Last disk state against which unsaved bytes were edited.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecoveredDiskBaseline {
    Missing,
    Existing {
        content_hash: ContentHash,
        length: u64,
    },
}

impl RecoveredDiskBaseline {
    pub const fn content_hash(self) -> Option<ContentHash> {
        match self {
            Self::Missing => None,
            Self::Existing { content_hash, .. } => Some(content_hash),
        }
    }

    pub const fn length(self) -> Option<u64> {
        match self {
            Self::Missing => None,
            Self::Existing { length, .. } => Some(length),
        }
    }
}

/// One dirty or conflicted editor buffer that may be restored without writing disk.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecoveredBuffer {
    buffer_id: [u8; IDENTITY_BYTES],
    relative_path: Vec<u8>,
    content_version: u64,
    cursor_anchor: u64,
    cursor_head: u64,
    base: RecoveredDiskBaseline,
    observed: Option<RecoveredDiskBaseline>,
    bytes: Vec<u8>,
    content_hash: ContentHash,
}

impl RecoveredBuffer {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        buffer_id: [u8; IDENTITY_BYTES],
        relative_path: Vec<u8>,
        content_version: u64,
        cursor_anchor: u64,
        cursor_head: u64,
        base: RecoveredDiskBaseline,
        observed: Option<RecoveredDiskBaseline>,
        bytes: Vec<u8>,
    ) -> Result<Self, WorkspacePayloadError> {
        if relative_path.is_empty() || relative_path.len() > MAX_PATH_BYTES {
            return Err(WorkspacePayloadError::InvalidPathLength(
                relative_path.len(),
            ));
        }
        validate_relative_path_bytes(&relative_path)?;
        if content_version == 0 {
            return Err(WorkspacePayloadError::ReservedContentVersion);
        }
        if bytes.len() > MAX_BUFFER_BYTES {
            return Err(WorkspacePayloadError::BufferTooLarge {
                maximum: MAX_BUFFER_BYTES,
                actual: bytes.len(),
            });
        }
        if cursor_anchor > bytes.len() as u64 || cursor_head > bytes.len() as u64 {
            return Err(WorkspacePayloadError::CursorOutsideBuffer {
                anchor: cursor_anchor,
                head: cursor_head,
                length: bytes.len() as u64,
            });
        }
        let content_hash = hash_canonical_bytes(HashDomain::File, &bytes);
        Ok(Self {
            buffer_id,
            relative_path,
            content_version,
            cursor_anchor,
            cursor_head,
            base,
            observed,
            bytes,
            content_hash,
        })
    }

    pub const fn buffer_id(&self) -> &[u8; IDENTITY_BYTES] {
        &self.buffer_id
    }
    pub fn relative_path(&self) -> &[u8] {
        &self.relative_path
    }
    pub const fn content_version(&self) -> u64 {
        self.content_version
    }
    pub const fn cursor_anchor(&self) -> u64 {
        self.cursor_anchor
    }
    pub const fn cursor_head(&self) -> u64 {
        self.cursor_head
    }
    pub const fn base(&self) -> RecoveredDiskBaseline {
        self.base
    }
    pub const fn observed(&self) -> Option<RecoveredDiskBaseline> {
        self.observed
    }
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }
    pub const fn content_hash(&self) -> ContentHash {
        self.content_hash
    }
}

/// Truthful terminal state after restart. Running terminals are never resurrected.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecoveredTerminalState {
    RequiresRestart,
    Exited {
        code: u32,
        terminated_by_operator: bool,
    },
}

/// Metadata for one terminal that existed before recovery.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecoveredTerminal {
    terminal_id: TerminalId,
    working_directory: Vec<u8>,
    state: RecoveredTerminalState,
}

impl RecoveredTerminal {
    pub fn new(
        terminal_id: TerminalId,
        working_directory: Vec<u8>,
        state: RecoveredTerminalState,
    ) -> Result<Self, WorkspacePayloadError> {
        if working_directory.len() > MAX_PATH_BYTES {
            return Err(WorkspacePayloadError::InvalidPathLength(
                working_directory.len(),
            ));
        }
        if !working_directory.is_empty() {
            validate_relative_path_bytes(&working_directory)?;
        }
        Ok(Self {
            terminal_id,
            working_directory,
            state,
        })
    }
    pub const fn terminal_id(&self) -> TerminalId {
        self.terminal_id
    }
    /// Empty means the repository root; otherwise exact repository-relative bytes.
    pub fn working_directory(&self) -> &[u8] {
        &self.working_directory
    }
    pub const fn state(&self) -> RecoveredTerminalState {
        self.state
    }
    pub const fn claims_alive(&self) -> bool {
        false
    }
}

/// Non-live service status retained across a restart.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecoveredServiceState {
    Stopped,
    RequiresRevalidation,
    RestartPending,
    Failed,
}

/// Metadata for one externally owned service.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecoveredService {
    name: String,
    prior_process_id: Option<ProcessId>,
    state: RecoveredServiceState,
}

impl RecoveredService {
    pub fn new(
        name: impl Into<String>,
        prior_process_id: Option<ProcessId>,
        state: RecoveredServiceState,
    ) -> Result<Self, WorkspacePayloadError> {
        let name = name.into();
        if name.is_empty() || name.len() > MAX_SERVICE_NAME_BYTES || !name.is_ascii() {
            return Err(WorkspacePayloadError::InvalidServiceName(name));
        }
        match state {
            RecoveredServiceState::RequiresRevalidation if prior_process_id.is_none() => {
                return Err(WorkspacePayloadError::MissingPriorProcessId(name));
            }
            RecoveredServiceState::Stopped
            | RecoveredServiceState::RestartPending
            | RecoveredServiceState::Failed
                if prior_process_id.is_some() =>
            {
                return Err(WorkspacePayloadError::NonLiveServiceRetainsProcess(name));
            }
            _ => {}
        }
        Ok(Self {
            name,
            prior_process_id,
            state,
        })
    }
    pub fn name(&self) -> &str {
        &self.name
    }
    pub const fn prior_process_id(&self) -> Option<ProcessId> {
        self.prior_process_id
    }
    pub const fn state(&self) -> RecoveredServiceState {
        self.state
    }
    pub const fn claims_alive(&self) -> bool {
        false
    }
}

/// Canonical safe workspace state restored only after explicit recovery selection.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DurableWorkspaceState {
    project_id: ProjectId,
    repository_id: RepositoryId,
    session_id: SessionId,
    buffers: Vec<RecoveredBuffer>,
    terminals: Vec<RecoveredTerminal>,
    services: Vec<RecoveredService>,
}

impl DurableWorkspaceState {
    pub fn new(
        project_id: ProjectId,
        repository_id: RepositoryId,
        session_id: SessionId,
        mut buffers: Vec<RecoveredBuffer>,
        mut terminals: Vec<RecoveredTerminal>,
        mut services: Vec<RecoveredService>,
    ) -> Result<Self, WorkspacePayloadError> {
        if buffers.len() > MAX_BUFFERS {
            return Err(WorkspacePayloadError::TooManyBuffers(buffers.len()));
        }
        if terminals.len() > MAX_TERMINALS {
            return Err(WorkspacePayloadError::TooManyTerminals(terminals.len()));
        }
        if services.len() > MAX_SERVICES {
            return Err(WorkspacePayloadError::TooManyServices(services.len()));
        }
        buffers.sort_by(|left, right| left.buffer_id.cmp(&right.buffer_id));
        terminals.sort_by_key(RecoveredTerminal::terminal_id);
        services.sort_by(|left, right| left.name.as_bytes().cmp(right.name.as_bytes()));
        ensure_unique_buffers(&buffers)?;
        ensure_unique_terminals(&terminals)?;
        ensure_unique_services(&services)?;
        Ok(Self {
            project_id,
            repository_id,
            session_id,
            buffers,
            terminals,
            services,
        })
    }

    pub const fn project_id(&self) -> ProjectId {
        self.project_id
    }
    pub const fn repository_id(&self) -> RepositoryId {
        self.repository_id
    }
    pub const fn session_id(&self) -> SessionId {
        self.session_id
    }
    pub fn buffers(&self) -> &[RecoveredBuffer] {
        &self.buffers
    }
    pub fn terminals(&self) -> &[RecoveredTerminal] {
        &self.terminals
    }
    pub fn services(&self) -> &[RecoveredService] {
        &self.services
    }

    pub fn to_safe_snapshot(&self) -> Result<SafeWorkspaceSnapshot, WorkspacePayloadError> {
        SafeWorkspaceSnapshot::new(WORKSPACE_PAYLOAD_SCHEMA_VERSION, self.encode())
            .map_err(WorkspacePayloadError::Snapshot)
    }

    pub fn from_safe_snapshot(
        snapshot: &SafeWorkspaceSnapshot,
    ) -> Result<Self, WorkspacePayloadError> {
        if snapshot.schema_version() != WORKSPACE_PAYLOAD_SCHEMA_VERSION {
            return Err(WorkspacePayloadError::UnsupportedSchema(
                snapshot.schema_version(),
            ));
        }
        Self::decode(snapshot.payload())
    }

    fn encode(&self) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(&MAGIC);
        out.extend_from_slice(&WORKSPACE_PAYLOAD_SCHEMA_VERSION.to_be_bytes());
        out.extend_from_slice(self.project_id.as_bytes());
        out.extend_from_slice(self.repository_id.as_bytes());
        out.extend_from_slice(self.session_id.as_bytes());
        push_u16(&mut out, self.buffers.len());
        for buffer in &self.buffers {
            out.extend_from_slice(&buffer.buffer_id);
            push_bytes(&mut out, &buffer.relative_path);
            out.extend_from_slice(&buffer.content_version.to_be_bytes());
            out.extend_from_slice(&buffer.cursor_anchor.to_be_bytes());
            out.extend_from_slice(&buffer.cursor_head.to_be_bytes());
            encode_baseline(&mut out, buffer.base);
            match buffer.observed {
                Some(observed) => {
                    out.push(1);
                    encode_baseline(&mut out, observed);
                }
                None => out.push(0),
            }
            out.extend_from_slice(buffer.content_hash.as_bytes());
            push_bytes(&mut out, &buffer.bytes);
        }
        push_u16(&mut out, self.terminals.len());
        for terminal in &self.terminals {
            out.extend_from_slice(terminal.terminal_id.as_bytes());
            push_bytes(&mut out, &terminal.working_directory);
            match terminal.state {
                RecoveredTerminalState::RequiresRestart => out.push(1),
                RecoveredTerminalState::Exited {
                    code,
                    terminated_by_operator,
                } => {
                    out.push(2);
                    out.extend_from_slice(&code.to_be_bytes());
                    out.push(u8::from(terminated_by_operator));
                }
            }
        }
        push_u16(&mut out, self.services.len());
        for service in &self.services {
            push_bytes(&mut out, service.name.as_bytes());
            match service.prior_process_id {
                Some(process_id) => {
                    out.push(1);
                    out.extend_from_slice(process_id.as_bytes());
                }
                None => out.push(0),
            }
            out.push(match service.state {
                RecoveredServiceState::Stopped => 1,
                RecoveredServiceState::RequiresRevalidation => 2,
                RecoveredServiceState::RestartPending => 3,
                RecoveredServiceState::Failed => 4,
            });
        }
        out
    }

    fn decode(bytes: &[u8]) -> Result<Self, WorkspacePayloadError> {
        let mut decoder = Decoder::new(bytes);
        if decoder.take(MAGIC.len())? != MAGIC.as_slice() {
            return Err(WorkspacePayloadError::InvalidMagic);
        }
        let schema = decoder.u16()?;
        if schema != WORKSPACE_PAYLOAD_SCHEMA_VERSION {
            return Err(WorkspacePayloadError::UnsupportedSchema(schema));
        }
        let project_id = ProjectId::from_bytes(decoder.identity()?);
        let repository_id = RepositoryId::from_bytes(decoder.identity()?);
        let session_id = SessionId::from_bytes(decoder.identity()?);
        let buffer_count = decoder.u16()? as usize;
        if buffer_count > MAX_BUFFERS {
            return Err(WorkspacePayloadError::TooManyBuffers(buffer_count));
        }
        let mut buffers = Vec::with_capacity(buffer_count);
        for _ in 0..buffer_count {
            let buffer_id = decoder.identity()?;
            let path = decoder.bytes(MAX_PATH_BYTES)?;
            let content_version = decoder.u64()?;
            let cursor_anchor = decoder.u64()?;
            let cursor_head = decoder.u64()?;
            let base = decode_baseline(&mut decoder)?;
            let observed = match decoder.u8()? {
                0 => None,
                1 => Some(decode_baseline(&mut decoder)?),
                found => return Err(WorkspacePayloadError::InvalidPresence(found)),
            };
            let expected_hash = ContentHash::from_bytes(decoder.hash()?);
            let content = decoder.bytes(MAX_BUFFER_BYTES)?;
            let buffer = RecoveredBuffer::new(
                buffer_id,
                path,
                content_version,
                cursor_anchor,
                cursor_head,
                base,
                observed,
                content,
            )?;
            if buffer.content_hash != expected_hash {
                return Err(WorkspacePayloadError::ContentHashMismatch);
            }
            buffers.push(buffer);
        }
        let terminal_count = decoder.u16()? as usize;
        if terminal_count > MAX_TERMINALS {
            return Err(WorkspacePayloadError::TooManyTerminals(terminal_count));
        }
        let mut terminals = Vec::with_capacity(terminal_count);
        for _ in 0..terminal_count {
            let terminal_id = TerminalId::from_bytes(decoder.identity()?);
            let directory = decoder.bytes(MAX_PATH_BYTES)?;
            let state = match decoder.u8()? {
                1 => RecoveredTerminalState::RequiresRestart,
                2 => RecoveredTerminalState::Exited {
                    code: decoder.u32()?,
                    terminated_by_operator: decoder.bool()?,
                },
                found => return Err(WorkspacePayloadError::InvalidTerminalState(found)),
            };
            terminals.push(RecoveredTerminal::new(terminal_id, directory, state)?);
        }
        let service_count = decoder.u16()? as usize;
        if service_count > MAX_SERVICES {
            return Err(WorkspacePayloadError::TooManyServices(service_count));
        }
        let mut services = Vec::with_capacity(service_count);
        for _ in 0..service_count {
            let name = String::from_utf8(decoder.bytes(MAX_SERVICE_NAME_BYTES)?)
                .map_err(|_| WorkspacePayloadError::InvalidServiceEncoding)?;
            let process_id = match decoder.u8()? {
                0 => None,
                1 => Some(ProcessId::from_bytes(decoder.identity()?)),
                found => return Err(WorkspacePayloadError::InvalidPresence(found)),
            };
            let state = match decoder.u8()? {
                1 => RecoveredServiceState::Stopped,
                2 => RecoveredServiceState::RequiresRevalidation,
                3 => RecoveredServiceState::RestartPending,
                4 => RecoveredServiceState::Failed,
                found => return Err(WorkspacePayloadError::InvalidServiceState(found)),
            };
            services.push(RecoveredService::new(name, process_id, state)?);
        }
        decoder.finish()?;
        Self::new(
            project_id,
            repository_id,
            session_id,
            buffers,
            terminals,
            services,
        )
    }
}

fn validate_relative_path_bytes(path: &[u8]) -> Result<(), WorkspacePayloadError> {
    if path.starts_with(b"/") || path.ends_with(b"/") || path.contains(&0) {
        return Err(WorkspacePayloadError::InvalidRelativePath);
    }
    if path
        .split(|byte| *byte == b'/')
        .any(|component| component.is_empty() || component == b"." || component == b"..")
    {
        return Err(WorkspacePayloadError::InvalidRelativePath);
    }
    Ok(())
}

fn ensure_unique_buffers(buffers: &[RecoveredBuffer]) -> Result<(), WorkspacePayloadError> {
    let mut seen = BTreeSet::new();
    for buffer in buffers {
        if !seen.insert(buffer.buffer_id) {
            return Err(WorkspacePayloadError::DuplicateBuffer(buffer.buffer_id));
        }
    }
    Ok(())
}

fn ensure_unique_terminals(terminals: &[RecoveredTerminal]) -> Result<(), WorkspacePayloadError> {
    let mut seen = BTreeSet::new();
    for terminal in terminals {
        if !seen.insert(terminal.terminal_id) {
            return Err(WorkspacePayloadError::DuplicateTerminal(
                terminal.terminal_id,
            ));
        }
    }
    Ok(())
}

fn ensure_unique_services(services: &[RecoveredService]) -> Result<(), WorkspacePayloadError> {
    let mut seen = BTreeSet::new();
    for service in services {
        if !seen.insert(service.name.as_str()) {
            return Err(WorkspacePayloadError::DuplicateService(
                service.name.clone(),
            ));
        }
    }
    Ok(())
}

fn encode_baseline(out: &mut Vec<u8>, baseline: RecoveredDiskBaseline) {
    match baseline {
        RecoveredDiskBaseline::Missing => out.push(0),
        RecoveredDiskBaseline::Existing {
            content_hash,
            length,
        } => {
            out.push(1);
            out.extend_from_slice(content_hash.as_bytes());
            out.extend_from_slice(&length.to_be_bytes());
        }
    }
}

fn decode_baseline(
    decoder: &mut Decoder<'_>,
) -> Result<RecoveredDiskBaseline, WorkspacePayloadError> {
    match decoder.u8()? {
        0 => Ok(RecoveredDiskBaseline::Missing),
        1 => Ok(RecoveredDiskBaseline::Existing {
            content_hash: ContentHash::from_bytes(decoder.hash()?),
            length: decoder.u64()?,
        }),
        found => Err(WorkspacePayloadError::InvalidBaseline(found)),
    }
}

fn push_u16(out: &mut Vec<u8>, value: usize) {
    out.extend_from_slice(&(value as u16).to_be_bytes());
}

fn push_bytes(out: &mut Vec<u8>, bytes: &[u8]) {
    out.extend_from_slice(&(bytes.len() as u32).to_be_bytes());
    out.extend_from_slice(bytes);
}

struct Decoder<'a> {
    bytes: &'a [u8],
    offset: usize,
}

impl<'a> Decoder<'a> {
    const fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, offset: 0 }
    }
    fn take(&mut self, length: usize) -> Result<&'a [u8], WorkspacePayloadError> {
        let end = self
            .offset
            .checked_add(length)
            .ok_or(WorkspacePayloadError::Truncated)?;
        let value = self
            .bytes
            .get(self.offset..end)
            .ok_or(WorkspacePayloadError::Truncated)?;
        self.offset = end;
        Ok(value)
    }
    fn u8(&mut self) -> Result<u8, WorkspacePayloadError> {
        Ok(self.take(1)?[0])
    }
    fn bool(&mut self) -> Result<bool, WorkspacePayloadError> {
        match self.u8()? {
            0 => Ok(false),
            1 => Ok(true),
            found => Err(WorkspacePayloadError::InvalidBoolean(found)),
        }
    }
    fn u16(&mut self) -> Result<u16, WorkspacePayloadError> {
        Ok(u16::from_be_bytes(
            self.take(2)?.try_into().expect("length"),
        ))
    }
    fn u32(&mut self) -> Result<u32, WorkspacePayloadError> {
        Ok(u32::from_be_bytes(
            self.take(4)?.try_into().expect("length"),
        ))
    }
    fn u64(&mut self) -> Result<u64, WorkspacePayloadError> {
        Ok(u64::from_be_bytes(
            self.take(8)?.try_into().expect("length"),
        ))
    }
    fn identity(&mut self) -> Result<[u8; IDENTITY_BYTES], WorkspacePayloadError> {
        Ok(self
            .take(IDENTITY_BYTES)?
            .try_into()
            .expect("identity length"))
    }
    fn hash(&mut self) -> Result<[u8; 32], WorkspacePayloadError> {
        Ok(self.take(32)?.try_into().expect("hash length"))
    }
    fn bytes(&mut self, maximum: usize) -> Result<Vec<u8>, WorkspacePayloadError> {
        let length = self.u32()? as usize;
        if length > maximum {
            return Err(WorkspacePayloadError::FieldTooLarge {
                maximum,
                actual: length,
            });
        }
        Ok(self.take(length)?.to_vec())
    }
    fn finish(self) -> Result<(), WorkspacePayloadError> {
        if self.offset == self.bytes.len() {
            Ok(())
        } else {
            Err(WorkspacePayloadError::TrailingBytes(
                self.bytes.len() - self.offset,
            ))
        }
    }
}

/// Exact reason a durable workspace payload was rejected.
#[derive(Debug)]
pub enum WorkspacePayloadError {
    Snapshot(ProjectRegistryStateError),
    InvalidMagic,
    UnsupportedSchema(u16),
    Truncated,
    TrailingBytes(usize),
    FieldTooLarge { maximum: usize, actual: usize },
    InvalidPathLength(usize),
    InvalidRelativePath,
    ReservedContentVersion,
    BufferTooLarge { maximum: usize, actual: usize },
    CursorOutsideBuffer { anchor: u64, head: u64, length: u64 },
    ContentHashMismatch,
    TooManyBuffers(usize),
    TooManyTerminals(usize),
    TooManyServices(usize),
    DuplicateBuffer([u8; IDENTITY_BYTES]),
    DuplicateTerminal(TerminalId),
    DuplicateService(String),
    InvalidPresence(u8),
    InvalidBoolean(u8),
    InvalidBaseline(u8),
    InvalidTerminalState(u8),
    InvalidServiceState(u8),
    InvalidServiceName(String),
    InvalidServiceEncoding,
    MissingPriorProcessId(String),
    NonLiveServiceRetainsProcess(String),
}

impl fmt::Display for WorkspacePayloadError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "invalid durable workspace payload: {self:?}")
    }
}
impl std::error::Error for WorkspacePayloadError {}
