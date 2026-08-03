use super::NyxResumeToken;
use forge_core::state::StateRecordError;
use forge_protocol::hashes::{ContentHash, HashContractError};
use forge_protocol::paths::RepositoryPathError;
use std::fmt;
use std::path::PathBuf;

/// Exact reason a permission request, decision, token, or checkpoint was rejected.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NyxPermissionError {
    EmptyToolName,
    ToolNameTooLong {
        maximum: usize,
        actual: usize,
    },
    InvalidToolNameByte {
        index: usize,
        byte: u8,
    },
    TooManyScopePaths {
        maximum: usize,
        actual: usize,
    },
    ScopePathTooLong {
        maximum: usize,
        actual: usize,
    },
    DuplicateScopePath(PathBuf),
    NonCanonicalPlatformPath,
    ToolPayloadTooLarge {
        maximum: usize,
        actual: usize,
    },
    InvalidExpiration,
    RequestIdentityMismatch {
        expected: ContentHash,
        actual: ContentHash,
    },
    RequestExpired {
        expired_at: u64,
        observed_at: u64,
    },
    DecisionAlreadyRecorded,
    RequestPending,
    RequestDenied,
    ResumeTokenMismatch {
        expected: NyxResumeToken,
        actual: NyxResumeToken,
    },
    ResumeTokenConsumed,
    WrongStateRecordType {
        expected: u16,
        actual: u16,
    },
    UnsupportedCheckpointSchema {
        found: u16,
    },
    StoredRequestIdentityMismatch {
        expected: ContentHash,
        actual: ContentHash,
    },
    StoredDecisionIdentityMismatch,
    MalformedCheckpoint(&'static str),
    HashContract(HashContractError),
    RepositoryPath(RepositoryPathError),
    StateRecord(StateRecordError),
}

impl fmt::Display for NyxPermissionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyToolName => formatter.write_str("Nyx tool name is empty"),
            Self::ToolNameTooLong { maximum, actual } => {
                write!(formatter, "Nyx tool name exceeds {maximum} bytes: {actual}")
            }
            Self::InvalidToolNameByte { index, byte } => write!(
                formatter,
                "Nyx tool name byte {index} is not canonical: 0x{byte:02x}"
            ),
            Self::TooManyScopePaths { maximum, actual } => write!(
                formatter,
                "Nyx authority scope exceeds {maximum} paths: {actual}"
            ),
            Self::ScopePathTooLong { maximum, actual } => write!(
                formatter,
                "Nyx authority path exceeds {maximum} bytes: {actual}"
            ),
            Self::DuplicateScopePath(path) => write!(
                formatter,
                "Nyx authority scope duplicates {}",
                path.display()
            ),
            Self::NonCanonicalPlatformPath => {
                formatter.write_str("Nyx authority path has no canonical platform encoding")
            }
            Self::ToolPayloadTooLarge { maximum, actual } => write!(
                formatter,
                "Nyx tool payload exceeds {maximum} bytes: {actual}"
            ),
            Self::InvalidExpiration => formatter.write_str("Nyx tool request expiration is zero"),
            Self::RequestIdentityMismatch { expected, actual } => write!(
                formatter,
                "Nyx tool request identity mismatch: expected {expected}, got {actual}"
            ),
            Self::RequestExpired {
                expired_at,
                observed_at,
            } => write!(
                formatter,
                "Nyx tool request expired at {expired_at}; observed at {observed_at}"
            ),
            Self::DecisionAlreadyRecorded => {
                formatter.write_str("Nyx permission decision is already recorded")
            }
            Self::RequestPending => formatter.write_str("Nyx tool request is still pending"),
            Self::RequestDenied => formatter.write_str("Nyx tool request was denied"),
            Self::ResumeTokenMismatch { expected, actual } => write!(
                formatter,
                "Nyx resume token mismatch: expected {expected}, got {actual}"
            ),
            Self::ResumeTokenConsumed => {
                formatter.write_str("Nyx resume token was already consumed")
            }
            Self::WrongStateRecordType { expected, actual } => write!(
                formatter,
                "Nyx permission state record type mismatch: expected {expected}, got {actual}"
            ),
            Self::UnsupportedCheckpointSchema { found } => write!(
                formatter,
                "unsupported Nyx permission checkpoint schema {found}"
            ),
            Self::StoredRequestIdentityMismatch { expected, actual } => write!(
                formatter,
                "stored Nyx request identity mismatch: expected {expected}, got {actual}"
            ),
            Self::StoredDecisionIdentityMismatch => {
                formatter.write_str("stored Nyx decision or resume token identity mismatch")
            }
            Self::MalformedCheckpoint(message) => {
                write!(formatter, "malformed Nyx permission checkpoint: {message}")
            }
            Self::HashContract(error) => fmt::Display::fmt(error, formatter),
            Self::RepositoryPath(error) => fmt::Display::fmt(error, formatter),
            Self::StateRecord(error) => fmt::Display::fmt(error, formatter),
        }
    }
}

impl std::error::Error for NyxPermissionError {}

impl From<HashContractError> for NyxPermissionError {
    fn from(error: HashContractError) -> Self {
        Self::HashContract(error)
    }
}
