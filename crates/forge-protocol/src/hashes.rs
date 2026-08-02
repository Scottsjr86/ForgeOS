//! Stable domain-separated SHA-256 identities over declared canonical bytes.
//!
//! Callers choose the semantic domain and provide only bytes that belong to the
//! identity contract. Host paths, timestamps, display labels, locale, and other
//! descriptive metadata are absent unless the caller deliberately declares them
//! as canonical fields.

use crate::sha256;
use std::fmt;
use std::str::FromStr;

const HASH_MAGIC: &[u8; 12] = b"FORGEHASH\0\0\0";
const HASH_SCHEMA_VERSION: u8 = 1;
const MAX_FIELD_NAME_BYTES: usize = 64;
const MAX_FIELD_COUNT: usize = u16::MAX as usize;

/// Semantic separation for otherwise identical canonical payload bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
pub enum HashDomain {
    File = 1,
    Patch = 2,
    ToolRequest = 3,
    Snapshot = 4,
    ResultPayload = 5,
}

impl HashDomain {
    pub const fn code(self) -> u8 {
        self as u8
    }
}

/// Exact SHA-256 identity rendered as 64 lowercase hexadecimal characters.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ContentHash([u8; 32]);

impl ContentHash {
    pub const fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    pub const fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }

    pub fn to_hex(self) -> String {
        let mut text = String::with_capacity(64);
        for byte in self.0 {
            use std::fmt::Write;
            write!(&mut text, "{byte:02x}").expect("writing to a String cannot fail");
        }
        text
    }
}

impl fmt::Debug for ContentHash {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_tuple("ContentHash")
            .field(&self.to_hex())
            .finish()
    }
}

impl fmt::Display for ContentHash {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.to_hex())
    }
}

impl FromStr for ContentHash {
    type Err = HashContractError;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        if text.len() != 64 {
            return Err(HashContractError::InvalidDigestLength {
                expected: 64,
                actual: text.len(),
            });
        }

        let mut bytes = [0u8; 32];
        for (index, pair) in text.as_bytes().chunks_exact(2).enumerate() {
            let high = decode_lower_hex(pair[0], index * 2)?;
            let low = decode_lower_hex(pair[1], index * 2 + 1)?;
            bytes[index] = (high << 4) | low;
        }
        Ok(Self(bytes))
    }
}

/// One named field included in a structured canonical identity.
#[derive(Debug, Clone, PartialEq, Eq)]
struct CanonicalField {
    name: String,
    value: Vec<u8>,
}

/// A canonical identity input whose field ordering is stable by construction.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CanonicalHashInput {
    domain: HashDomain,
    fields: Vec<CanonicalField>,
}

impl CanonicalHashInput {
    pub const fn new(domain: HashDomain) -> Self {
        Self {
            domain,
            fields: Vec::new(),
        }
    }

    pub const fn domain(&self) -> HashDomain {
        self.domain
    }

    /// Adds one declared canonical field. Insertion order does not affect identity.
    pub fn add_field(
        &mut self,
        name: impl Into<String>,
        value: impl Into<Vec<u8>>,
    ) -> Result<(), HashContractError> {
        let name = name.into();
        validate_field_name(&name)?;
        if self.fields.len() >= MAX_FIELD_COUNT {
            return Err(HashContractError::TooManyFields {
                maximum: MAX_FIELD_COUNT,
            });
        }
        if self.fields.iter().any(|field| field.name == name) {
            return Err(HashContractError::DuplicateField(name));
        }
        self.fields.push(CanonicalField {
            name,
            value: value.into(),
        });
        Ok(())
    }

    /// Exact versioned bytes consumed by SHA-256.
    pub fn canonical_bytes(&self) -> Vec<u8> {
        let mut fields: Vec<&CanonicalField> = self.fields.iter().collect();
        fields.sort_by(|left, right| left.name.as_bytes().cmp(right.name.as_bytes()));

        let estimated = HASH_MAGIC.len()
            + 1
            + 1
            + 2
            + fields
                .iter()
                .map(|field| 2 + field.name.len() + 8 + field.value.len())
                .sum::<usize>();
        let mut bytes = Vec::with_capacity(estimated);
        bytes.extend_from_slice(HASH_MAGIC);
        bytes.push(HASH_SCHEMA_VERSION);
        bytes.push(self.domain.code());
        bytes.extend_from_slice(&(fields.len() as u16).to_be_bytes());
        for field in fields {
            bytes.extend_from_slice(&(field.name.len() as u16).to_be_bytes());
            bytes.extend_from_slice(field.name.as_bytes());
            bytes.extend_from_slice(&(field.value.len() as u64).to_be_bytes());
            bytes.extend_from_slice(&field.value);
        }
        bytes
    }

    pub fn identity(&self) -> ContentHash {
        ContentHash::from_bytes(sha256::digest(&self.canonical_bytes()))
    }
}

/// Hashes one declared canonical byte payload in a semantic domain.
pub fn hash_canonical_bytes(domain: HashDomain, bytes: &[u8]) -> ContentHash {
    let mut input = CanonicalHashInput::new(domain);
    input
        .add_field("content", bytes)
        .expect("the built-in canonical field name is valid and unique");
    input.identity()
}

/// Verifies canonical bytes against an expected identity.
pub fn verify_canonical_bytes(
    domain: HashDomain,
    bytes: &[u8],
    expected: ContentHash,
) -> Result<(), HashContractError> {
    let actual = hash_canonical_bytes(domain, bytes);
    if actual == expected {
        Ok(())
    } else {
        Err(HashContractError::DigestMismatch { expected, actual })
    }
}

/// Exact reason a canonical hash contract was rejected.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HashContractError {
    EmptyFieldName,
    FieldNameTooLong {
        maximum: usize,
        actual: usize,
    },
    InvalidFieldNameByte {
        index: usize,
        byte: u8,
    },
    DuplicateField(String),
    TooManyFields {
        maximum: usize,
    },
    InvalidDigestLength {
        expected: usize,
        actual: usize,
    },
    InvalidDigestCharacter {
        index: usize,
        byte: u8,
    },
    DigestMismatch {
        expected: ContentHash,
        actual: ContentHash,
    },
}

impl fmt::Display for HashContractError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyFieldName => formatter.write_str("canonical hash field name is empty"),
            Self::FieldNameTooLong { maximum, actual } => write!(
                formatter,
                "canonical hash field name is too long: maximum {maximum} bytes, got {actual}"
            ),
            Self::InvalidFieldNameByte { index, byte } => write!(
                formatter,
                "canonical hash field name byte {index} is invalid: 0x{byte:02x}"
            ),
            Self::DuplicateField(name) => {
                write!(formatter, "canonical hash field is duplicated: {name}")
            }
            Self::TooManyFields { maximum } => {
                write!(formatter, "canonical hash input exceeds {maximum} fields")
            }
            Self::InvalidDigestLength { expected, actual } => write!(
                formatter,
                "SHA-256 text must be {expected} lowercase hexadecimal bytes, got {actual}"
            ),
            Self::InvalidDigestCharacter { index, byte } => write!(
                formatter,
                "SHA-256 text byte {index} is not lowercase hexadecimal: 0x{byte:02x}"
            ),
            Self::DigestMismatch { expected, actual } => {
                write!(
                    formatter,
                    "SHA-256 mismatch: expected {expected}, got {actual}"
                )
            }
        }
    }
}

impl std::error::Error for HashContractError {}

fn validate_field_name(name: &str) -> Result<(), HashContractError> {
    if name.is_empty() {
        return Err(HashContractError::EmptyFieldName);
    }
    if name.len() > MAX_FIELD_NAME_BYTES {
        return Err(HashContractError::FieldNameTooLong {
            maximum: MAX_FIELD_NAME_BYTES,
            actual: name.len(),
        });
    }
    for (index, byte) in name.bytes().enumerate() {
        if !(byte.is_ascii_lowercase()
            || byte.is_ascii_digit()
            || matches!(byte, b'.' | b'_' | b'-'))
        {
            return Err(HashContractError::InvalidFieldNameByte { index, byte });
        }
    }
    Ok(())
}

fn decode_lower_hex(byte: u8, index: usize) -> Result<u8, HashContractError> {
    match byte {
        b'0'..=b'9' => Ok(byte - b'0'),
        b'a'..=b'f' => Ok(byte - b'a' + 10),
        _ => Err(HashContractError::InvalidDigestCharacter { index, byte }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn structured_request(order: &[&str]) -> CanonicalHashInput {
        let mut input = CanonicalHashInput::new(HashDomain::ToolRequest);
        for name in order {
            let value: &[u8] = match *name {
                "argv" => b"cargo\0test\0--workspace",
                "command_id" => b"00000000000000000000000000000007",
                "revision" => b"4f9a1c2",
                _ => panic!("unexpected fixture field"),
            };
            input.add_field(*name, value).unwrap();
        }
        input
    }

    #[test]
    fn identical_file_bytes_have_identical_identity() {
        let first = hash_canonical_bytes(HashDomain::File, b"fn main() {}\n");
        let second = hash_canonical_bytes(HashDomain::File, b"fn main() {}\n");
        assert_eq!(first, second);
        assert_eq!(first.to_string().len(), 64);
        assert_eq!(first.to_string().parse::<ContentHash>().unwrap(), first);
    }

    #[test]
    fn structured_field_order_does_not_change_identity() {
        let first = structured_request(&["argv", "command_id", "revision"]);
        let second = structured_request(&["revision", "argv", "command_id"]);
        assert_eq!(first.canonical_bytes(), second.canonical_bytes());
        assert_eq!(first.identity(), second.identity());
    }

    #[test]
    fn v1_structured_request_identity_is_golden_locked() {
        let input = structured_request(&["revision", "argv", "command_id"]);
        assert_eq!(
            input.identity().to_string(),
            "b67a0fb7b1c419838df818f1ca86e91400d36179f56eaff28d7521240d2047b8"
        );
    }

    #[test]
    fn changed_content_or_domain_changes_identity() {
        let original = hash_canonical_bytes(HashDomain::Patch, b"diff --git a/a b/a\n");
        let changed = hash_canonical_bytes(HashDomain::Patch, b"diff --git a/a b/b\n");
        let other_domain = hash_canonical_bytes(HashDomain::File, b"diff --git a/a b/a\n");
        assert_ne!(original, changed);
        assert_ne!(original, other_domain);
    }

    #[test]
    fn corrupt_payload_fails_verification() {
        let expected = hash_canonical_bytes(HashDomain::Snapshot, b"canonical snapshot");
        let error = verify_canonical_bytes(HashDomain::Snapshot, b"corrupt snapshot", expected)
            .unwrap_err();
        assert!(matches!(error, HashContractError::DigestMismatch { .. }));
    }

    #[test]
    fn duplicate_and_unstable_field_names_are_rejected() {
        let mut input = CanonicalHashInput::new(HashDomain::ResultPayload);
        input.add_field("payload", b"ok".as_slice()).unwrap();
        assert_eq!(
            input.add_field("payload", b"again".as_slice()).unwrap_err(),
            HashContractError::DuplicateField("payload".to_string())
        );
        assert!(matches!(
            input.add_field("Display Name", b"ignored".as_slice()),
            Err(HashContractError::InvalidFieldNameByte { .. })
        ));
    }

    #[test]
    fn uppercase_or_malformed_digest_text_is_rejected() {
        let valid = hash_canonical_bytes(HashDomain::File, b"bytes").to_string();
        let uppercase = valid.to_uppercase();
        assert!(matches!(
            uppercase.parse::<ContentHash>(),
            Err(HashContractError::InvalidDigestCharacter { .. })
        ));
        assert!(matches!(
            "abcd".parse::<ContentHash>(),
            Err(HashContractError::InvalidDigestLength { .. })
        ));
    }

    #[test]
    fn all_v1_domains_are_distinct_for_the_same_payload() {
        let payload = b"same canonical bytes";
        let hashes = [
            hash_canonical_bytes(HashDomain::File, payload),
            hash_canonical_bytes(HashDomain::Patch, payload),
            hash_canonical_bytes(HashDomain::ToolRequest, payload),
            hash_canonical_bytes(HashDomain::Snapshot, payload),
            hash_canonical_bytes(HashDomain::ResultPayload, payload),
        ];
        for left in 0..hashes.len() {
            for right in left + 1..hashes.len() {
                assert_ne!(hashes[left], hashes[right]);
            }
        }
    }
}
