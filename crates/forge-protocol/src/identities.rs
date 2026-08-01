//! Stable typed identities shared across ForgeOS subsystem seams.
//!
//! Canonical identities are opaque 128-bit values. Their text form is exactly
//! 32 lowercase hexadecimal characters. The protocol intentionally exposes no
//! constructor from display names, list indexes, filesystem paths, timestamps,
//! or model-authored text.

use std::collections::BTreeSet;
use std::fmt;
use std::str::FromStr;

/// Number of bytes in every V1 stable identity.
pub const IDENTITY_BYTES: usize = 16;
/// Number of lowercase hexadecimal characters in every canonical identity.
pub const IDENTITY_TEXT_LENGTH: usize = IDENTITY_BYTES * 2;

/// Stable identity namespaces used by typed errors and diagnostics.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
pub enum IdentityKind {
    Project = 1,
    Repository = 2,
    Process = 3,
    Terminal = 4,
    Command = 5,
    Session = 6,
    Task = 7,
    Patch = 8,
    Result = 9,
    Event = 10,
}

impl IdentityKind {
    /// Stable V1 wire code.
    pub const fn code(self) -> u8 {
        self as u8
    }

    /// Stable human-readable namespace label.
    pub const fn label(self) -> &'static str {
        match self {
            Self::Project => "project",
            Self::Repository => "repository",
            Self::Process => "process",
            Self::Terminal => "terminal",
            Self::Command => "command",
            Self::Session => "session",
            Self::Task => "task",
            Self::Patch => "patch",
            Self::Result => "result",
            Self::Event => "event",
        }
    }

    pub(crate) const fn from_code(code: u8) -> Option<Self> {
        match code {
            1 => Some(Self::Project),
            2 => Some(Self::Repository),
            3 => Some(Self::Process),
            4 => Some(Self::Terminal),
            5 => Some(Self::Command),
            6 => Some(Self::Session),
            7 => Some(Self::Task),
            8 => Some(Self::Patch),
            9 => Some(Self::Result),
            10 => Some(Self::Event),
            _ => None,
        }
    }
}

/// Exact reason a canonical identity string was rejected.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IdentityParseFailure {
    InvalidLength { found: usize },
    NonCanonicalHex { index: usize, byte: u8 },
}

/// Typed identity parsing failure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IdentityParseError {
    kind: IdentityKind,
    failure: IdentityParseFailure,
}

impl IdentityParseError {
    pub(crate) fn from_parts(kind: IdentityKind, failure: IdentityParseFailure) -> Self {
        Self { kind, failure }
    }

    /// Identity namespace that failed to parse.
    pub const fn kind(&self) -> IdentityKind {
        self.kind
    }

    /// Exact structural failure.
    pub fn failure(&self) -> &IdentityParseFailure {
        &self.failure
    }
}

impl fmt::Display for IdentityParseError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.failure {
            IdentityParseFailure::InvalidLength { found } => write!(
                formatter,
                "{} identity must contain exactly {IDENTITY_TEXT_LENGTH} lowercase hex characters; found {found}",
                self.kind.label()
            ),
            IdentityParseFailure::NonCanonicalHex { index, byte } => write!(
                formatter,
                "{} identity contains non-canonical hex byte 0x{byte:02x} at index {index}",
                self.kind.label()
            ),
        }
    }
}

impl std::error::Error for IdentityParseError {}

/// Deterministic duplicate-identity report.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DuplicateIdentity {
    kind: IdentityKind,
    canonical: String,
}

impl DuplicateIdentity {
    /// Namespace of the duplicate value.
    pub const fn kind(&self) -> IdentityKind {
        self.kind
    }

    /// Canonical lowercase hexadecimal identity text.
    pub fn canonical(&self) -> &str {
        &self.canonical
    }

    pub(crate) fn from_parts(kind: IdentityKind, canonical: String) -> Self {
        Self { kind, canonical }
    }
}

impl fmt::Display for DuplicateIdentity {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "duplicate {} identity {}",
            self.kind.label(),
            self.canonical
        )
    }
}

impl std::error::Error for DuplicateIdentity {}

/// Shared behavior implemented by every V1 typed identity.
pub trait CanonicalIdentity:
    Copy + Eq + Ord + fmt::Debug + fmt::Display + Send + Sync + 'static
{
    /// Identity namespace.
    const KIND: IdentityKind;

    /// Exact canonical bytes.
    fn as_bytes(&self) -> &[u8; IDENTITY_BYTES];
}

macro_rules! typed_identity {
    ($name:ident, $kind:ident) => {
        #[doc = concat!("Stable typed ", stringify!($kind), " identity.")]
        #[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub struct $name([u8; IDENTITY_BYTES]);

        impl $name {
            /// Constructs an identity from already canonical opaque bytes.
            pub const fn from_bytes(bytes: [u8; IDENTITY_BYTES]) -> Self {
                Self(bytes)
            }

            /// Returns the exact canonical bytes.
            pub const fn as_bytes(&self) -> &[u8; IDENTITY_BYTES] {
                &self.0
            }

            /// Parses the exact lowercase hexadecimal V1 text representation.
            pub fn parse_canonical(input: &str) -> Result<Self, IdentityParseError> {
                parse_identity(IdentityKind::$kind, input).map(Self)
            }
        }

        impl CanonicalIdentity for $name {
            const KIND: IdentityKind = IdentityKind::$kind;

            fn as_bytes(&self) -> &[u8; IDENTITY_BYTES] {
                &self.0
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                write_identity(formatter, &self.0)
            }
        }

        impl fmt::Debug for $name {
            fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter
                    .debug_tuple(stringify!($name))
                    .field(&self.to_string())
                    .finish()
            }
        }

        impl FromStr for $name {
            type Err = IdentityParseError;

            fn from_str(input: &str) -> Result<Self, Self::Err> {
                Self::parse_canonical(input)
            }
        }
    };
}

typed_identity!(ProjectId, Project);
typed_identity!(RepositoryId, Repository);
typed_identity!(ProcessId, Process);
typed_identity!(TerminalId, Terminal);
typed_identity!(CommandId, Command);
typed_identity!(SessionId, Session);
typed_identity!(TaskId, Task);
typed_identity!(PatchId, Patch);
typed_identity!(ResultId, Result);
typed_identity!(EventId, Event);

/// Rejects the first duplicate in deterministic input order.
pub fn ensure_unique<T, I>(identities: I) -> Result<(), DuplicateIdentity>
where
    T: CanonicalIdentity,
    I: IntoIterator<Item = T>,
{
    let mut seen = BTreeSet::new();
    for identity in identities {
        if !seen.insert(identity) {
            return Err(DuplicateIdentity::from_parts(T::KIND, identity.to_string()));
        }
    }
    Ok(())
}

pub(crate) fn canonical_text_is_valid(input: &str) -> bool {
    input.len() == IDENTITY_TEXT_LENGTH
        && input
            .as_bytes()
            .iter()
            .all(|byte| matches!(byte, b'0'..=b'9' | b'a'..=b'f'))
}

fn parse_identity(
    kind: IdentityKind,
    input: &str,
) -> Result<[u8; IDENTITY_BYTES], IdentityParseError> {
    let source = input.as_bytes();
    if source.len() != IDENTITY_TEXT_LENGTH {
        return Err(IdentityParseError {
            kind,
            failure: IdentityParseFailure::InvalidLength {
                found: source.len(),
            },
        });
    }

    let mut bytes = [0_u8; IDENTITY_BYTES];
    for (index, output) in bytes.iter_mut().enumerate() {
        let high_index = index * 2;
        let low_index = high_index + 1;
        let high = decode_hex(kind, source[high_index], high_index)?;
        let low = decode_hex(kind, source[low_index], low_index)?;
        *output = (high << 4) | low;
    }
    Ok(bytes)
}

fn decode_hex(kind: IdentityKind, byte: u8, index: usize) -> Result<u8, IdentityParseError> {
    match byte {
        b'0'..=b'9' => Ok(byte - b'0'),
        b'a'..=b'f' => Ok(byte - b'a' + 10),
        _ => Err(IdentityParseError {
            kind,
            failure: IdentityParseFailure::NonCanonicalHex { index, byte },
        }),
    }
}

fn write_identity(formatter: &mut fmt::Formatter<'_>, bytes: &[u8; IDENTITY_BYTES]) -> fmt::Result {
    for byte in bytes {
        write!(formatter, "{byte:02x}")?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canonical_text_round_trips_and_rejects_uppercase() {
        let identity = ProjectId::from_bytes([0xab; IDENTITY_BYTES]);
        let text = identity.to_string();
        assert_eq!(text, "abababababababababababababababab");
        assert_eq!(text.parse::<ProjectId>(), Ok(identity));

        let error = "ABABABABABABABABABABABABABABABAB"
            .parse::<ProjectId>()
            .expect_err("uppercase hex is not canonical");
        assert_eq!(error.kind(), IdentityKind::Project);
        assert!(matches!(
            error.failure(),
            IdentityParseFailure::NonCanonicalHex {
                index: 0,
                byte: b'A'
            }
        ));
    }

    #[test]
    fn duplicate_detection_is_typed_and_deterministic() {
        let first = RepositoryId::from_bytes([1; IDENTITY_BYTES]);
        let second = RepositoryId::from_bytes([2; IDENTITY_BYTES]);
        let duplicate = ensure_unique([first, second, first])
            .expect_err("duplicate repository identity must fail");
        assert_eq!(duplicate.kind(), IdentityKind::Repository);
        assert_eq!(duplicate.canonical(), first.to_string());
    }
}
