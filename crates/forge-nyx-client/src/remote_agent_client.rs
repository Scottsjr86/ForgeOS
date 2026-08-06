//! HTTP client for Nyx-owned remote-agent run records.
//!
//! ForgeOS submits and inspects exact Nyx requests. Nyx remains authoritative for
//! routing, execution, cancellation, budgets, cost, persistence, and terminal state.

use crate::remote_agent::{
    NyxRemoteAgentProtocolError, NyxRemoteAgentRunControl, NyxRemoteAgentRunCreate,
    NyxRemoteAgentRunList, NyxRemoteAgentRunRecord, REMOTE_AGENT_CANCEL_PATH_LABEL,
    REMOTE_AGENT_CONTINUE_PATH_LABEL, REMOTE_AGENT_RUN_PATH_LABEL, REMOTE_AGENT_RUNS_PATH,
    decode_remote_agent,
};
use crate::transport::{
    NyxClientConfig, NyxHttpMethod, NyxIncompatibility, NyxJsonRequestFailure,
    NyxUnavailableReason, request_json,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NyxRemoteAgentClient {
    config: NyxClientConfig,
}

impl NyxRemoteAgentClient {
    pub const fn new(config: NyxClientConfig) -> Self {
        Self { config }
    }

    pub fn config(&self) -> &NyxClientConfig {
        &self.config
    }

    pub fn create_run(
        &self,
        request: &NyxRemoteAgentRunCreate,
    ) -> Result<NyxRemoteAgentRunRecord, NyxRemoteAgentClientError> {
        let response = self.post(
            REMOTE_AGENT_RUNS_PATH,
            REMOTE_AGENT_RUNS_PATH,
            request,
            &[200, 201],
        )?;
        let record: NyxRemoteAgentRunRecord = decode_remote_agent(&response)?;
        record.validate_against(request)?;
        Ok(record)
    }

    pub fn runs(&self) -> Result<NyxRemoteAgentRunList, NyxRemoteAgentClientError> {
        let response = self.get(REMOTE_AGENT_RUNS_PATH, REMOTE_AGENT_RUNS_PATH, 200)?;
        let runs: NyxRemoteAgentRunList = decode_remote_agent(&response)?;
        runs.validate()?;
        Ok(runs)
    }

    pub fn run(&self, run_id: &str) -> Result<NyxRemoteAgentRunRecord, NyxRemoteAgentClientError> {
        validate_path_segment("run_id", run_id)?;
        let path = format!("{REMOTE_AGENT_RUNS_PATH}/{run_id}");
        let response = self.get(REMOTE_AGENT_RUN_PATH_LABEL, &path, 200)?;
        let record: NyxRemoteAgentRunRecord = decode_remote_agent(&response)?;
        record.validate()?;
        if record.run_id() != run_id {
            return Err(NyxRemoteAgentProtocolError::ImmutableMismatch { field: "run_id" }.into());
        }
        Ok(record)
    }

    pub fn cancel(
        &self,
        record: &NyxRemoteAgentRunRecord,
    ) -> Result<NyxRemoteAgentRunRecord, NyxRemoteAgentClientError> {
        record.validate()?;
        let control = NyxRemoteAgentRunControl::for_record(record);
        self.control(record, "cancel", REMOTE_AGENT_CANCEL_PATH_LABEL, &control)
    }

    pub fn continue_run(
        &self,
        record: &NyxRemoteAgentRunRecord,
    ) -> Result<NyxRemoteAgentRunRecord, NyxRemoteAgentClientError> {
        record.validate()?;
        let control = NyxRemoteAgentRunControl::for_record(record);
        self.control(
            record,
            "continue",
            REMOTE_AGENT_CONTINUE_PATH_LABEL,
            &control,
        )
    }

    fn control(
        &self,
        prior: &NyxRemoteAgentRunRecord,
        action: &str,
        path_label: &'static str,
        control: &NyxRemoteAgentRunControl,
    ) -> Result<NyxRemoteAgentRunRecord, NyxRemoteAgentClientError> {
        validate_path_segment("run_id", prior.run_id())?;
        control.validate()?;
        let path = format!("{REMOTE_AGENT_RUNS_PATH}/{}/{action}", prior.run_id());
        let response = self.post(path_label, &path, control, &[200])?;
        let record: NyxRemoteAgentRunRecord = decode_remote_agent(&response)?;
        record.validate()?;
        for (field, expected, actual) in [
            ("task_id", prior.task_id(), record.task_id()),
            ("run_id", prior.run_id(), record.run_id()),
            (
                "request_sha256",
                prior.request_sha256(),
                record.request_sha256(),
            ),
        ] {
            if expected != actual {
                return Err(NyxRemoteAgentProtocolError::ImmutableMismatch { field }.into());
            }
        }
        Ok(record)
    }

    fn get(
        &self,
        path_label: &'static str,
        path: &str,
        expected_status: u16,
    ) -> Result<Vec<u8>, NyxRemoteAgentClientError> {
        self.exchange::<Value>(
            NyxHttpMethod::Get,
            path_label,
            path,
            None,
            &[expected_status],
        )
    }

    fn post<T: Serialize>(
        &self,
        path_label: &'static str,
        path: &str,
        body: &T,
        expected_statuses: &[u16],
    ) -> Result<Vec<u8>, NyxRemoteAgentClientError> {
        self.exchange(
            NyxHttpMethod::Post,
            path_label,
            path,
            Some(body),
            expected_statuses,
        )
    }

    fn exchange<T: Serialize>(
        &self,
        method: NyxHttpMethod,
        path_label: &'static str,
        path: &str,
        body: Option<&T>,
        expected_statuses: &[u16],
    ) -> Result<Vec<u8>, NyxRemoteAgentClientError> {
        let serialized = body
            .map(serde_json::to_vec)
            .transpose()
            .map_err(|error| NyxRemoteAgentClientError::Serialization(error.to_string()))?;
        let response = request_json(
            &self.config,
            method,
            path_label,
            path,
            self.config.preferred_version(),
            serialized.as_deref(),
        )
        .map_err(NyxRemoteAgentClientError::from)?;
        if !expected_statuses.contains(&response.status) {
            let error = decode_server_error(&response.body)?;
            return Err(NyxRemoteAgentClientError::Rejected {
                path: path.to_owned(),
                status: response.status,
                error,
            });
        }
        Ok(response.body)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct NyxRemoteAgentServerError {
    code: String,
    message: String,
    details: Option<Value>,
}

impl NyxRemoteAgentServerError {
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
pub enum NyxRemoteAgentClientError {
    Unavailable(NyxUnavailableReason),
    Incompatible(NyxIncompatibility),
    Protocol(NyxRemoteAgentProtocolError),
    Serialization(String),
    Rejected {
        path: String,
        status: u16,
        error: NyxRemoteAgentServerError,
    },
}

impl fmt::Display for NyxRemoteAgentClientError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unavailable(reason) => {
                write!(formatter, "Nyx remote-agent API unavailable: {reason:?}")
            }
            Self::Incompatible(reason) => {
                write!(formatter, "Nyx remote-agent API incompatible: {reason:?}")
            }
            Self::Protocol(error) => error.fmt(formatter),
            Self::Serialization(detail) => {
                write!(formatter, "serialize Nyx remote-agent request: {detail}")
            }
            Self::Rejected {
                path,
                status,
                error,
            } => write!(
                formatter,
                "Nyx remote-agent request {path} returned HTTP {status}: {}: {}",
                error.code, error.message
            ),
        }
    }
}

impl std::error::Error for NyxRemoteAgentClientError {}

impl From<NyxRemoteAgentProtocolError> for NyxRemoteAgentClientError {
    fn from(error: NyxRemoteAgentProtocolError) -> Self {
        Self::Protocol(error)
    }
}

impl From<NyxJsonRequestFailure> for NyxRemoteAgentClientError {
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

fn decode_server_error(
    bytes: &[u8],
) -> Result<NyxRemoteAgentServerError, NyxRemoteAgentClientError> {
    let envelope: ServerErrorEnvelope = serde_json::from_slice(bytes).map_err(|error| {
        NyxRemoteAgentClientError::Protocol(NyxRemoteAgentProtocolError::Json(format!(
            "decode Nyx remote-agent error response: {error}"
        )))
    })?;
    if envelope.error.code.trim().is_empty() || envelope.error.message.trim().is_empty() {
        return Err(NyxRemoteAgentProtocolError::InvalidField {
            context: "remote-agent error",
            field: "error",
            detail: "code and message must be non-empty".to_owned(),
        }
        .into());
    }
    Ok(NyxRemoteAgentServerError {
        code: envelope.error.code,
        message: envelope.error.message,
        details: envelope.error.details,
    })
}

fn validate_path_segment(
    field: &'static str,
    value: &str,
) -> Result<(), NyxRemoteAgentClientError> {
    if value.is_empty()
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-'))
    {
        return Err(NyxRemoteAgentProtocolError::InvalidField {
            context: "remote-agent route",
            field,
            detail: "must be one safe path segment".to_owned(),
        }
        .into());
    }
    Ok(())
}
