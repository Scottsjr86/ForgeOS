//! OpenAI-compatible Nyx response-stream consumption.
//!
//! The compatibility stream is a Nyx-owned projection. ForgeOS validates and
//! presents frames as they arrive from the public surface; it does not synthesize
//! frames or use them as a competing conversation ledger.

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::protocol::NyxProtocolVersion;

use super::{NyxConversationProtocolError, invalid, validate_text};

pub const OPENAI_CHAT_COMPLETIONS_PATH: &str = "/v1/chat/completions";
pub const OPENAI_STREAM_SCHEMA_V1: &str = "nyx.openai_chat_stream_event.v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum NyxOpenAiChatRole {
    System,
    Developer,
    User,
    Assistant,
    Tool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct NyxOpenAiChatMessage {
    role: NyxOpenAiChatRole,
    content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    name: Option<String>,
}

impl NyxOpenAiChatMessage {
    pub fn new(role: NyxOpenAiChatRole, content: impl Into<String>) -> Self {
        Self {
            role,
            content: content.into(),
            name: None,
        }
    }

    pub const fn role(&self) -> NyxOpenAiChatRole {
        self.role
    }

    pub fn content(&self) -> &str {
        &self.content
    }

    fn validate(&self) -> Result<(), NyxConversationProtocolError> {
        validate_message_content("OpenAI chat message", "content", &self.content)?;
        if let Some(name) = &self.name {
            validate_text("OpenAI chat message", "name", name)?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct NyxOpenAiChatRequest {
    model: String,
    messages: Vec<NyxOpenAiChatMessage>,
    stream: bool,
}

impl NyxOpenAiChatRequest {
    pub fn new(
        model: impl Into<String>,
        messages: impl IntoIterator<Item = NyxOpenAiChatMessage>,
    ) -> Self {
        Self {
            model: model.into(),
            messages: messages.into_iter().collect(),
            stream: true,
        }
    }

    pub fn model(&self) -> &str {
        &self.model
    }

    pub fn messages(&self) -> &[NyxOpenAiChatMessage] {
        &self.messages
    }

    pub(crate) fn validate(&self) -> Result<(), NyxConversationProtocolError> {
        validate_text("OpenAI chat request", "model", &self.model)?;
        if self.model != self.model.trim() {
            return invalid(
                "OpenAI chat request",
                "model",
                "must not contain surrounding whitespace",
            );
        }
        if !self.stream {
            return invalid(
                "OpenAI chat request",
                "stream",
                "ForgeOS stream requests must set stream=true",
            );
        }
        if self.messages.is_empty() {
            return invalid(
                "OpenAI chat request",
                "messages",
                "must contain at least one message",
            );
        }
        for message in &self.messages {
            message.validate()?;
        }
        if !self
            .messages
            .iter()
            .any(|message| message.role == NyxOpenAiChatRole::User)
        {
            return invalid(
                "OpenAI chat request",
                "messages",
                "must contain at least one user message",
            );
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub enum NyxOpenAiStreamEventKind {
    #[serde(rename = "response.created")]
    ResponseCreated,
    #[serde(rename = "response.output_text.delta")]
    OutputTextDelta,
    #[serde(rename = "response.tool_call")]
    ToolCall,
    #[serde(rename = "response.completed")]
    ResponseCompleted,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct NyxOpenAiStreamChoice {
    index: u32,
    delta: Value,
    finish_reason: Option<String>,
}

impl NyxOpenAiStreamChoice {
    pub fn delta(&self) -> &Value {
        &self.delta
    }

    pub fn finish_reason(&self) -> Option<&str> {
        self.finish_reason.as_deref()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
struct NyxOpenAiStreamMetadata {
    nyx: NyxOpenAiStreamMetadataBody,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
struct NyxOpenAiStreamMetadataBody {
    schema_id: String,
    sequence: u64,
    event: NyxOpenAiStreamEventKind,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct NyxOpenAiStreamFrame {
    id: String,
    object: String,
    created: i64,
    model: String,
    choices: Vec<NyxOpenAiStreamChoice>,
    metadata: NyxOpenAiStreamMetadata,
}

impl NyxOpenAiStreamFrame {
    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn model(&self) -> &str {
        &self.model
    }

    pub const fn sequence(&self) -> u64 {
        self.metadata.nyx.sequence
    }

    pub const fn event_kind(&self) -> NyxOpenAiStreamEventKind {
        self.metadata.nyx.event
    }

    pub fn choice(&self) -> &NyxOpenAiStreamChoice {
        &self.choices[0]
    }

    fn validate(&self) -> Result<(), NyxConversationProtocolError> {
        validate_text("OpenAI stream frame", "id", &self.id)?;
        validate_text("OpenAI stream frame", "model", &self.model)?;
        if self.object != "chat.completion.chunk" {
            return invalid(
                "OpenAI stream frame",
                "object",
                "must be chat.completion.chunk",
            );
        }
        if self.created < 0 {
            return invalid(
                "OpenAI stream frame",
                "created",
                "must be a non-negative Unix timestamp",
            );
        }
        if self.metadata.nyx.schema_id != OPENAI_STREAM_SCHEMA_V1 {
            return Err(NyxConversationProtocolError::UnsupportedSchema {
                context: "OpenAI stream frame",
                expected: OPENAI_STREAM_SCHEMA_V1,
                found: self.metadata.nyx.schema_id.clone(),
            });
        }
        if self.choices.len() != 1 || self.choices[0].index != 0 {
            return invalid(
                "OpenAI stream frame",
                "choices",
                "V1 requires exactly one choice at index zero",
            );
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct NyxOpenAiChatStream {
    selected_protocol: NyxProtocolVersion,
    frames: Vec<NyxOpenAiStreamFrame>,
}

impl NyxOpenAiChatStream {
    pub const fn selected_protocol(&self) -> NyxProtocolVersion {
        self.selected_protocol
    }

    pub fn frames(&self) -> &[NyxOpenAiStreamFrame] {
        &self.frames
    }

    pub fn rendered_text(&self) -> String {
        let mut text = String::new();
        for frame in &self.frames {
            if frame.event_kind() == NyxOpenAiStreamEventKind::OutputTextDelta
                && let Some(delta) = frame
                    .choice()
                    .delta()
                    .get("content")
                    .and_then(Value::as_str)
            {
                text.push_str(delta);
            }
        }
        text
    }

    pub(crate) fn decode(
        body: &[u8],
        selected_protocol: NyxProtocolVersion,
        stream_schema: Option<&str>,
        request: &NyxOpenAiChatRequest,
    ) -> Result<Self, NyxConversationProtocolError> {
        if stream_schema != Some(OPENAI_STREAM_SCHEMA_V1) {
            return Err(NyxConversationProtocolError::UnsupportedSchema {
                context: "OpenAI stream header",
                expected: OPENAI_STREAM_SCHEMA_V1,
                found: stream_schema.unwrap_or("<missing>").to_owned(),
            });
        }
        let text = std::str::from_utf8(body).map_err(|error| {
            NyxConversationProtocolError::Json(format!("decode Nyx SSE body as UTF-8: {error}"))
        })?;
        let normalized = text.replace("\r\n", "\n");
        let mut frames = Vec::new();
        let mut done = false;
        for block in normalized
            .split("\n\n")
            .filter(|block| !block.trim().is_empty())
        {
            let mut lines = block.lines();
            let line = lines
                .next()
                .ok_or_else(|| NyxConversationProtocolError::NonCanonical {
                    context: "OpenAI SSE block",
                })?;
            if lines.next().is_some() || !line.starts_with("data: ") {
                return Err(NyxConversationProtocolError::NonCanonical {
                    context: "OpenAI SSE block",
                });
            }
            let data = &line[6..];
            if data == "[DONE]" {
                if done {
                    return Err(NyxConversationProtocolError::NonCanonical {
                        context: "OpenAI SSE terminator",
                    });
                }
                done = true;
                continue;
            }
            if done {
                return Err(NyxConversationProtocolError::NonCanonical {
                    context: "OpenAI SSE data after terminator",
                });
            }
            let frame: NyxOpenAiStreamFrame = serde_json::from_str(data).map_err(|error| {
                NyxConversationProtocolError::Json(format!("decode Nyx SSE frame: {error}"))
            })?;
            frame.validate()?;
            frames.push(frame);
        }
        if !done || frames.is_empty() {
            return Err(NyxConversationProtocolError::NonCanonical {
                context: "OpenAI SSE completion",
            });
        }
        validate_frames(&frames, request)?;
        Ok(Self {
            selected_protocol,
            frames,
        })
    }
}

fn validate_frames(
    frames: &[NyxOpenAiStreamFrame],
    request: &NyxOpenAiChatRequest,
) -> Result<(), NyxConversationProtocolError> {
    let first = &frames[0];
    let expected_id = first.id.as_str();
    let expected_created = first.created;
    if first.event_kind() != NyxOpenAiStreamEventKind::ResponseCreated {
        return Err(NyxConversationProtocolError::NonCanonical {
            context: "OpenAI stream start",
        });
    }
    for (index, frame) in frames.iter().enumerate() {
        if frame.sequence() != index as u64
            || frame.id != expected_id
            || frame.created != expected_created
            || frame.model != request.model
        {
            return Err(NyxConversationProtocolError::ImmutableMismatch {
                field: "OpenAI stream identity or ordering",
            });
        }
        let is_last = index + 1 == frames.len();
        match frame.event_kind() {
            NyxOpenAiStreamEventKind::ResponseCreated
                if index == 0 && frame.choice().finish_reason.is_none() => {}
            NyxOpenAiStreamEventKind::OutputTextDelta | NyxOpenAiStreamEventKind::ToolCall
                if !is_last && frame.choice().finish_reason.is_none() => {}
            NyxOpenAiStreamEventKind::ResponseCompleted
                if is_last && frame.choice().finish_reason.is_some() => {}
            _ => {
                return Err(NyxConversationProtocolError::NonCanonical {
                    context: "OpenAI stream event order",
                });
            }
        }
    }
    Ok(())
}

fn validate_message_content(
    context: &'static str,
    field: &'static str,
    value: &str,
) -> Result<(), NyxConversationProtocolError> {
    if value.trim().is_empty() || value.contains('\0') {
        return invalid(context, field, "must contain non-NUL message content");
    }
    Ok(())
}
