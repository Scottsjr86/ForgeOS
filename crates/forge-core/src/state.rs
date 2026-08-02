//! Canonical versioned local-state records.
//!
//! Forge Core owns the exact bytes and schema meaning. Filesystem effects remain
//! outside this crate. The integrity checksum detects accidental corruption; it
//! is not an artifact identity and does not replace the SHA-256 contract owned by
//! `FORGEOS-V1-HASH-000`.

use std::fmt;

const STATE_MAGIC: [u8; 8] = *b"FGSTATE\0";
const HEADER_BYTES: usize = STATE_MAGIC.len() + 2 + 2 + 4;
const TRAILER_BYTES: usize = 4;
const MAX_PAYLOAD_BYTES: usize = 16 * 1024 * 1024;

/// The only schema version written by the current V1 implementation.
pub const CURRENT_STATE_SCHEMA_VERSION: u16 = 1;

/// The explicit legacy fixture version supported by the V1 migration path.
pub const LEGACY_STATE_SCHEMA_VERSION: u16 = 0;

/// One canonical local-state record owned by Forge Core.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StateRecord {
    record_type: u16,
    payload: Vec<u8>,
}

impl StateRecord {
    /// Creates a current-schema record from already canonical payload bytes.
    pub fn new(record_type: u16, payload: Vec<u8>) -> Result<Self, StateRecordError> {
        validate_record_type(record_type)?;
        validate_payload_len(payload.len())?;
        Ok(Self {
            record_type,
            payload,
        })
    }

    /// Stable application-defined record type. Zero is reserved and rejected.
    pub const fn record_type(&self) -> u16 {
        self.record_type
    }

    /// Canonical payload bytes. Their meaning belongs to the record type.
    pub fn payload(&self) -> &[u8] {
        &self.payload
    }

    /// Current explicit schema version.
    pub const fn schema_version(&self) -> u16 {
        CURRENT_STATE_SCHEMA_VERSION
    }

    /// Encodes the exact current-schema bytes written by persistence adapters.
    pub fn encode(&self) -> Vec<u8> {
        encode_current(self.record_type, &self.payload)
    }

    /// Decodes only the current schema. Legacy bytes require an explicit migration.
    pub fn decode(bytes: &[u8]) -> Result<Self, StateRecordError> {
        let header = parse_header(bytes)?;
        match header.schema_version {
            CURRENT_STATE_SCHEMA_VERSION => decode_current(bytes, header),
            LEGACY_STATE_SCHEMA_VERSION => Err(StateRecordError::MigrationRequired {
                found: LEGACY_STATE_SCHEMA_VERSION,
                target: CURRENT_STATE_SCHEMA_VERSION,
            }),
            found => Err(StateRecordError::UnsupportedSchemaVersion { found }),
        }
    }
}

/// Result of one explicit legacy-to-current migration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MigratedStateRecord {
    source_schema_version: u16,
    record: StateRecord,
}

impl MigratedStateRecord {
    pub const fn source_schema_version(&self) -> u16 {
        self.source_schema_version
    }

    pub fn record(&self) -> &StateRecord {
        &self.record
    }

    pub fn into_record(self) -> StateRecord {
        self.record
    }
}

/// Explicitly migrates the one reviewed V0 fixture format to current V1 bytes.
pub fn migrate_legacy_v0(bytes: &[u8]) -> Result<MigratedStateRecord, StateRecordError> {
    let header = parse_header(bytes)?;
    if header.schema_version != LEGACY_STATE_SCHEMA_VERSION {
        return Err(StateRecordError::MigrationSourceMismatch {
            expected: LEGACY_STATE_SCHEMA_VERSION,
            found: header.schema_version,
        });
    }

    let payload_end = HEADER_BYTES
        .checked_add(header.payload_len)
        .ok_or(StateRecordError::LengthOverflow)?;
    if bytes.len() != payload_end {
        return Err(StateRecordError::LengthMismatch {
            declared: header.payload_len,
            actual: bytes.len().saturating_sub(HEADER_BYTES),
        });
    }

    let record = StateRecord::new(header.record_type, bytes[HEADER_BYTES..payload_end].to_vec())?;
    Ok(MigratedStateRecord {
        source_schema_version: LEGACY_STATE_SCHEMA_VERSION,
        record,
    })
}

/// Encodes the reviewed V0 migration fixture. Production writers must use
/// [`StateRecord::encode`] and therefore emit the current schema only.
pub fn encode_legacy_v0_fixture(
    record_type: u16,
    payload: &[u8],
) -> Result<Vec<u8>, StateRecordError> {
    validate_record_type(record_type)?;
    validate_payload_len(payload.len())?;
    let payload_len = u32::try_from(payload.len()).map_err(|_| StateRecordError::PayloadTooLarge {
        maximum: MAX_PAYLOAD_BYTES,
        actual: payload.len(),
    })?;

    let mut bytes = Vec::with_capacity(HEADER_BYTES + payload.len());
    bytes.extend_from_slice(&STATE_MAGIC);
    bytes.extend_from_slice(&LEGACY_STATE_SCHEMA_VERSION.to_be_bytes());
    bytes.extend_from_slice(&record_type.to_be_bytes());
    bytes.extend_from_slice(&payload_len.to_be_bytes());
    bytes.extend_from_slice(payload);
    Ok(bytes)
}

#[derive(Debug, Clone, Copy)]
struct ParsedHeader {
    schema_version: u16,
    record_type: u16,
    payload_len: usize,
}

fn parse_header(bytes: &[u8]) -> Result<ParsedHeader, StateRecordError> {
    if bytes.len() < HEADER_BYTES {
        return Err(StateRecordError::Truncated {
            minimum: HEADER_BYTES,
            actual: bytes.len(),
        });
    }
    if bytes[..STATE_MAGIC.len()] != STATE_MAGIC[..] {
        return Err(StateRecordError::InvalidMagic);
    }

    let schema_version = u16::from_be_bytes([bytes[8], bytes[9]]);
    let record_type = u16::from_be_bytes([bytes[10], bytes[11]]);
    validate_record_type(record_type)?;
    let declared = u32::from_be_bytes([bytes[12], bytes[13], bytes[14], bytes[15]]) as usize;
    validate_payload_len(declared)?;

    Ok(ParsedHeader {
        schema_version,
        record_type,
        payload_len: declared,
    })
}

fn decode_current(bytes: &[u8], header: ParsedHeader) -> Result<StateRecord, StateRecordError> {
    let payload_end = HEADER_BYTES
        .checked_add(header.payload_len)
        .ok_or(StateRecordError::LengthOverflow)?;
    let expected_len = payload_end
        .checked_add(TRAILER_BYTES)
        .ok_or(StateRecordError::LengthOverflow)?;
    if bytes.len() != expected_len {
        return Err(StateRecordError::LengthMismatch {
            declared: header.payload_len,
            actual: bytes
                .len()
                .saturating_sub(HEADER_BYTES + TRAILER_BYTES),
        });
    }

    let expected_checksum = u32::from_be_bytes([
        bytes[payload_end],
        bytes[payload_end + 1],
        bytes[payload_end + 2],
        bytes[payload_end + 3],
    ]);
    let actual_checksum = integrity_checksum(&bytes[..payload_end]);
    if expected_checksum != actual_checksum {
        return Err(StateRecordError::ChecksumMismatch {
            expected: expected_checksum,
            actual: actual_checksum,
        });
    }

    StateRecord::new(
        header.record_type,
        bytes[HEADER_BYTES..payload_end].to_vec(),
    )
}

fn encode_current(record_type: u16, payload: &[u8]) -> Vec<u8> {
    let payload_len = u32::try_from(payload.len()).expect("validated payload length fits u32");
    let mut bytes = Vec::with_capacity(HEADER_BYTES + payload.len() + TRAILER_BYTES);
    bytes.extend_from_slice(&STATE_MAGIC);
    bytes.extend_from_slice(&CURRENT_STATE_SCHEMA_VERSION.to_be_bytes());
    bytes.extend_from_slice(&record_type.to_be_bytes());
    bytes.extend_from_slice(&payload_len.to_be_bytes());
    bytes.extend_from_slice(payload);
    let checksum = integrity_checksum(&bytes);
    bytes.extend_from_slice(&checksum.to_be_bytes());
    bytes
}

fn validate_record_type(record_type: u16) -> Result<(), StateRecordError> {
    if record_type == 0 {
        Err(StateRecordError::ReservedRecordType)
    } else {
        Ok(())
    }
}

fn validate_payload_len(payload_len: usize) -> Result<(), StateRecordError> {
    if payload_len > MAX_PAYLOAD_BYTES {
        Err(StateRecordError::PayloadTooLarge {
            maximum: MAX_PAYLOAD_BYTES,
            actual: payload_len,
        })
    } else {
        Ok(())
    }
}

fn integrity_checksum(bytes: &[u8]) -> u32 {
    let mut checksum = 0x811c_9dc5u32;
    for byte in bytes {
        checksum ^= u32::from(*byte);
        checksum = checksum.wrapping_mul(0x0100_0193);
    }
    checksum
}

/// Exact reason canonical state bytes were rejected.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StateRecordError {
    Truncated { minimum: usize, actual: usize },
    InvalidMagic,
    ReservedRecordType,
    PayloadTooLarge { maximum: usize, actual: usize },
    LengthOverflow,
    LengthMismatch { declared: usize, actual: usize },
    ChecksumMismatch { expected: u32, actual: u32 },
    MigrationRequired { found: u16, target: u16 },
    MigrationSourceMismatch { expected: u16, found: u16 },
    UnsupportedSchemaVersion { found: u16 },
}

impl fmt::Display for StateRecordError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Truncated { minimum, actual } => {
                write!(formatter, "state record needs at least {minimum} bytes, found {actual}")
            }
            Self::InvalidMagic => write!(formatter, "state record magic is invalid"),
            Self::ReservedRecordType => write!(formatter, "state record type zero is reserved"),
            Self::PayloadTooLarge { maximum, actual } => write!(
                formatter,
                "state payload is {actual} bytes; maximum is {maximum}"
            ),
            Self::LengthOverflow => write!(formatter, "state record length overflowed"),
            Self::LengthMismatch { declared, actual } => write!(
                formatter,
                "state payload declared {declared} bytes but encoded {actual}"
            ),
            Self::ChecksumMismatch { expected, actual } => write!(
                formatter,
                "state integrity checksum mismatch: encoded {expected:08x}, computed {actual:08x}"
            ),
            Self::MigrationRequired { found, target } => write!(
                formatter,
                "state schema {found} requires explicit migration to schema {target}"
            ),
            Self::MigrationSourceMismatch { expected, found } => write!(
                formatter,
                "migration expects schema {expected}, found schema {found}"
            ),
            Self::UnsupportedSchemaVersion { found } => {
                write!(formatter, "state schema {found} is unsupported")
            }
        }
    }
}

impl std::error::Error for StateRecordError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn current_record_round_trips_to_exact_bytes() {
        let record = StateRecord::new(7, b"canonical-state".to_vec()).unwrap();
        let bytes = record.encode();

        assert_eq!(&bytes[..8], b"FGSTATE\0");
        assert_eq!(&bytes[8..10], &1u16.to_be_bytes());
        assert_eq!(&bytes[10..12], &7u16.to_be_bytes());
        assert_eq!(&bytes[12..16], &15u32.to_be_bytes());
        assert_eq!(StateRecord::decode(&bytes), Ok(record));
    }

    #[test]
    fn payload_corruption_is_rejected() {
        let record = StateRecord::new(1, b"untouched".to_vec()).unwrap();
        let mut bytes = record.encode();
        bytes[HEADER_BYTES + 2] ^= 0x40;

        assert!(matches!(
            StateRecord::decode(&bytes),
            Err(StateRecordError::ChecksumMismatch { .. })
        ));
    }

    #[test]
    fn legacy_schema_requires_explicit_migration() {
        let bytes = encode_legacy_v0_fixture(3, b"legacy").unwrap();
        assert_eq!(
            StateRecord::decode(&bytes),
            Err(StateRecordError::MigrationRequired {
                found: LEGACY_STATE_SCHEMA_VERSION,
                target: CURRENT_STATE_SCHEMA_VERSION,
            })
        );

        let migrated = migrate_legacy_v0(&bytes).unwrap();
        assert_eq!(migrated.source_schema_version(), 0);
        assert_eq!(migrated.record().record_type(), 3);
        assert_eq!(migrated.record().payload(), b"legacy");
        assert_eq!(
            StateRecord::decode(&migrated.record().encode()),
            Ok(migrated.into_record())
        );
    }

    #[test]
    fn unknown_schema_never_falls_back_or_guesses() {
        let record = StateRecord::new(4, b"future".to_vec()).unwrap();
        let mut bytes = record.encode();
        bytes[8..10].copy_from_slice(&99u16.to_be_bytes());

        assert_eq!(
            StateRecord::decode(&bytes),
            Err(StateRecordError::UnsupportedSchemaVersion { found: 99 })
        );
    }

    #[test]
    fn truncation_trailing_bytes_and_reserved_type_fail() {
        assert!(matches!(
            StateRecord::decode(b"short"),
            Err(StateRecordError::Truncated { .. })
        ));

        let record = StateRecord::new(5, b"payload".to_vec()).unwrap();
        let mut trailing = record.encode();
        trailing.push(0);
        assert!(matches!(
            StateRecord::decode(&trailing),
            Err(StateRecordError::LengthMismatch { .. })
        ));

        assert_eq!(
            StateRecord::new(0, Vec::new()),
            Err(StateRecordError::ReservedRecordType)
        );
    }
}
