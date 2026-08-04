//! Buffer-version-bound Rust syntax state.
//!
//! The editor layer binds the Tree-sitter adapter to one canonical buffer and
//! content generation. It stores no source bytes. A syntax snapshot is readable
//! only when the caller proves that buffer identity, document identity, content
//! version, length, and content hash still match the parsed state.

use crate::buffers::{BufferId, ContentVersion, DocumentKey, EditorBuffer};
use forge_bridge::parsing::{RustParseError, RustSyntaxParser, RustSyntaxSnapshot};
use forge_protocol::hashes::{hash_canonical_bytes, ContentHash, HashDomain};
use std::fmt;

/// Incremental parse state attached to one editor buffer.
pub struct ParsedBuffer {
    buffer_id: BufferId,
    document: DocumentKey,
    content_version: ContentVersion,
    parser: RustSyntaxParser,
}

impl ParsedBuffer {
    /// Parses the exact current bytes of one buffer.
    pub fn parse(buffer: &EditorBuffer) -> Result<Self, BufferParseError> {
        let parser = RustSyntaxParser::parse(buffer.bytes()).map_err(BufferParseError::Parser)?;
        Ok(Self {
            buffer_id: buffer.id(),
            document: buffer.document().clone(),
            content_version: buffer.content_version(),
            parser,
        })
    }

    pub const fn buffer_id(&self) -> BufferId {
        self.buffer_id
    }

    pub fn document(&self) -> &DocumentKey {
        &self.document
    }

    pub const fn content_version(&self) -> ContentVersion {
        self.content_version
    }

    /// Returns syntax only when it still describes the exact current buffer.
    pub fn snapshot_for(
        &self,
        buffer: &EditorBuffer,
    ) -> Result<&RustSyntaxSnapshot, BufferParseError> {
        self.validate_buffer_identity(buffer)?;
        self.validate_current_content(buffer)?;
        Ok(self.parser.snapshot())
    }

    /// Prepares an incremental parse without mutating committed parser state.
    pub fn prepare_update(
        &self,
        buffer: &EditorBuffer,
        previous_source: &[u8],
    ) -> Result<PendingBufferParse, BufferParseError> {
        self.validate_buffer_identity(buffer)?;
        if buffer.content_version() <= self.content_version {
            return Err(BufferParseError::NonAdvancingVersion {
                parsed: self.content_version,
                requested: buffer.content_version(),
            });
        }
        let parser = self
            .parser
            .prepare_update(previous_source, buffer.bytes())
            .map_err(BufferParseError::Parser)?;
        Ok(PendingBufferParse {
            buffer_id: self.buffer_id,
            document: self.document.clone(),
            expected_content_version: self.content_version,
            next_content_version: buffer.content_version(),
            parser,
        })
    }

    /// Commits one already prepared syntax generation.
    pub fn commit_update(
        &mut self,
        pending: PendingBufferParse,
    ) -> Result<&RustSyntaxSnapshot, BufferParseError> {
        self.validate_pending(&pending)?;
        self.content_version = pending.next_content_version;
        self.parser = pending.parser;
        Ok(self.parser.snapshot())
    }

    /// Incrementally advances parser state to the current buffer bytes.
    pub fn update<'a>(
        &'a mut self,
        buffer: &EditorBuffer,
        previous_source: &[u8],
    ) -> Result<&'a RustSyntaxSnapshot, BufferParseError> {
        let pending = self.prepare_update(buffer, previous_source)?;
        self.commit_update(pending)
    }

    pub(crate) fn validate_pending(
        &self,
        pending: &PendingBufferParse,
    ) -> Result<(), BufferParseError> {
        if pending.buffer_id != self.buffer_id
            || pending.document != self.document
            || pending.expected_content_version != self.content_version
            || pending.next_content_version <= self.content_version
        {
            return Err(BufferParseError::PendingUpdateMismatch);
        }
        Ok(())
    }

    fn validate_buffer_identity(&self, buffer: &EditorBuffer) -> Result<(), BufferParseError> {
        if buffer.id() != self.buffer_id {
            return Err(BufferParseError::BufferMismatch {
                expected: self.buffer_id,
                actual: buffer.id(),
            });
        }
        if buffer.document() != &self.document {
            return Err(BufferParseError::DocumentMismatch);
        }
        Ok(())
    }

    fn validate_current_content(&self, buffer: &EditorBuffer) -> Result<(), BufferParseError> {
        if buffer.content_version() != self.content_version {
            return Err(BufferParseError::StaleSnapshot {
                parsed: self.content_version,
                current: buffer.content_version(),
            });
        }
        let snapshot = self.parser.snapshot();
        let actual_hash = source_hash(buffer.bytes());
        if snapshot.source_len() != buffer.bytes().len() || snapshot.source_hash() != actual_hash {
            return Err(BufferParseError::ContentIdentityMismatch {
                expected_hash: snapshot.source_hash(),
                actual_hash,
                expected_len: snapshot.source_len(),
                actual_len: buffer.bytes().len(),
            });
        }
        Ok(())
    }
}

/// Prepared parser replacement bound to one exact editor generation transition.
pub struct PendingBufferParse {
    buffer_id: BufferId,
    document: DocumentKey,
    expected_content_version: ContentVersion,
    next_content_version: ContentVersion,
    parser: RustSyntaxParser,
}

impl PendingBufferParse {
    pub const fn next_content_version(&self) -> ContentVersion {
        self.next_content_version
    }

    pub fn snapshot(&self) -> &RustSyntaxSnapshot {
        self.parser.snapshot()
    }
}

/// Exact reason editor syntax state was unavailable or stale.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BufferParseError {
    Parser(RustParseError),
    BufferMismatch {
        expected: BufferId,
        actual: BufferId,
    },
    DocumentMismatch,
    PendingUpdateMismatch,
    StaleSnapshot {
        parsed: ContentVersion,
        current: ContentVersion,
    },
    NonAdvancingVersion {
        parsed: ContentVersion,
        requested: ContentVersion,
    },
    ContentIdentityMismatch {
        expected_hash: ContentHash,
        actual_hash: ContentHash,
        expected_len: usize,
        actual_len: usize,
    },
}

impl fmt::Display for BufferParseError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Parser(error) => write!(formatter, "Rust parse failed: {error}"),
            Self::BufferMismatch { expected, actual } => write!(
                formatter,
                "parser belongs to buffer {expected}, not buffer {actual}"
            ),
            Self::DocumentMismatch => {
                formatter.write_str("parser document identity does not match buffer")
            }
            Self::PendingUpdateMismatch => {
                formatter.write_str("pending parser update no longer matches committed state")
            }
            Self::StaleSnapshot { parsed, current } => write!(
                formatter,
                "syntax snapshot targets content version {}, current buffer is {}",
                parsed.get(),
                current.get()
            ),
            Self::NonAdvancingVersion { parsed, requested } => write!(
                formatter,
                "incremental parse version {} does not advance parsed version {}",
                requested.get(),
                parsed.get()
            ),
            Self::ContentIdentityMismatch {
                expected_len,
                actual_len,
                ..
            } => write!(
                formatter,
                "syntax content identity mismatch: expected {expected_len} bytes, got {actual_len}"
            ),
        }
    }
}

impl std::error::Error for BufferParseError {}

fn source_hash(source: &[u8]) -> ContentHash {
    hash_canonical_bytes(HashDomain::File, source)
}
