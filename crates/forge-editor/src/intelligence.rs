//! Atomic editor-owned Rust syntax and language-document state.
//!
//! Tree-sitter remains available without a language server. Rust Analyzer owns
//! diagnostics and navigation responses. This state only binds both mechanisms to
//! the same exact buffer generation so stale responses cannot cross documents.

use crate::buffers::{BufferId, ContentVersion, EditorBuffer};
use crate::language::{LanguageDocumentError, PendingLanguageUpdate, RustLanguageDocument};
use crate::parsing::{BufferParseError, ParsedBuffer, PendingBufferParse};
use forge_bridge::lsp::{CompletionResult, DefinitionResult, LspDocument, PublishedDiagnostics};
use forge_bridge::parsing::RustSyntaxSnapshot;
use forge_protocol::identities::ProjectId;
use std::fmt;

/// Current syntax and LSP identity for one Rust editor buffer.
pub struct RustBufferIntelligence {
    parsed: ParsedBuffer,
    language: RustLanguageDocument,
    lsp_document: LspDocument,
}

impl RustBufferIntelligence {
    pub fn open(
        project_id: ProjectId,
        buffer: &EditorBuffer,
    ) -> Result<Self, RustIntelligenceError> {
        let parsed = ParsedBuffer::parse(buffer)?;
        let (language, lsp_document) = RustLanguageDocument::open(project_id, buffer)?;
        if parsed.content_version().get() != lsp_document.version().get() as u64 {
            return Err(RustIntelligenceError::GenerationMismatch);
        }
        Ok(Self {
            parsed,
            language,
            lsp_document,
        })
    }

    pub const fn buffer_id(&self) -> BufferId {
        self.parsed.buffer_id()
    }

    pub const fn content_version(&self) -> ContentVersion {
        self.parsed.content_version()
    }

    pub fn lsp_document(&self) -> &LspDocument {
        &self.lsp_document
    }

    pub fn status(&self) -> RustIntelligenceStatus {
        let syntax_version = self.parsed.content_version();
        let language_version = self.language.content_version();
        if syntax_version == language_version {
            RustIntelligenceStatus::Current { syntax_version }
        } else {
            RustIntelligenceStatus::LanguageDegraded {
                syntax_version,
                language_version,
            }
        }
    }

    pub fn syntax_snapshot(
        &self,
        buffer: &EditorBuffer,
    ) -> Result<&RustSyntaxSnapshot, RustIntelligenceError> {
        Ok(self.parsed.snapshot_for(buffer)?)
    }

    /// Prepares parser and LSP state for one newer buffer generation without
    /// mutating either committed mechanism.
    pub fn prepare_update(
        &self,
        buffer: &EditorBuffer,
        previous_source: &[u8],
    ) -> Result<PendingRustIntelligenceUpdate, RustIntelligenceError> {
        let parsed = self.parsed.prepare_update(buffer, previous_source)?;
        let language = self.language.prepare_update(buffer)?;
        if parsed.next_content_version().get() != language.lsp_document().version().get() as u64 {
            return Err(RustIntelligenceError::GenerationMismatch);
        }
        Ok(PendingRustIntelligenceUpdate { parsed, language })
    }

    /// Commits a prepared generation after the caller successfully sent its LSP
    /// notification. Both pending halves are validated before either is changed.
    pub fn commit_syntax_only(
        &mut self,
        update: PendingRustIntelligenceUpdate,
    ) -> Result<(), RustIntelligenceError> {
        self.parsed.validate_pending(&update.parsed)?;
        self.language.validate_pending(&update.language)?;
        self.parsed.commit_update(update.parsed)?;
        Ok(())
    }

    pub fn prepare_language_resync(
        &self,
        buffer: &EditorBuffer,
    ) -> Result<PendingLanguageUpdate, RustIntelligenceError> {
        Ok(self.language.prepare_update(buffer)?)
    }

    pub fn commit_language_resync(
        &mut self,
        update: PendingLanguageUpdate,
    ) -> Result<(), RustIntelligenceError> {
        self.language.validate_pending(&update)?;
        let next_document = update.lsp_document().clone();
        self.language.commit_update(update)?;
        self.lsp_document = next_document;
        Ok(())
    }

    pub fn commit_update(
        &mut self,
        update: PendingRustIntelligenceUpdate,
    ) -> Result<(), RustIntelligenceError> {
        self.parsed.validate_pending(&update.parsed)?;
        self.language.validate_pending(&update.language)?;
        let next_document = update.language.lsp_document().clone();
        self.parsed.commit_update(update.parsed)?;
        self.language.commit_update(update.language)?;
        self.lsp_document = next_document;
        Ok(())
    }

    pub fn validate_diagnostics(
        &self,
        diagnostics: &PublishedDiagnostics,
    ) -> Result<(), RustIntelligenceError> {
        Ok(self.language.validate_diagnostics(diagnostics)?)
    }

    pub fn validate_definition(
        &self,
        result: &DefinitionResult,
    ) -> Result<(), RustIntelligenceError> {
        Ok(self.language.validate_definition(result)?)
    }

    pub fn validate_completion(
        &self,
        result: &CompletionResult,
    ) -> Result<(), RustIntelligenceError> {
        Ok(self.language.validate_completion(result)?)
    }
}

/// Whether syntax and Rust Analyzer document state target the same generation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RustIntelligenceStatus {
    Current {
        syntax_version: ContentVersion,
    },
    LanguageDegraded {
        syntax_version: ContentVersion,
        language_version: ContentVersion,
    },
}

/// Prepared syntax and language-document transition for one exact generation.
pub struct PendingRustIntelligenceUpdate {
    parsed: PendingBufferParse,
    language: PendingLanguageUpdate,
}

impl PendingRustIntelligenceUpdate {
    pub fn lsp_document(&self) -> &LspDocument {
        self.language.lsp_document()
    }
}

/// Failure to bind parser and language state to one buffer generation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RustIntelligenceError {
    Parse(BufferParseError),
    Language(LanguageDocumentError),
    GenerationMismatch,
}

impl fmt::Display for RustIntelligenceError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Parse(error) => write!(formatter, "Rust syntax state failed: {error}"),
            Self::Language(error) => write!(formatter, "Rust language state failed: {error}"),
            Self::GenerationMismatch => {
                formatter.write_str("Rust syntax and language generations disagree")
            }
        }
    }
}

impl std::error::Error for RustIntelligenceError {}

impl From<BufferParseError> for RustIntelligenceError {
    fn from(error: BufferParseError) -> Self {
        Self::Parse(error)
    }
}

impl From<LanguageDocumentError> for RustIntelligenceError {
    fn from(error: LanguageDocumentError) -> Self {
        Self::Language(error)
    }
}
