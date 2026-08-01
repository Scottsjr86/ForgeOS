//! Deterministic versioned request, result, and error envelopes.

use crate::errors::{
    decode_protocol_error, encode_protocol_error, malformed_from_wire, EnvelopeViolation,
    ProtocolError,
};
use crate::identities::{CommandId, ResultId, TaskId, IDENTITY_BYTES};
use crate::wire::{Decoder, Encoder};

const MAGIC: [u8; 4] = *b"FGOS";
/// Current published V1 envelope schema version.
pub const CURRENT_PROTOCOL_VERSION: u16 = 1;
/// Maximum payload length accepted by V1 request and result envelopes.
pub const MAX_ENVELOPE_PAYLOAD_BYTES: u32 = 16 * 1024 * 1024;

/// Stable envelope discriminator.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum EnvelopeKind {
    Request = 1,
    Result = 2,
    Error = 3,
}

impl EnvelopeKind {
    /// Stable V1 wire code.
    pub const fn code(self) -> u8 {
        self as u8
    }

    /// Stable diagnostic label.
    pub const fn label(self) -> &'static str {
        match self {
            Self::Request => "request",
            Self::Result => "result",
            Self::Error => "error",
        }
    }

    pub(crate) const fn from_code(code: u8) -> Option<Self> {
        match code {
            1 => Some(Self::Request),
            2 => Some(Self::Result),
            3 => Some(Self::Error),
            _ => None,
        }
    }
}

/// Versioned command request correlated to one stable task.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RequestEnvelope {
    task_id: TaskId,
    command_id: CommandId,
    payload: Vec<u8>,
}

impl RequestEnvelope {
    /// Creates a current-version request envelope.
    pub fn new(
        task_id: TaskId,
        command_id: CommandId,
        payload: impl Into<Vec<u8>>,
    ) -> Result<Self, ProtocolError> {
        let payload = payload.into();
        validate_payload(EnvelopeKind::Request, payload.len())?;
        Ok(Self {
            task_id,
            command_id,
            payload,
        })
    }

    /// Published wire version.
    pub const fn version(&self) -> u16 {
        CURRENT_PROTOCOL_VERSION
    }

    /// Stable task correlation identity.
    pub const fn task_id(&self) -> TaskId {
        self.task_id
    }

    /// Stable command identity.
    pub const fn command_id(&self) -> CommandId {
        self.command_id
    }

    /// Opaque canonical request payload bytes.
    pub fn payload(&self) -> &[u8] {
        &self.payload
    }

    /// Encodes the exact deterministic V1 wire representation.
    pub fn encode(&self) -> Vec<u8> {
        let mut encoder = Encoder::new();
        encode_header(EnvelopeKind::Request, &mut encoder);
        encoder.write_bytes(self.task_id.as_bytes());
        encoder.write_bytes(self.command_id.as_bytes());
        encode_payload(&self.payload, &mut encoder);
        encoder.finish()
    }

    /// Decodes only the published current request version.
    pub fn decode(bytes: &[u8]) -> Result<Self, ProtocolError> {
        let mut decoder = Decoder::new(bytes);
        decode_header(EnvelopeKind::Request, &mut decoder)?;
        let task_id = TaskId::from_bytes(read_identity(EnvelopeKind::Request, &mut decoder)?);
        let command_id = CommandId::from_bytes(read_identity(EnvelopeKind::Request, &mut decoder)?);
        let payload = decode_payload(EnvelopeKind::Request, &mut decoder)?;
        decoder
            .finish()
            .map_err(|error| malformed_from_wire(EnvelopeKind::Request, error))?;
        Ok(Self {
            task_id,
            command_id,
            payload,
        })
    }
}

/// Versioned successful result correlated to one task.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResultEnvelope {
    task_id: TaskId,
    result_id: ResultId,
    payload: Vec<u8>,
}

impl ResultEnvelope {
    /// Creates a current-version result envelope.
    pub fn new(
        task_id: TaskId,
        result_id: ResultId,
        payload: impl Into<Vec<u8>>,
    ) -> Result<Self, ProtocolError> {
        let payload = payload.into();
        validate_payload(EnvelopeKind::Result, payload.len())?;
        Ok(Self {
            task_id,
            result_id,
            payload,
        })
    }

    /// Published wire version.
    pub const fn version(&self) -> u16 {
        CURRENT_PROTOCOL_VERSION
    }

    /// Stable task correlation identity.
    pub const fn task_id(&self) -> TaskId {
        self.task_id
    }

    /// Stable result identity.
    pub const fn result_id(&self) -> ResultId {
        self.result_id
    }

    /// Opaque canonical result payload bytes.
    pub fn payload(&self) -> &[u8] {
        &self.payload
    }

    /// Encodes the exact deterministic V1 wire representation.
    pub fn encode(&self) -> Vec<u8> {
        let mut encoder = Encoder::new();
        encode_header(EnvelopeKind::Result, &mut encoder);
        encoder.write_bytes(self.task_id.as_bytes());
        encoder.write_bytes(self.result_id.as_bytes());
        encode_payload(&self.payload, &mut encoder);
        encoder.finish()
    }

    /// Decodes only the published current result version.
    pub fn decode(bytes: &[u8]) -> Result<Self, ProtocolError> {
        let mut decoder = Decoder::new(bytes);
        decode_header(EnvelopeKind::Result, &mut decoder)?;
        let task_id = TaskId::from_bytes(read_identity(EnvelopeKind::Result, &mut decoder)?);
        let result_id = ResultId::from_bytes(read_identity(EnvelopeKind::Result, &mut decoder)?);
        let payload = decode_payload(EnvelopeKind::Result, &mut decoder)?;
        decoder
            .finish()
            .map_err(|error| malformed_from_wire(EnvelopeKind::Result, error))?;
        Ok(Self {
            task_id,
            result_id,
            payload,
        })
    }
}

/// Versioned typed failure correlated to one task.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ErrorEnvelope {
    task_id: TaskId,
    error: ProtocolError,
}

impl ErrorEnvelope {
    /// Creates a current-version typed error envelope.
    pub fn new(task_id: TaskId, error: ProtocolError) -> Self {
        Self { task_id, error }
    }

    /// Published wire version.
    pub const fn version(&self) -> u16 {
        CURRENT_PROTOCOL_VERSION
    }

    /// Stable task correlation identity.
    pub const fn task_id(&self) -> TaskId {
        self.task_id
    }

    /// Typed protocol failure.
    pub fn error(&self) -> &ProtocolError {
        &self.error
    }

    /// Encodes the exact deterministic V1 wire representation.
    pub fn encode(&self) -> Vec<u8> {
        let mut encoder = Encoder::new();
        encode_header(EnvelopeKind::Error, &mut encoder);
        encoder.write_bytes(self.task_id.as_bytes());
        encode_protocol_error(&self.error, &mut encoder);
        encoder.finish()
    }

    /// Decodes only the published current error version.
    pub fn decode(bytes: &[u8]) -> Result<Self, ProtocolError> {
        let mut decoder = Decoder::new(bytes);
        decode_header(EnvelopeKind::Error, &mut decoder)?;
        let task_id = TaskId::from_bytes(read_identity(EnvelopeKind::Error, &mut decoder)?);
        let error = decode_protocol_error(&mut decoder)?;
        decoder
            .finish()
            .map_err(|error| malformed_from_wire(EnvelopeKind::Error, error))?;
        Ok(Self { task_id, error })
    }
}

fn encode_header(kind: EnvelopeKind, encoder: &mut Encoder) {
    encoder.write_bytes(&MAGIC);
    encoder.write_u16(CURRENT_PROTOCOL_VERSION);
    encoder.write_u8(kind.code());
}

fn decode_header(kind: EnvelopeKind, decoder: &mut Decoder<'_>) -> Result<(), ProtocolError> {
    let magic = decoder
        .read_array::<4>()
        .map_err(|error| malformed_from_wire(kind, error))?;
    if magic != MAGIC {
        return Err(ProtocolError::MalformedEnvelope {
            envelope: kind,
            violation: EnvelopeViolation::BadMagic,
        });
    }

    let version = decoder
        .read_u16()
        .map_err(|error| malformed_from_wire(kind, error))?;
    if version != CURRENT_PROTOCOL_VERSION {
        return Err(ProtocolError::UnsupportedVersion {
            envelope: kind,
            found: version,
            supported: CURRENT_PROTOCOL_VERSION,
        });
    }

    let found_kind = decoder
        .read_u8()
        .map_err(|error| malformed_from_wire(kind, error))?;
    if found_kind != kind.code() {
        return Err(ProtocolError::MalformedEnvelope {
            envelope: kind,
            violation: EnvelopeViolation::WrongKind { found: found_kind },
        });
    }
    Ok(())
}

fn read_identity(
    kind: EnvelopeKind,
    decoder: &mut Decoder<'_>,
) -> Result<[u8; IDENTITY_BYTES], ProtocolError> {
    decoder
        .read_array()
        .map_err(|error| malformed_from_wire(kind, error))
}

fn validate_payload(kind: EnvelopeKind, length: usize) -> Result<(), ProtocolError> {
    if length > MAX_ENVELOPE_PAYLOAD_BYTES as usize {
        return Err(ProtocolError::PayloadTooLarge {
            envelope: kind,
            length: length as u64,
            maximum: MAX_ENVELOPE_PAYLOAD_BYTES,
        });
    }
    Ok(())
}

fn encode_payload(payload: &[u8], encoder: &mut Encoder) {
    let length = u32::try_from(payload.len()).expect("validated payload length exceeds u32");
    encoder.write_u32(length);
    encoder.write_bytes(payload);
}

fn decode_payload(
    kind: EnvelopeKind,
    decoder: &mut Decoder<'_>,
) -> Result<Vec<u8>, ProtocolError> {
    let length = decoder
        .read_u32()
        .map_err(|error| malformed_from_wire(kind, error))?;
    if length > MAX_ENVELOPE_PAYLOAD_BYTES {
        return Err(ProtocolError::PayloadTooLarge {
            envelope: kind,
            length: u64::from(length),
            maximum: MAX_ENVELOPE_PAYLOAD_BYTES,
        });
    }
    decoder
        .read_bytes(length)
        .map_err(|error| malformed_from_wire(kind, error))
}
