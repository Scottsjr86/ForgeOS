//! Thin ForgeOS representation of Nyx-owned remote-agent run contracts.
//!
//! Nyx_Server owns run creation, routing, execution, cancellation, budgets, cost,
//! persistence, and terminal state. ForgeOS constructs exact requests, consumes
//! returned records, and independently verifies immutable identities and bounds.

use serde::{Deserialize, Serialize};
use std::path::{Component, Path};

mod routing;
mod support;

pub use routing::{
    NyxBackendCapabilities, NyxBackendIdentity, NyxRouteCandidate, NyxRouteCost,
    NyxRoutableModelIdentity,
};
pub(crate) use support::decode_remote_agent;
pub use support::NyxRemoteAgentProtocolError;

use support::{
    canonical_strings, ensure_schema, raw_json_sha256, validate_canonical_strings, validate_hash,
    validate_text, verify_hash,
};

pub const REMOTE_AGENT_RUNS_PATH: &str = "/v1/nyx/agent/runs";
pub const REMOTE_AGENT_RUN_PATH_LABEL: &str = "/v1/nyx/agent/runs/:run_id";
pub const REMOTE_AGENT_CANCEL_PATH_LABEL: &str = "/v1/nyx/agent/runs/:run_id/cancel";
pub const REMOTE_AGENT_CONTINUE_PATH_LABEL: &str = "/v1/nyx/agent/runs/:run_id/continue";

pub(crate) const SCHEMA_VERSION_V1: &str = "nyx.1.0";
const SCHEMA_SCOPE: &str = "nyx.remote_agent_scope.v1";
const SCHEMA_SOURCE_BINDING: &str = "nyx.remote_agent_source_binding.v1";
const SCHEMA_BUDGET: &str = "nyx.remote_agent_budget.v1";
const SCHEMA_RUN_CREATE: &str = "nyx.remote_agent_run_create.v1";
const SCHEMA_RUN_CONTROL: &str = "nyx.remote_agent_run_control.v1";
const SCHEMA_RUN_RECORD: &str = "nyx.remote_agent_run_record.v1";
const SCHEMA_RUN_LIST: &str = "nyx.remote_agent_run_list.v1";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NyxRemoteAgentScope {
    schema_version: String,
    schema_id: String,
    allowed_paths: Vec<String>,
    deny_network: bool,
}

impl NyxRemoteAgentScope {
    pub fn new<I, S>(
        allowed_paths: I,
        deny_network: bool,
    ) -> Result<Self, NyxRemoteAgentProtocolError>
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        let scope = Self {
            schema_version: SCHEMA_VERSION_V1.to_owned(),
            schema_id: SCHEMA_SCOPE.to_owned(),
            allowed_paths: canonical_strings(allowed_paths),
            deny_network,
        };
        scope.validate()?;
        Ok(scope)
    }

    pub fn allowed_paths(&self) -> &[String] {
        &self.allowed_paths
    }

    pub const fn deny_network(&self) -> bool {
        self.deny_network
    }

    pub fn sha256(&self) -> Result<String, NyxRemoteAgentProtocolError> {
        raw_json_sha256(self)
    }

    pub(crate) fn validate(&self) -> Result<(), NyxRemoteAgentProtocolError> {
        ensure_schema(
            "remote-agent scope",
            &self.schema_version,
            &self.schema_id,
            SCHEMA_SCOPE,
        )?;
        validate_canonical_strings(
            "remote-agent allowed path set",
            "allowed_paths",
            &self.allowed_paths,
            false,
        )?;
        for path in &self.allowed_paths {
            if path.contains('\\') || path.contains("..") {
                return Err(NyxRemoteAgentProtocolError::InvalidField {
                    context: "remote-agent scope",
                    field: "allowed_paths",
                    detail: format!("path '{path}' contains a forbidden separator or traversal"),
                });
            }
            let candidate = Path::new(path);
            if candidate.is_absolute()
                || candidate.components().any(|component| {
                    matches!(
                        component,
                        Component::ParentDir | Component::RootDir | Component::Prefix(_)
                    )
                })
            {
                return Err(NyxRemoteAgentProtocolError::InvalidField {
                    context: "remote-agent scope",
                    field: "allowed_paths",
                    detail: format!("path '{path}' is absolute or traversing"),
                });
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NyxRemoteAgentSourceBinding {
    schema_version: String,
    schema_id: String,
    source_revision: String,
    workspace_root_sha256: String,
    worktree_id: String,
    worktree_clean: bool,
    scope: NyxRemoteAgentScope,
    scope_sha256: String,
}

impl NyxRemoteAgentSourceBinding {
    pub fn source_revision(&self) -> &str {
        &self.source_revision
    }

    pub fn workspace_root_sha256(&self) -> &str {
        &self.workspace_root_sha256
    }

    pub fn worktree_id(&self) -> &str {
        &self.worktree_id
    }

    pub const fn worktree_clean(&self) -> bool {
        self.worktree_clean
    }

    pub fn scope(&self) -> &NyxRemoteAgentScope {
        &self.scope
    }

    pub fn scope_sha256(&self) -> &str {
        &self.scope_sha256
    }

    fn validate(&self) -> Result<(), NyxRemoteAgentProtocolError> {
        ensure_schema(
            "remote-agent source binding",
            &self.schema_version,
            &self.schema_id,
            SCHEMA_SOURCE_BINDING,
        )?;
        validate_text(
            "remote-agent source binding",
            "source_revision",
            &self.source_revision,
        )?;
        validate_text(
            "remote-agent source binding",
            "worktree_id",
            &self.worktree_id,
        )?;
        validate_hash(
            "remote-agent source binding",
            "workspace_root_sha256",
            &self.workspace_root_sha256,
        )?;
        validate_hash(
            "remote-agent source binding",
            "scope_sha256",
            &self.scope_sha256,
        )?;
        self.scope.validate()?;
        verify_hash("scope_sha256", &self.scope_sha256, &self.scope)?;
        let expected_worktree_id = format!("nyxwt_{}", &self.workspace_root_sha256[..24]);
        if self.worktree_id != expected_worktree_id {
            return Err(NyxRemoteAgentProtocolError::ImmutableMismatch {
                field: "worktree_id",
            });
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NyxRemoteAgentBudget {
    schema_version: String,
    schema_id: String,
    max_model_turns: u32,
    max_tool_steps: u32,
    max_input_tokens: u32,
    max_output_tokens: u32,
    max_total_tokens: u32,
    max_elapsed_ms: u64,
    max_retries: u32,
    max_personas: u32,
    max_proposals: u32,
    max_monetary_microunits: Option<u64>,
}

impl Default for NyxRemoteAgentBudget {
    fn default() -> Self {
        Self {
            schema_version: SCHEMA_VERSION_V1.to_owned(),
            schema_id: SCHEMA_BUDGET.to_owned(),
            max_model_turns: 8,
            max_tool_steps: 8,
            max_input_tokens: 32_768,
            max_output_tokens: 4_096,
            max_total_tokens: 36_864,
            max_elapsed_ms: 120_000,
            max_retries: 1,
            max_personas: 1,
            max_proposals: 0,
            max_monetary_microunits: None,
        }
    }
}

impl NyxRemoteAgentBudget {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        max_model_turns: u32,
        max_tool_steps: u32,
        max_input_tokens: u32,
        max_output_tokens: u32,
        max_total_tokens: u32,
        max_elapsed_ms: u64,
        max_retries: u32,
        max_personas: u32,
        max_proposals: u32,
        max_monetary_microunits: Option<u64>,
    ) -> Result<Self, NyxRemoteAgentProtocolError> {
        let budget = Self {
            schema_version: SCHEMA_VERSION_V1.to_owned(),
            schema_id: SCHEMA_BUDGET.to_owned(),
            max_model_turns,
            max_tool_steps,
            max_input_tokens,
            max_output_tokens,
            max_total_tokens,
            max_elapsed_ms,
            max_retries,
            max_personas,
            max_proposals,
            max_monetary_microunits,
        };
        budget.validate()?;
        Ok(budget)
    }

    pub const fn max_model_turns(&self) -> u32 {
        self.max_model_turns
    }

    pub const fn max_tool_steps(&self) -> u32 {
        self.max_tool_steps
    }

    pub const fn max_input_tokens(&self) -> u32 {
        self.max_input_tokens
    }

    pub const fn max_output_tokens(&self) -> u32 {
        self.max_output_tokens
    }

    pub const fn max_total_tokens(&self) -> u32 {
        self.max_total_tokens
    }

    pub const fn max_elapsed_ms(&self) -> u64 {
        self.max_elapsed_ms
    }

    pub const fn max_retries(&self) -> u32 {
        self.max_retries
    }

    pub const fn max_personas(&self) -> u32 {
        self.max_personas
    }

    pub const fn max_proposals(&self) -> u32 {
        self.max_proposals
    }

    pub const fn max_monetary_microunits(&self) -> Option<u64> {
        self.max_monetary_microunits
    }

    fn validate(&self) -> Result<(), NyxRemoteAgentProtocolError> {
        ensure_schema(
            "remote-agent budget",
            &self.schema_version,
            &self.schema_id,
            SCHEMA_BUDGET,
        )?;
        if self.max_total_tokens < self.max_input_tokens
            || self.max_total_tokens < self.max_output_tokens
        {
            return Err(NyxRemoteAgentProtocolError::InvalidField {
                context: "remote-agent budget",
                field: "max_total_tokens",
                detail: "must not be smaller than either component token limit".to_owned(),
            });
        }
        if self.max_elapsed_ms == 0 || self.max_personas == 0 {
            return Err(NyxRemoteAgentProtocolError::InvalidField {
                context: "remote-agent budget",
                field: "nonzero limits",
                detail: "max_elapsed_ms and max_personas must be non-zero".to_owned(),
            });
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NyxRemoteAgentStartMode {
    Immediate,
    Deferred,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NyxRemoteAgentRunState {
    Queued,
    Running,
    Completed,
    Failed,
    Cancelled,
    BudgetHit,
}

impl NyxRemoteAgentRunState {
    pub const fn is_terminal(self) -> bool {
        matches!(
            self,
            Self::Completed | Self::Failed | Self::Cancelled | Self::BudgetHit
        )
    }

    const fn audit_tail(self) -> &'static str {
        match self {
            Self::Queued => "queued",
            Self::Running => "running",
            Self::Completed => "completed",
            Self::Failed => "failed",
            Self::Cancelled => "cancelled",
            Self::BudgetHit => "budget_hit",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NyxRemoteAgentRunCreate {
    schema_version: String,
    schema_id: String,
    idempotency_key: String,
    provider_id: String,
    model_id: String,
    prompt: String,
    expected_source_revision: String,
    expected_worktree_id: String,
    scope: NyxRemoteAgentScope,
    budget: NyxRemoteAgentBudget,
    start_mode: NyxRemoteAgentStartMode,
}

impl NyxRemoteAgentRunCreate {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        idempotency_key: impl Into<String>,
        provider_id: impl Into<String>,
        model_id: impl Into<String>,
        prompt: impl Into<String>,
        expected_source_revision: impl Into<String>,
        expected_worktree_id: impl Into<String>,
        scope: NyxRemoteAgentScope,
        budget: NyxRemoteAgentBudget,
        start_mode: NyxRemoteAgentStartMode,
    ) -> Result<Self, NyxRemoteAgentProtocolError> {
        let request = Self {
            schema_version: SCHEMA_VERSION_V1.to_owned(),
            schema_id: SCHEMA_RUN_CREATE.to_owned(),
            idempotency_key: idempotency_key.into(),
            provider_id: provider_id.into(),
            model_id: model_id.into(),
            prompt: prompt.into(),
            expected_source_revision: expected_source_revision.into(),
            expected_worktree_id: expected_worktree_id.into(),
            scope,
            budget,
            start_mode,
        };
        request.validate()?;
        Ok(request)
    }

    pub fn idempotency_key(&self) -> &str {
        &self.idempotency_key
    }

    pub fn provider_id(&self) -> &str {
        &self.provider_id
    }

    pub fn model_id(&self) -> &str {
        &self.model_id
    }

    pub fn prompt(&self) -> &str {
        &self.prompt
    }

    pub fn expected_source_revision(&self) -> &str {
        &self.expected_source_revision
    }

    pub fn expected_worktree_id(&self) -> &str {
        &self.expected_worktree_id
    }

    pub fn scope(&self) -> &NyxRemoteAgentScope {
        &self.scope
    }

    pub fn budget(&self) -> &NyxRemoteAgentBudget {
        &self.budget
    }

    pub const fn start_mode(&self) -> NyxRemoteAgentStartMode {
        self.start_mode
    }

    pub fn request_sha256(&self) -> Result<String, NyxRemoteAgentProtocolError> {
        raw_json_sha256(self)
    }

    fn validate(&self) -> Result<(), NyxRemoteAgentProtocolError> {
        ensure_schema(
            "remote-agent create",
            &self.schema_version,
            &self.schema_id,
            SCHEMA_RUN_CREATE,
        )?;
        for (field, value) in [
            ("idempotency_key", self.idempotency_key.as_str()),
            ("provider_id", self.provider_id.as_str()),
            ("model_id", self.model_id.as_str()),
            ("prompt", self.prompt.as_str()),
            (
                "expected_source_revision",
                self.expected_source_revision.as_str(),
            ),
            ("expected_worktree_id", self.expected_worktree_id.as_str()),
        ] {
            validate_text("remote-agent create", field, value)?;
        }
        self.scope.validate()?;
        self.budget.validate()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NyxRemoteAgentRunControl {
    schema_version: String,
    schema_id: String,
    task_id: String,
    request_sha256: String,
}

impl NyxRemoteAgentRunControl {
    pub fn for_record(record: &NyxRemoteAgentRunRecord) -> Self {
        Self {
            schema_version: SCHEMA_VERSION_V1.to_owned(),
            schema_id: SCHEMA_RUN_CONTROL.to_owned(),
            task_id: record.task_id.clone(),
            request_sha256: record.request_sha256.clone(),
        }
    }

    pub fn task_id(&self) -> &str {
        &self.task_id
    }

    pub fn request_sha256(&self) -> &str {
        &self.request_sha256
    }

    pub(crate) fn validate(&self) -> Result<(), NyxRemoteAgentProtocolError> {
        ensure_schema(
            "remote-agent control",
            &self.schema_version,
            &self.schema_id,
            SCHEMA_RUN_CONTROL,
        )?;
        validate_text("remote-agent control", "task_id", &self.task_id)?;
        validate_hash(
            "remote-agent control",
            "request_sha256",
            &self.request_sha256,
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NyxRemoteAgentRunRecord {
    schema_version: String,
    schema_id: String,
    task_id: String,
    run_id: String,
    accepted_operation_id: String,
    request_sha256: String,
    idempotency_key: String,
    request: NyxRemoteAgentRunCreate,
    state: NyxRemoteAgentRunState,
    created_at: String,
    updated_at: String,
    terminal_at: Option<String>,
    provider_id: String,
    model_id: String,
    route: Option<NyxRouteCandidate>,
    source: NyxRemoteAgentSourceBinding,
    budget: NyxRemoteAgentBudget,
    recorded_cost: NyxRouteCost,
    model_turns: u32,
    tool_steps: u32,
    output: Option<String>,
    error_code: Option<String>,
    error_message: Option<String>,
    cancellation_reason: Option<String>,
    continuation_allowed: bool,
    audit: Vec<String>,
}

impl NyxRemoteAgentRunRecord {
    pub fn task_id(&self) -> &str {
        &self.task_id
    }

    pub fn run_id(&self) -> &str {
        &self.run_id
    }

    pub fn accepted_operation_id(&self) -> &str {
        &self.accepted_operation_id
    }

    pub fn request_sha256(&self) -> &str {
        &self.request_sha256
    }

    pub fn request(&self) -> &NyxRemoteAgentRunCreate {
        &self.request
    }

    pub const fn state(&self) -> NyxRemoteAgentRunState {
        self.state
    }

    pub fn route(&self) -> Option<&NyxRouteCandidate> {
        self.route.as_ref()
    }

    pub fn source(&self) -> &NyxRemoteAgentSourceBinding {
        &self.source
    }

    pub fn budget(&self) -> &NyxRemoteAgentBudget {
        &self.budget
    }

    pub fn recorded_cost(&self) -> &NyxRouteCost {
        &self.recorded_cost
    }

    pub const fn model_turns(&self) -> u32 {
        self.model_turns
    }

    pub const fn tool_steps(&self) -> u32 {
        self.tool_steps
    }

    pub fn output(&self) -> Option<&str> {
        self.output.as_deref()
    }

    pub fn error_code(&self) -> Option<&str> {
        self.error_code.as_deref()
    }

    pub fn cancellation_reason(&self) -> Option<&str> {
        self.cancellation_reason.as_deref()
    }

    pub const fn continuation_allowed(&self) -> bool {
        self.continuation_allowed
    }

    pub fn audit(&self) -> &[String] {
        &self.audit
    }

    pub fn validate_against(
        &self,
        request: &NyxRemoteAgentRunCreate,
    ) -> Result<(), NyxRemoteAgentProtocolError> {
        self.validate()?;
        if &self.request != request {
            return Err(NyxRemoteAgentProtocolError::ImmutableMismatch { field: "request" });
        }
        Ok(())
    }

    pub(crate) fn validate(&self) -> Result<(), NyxRemoteAgentProtocolError> {
        ensure_schema(
            "remote-agent record",
            &self.schema_version,
            &self.schema_id,
            SCHEMA_RUN_RECORD,
        )?;
        for (field, value) in [
            ("task_id", self.task_id.as_str()),
            ("run_id", self.run_id.as_str()),
            ("accepted_operation_id", self.accepted_operation_id.as_str()),
            ("idempotency_key", self.idempotency_key.as_str()),
            ("provider_id", self.provider_id.as_str()),
            ("model_id", self.model_id.as_str()),
            ("created_at", self.created_at.as_str()),
            ("updated_at", self.updated_at.as_str()),
        ] {
            validate_text("remote-agent record", field, value)?;
        }
        validate_hash(
            "remote-agent record",
            "request_sha256",
            &self.request_sha256,
        )?;
        self.request.validate()?;
        self.source.validate()?;
        self.budget.validate()?;
        self.recorded_cost.validate()?;
        let Some(route) = &self.route else {
            return Err(NyxRemoteAgentProtocolError::InvalidField {
                context: "remote-agent record",
                field: "route",
                detail: "accepted run is missing Nyx route evidence".to_owned(),
            });
        };
        route.validate()?;
        verify_hash("request_sha256", &self.request_sha256, &self.request)?;
        let expected_task = format!("nyxtask_{}", &self.request_sha256[..24]);
        let expected_run = format!("nyxrun_{}", &self.request_sha256[24..48]);
        let expected_operation = format!("nyxop_{}", self.request_sha256);
        for (field, expected, actual) in [
            ("task_id", expected_task.as_str(), self.task_id.as_str()),
            ("run_id", expected_run.as_str(), self.run_id.as_str()),
            (
                "accepted_operation_id",
                expected_operation.as_str(),
                self.accepted_operation_id.as_str(),
            ),
        ] {
            if expected != actual {
                return Err(NyxRemoteAgentProtocolError::ImmutableMismatch { field });
            }
        }
        if self.request.idempotency_key != self.idempotency_key
            || self.request.provider_id != self.provider_id
            || self.request.model_id != self.model_id
        {
            return Err(NyxRemoteAgentProtocolError::ImmutableMismatch {
                field: "request attribution",
            });
        }
        if route.backend().backend_id() != self.provider_id
            || route.backend().provider() != self.provider_id
            || route.model().public_model_id() != self.model_id
        {
            return Err(NyxRemoteAgentProtocolError::ImmutableMismatch {
                field: "route provider or model attribution",
            });
        }
        if self.source.source_revision != self.request.expected_source_revision
            || self.source.worktree_id != self.request.expected_worktree_id
            || self.source.scope != self.request.scope
            || self.budget != self.request.budget
        {
            return Err(NyxRemoteAgentProtocolError::ImmutableMismatch {
                field: "source or budget binding",
            });
        }
        if self.model_turns > self.budget.max_model_turns
            || self.tool_steps > self.budget.max_tool_steps
        {
            return Err(NyxRemoteAgentProtocolError::InvalidField {
                context: "remote-agent record",
                field: "execution counters",
                detail: "exceed declared budget".to_owned(),
            });
        }
        for (field, actual, limit) in [
            (
                "recorded_cost.prompt_tokens",
                self.recorded_cost.prompt_tokens(),
                self.budget.max_input_tokens,
            ),
            (
                "recorded_cost.completion_tokens",
                self.recorded_cost.completion_tokens(),
                self.budget.max_output_tokens,
            ),
            (
                "recorded_cost.total_tokens",
                self.recorded_cost.total_tokens(),
                self.budget.max_total_tokens,
            ),
        ] {
            if actual.is_some_and(|value| value > limit) {
                return Err(NyxRemoteAgentProtocolError::InvalidField {
                    context: "remote-agent record",
                    field,
                    detail: "exceeds declared token budget".to_owned(),
                });
            }
        }
        if let (Some(limit), Some(actual)) = (
            self.budget.max_monetary_microunits,
            self.recorded_cost.monetary_microunits(),
        ) && actual > limit
        {
            return Err(NyxRemoteAgentProtocolError::InvalidField {
                context: "remote-agent record",
                field: "recorded_cost.monetary_microunits",
                detail: "exceeds declared monetary budget".to_owned(),
            });
        }
        self.validate_state_and_audit()
    }

    fn validate_state_and_audit(&self) -> Result<(), NyxRemoteAgentProtocolError> {
        let audit_tail = self.audit.last().map(String::as_str);
        let tail_matches = audit_tail == Some(self.state.audit_tail())
            || (self.state == NyxRemoteAgentRunState::BudgetHit
                && audit_tail == Some("budget_hit_preflight"));
        if self.audit.is_empty()
            || self.audit.iter().any(|entry| entry.is_empty() || entry.trim() != entry)
            || self.audit.first().map(String::as_str) != Some("accepted")
            || !tail_matches
        {
            return Err(NyxRemoteAgentProtocolError::InvalidField {
                context: "remote-agent record",
                field: "audit",
                detail: "does not match the accepted run state transition history".to_owned(),
            });
        }
        if self.state.is_terminal() {
            if self.terminal_at.as_deref().is_none_or(|value| value.trim().is_empty())
                || self.continuation_allowed
            {
                return Err(NyxRemoteAgentProtocolError::InvalidField {
                    context: "remote-agent record",
                    field: "terminal state",
                    detail: "must include terminal_at and forbid continuation".to_owned(),
                });
            }
        } else if self.terminal_at.is_some() || !self.continuation_allowed {
            return Err(NyxRemoteAgentProtocolError::InvalidField {
                context: "remote-agent record",
                field: "nonterminal state",
                detail: "must omit terminal_at and allow continuation".to_owned(),
            });
        }
        match self.state {
            NyxRemoteAgentRunState::Completed if self.output.is_none() => {
                Err(NyxRemoteAgentProtocolError::InvalidField {
                    context: "remote-agent record",
                    field: "output",
                    detail: "completed run is missing output".to_owned(),
                })
            }
            NyxRemoteAgentRunState::Failed
                if self.error_code.as_deref().is_none_or(str::is_empty)
                    || self.error_message.as_deref().is_none_or(str::is_empty) =>
            {
                Err(NyxRemoteAgentProtocolError::InvalidField {
                    context: "remote-agent record",
                    field: "failure",
                    detail: "failed run is missing error code or message".to_owned(),
                })
            }
            NyxRemoteAgentRunState::Cancelled
                if self
                    .cancellation_reason
                    .as_deref()
                    .is_none_or(str::is_empty) =>
            {
                Err(NyxRemoteAgentProtocolError::InvalidField {
                    context: "remote-agent record",
                    field: "cancellation_reason",
                    detail: "cancelled run is missing a reason".to_owned(),
                })
            }
            NyxRemoteAgentRunState::BudgetHit
                if self.error_code.as_deref() != Some("budget_hit") =>
            {
                Err(NyxRemoteAgentProtocolError::InvalidField {
                    context: "remote-agent record",
                    field: "error_code",
                    detail: "budget-hit run must carry budget_hit".to_owned(),
                })
            }
            _ => Ok(()),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NyxRemoteAgentRunList {
    schema_version: String,
    schema_id: String,
    runs: Vec<NyxRemoteAgentRunRecord>,
}

impl NyxRemoteAgentRunList {
    pub fn runs(&self) -> &[NyxRemoteAgentRunRecord] {
        &self.runs
    }

    pub(crate) fn validate(&self) -> Result<(), NyxRemoteAgentProtocolError> {
        ensure_schema(
            "remote-agent run list",
            &self.schema_version,
            &self.schema_id,
            SCHEMA_RUN_LIST,
        )?;
        for run in &self.runs {
            run.validate()?;
        }
        if self
            .runs
            .windows(2)
            .any(|pair| pair[0].run_id >= pair[1].run_id)
        {
            return Err(NyxRemoteAgentProtocolError::NonCanonical {
                context: "run list ordering",
            });
        }
        Ok(())
    }
}
