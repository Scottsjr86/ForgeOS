//! Immutable Nyx tool-permission requests, decisions, and resume checkpoints.
//!
//! ForgeOS may prepare and verify the exact authority packet crossing the Nyx
//! seam, but this module does not execute tools. Approval is bound to the full
//! canonical request identity, expires explicitly, and yields a one-time resume
//! token that cannot authorize altered payload bytes or broader scope.

use forge_core::state::StateRecord;
use forge_protocol::hashes::{CanonicalHashInput, ContentHash, HashDomain};
use forge_protocol::identities::{CommandId, RepositoryId, TaskId, IDENTITY_BYTES};
use forge_protocol::paths::RepositoryRelativePath;
use std::fmt;
use std::path::{Path, PathBuf};

mod error;

pub use self::error::NyxPermissionError;

#[cfg(unix)]
use std::ffi::OsString;
#[cfg(unix)]
use std::os::unix::ffi::{OsStrExt, OsStringExt};

const PERMISSION_RECORD_TYPE: u16 = 0x4e59;
const PERMISSION_MAGIC: [u8; 8] = *b"NYXPERM\0";
const PERMISSION_SCHEMA_VERSION: u16 = 1;
const MAX_TOOL_NAME_BYTES: usize = 64;
const MAX_SCOPE_PATHS: usize = 256;
const MAX_SCOPE_PATH_BYTES: usize = 16 * 1024;
const MAX_TOOL_PAYLOAD_BYTES: usize = 4 * 1024 * 1024;

/// Canonical lower-case identifier for one Nyx-routed tool operation.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NyxToolName(String);

impl NyxToolName {
    pub fn new(value: impl Into<String>) -> Result<Self, NyxPermissionError> {
        let value = value.into();
        if value.is_empty() {
            return Err(NyxPermissionError::EmptyToolName);
        }
        if value.len() > MAX_TOOL_NAME_BYTES {
            return Err(NyxPermissionError::ToolNameTooLong {
                maximum: MAX_TOOL_NAME_BYTES,
                actual: value.len(),
            });
        }
        for (index, byte) in value.bytes().enumerate() {
            if !(byte.is_ascii_lowercase()
                || byte.is_ascii_digit()
                || matches!(byte, b'.' | b'_' | b'-'))
            {
                return Err(NyxPermissionError::InvalidToolNameByte { index, byte });
            }
        }
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Exact repository, path, and optional registered-command authority requested.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NyxAuthorityScope {
    tool: NyxToolName,
    repository_id: RepositoryId,
    paths: Vec<RepositoryRelativePath>,
    command_id: Option<CommandId>,
}

impl NyxAuthorityScope {
    pub fn new(
        tool: NyxToolName,
        repository_id: RepositoryId,
        paths: impl IntoIterator<Item = RepositoryRelativePath>,
        command_id: Option<CommandId>,
    ) -> Result<Self, NyxPermissionError> {
        let mut keyed = paths
            .into_iter()
            .map(|path| path_sort_key(&path).map(|key| (key, path)))
            .collect::<Result<Vec<_>, _>>()?;
        if keyed.len() > MAX_SCOPE_PATHS {
            return Err(NyxPermissionError::TooManyScopePaths {
                maximum: MAX_SCOPE_PATHS,
                actual: keyed.len(),
            });
        }
        keyed.sort_by(|left, right| left.0.cmp(&right.0));
        for pair in keyed.windows(2) {
            if pair[0].0 == pair[1].0 {
                return Err(NyxPermissionError::DuplicateScopePath(
                    pair[0].1.as_path().to_path_buf(),
                ));
            }
        }
        Ok(Self {
            tool,
            repository_id,
            paths: keyed.into_iter().map(|(_, path)| path).collect(),
            command_id,
        })
    }

    pub fn tool(&self) -> &NyxToolName {
        &self.tool
    }

    pub const fn repository_id(&self) -> RepositoryId {
        self.repository_id
    }

    pub fn paths(&self) -> &[RepositoryRelativePath] {
        &self.paths
    }

    pub const fn command_id(&self) -> Option<CommandId> {
        self.command_id
    }

    fn canonical_bytes(&self) -> Result<Vec<u8>, NyxPermissionError> {
        let mut bytes = Vec::new();
        push_text_u16(&mut bytes, self.tool.as_str())?;
        bytes.extend_from_slice(self.repository_id.as_bytes());
        bytes.extend_from_slice(&(self.paths.len() as u16).to_be_bytes());
        for path in &self.paths {
            let path = path_sort_key(path)?;
            push_bytes_u32(&mut bytes, &path)?;
        }
        match self.command_id {
            Some(command_id) => {
                bytes.push(1);
                bytes.extend_from_slice(command_id.as_bytes());
            }
            None => bytes.push(0),
        }
        Ok(bytes)
    }
}

/// Immutable Nyx tool request whose identity covers payload, authority, and expiry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NyxToolRequest {
    task_id: TaskId,
    scope: NyxAuthorityScope,
    payload: Vec<u8>,
    expires_at_unix_seconds: u64,
    identity: ContentHash,
}

impl NyxToolRequest {
    pub fn new(
        task_id: TaskId,
        scope: NyxAuthorityScope,
        payload: Vec<u8>,
        expires_at_unix_seconds: u64,
    ) -> Result<Self, NyxPermissionError> {
        if payload.len() > MAX_TOOL_PAYLOAD_BYTES {
            return Err(NyxPermissionError::ToolPayloadTooLarge {
                maximum: MAX_TOOL_PAYLOAD_BYTES,
                actual: payload.len(),
            });
        }
        if expires_at_unix_seconds == 0 {
            return Err(NyxPermissionError::InvalidExpiration);
        }
        let identity = request_identity(task_id, &scope, &payload, expires_at_unix_seconds)?;
        Ok(Self {
            task_id,
            scope,
            payload,
            expires_at_unix_seconds,
            identity,
        })
    }

    pub const fn task_id(&self) -> TaskId {
        self.task_id
    }

    pub fn scope(&self) -> &NyxAuthorityScope {
        &self.scope
    }

    pub fn payload(&self) -> &[u8] {
        &self.payload
    }

    pub const fn expires_at_unix_seconds(&self) -> u64 {
        self.expires_at_unix_seconds
    }

    pub const fn identity(&self) -> ContentHash {
        self.identity
    }

    pub const fn is_expired_at(&self, unix_seconds: u64) -> bool {
        unix_seconds > self.expires_at_unix_seconds
    }
}

/// Operator decision applied to one exact request identity.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NyxPermissionDecisionKind {
    Approve,
    Deny,
}

impl NyxPermissionDecisionKind {
    const fn code(self) -> u8 {
        match self {
            Self::Approve => 1,
            Self::Deny => 2,
        }
    }

    fn from_code(code: u8) -> Result<Self, NyxPermissionError> {
        match code {
            1 => Ok(Self::Approve),
            2 => Ok(Self::Deny),
            _ => Err(NyxPermissionError::MalformedCheckpoint(
                "unknown permission decision code",
            )),
        }
    }
}

/// Deterministic one-time token derived from the approved request and decision.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NyxResumeToken(ContentHash);

impl NyxResumeToken {
    pub const fn identity(self) -> ContentHash {
        self.0
    }
}

impl fmt::Display for NyxResumeToken {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.0, formatter)
    }
}

/// Immutable decision record. Only approvals carry a resume token.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NyxPermissionDecision {
    request_identity: ContentHash,
    kind: NyxPermissionDecisionKind,
    decided_at_unix_seconds: u64,
    identity: ContentHash,
    resume_token: Option<NyxResumeToken>,
}

impl NyxPermissionDecision {
    fn build(
        request: &NyxToolRequest,
        kind: NyxPermissionDecisionKind,
        decided_at_unix_seconds: u64,
    ) -> Result<Self, NyxPermissionError> {
        if kind == NyxPermissionDecisionKind::Approve
            && request.is_expired_at(decided_at_unix_seconds)
        {
            return Err(NyxPermissionError::RequestExpired {
                expired_at: request.expires_at_unix_seconds(),
                observed_at: decided_at_unix_seconds,
            });
        }
        let identity = decision_identity(request, kind, decided_at_unix_seconds)?;
        let resume_token = if kind == NyxPermissionDecisionKind::Approve {
            Some(resume_token(request.identity(), identity)?)
        } else {
            None
        };
        Ok(Self {
            request_identity: request.identity(),
            kind,
            decided_at_unix_seconds,
            identity,
            resume_token,
        })
    }

    pub const fn request_identity(&self) -> ContentHash {
        self.request_identity
    }

    pub const fn kind(&self) -> NyxPermissionDecisionKind {
        self.kind
    }

    pub const fn decided_at_unix_seconds(&self) -> u64 {
        self.decided_at_unix_seconds
    }

    pub const fn identity(&self) -> ContentHash {
        self.identity
    }

    pub const fn resume_token(&self) -> Option<NyxResumeToken> {
        self.resume_token
    }
}

/// Public checkpoint state used by ForgeOS witness surfaces and persistence.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NyxPermissionCheckpointStatus {
    Pending,
    Approved,
    Denied,
    Consumed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum CheckpointState {
    Pending,
    Approved {
        decision: NyxPermissionDecision,
        consumed: bool,
    },
    Denied {
        decision: NyxPermissionDecision,
    },
}

/// Durable exact-request checkpoint. It never executes the requested tool itself.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NyxPermissionCheckpoint {
    request: NyxToolRequest,
    state: CheckpointState,
}

impl NyxPermissionCheckpoint {
    pub fn pending(request: NyxToolRequest) -> Self {
        Self {
            request,
            state: CheckpointState::Pending,
        }
    }

    pub fn request(&self) -> &NyxToolRequest {
        &self.request
    }

    pub fn status(&self) -> NyxPermissionCheckpointStatus {
        match &self.state {
            CheckpointState::Pending => NyxPermissionCheckpointStatus::Pending,
            CheckpointState::Approved {
                consumed: false, ..
            } => NyxPermissionCheckpointStatus::Approved,
            CheckpointState::Approved { consumed: true, .. } => {
                NyxPermissionCheckpointStatus::Consumed
            }
            CheckpointState::Denied { .. } => NyxPermissionCheckpointStatus::Denied,
        }
    }

    pub fn decision(&self) -> Option<&NyxPermissionDecision> {
        match &self.state {
            CheckpointState::Pending => None,
            CheckpointState::Approved { decision, .. } | CheckpointState::Denied { decision } => {
                Some(decision)
            }
        }
    }

    pub fn decide(
        &mut self,
        presented_request_identity: ContentHash,
        kind: NyxPermissionDecisionKind,
        decided_at_unix_seconds: u64,
    ) -> Result<Option<NyxResumeToken>, NyxPermissionError> {
        if presented_request_identity != self.request.identity() {
            return Err(NyxPermissionError::RequestIdentityMismatch {
                expected: self.request.identity(),
                actual: presented_request_identity,
            });
        }
        if !matches!(&self.state, CheckpointState::Pending) {
            return Err(NyxPermissionError::DecisionAlreadyRecorded);
        }
        let decision = NyxPermissionDecision::build(&self.request, kind, decided_at_unix_seconds)?;
        let token = decision.resume_token();
        self.state = match kind {
            NyxPermissionDecisionKind::Approve => CheckpointState::Approved {
                decision,
                consumed: false,
            },
            NyxPermissionDecisionKind::Deny => CheckpointState::Denied { decision },
        };
        Ok(token)
    }

    pub fn authorize(
        &mut self,
        presented_request: &NyxToolRequest,
        presented_token: NyxResumeToken,
        observed_at_unix_seconds: u64,
    ) -> Result<NyxAuthorizedToolRequest, NyxPermissionError> {
        if presented_request != &self.request {
            return Err(NyxPermissionError::RequestIdentityMismatch {
                expected: self.request.identity(),
                actual: presented_request.identity(),
            });
        }
        if self.request.is_expired_at(observed_at_unix_seconds) {
            return Err(NyxPermissionError::RequestExpired {
                expired_at: self.request.expires_at_unix_seconds(),
                observed_at: observed_at_unix_seconds,
            });
        }
        match &mut self.state {
            CheckpointState::Pending => Err(NyxPermissionError::RequestPending),
            CheckpointState::Denied { .. } => Err(NyxPermissionError::RequestDenied),
            CheckpointState::Approved { decision, consumed } => {
                if *consumed {
                    return Err(NyxPermissionError::ResumeTokenConsumed);
                }
                let expected = decision
                    .resume_token()
                    .expect("approved decisions always carry a resume token");
                if expected != presented_token {
                    return Err(NyxPermissionError::ResumeTokenMismatch {
                        expected,
                        actual: presented_token,
                    });
                }
                *consumed = true;
                Ok(NyxAuthorizedToolRequest {
                    request: self.request.clone(),
                    decision_identity: decision.identity(),
                    resume_token: expected,
                })
            }
        }
    }

    /// Encodes this checkpoint inside the canonical Forge Core state envelope.
    pub fn to_state_record(&self) -> Result<StateRecord, NyxPermissionError> {
        StateRecord::new(PERMISSION_RECORD_TYPE, self.encode_payload()?)
            .map_err(NyxPermissionError::StateRecord)
    }

    /// Restores a checkpoint and re-verifies every stored identity and token.
    pub fn from_state_record(record: &StateRecord) -> Result<Self, NyxPermissionError> {
        if record.record_type() != PERMISSION_RECORD_TYPE {
            return Err(NyxPermissionError::WrongStateRecordType {
                expected: PERMISSION_RECORD_TYPE,
                actual: record.record_type(),
            });
        }
        Self::decode_payload(record.payload())
    }

    fn encode_payload(&self) -> Result<Vec<u8>, NyxPermissionError> {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&PERMISSION_MAGIC);
        bytes.extend_from_slice(&PERMISSION_SCHEMA_VERSION.to_be_bytes());
        encode_request(&mut bytes, &self.request)?;
        match &self.state {
            CheckpointState::Pending => bytes.push(0),
            CheckpointState::Approved {
                decision,
                consumed: false,
            } => {
                bytes.push(1);
                encode_decision(&mut bytes, decision);
            }
            CheckpointState::Approved {
                decision,
                consumed: true,
            } => {
                bytes.push(2);
                encode_decision(&mut bytes, decision);
            }
            CheckpointState::Denied { decision } => {
                bytes.push(3);
                encode_decision(&mut bytes, decision);
            }
        }
        Ok(bytes)
    }

    fn decode_payload(bytes: &[u8]) -> Result<Self, NyxPermissionError> {
        let mut decoder = PermissionDecoder::new(bytes);
        if decoder.array::<8>()? != PERMISSION_MAGIC {
            return Err(NyxPermissionError::MalformedCheckpoint(
                "permission checkpoint magic mismatch",
            ));
        }
        let schema = decoder.u16()?;
        if schema != PERMISSION_SCHEMA_VERSION {
            return Err(NyxPermissionError::UnsupportedCheckpointSchema { found: schema });
        }
        let request = decode_request(&mut decoder)?;
        let state_code = decoder.u8()?;
        let state = match state_code {
            0 => CheckpointState::Pending,
            1 | 2 | 3 => {
                let stored = decode_stored_decision(&mut decoder)?;
                let rebuilt = NyxPermissionDecision::build(
                    &request,
                    stored.kind,
                    stored.decided_at_unix_seconds,
                )?;
                if rebuilt.request_identity() != stored.request_identity
                    || rebuilt.identity() != stored.identity
                    || rebuilt.resume_token() != stored.resume_token
                {
                    return Err(NyxPermissionError::StoredDecisionIdentityMismatch);
                }
                match state_code {
                    1 => {
                        if rebuilt.kind() != NyxPermissionDecisionKind::Approve {
                            return Err(NyxPermissionError::MalformedCheckpoint(
                                "approved checkpoint stores a denial",
                            ));
                        }
                        CheckpointState::Approved {
                            decision: rebuilt,
                            consumed: false,
                        }
                    }
                    2 => {
                        if rebuilt.kind() != NyxPermissionDecisionKind::Approve {
                            return Err(NyxPermissionError::MalformedCheckpoint(
                                "consumed checkpoint stores a denial",
                            ));
                        }
                        CheckpointState::Approved {
                            decision: rebuilt,
                            consumed: true,
                        }
                    }
                    3 => {
                        if rebuilt.kind() != NyxPermissionDecisionKind::Deny {
                            return Err(NyxPermissionError::MalformedCheckpoint(
                                "denied checkpoint stores an approval",
                            ));
                        }
                        CheckpointState::Denied { decision: rebuilt }
                    }
                    _ => unreachable!(),
                }
            }
            _ => {
                return Err(NyxPermissionError::MalformedCheckpoint(
                    "unknown checkpoint state code",
                ))
            }
        };
        decoder.finish()?;
        Ok(Self { request, state })
    }
}

/// Exact request released by one successful, consuming authorization check.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NyxAuthorizedToolRequest {
    request: NyxToolRequest,
    decision_identity: ContentHash,
    resume_token: NyxResumeToken,
}

impl NyxAuthorizedToolRequest {
    pub fn request(&self) -> &NyxToolRequest {
        &self.request
    }

    pub const fn decision_identity(&self) -> ContentHash {
        self.decision_identity
    }

    pub const fn resume_token(&self) -> NyxResumeToken {
        self.resume_token
    }
}

fn request_identity(
    task_id: TaskId,
    scope: &NyxAuthorityScope,
    payload: &[u8],
    expires_at_unix_seconds: u64,
) -> Result<ContentHash, NyxPermissionError> {
    let mut input = CanonicalHashInput::new(HashDomain::ToolRequest);
    input.add_field("task_id", task_id.as_bytes().to_vec())?;
    input.add_field("authority_scope", scope.canonical_bytes()?)?;
    input.add_field("payload", payload.to_vec())?;
    input.add_field(
        "expires_at_unix_seconds",
        expires_at_unix_seconds.to_be_bytes().to_vec(),
    )?;
    Ok(input.identity())
}

fn decision_identity(
    request: &NyxToolRequest,
    kind: NyxPermissionDecisionKind,
    decided_at_unix_seconds: u64,
) -> Result<ContentHash, NyxPermissionError> {
    let mut input = CanonicalHashInput::new(HashDomain::PermissionDecision);
    input.add_field("request_identity", request.identity().as_bytes().to_vec())?;
    input.add_field("decision", vec![kind.code()])?;
    input.add_field(
        "decided_at_unix_seconds",
        decided_at_unix_seconds.to_be_bytes().to_vec(),
    )?;
    input.add_field(
        "expires_at_unix_seconds",
        request.expires_at_unix_seconds().to_be_bytes().to_vec(),
    )?;
    Ok(input.identity())
}

fn resume_token(
    request_identity: ContentHash,
    decision_identity: ContentHash,
) -> Result<NyxResumeToken, NyxPermissionError> {
    let mut input = CanonicalHashInput::new(HashDomain::ResumeToken);
    input.add_field("request_identity", request_identity.as_bytes().to_vec())?;
    input.add_field("decision_identity", decision_identity.as_bytes().to_vec())?;
    Ok(NyxResumeToken(input.identity()))
}

fn encode_request(bytes: &mut Vec<u8>, request: &NyxToolRequest) -> Result<(), NyxPermissionError> {
    bytes.extend_from_slice(request.task_id().as_bytes());
    let scope = request.scope();
    push_text_u16(bytes, scope.tool().as_str())?;
    bytes.extend_from_slice(scope.repository_id().as_bytes());
    bytes.extend_from_slice(&(scope.paths().len() as u16).to_be_bytes());
    for path in scope.paths() {
        push_bytes_u32(bytes, &path_sort_key(path)?)?;
    }
    match scope.command_id() {
        Some(command_id) => {
            bytes.push(1);
            bytes.extend_from_slice(command_id.as_bytes());
        }
        None => bytes.push(0),
    }
    push_bytes_u32(bytes, request.payload())?;
    bytes.extend_from_slice(&request.expires_at_unix_seconds().to_be_bytes());
    bytes.extend_from_slice(request.identity().as_bytes());
    Ok(())
}

fn decode_request(
    decoder: &mut PermissionDecoder<'_>,
) -> Result<NyxToolRequest, NyxPermissionError> {
    let task_id = TaskId::from_bytes(decoder.array::<IDENTITY_BYTES>()?);
    let tool = NyxToolName::new(decoder.text_u16(MAX_TOOL_NAME_BYTES)?)?;
    let repository_id = RepositoryId::from_bytes(decoder.array::<IDENTITY_BYTES>()?);
    let count = usize::from(decoder.u16()?);
    if count > MAX_SCOPE_PATHS {
        return Err(NyxPermissionError::TooManyScopePaths {
            maximum: MAX_SCOPE_PATHS,
            actual: count,
        });
    }
    let mut paths = Vec::with_capacity(count);
    for _ in 0..count {
        let raw = decoder.bytes_u32(MAX_SCOPE_PATH_BYTES)?;
        paths.push(path_from_canonical_bytes(raw)?);
    }
    let command_id = match decoder.u8()? {
        0 => None,
        1 => Some(CommandId::from_bytes(decoder.array::<IDENTITY_BYTES>()?)),
        _ => {
            return Err(NyxPermissionError::MalformedCheckpoint(
                "invalid command identity presence marker",
            ))
        }
    };
    let payload = decoder.bytes_u32(MAX_TOOL_PAYLOAD_BYTES)?.to_vec();
    let expires_at_unix_seconds = decoder.u64()?;
    let stored_identity = ContentHash::from_bytes(decoder.array::<32>()?);
    let scope = NyxAuthorityScope::new(tool, repository_id, paths, command_id)?;
    let request = NyxToolRequest::new(task_id, scope, payload, expires_at_unix_seconds)?;
    if request.identity() != stored_identity {
        return Err(NyxPermissionError::StoredRequestIdentityMismatch {
            expected: request.identity(),
            actual: stored_identity,
        });
    }
    Ok(request)
}

fn encode_decision(bytes: &mut Vec<u8>, decision: &NyxPermissionDecision) {
    bytes.extend_from_slice(decision.request_identity().as_bytes());
    bytes.push(decision.kind().code());
    bytes.extend_from_slice(&decision.decided_at_unix_seconds().to_be_bytes());
    bytes.extend_from_slice(decision.identity().as_bytes());
    match decision.resume_token() {
        Some(token) => {
            bytes.push(1);
            bytes.extend_from_slice(token.identity().as_bytes());
        }
        None => bytes.push(0),
    }
}

struct StoredDecision {
    request_identity: ContentHash,
    kind: NyxPermissionDecisionKind,
    decided_at_unix_seconds: u64,
    identity: ContentHash,
    resume_token: Option<NyxResumeToken>,
}

fn decode_stored_decision(
    decoder: &mut PermissionDecoder<'_>,
) -> Result<StoredDecision, NyxPermissionError> {
    let request_identity = ContentHash::from_bytes(decoder.array::<32>()?);
    let kind = NyxPermissionDecisionKind::from_code(decoder.u8()?)?;
    let decided_at_unix_seconds = decoder.u64()?;
    let identity = ContentHash::from_bytes(decoder.array::<32>()?);
    let resume_token = match decoder.u8()? {
        0 => None,
        1 => Some(NyxResumeToken(ContentHash::from_bytes(
            decoder.array::<32>()?,
        ))),
        _ => {
            return Err(NyxPermissionError::MalformedCheckpoint(
                "invalid resume token presence marker",
            ))
        }
    };
    Ok(StoredDecision {
        request_identity,
        kind,
        decided_at_unix_seconds,
        identity,
        resume_token,
    })
}

fn push_text_u16(bytes: &mut Vec<u8>, text: &str) -> Result<(), NyxPermissionError> {
    let length = u16::try_from(text.len()).map_err(|_| {
        NyxPermissionError::MalformedCheckpoint("text length does not fit the permission codec")
    })?;
    bytes.extend_from_slice(&length.to_be_bytes());
    bytes.extend_from_slice(text.as_bytes());
    Ok(())
}

fn push_bytes_u32(bytes: &mut Vec<u8>, value: &[u8]) -> Result<(), NyxPermissionError> {
    let length = u32::try_from(value.len()).map_err(|_| {
        NyxPermissionError::MalformedCheckpoint("byte length does not fit the permission codec")
    })?;
    bytes.extend_from_slice(&length.to_be_bytes());
    bytes.extend_from_slice(value);
    Ok(())
}

fn path_sort_key(path: &RepositoryRelativePath) -> Result<Vec<u8>, NyxPermissionError> {
    canonical_path_bytes(path.as_path())
}

#[cfg(unix)]
fn canonical_path_bytes(path: &Path) -> Result<Vec<u8>, NyxPermissionError> {
    let bytes = path.as_os_str().as_bytes();
    if bytes.len() > MAX_SCOPE_PATH_BYTES {
        return Err(NyxPermissionError::ScopePathTooLong {
            maximum: MAX_SCOPE_PATH_BYTES,
            actual: bytes.len(),
        });
    }
    Ok(bytes.to_vec())
}

#[cfg(not(unix))]
fn canonical_path_bytes(path: &Path) -> Result<Vec<u8>, NyxPermissionError> {
    let text = path
        .to_str()
        .ok_or(NyxPermissionError::NonCanonicalPlatformPath)?;
    if text.len() > MAX_SCOPE_PATH_BYTES {
        return Err(NyxPermissionError::ScopePathTooLong {
            maximum: MAX_SCOPE_PATH_BYTES,
            actual: text.len(),
        });
    }
    Ok(text.as_bytes().to_vec())
}

#[cfg(unix)]
fn path_from_canonical_bytes(bytes: &[u8]) -> Result<RepositoryRelativePath, NyxPermissionError> {
    RepositoryRelativePath::new(PathBuf::from(OsString::from_vec(bytes.to_vec())))
        .map_err(NyxPermissionError::RepositoryPath)
}

#[cfg(not(unix))]
fn path_from_canonical_bytes(bytes: &[u8]) -> Result<RepositoryRelativePath, NyxPermissionError> {
    let text =
        std::str::from_utf8(bytes).map_err(|_| NyxPermissionError::NonCanonicalPlatformPath)?;
    RepositoryRelativePath::new(text).map_err(NyxPermissionError::RepositoryPath)
}

struct PermissionDecoder<'a> {
    bytes: &'a [u8],
    offset: usize,
}

impl<'a> PermissionDecoder<'a> {
    const fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, offset: 0 }
    }

    fn u8(&mut self) -> Result<u8, NyxPermissionError> {
        Ok(self.take(1)?[0])
    }

    fn u16(&mut self) -> Result<u16, NyxPermissionError> {
        Ok(u16::from_be_bytes(self.array()?))
    }

    fn u64(&mut self) -> Result<u64, NyxPermissionError> {
        Ok(u64::from_be_bytes(self.array()?))
    }

    fn array<const N: usize>(&mut self) -> Result<[u8; N], NyxPermissionError> {
        self.take(N)?
            .try_into()
            .map_err(|_| NyxPermissionError::MalformedCheckpoint("truncated fixed-width field"))
    }

    fn text_u16(&mut self, maximum: usize) -> Result<String, NyxPermissionError> {
        let length = usize::from(self.u16()?);
        if length > maximum {
            return Err(NyxPermissionError::MalformedCheckpoint(
                "text field exceeds its declared maximum",
            ));
        }
        let bytes = self.take(length)?;
        let text = std::str::from_utf8(bytes)
            .map_err(|_| NyxPermissionError::MalformedCheckpoint("text field is not UTF-8"))?;
        Ok(text.to_owned())
    }

    fn bytes_u32(&mut self, maximum: usize) -> Result<&'a [u8], NyxPermissionError> {
        let length = usize::try_from(u32::from_be_bytes(self.array()?))
            .map_err(|_| NyxPermissionError::MalformedCheckpoint("length does not fit usize"))?;
        if length > maximum {
            return Err(NyxPermissionError::MalformedCheckpoint(
                "byte field exceeds its declared maximum",
            ));
        }
        self.take(length)
    }

    fn take(&mut self, length: usize) -> Result<&'a [u8], NyxPermissionError> {
        let end =
            self.offset
                .checked_add(length)
                .ok_or(NyxPermissionError::MalformedCheckpoint(
                    "checkpoint length overflow",
                ))?;
        if end > self.bytes.len() {
            return Err(NyxPermissionError::MalformedCheckpoint(
                "permission checkpoint is truncated",
            ));
        }
        let value = &self.bytes[self.offset..end];
        self.offset = end;
        Ok(value)
    }

    fn finish(self) -> Result<(), NyxPermissionError> {
        if self.offset == self.bytes.len() {
            Ok(())
        } else {
            Err(NyxPermissionError::MalformedCheckpoint(
                "permission checkpoint has trailing bytes",
            ))
        }
    }
}
