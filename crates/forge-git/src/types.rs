//! Raw Git identity and path values preserved without lossy text conversion.

use std::fmt;

/// Exact SHA-1 or SHA-256 object identity emitted by native Git.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct GitObjectId(String);

impl GitObjectId {
    /// Parses one exact lowercase SHA-1 or SHA-256 object identity.
    pub fn from_hex(value: &str) -> Result<Self, String> {
        Self::parse(value.as_bytes())
    }

    pub(crate) fn parse(bytes: &[u8]) -> Result<Self, String> {
        if !matches!(bytes.len(), 40 | 64) {
            return Err(format!(
                "Git object identity must contain 40 or 64 hex bytes; found {}",
                bytes.len()
            ));
        }
        if let Some((index, byte)) = bytes
            .iter()
            .copied()
            .enumerate()
            .find(|(_, byte)| !matches!(byte, b'0'..=b'9' | b'a'..=b'f'))
        {
            return Err(format!(
                "Git object identity contains non-canonical byte 0x{byte:02x} at {index}"
            ));
        }
        Ok(Self(
            String::from_utf8(bytes.to_vec())
                .expect("validated lowercase hexadecimal bytes are UTF-8"),
        ))
    }

    /// Canonical lowercase hexadecimal object identity.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for GitObjectId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

/// Exact path bytes emitted by Git porcelain `-z` output.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct GitPath(Vec<u8>);

impl GitPath {
    pub(crate) fn from_bytes(bytes: &[u8]) -> Result<Self, String> {
        if bytes.is_empty() {
            return Err("Git path may not be empty".to_owned());
        }
        if bytes.contains(&0) {
            return Err("Git path may not contain NUL".to_owned());
        }
        Ok(Self(bytes.to_vec()))
    }

    /// Exact unquoted path bytes from native Git.
    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }

    /// Lossy display text for diagnostics only. It is never canonical identity.
    pub fn display_lossy(&self) -> String {
        String::from_utf8_lossy(&self.0).into_owned()
    }
}

/// Exact ref-name bytes from Git porcelain output.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct GitRefName(Vec<u8>);

impl GitRefName {
    pub(crate) fn from_bytes(bytes: &[u8]) -> Result<Self, String> {
        if bytes.is_empty() {
            return Err("Git ref name may not be empty".to_owned());
        }
        if bytes.contains(&0) {
            return Err("Git ref name may not contain NUL".to_owned());
        }
        Ok(Self(bytes.to_vec()))
    }

    /// Exact native ref-name bytes.
    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }

    pub fn display_lossy(&self) -> String {
        String::from_utf8_lossy(&self.0).into_owned()
    }
}
