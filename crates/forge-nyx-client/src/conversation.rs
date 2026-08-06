//! Typed client view of the Nyx-owned model, session, and conversation contract.
//!
//! These values are transient decoded server truth. ForgeOS does not persist a
//! competing model registry or conversation ledger.

use serde::{Deserialize, Serialize};
use std::fmt;

pub const MODELS_PATH: &str = "/v1/nyx/models";
pub const SESSIONS_PATH: &str = "/v1/nyx/sessions";
pub const SCHEMA_VERSION_V1: &str = "nyx.1.0";

const SCHEMA_MODEL_CATALOG: &str = "nyx.model_catalog.v1";
const SCHEMA_ROUTABLE_MODEL: &str = "nyx.routable_model.v1";
const SCHEMA_SESSION_CREATE: &str = "nyx.session_create.v1";
const SCHEMA_SESSION_VIEW: &str = "nyx.session_view.v1";
const SCHEMA_SESSION_LIST: &str = "nyx.session_list.v1";
const SCHEMA_SESSION_CONTROL: &str = "nyx.session_control.v1";
const SCHEMA_CONVERSATION_CREATE: &str = "nyx.conversation_create.v1";
const SCHEMA_CONVERSATION_VIEW: &str = "nyx.conversation_view.v1";
const SCHEMA_CONVERSATION_LIST: &str = "nyx.conversation_list.v1";
const SCHEMA_CONVERSATION_CONTROL: &str = "nyx.conversation_control.v1";
const SCHEMA_MESSAGE_CREATE: &str = "nyx.conversation_message_create.v1";
const SCHEMA_MESSAGE: &str = "nyx.conversation_message.v1";
const SCHEMA_MESSAGE_LIST: &str = "nyx.conversation_message_list.v1";
const SCHEMA_RESPONSE: &str = "nyx.conversation_response.v1";
const SCHEMA_EVENT: &str = "nyx.conversation_event.v1";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NyxConversationProtocolError {
    Json(String),
    UnsupportedSchema {
        context: &'static str,
        expected: &'static str,
        found: String,
    },
    InvalidField {
        context: &'static str,
        field: &'static str,
        detail: String,
    },
    ImmutableMismatch {
        field: &'static str,
    },
    NonCanonical {
        context: &'static str,
    },
}

impl fmt::Display for NyxConversationProtocolError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Json(detail) => write!(formatter, "decode Nyx conversation JSON: {detail}"),
            Self::UnsupportedSchema {
                context,
                expected,
                found,
            } => write!(
                formatter,
                "Nyx {context} uses schema '{found}', expected '{expected}'"
            ),
            Self::InvalidField {
                context,
                field,
                detail,
            } => write!(
                formatter,
                "Nyx {context} field '{field}' is invalid: {detail}"
            ),
            Self::ImmutableMismatch { field } => {
                write!(formatter, "Nyx conversation identity mismatch in '{field}'")
            }
            Self::NonCanonical { context } => {
                write!(
                    formatter,
                    "Nyx {context} is not canonically ordered or unique"
                )
            }
        }
    }
}

impl std::error::Error for NyxConversationProtocolError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NyxModelReadiness {
    Ready,
    Degraded,
    Unavailable,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NyxModelCapabilities {
    #[serde(default)]
    max_context_tokens: Option<u32>,
    #[serde(default)]
    tool_calling: bool,
    #[serde(default)]
    json_mode: bool,
    #[serde(default)]
    streaming: bool,
}

impl NyxModelCapabilities {
    pub const fn max_context_tokens(&self) -> Option<u32> {
        self.max_context_tokens
    }

    pub const fn tool_calling(&self) -> bool {
        self.tool_calling
    }

    pub const fn json_mode(&self) -> bool {
        self.json_mode
    }

    pub const fn streaming(&self) -> bool {
        self.streaming
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NyxRoutableModel {
    schema_version: String,
    schema_id: String,
    model_id: String,
    display_name: String,
    backend_id: String,
    backend_kind: String,
    capabilities: NyxModelCapabilities,
    readiness: NyxModelReadiness,
    readiness_reason: String,
    availability: String,
    default_constraints_profile: String,
}

impl NyxRoutableModel {
    pub fn model_id(&self) -> &str {
        &self.model_id
    }

    pub fn display_name(&self) -> &str {
        &self.display_name
    }

    pub fn backend_id(&self) -> &str {
        &self.backend_id
    }

    pub fn backend_kind(&self) -> &str {
        &self.backend_kind
    }

    pub fn capabilities(&self) -> &NyxModelCapabilities {
        &self.capabilities
    }

    pub const fn readiness(&self) -> NyxModelReadiness {
        self.readiness
    }

    pub fn readiness_reason(&self) -> &str {
        &self.readiness_reason
    }

    pub fn availability(&self) -> &str {
        &self.availability
    }

    pub fn default_constraints_profile(&self) -> &str {
        &self.default_constraints_profile
    }

    fn validate(&self) -> Result<(), NyxConversationProtocolError> {
        ensure_schema(
            "routable model",
            &self.schema_version,
            &self.schema_id,
            SCHEMA_ROUTABLE_MODEL,
        )?;
        for (field, value) in [
            ("model_id", self.model_id.as_str()),
            ("display_name", self.display_name.as_str()),
            ("backend_id", self.backend_id.as_str()),
            ("backend_kind", self.backend_kind.as_str()),
            ("readiness_reason", self.readiness_reason.as_str()),
            ("availability", self.availability.as_str()),
            (
                "default_constraints_profile",
                self.default_constraints_profile.as_str(),
            ),
        ] {
            validate_text("routable model", field, value)?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NyxModelCatalog {
    schema_version: String,
    schema_id: String,
    ordering: String,
    models: Vec<NyxRoutableModel>,
}

impl NyxModelCatalog {
    pub fn models(&self) -> &[NyxRoutableModel] {
        &self.models
    }

    pub fn find(&self, model_id: &str) -> Option<&NyxRoutableModel> {
        self.models.iter().find(|model| model.model_id == model_id)
    }

    pub(crate) fn validate(&self) -> Result<(), NyxConversationProtocolError> {
        ensure_schema(
            "model catalog",
            &self.schema_version,
            &self.schema_id,
            SCHEMA_MODEL_CATALOG,
        )?;
        if self.ordering != "model_id_asc_then_backend_id_asc" {
            return invalid("model catalog", "ordering", "unsupported ordering contract");
        }
        for model in &self.models {
            model.validate()?;
        }
        if !self.models.windows(2).all(|pair| {
            (pair[0].model_id.as_str(), pair[0].backend_id.as_str())
                < (pair[1].model_id.as_str(), pair[1].backend_id.as_str())
        }) {
            return Err(NyxConversationProtocolError::NonCanonical {
                context: "model catalog",
            });
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NyxSandboxSymlinkPolicy {
    #[default]
    DenyEscape,
    Allow,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NyxNetworkPolicy {
    #[default]
    Deny,
    Allow,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NyxWorkspaceConfig {
    root: String,
    #[serde(default)]
    symlink_policy: NyxSandboxSymlinkPolicy,
    #[serde(default = "default_exclude_dirs")]
    exclude_dirs: Vec<String>,
    #[serde(default = "default_max_file_bytes")]
    max_file_bytes: u64,
    #[serde(default = "default_max_list_entries")]
    max_list_entries: u32,
    #[serde(default = "default_max_list_depth")]
    max_list_depth: u32,
}

impl NyxWorkspaceConfig {
    pub fn new(root: impl Into<String>) -> Self {
        Self {
            root: root.into(),
            symlink_policy: NyxSandboxSymlinkPolicy::DenyEscape,
            exclude_dirs: default_exclude_dirs(),
            max_file_bytes: default_max_file_bytes(),
            max_list_entries: default_max_list_entries(),
            max_list_depth: default_max_list_depth(),
        }
    }

    pub fn root(&self) -> &str {
        &self.root
    }

    pub const fn symlink_policy(&self) -> NyxSandboxSymlinkPolicy {
        self.symlink_policy
    }

    pub fn exclude_dirs(&self) -> &[String] {
        &self.exclude_dirs
    }
}

fn default_exclude_dirs() -> Vec<String> {
    vec![
        ".git".to_owned(),
        "node_modules".to_owned(),
        "target".to_owned(),
    ]
}

const fn default_max_file_bytes() -> u64 {
    10 * 1024 * 1024
}

const fn default_max_list_entries() -> u32 {
    20_000
}

const fn default_max_list_depth() -> u32 {
    25
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NyxSessionCreate {
    schema_version: String,
    schema_id: String,
    request_id: String,
    name: String,
    workspace: NyxWorkspaceConfig,
    #[serde(default)]
    network_policy: NyxNetworkPolicy,
}

impl NyxSessionCreate {
    pub fn new(
        request_id: impl Into<String>,
        name: impl Into<String>,
        workspace_root: impl Into<String>,
    ) -> Self {
        Self {
            schema_version: SCHEMA_VERSION_V1.to_owned(),
            schema_id: SCHEMA_SESSION_CREATE.to_owned(),
            request_id: request_id.into(),
            name: name.into(),
            workspace: NyxWorkspaceConfig::new(workspace_root),
            network_policy: NyxNetworkPolicy::Deny,
        }
    }

    pub fn request_id(&self) -> &str {
        &self.request_id
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub const fn network_policy(&self) -> NyxNetworkPolicy {
        self.network_policy
    }

    pub fn workspace(&self) -> &NyxWorkspaceConfig {
        &self.workspace
    }

    pub(crate) fn validate(&self) -> Result<(), NyxConversationProtocolError> {
        ensure_schema(
            "session create",
            &self.schema_version,
            &self.schema_id,
            SCHEMA_SESSION_CREATE,
        )?;
        validate_text("session create", "request_id", &self.request_id)?;
        validate_text("session create", "name", &self.name)?;
        validate_text("session create", "workspace.root", &self.workspace.root)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NyxSessionState {
    session_id: String,
    created_at: String,
    workspace: NyxWorkspaceConfig,
    #[serde(default)]
    network_policy: NyxNetworkPolicy,
    #[serde(default)]
    active_thread_id: Option<String>,
}

impl NyxSessionState {
    pub fn session_id(&self) -> &str {
        &self.session_id
    }

    pub fn workspace(&self) -> &NyxWorkspaceConfig {
        &self.workspace
    }

    pub fn active_thread_id(&self) -> Option<&str> {
        self.active_thread_id.as_deref()
    }

    pub const fn network_policy(&self) -> NyxNetworkPolicy {
        self.network_policy
    }

    fn validate(&self) -> Result<(), NyxConversationProtocolError> {
        validate_identifier("session state", "session_id", &self.session_id)?;
        validate_text("session state", "created_at", &self.created_at)?;
        validate_text("session state", "workspace.root", &self.workspace.root)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NyxSessionView {
    schema_version: String,
    schema_id: String,
    state: NyxSessionState,
    name: String,
    updated_at: String,
    closed_at: Option<String>,
    selected: bool,
}

impl NyxSessionView {
    pub fn state(&self) -> &NyxSessionState {
        &self.state
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub const fn selected(&self) -> bool {
        self.selected
    }

    pub const fn is_closed(&self) -> bool {
        self.closed_at.is_some()
    }

    pub(crate) fn validate(&self) -> Result<(), NyxConversationProtocolError> {
        ensure_schema(
            "session view",
            &self.schema_version,
            &self.schema_id,
            SCHEMA_SESSION_VIEW,
        )?;
        self.state.validate()?;
        validate_text("session view", "name", &self.name)?;
        validate_text("session view", "updated_at", &self.updated_at)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NyxSessionList {
    schema_version: String,
    schema_id: String,
    ordering: String,
    active_session_id: Option<String>,
    sessions: Vec<NyxSessionView>,
}

impl NyxSessionList {
    pub fn active_session_id(&self) -> Option<&str> {
        self.active_session_id.as_deref()
    }

    pub fn sessions(&self) -> &[NyxSessionView] {
        &self.sessions
    }

    pub(crate) fn validate(&self) -> Result<(), NyxConversationProtocolError> {
        ensure_schema(
            "session list",
            &self.schema_version,
            &self.schema_id,
            SCHEMA_SESSION_LIST,
        )?;
        if self.ordering != "session_id_asc" {
            return invalid("session list", "ordering", "unsupported ordering contract");
        }
        for session in &self.sessions {
            session.validate()?;
        }
        if !self
            .sessions
            .windows(2)
            .all(|pair| pair[0].state.session_id.as_str() < pair[1].state.session_id.as_str())
        {
            return Err(NyxConversationProtocolError::NonCanonical {
                context: "session list",
            });
        }
        let selected: Vec<_> = self.sessions.iter().filter(|item| item.selected).collect();
        match self.active_session_id.as_deref() {
            None if selected.is_empty() => Ok(()),
            Some(active)
                if selected.len() == 1 && selected[0].state.session_id.as_str() == active =>
            {
                Ok(())
            }
            _ => Err(NyxConversationProtocolError::ImmutableMismatch {
                field: "active session selection",
            }),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NyxSessionControl {
    schema_version: String,
    schema_id: String,
    request_id: String,
}

impl NyxSessionControl {
    pub fn new(request_id: impl Into<String>) -> Self {
        Self {
            schema_version: SCHEMA_VERSION_V1.to_owned(),
            schema_id: SCHEMA_SESSION_CONTROL.to_owned(),
            request_id: request_id.into(),
        }
    }

    pub(crate) fn validate(&self) -> Result<(), NyxConversationProtocolError> {
        ensure_schema(
            "session control",
            &self.schema_version,
            &self.schema_id,
            SCHEMA_SESSION_CONTROL,
        )?;
        validate_text("session control", "request_id", &self.request_id)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NyxConversationCreate {
    schema_version: String,
    schema_id: String,
    request_id: String,
    title: String,
}

impl NyxConversationCreate {
    pub fn new(request_id: impl Into<String>, title: impl Into<String>) -> Self {
        Self {
            schema_version: SCHEMA_VERSION_V1.to_owned(),
            schema_id: SCHEMA_CONVERSATION_CREATE.to_owned(),
            request_id: request_id.into(),
            title: title.into(),
        }
    }

    pub fn request_id(&self) -> &str {
        &self.request_id
    }

    pub fn title(&self) -> &str {
        &self.title
    }

    pub(crate) fn validate(&self) -> Result<(), NyxConversationProtocolError> {
        ensure_schema(
            "conversation create",
            &self.schema_version,
            &self.schema_id,
            SCHEMA_CONVERSATION_CREATE,
        )?;
        validate_text("conversation create", "request_id", &self.request_id)?;
        validate_text("conversation create", "title", &self.title)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NyxThread {
    thread_id: String,
    session_id: String,
    created_at: String,
    title: String,
}

impl NyxThread {
    pub fn thread_id(&self) -> &str {
        &self.thread_id
    }

    pub fn session_id(&self) -> &str {
        &self.session_id
    }

    pub fn title(&self) -> &str {
        &self.title
    }

    fn validate(&self) -> Result<(), NyxConversationProtocolError> {
        validate_identifier("conversation thread", "thread_id", &self.thread_id)?;
        validate_identifier("conversation thread", "session_id", &self.session_id)?;
        validate_text("conversation thread", "created_at", &self.created_at)?;
        validate_text("conversation thread", "title", &self.title)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NyxConversationView {
    schema_version: String,
    schema_id: String,
    thread: NyxThread,
    updated_at: String,
    closed_at: Option<String>,
}

impl NyxConversationView {
    pub fn thread(&self) -> &NyxThread {
        &self.thread
    }

    pub const fn is_closed(&self) -> bool {
        self.closed_at.is_some()
    }

    pub(crate) fn validate(&self) -> Result<(), NyxConversationProtocolError> {
        ensure_schema(
            "conversation view",
            &self.schema_version,
            &self.schema_id,
            SCHEMA_CONVERSATION_VIEW,
        )?;
        self.thread.validate()?;
        validate_text("conversation view", "updated_at", &self.updated_at)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NyxConversationList {
    schema_version: String,
    schema_id: String,
    session_id: String,
    ordering: String,
    conversations: Vec<NyxConversationView>,
}

impl NyxConversationList {
    pub fn session_id(&self) -> &str {
        &self.session_id
    }

    pub fn conversations(&self) -> &[NyxConversationView] {
        &self.conversations
    }

    pub(crate) fn validate_for(
        &self,
        session_id: &str,
    ) -> Result<(), NyxConversationProtocolError> {
        ensure_schema(
            "conversation list",
            &self.schema_version,
            &self.schema_id,
            SCHEMA_CONVERSATION_LIST,
        )?;
        if self.session_id != session_id {
            return Err(NyxConversationProtocolError::ImmutableMismatch {
                field: "conversation list session_id",
            });
        }
        if self.ordering != "conversation_id_asc" {
            return invalid(
                "conversation list",
                "ordering",
                "unsupported ordering contract",
            );
        }
        for conversation in &self.conversations {
            conversation.validate()?;
            if conversation.thread.session_id != self.session_id {
                return Err(NyxConversationProtocolError::ImmutableMismatch {
                    field: "conversation session_id",
                });
            }
        }
        if !self
            .conversations
            .windows(2)
            .all(|pair| pair[0].thread.thread_id.as_str() < pair[1].thread.thread_id.as_str())
        {
            return Err(NyxConversationProtocolError::NonCanonical {
                context: "conversation list",
            });
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NyxConversationControl {
    schema_version: String,
    schema_id: String,
    request_id: String,
}

impl NyxConversationControl {
    pub fn new(request_id: impl Into<String>) -> Self {
        Self {
            schema_version: SCHEMA_VERSION_V1.to_owned(),
            schema_id: SCHEMA_CONVERSATION_CONTROL.to_owned(),
            request_id: request_id.into(),
        }
    }

    pub(crate) fn validate(&self) -> Result<(), NyxConversationProtocolError> {
        ensure_schema(
            "conversation control",
            &self.schema_version,
            &self.schema_id,
            SCHEMA_CONVERSATION_CONTROL,
        )?;
        validate_text("conversation control", "request_id", &self.request_id)
    }
}

mod message;
mod stream;
pub use message::*;
pub use stream::*;

pub(crate) fn decode<T: for<'de> Deserialize<'de>>(
    bytes: &[u8],
) -> Result<T, NyxConversationProtocolError> {
    serde_json::from_slice(bytes)
        .map_err(|error| NyxConversationProtocolError::Json(error.to_string()))
}

fn ensure_schema(
    context: &'static str,
    version: &str,
    schema_id: &str,
    expected: &'static str,
) -> Result<(), NyxConversationProtocolError> {
    if version != SCHEMA_VERSION_V1 {
        return Err(NyxConversationProtocolError::UnsupportedSchema {
            context,
            expected: SCHEMA_VERSION_V1,
            found: version.to_owned(),
        });
    }
    if schema_id != expected {
        return Err(NyxConversationProtocolError::UnsupportedSchema {
            context,
            expected,
            found: schema_id.to_owned(),
        });
    }
    Ok(())
}

fn validate_text(
    context: &'static str,
    field: &'static str,
    value: &str,
) -> Result<(), NyxConversationProtocolError> {
    if value.trim().is_empty()
        || value
            .chars()
            .any(|character| matches!(character, '\r' | '\n' | '\0'))
    {
        return invalid(context, field, "must be non-empty single-line text");
    }
    Ok(())
}

fn validate_identifier(
    context: &'static str,
    field: &'static str,
    value: &str,
) -> Result<(), NyxConversationProtocolError> {
    validate_text(context, field, value)?;
    if !value
        .bytes()
        .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-' | b'.'))
    {
        return invalid(context, field, "contains unsupported identity bytes");
    }
    Ok(())
}

fn invalid<T>(
    context: &'static str,
    field: &'static str,
    detail: impl Into<String>,
) -> Result<T, NyxConversationProtocolError> {
    Err(NyxConversationProtocolError::InvalidField {
        context,
        field,
        detail: detail.into(),
    })
}
