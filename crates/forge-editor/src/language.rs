//! Editor-owned Rust language document generations.
//!
//! This layer converts one exact [`EditorBuffer`] generation into the immutable
//! text-document payload consumed by the Rust Analyzer adapter. It never starts a
//! process, reads a file, or invents diagnostics. Published diagnostics are accepted
//! only when their project, repository, path, and version still match the buffer.

use crate::buffers::{BufferId, ContentVersion, DocumentKey, EditorBuffer};
use forge_bridge::lsp::{
    CompletionResult, DefinitionResult, DocumentVersion, LspDocument, PublishedDiagnostics,
};
use forge_protocol::identities::ProjectId;
use std::fmt;

/// Rust language state bound to one editor buffer and content generation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RustLanguageDocument {
    project_id: ProjectId,
    buffer_id: BufferId,
    document: DocumentKey,
    content_version: ContentVersion,
    lsp_version: DocumentVersion,
}

impl RustLanguageDocument {
    /// Creates the first exact LSP payload for one UTF-8 Rust buffer.
    pub fn open(
        project_id: ProjectId,
        buffer: &EditorBuffer,
    ) -> Result<(Self, LspDocument), LanguageDocumentError> {
        let payload = payload_for(project_id, buffer)?;
        Ok((
            Self {
                project_id,
                buffer_id: buffer.id(),
                document: buffer.document().clone(),
                content_version: buffer.content_version(),
                lsp_version: payload.version(),
            },
            payload,
        ))
    }

    pub const fn project_id(&self) -> ProjectId {
        self.project_id
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

    pub const fn lsp_version(&self) -> DocumentVersion {
        self.lsp_version
    }

    /// Prepares a strictly newer payload without advancing committed language state.
    ///
    /// The caller sends [`PendingLanguageUpdate::lsp_document`] through the LSP adapter
    /// and commits this token only after the notification write succeeds.
    pub fn prepare_update(
        &self,
        buffer: &EditorBuffer,
    ) -> Result<PendingLanguageUpdate, LanguageDocumentError> {
        self.validate_identity(buffer)?;
        if buffer.content_version() <= self.content_version {
            return Err(LanguageDocumentError::NonAdvancingVersion {
                current: self.content_version,
                requested: buffer.content_version(),
            });
        }
        Ok(PendingLanguageUpdate {
            project_id: self.project_id,
            buffer_id: self.buffer_id,
            document: self.document.clone(),
            expected_content_version: self.content_version,
            expected_lsp_version: self.lsp_version,
            next_content_version: buffer.content_version(),
            lsp_document: payload_for(self.project_id, buffer)?,
        })
    }

    /// Commits one prepared update after the adapter accepted its notification write.
    pub fn commit_update(
        &mut self,
        update: PendingLanguageUpdate,
    ) -> Result<(), LanguageDocumentError> {
        self.validate_pending(&update)?;
        self.content_version = update.next_content_version;
        self.lsp_version = update.lsp_document.version();
        Ok(())
    }

    pub(crate) fn validate_pending(
        &self,
        update: &PendingLanguageUpdate,
    ) -> Result<(), LanguageDocumentError> {
        if update.project_id != self.project_id
            || update.buffer_id != self.buffer_id
            || update.document != self.document
            || update.expected_content_version != self.content_version
            || update.expected_lsp_version != self.lsp_version
            || update.lsp_document.project_id() != self.project_id
            || update.lsp_document.repository_id() != self.document.repository_id()
            || update.lsp_document.relative_path() != self.document.relative_path()
            || update.lsp_document.version().get() as u64 != update.next_content_version.get()
        {
            return Err(LanguageDocumentError::PendingUpdateMismatch);
        }
        Ok(())
    }

    pub fn validate_definition(
        &self,
        result: &DefinitionResult,
    ) -> Result<(), LanguageDocumentError> {
        self.validate_feature_result(
            result.project_id(),
            result.repository_id(),
            result.source_path(),
            result.source_version(),
        )
    }

    pub fn validate_completion(
        &self,
        result: &CompletionResult,
    ) -> Result<(), LanguageDocumentError> {
        self.validate_feature_result(
            result.project_id(),
            result.repository_id(),
            result.source_path(),
            result.source_version(),
        )
    }

    /// Rejects diagnostics from another project, file, or stale generation.
    pub fn validate_diagnostics(
        &self,
        diagnostics: &PublishedDiagnostics,
    ) -> Result<(), LanguageDocumentError> {
        if diagnostics.project_id() != self.project_id {
            return Err(LanguageDocumentError::ProjectMismatch);
        }
        if diagnostics.repository_id() != self.document.repository_id()
            || diagnostics.relative_path() != self.document.relative_path()
        {
            return Err(LanguageDocumentError::DocumentMismatch);
        }
        if diagnostics.version() != self.lsp_version {
            return Err(LanguageDocumentError::StaleDiagnostics {
                current: self.lsp_version,
                received: diagnostics.version(),
            });
        }
        Ok(())
    }

    fn validate_feature_result(
        &self,
        project_id: ProjectId,
        repository_id: forge_protocol::identities::RepositoryId,
        relative_path: &forge_protocol::paths::RepositoryRelativePath,
        version: DocumentVersion,
    ) -> Result<(), LanguageDocumentError> {
        if project_id != self.project_id {
            return Err(LanguageDocumentError::ProjectMismatch);
        }
        if repository_id != self.document.repository_id()
            || relative_path != self.document.relative_path()
        {
            return Err(LanguageDocumentError::DocumentMismatch);
        }
        if version != self.lsp_version {
            return Err(LanguageDocumentError::StaleFeatureResult {
                current: self.lsp_version,
                received: version,
            });
        }
        Ok(())
    }

    fn validate_identity(&self, buffer: &EditorBuffer) -> Result<(), LanguageDocumentError> {
        if buffer.id() != self.buffer_id {
            return Err(LanguageDocumentError::BufferMismatch {
                expected: self.buffer_id,
                actual: buffer.id(),
            });
        }
        if buffer.document() != &self.document {
            return Err(LanguageDocumentError::DocumentMismatch);
        }
        Ok(())
    }
}

/// One exact editor generation prepared for LSP delivery but not yet committed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PendingLanguageUpdate {
    project_id: ProjectId,
    buffer_id: BufferId,
    document: DocumentKey,
    expected_content_version: ContentVersion,
    expected_lsp_version: DocumentVersion,
    next_content_version: ContentVersion,
    lsp_document: LspDocument,
}

impl PendingLanguageUpdate {
    pub fn lsp_document(&self) -> &LspDocument {
        &self.lsp_document
    }
}

/// Exact reason an editor generation could not cross the LSP seam.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LanguageDocumentError {
    InvalidUtf8 {
        valid_up_to: usize,
    },
    InvalidVersion(u64),
    BufferMismatch {
        expected: BufferId,
        actual: BufferId,
    },
    DocumentMismatch,
    ProjectMismatch,
    PendingUpdateMismatch,
    NonAdvancingVersion {
        current: ContentVersion,
        requested: ContentVersion,
    },
    StaleDiagnostics {
        current: DocumentVersion,
        received: DocumentVersion,
    },
    StaleFeatureResult {
        current: DocumentVersion,
        received: DocumentVersion,
    },
}

impl fmt::Display for LanguageDocumentError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidUtf8 { valid_up_to } => {
                write!(
                    formatter,
                    "Rust LSP text is not UTF-8 at byte {valid_up_to}"
                )
            }
            Self::InvalidVersion(version) => {
                write!(
                    formatter,
                    "buffer generation {version} cannot be represented by LSP"
                )
            }
            Self::BufferMismatch { expected, actual } => write!(
                formatter,
                "language state belongs to buffer {expected}, not buffer {actual}"
            ),
            Self::DocumentMismatch => {
                formatter.write_str("language document identity does not match")
            }
            Self::ProjectMismatch => {
                formatter.write_str("language project identity does not match")
            }
            Self::PendingUpdateMismatch => {
                formatter.write_str("pending language update no longer matches committed state")
            }
            Self::NonAdvancingVersion { current, requested } => write!(
                formatter,
                "language generation {} does not advance current generation {}",
                requested.get(),
                current.get()
            ),
            Self::StaleDiagnostics { current, received } => write!(
                formatter,
                "diagnostics generation {} does not match current generation {}",
                received.get(),
                current.get()
            ),
            Self::StaleFeatureResult { current, received } => write!(
                formatter,
                "language feature generation {} does not match current generation {}",
                received.get(),
                current.get()
            ),
        }
    }
}

impl std::error::Error for LanguageDocumentError {}

fn payload_for(
    project_id: ProjectId,
    buffer: &EditorBuffer,
) -> Result<LspDocument, LanguageDocumentError> {
    let text = std::str::from_utf8(buffer.bytes()).map_err(|error| {
        LanguageDocumentError::InvalidUtf8 {
            valid_up_to: error.valid_up_to(),
        }
    })?;
    let version = DocumentVersion::new(buffer.content_version().get())
        .map_err(|_| LanguageDocumentError::InvalidVersion(buffer.content_version().get()))?;
    LspDocument::new(
        project_id,
        buffer.document().repository_id(),
        buffer.document().relative_path().clone(),
        version,
        "rust",
        text,
    )
    .map_err(|_| LanguageDocumentError::InvalidVersion(buffer.content_version().get()))
}
