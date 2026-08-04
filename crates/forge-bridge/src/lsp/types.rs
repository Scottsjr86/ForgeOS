use forge_protocol::identities::{ProcessId, ProjectId, RepositoryId};
use forge_protocol::paths::RepositoryRelativePath;
use forge_protocol::processes::{ProcessRequestError, ProcessSpawnRequest};
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

/// Positive document generation carried over the LSP wire.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DocumentVersion(i32);

impl DocumentVersion {
    pub fn new(value: u64) -> Result<Self, LspError> {
        let value = i32::try_from(value).map_err(|_| LspError::InvalidDocumentVersion(value))?;
        if value <= 0 {
            return Err(LspError::InvalidDocumentVersion(value as u64));
        }
        Ok(Self(value))
    }

    pub const fn get(self) -> i32 {
        self.0
    }
}

/// Exact text-document payload sent to Rust Analyzer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LspDocument {
    project_id: ProjectId,
    repository_id: RepositoryId,
    relative_path: RepositoryRelativePath,
    version: DocumentVersion,
    language_id: String,
    text: String,
}

impl LspDocument {
    pub fn new(
        project_id: ProjectId,
        repository_id: RepositoryId,
        relative_path: RepositoryRelativePath,
        version: DocumentVersion,
        language_id: impl Into<String>,
        text: impl Into<String>,
    ) -> Result<Self, LspError> {
        let language_id = language_id.into();
        if language_id.is_empty() || language_id.as_bytes().contains(&0) {
            return Err(LspError::InvalidLanguageId);
        }
        Ok(Self {
            project_id,
            repository_id,
            relative_path,
            version,
            language_id,
            text: text.into(),
        })
    }

    pub const fn project_id(&self) -> ProjectId {
        self.project_id
    }

    pub const fn repository_id(&self) -> RepositoryId {
        self.repository_id
    }

    pub fn relative_path(&self) -> &RepositoryRelativePath {
        &self.relative_path
    }

    pub const fn version(&self) -> DocumentVersion {
        self.version
    }

    pub fn language_id(&self) -> &str {
        &self.language_id
    }

    pub fn text(&self) -> &str {
        &self.text
    }
}

/// One LSP source position.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct LspPosition {
    line: u32,
    character: u32,
}

impl LspPosition {
    pub const fn new(line: u32, character: u32) -> Self {
        Self { line, character }
    }

    pub const fn line(self) -> u32 {
        self.line
    }

    pub const fn character(self) -> u32 {
        self.character
    }
}

/// One LSP source range.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct LspRange {
    start: LspPosition,
    end: LspPosition,
}

impl LspRange {
    pub const fn new(start: LspPosition, end: LspPosition) -> Self {
        Self { start, end }
    }

    pub const fn start(self) -> LspPosition {
        self.start
    }

    pub const fn end(self) -> LspPosition {
        self.end
    }
}

/// One native diagnostic published by Rust Analyzer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LspDiagnostic {
    pub(super) range: LspRange,
    pub(super) severity: Option<u8>,
    pub(super) code: Option<String>,
    pub(super) source: Option<String>,
    pub(super) message: String,
}

impl LspDiagnostic {
    pub const fn range(&self) -> LspRange {
        self.range
    }

    pub const fn severity(&self) -> Option<u8> {
        self.severity
    }

    pub fn code(&self) -> Option<&str> {
        self.code.as_deref()
    }

    pub fn source(&self) -> Option<&str> {
        self.source.as_deref()
    }

    pub fn message(&self) -> &str {
        &self.message
    }
}

/// Diagnostics proven to target one tracked project document generation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PublishedDiagnostics {
    pub(super) project_id: ProjectId,
    pub(super) repository_id: RepositoryId,
    pub(super) relative_path: RepositoryRelativePath,
    pub(super) version: DocumentVersion,
    pub(super) diagnostics: Vec<LspDiagnostic>,
}

impl PublishedDiagnostics {
    pub const fn project_id(&self) -> ProjectId {
        self.project_id
    }

    pub const fn repository_id(&self) -> RepositoryId {
        self.repository_id
    }

    pub fn relative_path(&self) -> &RepositoryRelativePath {
        &self.relative_path
    }

    pub const fn version(&self) -> DocumentVersion {
        self.version
    }

    pub fn diagnostics(&self) -> &[LspDiagnostic] {
        &self.diagnostics
    }
}

/// LSP text synchronization mode accepted by the server.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextDocumentSyncKind {
    Full,
    Incremental,
}

/// Reviewed capabilities reported by the initialized server.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct RustAnalyzerCapabilities {
    pub(super) text_document_sync: Option<TextDocumentSyncKind>,
    pub(super) hover: bool,
    pub(super) definition: bool,
    pub(super) completion: bool,
    pub(super) workspace_symbol: bool,
}

impl RustAnalyzerCapabilities {
    pub const fn text_document_sync(self) -> Option<TextDocumentSyncKind> {
        self.text_document_sync
    }

    pub const fn full_document_sync(self) -> bool {
        matches!(self.text_document_sync, Some(TextDocumentSyncKind::Full))
    }

    pub const fn incremental_document_sync(self) -> bool {
        matches!(
            self.text_document_sync,
            Some(TextDocumentSyncKind::Incremental)
        )
    }

    pub const fn hover(self) -> bool {
        self.hover
    }

    pub const fn definition(self) -> bool {
        self.definition
    }

    pub const fn completion(self) -> bool {
        self.completion
    }

    pub const fn workspace_symbol(self) -> bool {
        self.workspace_symbol
    }
}

/// Exact executable and project boundary for one Rust Analyzer process.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RustAnalyzerConfig {
    process_id: ProcessId,
    project_id: ProjectId,
    repository_id: RepositoryId,
    executable: String,
    arguments: Vec<String>,
    workspace_root: PathBuf,
    request_timeout: Duration,
}

impl RustAnalyzerConfig {
    pub fn new<I, S>(
        process_id: ProcessId,
        project_id: ProjectId,
        repository_id: RepositoryId,
        executable: impl Into<String>,
        arguments: I,
        workspace_root: impl Into<PathBuf>,
    ) -> Result<Self, LspError>
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        let request = ProcessSpawnRequest::new(process_id, executable, arguments)
            .map_err(LspError::InvalidProcessRequest)?;
        let workspace_root = validate_workspace_root(workspace_root.into())?;
        Ok(Self {
            process_id,
            project_id,
            repository_id,
            executable: request.program().to_owned(),
            arguments: request.arguments().to_vec(),
            workspace_root,
            request_timeout: Duration::from_secs(5),
        })
    }

    pub fn with_request_timeout(mut self, timeout: Duration) -> Result<Self, LspError> {
        if timeout.is_zero() {
            return Err(LspError::ZeroRequestTimeout);
        }
        self.request_timeout = timeout;
        Ok(self)
    }

    pub const fn process_id(&self) -> ProcessId {
        self.process_id
    }

    pub const fn project_id(&self) -> ProjectId {
        self.project_id
    }

    pub const fn repository_id(&self) -> RepositoryId {
        self.repository_id
    }

    pub fn executable(&self) -> &str {
        &self.executable
    }

    pub fn arguments(&self) -> &[String] {
        &self.arguments
    }

    pub fn workspace_root(&self) -> &Path {
        &self.workspace_root
    }

    pub const fn request_timeout(&self) -> Duration {
        self.request_timeout
    }

    pub(super) fn with_process_id(&self, process_id: ProcessId) -> Self {
        let mut next = self.clone();
        next.process_id = process_id;
        next
    }
}

/// Stable transport failure classes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LspProtocolError {
    Io(String),
    MissingContentLength,
    DuplicateContentLength,
    InvalidContentLength,
    MessageTooLarge { length: usize },
    InvalidJson(String),
    InvalidMessageShape,
    UnexpectedResponse,
    UnexpectedResponseId,
}

impl fmt::Display for LspProtocolError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(message) => write!(formatter, "LSP transport I/O failed: {message}"),
            Self::MissingContentLength => formatter.write_str("LSP message has no Content-Length"),
            Self::DuplicateContentLength => {
                formatter.write_str("LSP message has duplicate Content-Length headers")
            }
            Self::InvalidContentLength => formatter.write_str("invalid LSP Content-Length"),
            Self::MessageTooLarge { length } => {
                write!(
                    formatter,
                    "LSP message length {length} exceeds the V1 limit"
                )
            }
            Self::InvalidJson(message) => write!(formatter, "invalid LSP JSON: {message}"),
            Self::InvalidMessageShape => formatter.write_str("invalid JSON-RPC message shape"),
            Self::UnexpectedResponse => formatter.write_str("unexpected JSON-RPC response"),
            Self::UnexpectedResponseId => {
                formatter.write_str("JSON-RPC response ID did not match the active request")
            }
        }
    }
}

impl std::error::Error for LspProtocolError {}

/// Exact adapter failure without collapsing native or protocol distinctions.
#[derive(Debug)]
pub enum LspError {
    InvalidProcessRequest(ProcessRequestError),
    ZeroRequestTimeout,
    InvalidWorkspaceRoot(PathBuf),
    Spawn(String),
    Termination(String),
    Protocol(LspProtocolError),
    Timeout,
    ServerExited(String),
    Remote {
        code: i64,
        message: String,
    },
    InvalidInitializeResult,
    UnsupportedCapability(&'static str),
    ProjectMismatch {
        expected: ProjectId,
        actual: ProjectId,
    },
    RepositoryMismatch {
        expected: RepositoryId,
        actual: RepositoryId,
    },
    InvalidDocumentVersion(u64),
    InvalidDiagnosticVersion(i64),
    MissingDiagnosticVersion,
    InvalidLanguageId,
    InvalidMethod,
    InvalidDocumentUri,
    ResultOutsideWorkspace(String),
    InvalidSymbolQuery,
    DocumentPositionOverflow,
    DocumentAlreadyOpen,
    DocumentNotOpen,
    PreviousDocumentMismatch,
    StaleDocumentVersion {
        current: DocumentVersion,
        requested: DocumentVersion,
    },
    UntrackedDiagnosticDocument(String),
    StaleDiagnostics {
        current: DocumentVersion,
        received: DocumentVersion,
    },
    ProcessIdentityReused(ProcessId),
    RequestIdExhausted,
    GenerationExhausted,
}

impl fmt::Display for LspError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidProcessRequest(error) => {
                write!(formatter, "invalid process request: {error}")
            }
            Self::ZeroRequestTimeout => formatter.write_str("LSP request timeout must be nonzero"),
            Self::InvalidWorkspaceRoot(path) => {
                write!(
                    formatter,
                    "invalid canonical workspace root {}",
                    path.display()
                )
            }
            Self::Spawn(message) => write!(formatter, "Rust Analyzer spawn failed: {message}"),
            Self::Termination(message) => {
                write!(formatter, "Rust Analyzer termination failed: {message}")
            }
            Self::Protocol(error) => write!(formatter, "{error}"),
            Self::Timeout => formatter.write_str("timed out waiting for Rust Analyzer"),
            Self::ServerExited(stderr) if stderr.is_empty() => {
                formatter.write_str("Rust Analyzer exited before completing the request")
            }
            Self::ServerExited(stderr) => write!(formatter, "Rust Analyzer exited: {stderr}"),
            Self::Remote { code, message } => {
                write!(formatter, "Rust Analyzer JSON-RPC error {code}: {message}")
            }
            Self::InvalidInitializeResult => {
                formatter.write_str("Rust Analyzer returned an invalid initialize result")
            }
            Self::UnsupportedCapability(method) => {
                write!(formatter, "Rust Analyzer does not support {method}")
            }
            Self::ProjectMismatch { expected, actual } => {
                write!(
                    formatter,
                    "LSP project mismatch: expected {expected}, got {actual}"
                )
            }
            Self::RepositoryMismatch { expected, actual } => write!(
                formatter,
                "LSP repository mismatch: expected {expected}, got {actual}"
            ),
            Self::InvalidDocumentVersion(version) => {
                write!(formatter, "invalid LSP document version {version}")
            }
            Self::InvalidDiagnosticVersion(version) => {
                write!(formatter, "invalid diagnostic version {version}")
            }
            Self::MissingDiagnosticVersion => {
                formatter.write_str("diagnostics omitted the required document version")
            }
            Self::InvalidLanguageId => formatter.write_str("invalid LSP language ID"),
            Self::InvalidMethod => formatter.write_str("invalid JSON-RPC method"),
            Self::InvalidDocumentUri => formatter.write_str("invalid LSP document URI"),
            Self::ResultOutsideWorkspace(uri) => {
                write!(
                    formatter,
                    "LSP result targets a path outside the workspace: {uri}"
                )
            }
            Self::InvalidSymbolQuery => {
                formatter.write_str("workspace symbol query contains a forbidden NUL byte")
            }
            Self::DocumentPositionOverflow => {
                formatter.write_str("LSP document position exceeds the V1 integer limit")
            }
            Self::DocumentAlreadyOpen => formatter.write_str("LSP document is already open"),
            Self::DocumentNotOpen => formatter.write_str("LSP document is not open"),
            Self::PreviousDocumentMismatch => {
                formatter.write_str("previous LSP document does not match the tracked generation")
            }
            Self::StaleDocumentVersion { current, requested } => write!(
                formatter,
                "LSP document version {} does not advance current version {}",
                requested.get(),
                current.get()
            ),
            Self::UntrackedDiagnosticDocument(uri) => {
                write!(formatter, "diagnostics target untracked document {uri}")
            }
            Self::StaleDiagnostics { current, received } => write!(
                formatter,
                "diagnostics version {} does not match current version {}",
                received.get(),
                current.get()
            ),
            Self::ProcessIdentityReused(process_id) => {
                write!(formatter, "restart reused process identity {process_id}")
            }
            Self::RequestIdExhausted => formatter.write_str("JSON-RPC request IDs exhausted"),
            Self::GenerationExhausted => {
                formatter.write_str("Rust Analyzer restart generation exhausted")
            }
        }
    }
}

impl std::error::Error for LspError {}

fn validate_workspace_root(path: PathBuf) -> Result<PathBuf, LspError> {
    if !path.is_absolute() {
        return Err(LspError::InvalidWorkspaceRoot(path));
    }
    let metadata =
        fs::symlink_metadata(&path).map_err(|_| LspError::InvalidWorkspaceRoot(path.clone()))?;
    if !metadata.is_dir() || metadata.file_type().is_symlink() {
        return Err(LspError::InvalidWorkspaceRoot(path));
    }
    let canonical =
        fs::canonicalize(&path).map_err(|_| LspError::InvalidWorkspaceRoot(path.clone()))?;
    if canonical != path {
        return Err(LspError::InvalidWorkspaceRoot(path));
    }
    Ok(path)
}
