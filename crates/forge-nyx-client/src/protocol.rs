//! Client-side decoding for the Nyx-owned public HTTP contract.
//!
//! Nyx_Server owns the canonical schemas. ForgeOS validates the schema IDs and
//! extracts only the health, compatibility, capability, engine, and provider
//! fields needed by the local integration surface.

use serde_json::{Map, Value};
use std::collections::BTreeSet;
use std::fmt;

pub const HEADER_NYX_CONTRACT_VERSION: &str = "x-nyx-contract-version";
pub const HEALTH_PATH: &str = "/v1/nyx/health";
pub const VERSION_PATH: &str = "/v1/nyx/version";
pub const CAPABILITIES_PATH: &str = "/v1/nyx/capabilities";

const SCHEMA_VERSION_V1: &str = "nyx.1.0";
const SCHEMA_ID_VERSION: &str = "nyx.api_version_manifest.v1";
const SCHEMA_ID_HEALTH: &str = "nyx.server_health.v1";
const SCHEMA_ID_CAPABILITIES: &str = "nyx.api_capability_manifest.v1";
const SCHEMA_ID_CAPABILITY: &str = "nyx.api_capability_descriptor.v1";
const SCHEMA_ID_ERROR: &str = "nyx.api_error_envelope.v1";
const INCOMPATIBLE_MAJOR_CODE: &str = "incompatible_api_major_version";

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NyxProtocolVersion {
    major: u16,
    minor: u16,
}

impl NyxProtocolVersion {
    pub const fn new(major: u16, minor: u16) -> Self {
        Self { major, minor }
    }

    pub const fn major(self) -> u16 {
        self.major
    }

    pub const fn minor(self) -> u16 {
        self.minor
    }

    pub fn parse(value: &str) -> Result<Self, NyxProtocolError> {
        let mut parts = value.trim().split('.');
        let major = parts
            .next()
            .ok_or_else(|| NyxProtocolError::InvalidVersion(value.to_owned()))?
            .parse::<u16>()
            .map_err(|_| NyxProtocolError::InvalidVersion(value.to_owned()))?;
        let minor = parts
            .next()
            .ok_or_else(|| NyxProtocolError::InvalidVersion(value.to_owned()))?
            .parse::<u16>()
            .map_err(|_| NyxProtocolError::InvalidVersion(value.to_owned()))?;
        if parts.next().is_some() {
            return Err(NyxProtocolError::InvalidVersion(value.to_owned()));
        }
        Ok(Self { major, minor })
    }
}

impl fmt::Display for NyxProtocolVersion {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}.{}", self.major, self.minor)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NyxHealth {
    Healthy,
    Degraded,
    Unavailable,
}

impl NyxHealth {
    fn parse(value: &str) -> Result<Self, NyxProtocolError> {
        match value {
            "ready" => Ok(Self::Healthy),
            "degraded" => Ok(Self::Degraded),
            "unavailable" => Ok(Self::Unavailable),
            other => Err(NyxProtocolError::UnsupportedHealth(other.to_owned())),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum NyxAvailability {
    Ready,
    Degraded,
    Unavailable,
    ContractOnly,
}

impl NyxAvailability {
    fn parse(value: &str) -> Result<Self, NyxProtocolError> {
        match value {
            "ready" => Ok(Self::Ready),
            "degraded" => Ok(Self::Degraded),
            "unavailable" => Ok(Self::Unavailable),
            "contract_only" => Ok(Self::ContractOnly),
            other => Err(NyxProtocolError::UnsupportedAvailability(other.to_owned())),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NyxCapability {
    capability_id: String,
    supported_version: NyxProtocolVersion,
    required_engine: String,
    availability: NyxAvailability,
    endpoint_ids: Vec<String>,
}

impl NyxCapability {
    pub fn capability_id(&self) -> &str {
        &self.capability_id
    }

    pub const fn supported_version(&self) -> NyxProtocolVersion {
        self.supported_version
    }

    pub fn required_engine(&self) -> &str {
        &self.required_engine
    }

    pub const fn availability(&self) -> NyxAvailability {
        self.availability
    }

    pub fn endpoint_ids(&self) -> &[String] {
        &self.endpoint_ids
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NyxEngineReadiness {
    engine: String,
    availability: NyxAvailability,
    live: bool,
    ready: bool,
    reason: String,
}

impl NyxEngineReadiness {
    pub fn engine(&self) -> &str {
        &self.engine
    }

    pub const fn availability(&self) -> NyxAvailability {
        self.availability
    }

    pub const fn live(&self) -> bool {
        self.live
    }

    pub const fn ready(&self) -> bool {
        self.ready
    }

    pub fn reason(&self) -> &str {
        &self.reason
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NyxProviderReadiness {
    provider_id: String,
    kind: String,
    configured: bool,
    availability: NyxAvailability,
    ready: bool,
    probe_posture: String,
    reason: String,
}

impl NyxProviderReadiness {
    pub fn provider_id(&self) -> &str {
        &self.provider_id
    }

    pub fn kind(&self) -> &str {
        &self.kind
    }

    pub const fn configured(&self) -> bool {
        self.configured
    }

    pub const fn availability(&self) -> NyxAvailability {
        self.availability
    }

    pub const fn ready(&self) -> bool {
        self.ready
    }

    pub fn probe_posture(&self) -> &str {
        &self.probe_posture
    }

    pub fn reason(&self) -> &str {
        &self.reason
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NyxServiceReport {
    selected_protocol: NyxProtocolVersion,
    server_version: String,
    protocol_schema_version: String,
    health: NyxHealth,
    live: bool,
    control_plane_ready: bool,
    model_requests_ready: bool,
    engine_ready_count: u32,
    engine_total_count: u32,
    provider_ready_count: u32,
    provider_total_count: u32,
    capabilities: Vec<NyxCapability>,
    engines: Vec<NyxEngineReadiness>,
    providers: Vec<NyxProviderReadiness>,
}

impl NyxServiceReport {
    pub const fn selected_protocol(&self) -> NyxProtocolVersion {
        self.selected_protocol
    }

    pub fn server_version(&self) -> &str {
        &self.server_version
    }

    pub fn protocol_schema_version(&self) -> &str {
        &self.protocol_schema_version
    }

    pub const fn health(&self) -> NyxHealth {
        self.health
    }

    pub const fn live(&self) -> bool {
        self.live
    }

    pub const fn control_plane_ready(&self) -> bool {
        self.control_plane_ready
    }

    pub const fn model_requests_ready(&self) -> bool {
        self.model_requests_ready
    }

    pub const fn engine_ready_count(&self) -> u32 {
        self.engine_ready_count
    }

    pub const fn engine_total_count(&self) -> u32 {
        self.engine_total_count
    }

    pub const fn provider_ready_count(&self) -> u32 {
        self.provider_ready_count
    }

    pub const fn provider_total_count(&self) -> u32 {
        self.provider_total_count
    }

    pub fn capabilities(&self) -> &[NyxCapability] {
        &self.capabilities
    }

    pub fn engines(&self) -> &[NyxEngineReadiness] {
        &self.engines
    }

    pub fn providers(&self) -> &[NyxProviderReadiness] {
        &self.providers
    }

    pub const fn is_ready(&self) -> bool {
        matches!(self.health, NyxHealth::Healthy)
            && self.live
            && self.control_plane_ready
            && self.model_requests_ready
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NyxProtocolError {
    InvalidJson(String),
    InvalidVersion(String),
    MissingField {
        context: &'static str,
        field: &'static str,
    },
    InvalidField {
        context: &'static str,
        field: &'static str,
        detail: String,
    },
    UnsupportedSchema {
        context: &'static str,
        expected: &'static str,
        found: String,
    },
    UnsupportedHealth(String),
    UnsupportedAvailability(String),
    DuplicateIdentifier {
        collection: &'static str,
        identifier: String,
    },
    InconsistentContract {
        context: &'static str,
        field: &'static str,
        expected: String,
        found: String,
    },
}

impl fmt::Display for NyxProtocolError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidJson(detail) => write!(formatter, "invalid Nyx JSON: {detail}"),
            Self::InvalidVersion(value) => {
                write!(formatter, "invalid Nyx contract version '{value}'")
            }
            Self::MissingField { context, field } => {
                write!(formatter, "Nyx {context} is missing field '{field}'")
            }
            Self::InvalidField {
                context,
                field,
                detail,
            } => write!(
                formatter,
                "Nyx {context} field '{field}' is invalid: {detail}"
            ),
            Self::UnsupportedSchema {
                context,
                expected,
                found,
            } => write!(
                formatter,
                "Nyx {context} uses schema '{found}', expected '{expected}'"
            ),
            Self::UnsupportedHealth(value) => {
                write!(formatter, "unsupported Nyx health status '{value}'")
            }
            Self::UnsupportedAvailability(value) => {
                write!(formatter, "unsupported Nyx availability '{value}'")
            }
            Self::DuplicateIdentifier {
                collection,
                identifier,
            } => write!(
                formatter,
                "duplicate Nyx {collection} identifier '{identifier}'"
            ),
            Self::InconsistentContract {
                context,
                field,
                expected,
                found,
            } => write!(
                formatter,
                "Nyx {context} field '{field}' is '{found}', expected '{expected}'"
            ),
        }
    }
}

impl std::error::Error for NyxProtocolError {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct VersionDocument {
    pub server_version: String,
    pub protocol_schema_version: String,
    pub public_contract_version: NyxProtocolVersion,
    pub compatible_major_versions: Vec<u16>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct HealthDocument {
    pub status: NyxHealth,
    pub live: bool,
    pub control_plane_ready: bool,
    pub model_requests_ready: bool,
    pub server_version: String,
    pub protocol_schema_version: String,
    pub public_contract_version: NyxProtocolVersion,
    pub engine_ready_count: u32,
    pub engine_total_count: u32,
    pub provider_ready_count: u32,
    pub provider_total_count: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CapabilityDocument {
    pub server_version: String,
    pub protocol_schema_version: String,
    pub public_contract_version: NyxProtocolVersion,
    pub capabilities: Vec<NyxCapability>,
    pub engines: Vec<NyxEngineReadiness>,
    pub providers: Vec<NyxProviderReadiness>,
}

pub(crate) fn decode_version(body: &[u8]) -> Result<VersionDocument, NyxProtocolError> {
    let object = decode_object(body, "version response")?;
    validate_schema(&object, "version response", SCHEMA_ID_VERSION)?;
    let public_contract_version =
        version_field(&object, "version response", "public_contract_version")?;
    let supported_contract_versions =
        version_array_field(&object, "version response", "supported_contract_versions")?;
    if !supported_contract_versions.contains(&public_contract_version) {
        return Err(NyxProtocolError::InvalidField {
            context: "version response",
            field: "supported_contract_versions",
            detail: "does not contain public_contract_version".to_owned(),
        });
    }
    let compatible_major_versions =
        u16_array_field(&object, "version response", "compatible_major_versions")?;
    if !compatible_major_versions.contains(&public_contract_version.major()) {
        return Err(NyxProtocolError::InvalidField {
            context: "version response",
            field: "compatible_major_versions",
            detail: "does not contain the public contract major".to_owned(),
        });
    }
    Ok(VersionDocument {
        server_version: nonempty_text_field(&object, "version response", "server_version")?,
        protocol_schema_version: exact_text_field(
            &object,
            "version response",
            "protocol_schema_version",
            SCHEMA_VERSION_V1,
        )?,
        public_contract_version,
        compatible_major_versions,
    })
}

pub(crate) fn decode_health(body: &[u8]) -> Result<HealthDocument, NyxProtocolError> {
    let object = decode_object(body, "health response")?;
    validate_schema(&object, "health response", SCHEMA_ID_HEALTH)?;
    Ok(HealthDocument {
        status: NyxHealth::parse(text_field(&object, "health response", "status")?)?,
        live: bool_field(&object, "health response", "live")?,
        control_plane_ready: bool_field(&object, "health response", "control_plane_ready")?,
        model_requests_ready: bool_field(&object, "health response", "model_requests_ready")?,
        server_version: nonempty_text_field(&object, "health response", "server_version")?,
        protocol_schema_version: exact_text_field(
            &object,
            "health response",
            "protocol_schema_version",
            SCHEMA_VERSION_V1,
        )?,
        public_contract_version: version_field(
            &object,
            "health response",
            "public_contract_version",
        )?,
        engine_ready_count: u32_field(&object, "health response", "engine_ready_count")?,
        engine_total_count: u32_field(&object, "health response", "engine_total_count")?,
        provider_ready_count: u32_field(&object, "health response", "provider_ready_count")?,
        provider_total_count: u32_field(&object, "health response", "provider_total_count")?,
    })
}

pub(crate) fn decode_capabilities(body: &[u8]) -> Result<CapabilityDocument, NyxProtocolError> {
    let object = decode_object(body, "capability response")?;
    validate_schema(&object, "capability response", SCHEMA_ID_CAPABILITIES)?;
    let mut capabilities = parse_capabilities(&object)?;
    let mut engines = parse_engines(&object)?;
    let mut providers = parse_providers(&object)?;
    capabilities.sort_by(|left, right| left.capability_id.cmp(&right.capability_id));
    engines.sort_by(|left, right| left.engine.cmp(&right.engine));
    providers.sort_by(|left, right| left.provider_id.cmp(&right.provider_id));
    Ok(CapabilityDocument {
        server_version: nonempty_text_field(&object, "capability response", "server_version")?,
        protocol_schema_version: exact_text_field(
            &object,
            "capability response",
            "protocol_schema_version",
            SCHEMA_VERSION_V1,
        )?,
        public_contract_version: version_field(
            &object,
            "capability response",
            "public_contract_version",
        )?,
        capabilities,
        engines,
        providers,
    })
}

pub(crate) fn decode_incompatible_versions(
    body: &[u8],
) -> Result<Vec<NyxProtocolVersion>, NyxProtocolError> {
    let object = decode_object(body, "error response")?;
    validate_schema(&object, "error response", SCHEMA_ID_ERROR)?;
    let error = object_field(&object, "error response", "error")?;
    let code = text_field(error, "error response", "code")?;
    if code != INCOMPATIBLE_MAJOR_CODE {
        return Err(NyxProtocolError::InvalidField {
            context: "error response",
            field: "code",
            detail: format!("expected '{INCOMPATIBLE_MAJOR_CODE}', found '{code}'"),
        });
    }
    version_array_field(error, "error response", "supported_contract_versions")
}

pub(crate) fn assemble_report(
    version: VersionDocument,
    health: HealthDocument,
    capabilities: CapabilityDocument,
) -> Result<NyxServiceReport, NyxProtocolError> {
    if health.engine_ready_count > health.engine_total_count {
        return Err(NyxProtocolError::InvalidField {
            context: "health response",
            field: "engine_ready_count",
            detail: "ready count exceeds total count".to_owned(),
        });
    }
    if health.provider_ready_count > health.provider_total_count {
        return Err(NyxProtocolError::InvalidField {
            context: "health response",
            field: "provider_ready_count",
            detail: "ready count exceeds total count".to_owned(),
        });
    }
    if usize::try_from(health.engine_total_count).ok() != Some(capabilities.engines.len()) {
        return Err(NyxProtocolError::InvalidField {
            context: "capability response",
            field: "engines",
            detail: "engine inventory length does not match health total".to_owned(),
        });
    }
    if usize::try_from(health.provider_total_count).ok() != Some(capabilities.providers.len()) {
        return Err(NyxProtocolError::InvalidField {
            context: "capability response",
            field: "providers",
            detail: "provider inventory length does not match health total".to_owned(),
        });
    }
    let ready_engines = capabilities
        .engines
        .iter()
        .filter(|item| item.ready)
        .count();
    if usize::try_from(health.engine_ready_count).ok() != Some(ready_engines) {
        return Err(NyxProtocolError::InvalidField {
            context: "capability response",
            field: "engines",
            detail: "ready engine count does not match health summary".to_owned(),
        });
    }
    let ready_providers = capabilities
        .providers
        .iter()
        .filter(|item| item.ready)
        .count();
    if usize::try_from(health.provider_ready_count).ok() != Some(ready_providers) {
        return Err(NyxProtocolError::InvalidField {
            context: "capability response",
            field: "providers",
            detail: "ready provider count does not match health summary".to_owned(),
        });
    }
    if capabilities
        .capabilities
        .iter()
        .any(|item| item.supported_version.major() != version.public_contract_version.major())
    {
        return Err(NyxProtocolError::InvalidField {
            context: "capability response",
            field: "capabilities",
            detail: "capability version has an incompatible major".to_owned(),
        });
    }
    require_equal(
        "health response",
        "server_version",
        &version.server_version,
        &health.server_version,
    )?;
    require_equal(
        "capability response",
        "server_version",
        &version.server_version,
        &capabilities.server_version,
    )?;
    require_equal(
        "health response",
        "protocol_schema_version",
        &version.protocol_schema_version,
        &health.protocol_schema_version,
    )?;
    require_equal(
        "capability response",
        "protocol_schema_version",
        &version.protocol_schema_version,
        &capabilities.protocol_schema_version,
    )?;
    require_equal(
        "health response",
        "public_contract_version",
        &version.public_contract_version.to_string(),
        &health.public_contract_version.to_string(),
    )?;
    require_equal(
        "capability response",
        "public_contract_version",
        &version.public_contract_version.to_string(),
        &capabilities.public_contract_version.to_string(),
    )?;
    Ok(NyxServiceReport {
        selected_protocol: version.public_contract_version,
        server_version: version.server_version,
        protocol_schema_version: version.protocol_schema_version,
        health: health.status,
        live: health.live,
        control_plane_ready: health.control_plane_ready,
        model_requests_ready: health.model_requests_ready,
        engine_ready_count: health.engine_ready_count,
        engine_total_count: health.engine_total_count,
        provider_ready_count: health.provider_ready_count,
        provider_total_count: health.provider_total_count,
        capabilities: capabilities.capabilities,
        engines: capabilities.engines,
        providers: capabilities.providers,
    })
}

fn parse_capabilities(root: &Map<String, Value>) -> Result<Vec<NyxCapability>, NyxProtocolError> {
    let values = array_field(root, "capability response", "capabilities")?;
    let mut seen = BTreeSet::new();
    let mut result = Vec::with_capacity(values.len());
    for value in values {
        let object = value
            .as_object()
            .ok_or_else(|| NyxProtocolError::InvalidField {
                context: "capability response",
                field: "capabilities",
                detail: "entry is not an object".to_owned(),
            })?;
        validate_schema(object, "capability descriptor", SCHEMA_ID_CAPABILITY)?;
        let capability_id = nonempty_text_field(object, "capability descriptor", "capability_id")?;
        if !seen.insert(capability_id.clone()) {
            return Err(NyxProtocolError::DuplicateIdentifier {
                collection: "capability",
                identifier: capability_id,
            });
        }
        result.push(NyxCapability {
            capability_id,
            supported_version: version_field(object, "capability descriptor", "supported_version")?,
            required_engine: nonempty_text_field(
                object,
                "capability descriptor",
                "required_engine",
            )?,
            availability: NyxAvailability::parse(text_field(
                object,
                "capability descriptor",
                "availability",
            )?)?,
            endpoint_ids: string_array_field(object, "capability descriptor", "endpoint_ids")?,
        });
    }
    Ok(result)
}

fn parse_engines(root: &Map<String, Value>) -> Result<Vec<NyxEngineReadiness>, NyxProtocolError> {
    let values = array_field(root, "capability response", "engines")?;
    let mut seen = BTreeSet::new();
    let mut result = Vec::with_capacity(values.len());
    for value in values {
        let object = value
            .as_object()
            .ok_or_else(|| NyxProtocolError::InvalidField {
                context: "capability response",
                field: "engines",
                detail: "entry is not an object".to_owned(),
            })?;
        let engine = nonempty_text_field(object, "engine readiness", "engine")?;
        if !seen.insert(engine.clone()) {
            return Err(NyxProtocolError::DuplicateIdentifier {
                collection: "engine",
                identifier: engine,
            });
        }
        result.push(NyxEngineReadiness {
            engine,
            availability: NyxAvailability::parse(text_field(
                object,
                "engine readiness",
                "availability",
            )?)?,
            live: bool_field(object, "engine readiness", "live")?,
            ready: bool_field(object, "engine readiness", "ready")?,
            reason: text_field(object, "engine readiness", "reason")?.to_owned(),
        });
    }
    Ok(result)
}

fn parse_providers(
    root: &Map<String, Value>,
) -> Result<Vec<NyxProviderReadiness>, NyxProtocolError> {
    let values = array_field(root, "capability response", "providers")?;
    let mut seen = BTreeSet::new();
    let mut result = Vec::with_capacity(values.len());
    for value in values {
        let object = value
            .as_object()
            .ok_or_else(|| NyxProtocolError::InvalidField {
                context: "capability response",
                field: "providers",
                detail: "entry is not an object".to_owned(),
            })?;
        let provider_id = nonempty_text_field(object, "provider readiness", "provider_id")?;
        if !seen.insert(provider_id.clone()) {
            return Err(NyxProtocolError::DuplicateIdentifier {
                collection: "provider",
                identifier: provider_id,
            });
        }
        result.push(NyxProviderReadiness {
            provider_id,
            kind: nonempty_text_field(object, "provider readiness", "kind")?,
            configured: bool_field(object, "provider readiness", "configured")?,
            availability: NyxAvailability::parse(text_field(
                object,
                "provider readiness",
                "availability",
            )?)?,
            ready: bool_field(object, "provider readiness", "ready")?,
            probe_posture: text_field(object, "provider readiness", "probe_posture")?.to_owned(),
            reason: text_field(object, "provider readiness", "reason")?.to_owned(),
        });
    }
    Ok(result)
}

fn decode_object(
    body: &[u8],
    context: &'static str,
) -> Result<Map<String, Value>, NyxProtocolError> {
    let value: Value = serde_json::from_slice(body)
        .map_err(|error| NyxProtocolError::InvalidJson(error.to_string()))?;
    value
        .as_object()
        .cloned()
        .ok_or_else(|| NyxProtocolError::InvalidField {
            context,
            field: "body",
            detail: "top-level JSON value is not an object".to_owned(),
        })
}

fn validate_schema(
    object: &Map<String, Value>,
    context: &'static str,
    schema_id: &'static str,
) -> Result<(), NyxProtocolError> {
    exact_text_field(object, context, "schema_version", SCHEMA_VERSION_V1)?;
    let found = text_field(object, context, "schema_id")?;
    if found != schema_id {
        return Err(NyxProtocolError::UnsupportedSchema {
            context,
            expected: schema_id,
            found: found.to_owned(),
        });
    }
    Ok(())
}

fn object_field<'a>(
    object: &'a Map<String, Value>,
    context: &'static str,
    field: &'static str,
) -> Result<&'a Map<String, Value>, NyxProtocolError> {
    object
        .get(field)
        .ok_or(NyxProtocolError::MissingField { context, field })?
        .as_object()
        .ok_or_else(|| NyxProtocolError::InvalidField {
            context,
            field,
            detail: "expected object".to_owned(),
        })
}

fn array_field<'a>(
    object: &'a Map<String, Value>,
    context: &'static str,
    field: &'static str,
) -> Result<&'a [Value], NyxProtocolError> {
    object
        .get(field)
        .ok_or(NyxProtocolError::MissingField { context, field })?
        .as_array()
        .map(|values| values.as_slice())
        .ok_or_else(|| NyxProtocolError::InvalidField {
            context,
            field,
            detail: "expected array".to_owned(),
        })
}

fn text_field<'a>(
    object: &'a Map<String, Value>,
    context: &'static str,
    field: &'static str,
) -> Result<&'a str, NyxProtocolError> {
    object
        .get(field)
        .ok_or(NyxProtocolError::MissingField { context, field })?
        .as_str()
        .ok_or_else(|| NyxProtocolError::InvalidField {
            context,
            field,
            detail: "expected string".to_owned(),
        })
}

fn nonempty_text_field(
    object: &Map<String, Value>,
    context: &'static str,
    field: &'static str,
) -> Result<String, NyxProtocolError> {
    let value = text_field(object, context, field)?.trim();
    if value.is_empty() {
        return Err(NyxProtocolError::InvalidField {
            context,
            field,
            detail: "must not be empty".to_owned(),
        });
    }
    Ok(value.to_owned())
}

fn exact_text_field(
    object: &Map<String, Value>,
    context: &'static str,
    field: &'static str,
    expected: &'static str,
) -> Result<String, NyxProtocolError> {
    let found = text_field(object, context, field)?;
    if found != expected {
        return Err(NyxProtocolError::InconsistentContract {
            context,
            field,
            expected: expected.to_owned(),
            found: found.to_owned(),
        });
    }
    Ok(found.to_owned())
}

fn bool_field(
    object: &Map<String, Value>,
    context: &'static str,
    field: &'static str,
) -> Result<bool, NyxProtocolError> {
    object
        .get(field)
        .ok_or(NyxProtocolError::MissingField { context, field })?
        .as_bool()
        .ok_or_else(|| NyxProtocolError::InvalidField {
            context,
            field,
            detail: "expected boolean".to_owned(),
        })
}

fn u32_field(
    object: &Map<String, Value>,
    context: &'static str,
    field: &'static str,
) -> Result<u32, NyxProtocolError> {
    let value = object
        .get(field)
        .ok_or(NyxProtocolError::MissingField { context, field })?
        .as_u64()
        .ok_or_else(|| NyxProtocolError::InvalidField {
            context,
            field,
            detail: "expected unsigned integer".to_owned(),
        })?;
    u32::try_from(value).map_err(|_| NyxProtocolError::InvalidField {
        context,
        field,
        detail: "value exceeds u32".to_owned(),
    })
}

fn version_field(
    object: &Map<String, Value>,
    context: &'static str,
    field: &'static str,
) -> Result<NyxProtocolVersion, NyxProtocolError> {
    NyxProtocolVersion::parse(text_field(object, context, field)?)
}

fn string_array_field(
    object: &Map<String, Value>,
    context: &'static str,
    field: &'static str,
) -> Result<Vec<String>, NyxProtocolError> {
    array_field(object, context, field)?
        .iter()
        .map(|value| {
            value
                .as_str()
                .map(ToOwned::to_owned)
                .ok_or_else(|| NyxProtocolError::InvalidField {
                    context,
                    field,
                    detail: "array contains non-string value".to_owned(),
                })
        })
        .collect()
}

fn version_array_field(
    object: &Map<String, Value>,
    context: &'static str,
    field: &'static str,
) -> Result<Vec<NyxProtocolVersion>, NyxProtocolError> {
    let mut versions = Vec::new();
    for value in array_field(object, context, field)? {
        let text = value
            .as_str()
            .ok_or_else(|| NyxProtocolError::InvalidField {
                context,
                field,
                detail: "array contains non-string value".to_owned(),
            })?;
        versions.push(NyxProtocolVersion::parse(text)?);
    }
    versions.sort_unstable();
    versions.dedup();
    Ok(versions)
}

fn u16_array_field(
    object: &Map<String, Value>,
    context: &'static str,
    field: &'static str,
) -> Result<Vec<u16>, NyxProtocolError> {
    let mut values = Vec::new();
    for value in array_field(object, context, field)? {
        let number = value
            .as_u64()
            .ok_or_else(|| NyxProtocolError::InvalidField {
                context,
                field,
                detail: "array contains non-integer value".to_owned(),
            })?;
        values.push(
            u16::try_from(number).map_err(|_| NyxProtocolError::InvalidField {
                context,
                field,
                detail: "array value exceeds u16".to_owned(),
            })?,
        );
    }
    values.sort_unstable();
    values.dedup();
    Ok(values)
}

fn require_equal(
    context: &'static str,
    field: &'static str,
    expected: &str,
    found: &str,
) -> Result<(), NyxProtocolError> {
    if expected != found {
        return Err(NyxProtocolError::InconsistentContract {
            context,
            field,
            expected: expected.to_owned(),
            found: found.to_owned(),
        });
    }
    Ok(())
}
