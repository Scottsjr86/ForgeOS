//! Canonical version-bound validation records and append-only history.
//!
//! Forge Core owns the stable meaning and identity of validation evidence. Native
//! Git and process execution remain authoritative for the source and command facts
//! consumed by these records.

use crate::state::{StateRecord, StateRecordError};
use forge_protocol::hashes::{hash_canonical_bytes, CanonicalHashInput, ContentHash, HashDomain};
use forge_protocol::identities::{CommandId, ProcessId, ProjectId, RepositoryId};
use forge_protocol::processes::ProcessFailureStage;
use std::collections::{btree_map::Entry, BTreeMap};
use std::fmt;

const RECORD_MAGIC: &[u8; 8] = b"FGVERIFY";
const LEDGER_MAGIC: &[u8; 8] = b"FGVERLOG";
const OUTPUT_MAGIC: &[u8; 8] = b"FGVROUT\0";
const VERIFICATION_SCHEMA_VERSION: u8 = 1;
const MAX_REVISION_BYTES: usize = 128;
const MAX_PROGRAM_BYTES: usize = 4 * 1024;
const MAX_ARGUMENT_BYTES: usize = 1024 * 1024;
const MAX_ARGUMENTS: usize = u16::MAX as usize;

/// Forge Core record type used by the append-only verification ledger.
pub const VERIFICATION_LEDGER_STATE_RECORD_TYPE: u16 = 0x5601;

/// Exact project source state before or after one validation command.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerificationSourceState {
    project_id: ProjectId,
    repository_id: RepositoryId,
    revision: Vec<u8>,
    revision_identity: ContentHash,
    dirty_state_identity: ContentHash,
}

impl VerificationSourceState {
    pub fn new(
        project_id: ProjectId,
        repository_id: RepositoryId,
        revision: impl Into<Vec<u8>>,
        dirty_state_identity: ContentHash,
    ) -> Result<Self, VerificationRecordError> {
        let revision = revision.into();
        validate_revision(&revision)?;
        let revision_identity = source_revision_identity(&revision);
        Ok(Self {
            project_id,
            repository_id,
            revision,
            revision_identity,
            dirty_state_identity,
        })
    }

    pub const fn project_id(&self) -> ProjectId {
        self.project_id
    }

    pub const fn repository_id(&self) -> RepositoryId {
        self.repository_id
    }

    /// Exact native revision bytes, normally the Git object ID text.
    pub fn revision(&self) -> &[u8] {
        &self.revision
    }

    /// Stable SHA-256 identity used by command execution to bind the revision.
    pub const fn revision_identity(&self) -> ContentHash {
        self.revision_identity
    }

    /// Stable identity of the complete consistency-checked dirty source view.
    pub const fn dirty_state_identity(&self) -> ContentHash {
        self.dirty_state_identity
    }
}

/// Computes the canonical identity used to bind a native revision to a command run.
pub fn source_revision_identity(revision: &[u8]) -> ContentHash {
    let mut input = CanonicalHashInput::new(HashDomain::Snapshot);
    input
        .add_field("kind", b"native_source_revision".to_vec())
        .expect("built-in revision identity field is valid");
    input
        .add_field("revision", revision.to_vec())
        .expect("built-in revision identity field is valid");
    input.identity()
}

/// Content-addressed reference to exact stdout and stderr bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VerificationOutputReference {
    identity: ContentHash,
    stdout_bytes: u64,
    stderr_bytes: u64,
}

impl VerificationOutputReference {
    pub fn from_output(stdout: &[u8], stderr: &[u8]) -> Self {
        let stdout_bytes = u64::try_from(stdout.len()).expect("slice length fits u64");
        let stderr_bytes = u64::try_from(stderr.len()).expect("slice length fits u64");
        let identity = output_identity(stdout, stderr);
        Self {
            identity,
            stdout_bytes,
            stderr_bytes,
        }
    }

    pub const fn identity(self) -> ContentHash {
        self.identity
    }

    pub const fn stdout_bytes(self) -> u64 {
        self.stdout_bytes
    }

    pub const fn stderr_bytes(self) -> u64 {
        self.stderr_bytes
    }

    pub fn matches(self, stdout: &[u8], stderr: &[u8]) -> bool {
        self.stdout_bytes == u64::try_from(stdout.len()).expect("slice length fits u64")
            && self.stderr_bytes == u64::try_from(stderr.len()).expect("slice length fits u64")
            && self.identity == output_identity(stdout, stderr)
    }
}

/// Exact terminal meaning of one validation command.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VerificationOutcome {
    Passed {
        exit_code: Option<i32>,
    },
    Failed {
        exit_code: Option<i32>,
    },
    TimedOut,
    Cancelled,
    ExecutionFailed {
        stage: ProcessFailureStage,
        message_identity: ContentHash,
    },
}

impl VerificationOutcome {
    pub const fn is_pass(&self) -> bool {
        matches!(self, Self::Passed { .. })
    }

    pub const fn exit_code(&self) -> Option<i32> {
        match self {
            Self::Passed { exit_code } | Self::Failed { exit_code } => *exit_code,
            Self::TimedOut | Self::Cancelled | Self::ExecutionFailed { .. } => None,
        }
    }

    pub const fn label(&self) -> &'static str {
        match self {
            Self::Passed { .. } => "passed",
            Self::Failed { .. } => "failed",
            Self::TimedOut => "timed_out",
            Self::Cancelled => "cancelled",
            Self::ExecutionFailed { .. } => "execution_failed",
        }
    }
}

/// One immutable validation result bound to exact source, command, and output facts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerificationRecord {
    command_id: CommandId,
    command_definition: ContentHash,
    process_id: ProcessId,
    program: String,
    arguments: Vec<String>,
    start_state: VerificationSourceState,
    end_state: VerificationSourceState,
    outcome: VerificationOutcome,
    output: VerificationOutputReference,
    identity: ContentHash,
}

impl VerificationRecord {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        command_id: CommandId,
        command_definition: ContentHash,
        process_id: ProcessId,
        program: impl Into<String>,
        arguments: impl IntoIterator<Item = String>,
        start_state: VerificationSourceState,
        end_state: VerificationSourceState,
        outcome: VerificationOutcome,
        output: VerificationOutputReference,
    ) -> Result<Self, VerificationRecordError> {
        if start_state.project_id != end_state.project_id
            || start_state.repository_id != end_state.repository_id
        {
            return Err(VerificationRecordError::SourceScopeMismatch {
                start_project: start_state.project_id,
                end_project: end_state.project_id,
                start_repository: start_state.repository_id,
                end_repository: end_state.repository_id,
            });
        }
        let program = program.into();
        validate_program(&program)?;
        let arguments = arguments.into_iter().collect::<Vec<_>>();
        validate_arguments(&arguments)?;
        let mut record = Self {
            command_id,
            command_definition,
            process_id,
            program,
            arguments,
            start_state,
            end_state,
            outcome,
            output,
            identity: ContentHash::from_bytes([0; 32]),
        };
        record.identity =
            hash_canonical_bytes(HashDomain::ResultPayload, &record.canonical_bytes());
        Ok(record)
    }

    pub const fn command_id(&self) -> CommandId {
        self.command_id
    }

    pub const fn command_definition(&self) -> ContentHash {
        self.command_definition
    }

    pub const fn process_id(&self) -> ProcessId {
        self.process_id
    }

    pub fn program(&self) -> &str {
        &self.program
    }

    pub fn arguments(&self) -> &[String] {
        &self.arguments
    }

    pub fn start_state(&self) -> &VerificationSourceState {
        &self.start_state
    }

    pub fn end_state(&self) -> &VerificationSourceState {
        &self.end_state
    }

    pub fn outcome(&self) -> &VerificationOutcome {
        &self.outcome
    }

    pub const fn output(&self) -> VerificationOutputReference {
        self.output
    }

    pub const fn identity(&self) -> ContentHash {
        self.identity
    }

    /// A passing record satisfies only the exact source state observed after the run.
    pub fn satisfies(&self, current: &VerificationSourceState) -> bool {
        self.outcome.is_pass() && &self.end_state == current
    }

    /// Exact versioned bytes whose SHA-256 identity names this record.
    pub fn canonical_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(RECORD_MAGIC);
        bytes.push(VERIFICATION_SCHEMA_VERSION);
        bytes.extend_from_slice(self.command_id.as_bytes());
        bytes.extend_from_slice(self.command_definition.as_bytes());
        bytes.extend_from_slice(self.process_id.as_bytes());
        push_text(&mut bytes, &self.program);
        bytes.extend_from_slice(&(self.arguments.len() as u16).to_be_bytes());
        for argument in &self.arguments {
            push_text(&mut bytes, argument);
        }
        encode_source_state(&mut bytes, &self.start_state);
        encode_source_state(&mut bytes, &self.end_state);
        encode_outcome(&mut bytes, &self.outcome);
        bytes.extend_from_slice(self.output.identity.as_bytes());
        bytes.extend_from_slice(&self.output.stdout_bytes.to_be_bytes());
        bytes.extend_from_slice(&self.output.stderr_bytes.to_be_bytes());
        bytes
    }

    pub fn decode(bytes: &[u8]) -> Result<Self, VerificationRecordError> {
        let mut cursor = Cursor::new(bytes);
        if cursor.take(RECORD_MAGIC.len())? != RECORD_MAGIC.as_slice() {
            return Err(VerificationRecordError::InvalidMagic);
        }
        let version = cursor.byte()?;
        if version != VERIFICATION_SCHEMA_VERSION {
            return Err(VerificationRecordError::UnsupportedSchemaVersion(version));
        }
        let command_id = CommandId::from_bytes(cursor.array::<16>()?);
        let command_definition = ContentHash::from_bytes(cursor.array::<32>()?);
        let process_id = ProcessId::from_bytes(cursor.array::<16>()?);
        let program = cursor.text()?;
        let argument_count = usize::from(cursor.u16()?);
        let mut arguments = Vec::with_capacity(argument_count);
        for _ in 0..argument_count {
            arguments.push(cursor.text()?);
        }
        let start_state = decode_source_state(&mut cursor)?;
        let end_state = decode_source_state(&mut cursor)?;
        let outcome = decode_outcome(&mut cursor)?;
        let output = VerificationOutputReference {
            identity: ContentHash::from_bytes(cursor.array::<32>()?),
            stdout_bytes: cursor.u64()?,
            stderr_bytes: cursor.u64()?,
        };
        cursor.finish()?;
        let record = Self::new(
            command_id,
            command_definition,
            process_id,
            program,
            arguments,
            start_state,
            end_state,
            outcome,
            output,
        )?;
        if record.canonical_bytes() != bytes {
            return Err(VerificationRecordError::NonCanonicalEncoding);
        }
        Ok(record)
    }
}

/// Append-only deterministic verification history.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct VerificationLedger {
    records: BTreeMap<ContentHash, VerificationRecord>,
}

impl VerificationLedger {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn record(
        &mut self,
        record: VerificationRecord,
    ) -> Result<VerificationRecordStatus, VerificationLedgerError> {
        let identity = record.identity();
        match self.records.entry(identity) {
            Entry::Vacant(entry) => {
                entry.insert(record);
                Ok(VerificationRecordStatus::Inserted)
            }
            Entry::Occupied(entry) if entry.get() == &record => {
                Ok(VerificationRecordStatus::AlreadyRecorded)
            }
            Entry::Occupied(_) => Err(VerificationLedgerError::IdentityConflict(identity)),
        }
    }

    pub fn get(&self, identity: ContentHash) -> Option<&VerificationRecord> {
        self.records.get(&identity)
    }

    pub fn iter(&self) -> impl ExactSizeIterator<Item = &VerificationRecord> {
        self.records.values()
    }

    pub fn len(&self) -> usize {
        self.records.len()
    }

    pub fn is_empty(&self) -> bool {
        self.records.is_empty()
    }

    pub fn state_record(&self) -> Result<StateRecord, VerificationLedgerError> {
        StateRecord::new(
            VERIFICATION_LEDGER_STATE_RECORD_TYPE,
            self.canonical_bytes(),
        )
        .map_err(VerificationLedgerError::State)
    }

    pub fn from_state_record(record: &StateRecord) -> Result<Self, VerificationLedgerError> {
        if record.record_type() != VERIFICATION_LEDGER_STATE_RECORD_TYPE {
            return Err(VerificationLedgerError::UnexpectedRecordType {
                expected: VERIFICATION_LEDGER_STATE_RECORD_TYPE,
                found: record.record_type(),
            });
        }
        Self::decode(record.payload())
    }

    fn canonical_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(LEDGER_MAGIC);
        bytes.push(VERIFICATION_SCHEMA_VERSION);
        bytes.extend_from_slice(&(self.records.len() as u32).to_be_bytes());
        for (identity, record) in &self.records {
            let record_bytes = record.canonical_bytes();
            bytes.extend_from_slice(identity.as_bytes());
            bytes.extend_from_slice(&(record_bytes.len() as u32).to_be_bytes());
            bytes.extend_from_slice(&record_bytes);
        }
        bytes
    }

    fn decode(bytes: &[u8]) -> Result<Self, VerificationLedgerError> {
        let mut cursor = Cursor::new(bytes);
        if cursor
            .take(LEDGER_MAGIC.len())
            .map_err(VerificationLedgerError::Record)?
            != LEDGER_MAGIC.as_slice()
        {
            return Err(VerificationLedgerError::InvalidMagic);
        }
        let version = cursor.byte().map_err(VerificationLedgerError::Record)?;
        if version != VERIFICATION_SCHEMA_VERSION {
            return Err(VerificationLedgerError::UnsupportedSchemaVersion(version));
        }
        let count = cursor.u32().map_err(VerificationLedgerError::Record)? as usize;
        let mut ledger = Self::new();
        for _ in 0..count {
            let stored_identity = ContentHash::from_bytes(
                cursor
                    .array::<32>()
                    .map_err(VerificationLedgerError::Record)?,
            );
            let length = cursor.u32().map_err(VerificationLedgerError::Record)? as usize;
            let record_bytes = cursor
                .take(length)
                .map_err(VerificationLedgerError::Record)?;
            let record = VerificationRecord::decode(record_bytes)
                .map_err(VerificationLedgerError::Record)?;
            if record.identity() != stored_identity {
                return Err(VerificationLedgerError::StoredIdentityMismatch {
                    stored: stored_identity,
                    actual: record.identity(),
                });
            }
            ledger.record(record)?;
        }
        cursor.finish().map_err(VerificationLedgerError::Record)?;
        if ledger.canonical_bytes() != bytes {
            return Err(VerificationLedgerError::NonCanonicalEncoding);
        }
        Ok(ledger)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VerificationRecordStatus {
    Inserted,
    AlreadyRecorded,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VerificationRecordError {
    EmptyRevision,
    RevisionTooLong {
        maximum: usize,
        actual: usize,
    },
    RevisionContainsNul {
        byte_index: usize,
    },
    EmptyProgram,
    ProgramTooLong {
        maximum: usize,
        actual: usize,
    },
    ProgramContainsNul {
        byte_index: usize,
    },
    TooManyArguments {
        maximum: usize,
        actual: usize,
    },
    ArgumentTooLong {
        index: usize,
        maximum: usize,
        actual: usize,
    },
    ArgumentContainsNul {
        index: usize,
        byte_index: usize,
    },
    SourceScopeMismatch {
        start_project: ProjectId,
        end_project: ProjectId,
        start_repository: RepositoryId,
        end_repository: RepositoryId,
    },
    Truncated {
        requested: usize,
        remaining: usize,
    },
    InvalidMagic,
    UnsupportedSchemaVersion(u8),
    InvalidUtf8,
    InvalidOutcomeCode(u8),
    InvalidFailureStage(u8),
    InvalidOptionalExitCode(u8),
    NonCanonicalEncoding,
    TrailingBytes(usize),
}

impl fmt::Display for VerificationRecordError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "verification record rejected: {self:?}")
    }
}

impl std::error::Error for VerificationRecordError {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VerificationLedgerError {
    State(StateRecordError),
    Record(VerificationRecordError),
    UnexpectedRecordType {
        expected: u16,
        found: u16,
    },
    InvalidMagic,
    UnsupportedSchemaVersion(u8),
    IdentityConflict(ContentHash),
    StoredIdentityMismatch {
        stored: ContentHash,
        actual: ContentHash,
    },
    NonCanonicalEncoding,
}

impl fmt::Display for VerificationLedgerError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "verification ledger rejected: {self:?}")
    }
}

impl std::error::Error for VerificationLedgerError {}

fn validate_revision(revision: &[u8]) -> Result<(), VerificationRecordError> {
    if revision.is_empty() {
        return Err(VerificationRecordError::EmptyRevision);
    }
    if revision.len() > MAX_REVISION_BYTES {
        return Err(VerificationRecordError::RevisionTooLong {
            maximum: MAX_REVISION_BYTES,
            actual: revision.len(),
        });
    }
    if let Some(byte_index) = revision.iter().position(|byte| *byte == 0) {
        return Err(VerificationRecordError::RevisionContainsNul { byte_index });
    }
    Ok(())
}

fn validate_program(program: &str) -> Result<(), VerificationRecordError> {
    if program.is_empty() {
        return Err(VerificationRecordError::EmptyProgram);
    }
    if program.len() > MAX_PROGRAM_BYTES {
        return Err(VerificationRecordError::ProgramTooLong {
            maximum: MAX_PROGRAM_BYTES,
            actual: program.len(),
        });
    }
    if let Some(byte_index) = program.as_bytes().iter().position(|byte| *byte == 0) {
        return Err(VerificationRecordError::ProgramContainsNul { byte_index });
    }
    Ok(())
}

fn validate_arguments(arguments: &[String]) -> Result<(), VerificationRecordError> {
    if arguments.len() > MAX_ARGUMENTS {
        return Err(VerificationRecordError::TooManyArguments {
            maximum: MAX_ARGUMENTS,
            actual: arguments.len(),
        });
    }
    for (index, argument) in arguments.iter().enumerate() {
        if argument.len() > MAX_ARGUMENT_BYTES {
            return Err(VerificationRecordError::ArgumentTooLong {
                index,
                maximum: MAX_ARGUMENT_BYTES,
                actual: argument.len(),
            });
        }
        if let Some(byte_index) = argument.as_bytes().iter().position(|byte| *byte == 0) {
            return Err(VerificationRecordError::ArgumentContainsNul { index, byte_index });
        }
    }
    Ok(())
}

fn output_identity(stdout: &[u8], stderr: &[u8]) -> ContentHash {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(OUTPUT_MAGIC);
    bytes.extend_from_slice(&(stdout.len() as u64).to_be_bytes());
    bytes.extend_from_slice(stdout);
    bytes.extend_from_slice(&(stderr.len() as u64).to_be_bytes());
    bytes.extend_from_slice(stderr);
    hash_canonical_bytes(HashDomain::ResultPayload, &bytes)
}

fn encode_source_state(bytes: &mut Vec<u8>, state: &VerificationSourceState) {
    bytes.extend_from_slice(state.project_id.as_bytes());
    bytes.extend_from_slice(state.repository_id.as_bytes());
    bytes.extend_from_slice(&(state.revision.len() as u16).to_be_bytes());
    bytes.extend_from_slice(&state.revision);
    bytes.extend_from_slice(state.dirty_state_identity.as_bytes());
}

fn decode_source_state(
    cursor: &mut Cursor<'_>,
) -> Result<VerificationSourceState, VerificationRecordError> {
    let project_id = ProjectId::from_bytes(cursor.array::<16>()?);
    let repository_id = RepositoryId::from_bytes(cursor.array::<16>()?);
    let revision_len = usize::from(cursor.u16()?);
    let revision = cursor.take(revision_len)?.to_vec();
    let dirty_state_identity = ContentHash::from_bytes(cursor.array::<32>()?);
    VerificationSourceState::new(project_id, repository_id, revision, dirty_state_identity)
}

fn encode_outcome(bytes: &mut Vec<u8>, outcome: &VerificationOutcome) {
    match outcome {
        VerificationOutcome::Passed { exit_code } => {
            bytes.push(1);
            encode_exit_code(bytes, *exit_code);
        }
        VerificationOutcome::Failed { exit_code } => {
            bytes.push(2);
            encode_exit_code(bytes, *exit_code);
        }
        VerificationOutcome::TimedOut => bytes.push(3),
        VerificationOutcome::Cancelled => bytes.push(4),
        VerificationOutcome::ExecutionFailed {
            stage,
            message_identity,
        } => {
            bytes.push(5);
            bytes.push(stage.code());
            bytes.extend_from_slice(message_identity.as_bytes());
        }
    }
}

fn decode_outcome(cursor: &mut Cursor<'_>) -> Result<VerificationOutcome, VerificationRecordError> {
    match cursor.byte()? {
        1 => Ok(VerificationOutcome::Passed {
            exit_code: decode_exit_code(cursor)?,
        }),
        2 => Ok(VerificationOutcome::Failed {
            exit_code: decode_exit_code(cursor)?,
        }),
        3 => Ok(VerificationOutcome::TimedOut),
        4 => Ok(VerificationOutcome::Cancelled),
        5 => {
            let stage = match cursor.byte()? {
                1 => ProcessFailureStage::Spawn,
                2 => ProcessFailureStage::Wait,
                3 => ProcessFailureStage::Termination,
                4 => ProcessFailureStage::Output,
                code => return Err(VerificationRecordError::InvalidFailureStage(code)),
            };
            let message_identity = ContentHash::from_bytes(cursor.array::<32>()?);
            Ok(VerificationOutcome::ExecutionFailed {
                stage,
                message_identity,
            })
        }
        code => Err(VerificationRecordError::InvalidOutcomeCode(code)),
    }
}

fn encode_exit_code(bytes: &mut Vec<u8>, exit_code: Option<i32>) {
    match exit_code {
        Some(code) => {
            bytes.push(1);
            bytes.extend_from_slice(&code.to_be_bytes());
        }
        None => bytes.push(0),
    }
}

fn decode_exit_code(cursor: &mut Cursor<'_>) -> Result<Option<i32>, VerificationRecordError> {
    match cursor.byte()? {
        0 => Ok(None),
        1 => Ok(Some(i32::from_be_bytes(cursor.array::<4>()?))),
        code => Err(VerificationRecordError::InvalidOptionalExitCode(code)),
    }
}

fn push_text(bytes: &mut Vec<u8>, text: &str) {
    bytes.extend_from_slice(&(text.len() as u32).to_be_bytes());
    bytes.extend_from_slice(text.as_bytes());
}

struct Cursor<'a> {
    bytes: &'a [u8],
    offset: usize,
}

impl<'a> Cursor<'a> {
    const fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, offset: 0 }
    }

    fn take(&mut self, count: usize) -> Result<&'a [u8], VerificationRecordError> {
        let remaining = self.bytes.len().saturating_sub(self.offset);
        if count > remaining {
            return Err(VerificationRecordError::Truncated {
                requested: count,
                remaining,
            });
        }
        let start = self.offset;
        self.offset += count;
        Ok(&self.bytes[start..self.offset])
    }

    fn array<const N: usize>(&mut self) -> Result<[u8; N], VerificationRecordError> {
        let mut array = [0u8; N];
        array.copy_from_slice(self.take(N)?);
        Ok(array)
    }

    fn byte(&mut self) -> Result<u8, VerificationRecordError> {
        Ok(self.take(1)?[0])
    }

    fn u16(&mut self) -> Result<u16, VerificationRecordError> {
        Ok(u16::from_be_bytes(self.array::<2>()?))
    }

    fn u32(&mut self) -> Result<u32, VerificationRecordError> {
        Ok(u32::from_be_bytes(self.array::<4>()?))
    }

    fn u64(&mut self) -> Result<u64, VerificationRecordError> {
        Ok(u64::from_be_bytes(self.array::<8>()?))
    }

    fn text(&mut self) -> Result<String, VerificationRecordError> {
        let length = self.u32()? as usize;
        String::from_utf8(self.take(length)?.to_vec())
            .map_err(|_| VerificationRecordError::InvalidUtf8)
    }

    fn finish(self) -> Result<(), VerificationRecordError> {
        let trailing = self.bytes.len().saturating_sub(self.offset);
        if trailing == 0 {
            Ok(())
        } else {
            Err(VerificationRecordError::TrailingBytes(trailing))
        }
    }
}
