use crate::canonical_json::raw_sha256_hex;
use serde::Serialize;
use serde::de::DeserializeOwned;
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NyxRemoteAgentProtocolError {
    Json(String),
    InvalidField {
        context: &'static str,
        field: &'static str,
        detail: String,
    },
    NonCanonical {
        context: &'static str,
    },
    HashMismatch {
        field: &'static str,
        expected: String,
        actual: String,
    },
    ImmutableMismatch {
        field: &'static str,
    },
}

impl fmt::Display for NyxRemoteAgentProtocolError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Json(detail) => write!(formatter, "Nyx remote-agent JSON error: {detail}"),
            Self::InvalidField {
                context,
                field,
                detail,
            } => write!(formatter, "invalid {context} field {field}: {detail}"),
            Self::NonCanonical { context } => {
                write!(formatter, "noncanonical Nyx remote-agent {context}")
            }
            Self::HashMismatch {
                field,
                expected,
                actual,
            } => write!(
                formatter,
                "Nyx remote-agent hash mismatch for {field}: expected {expected}, got {actual}"
            ),
            Self::ImmutableMismatch { field } => {
                write!(
                    formatter,
                    "Nyx remote-agent immutable field changed: {field}"
                )
            }
        }
    }
}

impl std::error::Error for NyxRemoteAgentProtocolError {}

pub(crate) fn decode_remote_agent<T: DeserializeOwned>(
    bytes: &[u8],
) -> Result<T, NyxRemoteAgentProtocolError> {
    serde_json::from_slice(bytes).map_err(|error| {
        NyxRemoteAgentProtocolError::Json(format!("decode Nyx remote-agent response: {error}"))
    })
}

pub(crate) fn ensure_schema(
    context: &'static str,
    schema_version: &str,
    schema_id: &str,
    expected_id: &'static str,
) -> Result<(), NyxRemoteAgentProtocolError> {
    if schema_version != super::SCHEMA_VERSION_V1 || schema_id != expected_id {
        return Err(NyxRemoteAgentProtocolError::InvalidField {
            context,
            field: "schema",
            detail: format!(
                "expected {}/{} but received {schema_version}/{schema_id}",
                super::SCHEMA_VERSION_V1,
                expected_id
            ),
        });
    }
    Ok(())
}

pub(crate) fn validate_text(
    context: &'static str,
    field: &'static str,
    value: &str,
) -> Result<(), NyxRemoteAgentProtocolError> {
    if value.is_empty() || value.trim() != value {
        return Err(NyxRemoteAgentProtocolError::InvalidField {
            context,
            field,
            detail: "must be non-empty and free of surrounding whitespace".to_owned(),
        });
    }
    Ok(())
}

pub(crate) fn validate_hash(
    context: &'static str,
    field: &'static str,
    value: &str,
) -> Result<(), NyxRemoteAgentProtocolError> {
    if value.len() != 64
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
    {
        return Err(NyxRemoteAgentProtocolError::InvalidField {
            context,
            field,
            detail: "must be exactly 64 lowercase hexadecimal characters".to_owned(),
        });
    }
    Ok(())
}

pub(crate) fn validate_canonical_strings(
    context: &'static str,
    field: &'static str,
    values: &[String],
    allow_empty: bool,
) -> Result<(), NyxRemoteAgentProtocolError> {
    if !allow_empty && values.is_empty() {
        return Err(NyxRemoteAgentProtocolError::InvalidField {
            context,
            field,
            detail: "must contain at least one value".to_owned(),
        });
    }
    if values
        .iter()
        .any(|value| value.is_empty() || value.trim() != value)
    {
        return Err(NyxRemoteAgentProtocolError::InvalidField {
            context,
            field,
            detail: "contains an empty or padded value".to_owned(),
        });
    }
    if values.windows(2).any(|pair| pair[0] >= pair[1]) {
        return Err(NyxRemoteAgentProtocolError::NonCanonical { context });
    }
    Ok(())
}

pub(crate) fn canonical_strings<I, S>(values: I) -> Vec<String>
where
    I: IntoIterator<Item = S>,
    S: Into<String>,
{
    let mut values: Vec<String> = values.into_iter().map(Into::into).collect();
    values.sort();
    values.dedup();
    values
}

pub(crate) fn raw_json_sha256<T: Serialize>(
    value: &T,
) -> Result<String, NyxRemoteAgentProtocolError> {
    serde_json::to_vec(value)
        .map(|bytes| raw_sha256_hex(&bytes))
        .map_err(|error| NyxRemoteAgentProtocolError::Json(error.to_string()))
}

pub(crate) fn verify_hash<T: Serialize>(
    field: &'static str,
    actual: &str,
    value: &T,
) -> Result<(), NyxRemoteAgentProtocolError> {
    let expected = raw_json_sha256(value)?;
    if expected != actual {
        return Err(NyxRemoteAgentProtocolError::HashMismatch {
            field,
            expected,
            actual: actual.to_owned(),
        });
    }
    Ok(())
}
