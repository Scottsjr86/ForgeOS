//! Nyx-owned conversation messages and ordered response-event validation.

use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::{
    NyxConversationProtocolError, SCHEMA_EVENT, SCHEMA_MESSAGE, SCHEMA_MESSAGE_CREATE,
    SCHEMA_MESSAGE_LIST, SCHEMA_RESPONSE, SCHEMA_VERSION_V1, ensure_schema, invalid,
    validate_identifier, validate_text,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum NyxMessageRole {
    System,
    Developer,
    User,
    Assistant,
    Tool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NyxConversationModelAttribution {
    model_id: String,
    backend_id: String,
    backend_kind: String,
}

impl NyxConversationModelAttribution {
    pub fn model_id(&self) -> &str {
        &self.model_id
    }

    pub fn backend_id(&self) -> &str {
        &self.backend_id
    }

    pub fn backend_kind(&self) -> &str {
        &self.backend_kind
    }

    fn validate(&self) -> Result<(), NyxConversationProtocolError> {
        validate_text("model attribution", "model_id", &self.model_id)?;
        validate_text("model attribution", "backend_id", &self.backend_id)?;
        validate_text("model attribution", "backend_kind", &self.backend_kind)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NyxConversationMessage {
    schema_version: String,
    schema_id: String,
    message_id: String,
    session_id: String,
    conversation_id: String,
    sequence: u64,
    role: NyxMessageRole,
    content: String,
    created_at: String,
    run_id: Option<String>,
    model: Option<NyxConversationModelAttribution>,
}

impl NyxConversationMessage {
    pub fn message_id(&self) -> &str {
        &self.message_id
    }

    pub fn session_id(&self) -> &str {
        &self.session_id
    }

    pub fn conversation_id(&self) -> &str {
        &self.conversation_id
    }

    pub const fn sequence(&self) -> u64 {
        self.sequence
    }

    pub const fn role(&self) -> NyxMessageRole {
        self.role
    }

    pub fn content(&self) -> &str {
        &self.content
    }

    pub fn run_id(&self) -> Option<&str> {
        self.run_id.as_deref()
    }

    pub fn model(&self) -> Option<&NyxConversationModelAttribution> {
        self.model.as_ref()
    }

    fn validate(&self) -> Result<(), NyxConversationProtocolError> {
        ensure_schema(
            "conversation message",
            &self.schema_version,
            &self.schema_id,
            SCHEMA_MESSAGE,
        )?;
        validate_identifier("conversation message", "message_id", &self.message_id)?;
        validate_identifier("conversation message", "session_id", &self.session_id)?;
        validate_identifier(
            "conversation message",
            "conversation_id",
            &self.conversation_id,
        )?;
        validate_content("conversation message", "content", &self.content)?;
        validate_text("conversation message", "created_at", &self.created_at)?;
        if let Some(run_id) = &self.run_id {
            validate_identifier("conversation message", "run_id", run_id)?;
        }
        if let Some(model) = &self.model {
            model.validate()?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NyxConversationMessageList {
    schema_version: String,
    schema_id: String,
    session_id: String,
    conversation_id: String,
    ordering: String,
    messages: Vec<NyxConversationMessage>,
}

impl NyxConversationMessageList {
    pub fn messages(&self) -> &[NyxConversationMessage] {
        &self.messages
    }

    pub(crate) fn validate_for(
        &self,
        session_id: &str,
        conversation_id: &str,
    ) -> Result<(), NyxConversationProtocolError> {
        ensure_schema(
            "message list",
            &self.schema_version,
            &self.schema_id,
            SCHEMA_MESSAGE_LIST,
        )?;
        if self.session_id != session_id || self.conversation_id != conversation_id {
            return Err(NyxConversationProtocolError::ImmutableMismatch {
                field: "message list identity",
            });
        }
        if self.ordering != "sequence_asc_then_message_id_asc" {
            return invalid("message list", "ordering", "unsupported ordering contract");
        }
        for message in &self.messages {
            message.validate()?;
            if message.session_id != self.session_id
                || message.conversation_id != self.conversation_id
            {
                return Err(NyxConversationProtocolError::ImmutableMismatch {
                    field: "message identity",
                });
            }
        }
        if !self
            .messages
            .iter()
            .enumerate()
            .all(|(index, message)| message.sequence == index as u64)
            || !self.messages.windows(2).all(|pair| {
                (pair[0].sequence, pair[0].message_id.as_str())
                    < (pair[1].sequence, pair[1].message_id.as_str())
            })
        {
            return Err(NyxConversationProtocolError::NonCanonical {
                context: "message list",
            });
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NyxConversationMessageCreate {
    schema_version: String,
    schema_id: String,
    request_id: String,
    model: String,
    content: String,
    #[serde(default)]
    stream: bool,
}

impl NyxConversationMessageCreate {
    pub fn new(
        request_id: impl Into<String>,
        model: impl Into<String>,
        content: impl Into<String>,
    ) -> Self {
        Self {
            schema_version: SCHEMA_VERSION_V1.to_owned(),
            schema_id: SCHEMA_MESSAGE_CREATE.to_owned(),
            request_id: request_id.into(),
            model: model.into(),
            content: content.into(),
            stream: false,
        }
    }

    pub fn request_id(&self) -> &str {
        &self.request_id
    }

    pub fn model(&self) -> &str {
        &self.model
    }

    pub fn content(&self) -> &str {
        &self.content
    }

    pub(crate) fn validate(&self) -> Result<(), NyxConversationProtocolError> {
        ensure_schema(
            "message create",
            &self.schema_version,
            &self.schema_id,
            SCHEMA_MESSAGE_CREATE,
        )?;
        validate_text("message create", "request_id", &self.request_id)?;
        validate_text("message create", "model", &self.model)?;
        validate_content("message create", "content", &self.content)?;
        if self.model != self.model.trim() {
            return invalid(
                "message create",
                "model",
                "must not contain surrounding whitespace",
            );
        }
        if self.stream {
            return invalid(
                "message create",
                "stream",
                "native ForgeOS conversation requests consume ordered JSON events",
            );
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NyxConversationEventKind {
    ResponseCreated,
    OutputTextDelta,
    ToolCall,
    ResponseCompleted,
    ResponseFailed,
    ResponseCancelled,
}

impl NyxConversationEventKind {
    pub const fn is_terminal(self) -> bool {
        matches!(
            self,
            Self::ResponseCompleted | Self::ResponseFailed | Self::ResponseCancelled
        )
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NyxConversationEvent {
    schema_version: String,
    schema_id: String,
    run_id: String,
    session_id: String,
    conversation_id: String,
    sequence: u64,
    choice_index: u32,
    event_kind: NyxConversationEventKind,
    payload: Value,
}

impl NyxConversationEvent {
    pub const fn sequence(&self) -> u64 {
        self.sequence
    }

    pub const fn event_kind(&self) -> NyxConversationEventKind {
        self.event_kind
    }

    pub fn payload(&self) -> &Value {
        &self.payload
    }

    fn validate(&self) -> Result<(), NyxConversationProtocolError> {
        ensure_schema(
            "conversation event",
            &self.schema_version,
            &self.schema_id,
            SCHEMA_EVENT,
        )?;
        validate_identifier("conversation event", "run_id", &self.run_id)?;
        validate_identifier("conversation event", "session_id", &self.session_id)?;
        validate_identifier(
            "conversation event",
            "conversation_id",
            &self.conversation_id,
        )?;
        if self.choice_index != 0 {
            return invalid(
                "conversation event",
                "choice_index",
                "V1 requires choice index zero",
            );
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NyxConversationResponse {
    schema_version: String,
    schema_id: String,
    run_id: String,
    session_id: String,
    conversation_id: String,
    model: NyxConversationModelAttribution,
    user_message: NyxConversationMessage,
    assistant_message: NyxConversationMessage,
    events: Vec<NyxConversationEvent>,
}

impl NyxConversationResponse {
    pub fn run_id(&self) -> &str {
        &self.run_id
    }

    pub fn session_id(&self) -> &str {
        &self.session_id
    }

    pub fn conversation_id(&self) -> &str {
        &self.conversation_id
    }

    pub fn model(&self) -> &NyxConversationModelAttribution {
        &self.model
    }

    pub fn user_message(&self) -> &NyxConversationMessage {
        &self.user_message
    }

    pub fn assistant_message(&self) -> &NyxConversationMessage {
        &self.assistant_message
    }

    pub fn events(&self) -> &[NyxConversationEvent] {
        &self.events
    }

    pub(crate) fn validate_for(
        &self,
        session_id: &str,
        conversation_id: &str,
        request: &NyxConversationMessageCreate,
    ) -> Result<(), NyxConversationProtocolError> {
        ensure_schema(
            "conversation response",
            &self.schema_version,
            &self.schema_id,
            SCHEMA_RESPONSE,
        )?;
        validate_identifier("conversation response", "run_id", &self.run_id)?;
        if self.session_id != session_id || self.conversation_id != conversation_id {
            return Err(NyxConversationProtocolError::ImmutableMismatch {
                field: "conversation response identity",
            });
        }
        self.model.validate()?;
        self.user_message.validate()?;
        self.assistant_message.validate()?;
        if self.model.model_id != request.model
            || self.user_message.session_id != self.session_id
            || self.user_message.conversation_id != self.conversation_id
            || self.user_message.role != NyxMessageRole::User
            || self.user_message.content != request.content
            || self.user_message.run_id.is_some()
            || self.user_message.model.is_some()
            || self.assistant_message.session_id != self.session_id
            || self.assistant_message.conversation_id != self.conversation_id
            || self.assistant_message.role != NyxMessageRole::Assistant
            || self.assistant_message.run_id.as_deref() != Some(self.run_id.as_str())
            || self.assistant_message.model.as_ref() != Some(&self.model)
            || self.assistant_message.sequence != self.user_message.sequence.saturating_add(1)
        {
            return Err(NyxConversationProtocolError::ImmutableMismatch {
                field: "message or model attribution",
            });
        }
        if self.events.is_empty() {
            return invalid(
                "conversation response",
                "events",
                "must contain ordered response events",
            );
        }
        for (index, event) in self.events.iter().enumerate() {
            event.validate()?;
            if event.run_id != self.run_id
                || event.session_id != self.session_id
                || event.conversation_id != self.conversation_id
                || event.sequence != index as u64
            {
                return Err(NyxConversationProtocolError::ImmutableMismatch {
                    field: "event identity or ordering",
                });
            }
        }
        let terminal_count = self
            .events
            .iter()
            .filter(|event| event.event_kind.is_terminal())
            .count();
        if self.events[0].event_kind != NyxConversationEventKind::ResponseCreated
            || terminal_count != 1
            || !self
                .events
                .last()
                .is_some_and(|event| event.event_kind.is_terminal())
        {
            return Err(NyxConversationProtocolError::NonCanonical {
                context: "conversation terminal events",
            });
        }
        Ok(())
    }
}

fn validate_content(
    context: &'static str,
    field: &'static str,
    value: &str,
) -> Result<(), NyxConversationProtocolError> {
    if value.trim().is_empty() || value.contains('\0') {
        return invalid(context, field, "must contain non-NUL message content");
    }
    Ok(())
}
