use super::types::{DocumentVersion, LspPosition, LspRange};
use forge_protocol::identities::{ProjectId, RepositoryId};
use forge_protocol::paths::RepositoryRelativePath;

/// One workspace-contained source location returned by Rust Analyzer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LspLocation {
    relative_path: RepositoryRelativePath,
    range: LspRange,
}

impl LspLocation {
    pub(super) fn new(relative_path: RepositoryRelativePath, range: LspRange) -> Self {
        Self {
            relative_path,
            range,
        }
    }

    pub fn relative_path(&self) -> &RepositoryRelativePath {
        &self.relative_path
    }

    pub const fn range(&self) -> LspRange {
        self.range
    }
}

/// Definition locations bound to the exact source document generation queried.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DefinitionResult {
    pub(super) project_id: ProjectId,
    pub(super) repository_id: RepositoryId,
    pub(super) source_path: RepositoryRelativePath,
    pub(super) source_version: DocumentVersion,
    pub(super) locations: Vec<LspLocation>,
}

impl DefinitionResult {
    pub const fn project_id(&self) -> ProjectId {
        self.project_id
    }

    pub const fn repository_id(&self) -> RepositoryId {
        self.repository_id
    }

    pub fn source_path(&self) -> &RepositoryRelativePath {
        &self.source_path
    }

    pub const fn source_version(&self) -> DocumentVersion {
        self.source_version
    }

    pub fn locations(&self) -> &[LspLocation] {
        &self.locations
    }
}

/// One native completion item returned by Rust Analyzer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompletionItem {
    pub(super) label: String,
    pub(super) detail: Option<String>,
    pub(super) insert_text: Option<String>,
    pub(super) sort_text: Option<String>,
    pub(super) kind: Option<u32>,
}

impl CompletionItem {
    pub fn label(&self) -> &str {
        &self.label
    }

    pub fn detail(&self) -> Option<&str> {
        self.detail.as_deref()
    }

    pub fn insert_text(&self) -> Option<&str> {
        self.insert_text.as_deref()
    }

    pub fn sort_text(&self) -> Option<&str> {
        self.sort_text.as_deref()
    }

    pub const fn kind(&self) -> Option<u32> {
        self.kind
    }
}

/// Completion results bound to the exact source document generation queried.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompletionResult {
    pub(super) project_id: ProjectId,
    pub(super) repository_id: RepositoryId,
    pub(super) source_path: RepositoryRelativePath,
    pub(super) source_version: DocumentVersion,
    pub(super) is_incomplete: bool,
    pub(super) items: Vec<CompletionItem>,
}

impl CompletionResult {
    pub const fn project_id(&self) -> ProjectId {
        self.project_id
    }

    pub const fn repository_id(&self) -> RepositoryId {
        self.repository_id
    }

    pub fn source_path(&self) -> &RepositoryRelativePath {
        &self.source_path
    }

    pub const fn source_version(&self) -> DocumentVersion {
        self.source_version
    }

    pub const fn is_incomplete(&self) -> bool {
        self.is_incomplete
    }

    pub fn items(&self) -> &[CompletionItem] {
        &self.items
    }
}

/// One workspace symbol returned by Rust Analyzer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceSymbol {
    pub(super) name: String,
    pub(super) kind: u32,
    pub(super) container_name: Option<String>,
    pub(super) location: LspLocation,
}

impl WorkspaceSymbol {
    pub fn name(&self) -> &str {
        &self.name
    }

    pub const fn kind(&self) -> u32 {
        self.kind
    }

    pub fn container_name(&self) -> Option<&str> {
        self.container_name.as_deref()
    }

    pub fn location(&self) -> &LspLocation {
        &self.location
    }
}

/// Workspace symbol results bound to one initialized client generation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceSymbolResult {
    pub(super) project_id: ProjectId,
    pub(super) repository_id: RepositoryId,
    pub(super) client_generation: u64,
    pub(super) query: String,
    pub(super) symbols: Vec<WorkspaceSymbol>,
}

impl WorkspaceSymbolResult {
    pub const fn project_id(&self) -> ProjectId {
        self.project_id
    }

    pub const fn repository_id(&self) -> RepositoryId {
        self.repository_id
    }

    pub const fn client_generation(&self) -> u64 {
        self.client_generation
    }

    pub fn query(&self) -> &str {
        &self.query
    }

    pub fn symbols(&self) -> &[WorkspaceSymbol] {
        &self.symbols
    }
}

pub(super) fn location_sort_key(
    location: &LspLocation,
) -> (&std::path::Path, LspPosition, LspPosition) {
    (
        location.relative_path().as_path(),
        location.range().start(),
        location.range().end(),
    )
}
