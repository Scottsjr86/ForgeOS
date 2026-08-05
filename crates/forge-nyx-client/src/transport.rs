//! HTTP transport for the separate Nyx_Server process.
//!
//! ForgeOS performs three read-only public-contract requests. Transport success
//! alone is never compatibility: response headers, status, schema IDs, versions,
//! health, capabilities, engines, and providers are all validated.

use crate::protocol::{
    assemble_report, decode_capabilities, decode_health, decode_incompatible_versions,
    decode_version, NyxProtocolError, NyxProtocolVersion, NyxServiceReport, CAPABILITIES_PATH,
    HEADER_NYX_CONTRACT_VERSION, HEALTH_PATH, VERSION_PATH,
};
use std::collections::BTreeMap;
use std::fmt;
use std::io::{self, Read, Write};
use std::net::{SocketAddr, TcpStream};
use std::path::PathBuf;
use std::time::Duration;

#[cfg(unix)]
use std::os::unix::net::UnixStream;

const DEFAULT_IO_TIMEOUT: Duration = Duration::from_secs(2);
const DEFAULT_MAX_RESPONSE_BYTES: usize = 4 * 1024 * 1024;
const MAX_SUPPORTED_VERSIONS: usize = 32;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum NyxHttpMethod {
    Get,
    Post,
}

impl NyxHttpMethod {
    const fn as_str(self) -> &'static str {
        match self {
            Self::Get => "GET",
            Self::Post => "POST",
        }
    }
}

#[derive(Debug)]
pub(crate) struct NyxJsonResponse {
    pub(crate) status: u16,
    pub(crate) contract_version: NyxProtocolVersion,
    pub(crate) body: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NyxTransportEndpoint {
    UnixSocket(PathBuf),
    Tcp(SocketAddr),
}

impl NyxTransportEndpoint {
    pub fn unix_socket(path: impl Into<PathBuf>) -> Self {
        Self::UnixSocket(path.into())
    }

    pub const fn tcp(address: SocketAddr) -> Self {
        Self::Tcp(address)
    }

    fn host_header(&self) -> String {
        match self {
            Self::UnixSocket(_) => "localhost".to_owned(),
            Self::Tcp(address) => address.to_string(),
        }
    }
}

impl fmt::Display for NyxTransportEndpoint {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnixSocket(path) => write!(formatter, "unix:{}", path.display()),
            Self::Tcp(address) => write!(formatter, "tcp:{address}"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NyxClientConfig {
    endpoint: NyxTransportEndpoint,
    supported_versions: Vec<NyxProtocolVersion>,
    io_timeout: Duration,
    maximum_response_bytes: usize,
}

impl NyxClientConfig {
    pub fn new(
        endpoint: NyxTransportEndpoint,
        supported_versions: impl IntoIterator<Item = NyxProtocolVersion>,
    ) -> Result<Self, NyxProtocolError> {
        let mut supported_versions: Vec<_> = supported_versions.into_iter().collect();
        supported_versions.sort_unstable();
        supported_versions.dedup();
        if supported_versions.is_empty() || supported_versions.len() > MAX_SUPPORTED_VERSIONS {
            return Err(NyxProtocolError::InvalidField {
                context: "client configuration",
                field: "supported_versions",
                detail: format!("must contain between 1 and {MAX_SUPPORTED_VERSIONS} versions"),
            });
        }
        Ok(Self {
            endpoint,
            supported_versions,
            io_timeout: DEFAULT_IO_TIMEOUT,
            maximum_response_bytes: DEFAULT_MAX_RESPONSE_BYTES,
        })
    }

    pub fn with_io_timeout(mut self, timeout: Duration) -> Self {
        self.io_timeout = timeout;
        self
    }

    pub fn with_maximum_response_bytes(mut self, maximum: usize) -> Self {
        self.maximum_response_bytes = maximum;
        self
    }

    pub fn endpoint(&self) -> &NyxTransportEndpoint {
        &self.endpoint
    }

    pub fn supported_versions(&self) -> &[NyxProtocolVersion] {
        &self.supported_versions
    }

    pub const fn io_timeout(&self) -> Duration {
        self.io_timeout
    }

    pub const fn maximum_response_bytes(&self) -> usize {
        self.maximum_response_bytes
    }

    pub(crate) fn preferred_version(&self) -> NyxProtocolVersion {
        *self
            .supported_versions
            .last()
            .expect("validated Nyx supported version set")
    }

    pub(crate) fn supports_major(&self, major: u16) -> bool {
        self.supported_versions
            .iter()
            .any(|version| version.major() == major)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NyxProbeStatus {
    Ready,
    Unavailable,
    Incompatible,
    Unhealthy,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NyxProbeOutcome {
    Ready { response: NyxServiceReport },
    Unavailable { reason: NyxUnavailableReason },
    Incompatible { reason: NyxIncompatibility },
    Unhealthy { response: NyxServiceReport },
}

impl NyxProbeOutcome {
    pub const fn status(&self) -> NyxProbeStatus {
        match self {
            Self::Ready { .. } => NyxProbeStatus::Ready,
            Self::Unavailable { .. } => NyxProbeStatus::Unavailable,
            Self::Incompatible { .. } => NyxProbeStatus::Incompatible,
            Self::Unhealthy { .. } => NyxProbeStatus::Unhealthy,
        }
    }

    pub fn response(&self) -> Option<&NyxServiceReport> {
        match self {
            Self::Ready { response } | Self::Unhealthy { response } => Some(response),
            Self::Unavailable { .. } | Self::Incompatible { .. } => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NyxUnavailableReason {
    UnsupportedTransport,
    ConnectFailed(io::ErrorKind),
    WriteFailed(io::ErrorKind),
    ReadFailed(io::ErrorKind),
    ResponseTooLarge { maximum: usize, actual: usize },
    HttpStatus { path: &'static str, status: u16 },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NyxIncompatibility {
    MalformedResponse {
        path: &'static str,
        detail: String,
    },
    MissingContractHeader {
        path: &'static str,
    },
    ContractHeaderMismatch {
        path: &'static str,
        header: NyxProtocolVersion,
        body: NyxProtocolVersion,
    },
    RejectedContract {
        requested: NyxProtocolVersion,
        supported: Vec<NyxProtocolVersion>,
    },
    UnsupportedNegotiatedProtocol {
        requested: NyxProtocolVersion,
        negotiated: NyxProtocolVersion,
    },
}

pub fn probe_nyx(config: &NyxClientConfig) -> NyxProbeOutcome {
    match probe_nyx_inner(config) {
        Ok(response) if response.is_ready() => NyxProbeOutcome::Ready { response },
        Ok(response) => NyxProbeOutcome::Unhealthy { response },
        Err(NyxJsonRequestFailure::Unavailable(reason)) => NyxProbeOutcome::Unavailable { reason },
        Err(NyxJsonRequestFailure::Incompatible(reason)) => {
            NyxProbeOutcome::Incompatible { reason }
        }
    }
}

fn probe_nyx_inner(config: &NyxClientConfig) -> Result<NyxServiceReport, NyxJsonRequestFailure> {
    let requested = config.preferred_version();

    let version_response = fetch_contract(config, VERSION_PATH, requested)?;
    let version =
        decode_version(&version_response.body).map_err(|error| malformed(VERSION_PATH, error))?;
    verify_header(
        VERSION_PATH,
        version_response.contract_version,
        version.public_contract_version,
    )?;
    if !config.supports_major(version.public_contract_version.major())
        || !version
            .compatible_major_versions
            .contains(&requested.major())
    {
        return Err(NyxJsonRequestFailure::Incompatible(
            NyxIncompatibility::UnsupportedNegotiatedProtocol {
                requested,
                negotiated: version.public_contract_version,
            },
        ));
    }

    let health_response = fetch_contract(config, HEALTH_PATH, requested)?;
    let health =
        decode_health(&health_response.body).map_err(|error| malformed(HEALTH_PATH, error))?;
    verify_header(
        HEALTH_PATH,
        health_response.contract_version,
        health.public_contract_version,
    )?;

    let capability_response = fetch_contract(config, CAPABILITIES_PATH, requested)?;
    let capabilities = decode_capabilities(&capability_response.body)
        .map_err(|error| malformed(CAPABILITIES_PATH, error))?;
    verify_header(
        CAPABILITIES_PATH,
        capability_response.contract_version,
        capabilities.public_contract_version,
    )?;

    assemble_report(version, health, capabilities)
        .map_err(|error| malformed(CAPABILITIES_PATH, error))
}

fn fetch_contract(
    config: &NyxClientConfig,
    path: &'static str,
    requested: NyxProtocolVersion,
) -> Result<ContractResponse, NyxJsonRequestFailure> {
    let response = request_json(config, NyxHttpMethod::Get, path, path, requested, None)?;
    if response.status != 200 {
        return Err(NyxJsonRequestFailure::Unavailable(
            NyxUnavailableReason::HttpStatus {
                path,
                status: response.status,
            },
        ));
    }
    Ok(ContractResponse {
        contract_version: response.contract_version,
        body: response.body,
    })
}

pub(crate) fn request_json(
    config: &NyxClientConfig,
    method: NyxHttpMethod,
    path_label: &'static str,
    path: &str,
    requested: NyxProtocolVersion,
    body: Option<&[u8]>,
) -> Result<NyxJsonResponse, NyxJsonRequestFailure> {
    let request = build_json_request(config, method, path, requested, body);
    let bytes =
        connect_and_exchange(config, &request).map_err(NyxJsonRequestFailure::Unavailable)?;
    let response = parse_http_response(&bytes).map_err(|detail| {
        NyxJsonRequestFailure::Incompatible(NyxIncompatibility::MalformedResponse {
            path: path_label,
            detail,
        })
    })?;
    let contract_text = response
        .headers
        .get(HEADER_NYX_CONTRACT_VERSION)
        .ok_or_else(|| {
            NyxJsonRequestFailure::Incompatible(NyxIncompatibility::MissingContractHeader {
                path: path_label,
            })
        })?;
    let contract_version =
        NyxProtocolVersion::parse(contract_text).map_err(|error| malformed(path_label, error))?;
    if response.status == 426 {
        let supported = decode_incompatible_versions(&response.body)
            .map_err(|error| malformed(path_label, error))?;
        return Err(NyxJsonRequestFailure::Incompatible(
            NyxIncompatibility::RejectedContract {
                requested,
                supported,
            },
        ));
    }
    if !config.supports_major(contract_version.major()) {
        return Err(NyxJsonRequestFailure::Incompatible(
            NyxIncompatibility::UnsupportedNegotiatedProtocol {
                requested,
                negotiated: contract_version,
            },
        ));
    }
    if let Some(content_type) = response.headers.get("content-type") {
        if !content_type
            .split(';')
            .next()
            .is_some_and(|value| value.trim() == "application/json")
        {
            return Err(NyxJsonRequestFailure::Incompatible(
                NyxIncompatibility::MalformedResponse {
                    path: path_label,
                    detail: format!("unexpected content type '{content_type}'"),
                },
            ));
        }
    }
    Ok(NyxJsonResponse {
        status: response.status,
        contract_version,
        body: response.body,
    })
}

fn verify_header(
    path: &'static str,
    header: NyxProtocolVersion,
    body: NyxProtocolVersion,
) -> Result<(), NyxJsonRequestFailure> {
    if header != body {
        return Err(NyxJsonRequestFailure::Incompatible(
            NyxIncompatibility::ContractHeaderMismatch { path, header, body },
        ));
    }
    Ok(())
}

fn malformed(path: &'static str, error: NyxProtocolError) -> NyxJsonRequestFailure {
    NyxJsonRequestFailure::Incompatible(NyxIncompatibility::MalformedResponse {
        path,
        detail: error.to_string(),
    })
}

fn build_json_request(
    config: &NyxClientConfig,
    method: NyxHttpMethod,
    path: &str,
    requested: NyxProtocolVersion,
    body: Option<&[u8]>,
) -> Vec<u8> {
    let body = body.unwrap_or_default();
    let mut request = format!(
        "{} {path} HTTP/1.1\r\nHost: {}\r\nAccept: application/json\r\n{}: {requested}\r\n",
        method.as_str(),
        config.endpoint.host_header(),
        HEADER_NYX_CONTRACT_VERSION,
    );
    if matches!(method, NyxHttpMethod::Post) {
        request.push_str("Content-Type: application/json\r\n");
        request.push_str(&format!("Content-Length: {}\r\n", body.len()));
    }
    request.push_str("Connection: close\r\n\r\n");
    let mut bytes = request.into_bytes();
    bytes.extend_from_slice(body);
    bytes
}

fn connect_and_exchange(
    config: &NyxClientConfig,
    request_bytes: &[u8],
) -> Result<Vec<u8>, NyxUnavailableReason> {
    match &config.endpoint {
        NyxTransportEndpoint::Tcp(address) => {
            let stream = TcpStream::connect_timeout(address, config.io_timeout)
                .map_err(|error| NyxUnavailableReason::ConnectFailed(error.kind()))?;
            configure_tcp(&stream, config.io_timeout)?;
            exchange(stream, request_bytes, config.maximum_response_bytes)
        }
        NyxTransportEndpoint::UnixSocket(path) => connect_unix(config, path, request_bytes),
    }
}

fn configure_tcp(stream: &TcpStream, timeout: Duration) -> Result<(), NyxUnavailableReason> {
    stream
        .set_read_timeout(Some(timeout))
        .and_then(|_| stream.set_write_timeout(Some(timeout)))
        .map_err(|error| NyxUnavailableReason::ConnectFailed(error.kind()))
}

#[cfg(unix)]
fn connect_unix(
    config: &NyxClientConfig,
    path: &PathBuf,
    request_bytes: &[u8],
) -> Result<Vec<u8>, NyxUnavailableReason> {
    let stream = UnixStream::connect(path)
        .map_err(|error| NyxUnavailableReason::ConnectFailed(error.kind()))?;
    stream
        .set_read_timeout(Some(config.io_timeout))
        .and_then(|_| stream.set_write_timeout(Some(config.io_timeout)))
        .map_err(|error| NyxUnavailableReason::ConnectFailed(error.kind()))?;
    exchange(stream, request_bytes, config.maximum_response_bytes)
}

#[cfg(not(unix))]
fn connect_unix(
    _config: &NyxClientConfig,
    _path: &PathBuf,
    _request_bytes: &[u8],
) -> Result<Vec<u8>, NyxUnavailableReason> {
    Err(NyxUnavailableReason::UnsupportedTransport)
}

fn exchange(
    mut stream: impl Read + Write,
    request_bytes: &[u8],
    maximum_response_bytes: usize,
) -> Result<Vec<u8>, NyxUnavailableReason> {
    stream
        .write_all(request_bytes)
        .and_then(|_| stream.flush())
        .map_err(|error| NyxUnavailableReason::WriteFailed(error.kind()))?;
    read_limited(&mut stream, maximum_response_bytes)
}

fn read_limited(
    stream: &mut impl Read,
    maximum_response_bytes: usize,
) -> Result<Vec<u8>, NyxUnavailableReason> {
    let mut bytes = Vec::new();
    let mut buffer = [0_u8; 8192];
    loop {
        let count = stream
            .read(&mut buffer)
            .map_err(|error| NyxUnavailableReason::ReadFailed(error.kind()))?;
        if count == 0 {
            break;
        }
        let actual = bytes.len().saturating_add(count);
        if actual > maximum_response_bytes {
            return Err(NyxUnavailableReason::ResponseTooLarge {
                maximum: maximum_response_bytes,
                actual,
            });
        }
        bytes.extend_from_slice(&buffer[..count]);
    }
    Ok(bytes)
}

fn parse_http_response(bytes: &[u8]) -> Result<HttpResponse, String> {
    let separator = bytes
        .windows(4)
        .position(|window| window == b"\r\n\r\n")
        .ok_or_else(|| "HTTP header terminator is missing".to_owned())?;
    let header_bytes = &bytes[..separator];
    let body_start = separator + 4;
    let header_text = std::str::from_utf8(header_bytes)
        .map_err(|error| format!("HTTP headers are not UTF-8: {error}"))?;
    let mut lines = header_text.split("\r\n");
    let status_line = lines
        .next()
        .ok_or_else(|| "HTTP status line is missing".to_owned())?;
    let mut status_parts = status_line.split_whitespace();
    let protocol = status_parts
        .next()
        .ok_or_else(|| "HTTP protocol is missing".to_owned())?;
    if protocol != "HTTP/1.1" && protocol != "HTTP/1.0" {
        return Err(format!("unsupported HTTP protocol '{protocol}'"));
    }
    let status = status_parts
        .next()
        .ok_or_else(|| "HTTP status code is missing".to_owned())?
        .parse::<u16>()
        .map_err(|_| "HTTP status code is invalid".to_owned())?;

    let mut headers = BTreeMap::new();
    for line in lines {
        let (name, value) = line
            .split_once(':')
            .ok_or_else(|| format!("malformed HTTP header '{line}'"))?;
        let name = name.trim().to_ascii_lowercase();
        let value = value.trim().to_owned();
        if name.is_empty() || headers.insert(name.clone(), value).is_some() {
            return Err(format!("invalid or duplicate HTTP header '{name}'"));
        }
    }

    if let Some(encoding) = headers.get("transfer-encoding") {
        if encoding != "identity" {
            return Err(format!("unsupported transfer encoding '{encoding}'"));
        }
    }

    let body = bytes[body_start..].to_vec();
    if let Some(content_length) = headers.get("content-length") {
        let expected = content_length
            .parse::<usize>()
            .map_err(|_| "invalid content-length header".to_owned())?;
        if expected != body.len() {
            return Err(format!(
                "content-length is {expected}, received {} body bytes",
                body.len()
            ));
        }
    }

    Ok(HttpResponse {
        status,
        headers,
        body,
    })
}

#[derive(Debug)]
struct HttpResponse {
    status: u16,
    headers: BTreeMap<String, String>,
    body: Vec<u8>,
}

#[derive(Debug)]
struct ContractResponse {
    contract_version: NyxProtocolVersion,
    body: Vec<u8>,
}

#[derive(Debug)]
pub(crate) enum NyxJsonRequestFailure {
    Unavailable(NyxUnavailableReason),
    Incompatible(NyxIncompatibility),
}
