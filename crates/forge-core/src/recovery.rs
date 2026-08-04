//! Canonical workspace recovery snapshot and crash-journal state.
//!
//! The record contains only safe restorable workspace bytes, unresolved action
//! evidence, and historical process observations. It never claims a recorded
//! process is still alive and never authorizes replay of an interrupted action.

use crate::hashing::state_record_hash;
use crate::project_registry::{ProjectRegistryStateError, SafeWorkspaceSnapshot};
use crate::state::{StateRecord, StateRecordError};
use forge_protocol::hashes::ContentHash;
use forge_protocol::identities::{ProcessId, ProjectId, SessionId, IDENTITY_BYTES};
use std::collections::BTreeSet;
use std::fmt;

const RECOVERY_MAGIC: [u8; 8] = *b"FGRECOV\0";
const RECOVERY_SCHEMA_VERSION: u16 = 1;
const MAX_INTERRUPTED_ACTIONS: usize = 4096;
const MAX_RECORDED_PROCESSES: usize = 256;
const MAX_SERVICE_NAME_BYTES: usize = 64;

/// State-record type reserved for the canonical V1 workspace recovery image.
pub const WORKSPACE_RECOVERY_RECORD_TYPE: u16 = 0x0103;

/// Broad class of an action that was not conclusively completed before a crash.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(u8)]
pub enum RecoveryActionKind {
    FileWrite = 1,
    Command = 2,
    GitMutation = 3,
    ServiceTransition = 4,
    Other = 255,
}

impl RecoveryActionKind {
    fn from_code(code: u8) -> Result<Self, WorkspaceRecoveryError> {
        match code {
            1 => Ok(Self::FileWrite),
            2 => Ok(Self::Command),
            3 => Ok(Self::GitMutation),
            4 => Ok(Self::ServiceTransition),
            255 => Ok(Self::Other),
            found => Err(WorkspaceRecoveryError::InvalidActionKind(found)),
        }
    }
}

/// What can truthfully be said about the side effect of an interrupted action.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(u8)]
pub enum InterruptedEffectState {
    /// The adapter had not reported a committed side effect.
    CommitNotObserved = 1,
    /// The side effect may have committed and must be inspected manually.
    CommitUnknown = 2,
}

impl InterruptedEffectState {
    fn from_code(code: u8) -> Result<Self, WorkspaceRecoveryError> {
        match code {
            1 => Ok(Self::CommitNotObserved),
            2 => Ok(Self::CommitUnknown),
            found => Err(WorkspaceRecoveryError::InvalidEffectState(found)),
        }
    }
}

/// One unresolved operation retained for inspection. It is never replayable.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InterruptedAction {
    request_identity: ContentHash,
    kind: RecoveryActionKind,
    effect_state: InterruptedEffectState,
}

impl InterruptedAction {
    pub const fn new(
        request_identity: ContentHash,
        kind: RecoveryActionKind,
        effect_state: InterruptedEffectState,
    ) -> Self {
        Self {
            request_identity,
            kind,
            effect_state,
        }
    }

    pub const fn request_identity(&self) -> ContentHash {
        self.request_identity
    }

    pub const fn kind(&self) -> RecoveryActionKind {
        self.kind
    }

    pub const fn effect_state(&self) -> InterruptedEffectState {
        self.effect_state
    }

    /// Interrupted actions always require explicit inspection and never replay.
    pub const fn replay_allowed(&self) -> bool {
        false
    }
}

/// Conservative status assigned to a process observation after restart.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(u8)]
pub enum RecoveredProcessState {
    /// The process was observed active before the crash and must be probed again.
    RequiresRevalidation = 1,
    /// The process had already been confirmed stopped.
    ConfirmedStopped = 2,
}

impl RecoveredProcessState {
    fn from_code(code: u8) -> Result<Self, WorkspaceRecoveryError> {
        match code {
            1 => Ok(Self::RequiresRevalidation),
            2 => Ok(Self::ConfirmedStopped),
            found => Err(WorkspaceRecoveryError::InvalidProcessState(found)),
        }
    }
}

/// Historical process information retained without claiming current liveness.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecordedProcess {
    service_name: String,
    prior_process_id: Option<ProcessId>,
    state: RecoveredProcessState,
}

impl RecordedProcess {
    pub fn new(
        service_name: impl Into<String>,
        prior_process_id: Option<ProcessId>,
        state: RecoveredProcessState,
    ) -> Result<Self, WorkspaceRecoveryError> {
        let service_name = service_name.into();
        validate_service_name(&service_name)?;
        match (prior_process_id, state) {
            (None, RecoveredProcessState::RequiresRevalidation) => {
                return Err(WorkspaceRecoveryError::MissingPriorProcessId)
            }
            (Some(_), RecoveredProcessState::ConfirmedStopped) => {
                return Err(WorkspaceRecoveryError::StoppedProcessRetainsIdentity)
            }
            _ => {}
        }
        Ok(Self {
            service_name,
            prior_process_id,
            state,
        })
    }

    pub fn service_name(&self) -> &str {
        &self.service_name
    }

    pub const fn prior_process_id(&self) -> Option<ProcessId> {
        self.prior_process_id
    }

    pub const fn state(&self) -> RecoveredProcessState {
        self.state
    }

    /// Recovery never treats historical process metadata as a live-process claim.
    pub const fn claims_alive(&self) -> bool {
        false
    }
}

/// One canonical, versioned workspace recovery image.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceRecoveryRecord {
    generation: u64,
    project_id: ProjectId,
    session_id: Option<SessionId>,
    safe_snapshot: SafeWorkspaceSnapshot,
    interrupted_actions: Vec<InterruptedAction>,
    recorded_processes: Vec<RecordedProcess>,
}

impl WorkspaceRecoveryRecord {
    pub fn new(
        generation: u64,
        project_id: ProjectId,
        session_id: Option<SessionId>,
        safe_snapshot: SafeWorkspaceSnapshot,
        mut interrupted_actions: Vec<InterruptedAction>,
        mut recorded_processes: Vec<RecordedProcess>,
    ) -> Result<Self, WorkspaceRecoveryError> {
        if generation == 0 {
            return Err(WorkspaceRecoveryError::ReservedGeneration);
        }
        if interrupted_actions.len() > MAX_INTERRUPTED_ACTIONS {
            return Err(WorkspaceRecoveryError::TooManyInterruptedActions {
                maximum: MAX_INTERRUPTED_ACTIONS,
                actual: interrupted_actions.len(),
            });
        }
        if recorded_processes.len() > MAX_RECORDED_PROCESSES {
            return Err(WorkspaceRecoveryError::TooManyRecordedProcesses {
                maximum: MAX_RECORDED_PROCESSES,
                actual: recorded_processes.len(),
            });
        }

        interrupted_actions.sort_by_key(|entry| entry.request_identity);
        ensure_unique_actions(&interrupted_actions)?;
        recorded_processes.sort_by(|left, right| {
            left.service_name
                .as_bytes()
                .cmp(right.service_name.as_bytes())
        });
        ensure_unique_processes(&recorded_processes)?;

        Ok(Self {
            generation,
            project_id,
            session_id,
            safe_snapshot,
            interrupted_actions,
            recorded_processes,
        })
    }

    pub const fn generation(&self) -> u64 {
        self.generation
    }

    pub const fn project_id(&self) -> ProjectId {
        self.project_id
    }

    pub const fn session_id(&self) -> Option<SessionId> {
        self.session_id
    }

    pub const fn safe_snapshot(&self) -> &SafeWorkspaceSnapshot {
        &self.safe_snapshot
    }

    pub fn interrupted_actions(&self) -> &[InterruptedAction] {
        &self.interrupted_actions
    }

    pub fn recorded_processes(&self) -> &[RecordedProcess] {
        &self.recorded_processes
    }

    pub fn to_state_record(&self) -> Result<StateRecord, WorkspaceRecoveryError> {
        StateRecord::new(WORKSPACE_RECOVERY_RECORD_TYPE, self.encode_payload())
            .map_err(WorkspaceRecoveryError::State)
    }

    pub fn from_state_record(record: &StateRecord) -> Result<Self, WorkspaceRecoveryError> {
        if record.record_type() != WORKSPACE_RECOVERY_RECORD_TYPE {
            return Err(WorkspaceRecoveryError::WrongRecordType {
                expected: WORKSPACE_RECOVERY_RECORD_TYPE,
                found: record.record_type(),
            });
        }
        Self::decode_payload(record.payload())
    }

    pub fn identity(&self) -> Result<ContentHash, WorkspaceRecoveryError> {
        Ok(state_record_hash(&self.to_state_record()?))
    }

    fn encode_payload(&self) -> Vec<u8> {
        let snapshot = self.safe_snapshot.payload();
        let action_bytes = self.interrupted_actions.len() * (32 + 1 + 1);
        let process_bytes = self
            .recorded_processes
            .iter()
            .map(|process| 2 + process.service_name.len() + 1 + IDENTITY_BYTES + 1)
            .sum::<usize>();
        let mut bytes = Vec::with_capacity(
            RECOVERY_MAGIC.len()
                + 2
                + 8
                + IDENTITY_BYTES
                + 1
                + IDENTITY_BYTES
                + 2
                + 4
                + snapshot.len()
                + 32
                + 2
                + action_bytes
                + 2
                + process_bytes,
        );
        bytes.extend_from_slice(&RECOVERY_MAGIC);
        bytes.extend_from_slice(&RECOVERY_SCHEMA_VERSION.to_be_bytes());
        bytes.extend_from_slice(&self.generation.to_be_bytes());
        bytes.extend_from_slice(self.project_id.as_bytes());
        match self.session_id {
            Some(session_id) => {
                bytes.push(1);
                bytes.extend_from_slice(session_id.as_bytes());
            }
            None => bytes.push(0),
        }
        bytes.extend_from_slice(&self.safe_snapshot.schema_version().to_be_bytes());
        bytes.extend_from_slice(&(snapshot.len() as u32).to_be_bytes());
        bytes.extend_from_slice(snapshot);
        bytes.extend_from_slice(self.safe_snapshot.identity().as_bytes());
        bytes.extend_from_slice(&(self.interrupted_actions.len() as u16).to_be_bytes());
        for action in &self.interrupted_actions {
            bytes.extend_from_slice(action.request_identity.as_bytes());
            bytes.push(action.kind as u8);
            bytes.push(action.effect_state as u8);
        }
        bytes.extend_from_slice(&(self.recorded_processes.len() as u16).to_be_bytes());
        for process in &self.recorded_processes {
            bytes.extend_from_slice(&(process.service_name.len() as u16).to_be_bytes());
            bytes.extend_from_slice(process.service_name.as_bytes());
            match process.prior_process_id {
                Some(process_id) => {
                    bytes.push(1);
                    bytes.extend_from_slice(process_id.as_bytes());
                }
                None => bytes.push(0),
            }
            bytes.push(process.state as u8);
        }
        bytes
    }

    fn decode_payload(bytes: &[u8]) -> Result<Self, WorkspaceRecoveryError> {
        let mut cursor = Cursor::new(bytes);
        if cursor.array::<8>()? != RECOVERY_MAGIC {
            return Err(WorkspaceRecoveryError::InvalidMagic);
        }
        let schema = cursor.u16()?;
        if schema != RECOVERY_SCHEMA_VERSION {
            return Err(WorkspaceRecoveryError::UnsupportedSchemaVersion(schema));
        }
        let generation = cursor.u64()?;
        let project_id = ProjectId::from_bytes(cursor.array::<IDENTITY_BYTES>()?);
        let session_id = match cursor.u8()? {
            0 => None,
            1 => Some(SessionId::from_bytes(cursor.array::<IDENTITY_BYTES>()?)),
            found => return Err(WorkspaceRecoveryError::InvalidOptionalIdentityFlag(found)),
        };
        let snapshot_schema = cursor.u16()?;
        let snapshot_length = cursor.u32()? as usize;
        let snapshot_payload = cursor.bytes(snapshot_length)?.to_vec();
        let expected_snapshot_identity = ContentHash::from_bytes(cursor.array::<32>()?);
        let safe_snapshot = SafeWorkspaceSnapshot::new(snapshot_schema, snapshot_payload)
            .map_err(WorkspaceRecoveryError::Snapshot)?;
        if safe_snapshot.identity() != expected_snapshot_identity {
            return Err(WorkspaceRecoveryError::SnapshotIdentityMismatch {
                expected: expected_snapshot_identity,
                actual: safe_snapshot.identity(),
            });
        }

        let action_count = cursor.u16()? as usize;
        if action_count > MAX_INTERRUPTED_ACTIONS {
            return Err(WorkspaceRecoveryError::TooManyInterruptedActions {
                maximum: MAX_INTERRUPTED_ACTIONS,
                actual: action_count,
            });
        }
        let mut interrupted_actions = Vec::with_capacity(action_count);
        for _ in 0..action_count {
            interrupted_actions.push(InterruptedAction::new(
                ContentHash::from_bytes(cursor.array::<32>()?),
                RecoveryActionKind::from_code(cursor.u8()?)?,
                InterruptedEffectState::from_code(cursor.u8()?)?,
            ));
        }
        ensure_canonical_action_order(&interrupted_actions)?;

        let process_count = cursor.u16()? as usize;
        if process_count > MAX_RECORDED_PROCESSES {
            return Err(WorkspaceRecoveryError::TooManyRecordedProcesses {
                maximum: MAX_RECORDED_PROCESSES,
                actual: process_count,
            });
        }
        let mut recorded_processes = Vec::with_capacity(process_count);
        for _ in 0..process_count {
            let service_name = String::from_utf8(cursor.length_prefixed_u16()?.to_vec())
                .map_err(|_| WorkspaceRecoveryError::InvalidServiceNameUtf8)?;
            let prior_process_id = match cursor.u8()? {
                0 => None,
                1 => Some(ProcessId::from_bytes(cursor.array::<IDENTITY_BYTES>()?)),
                found => return Err(WorkspaceRecoveryError::InvalidOptionalIdentityFlag(found)),
            };
            let state = RecoveredProcessState::from_code(cursor.u8()?)?;
            recorded_processes.push(RecordedProcess::new(service_name, prior_process_id, state)?);
        }
        ensure_canonical_process_order(&recorded_processes)?;
        cursor.finish()?;

        Self::new(
            generation,
            project_id,
            session_id,
            safe_snapshot,
            interrupted_actions,
            recorded_processes,
        )
    }
}

fn validate_service_name(value: &str) -> Result<(), WorkspaceRecoveryError> {
    if value.is_empty() {
        return Err(WorkspaceRecoveryError::EmptyServiceName);
    }
    if value.len() > MAX_SERVICE_NAME_BYTES {
        return Err(WorkspaceRecoveryError::ServiceNameTooLong {
            maximum: MAX_SERVICE_NAME_BYTES,
            actual: value.len(),
        });
    }
    let bytes = value.as_bytes();
    if bytes.first() == Some(&b'-') || bytes.last() == Some(&b'-') {
        return Err(WorkspaceRecoveryError::InvalidServiceName);
    }
    if bytes.windows(2).any(|pair| pair == b"--")
        || bytes
            .iter()
            .any(|byte| !(byte.is_ascii_lowercase() || byte.is_ascii_digit() || *byte == b'-'))
    {
        return Err(WorkspaceRecoveryError::InvalidServiceName);
    }
    Ok(())
}

fn ensure_unique_actions(actions: &[InterruptedAction]) -> Result<(), WorkspaceRecoveryError> {
    let mut seen = BTreeSet::new();
    for action in actions {
        if !seen.insert(action.request_identity) {
            return Err(WorkspaceRecoveryError::DuplicateInterruptedAction(
                action.request_identity,
            ));
        }
    }
    Ok(())
}

fn ensure_unique_processes(processes: &[RecordedProcess]) -> Result<(), WorkspaceRecoveryError> {
    let mut seen = BTreeSet::new();
    for process in processes {
        if !seen.insert(process.service_name.clone()) {
            return Err(WorkspaceRecoveryError::DuplicateRecordedProcess(
                process.service_name.clone(),
            ));
        }
    }
    Ok(())
}

fn ensure_canonical_action_order(
    actions: &[InterruptedAction],
) -> Result<(), WorkspaceRecoveryError> {
    ensure_unique_actions(actions)?;
    if actions
        .windows(2)
        .any(|pair| pair[0].request_identity > pair[1].request_identity)
    {
        return Err(WorkspaceRecoveryError::NonCanonicalActionOrder);
    }
    Ok(())
}

fn ensure_canonical_process_order(
    processes: &[RecordedProcess],
) -> Result<(), WorkspaceRecoveryError> {
    ensure_unique_processes(processes)?;
    if processes
        .windows(2)
        .any(|pair| pair[0].service_name.as_bytes() > pair[1].service_name.as_bytes())
    {
        return Err(WorkspaceRecoveryError::NonCanonicalProcessOrder);
    }
    Ok(())
}

struct Cursor<'a> {
    bytes: &'a [u8],
    offset: usize,
}

impl<'a> Cursor<'a> {
    const fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, offset: 0 }
    }

    fn bytes(&mut self, length: usize) -> Result<&'a [u8], WorkspaceRecoveryError> {
        let end = self
            .offset
            .checked_add(length)
            .ok_or(WorkspaceRecoveryError::LengthOverflow)?;
        let slice = self
            .bytes
            .get(self.offset..end)
            .ok_or(WorkspaceRecoveryError::Truncated)?;
        self.offset = end;
        Ok(slice)
    }

    fn array<const N: usize>(&mut self) -> Result<[u8; N], WorkspaceRecoveryError> {
        let mut value = [0u8; N];
        value.copy_from_slice(self.bytes(N)?);
        Ok(value)
    }

    fn u8(&mut self) -> Result<u8, WorkspaceRecoveryError> {
        Ok(self.bytes(1)?[0])
    }

    fn u16(&mut self) -> Result<u16, WorkspaceRecoveryError> {
        Ok(u16::from_be_bytes(self.array()?))
    }

    fn u32(&mut self) -> Result<u32, WorkspaceRecoveryError> {
        Ok(u32::from_be_bytes(self.array()?))
    }

    fn u64(&mut self) -> Result<u64, WorkspaceRecoveryError> {
        Ok(u64::from_be_bytes(self.array()?))
    }

    fn length_prefixed_u16(&mut self) -> Result<&'a [u8], WorkspaceRecoveryError> {
        let length = self.u16()? as usize;
        self.bytes(length)
    }

    fn finish(self) -> Result<(), WorkspaceRecoveryError> {
        if self.offset == self.bytes.len() {
            Ok(())
        } else {
            Err(WorkspaceRecoveryError::TrailingBytes(
                self.bytes.len() - self.offset,
            ))
        }
    }
}

/// Exact reason a workspace recovery record was rejected.
#[derive(Debug)]
pub enum WorkspaceRecoveryError {
    State(StateRecordError),
    Snapshot(ProjectRegistryStateError),
    WrongRecordType {
        expected: u16,
        found: u16,
    },
    InvalidMagic,
    UnsupportedSchemaVersion(u16),
    ReservedGeneration,
    TooManyInterruptedActions {
        maximum: usize,
        actual: usize,
    },
    TooManyRecordedProcesses {
        maximum: usize,
        actual: usize,
    },
    DuplicateInterruptedAction(ContentHash),
    DuplicateRecordedProcess(String),
    NonCanonicalActionOrder,
    NonCanonicalProcessOrder,
    EmptyServiceName,
    ServiceNameTooLong {
        maximum: usize,
        actual: usize,
    },
    InvalidServiceName,
    InvalidServiceNameUtf8,
    MissingPriorProcessId,
    StoppedProcessRetainsIdentity,
    InvalidActionKind(u8),
    InvalidEffectState(u8),
    InvalidProcessState(u8),
    InvalidOptionalIdentityFlag(u8),
    SnapshotIdentityMismatch {
        expected: ContentHash,
        actual: ContentHash,
    },
    LengthOverflow,
    Truncated,
    TrailingBytes(usize),
}

impl fmt::Display for WorkspaceRecoveryError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::State(source) => write!(formatter, "invalid recovery state record: {source}"),
            Self::Snapshot(source) => {
                write!(formatter, "invalid safe workspace snapshot: {source}")
            }
            Self::WrongRecordType { expected, found } => write!(
                formatter,
                "workspace recovery record type mismatch: expected {expected}, found {found}"
            ),
            Self::InvalidMagic => {
                formatter.write_str("workspace recovery payload has invalid magic")
            }
            Self::UnsupportedSchemaVersion(found) => write!(
                formatter,
                "workspace recovery schema version {found} is unsupported"
            ),
            Self::ReservedGeneration => {
                formatter.write_str("workspace recovery generation zero is reserved")
            }
            Self::TooManyInterruptedActions { maximum, actual } => write!(
                formatter,
                "workspace recovery has {actual} interrupted actions; maximum is {maximum}"
            ),
            Self::TooManyRecordedProcesses { maximum, actual } => write!(
                formatter,
                "workspace recovery has {actual} process records; maximum is {maximum}"
            ),
            Self::DuplicateInterruptedAction(identity) => write!(
                formatter,
                "workspace recovery repeats interrupted action {identity}"
            ),
            Self::DuplicateRecordedProcess(service) => write!(
                formatter,
                "workspace recovery repeats process record for {service}"
            ),
            Self::NonCanonicalActionOrder => {
                formatter.write_str("interrupted actions are not canonically ordered")
            }
            Self::NonCanonicalProcessOrder => {
                formatter.write_str("process records are not canonically ordered")
            }
            Self::EmptyServiceName => formatter.write_str("recovery service name is empty"),
            Self::ServiceNameTooLong { maximum, actual } => write!(
                formatter,
                "recovery service name has {actual} bytes; maximum is {maximum}"
            ),
            Self::InvalidServiceName => {
                formatter.write_str("recovery service name is not lower-kebab")
            }
            Self::InvalidServiceNameUtf8 => {
                formatter.write_str("recovery service name is not UTF-8")
            }
            Self::MissingPriorProcessId => {
                formatter.write_str("process requiring revalidation has no prior process identity")
            }
            Self::StoppedProcessRetainsIdentity => formatter
                .write_str("confirmed-stopped process may not retain a prior live identity"),
            Self::InvalidActionKind(found) => {
                write!(formatter, "invalid recovery action kind {found}")
            }
            Self::InvalidEffectState(found) => {
                write!(formatter, "invalid interrupted effect state {found}")
            }
            Self::InvalidProcessState(found) => {
                write!(formatter, "invalid recovered process state {found}")
            }
            Self::InvalidOptionalIdentityFlag(found) => {
                write!(formatter, "invalid optional identity flag {found}")
            }
            Self::SnapshotIdentityMismatch { expected, actual } => write!(
                formatter,
                "safe snapshot identity mismatch: expected {expected}, found {actual}"
            ),
            Self::LengthOverflow => formatter.write_str("workspace recovery length overflow"),
            Self::Truncated => formatter.write_str("workspace recovery payload is truncated"),
            Self::TrailingBytes(count) => write!(
                formatter,
                "workspace recovery payload has {count} trailing bytes"
            ),
        }
    }
}

impl std::error::Error for WorkspaceRecoveryError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::State(source) => Some(source),
            Self::Snapshot(source) => Some(source),
            _ => None,
        }
    }
}
