use super::support::{
    ensure_schema, validate_canonical_strings, validate_text, NyxRemoteAgentProtocolError,
};
use serde::{Deserialize, Serialize};

const SCHEMA_BACKEND_IDENTITY: &str = "nyx.backend_identity.v1";
const SCHEMA_BACKEND_CAPABILITIES: &str = "nyx.backend_capabilities.v1";
const SCHEMA_MODEL_IDENTITY: &str = "nyx.routable_model_identity.v1";
const SCHEMA_ROUTE_COST: &str = "nyx.route_cost.v1";
const SCHEMA_ROUTE_CANDIDATE: &str = "nyx.route_candidate.v1";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NyxBackendIdentity {
    schema_version: String,
    schema_id: String,
    backend_id: String,
    kind: String,
    provider: String,
    transport: String,
    endpoint_posture: String,
    configuration_version: String,
}

impl NyxBackendIdentity {
    pub fn backend_id(&self) -> &str {
        &self.backend_id
    }

    pub fn provider(&self) -> &str {
        &self.provider
    }

    pub fn kind(&self) -> &str {
        &self.kind
    }

    pub fn transport(&self) -> &str {
        &self.transport
    }

    pub fn endpoint_posture(&self) -> &str {
        &self.endpoint_posture
    }

    pub fn configuration_version(&self) -> &str {
        &self.configuration_version
    }

    pub(crate) fn validate(&self) -> Result<(), NyxRemoteAgentProtocolError> {
        ensure_schema(
            "backend identity",
            &self.schema_version,
            &self.schema_id,
            SCHEMA_BACKEND_IDENTITY,
        )?;
        for (field, value) in [
            ("backend_id", self.backend_id.as_str()),
            ("kind", self.kind.as_str()),
            ("provider", self.provider.as_str()),
            ("transport", self.transport.as_str()),
            ("endpoint_posture", self.endpoint_posture.as_str()),
            ("configuration_version", self.configuration_version.as_str()),
        ] {
            validate_text("backend identity", field, value)?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NyxBackendCapabilities {
    schema_version: String,
    schema_id: String,
    max_context_tokens: Option<u32>,
    modalities: Vec<String>,
    streaming: bool,
    tool_calling: bool,
    json_mode: bool,
    embedding: bool,
    max_concurrency: Option<u32>,
    deterministic: bool,
}

impl NyxBackendCapabilities {
    pub const fn max_context_tokens(&self) -> Option<u32> {
        self.max_context_tokens
    }

    pub fn modalities(&self) -> &[String] {
        &self.modalities
    }

    pub const fn streaming(&self) -> bool {
        self.streaming
    }

    pub const fn tool_calling(&self) -> bool {
        self.tool_calling
    }

    pub const fn json_mode(&self) -> bool {
        self.json_mode
    }

    pub const fn embedding(&self) -> bool {
        self.embedding
    }

    pub const fn max_concurrency(&self) -> Option<u32> {
        self.max_concurrency
    }

    pub const fn deterministic(&self) -> bool {
        self.deterministic
    }

    pub(crate) fn validate(&self) -> Result<(), NyxRemoteAgentProtocolError> {
        ensure_schema(
            "backend capabilities",
            &self.schema_version,
            &self.schema_id,
            SCHEMA_BACKEND_CAPABILITIES,
        )?;
        validate_canonical_strings(
            "backend capabilities modalities",
            "modalities",
            &self.modalities,
            false,
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NyxRoutableModelIdentity {
    schema_version: String,
    schema_id: String,
    public_model_id: String,
    backend_local_name: String,
    backend_id: String,
    runtime_instance_id: Option<String>,
}

impl NyxRoutableModelIdentity {
    pub fn public_model_id(&self) -> &str {
        &self.public_model_id
    }

    pub fn backend_id(&self) -> &str {
        &self.backend_id
    }

    pub fn backend_local_name(&self) -> &str {
        &self.backend_local_name
    }

    pub fn runtime_instance_id(&self) -> Option<&str> {
        self.runtime_instance_id.as_deref()
    }

    pub(crate) fn validate(&self) -> Result<(), NyxRemoteAgentProtocolError> {
        ensure_schema(
            "routable model identity",
            &self.schema_version,
            &self.schema_id,
            SCHEMA_MODEL_IDENTITY,
        )?;
        for (field, value) in [
            ("public_model_id", self.public_model_id.as_str()),
            ("backend_local_name", self.backend_local_name.as_str()),
            ("backend_id", self.backend_id.as_str()),
        ] {
            validate_text("routable model identity", field, value)?;
        }
        if let Some(value) = &self.runtime_instance_id {
            validate_text("routable model identity", "runtime_instance_id", value)?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NyxRouteCost {
    schema_version: String,
    schema_id: String,
    source: String,
    currency: Option<String>,
    monetary_microunits: Option<u64>,
    prompt_tokens: Option<u32>,
    completion_tokens: Option<u32>,
    total_tokens: Option<u32>,
    energy_millijoules: Option<u64>,
    memory_bytes: Option<u64>,
    device_use: Vec<String>,
}

impl NyxRouteCost {
    pub fn source(&self) -> &str {
        &self.source
    }

    pub const fn monetary_microunits(&self) -> Option<u64> {
        self.monetary_microunits
    }

    pub const fn prompt_tokens(&self) -> Option<u32> {
        self.prompt_tokens
    }

    pub const fn completion_tokens(&self) -> Option<u32> {
        self.completion_tokens
    }

    pub const fn total_tokens(&self) -> Option<u32> {
        self.total_tokens
    }

    pub fn currency(&self) -> Option<&str> {
        self.currency.as_deref()
    }

    pub const fn energy_millijoules(&self) -> Option<u64> {
        self.energy_millijoules
    }

    pub const fn memory_bytes(&self) -> Option<u64> {
        self.memory_bytes
    }

    pub fn device_use(&self) -> &[String] {
        &self.device_use
    }

    pub(crate) fn validate(&self) -> Result<(), NyxRemoteAgentProtocolError> {
        ensure_schema(
            "route cost",
            &self.schema_version,
            &self.schema_id,
            SCHEMA_ROUTE_COST,
        )?;
        validate_text("route cost", "source", &self.source)?;
        if let Some(currency) = &self.currency {
            validate_text("route cost", "currency", currency)?;
        }
        validate_canonical_strings(
            "route cost device use",
            "device_use",
            &self.device_use,
            true,
        )?;
        if let (Some(prompt), Some(completion), Some(total)) = (
            self.prompt_tokens,
            self.completion_tokens,
            self.total_tokens,
        ) && prompt.saturating_add(completion) != total
        {
            return Err(NyxRemoteAgentProtocolError::InvalidField {
                context: "route cost",
                field: "total_tokens",
                detail: "does not equal prompt_tokens + completion_tokens".to_owned(),
            });
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NyxRouteCandidate {
    schema_version: String,
    schema_id: String,
    backend: NyxBackendIdentity,
    model: NyxRoutableModelIdentity,
    #[serde(default)]
    runtime_profile_id: Option<String>,
    capabilities: NyxBackendCapabilities,
    health: String,
    policy_posture: String,
    measured_signals: Vec<String>,
    declared_cost: NyxRouteCost,
}

impl NyxRouteCandidate {
    pub fn backend(&self) -> &NyxBackendIdentity {
        &self.backend
    }

    pub fn model(&self) -> &NyxRoutableModelIdentity {
        &self.model
    }

    pub fn health(&self) -> &str {
        &self.health
    }

    pub fn policy_posture(&self) -> &str {
        &self.policy_posture
    }

    pub fn runtime_profile_id(&self) -> Option<&str> {
        self.runtime_profile_id.as_deref()
    }

    pub fn capabilities(&self) -> &NyxBackendCapabilities {
        &self.capabilities
    }

    pub fn measured_signals(&self) -> &[String] {
        &self.measured_signals
    }

    pub fn declared_cost(&self) -> &NyxRouteCost {
        &self.declared_cost
    }

    pub(crate) fn validate(&self) -> Result<(), NyxRemoteAgentProtocolError> {
        ensure_schema(
            "route candidate",
            &self.schema_version,
            &self.schema_id,
            SCHEMA_ROUTE_CANDIDATE,
        )?;
        self.backend.validate()?;
        self.model.validate()?;
        self.capabilities.validate()?;
        self.declared_cost.validate()?;
        validate_text("route candidate", "health", &self.health)?;
        validate_text(
            "route candidate",
            "policy_posture",
            &self.policy_posture,
        )?;
        validate_canonical_strings(
            "route candidate measured signals",
            "measured_signals",
            &self.measured_signals,
            true,
        )?;
        if let Some(value) = &self.runtime_profile_id {
            validate_text("route candidate", "runtime_profile_id", value)?;
        }
        if self.backend.backend_id() != self.model.backend_id() {
            return Err(NyxRemoteAgentProtocolError::ImmutableMismatch {
                field: "route backend/model identity",
            });
        }
        Ok(())
    }
}
