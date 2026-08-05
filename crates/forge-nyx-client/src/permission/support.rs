//! Validation helpers for the Nyx permission client contract.

use super::{NyxPermissionCheckpoint, NyxPermissionCheckpointStatus, SCHEMA_VERSION_V1};
use crate::canonical_json::raw_sha256_json;
use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NyxPermissionProtocolError {
    Json(String),
    MissingField {
        context: &'static str,
        field: &'static str,
    },
    UnsupportedSchema {
        context: &'static str,
        expected: &'static str,
        version: String,
        schema_id: String,
    },
    InvalidField {
        context: &'static str,
        field: &'static str,
        detail: String,
    },
    HashMismatch {
        field: &'static str,
        expected: String,
        found: String,
    },
    ImmutableMismatch {
        field: &'static str,
    },
    InvalidStatusFields(NyxPermissionCheckpointStatus),
    UnexpectedStatus {
        expected: NyxPermissionCheckpointStatus,
        found: NyxPermissionCheckpointStatus,
    },
    NonCanonical {
        context: &'static str,
    },
}

impl fmt::Display for NyxPermissionProtocolError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Json(detail) => write!(formatter, "invalid Nyx permission JSON: {detail}"),
            Self::MissingField { context, field } => {
                write!(formatter, "Nyx {context} is missing '{field}'")
            }
            Self::UnsupportedSchema {
                context,
                expected,
                version,
                schema_id,
            } => write!(
                formatter,
                "Nyx {context} schema is {version}/{schema_id}, expected {SCHEMA_VERSION_V1}/{expected}"
            ),
            Self::InvalidField {
                context,
                field,
                detail,
            } => write!(formatter, "Nyx {context} field '{field}' is invalid: {detail}"),
            Self::HashMismatch {
                field,
                expected,
                found,
            } => write!(
                formatter,
                "Nyx permission hash '{field}' is '{found}', independently computed '{expected}'"
            ),
            Self::ImmutableMismatch { field } => {
                write!(formatter, "Nyx permission immutable field '{field}' changed")
            }
            Self::InvalidStatusFields(status) => {
                write!(formatter, "Nyx permission checkpoint fields contradict {status:?}")
            }
            Self::UnexpectedStatus { expected, found } => write!(
                formatter,
                "Nyx permission status is {found:?}, expected {expected:?}"
            ),
            Self::NonCanonical { context } => write!(formatter, "Nyx {context} is not canonical"),
        }
    }
}

impl std::error::Error for NyxPermissionProtocolError {}

pub(crate) fn decode_permission<T>(bytes: &[u8]) -> Result<T, NyxPermissionProtocolError>
where
    T: for<'de> Deserialize<'de>,
{
    serde_json::from_slice(bytes)
        .map_err(|error| NyxPermissionProtocolError::Json(error.to_string()))
}

pub(super) fn ensure_schema(
    context: &'static str,
    version: &str,
    schema_id: &str,
    expected: &'static str,
) -> Result<(), NyxPermissionProtocolError> {
    if version != SCHEMA_VERSION_V1 || schema_id != expected {
        return Err(NyxPermissionProtocolError::UnsupportedSchema {
            context,
            expected,
            version: version.to_owned(),
            schema_id: schema_id.to_owned(),
        });
    }
    Ok(())
}

pub(super) fn validate_text(
    context: &'static str,
    field: &'static str,
    value: &str,
) -> Result<(), NyxPermissionProtocolError> {
    if value.trim().is_empty() || value.trim() != value {
        return Err(NyxPermissionProtocolError::InvalidField {
            context,
            field,
            detail: "must be non-empty without surrounding whitespace".to_owned(),
        });
    }
    Ok(())
}

pub(super) fn validate_route_id(
    field: &'static str,
    value: &str,
) -> Result<(), NyxPermissionProtocolError> {
    validate_text("permission route", field, value)?;
    if !value
        .bytes()
        .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-'))
    {
        return Err(NyxPermissionProtocolError::InvalidField {
            context: "permission route",
            field,
            detail: "contains characters unsafe for an exact path segment".to_owned(),
        });
    }
    Ok(())
}

pub(super) fn validate_hash(
    context: &'static str,
    field: &'static str,
    value: &str,
) -> Result<(), NyxPermissionProtocolError> {
    if value.len() != 64
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err(NyxPermissionProtocolError::InvalidField {
            context,
            field,
            detail: "must be a lowercase 64-character hexadecimal SHA-256".to_owned(),
        });
    }
    Ok(())
}

pub(super) fn canonical_strings<I, S>(values: I) -> Vec<String>
where
    I: IntoIterator<Item = S>,
    S: Into<String>,
{
    let mut values = values.into_iter().map(Into::into).collect::<Vec<_>>();
    values.sort();
    values.dedup();
    values
}

pub(super) fn validate_canonical_strings(
    context: &'static str,
    field: &'static str,
    values: &[String],
) -> Result<(), NyxPermissionProtocolError> {
    if values
        .iter()
        .any(|value| value.trim().is_empty() || value.trim() != value)
    {
        return Err(NyxPermissionProtocolError::InvalidField {
            context,
            field,
            detail: "contains an empty or whitespace-padded value".to_owned(),
        });
    }
    if values.windows(2).any(|pair| pair[0] >= pair[1]) {
        return Err(NyxPermissionProtocolError::NonCanonical { context });
    }
    Ok(())
}

pub(super) fn hash_json<T: Serialize>(
    context: &'static str,
    value: &T,
) -> Result<String, NyxPermissionProtocolError> {
    raw_sha256_json(value)
        .map_err(|error| NyxPermissionProtocolError::Json(format!("serialize {context}: {error}")))
}

pub(super) fn verify_hash(
    field: &'static str,
    expected: &str,
    found: &str,
) -> Result<(), NyxPermissionProtocolError> {
    if expected != found {
        return Err(NyxPermissionProtocolError::HashMismatch {
            field,
            expected: expected.to_owned(),
            found: found.to_owned(),
        });
    }
    Ok(())
}

pub(super) fn verify_immutable_checkpoint(
    before: &NyxPermissionCheckpoint,
    after: &NyxPermissionCheckpoint,
) -> Result<(), NyxPermissionProtocolError> {
    for (field, same) in [
        ("checkpoint_id", before.checkpoint_id == after.checkpoint_id),
        ("request", before.request == after.request),
        (
            "accepted_operation_id",
            before.accepted_operation_id == after.accepted_operation_id,
        ),
        (
            "request_sha256",
            before.request_sha256 == after.request_sha256,
        ),
        (
            "payload_sha256",
            before.payload_sha256 == after.payload_sha256,
        ),
        ("scope_sha256", before.scope_sha256 == after.scope_sha256),
        (
            "policy_decision",
            before.policy_decision == after.policy_decision,
        ),
        (
            "policy_decision_sha256",
            before.policy_decision_sha256 == after.policy_decision_sha256,
        ),
        (
            "predicted_effects",
            before.predicted_effects == after.predicted_effects,
        ),
        (
            "predicted_effects_sha256",
            before.predicted_effects_sha256 == after.predicted_effects_sha256,
        ),
        (
            "created_at_unix_ms",
            before.created_at_unix_ms == after.created_at_unix_ms,
        ),
        (
            "expires_at_unix_ms",
            before.expires_at_unix_ms == after.expires_at_unix_ms,
        ),
    ] {
        if !same {
            return Err(NyxPermissionProtocolError::ImmutableMismatch { field });
        }
    }
    Ok(())
}
