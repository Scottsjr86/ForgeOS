//! Client-side representation and independent verification of Nyx permission contracts.
//!
//! Nyx_Server owns checkpoint creation, decisions, resume tokens, persistence, and
//! execution. ForgeOS constructs scoped requests, displays Nyx-owned state, and
//! independently reconciles the immutable identities returned over the wire.

use crate::canonical_json::canonical_value;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::{Component, Path};

mod audit;
mod decision;
mod support;

pub use audit::{NyxPermissionAuditEvent, NyxPermissionAuditEventKind, NyxPermissionAuditReport};
pub use decision::{
    NyxPermissionDecision, NyxPermissionDecisionKind, NyxPermissionResolution, NyxPermissionResume,
    NyxPermissionResumeResult,
};
pub(crate) use support::decode_permission;
pub use support::NyxPermissionProtocolError;

use support::{
    canonical_strings, ensure_schema, hash_json, validate_canonical_strings, validate_hash,
    validate_route_id, validate_text, verify_hash,
};

pub const PERMISSION_CHECKPOINTS_PATH: &str = "/v1/nyx/permissions/checkpoints";
pub const PERMISSION_RESUME_PATH: &str = "/v1/nyx/permissions/resume";
pub const PERMISSION_AUDIT_PATH: &str = "/v1/nyx/permissions/audit";
pub const PERMISSION_CHECKPOINT_PATH_LABEL: &str = "/v1/nyx/permissions/checkpoints/:checkpoint_id";
pub const PERMISSION_RESOLVE_PATH_LABEL: &str =
    "/v1/nyx/permissions/checkpoints/:checkpoint_id/resolve";

const SCHEMA_VERSION_V1: &str = "nyx.1.0";
const SCHEMA_SCOPE: &str = "nyx.permission_scope.v1";
const SCHEMA_REQUEST: &str = "nyx.scoped_tool_request.v1";
const SCHEMA_CHECKPOINT_CREATE: &str = "nyx.permission_checkpoint_create.v1";
const SCHEMA_CHECKPOINT: &str = "nyx.permission_checkpoint.v1";
const SCHEMA_DECISION: &str = "nyx.permission_decision.v1";
const SCHEMA_RESOLVE: &str = "nyx.permission_resolve.v1";
const SCHEMA_RESUME: &str = "nyx.permission_resume.v1";
const SCHEMA_RESUME_RESULT: &str = "nyx.permission_resume_result.v1";
const SCHEMA_AUDIT_EVENT: &str = "nyx.permission_audit_event.v1";
const SCHEMA_AUDIT_REPORT: &str = "nyx.permission_audit_report.v1";
const SCHEMA_POLICY_ACTOR: &str = "nyx.policy_actor.v1";
const SCHEMA_POLICY_DECISION: &str = "nyx.policy_decision.v1";
const OPERATION_TOOL_EXECUTE: &str = "tool.execute";
const MAX_PERMISSION_TTL_MS: u64 = 86_400_000;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NyxPermissionScope {
    schema_version: String,
    schema_id: String,
    workspace_root: String,
    #[serde(default)]
    allowed_paths: Vec<String>,
    #[serde(default)]
    network_origins: Vec<String>,
    #[serde(default)]
    declared_side_effects: Vec<String>,
}

impl NyxPermissionScope {
    pub fn new(workspace_root: impl Into<String>) -> Result<Self, NyxPermissionProtocolError> {
        let scope = Self {
            schema_version: SCHEMA_VERSION_V1.to_owned(),
            schema_id: SCHEMA_SCOPE.to_owned(),
            workspace_root: workspace_root.into(),
            allowed_paths: Vec::new(),
            network_origins: Vec::new(),
            declared_side_effects: Vec::new(),
        };
        scope.validate()?;
        Ok(scope)
    }

    pub fn with_allowed_paths<I, S>(mut self, paths: I) -> Result<Self, NyxPermissionProtocolError>
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.allowed_paths = canonical_strings(paths);
        self.validate()?;
        Ok(self)
    }

    pub fn with_network_origins<I, S>(
        mut self,
        origins: I,
    ) -> Result<Self, NyxPermissionProtocolError>
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.network_origins = canonical_strings(origins);
        self.validate()?;
        Ok(self)
    }

    pub fn with_declared_side_effects<I, S>(
        mut self,
        effects: I,
    ) -> Result<Self, NyxPermissionProtocolError>
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.declared_side_effects = canonical_strings(effects);
        self.validate()?;
        Ok(self)
    }

    pub fn workspace_root(&self) -> &str {
        &self.workspace_root
    }

    pub fn allowed_paths(&self) -> &[String] {
        &self.allowed_paths
    }

    pub fn network_origins(&self) -> &[String] {
        &self.network_origins
    }

    pub fn declared_side_effects(&self) -> &[String] {
        &self.declared_side_effects
    }

    fn validate(&self) -> Result<(), NyxPermissionProtocolError> {
        ensure_schema(
            "permission scope",
            &self.schema_version,
            &self.schema_id,
            SCHEMA_SCOPE,
        )?;
        validate_text("permission scope", "workspace_root", &self.workspace_root)?;
        validate_canonical_strings("permission scope", "allowed_paths", &self.allowed_paths)?;
        validate_canonical_strings("permission scope", "network_origins", &self.network_origins)?;
        validate_canonical_strings(
            "permission scope",
            "declared_side_effects",
            &self.declared_side_effects,
        )?;
        for path in &self.allowed_paths {
            let candidate = Path::new(path);
            if candidate.is_absolute()
                || candidate.components().any(|component| {
                    matches!(
                        component,
                        Component::ParentDir | Component::RootDir | Component::Prefix(_)
                    )
                })
            {
                return Err(NyxPermissionProtocolError::InvalidField {
                    context: "permission scope",
                    field: "allowed_paths",
                    detail: format!("path '{path}' is absolute or traversing"),
                });
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NyxPolicyActorKind {
    User,
    Service,
    Agent,
    Persona,
    Workflow,
    Plugin,
    DelegatedRole,
    LocalOperator,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NyxPolicyActor {
    schema_version: String,
    schema_id: String,
    actor_id: String,
    kind: NyxPolicyActorKind,
    attribution_method: String,
    delegated_by: Option<String>,
    roles: Vec<String>,
}

impl NyxPolicyActor {
    pub fn local_anonymous() -> Self {
        Self {
            schema_version: SCHEMA_VERSION_V1.to_owned(),
            schema_id: SCHEMA_POLICY_ACTOR.to_owned(),
            actor_id: "local_anonymous".to_owned(),
            kind: NyxPolicyActorKind::LocalOperator,
            attribution_method: "process_local_no_credential".to_owned(),
            delegated_by: None,
            roles: vec!["local_operator".to_owned()],
        }
    }

    pub fn actor_id(&self) -> &str {
        &self.actor_id
    }

    fn validate(&self) -> Result<(), NyxPermissionProtocolError> {
        ensure_schema(
            "policy actor",
            &self.schema_version,
            &self.schema_id,
            SCHEMA_POLICY_ACTOR,
        )?;
        validate_text("policy actor", "actor_id", &self.actor_id)?;
        validate_text(
            "policy actor",
            "attribution_method",
            &self.attribution_method,
        )?;
        if let Some(delegated_by) = &self.delegated_by {
            validate_text("policy actor", "delegated_by", delegated_by)?;
        }
        validate_canonical_strings("policy actor", "roles", &self.roles)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NyxScopedToolRequest {
    schema_version: String,
    schema_id: String,
    request_id: String,
    session_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    thread_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    run_id: Option<String>,
    tool_call_id: String,
    operation_class: String,
    tool_name: String,
    arguments: Value,
    scope: NyxPermissionScope,
    actor: NyxPolicyActor,
    idempotency_key: String,
}

impl NyxScopedToolRequest {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        request_id: impl Into<String>,
        session_id: impl Into<String>,
        tool_call_id: impl Into<String>,
        tool_name: impl Into<String>,
        arguments: Value,
        scope: NyxPermissionScope,
        idempotency_key: impl Into<String>,
    ) -> Result<Self, NyxPermissionProtocolError> {
        let request = Self {
            schema_version: SCHEMA_VERSION_V1.to_owned(),
            schema_id: SCHEMA_REQUEST.to_owned(),
            request_id: request_id.into(),
            session_id: session_id.into(),
            thread_id: None,
            run_id: None,
            tool_call_id: tool_call_id.into(),
            operation_class: OPERATION_TOOL_EXECUTE.to_owned(),
            tool_name: tool_name.into(),
            arguments: canonical_value(&arguments),
            scope,
            actor: NyxPolicyActor::local_anonymous(),
            idempotency_key: idempotency_key.into(),
        };
        request.validate()?;
        Ok(request)
    }

    pub fn with_thread(
        mut self,
        thread_id: impl Into<String>,
    ) -> Result<Self, NyxPermissionProtocolError> {
        self.thread_id = Some(thread_id.into());
        self.validate()?;
        Ok(self)
    }

    pub fn with_run(
        mut self,
        run_id: impl Into<String>,
    ) -> Result<Self, NyxPermissionProtocolError> {
        self.run_id = Some(run_id.into());
        self.validate()?;
        Ok(self)
    }

    pub fn request_id(&self) -> &str {
        &self.request_id
    }

    pub fn session_id(&self) -> &str {
        &self.session_id
    }

    pub fn thread_id(&self) -> Option<&str> {
        self.thread_id.as_deref()
    }

    pub fn run_id(&self) -> Option<&str> {
        self.run_id.as_deref()
    }

    pub fn tool_call_id(&self) -> &str {
        &self.tool_call_id
    }

    pub fn tool_name(&self) -> &str {
        &self.tool_name
    }

    pub fn arguments(&self) -> &Value {
        &self.arguments
    }

    pub fn scope(&self) -> &NyxPermissionScope {
        &self.scope
    }

    pub fn actor(&self) -> &NyxPolicyActor {
        &self.actor
    }

    pub fn idempotency_key(&self) -> &str {
        &self.idempotency_key
    }

    pub fn request_sha256(&self) -> Result<String, NyxPermissionProtocolError> {
        self.validate()?;
        hash_json("scoped tool request", self)
    }

    pub fn payload_sha256(&self) -> Result<String, NyxPermissionProtocolError> {
        hash_json("tool arguments", &self.arguments)
    }

    pub fn scope_sha256(&self) -> Result<String, NyxPermissionProtocolError> {
        self.scope.validate()?;
        hash_json("permission scope", &self.scope)
    }

    fn validate(&self) -> Result<(), NyxPermissionProtocolError> {
        ensure_schema(
            "scoped tool request",
            &self.schema_version,
            &self.schema_id,
            SCHEMA_REQUEST,
        )?;
        for (field, value) in [
            ("request_id", self.request_id.as_str()),
            ("session_id", self.session_id.as_str()),
            ("tool_call_id", self.tool_call_id.as_str()),
            ("operation_class", self.operation_class.as_str()),
            ("tool_name", self.tool_name.as_str()),
            ("idempotency_key", self.idempotency_key.as_str()),
        ] {
            validate_text("scoped tool request", field, value)?;
        }
        if self.operation_class != OPERATION_TOOL_EXECUTE {
            return Err(NyxPermissionProtocolError::InvalidField {
                context: "scoped tool request",
                field: "operation_class",
                detail: format!("expected '{OPERATION_TOOL_EXECUTE}'"),
            });
        }
        if !self.arguments.is_object() {
            return Err(NyxPermissionProtocolError::InvalidField {
                context: "scoped tool request",
                field: "arguments",
                detail: "must be a JSON object".to_owned(),
            });
        }
        if let Some(thread_id) = &self.thread_id {
            validate_text("scoped tool request", "thread_id", thread_id)?;
        }
        if let Some(run_id) = &self.run_id {
            validate_text("scoped tool request", "run_id", run_id)?;
            if self.thread_id.is_none() {
                return Err(NyxPermissionProtocolError::InvalidField {
                    context: "scoped tool request",
                    field: "run_id",
                    detail: "requires thread_id".to_owned(),
                });
            }
        }
        if canonical_value(&self.arguments) != self.arguments {
            return Err(NyxPermissionProtocolError::NonCanonical {
                context: "scoped tool request arguments",
            });
        }
        self.scope.validate()?;
        self.actor.validate()
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NyxPermissionCheckpointCreate {
    schema_version: String,
    schema_id: String,
    request: NyxScopedToolRequest,
    ttl_ms: u64,
}

impl NyxPermissionCheckpointCreate {
    pub fn new(
        request: NyxScopedToolRequest,
        ttl_ms: u64,
    ) -> Result<Self, NyxPermissionProtocolError> {
        let create = Self {
            schema_version: SCHEMA_VERSION_V1.to_owned(),
            schema_id: SCHEMA_CHECKPOINT_CREATE.to_owned(),
            request,
            ttl_ms,
        };
        create.validate()?;
        Ok(create)
    }

    pub fn request(&self) -> &NyxScopedToolRequest {
        &self.request
    }

    pub const fn ttl_ms(&self) -> u64 {
        self.ttl_ms
    }

    fn validate(&self) -> Result<(), NyxPermissionProtocolError> {
        ensure_schema(
            "checkpoint creation",
            &self.schema_version,
            &self.schema_id,
            SCHEMA_CHECKPOINT_CREATE,
        )?;
        if self.ttl_ms == 0 || self.ttl_ms > MAX_PERMISSION_TTL_MS {
            return Err(NyxPermissionProtocolError::InvalidField {
                context: "checkpoint creation",
                field: "ttl_ms",
                detail: format!("must be between 1 and {MAX_PERMISSION_TTL_MS}"),
            });
        }
        self.request.validate()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NyxPermissionCheckpointStatus {
    Pending,
    Approved,
    Denied,
    Expired,
    Executing,
    Consumed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NyxToolPolicyDecisionKind {
    Allow,
    Deny,
    Checkpoint,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NyxToolPolicyDecision {
    schema_version: String,
    schema_id: String,
    kind: NyxToolPolicyDecisionKind,
    policy_snapshot_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    reason: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    tags: Vec<String>,
}

impl NyxToolPolicyDecision {
    pub const fn kind(&self) -> NyxToolPolicyDecisionKind {
        self.kind
    }

    pub fn policy_snapshot_id(&self) -> &str {
        &self.policy_snapshot_id
    }

    pub fn tags(&self) -> &[String] {
        &self.tags
    }

    fn validate(&self) -> Result<(), NyxPermissionProtocolError> {
        ensure_schema(
            "policy decision",
            &self.schema_version,
            &self.schema_id,
            SCHEMA_POLICY_DECISION,
        )?;
        validate_text(
            "policy decision",
            "policy_snapshot_id",
            &self.policy_snapshot_id,
        )?;
        validate_canonical_strings("policy decision", "tags", &self.tags)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NyxPermissionCheckpoint {
    schema_version: String,
    schema_id: String,
    checkpoint_id: String,
    request: NyxScopedToolRequest,
    accepted_operation_id: String,
    request_sha256: String,
    payload_sha256: String,
    scope_sha256: String,
    policy_decision: NyxToolPolicyDecision,
    policy_decision_sha256: String,
    predicted_effects: Vec<String>,
    predicted_effects_sha256: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    approval_conditions: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    approval_conditions_sha256: Option<String>,
    created_at_unix_ms: u64,
    expires_at_unix_ms: u64,
    status: NyxPermissionCheckpointStatus,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    decided_at_unix_ms: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    decided_by: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    decision_reason: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    resume_token_sha256: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    execution_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    consumed_at_unix_ms: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    result_sha256: Option<String>,
    audit_sequence: u64,
}

impl NyxPermissionCheckpoint {
    pub fn checkpoint_id(&self) -> &str {
        &self.checkpoint_id
    }

    pub fn request(&self) -> &NyxScopedToolRequest {
        &self.request
    }

    pub fn accepted_operation_id(&self) -> &str {
        &self.accepted_operation_id
    }

    pub fn request_sha256(&self) -> &str {
        &self.request_sha256
    }

    pub fn payload_sha256(&self) -> &str {
        &self.payload_sha256
    }

    pub fn scope_sha256(&self) -> &str {
        &self.scope_sha256
    }

    pub fn policy_decision_sha256(&self) -> &str {
        &self.policy_decision_sha256
    }

    pub const fn status(&self) -> NyxPermissionCheckpointStatus {
        self.status
    }

    pub fn decided_by(&self) -> Option<&str> {
        self.decided_by.as_deref()
    }

    pub fn approval_conditions(&self) -> &[String] {
        &self.approval_conditions
    }

    pub fn resume_token_sha256(&self) -> Option<&str> {
        self.resume_token_sha256.as_deref()
    }

    pub fn execution_id(&self) -> Option<&str> {
        self.execution_id.as_deref()
    }

    pub const fn audit_sequence(&self) -> u64 {
        self.audit_sequence
    }

    pub fn validate(&self) -> Result<(), NyxPermissionProtocolError> {
        ensure_schema(
            "permission checkpoint",
            &self.schema_version,
            &self.schema_id,
            SCHEMA_CHECKPOINT,
        )?;
        validate_route_id("checkpoint_id", &self.checkpoint_id)?;
        validate_text(
            "permission checkpoint",
            "accepted_operation_id",
            &self.accepted_operation_id,
        )?;
        for (field, value) in [
            ("request_sha256", self.request_sha256.as_str()),
            ("payload_sha256", self.payload_sha256.as_str()),
            ("scope_sha256", self.scope_sha256.as_str()),
            (
                "policy_decision_sha256",
                self.policy_decision_sha256.as_str(),
            ),
            (
                "predicted_effects_sha256",
                self.predicted_effects_sha256.as_str(),
            ),
        ] {
            validate_hash("permission checkpoint", field, value)?;
        }
        if let Some(value) = &self.approval_conditions_sha256 {
            validate_hash("permission checkpoint", "approval_conditions_sha256", value)?;
        }
        if let Some(value) = &self.resume_token_sha256 {
            validate_hash("permission checkpoint", "resume_token_sha256", value)?;
        }
        if let Some(value) = &self.result_sha256 {
            validate_hash("permission checkpoint", "result_sha256", value)?;
        }
        if self.expires_at_unix_ms < self.created_at_unix_ms {
            return Err(NyxPermissionProtocolError::InvalidField {
                context: "permission checkpoint",
                field: "expires_at_unix_ms",
                detail: "precedes creation".to_owned(),
            });
        }
        self.request.validate()?;
        self.policy_decision.validate()?;
        if self.policy_decision.kind != NyxToolPolicyDecisionKind::Checkpoint {
            return Err(NyxPermissionProtocolError::InvalidField {
                context: "permission checkpoint",
                field: "policy_decision.kind",
                detail: "must be checkpoint".to_owned(),
            });
        }
        validate_canonical_strings(
            "permission checkpoint",
            "predicted_effects",
            &self.predicted_effects,
        )?;
        validate_canonical_strings(
            "permission checkpoint",
            "approval_conditions",
            &self.approval_conditions,
        )?;

        verify_hash(
            "request_sha256",
            &self.request.request_sha256()?,
            &self.request_sha256,
        )?;
        verify_hash(
            "payload_sha256",
            &self.request.payload_sha256()?,
            &self.payload_sha256,
        )?;
        verify_hash(
            "scope_sha256",
            &self.request.scope_sha256()?,
            &self.scope_sha256,
        )?;
        verify_hash(
            "policy_decision_sha256",
            &hash_json("policy decision", &self.policy_decision)?,
            &self.policy_decision_sha256,
        )?;
        verify_hash(
            "predicted_effects_sha256",
            &hash_json("predicted effects", &self.predicted_effects)?,
            &self.predicted_effects_sha256,
        )?;
        if self.predicted_effects != self.request.scope.declared_side_effects {
            return Err(NyxPermissionProtocolError::ImmutableMismatch {
                field: "predicted_effects",
            });
        }
        if self.accepted_operation_id != format!("nyxop_{}", self.request_sha256) {
            return Err(NyxPermissionProtocolError::ImmutableMismatch {
                field: "accepted_operation_id",
            });
        }
        match &self.approval_conditions_sha256 {
            Some(declared) => verify_hash(
                "approval_conditions_sha256",
                &hash_json("approval conditions", &self.approval_conditions)?,
                declared,
            )?,
            None if !self.approval_conditions.is_empty() => {
                return Err(NyxPermissionProtocolError::ImmutableMismatch {
                    field: "approval_conditions_sha256",
                });
            }
            None => {}
        }
        self.validate_status_fields()
    }

    fn validate_status_fields(&self) -> Result<(), NyxPermissionProtocolError> {
        let decided = self.decided_at_unix_ms.is_some() && self.decided_by.is_some();
        let token = self.resume_token_sha256.is_some();
        let execution = self.execution_id.is_some();
        let consumed = self.consumed_at_unix_ms.is_some();
        let result = self.result_sha256.is_some();
        let conditions = self.approval_conditions_sha256.is_some();
        let valid = match self.status {
            NyxPermissionCheckpointStatus::Pending => {
                !decided
                    && !token
                    && !execution
                    && !consumed
                    && !result
                    && !conditions
                    && self.approval_conditions.is_empty()
            }
            NyxPermissionCheckpointStatus::Approved => {
                decided && token && !execution && !consumed && !result && conditions
            }
            NyxPermissionCheckpointStatus::Denied => {
                decided && !token && !execution && !consumed && !result && conditions
            }
            NyxPermissionCheckpointStatus::Expired => !execution && !consumed && !result,
            NyxPermissionCheckpointStatus::Executing => {
                decided && token && execution && !consumed && !result && conditions
            }
            NyxPermissionCheckpointStatus::Consumed => {
                decided && token && execution && consumed && result && conditions
            }
        };
        if !valid {
            return Err(NyxPermissionProtocolError::InvalidStatusFields(self.status));
        }
        if let Some(actor) = &self.decided_by {
            validate_text("permission checkpoint", "decided_by", actor)?;
        }
        if let Some(execution_id) = &self.execution_id {
            validate_text("permission checkpoint", "execution_id", execution_id)?;
        }
        Ok(())
    }
}
