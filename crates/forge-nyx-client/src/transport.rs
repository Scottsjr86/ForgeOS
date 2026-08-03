//! Configured Nyx transport discovery and compatibility probing.
//!
//! The probe exchanges one framed handshake over a configured local Unix socket
//! or TCP endpoint. It returns typed unavailable, incompatible, unhealthy, or
//! ready outcomes and never treats transport success as protocol compatibility.

use crate::protocol::{
    NyxHandshakeRequest, NyxHandshakeResponse, NyxProtocolError, NyxProtocolVersion,
};
use std::fmt;
use std::io::{self, Read, Write};
use std::net::{SocketAddr, TcpStream};
use std::path::PathBuf;
use std::time::Duration;

#[cfg(unix)]
use std::os::unix::net::UnixStream;

const DEFAULT_IO_TIMEOUT: Duration = Duration::from_secs(2);
const DEFAULT_MAX_RESPONSE_BYTES: usize = 1024 * 1024;

/// Explicit configured route to the separate Nyx service.
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
}

impl fmt::Display for NyxTransportEndpoint {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnixSocket(path) => write!(formatter, "unix:{}", path.display()),
            Self::Tcp(address) => write!(formatter, "tcp:{address}"),
        }
    }
}

/// Complete bounded configuration for one Nyx compatibility probe.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NyxClientConfig {
    endpoint: NyxTransportEndpoint,
    request: NyxHandshakeRequest,
    io_timeout: Duration,
    maximum_response_bytes: usize,
}

impl NyxClientConfig {
    pub fn new(
        endpoint: NyxTransportEndpoint,
        supported_versions: impl IntoIterator<Item = NyxProtocolVersion>,
    ) -> Result<Self, NyxProtocolError> {
        Ok(Self {
            endpoint,
            request: NyxHandshakeRequest::new(supported_versions)?,
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

    pub fn request(&self) -> &NyxHandshakeRequest {
        &self.request
    }

    pub const fn io_timeout(&self) -> Duration {
        self.io_timeout
    }

    pub const fn maximum_response_bytes(&self) -> usize {
        self.maximum_response_bytes
    }
}

/// Stable high-level classification shown to the ForgeOS shell.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NyxProbeStatus {
    Ready,
    Unavailable,
    Incompatible,
    Unhealthy,
}

/// One complete result of probing the configured Nyx endpoint.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NyxProbeOutcome {
    Ready { response: NyxHandshakeResponse },
    Unavailable { reason: NyxUnavailableReason },
    Incompatible { reason: NyxIncompatibility },
    Unhealthy { response: NyxHandshakeResponse },
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

    pub fn response(&self) -> Option<&NyxHandshakeResponse> {
        match self {
            Self::Ready { response } | Self::Unhealthy { response } => Some(response),
            Self::Unavailable { .. } | Self::Incompatible { .. } => None,
        }
    }
}

/// Transport-level reason Nyx could not be reached or did not complete a frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NyxUnavailableReason {
    UnsupportedTransport,
    ConnectFailed(io::ErrorKind),
    WriteFailed(io::ErrorKind),
    ReadFailed(io::ErrorKind),
    ResponseFrameTooLarge { maximum: usize, actual: usize },
}

/// A responding endpoint that cannot speak the offered Nyx protocol.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NyxIncompatibility {
    MalformedResponse(NyxProtocolError),
    UnsupportedSelectedProtocol { selected: NyxProtocolVersion },
}

/// Probes exactly one configured endpoint and classifies the result without
/// panicking or bypassing Nyx to contact a model provider directly.
pub fn probe_nyx(config: &NyxClientConfig) -> NyxProbeOutcome {
    let request_bytes = config.request.encode();
    let response_bytes = match connect_and_exchange(config, &request_bytes) {
        Ok(bytes) => bytes,
        Err(reason) => return NyxProbeOutcome::Unavailable { reason },
    };
    let response = match NyxHandshakeResponse::decode(&response_bytes) {
        Ok(response) => response,
        Err(error) => {
            return NyxProbeOutcome::Incompatible {
                reason: NyxIncompatibility::MalformedResponse(error),
            }
        }
    };
    if !config.request.supports(response.selected_protocol()) {
        return NyxProbeOutcome::Incompatible {
            reason: NyxIncompatibility::UnsupportedSelectedProtocol {
                selected: response.selected_protocol(),
            },
        };
    }
    if response.health().is_healthy() {
        NyxProbeOutcome::Ready { response }
    } else {
        NyxProbeOutcome::Unhealthy { response }
    }
}

fn connect_and_exchange(
    config: &NyxClientConfig,
    request_bytes: &[u8],
) -> Result<Vec<u8>, NyxUnavailableReason> {
    match &config.endpoint {
        NyxTransportEndpoint::Tcp(address) => {
            let stream = TcpStream::connect_timeout(address, config.io_timeout)
                .map_err(|error| NyxUnavailableReason::ConnectFailed(error.kind()))?;
            stream
                .set_read_timeout(Some(config.io_timeout))
                .map_err(|error| NyxUnavailableReason::ConnectFailed(error.kind()))?;
            stream
                .set_write_timeout(Some(config.io_timeout))
                .map_err(|error| NyxUnavailableReason::ConnectFailed(error.kind()))?;
            exchange(stream, request_bytes, config.maximum_response_bytes)
        }
        NyxTransportEndpoint::UnixSocket(path) => connect_unix(config, path, request_bytes),
    }
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
        .map_err(|error| NyxUnavailableReason::ConnectFailed(error.kind()))?;
    stream
        .set_write_timeout(Some(config.io_timeout))
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
    write_frame(&mut stream, request_bytes)?;
    read_frame(&mut stream, maximum_response_bytes)
}

fn write_frame(stream: &mut impl Write, payload: &[u8]) -> Result<(), NyxUnavailableReason> {
    let length = u32::try_from(payload.len())
        .map_err(|_| NyxUnavailableReason::WriteFailed(io::ErrorKind::InvalidInput))?;
    stream
        .write_all(&length.to_be_bytes())
        .and_then(|_| stream.write_all(payload))
        .and_then(|_| stream.flush())
        .map_err(|error| NyxUnavailableReason::WriteFailed(error.kind()))
}

fn read_frame(
    stream: &mut impl Read,
    maximum_response_bytes: usize,
) -> Result<Vec<u8>, NyxUnavailableReason> {
    let mut length_bytes = [0_u8; 4];
    stream
        .read_exact(&mut length_bytes)
        .map_err(|error| NyxUnavailableReason::ReadFailed(error.kind()))?;
    let actual = usize::try_from(u32::from_be_bytes(length_bytes))
        .map_err(|_| NyxUnavailableReason::ReadFailed(io::ErrorKind::InvalidData))?;
    if actual > maximum_response_bytes {
        return Err(NyxUnavailableReason::ResponseFrameTooLarge {
            maximum: maximum_response_bytes,
            actual,
        });
    }
    let mut payload = vec![0_u8; actual];
    stream
        .read_exact(&mut payload)
        .map_err(|error| NyxUnavailableReason::ReadFailed(error.kind()))?;
    Ok(payload)
}
