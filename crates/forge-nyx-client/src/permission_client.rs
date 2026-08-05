//! Thin HTTP client for the Nyx-owned permission and immutable-resume API.
//!
//! This client carries exact requests and independently verifies returned hashes.
//! It stores no pending-action ledger, creates no checkpoint identity, and executes
//! no tool around Nyx.

use crate::permission::{
    decode_permission, NyxPermissionAuditReport, NyxPermissionCheckpoint,
    NyxPermissionCheckpointCreate, NyxPermissionDecision, NyxPermissionDecisionKind,
    NyxPermissionProtocolError, NyxPermissionResolution, NyxPermissionResume,
    NyxPermissionResumeResult, PERMISSION_AUDIT_PATH, PERMISSION_CHECKPOINTS_PATH,
    PERMISSION_CHECKPOINT_PATH_LABEL, PERMISSION_RESOLVE_PATH_LABEL, PERMISSION_RESUME_PATH,
};
use crate::transport::{
    request_json, NyxClientConfig, NyxHttpMethod, NyxIncompatibility, NyxJsonRequestFailure,
    NyxUnavailableReason,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NyxPermissionClient {
    config: NyxClientConfig,
}

impl NyxPermissionClient {
    pub const fn new(config: NyxClientConfig) -> Self {
        Self { config }
    }

    pub fn config(&self) -> &NyxClientConfig {
        &self.config
    }

    pub fn create_checkpoint(
        &self,
        create: &NyxPermissionCheckpointCreate,
    ) -> Result<NyxPermissionCheckpoint, NyxPermissionClientError> {
        let response = self.post(
            PERMISSION_CHECKPOINTS_PATH,
            PERMISSION_CHECKPOINTS_PATH,
            create,
            201,
        )?;
        let checkpoint: NyxPermissionCheckpoint = decode_permission(&response)?;
        checkpoint.validate()?;
        if checkpoint.request() != create.request() {
            return Err(NyxPermissionProtocolError::ImmutableMismatch { field: "request" }.into());
        }
        Ok(checkpoint)
    }

    pub fn checkpoint(
        &self,
        checkpoint_id: &str,
    ) -> Result<NyxPermissionCheckpoint, NyxPermissionClientError> {
        validate_path_segment(checkpoint_id)?;
        let path = format!("{PERMISSION_CHECKPOINTS_PATH}/{checkpoint_id}");
        let response = self.get(PERMISSION_CHECKPOINT_PATH_LABEL, &path, 200)?;
        let checkpoint: NyxPermissionCheckpoint = decode_permission(&response)?;
        checkpoint.validate()?;
        if checkpoint.checkpoint_id() != checkpoint_id {
            return Err(NyxPermissionProtocolError::ImmutableMismatch {
                field: "checkpoint_id",
            }
            .into());
        }
        Ok(checkpoint)
    }

    pub fn resolve_checkpoint(
        &self,
        checkpoint: &NyxPermissionCheckpoint,
        decision: NyxPermissionDecisionKind,
        reason: Option<String>,
    ) -> Result<NyxPermissionResolution, NyxPermissionClientError> {
        checkpoint.validate()?;
        validate_path_segment(checkpoint.checkpoint_id())?;
        let request = NyxPermissionDecision::for_checkpoint(checkpoint, decision, reason)?;
        let path = format!(
            "{PERMISSION_CHECKPOINTS_PATH}/{}/resolve",
            checkpoint.checkpoint_id()
        );
        let response = self.post(PERMISSION_RESOLVE_PATH_LABEL, &path, &request, 200)?;
        let resolution: NyxPermissionResolution = decode_permission(&response)?;
        resolution.validate_against(checkpoint, decision)?;
        Ok(resolution)
    }

    pub fn resume_approved(
        &self,
        resolution: &NyxPermissionResolution,
    ) -> Result<NyxPermissionResumeResult, NyxPermissionClientError> {
        let resume = NyxPermissionResume::new(resolution)?;
        self.resume_exact(&resume)
    }

    pub fn resume_exact(
        &self,
        resume: &NyxPermissionResume,
    ) -> Result<NyxPermissionResumeResult, NyxPermissionClientError> {
        let response = self.post(PERMISSION_RESUME_PATH, PERMISSION_RESUME_PATH, resume, 200)?;
        let result: NyxPermissionResumeResult = decode_permission(&response)?;
        result.validate_against(resume)?;
        Ok(result)
    }

    pub fn audit(&self) -> Result<NyxPermissionAuditReport, NyxPermissionClientError> {
        let response = self.get(PERMISSION_AUDIT_PATH, PERMISSION_AUDIT_PATH, 200)?;
        let report: NyxPermissionAuditReport = decode_permission(&response)?;
        report.validate()?;
        Ok(report)
    }

    fn get(
        &self,
        path_label: &'static str,
        path: &str,
        expected_status: u16,
    ) -> Result<Vec<u8>, NyxPermissionClientError> {
        self.exchange::<Value>(NyxHttpMethod::Get, path_label, path, None, expected_status)
    }

    fn post<T: Serialize>(
        &self,
        path_label: &'static str,
        path: &str,
        body: &T,
        expected_status: u16,
    ) -> Result<Vec<u8>, NyxPermissionClientError> {
        self.exchange(
            NyxHttpMethod::Post,
            path_label,
            path,
            Some(body),
            expected_status,
        )
    }

    fn exchange<T: Serialize>(
        &self,
        method: NyxHttpMethod,
        path_label: &'static str,
        path: &str,
        body: Option<&T>,
        expected_status: u16,
    ) -> Result<Vec<u8>, NyxPermissionClientError> {
        let serialized = body
            .map(serde_json::to_vec)
            .transpose()
            .map_err(|error| NyxPermissionClientError::Serialization(error.to_string()))?;
        let response = request_json(
            &self.config,
            method,
            path_label,
            path,
            self.config.preferred_version(),
            serialized.as_deref(),
        )
        .map_err(NyxPermissionClientError::from)?;
        if response.status != expected_status {
            let error = decode_server_error(&response.body)?;
            return Err(NyxPermissionClientError::Rejected {
                path: path.to_owned(),
                status: response.status,
                error,
            });
        }
        Ok(response.body)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct NyxPermissionServerError {
    code: String,
    message: String,
    details: Option<Value>,
}

impl NyxPermissionServerError {
    pub fn code(&self) -> &str {
        &self.code
    }

    pub fn message(&self) -> &str {
        &self.message
    }

    pub fn details(&self) -> Option<&Value> {
        self.details.as_ref()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum NyxPermissionClientError {
    Unavailable(NyxUnavailableReason),
    Incompatible(NyxIncompatibility),
    Protocol(NyxPermissionProtocolError),
    Serialization(String),
    Rejected {
        path: String,
        status: u16,
        error: NyxPermissionServerError,
    },
}

impl fmt::Display for NyxPermissionClientError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unavailable(reason) => {
                write!(formatter, "Nyx permission API unavailable: {reason:?}")
            }
            Self::Incompatible(reason) => {
                write!(formatter, "Nyx permission API incompatible: {reason:?}")
            }
            Self::Protocol(error) => error.fmt(formatter),
            Self::Serialization(detail) => {
                write!(formatter, "serialize Nyx permission request: {detail}")
            }
            Self::Rejected {
                path,
                status,
                error,
            } => write!(
                formatter,
                "Nyx permission request {path} returned HTTP {status}: {}: {}",
                error.code, error.message
            ),
        }
    }
}

impl std::error::Error for NyxPermissionClientError {}

impl From<NyxPermissionProtocolError> for NyxPermissionClientError {
    fn from(error: NyxPermissionProtocolError) -> Self {
        Self::Protocol(error)
    }
}

impl From<NyxJsonRequestFailure> for NyxPermissionClientError {
    fn from(error: NyxJsonRequestFailure) -> Self {
        match error {
            NyxJsonRequestFailure::Unavailable(reason) => Self::Unavailable(reason),
            NyxJsonRequestFailure::Incompatible(reason) => Self::Incompatible(reason),
        }
    }
}

#[derive(Debug, Deserialize)]
struct ServerErrorEnvelope {
    error: ServerErrorBody,
}

#[derive(Debug, Deserialize)]
struct ServerErrorBody {
    code: String,
    message: String,
    #[serde(default)]
    details: Option<Value>,
}

fn decode_server_error(bytes: &[u8]) -> Result<NyxPermissionServerError, NyxPermissionClientError> {
    let envelope: ServerErrorEnvelope = serde_json::from_slice(bytes).map_err(|error| {
        NyxPermissionClientError::Protocol(NyxPermissionProtocolError::Json(format!(
            "decode Nyx permission error response: {error}"
        )))
    })?;
    if envelope.error.code.trim().is_empty() || envelope.error.message.trim().is_empty() {
        return Err(NyxPermissionClientError::Protocol(
            NyxPermissionProtocolError::InvalidField {
                context: "permission error",
                field: "error",
                detail: "code and message must be non-empty".to_owned(),
            },
        ));
    }
    Ok(NyxPermissionServerError {
        code: envelope.error.code,
        message: envelope.error.message,
        details: envelope.error.details,
    })
}

fn validate_path_segment(value: &str) -> Result<(), NyxPermissionClientError> {
    if value.is_empty()
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-'))
    {
        return Err(NyxPermissionProtocolError::InvalidField {
            context: "permission route",
            field: "checkpoint_id",
            detail: "must be one safe path segment".to_owned(),
        }
        .into());
    }
    Ok(())
}
