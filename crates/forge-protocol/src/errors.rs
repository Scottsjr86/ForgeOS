//! Typed protocol errors with stable V1 wire meanings.

use std::fmt;

use crate::envelopes::EnvelopeKind;
use crate::identities::{
    DuplicateIdentity, IdentityKind, IdentityParseError, IdentityParseFailure,
    canonical_text_is_valid,
};
use crate::wire::{Decoder, Encoder, WireError};

/// Stable V1 protocol error codes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u16)]
pub enum ProtocolErrorCode {
    UnsupportedVersion = 1,
    DuplicateIdentity = 2,
    InvalidIdentity = 3,
    MalformedEnvelope = 4,
    PayloadTooLarge = 5,
}

impl ProtocolErrorCode {
    /// Stable V1 wire code.
    pub const fn code(self) -> u16 {
        self as u16
    }

    fn from_code(code: u16) -> Option<Self> {
        match code {
            1 => Some(Self::UnsupportedVersion),
            2 => Some(Self::DuplicateIdentity),
            3 => Some(Self::InvalidIdentity),
            4 => Some(Self::MalformedEnvelope),
            5 => Some(Self::PayloadTooLarge),
            _ => None,
        }
    }
}

/// Typed structural envelope violation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EnvelopeViolation {
    BadMagic,
    WrongKind { found: u8 },
    UnexpectedEnd,
    TrailingBytes { remaining: u32 },
    UnknownErrorCode { found: u16 },
    UnknownIdentityKind { found: u8 },
    InvalidUtf8,
    InvalidCanonicalIdentity,
    LengthOverflow { length: u64 },
}

/// Canonical V1 protocol failure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProtocolError {
    UnsupportedVersion {
        envelope: EnvelopeKind,
        found: u16,
        supported: u16,
    },
    DuplicateIdentity(DuplicateIdentity),
    InvalidIdentity(IdentityParseError),
    MalformedEnvelope {
        envelope: EnvelopeKind,
        violation: EnvelopeViolation,
    },
    PayloadTooLarge {
        envelope: EnvelopeKind,
        length: u64,
        maximum: u32,
    },
}

impl ProtocolError {
    /// Stable typed error code.
    pub fn code(&self) -> ProtocolErrorCode {
        match self {
            Self::UnsupportedVersion { .. } => ProtocolErrorCode::UnsupportedVersion,
            Self::DuplicateIdentity(_) => ProtocolErrorCode::DuplicateIdentity,
            Self::InvalidIdentity(_) => ProtocolErrorCode::InvalidIdentity,
            Self::MalformedEnvelope { .. } => ProtocolErrorCode::MalformedEnvelope,
            Self::PayloadTooLarge { .. } => ProtocolErrorCode::PayloadTooLarge,
        }
    }
}

impl From<DuplicateIdentity> for ProtocolError {
    fn from(value: DuplicateIdentity) -> Self {
        Self::DuplicateIdentity(value)
    }
}

impl From<IdentityParseError> for ProtocolError {
    fn from(value: IdentityParseError) -> Self {
        Self::InvalidIdentity(value)
    }
}

impl fmt::Display for ProtocolError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedVersion {
                envelope,
                found,
                supported,
            } => write!(
                formatter,
                "unsupported {} envelope version {found}; supported version is {supported}",
                envelope.label()
            ),
            Self::DuplicateIdentity(error) => fmt::Display::fmt(error, formatter),
            Self::InvalidIdentity(error) => fmt::Display::fmt(error, formatter),
            Self::MalformedEnvelope {
                envelope,
                violation,
            } => write!(
                formatter,
                "malformed {} envelope: {violation:?}",
                envelope.label()
            ),
            Self::PayloadTooLarge {
                envelope,
                length,
                maximum,
            } => write!(
                formatter,
                "{} envelope payload length {length} exceeds maximum {maximum}",
                envelope.label()
            ),
        }
    }
}

impl std::error::Error for ProtocolError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::DuplicateIdentity(error) => Some(error),
            Self::InvalidIdentity(error) => Some(error),
            Self::UnsupportedVersion { .. }
            | Self::MalformedEnvelope { .. }
            | Self::PayloadTooLarge { .. } => None,
        }
    }
}

pub(crate) fn encode_protocol_error(error: &ProtocolError, encoder: &mut Encoder) {
    encoder.write_u16(error.code().code());
    match error {
        ProtocolError::UnsupportedVersion {
            envelope,
            found,
            supported,
        } => {
            encoder.write_u8(envelope.code());
            encoder.write_u16(*found);
            encoder.write_u16(*supported);
        }
        ProtocolError::DuplicateIdentity(error) => {
            encoder.write_u8(error.kind().code());
            encoder.write_text(error.canonical());
        }
        ProtocolError::InvalidIdentity(error) => {
            encoder.write_u8(error.kind().code());
            match error.failure() {
                IdentityParseFailure::InvalidLength { found } => {
                    encoder.write_u8(1);
                    encoder.write_u64(*found as u64);
                }
                IdentityParseFailure::NonCanonicalHex { index, byte } => {
                    encoder.write_u8(2);
                    encoder.write_u64(*index as u64);
                    encoder.write_u8(*byte);
                }
            }
        }
        ProtocolError::MalformedEnvelope {
            envelope,
            violation,
        } => {
            encoder.write_u8(envelope.code());
            encode_violation(violation, encoder);
        }
        ProtocolError::PayloadTooLarge {
            envelope,
            length,
            maximum,
        } => {
            encoder.write_u8(envelope.code());
            encoder.write_u64(*length);
            encoder.write_u32(*maximum);
        }
    }
}

pub(crate) fn decode_protocol_error(
    decoder: &mut Decoder<'_>,
) -> Result<ProtocolError, ProtocolError> {
    let code = decoder
        .read_u16()
        .map_err(|error| malformed_from_wire(EnvelopeKind::Error, error))?;
    let code = ProtocolErrorCode::from_code(code).ok_or(ProtocolError::MalformedEnvelope {
        envelope: EnvelopeKind::Error,
        violation: EnvelopeViolation::UnknownErrorCode { found: code },
    })?;

    match code {
        ProtocolErrorCode::UnsupportedVersion => {
            let envelope = decode_envelope_kind(decoder)?;
            let found = decoder
                .read_u16()
                .map_err(|error| malformed_from_wire(EnvelopeKind::Error, error))?;
            let supported = decoder
                .read_u16()
                .map_err(|error| malformed_from_wire(EnvelopeKind::Error, error))?;
            Ok(ProtocolError::UnsupportedVersion {
                envelope,
                found,
                supported,
            })
        }
        ProtocolErrorCode::DuplicateIdentity => {
            let kind = decode_identity_kind(decoder)?;
            let canonical = decoder
                .read_text()
                .map_err(|error| malformed_from_wire(EnvelopeKind::Error, error))?;
            if !canonical_text_is_valid(&canonical) {
                return Err(ProtocolError::MalformedEnvelope {
                    envelope: EnvelopeKind::Error,
                    violation: EnvelopeViolation::InvalidCanonicalIdentity,
                });
            }
            Ok(ProtocolError::DuplicateIdentity(
                DuplicateIdentity::from_parts(kind, canonical),
            ))
        }
        ProtocolErrorCode::InvalidIdentity => {
            let kind = decode_identity_kind(decoder)?;
            let failure = match decoder
                .read_u8()
                .map_err(|error| malformed_from_wire(EnvelopeKind::Error, error))?
            {
                1 => IdentityParseFailure::InvalidLength {
                    found: usize::try_from(
                        decoder
                            .read_u64()
                            .map_err(|error| malformed_from_wire(EnvelopeKind::Error, error))?,
                    )
                    .map_err(|_| ProtocolError::MalformedEnvelope {
                        envelope: EnvelopeKind::Error,
                        violation: EnvelopeViolation::LengthOverflow { length: u64::MAX },
                    })?,
                },
                2 => {
                    let raw_index = decoder
                        .read_u64()
                        .map_err(|error| malformed_from_wire(EnvelopeKind::Error, error))?;
                    let index = usize::try_from(raw_index).map_err(|_| {
                        ProtocolError::MalformedEnvelope {
                            envelope: EnvelopeKind::Error,
                            violation: EnvelopeViolation::LengthOverflow { length: raw_index },
                        }
                    })?;
                    let byte = decoder
                        .read_u8()
                        .map_err(|error| malformed_from_wire(EnvelopeKind::Error, error))?;
                    IdentityParseFailure::NonCanonicalHex { index, byte }
                }
                found => {
                    return Err(ProtocolError::MalformedEnvelope {
                        envelope: EnvelopeKind::Error,
                        violation: EnvelopeViolation::UnknownErrorCode {
                            found: u16::from(found),
                        },
                    });
                }
            };
            Ok(ProtocolError::InvalidIdentity(
                IdentityParseError::from_parts(kind, failure),
            ))
        }
        ProtocolErrorCode::MalformedEnvelope => {
            let envelope = decode_envelope_kind(decoder)?;
            let violation = decode_violation(decoder)?;
            Ok(ProtocolError::MalformedEnvelope {
                envelope,
                violation,
            })
        }
        ProtocolErrorCode::PayloadTooLarge => {
            let envelope = decode_envelope_kind(decoder)?;
            let length = decoder
                .read_u64()
                .map_err(|error| malformed_from_wire(EnvelopeKind::Error, error))?;
            let maximum = decoder
                .read_u32()
                .map_err(|error| malformed_from_wire(EnvelopeKind::Error, error))?;
            Ok(ProtocolError::PayloadTooLarge {
                envelope,
                length,
                maximum,
            })
        }
    }
}

pub(crate) fn malformed_from_wire(envelope: EnvelopeKind, error: WireError) -> ProtocolError {
    let violation = match error {
        WireError::UnexpectedEnd => EnvelopeViolation::UnexpectedEnd,
        WireError::InvalidUtf8 => EnvelopeViolation::InvalidUtf8,
        WireError::LengthOverflow { length } => EnvelopeViolation::LengthOverflow { length },
        WireError::TrailingBytes { remaining } => EnvelopeViolation::TrailingBytes { remaining },
    };
    ProtocolError::MalformedEnvelope {
        envelope,
        violation,
    }
}

fn decode_envelope_kind(decoder: &mut Decoder<'_>) -> Result<EnvelopeKind, ProtocolError> {
    let found = decoder
        .read_u8()
        .map_err(|error| malformed_from_wire(EnvelopeKind::Error, error))?;
    EnvelopeKind::from_code(found).ok_or(ProtocolError::MalformedEnvelope {
        envelope: EnvelopeKind::Error,
        violation: EnvelopeViolation::WrongKind { found },
    })
}

fn decode_identity_kind(decoder: &mut Decoder<'_>) -> Result<IdentityKind, ProtocolError> {
    let found = decoder
        .read_u8()
        .map_err(|error| malformed_from_wire(EnvelopeKind::Error, error))?;
    IdentityKind::from_code(found).ok_or(ProtocolError::MalformedEnvelope {
        envelope: EnvelopeKind::Error,
        violation: EnvelopeViolation::UnknownIdentityKind { found },
    })
}

fn encode_violation(violation: &EnvelopeViolation, encoder: &mut Encoder) {
    match violation {
        EnvelopeViolation::BadMagic => encoder.write_u8(1),
        EnvelopeViolation::WrongKind { found } => {
            encoder.write_u8(2);
            encoder.write_u8(*found);
        }
        EnvelopeViolation::UnexpectedEnd => encoder.write_u8(3),
        EnvelopeViolation::TrailingBytes { remaining } => {
            encoder.write_u8(4);
            encoder.write_u32(*remaining);
        }
        EnvelopeViolation::UnknownErrorCode { found } => {
            encoder.write_u8(5);
            encoder.write_u16(*found);
        }
        EnvelopeViolation::UnknownIdentityKind { found } => {
            encoder.write_u8(6);
            encoder.write_u8(*found);
        }
        EnvelopeViolation::InvalidUtf8 => encoder.write_u8(7),
        EnvelopeViolation::InvalidCanonicalIdentity => encoder.write_u8(8),
        EnvelopeViolation::LengthOverflow { length } => {
            encoder.write_u8(9);
            encoder.write_u64(*length);
        }
    }
}

fn decode_violation(decoder: &mut Decoder<'_>) -> Result<EnvelopeViolation, ProtocolError> {
    let tag = decoder
        .read_u8()
        .map_err(|error| malformed_from_wire(EnvelopeKind::Error, error))?;
    match tag {
        1 => Ok(EnvelopeViolation::BadMagic),
        2 => Ok(EnvelopeViolation::WrongKind {
            found: decoder
                .read_u8()
                .map_err(|error| malformed_from_wire(EnvelopeKind::Error, error))?,
        }),
        3 => Ok(EnvelopeViolation::UnexpectedEnd),
        4 => Ok(EnvelopeViolation::TrailingBytes {
            remaining: decoder
                .read_u32()
                .map_err(|error| malformed_from_wire(EnvelopeKind::Error, error))?,
        }),
        5 => Ok(EnvelopeViolation::UnknownErrorCode {
            found: decoder
                .read_u16()
                .map_err(|error| malformed_from_wire(EnvelopeKind::Error, error))?,
        }),
        6 => Ok(EnvelopeViolation::UnknownIdentityKind {
            found: decoder
                .read_u8()
                .map_err(|error| malformed_from_wire(EnvelopeKind::Error, error))?,
        }),
        7 => Ok(EnvelopeViolation::InvalidUtf8),
        8 => Ok(EnvelopeViolation::InvalidCanonicalIdentity),
        9 => Ok(EnvelopeViolation::LengthOverflow {
            length: decoder
                .read_u64()
                .map_err(|error| malformed_from_wire(EnvelopeKind::Error, error))?,
        }),
        found => Err(ProtocolError::MalformedEnvelope {
            envelope: EnvelopeKind::Error,
            violation: EnvelopeViolation::UnknownErrorCode {
                found: u16::from(found),
            },
        }),
    }
}
