//! Thin HTTP client for Nyx-owned model and conversation state.

use crate::conversation::{
    MODELS_PATH, NyxConversationControl, NyxConversationCreate, NyxConversationList,
    NyxConversationMessageCreate, NyxConversationMessageList, NyxConversationProtocolError,
    NyxConversationResponse, NyxConversationView, NyxModelCatalog, NyxOpenAiChatRequest,
    NyxOpenAiChatStream, NyxSessionControl, NyxSessionCreate, NyxSessionList, NyxSessionView,
    OPENAI_CHAT_COMPLETIONS_PATH, SESSIONS_PATH, decode,
};
use crate::transport::{
    NyxClientConfig, NyxHttpMethod, NyxIncompatibility, NyxJsonRequestFailure,
    NyxUnavailableReason, request_json, request_sse,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fmt;

const MODELS_LABEL: &str = "Nyx model catalog";
const SESSIONS_LABEL: &str = "Nyx sessions";
const SESSION_LABEL: &str = "Nyx session";
const CONVERSATIONS_LABEL: &str = "Nyx conversations";
const CONVERSATION_LABEL: &str = "Nyx conversation";
const MESSAGES_LABEL: &str = "Nyx conversation messages";

#[derive(Debug, Clone)]
pub struct NyxConversationClient {
    config: NyxClientConfig,
}

impl NyxConversationClient {
    pub const fn new(config: NyxClientConfig) -> Self {
        Self { config }
    }

    pub fn config(&self) -> &NyxClientConfig {
        &self.config
    }

    pub fn models(&self) -> Result<NyxModelCatalog, NyxConversationClientError> {
        let bytes = self.get(MODELS_LABEL, MODELS_PATH, 200)?;
        let catalog: NyxModelCatalog = decode(&bytes)?;
        catalog.validate()?;
        Ok(catalog)
    }

    pub fn sessions(&self) -> Result<NyxSessionList, NyxConversationClientError> {
        let bytes = self.get(SESSIONS_LABEL, SESSIONS_PATH, 200)?;
        let sessions: NyxSessionList = decode(&bytes)?;
        sessions.validate()?;
        Ok(sessions)
    }

    pub fn create_session(
        &self,
        request: &NyxSessionCreate,
    ) -> Result<NyxSessionView, NyxConversationClientError> {
        request.validate()?;
        let bytes = self.post(SESSIONS_LABEL, SESSIONS_PATH, request, &[200, 201])?;
        let session: NyxSessionView = decode(&bytes)?;
        session.validate()?;
        if session.name() != request.name()
            || session.state().workspace() != request.workspace()
            || session.state().network_policy() != request.network_policy()
        {
            return Err(NyxConversationProtocolError::ImmutableMismatch {
                field: "session request attribution",
            }
            .into());
        }
        Ok(session)
    }

    pub fn session(&self, session_id: &str) -> Result<NyxSessionView, NyxConversationClientError> {
        validate_path_segment("session_id", session_id)?;
        let path = format!("{SESSIONS_PATH}/{session_id}");
        let bytes = self.get(SESSION_LABEL, &path, 200)?;
        let session: NyxSessionView = decode(&bytes)?;
        session.validate()?;
        if session.state().session_id() != session_id {
            return Err(NyxConversationProtocolError::ImmutableMismatch {
                field: "session_id",
            }
            .into());
        }
        Ok(session)
    }

    pub fn select_session(
        &self,
        session_id: &str,
        control: &NyxSessionControl,
    ) -> Result<NyxSessionView, NyxConversationClientError> {
        self.control_session(session_id, "select", control)
    }

    pub fn close_session(
        &self,
        session_id: &str,
        control: &NyxSessionControl,
    ) -> Result<NyxSessionView, NyxConversationClientError> {
        self.control_session(session_id, "close", control)
    }

    pub fn restore_session(
        &self,
        session_id: &str,
        control: &NyxSessionControl,
    ) -> Result<NyxSessionView, NyxConversationClientError> {
        self.control_session(session_id, "restore", control)
    }

    pub fn conversations(
        &self,
        session_id: &str,
    ) -> Result<NyxConversationList, NyxConversationClientError> {
        validate_path_segment("session_id", session_id)?;
        let path = format!("{SESSIONS_PATH}/{session_id}/conversations");
        let bytes = self.get(CONVERSATIONS_LABEL, &path, 200)?;
        let conversations: NyxConversationList = decode(&bytes)?;
        conversations.validate_for(session_id)?;
        Ok(conversations)
    }

    pub fn create_conversation(
        &self,
        session_id: &str,
        request: &NyxConversationCreate,
    ) -> Result<NyxConversationView, NyxConversationClientError> {
        validate_path_segment("session_id", session_id)?;
        request.validate()?;
        let path = format!("{SESSIONS_PATH}/{session_id}/conversations");
        let bytes = self.post(CONVERSATIONS_LABEL, &path, request, &[200, 201])?;
        let conversation: NyxConversationView = decode(&bytes)?;
        conversation.validate()?;
        if conversation.thread().session_id() != session_id
            || conversation.thread().title() != request.title()
        {
            return Err(NyxConversationProtocolError::ImmutableMismatch {
                field: "conversation request attribution",
            }
            .into());
        }
        Ok(conversation)
    }

    pub fn conversation(
        &self,
        conversation_id: &str,
    ) -> Result<NyxConversationView, NyxConversationClientError> {
        validate_path_segment("conversation_id", conversation_id)?;
        let path = format!("/v1/nyx/conversations/{conversation_id}");
        let bytes = self.get(CONVERSATION_LABEL, &path, 200)?;
        let conversation: NyxConversationView = decode(&bytes)?;
        conversation.validate()?;
        if conversation.thread().thread_id() != conversation_id {
            return Err(NyxConversationProtocolError::ImmutableMismatch {
                field: "conversation_id",
            }
            .into());
        }
        Ok(conversation)
    }

    pub fn select_conversation(
        &self,
        conversation_id: &str,
        control: &NyxConversationControl,
    ) -> Result<NyxConversationView, NyxConversationClientError> {
        self.control_conversation(conversation_id, "select", control)
    }

    pub fn close_conversation(
        &self,
        conversation_id: &str,
        control: &NyxConversationControl,
    ) -> Result<NyxConversationView, NyxConversationClientError> {
        self.control_conversation(conversation_id, "close", control)
    }

    pub fn restore_conversation(
        &self,
        conversation_id: &str,
        control: &NyxConversationControl,
    ) -> Result<NyxConversationView, NyxConversationClientError> {
        self.control_conversation(conversation_id, "restore", control)
    }

    pub fn messages(
        &self,
        session_id: &str,
        conversation_id: &str,
    ) -> Result<NyxConversationMessageList, NyxConversationClientError> {
        let path = message_path(session_id, conversation_id)?;
        let bytes = self.get(MESSAGES_LABEL, &path, 200)?;
        let messages: NyxConversationMessageList = decode(&bytes)?;
        messages.validate_for(session_id, conversation_id)?;
        Ok(messages)
    }

    pub fn send_message(
        &self,
        session_id: &str,
        conversation_id: &str,
        request: &NyxConversationMessageCreate,
    ) -> Result<NyxConversationResponse, NyxConversationClientError> {
        let path = message_path(session_id, conversation_id)?;
        request.validate()?;
        let bytes = self.post(MESSAGES_LABEL, &path, request, &[200])?;
        let response: NyxConversationResponse = decode(&bytes)?;
        response.validate_for(session_id, conversation_id, request)?;
        Ok(response)
    }

    pub fn stream_chat(
        &self,
        request: &NyxOpenAiChatRequest,
    ) -> Result<NyxOpenAiChatStream, NyxConversationClientError> {
        request.validate()?;
        let body = serde_json::to_vec(request)
            .map_err(|error| NyxConversationClientError::Serialization(error.to_string()))?;
        let response = request_sse(
            &self.config,
            "Nyx OpenAI chat stream",
            OPENAI_CHAT_COMPLETIONS_PATH,
            self.config.preferred_version(),
            &body,
        )?;
        if response.status != 200 {
            let error = decode_openai_server_error(&response.body)?;
            return Err(NyxConversationClientError::Rejected {
                path: OPENAI_CHAT_COMPLETIONS_PATH.to_owned(),
                status: response.status,
                error,
            });
        }
        let content_type = response
            .content_type
            .as_deref()
            .and_then(|value| value.split(';').next())
            .map(str::trim);
        if content_type != Some("text/event-stream") {
            return Err(NyxConversationProtocolError::InvalidField {
                context: "OpenAI stream response",
                field: "content-type",
                detail: format!(
                    "expected text/event-stream, found {}",
                    response.content_type.as_deref().unwrap_or("<missing>")
                ),
            }
            .into());
        }
        NyxOpenAiChatStream::decode(
            &response.body,
            response.contract_version,
            response.stream_schema.as_deref(),
            request,
        )
        .map_err(NyxConversationClientError::from)
    }

    fn control_session(
        &self,
        session_id: &str,
        action: &str,
        control: &NyxSessionControl,
    ) -> Result<NyxSessionView, NyxConversationClientError> {
        validate_path_segment("session_id", session_id)?;
        control.validate()?;
        let path = format!("{SESSIONS_PATH}/{session_id}/{action}");
        let bytes = self.post(SESSION_LABEL, &path, control, &[200])?;
        let session: NyxSessionView = decode(&bytes)?;
        session.validate()?;
        if session.state().session_id() != session_id {
            return Err(NyxConversationProtocolError::ImmutableMismatch {
                field: "session control identity",
            }
            .into());
        }
        Ok(session)
    }

    fn control_conversation(
        &self,
        conversation_id: &str,
        action: &str,
        control: &NyxConversationControl,
    ) -> Result<NyxConversationView, NyxConversationClientError> {
        validate_path_segment("conversation_id", conversation_id)?;
        control.validate()?;
        let path = format!("/v1/nyx/conversations/{conversation_id}/{action}");
        let bytes = self.post(CONVERSATION_LABEL, &path, control, &[200])?;
        let conversation: NyxConversationView = decode(&bytes)?;
        conversation.validate()?;
        if conversation.thread().thread_id() != conversation_id {
            return Err(NyxConversationProtocolError::ImmutableMismatch {
                field: "conversation control identity",
            }
            .into());
        }
        Ok(conversation)
    }

    fn get(
        &self,
        path_label: &'static str,
        path: &str,
        expected_status: u16,
    ) -> Result<Vec<u8>, NyxConversationClientError> {
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
    ) -> Result<Vec<u8>, NyxConversationClientError> {
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
    ) -> Result<Vec<u8>, NyxConversationClientError> {
        let serialized = body
            .map(serde_json::to_vec)
            .transpose()
            .map_err(|error| NyxConversationClientError::Serialization(error.to_string()))?;
        let response = request_json(
            &self.config,
            method,
            path_label,
            path,
            self.config.preferred_version(),
            serialized.as_deref(),
        )?;
        if !expected_statuses.contains(&response.status) {
            let error = decode_server_error(&response.body)?;
            return Err(NyxConversationClientError::Rejected {
                path: path.to_owned(),
                status: response.status,
                error,
            });
        }
        Ok(response.body)
    }
}

fn message_path(
    session_id: &str,
    conversation_id: &str,
) -> Result<String, NyxConversationClientError> {
    validate_path_segment("session_id", session_id)?;
    validate_path_segment("conversation_id", conversation_id)?;
    Ok(format!(
        "{SESSIONS_PATH}/{session_id}/conversations/{conversation_id}/messages"
    ))
}

fn validate_path_segment(
    field: &'static str,
    value: &str,
) -> Result<(), NyxConversationClientError> {
    if value.is_empty()
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-' | b'.'))
    {
        return Err(NyxConversationProtocolError::InvalidField {
            context: "conversation path",
            field,
            detail: "must be one non-empty identity segment".to_owned(),
        }
        .into());
    }
    Ok(())
}

#[derive(Debug, Clone, PartialEq)]
pub struct NyxConversationServerError {
    code: String,
    message: String,
    details: Option<Value>,
}

impl NyxConversationServerError {
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
pub enum NyxConversationClientError {
    Unavailable(NyxUnavailableReason),
    Incompatible(NyxIncompatibility),
    Protocol(NyxConversationProtocolError),
    Serialization(String),
    Rejected {
        path: String,
        status: u16,
        error: NyxConversationServerError,
    },
}

impl fmt::Display for NyxConversationClientError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unavailable(reason) => {
                write!(formatter, "Nyx conversation API unavailable: {reason:?}")
            }
            Self::Incompatible(reason) => {
                write!(formatter, "Nyx conversation API incompatible: {reason:?}")
            }
            Self::Protocol(error) => error.fmt(formatter),
            Self::Serialization(detail) => {
                write!(formatter, "serialize Nyx conversation request: {detail}")
            }
            Self::Rejected {
                path,
                status,
                error,
            } => write!(
                formatter,
                "Nyx conversation request {path} returned HTTP {status}: {}: {}",
                error.code, error.message
            ),
        }
    }
}

impl std::error::Error for NyxConversationClientError {}

impl From<NyxConversationProtocolError> for NyxConversationClientError {
    fn from(error: NyxConversationProtocolError) -> Self {
        Self::Protocol(error)
    }
}

impl From<NyxJsonRequestFailure> for NyxConversationClientError {
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

#[derive(Debug, Deserialize)]
struct OpenAiErrorEnvelope {
    error: OpenAiErrorBody,
}

#[derive(Debug, Deserialize)]
struct OpenAiErrorBody {
    code: String,
    message: String,
    #[serde(default)]
    metadata: Option<Value>,
}

fn decode_openai_server_error(
    bytes: &[u8],
) -> Result<NyxConversationServerError, NyxConversationClientError> {
    let envelope: OpenAiErrorEnvelope = serde_json::from_slice(bytes).map_err(|error| {
        NyxConversationProtocolError::Json(format!("decode Nyx OpenAI error response: {error}"))
    })?;
    if envelope.error.code.trim().is_empty() || envelope.error.message.trim().is_empty() {
        return Err(NyxConversationProtocolError::InvalidField {
            context: "OpenAI server error",
            field: "code or message",
            detail: "must be non-empty".to_owned(),
        }
        .into());
    }
    Ok(NyxConversationServerError {
        code: envelope.error.code,
        message: envelope.error.message,
        details: envelope.error.metadata,
    })
}

fn decode_server_error(
    bytes: &[u8],
) -> Result<NyxConversationServerError, NyxConversationClientError> {
    let envelope: ServerErrorEnvelope = serde_json::from_slice(bytes).map_err(|error| {
        NyxConversationProtocolError::Json(format!("decode Nyx error response: {error}"))
    })?;
    if envelope.error.code.trim().is_empty() || envelope.error.message.trim().is_empty() {
        return Err(NyxConversationProtocolError::InvalidField {
            context: "server error",
            field: "code or message",
            detail: "must be non-empty".to_owned(),
        }
        .into());
    }
    Ok(NyxConversationServerError {
        code: envelope.error.code,
        message: envelope.error.message,
        details: envelope.error.details,
    })
}
