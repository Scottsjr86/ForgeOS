//! Versioned Nyx handshake contract.
//!
//! This module owns the ForgeOS side of the compatibility handshake with the
//! separate `nyx_server` process. A successful transport exchange is not enough:
//! the response must decode under this wire schema, select a protocol version
//! ForgeOS offered, and report explicit health and capabilities.

use std::fmt;

const REQUEST_MAGIC: [u8; 8] = *b"FGNYXQ\0\0";
const RESPONSE_MAGIC: [u8; 8] = *b"FGNYXR\0\0";
const WIRE_SCHEMA_VERSION: u16 = 1;
const MAX_PROTOCOL_VERSIONS: usize = 32;
const MAX_CAPABILITIES: usize = 256;
const MAX_CAPABILITY_BYTES: usize = 96;
const MAX_SERVICE_VERSION_BYTES: usize = 128;

/// One Nyx application-protocol version supported by both ForgeOS and Nyx.
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
}

impl fmt::Display for NyxProtocolVersion {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}.{}", self.major, self.minor)
    }
}

/// Canonical Nyx capability token, such as `chat` or `tools.read`.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NyxCapability(String);

impl NyxCapability {
    pub fn new(value: impl Into<String>) -> Result<Self, NyxProtocolError> {
        let value = value.into();
        validate_capability(&value)?;
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for NyxCapability {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

/// Health declared by Nyx itself during the compatibility handshake.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NyxHealth {
    Healthy,
    Degraded,
    Unhealthy,
}

impl NyxHealth {
    const fn code(self) -> u8 {
        match self {
            Self::Healthy => 1,
            Self::Degraded => 2,
            Self::Unhealthy => 3,
        }
    }

    fn from_code(code: u8) -> Result<Self, NyxProtocolError> {
        match code {
            1 => Ok(Self::Healthy),
            2 => Ok(Self::Degraded),
            3 => Ok(Self::Unhealthy),
            found => Err(NyxProtocolError::UnknownHealthCode { found }),
        }
    }

    pub const fn is_healthy(self) -> bool {
        matches!(self, Self::Healthy)
    }
}

/// ForgeOS handshake request containing every application-protocol version it
/// is prepared to speak.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NyxHandshakeRequest {
    supported_versions: Vec<NyxProtocolVersion>,
}

impl NyxHandshakeRequest {
    pub fn new(
        supported_versions: impl IntoIterator<Item = NyxProtocolVersion>,
    ) -> Result<Self, NyxProtocolError> {
        let mut supported_versions: Vec<_> = supported_versions.into_iter().collect();
        supported_versions.sort_unstable();
        supported_versions.dedup();
        validate_protocol_count(supported_versions.len())?;
        Ok(Self { supported_versions })
    }

    pub fn supported_versions(&self) -> &[NyxProtocolVersion] {
        &self.supported_versions
    }

    pub fn supports(&self, version: NyxProtocolVersion) -> bool {
        self.supported_versions.binary_search(&version).is_ok()
    }

    pub fn encode(&self) -> Vec<u8> {
        let mut encoder = Encoder::new();
        encoder.bytes(&REQUEST_MAGIC);
        encoder.u16(WIRE_SCHEMA_VERSION);
        encoder.u16(self.supported_versions.len() as u16);
        for version in &self.supported_versions {
            encoder.u16(version.major());
            encoder.u16(version.minor());
        }
        encoder.finish()
    }

    pub fn decode(bytes: &[u8]) -> Result<Self, NyxProtocolError> {
        let mut decoder = Decoder::new(bytes);
        if decoder.array::<8>()? != REQUEST_MAGIC {
            return Err(NyxProtocolError::BadMagic {
                message: NyxWireMessage::Request,
            });
        }
        decode_schema(&mut decoder, NyxWireMessage::Request)?;
        let count = usize::from(decoder.u16()?);
        validate_protocol_count(count)?;
        let mut supported_versions = Vec::with_capacity(count);
        for _ in 0..count {
            supported_versions.push(NyxProtocolVersion::new(decoder.u16()?, decoder.u16()?));
        }
        decoder.finish()?;
        require_strict_order(&supported_versions)?;
        Ok(Self { supported_versions })
    }
}

/// Nyx handshake response containing the selected protocol, service health,
/// service version, and canonical capability set.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NyxHandshakeResponse {
    selected_protocol: NyxProtocolVersion,
    service_version: String,
    health: NyxHealth,
    capabilities: Vec<NyxCapability>,
}

impl NyxHandshakeResponse {
    pub fn new(
        selected_protocol: NyxProtocolVersion,
        service_version: impl Into<String>,
        health: NyxHealth,
        capabilities: impl IntoIterator<Item = NyxCapability>,
    ) -> Result<Self, NyxProtocolError> {
        let service_version = service_version.into();
        validate_service_version(&service_version)?;
        let mut capabilities: Vec<_> = capabilities.into_iter().collect();
        capabilities.sort_unstable();
        if capabilities.len() > MAX_CAPABILITIES {
            return Err(NyxProtocolError::TooManyCapabilities {
                maximum: MAX_CAPABILITIES,
                actual: capabilities.len(),
            });
        }
        for pair in capabilities.windows(2) {
            if pair[0] == pair[1] {
                return Err(NyxProtocolError::DuplicateCapability(
                    pair[0].as_str().to_owned(),
                ));
            }
        }
        Ok(Self {
            selected_protocol,
            service_version,
            health,
            capabilities,
        })
    }

    pub const fn selected_protocol(&self) -> NyxProtocolVersion {
        self.selected_protocol
    }

    pub fn service_version(&self) -> &str {
        &self.service_version
    }

    pub const fn health(&self) -> NyxHealth {
        self.health
    }

    pub fn capabilities(&self) -> &[NyxCapability] {
        &self.capabilities
    }

    pub fn encode(&self) -> Vec<u8> {
        let mut encoder = Encoder::new();
        encoder.bytes(&RESPONSE_MAGIC);
        encoder.u16(WIRE_SCHEMA_VERSION);
        encoder.u16(self.selected_protocol.major());
        encoder.u16(self.selected_protocol.minor());
        encoder.u8(self.health.code());
        encoder.text(&self.service_version);
        encoder.u16(self.capabilities.len() as u16);
        for capability in &self.capabilities {
            encoder.text(capability.as_str());
        }
        encoder.finish()
    }

    pub fn decode(bytes: &[u8]) -> Result<Self, NyxProtocolError> {
        let mut decoder = Decoder::new(bytes);
        if decoder.array::<8>()? != RESPONSE_MAGIC {
            return Err(NyxProtocolError::BadMagic {
                message: NyxWireMessage::Response,
            });
        }
        decode_schema(&mut decoder, NyxWireMessage::Response)?;
        let selected_protocol = NyxProtocolVersion::new(decoder.u16()?, decoder.u16()?);
        let health = NyxHealth::from_code(decoder.u8()?)?;
        let service_version = decoder.text(MAX_SERVICE_VERSION_BYTES)?;
        validate_service_version(&service_version)?;
        let count = usize::from(decoder.u16()?);
        if count > MAX_CAPABILITIES {
            return Err(NyxProtocolError::TooManyCapabilities {
                maximum: MAX_CAPABILITIES,
                actual: count,
            });
        }
        let mut capabilities = Vec::with_capacity(count);
        for _ in 0..count {
            capabilities.push(NyxCapability::new(decoder.text(MAX_CAPABILITY_BYTES)?)?);
        }
        decoder.finish()?;
        require_strict_capability_order(&capabilities)?;
        Ok(Self {
            selected_protocol,
            service_version,
            health,
            capabilities,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NyxWireMessage {
    Request,
    Response,
}

impl NyxWireMessage {
    const fn label(self) -> &'static str {
        match self {
            Self::Request => "request",
            Self::Response => "response",
        }
    }
}

/// Structural or canonical violation in the Nyx handshake contract.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NyxProtocolError {
    BadMagic {
        message: NyxWireMessage,
    },
    UnsupportedWireSchema {
        message: NyxWireMessage,
        found: u16,
        supported: u16,
    },
    EmptyProtocolSet,
    TooManyProtocolVersions {
        maximum: usize,
        actual: usize,
    },
    NonCanonicalProtocolOrder,
    EmptyServiceVersion,
    ServiceVersionTooLong {
        maximum: usize,
        actual: usize,
    },
    InvalidServiceVersion,
    EmptyCapability,
    CapabilityTooLong {
        maximum: usize,
        actual: usize,
    },
    InvalidCapability(String),
    DuplicateCapability(String),
    NonCanonicalCapabilityOrder,
    TooManyCapabilities {
        maximum: usize,
        actual: usize,
    },
    UnknownHealthCode {
        found: u8,
    },
    UnexpectedEnd,
    InvalidUtf8,
    FieldTooLong {
        maximum: usize,
        actual: usize,
    },
    TrailingBytes {
        remaining: usize,
    },
}

impl fmt::Display for NyxProtocolError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::BadMagic { message } => {
                write!(formatter, "bad Nyx {} magic", message.label())
            }
            Self::UnsupportedWireSchema {
                message,
                found,
                supported,
            } => write!(
                formatter,
                "unsupported Nyx {} wire schema {found}; supported schema is {supported}",
                message.label()
            ),
            Self::EmptyProtocolSet => formatter.write_str("Nyx protocol set is empty"),
            Self::TooManyProtocolVersions { maximum, actual } => write!(
                formatter,
                "Nyx protocol set contains {actual} versions; maximum is {maximum}"
            ),
            Self::NonCanonicalProtocolOrder => {
                formatter.write_str("Nyx protocol versions are not strictly ordered")
            }
            Self::EmptyServiceVersion => formatter.write_str("Nyx service version is empty"),
            Self::ServiceVersionTooLong { maximum, actual } => write!(
                formatter,
                "Nyx service version length {actual} exceeds maximum {maximum}"
            ),
            Self::InvalidServiceVersion => {
                formatter.write_str("Nyx service version contains invalid bytes")
            }
            Self::EmptyCapability => formatter.write_str("Nyx capability is empty"),
            Self::CapabilityTooLong { maximum, actual } => write!(
                formatter,
                "Nyx capability length {actual} exceeds maximum {maximum}"
            ),
            Self::InvalidCapability(value) => write!(formatter, "invalid Nyx capability {value:?}"),
            Self::DuplicateCapability(value) => {
                write!(formatter, "duplicate Nyx capability {value:?}")
            }
            Self::NonCanonicalCapabilityOrder => {
                formatter.write_str("Nyx capabilities are not strictly ordered")
            }
            Self::TooManyCapabilities { maximum, actual } => write!(
                formatter,
                "Nyx response contains {actual} capabilities; maximum is {maximum}"
            ),
            Self::UnknownHealthCode { found } => {
                write!(formatter, "unknown Nyx health code {found}")
            }
            Self::UnexpectedEnd => formatter.write_str("Nyx message ended unexpectedly"),
            Self::InvalidUtf8 => formatter.write_str("Nyx text field is not UTF-8"),
            Self::FieldTooLong { maximum, actual } => write!(
                formatter,
                "Nyx text field length {actual} exceeds maximum {maximum}"
            ),
            Self::TrailingBytes { remaining } => {
                write!(formatter, "Nyx message has {remaining} trailing bytes")
            }
        }
    }
}

impl std::error::Error for NyxProtocolError {}

fn validate_protocol_count(count: usize) -> Result<(), NyxProtocolError> {
    if count == 0 {
        return Err(NyxProtocolError::EmptyProtocolSet);
    }
    if count > MAX_PROTOCOL_VERSIONS {
        return Err(NyxProtocolError::TooManyProtocolVersions {
            maximum: MAX_PROTOCOL_VERSIONS,
            actual: count,
        });
    }
    Ok(())
}

fn require_strict_order(versions: &[NyxProtocolVersion]) -> Result<(), NyxProtocolError> {
    if versions.windows(2).any(|pair| pair[0] >= pair[1]) {
        Err(NyxProtocolError::NonCanonicalProtocolOrder)
    } else {
        Ok(())
    }
}

fn validate_service_version(value: &str) -> Result<(), NyxProtocolError> {
    if value.is_empty() {
        return Err(NyxProtocolError::EmptyServiceVersion);
    }
    if value.len() > MAX_SERVICE_VERSION_BYTES {
        return Err(NyxProtocolError::ServiceVersionTooLong {
            maximum: MAX_SERVICE_VERSION_BYTES,
            actual: value.len(),
        });
    }
    if !value.bytes().all(|byte| byte.is_ascii_graphic()) {
        return Err(NyxProtocolError::InvalidServiceVersion);
    }
    Ok(())
}

fn validate_capability(value: &str) -> Result<(), NyxProtocolError> {
    if value.is_empty() {
        return Err(NyxProtocolError::EmptyCapability);
    }
    if value.len() > MAX_CAPABILITY_BYTES {
        return Err(NyxProtocolError::CapabilityTooLong {
            maximum: MAX_CAPABILITY_BYTES,
            actual: value.len(),
        });
    }
    let valid = value.bytes().all(|byte| {
        byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'.' | b'-' | b'_')
    });
    if !valid {
        return Err(NyxProtocolError::InvalidCapability(value.to_owned()));
    }
    Ok(())
}

fn require_strict_capability_order(capabilities: &[NyxCapability]) -> Result<(), NyxProtocolError> {
    if capabilities.windows(2).any(|pair| pair[0] >= pair[1]) {
        Err(NyxProtocolError::NonCanonicalCapabilityOrder)
    } else {
        Ok(())
    }
}

fn decode_schema(
    decoder: &mut Decoder<'_>,
    message: NyxWireMessage,
) -> Result<(), NyxProtocolError> {
    let found = decoder.u16()?;
    if found != WIRE_SCHEMA_VERSION {
        return Err(NyxProtocolError::UnsupportedWireSchema {
            message,
            found,
            supported: WIRE_SCHEMA_VERSION,
        });
    }
    Ok(())
}

struct Encoder {
    bytes: Vec<u8>,
}

impl Encoder {
    fn new() -> Self {
        Self { bytes: Vec::new() }
    }

    fn u8(&mut self, value: u8) {
        self.bytes.push(value);
    }

    fn u16(&mut self, value: u16) {
        self.bytes.extend_from_slice(&value.to_be_bytes());
    }

    fn bytes(&mut self, value: &[u8]) {
        self.bytes.extend_from_slice(value);
    }

    fn text(&mut self, value: &str) {
        self.u16(value.len() as u16);
        self.bytes(value.as_bytes());
    }

    fn finish(self) -> Vec<u8> {
        self.bytes
    }
}

struct Decoder<'a> {
    bytes: &'a [u8],
    cursor: usize,
}

impl<'a> Decoder<'a> {
    fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, cursor: 0 }
    }

    fn array<const N: usize>(&mut self) -> Result<[u8; N], NyxProtocolError> {
        let end = self
            .cursor
            .checked_add(N)
            .ok_or(NyxProtocolError::UnexpectedEnd)?;
        let slice = self
            .bytes
            .get(self.cursor..end)
            .ok_or(NyxProtocolError::UnexpectedEnd)?;
        self.cursor = end;
        let mut output = [0_u8; N];
        output.copy_from_slice(slice);
        Ok(output)
    }

    fn u8(&mut self) -> Result<u8, NyxProtocolError> {
        Ok(self.array::<1>()?[0])
    }

    fn u16(&mut self) -> Result<u16, NyxProtocolError> {
        Ok(u16::from_be_bytes(self.array()?))
    }

    fn text(&mut self, maximum: usize) -> Result<String, NyxProtocolError> {
        let length = usize::from(self.u16()?);
        if length > maximum {
            return Err(NyxProtocolError::FieldTooLong {
                maximum,
                actual: length,
            });
        }
        let end = self
            .cursor
            .checked_add(length)
            .ok_or(NyxProtocolError::UnexpectedEnd)?;
        let slice = self
            .bytes
            .get(self.cursor..end)
            .ok_or(NyxProtocolError::UnexpectedEnd)?;
        self.cursor = end;
        std::str::from_utf8(slice)
            .map(str::to_owned)
            .map_err(|_| NyxProtocolError::InvalidUtf8)
    }

    fn finish(self) -> Result<(), NyxProtocolError> {
        let remaining = self.bytes.len().saturating_sub(self.cursor);
        if remaining == 0 {
            Ok(())
        } else {
            Err(NyxProtocolError::TrailingBytes { remaining })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn version(major: u16, minor: u16) -> NyxProtocolVersion {
        NyxProtocolVersion::new(major, minor)
    }

    fn capability(value: &str) -> NyxCapability {
        NyxCapability::new(value).unwrap()
    }

    #[test]
    fn request_round_trip_is_sorted_and_canonical() {
        let request =
            NyxHandshakeRequest::new([version(1, 1), version(1, 0), version(1, 1)]).unwrap();
        assert_eq!(
            request.supported_versions(),
            &[version(1, 0), version(1, 1)]
        );
        assert_eq!(
            NyxHandshakeRequest::decode(&request.encode()).unwrap(),
            request
        );
    }

    #[test]
    fn response_round_trip_retains_health_and_capabilities() {
        let response = NyxHandshakeResponse::new(
            version(1, 1),
            "nyx-0.1.0",
            NyxHealth::Degraded,
            [capability("tools.read"), capability("chat")],
        )
        .unwrap();
        assert_eq!(
            response
                .capabilities()
                .iter()
                .map(NyxCapability::as_str)
                .collect::<Vec<_>>(),
            vec!["chat", "tools.read"]
        );
        assert_eq!(
            NyxHandshakeResponse::decode(&response.encode()).unwrap(),
            response
        );
    }

    #[test]
    fn duplicate_capabilities_are_rejected() {
        let error = NyxHandshakeResponse::new(
            version(1, 0),
            "nyx-0.1.0",
            NyxHealth::Healthy,
            [capability("chat"), capability("chat")],
        )
        .unwrap_err();
        assert_eq!(
            error,
            NyxProtocolError::DuplicateCapability("chat".to_owned())
        );
    }
}
