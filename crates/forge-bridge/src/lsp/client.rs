use super::protocol::LspConnection;
use super::types::{
    DocumentVersion, LspDiagnostic, LspDocument, LspError, LspPosition, LspProtocolError, LspRange,
    PublishedDiagnostics, RustAnalyzerCapabilities, RustAnalyzerConfig, TextDocumentSyncKind,
};
use forge_protocol::identities::{ProcessId, ProjectId, RepositoryId};
use forge_protocol::paths::RepositoryRelativePath;
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::path::Path;
use std::time::Duration;

/// One initialized Rust Analyzer client bound to a single project boundary.
pub struct RustAnalyzerClient {
    config: RustAnalyzerConfig,
    connection: LspConnection,
    capabilities: RustAnalyzerCapabilities,
    documents: BTreeMap<String, TrackedDocument>,
    generation: u64,
}

impl RustAnalyzerClient {
    pub fn start(config: RustAnalyzerConfig) -> Result<Self, LspError> {
        let mut connection = LspConnection::spawn(&config)?;
        let capabilities = initialize(&mut connection, &config)?;
        Ok(Self {
            config,
            connection,
            capabilities,
            documents: BTreeMap::new(),
            generation: 1,
        })
    }

    pub const fn process_id(&self) -> ProcessId {
        self.config.process_id()
    }

    pub fn system_pid(&self) -> u32 {
        self.connection.system_pid
    }

    pub const fn generation(&self) -> u64 {
        self.generation
    }

    pub const fn capabilities(&self) -> RustAnalyzerCapabilities {
        self.capabilities
    }

    pub fn open_document(&mut self, document: &LspDocument) -> Result<(), LspError> {
        self.validate_document_boundary(document)?;
        let uri = document_uri(self.config.workspace_root(), document.relative_path())?;
        if self.documents.contains_key(&uri) {
            return Err(LspError::DocumentAlreadyOpen);
        }
        self.connection.notify(
            "textDocument/didOpen",
            json!({
                "textDocument": {
                    "uri": uri,
                    "languageId": document.language_id(),
                    "version": document.version().get(),
                    "text": document.text(),
                }
            }),
        )?;
        self.documents.insert(
            uri,
            TrackedDocument {
                project_id: document.project_id(),
                repository_id: document.repository_id(),
                relative_path: document.relative_path().clone(),
                version: document.version(),
            },
        );
        Ok(())
    }

    pub fn change_document(
        &mut self,
        previous: &LspDocument,
        document: &LspDocument,
    ) -> Result<(), LspError> {
        self.validate_document_boundary(previous)?;
        self.validate_document_boundary(document)?;
        if previous.project_id() != document.project_id()
            || previous.repository_id() != document.repository_id()
            || previous.relative_path() != document.relative_path()
        {
            return Err(LspError::PreviousDocumentMismatch);
        }
        let uri = document_uri(self.config.workspace_root(), document.relative_path())?;
        let tracked = self.documents.get(&uri).ok_or(LspError::DocumentNotOpen)?;
        if previous.version() != tracked.version {
            return Err(LspError::PreviousDocumentMismatch);
        }
        if document.version() <= tracked.version {
            return Err(LspError::StaleDocumentVersion {
                current: tracked.version,
                requested: document.version(),
            });
        }
        let content_change = match self.capabilities.text_document_sync() {
            Some(TextDocumentSyncKind::Full) => json!({"text": document.text()}),
            Some(TextDocumentSyncKind::Incremental) => {
                let (end, range_length) = utf16_document_extent(previous.text())?;
                json!({
                    "range": {
                        "start": {"line": 0, "character": 0},
                        "end": {"line": end.line(), "character": end.character()},
                    },
                    "rangeLength": range_length,
                    "text": document.text(),
                })
            }
            None => return Err(LspError::UnsupportedCapability("textDocumentSync")),
        };
        self.connection.notify(
            "textDocument/didChange",
            json!({
                "textDocument": {"uri": uri, "version": document.version().get()},
                "contentChanges": [content_change],
            }),
        )?;
        self.documents
            .get_mut(&uri)
            .expect("tracked document was checked before notification")
            .version = document.version();
        Ok(())
    }

    pub fn close_document(&mut self, document: &LspDocument) -> Result<(), LspError> {
        self.validate_document_boundary(document)?;
        let uri = document_uri(self.config.workspace_root(), document.relative_path())?;
        let tracked = self.documents.get(&uri).ok_or(LspError::DocumentNotOpen)?;
        if tracked.version != document.version() {
            return Err(LspError::StaleDocumentVersion {
                current: tracked.version,
                requested: document.version(),
            });
        }
        self.connection.notify(
            "textDocument/didClose",
            json!({"textDocument": {"uri": uri}}),
        )?;
        self.documents.remove(&uri);
        Ok(())
    }

    pub fn wait_for_diagnostics(
        &mut self,
        timeout: Duration,
    ) -> Result<PublishedDiagnostics, LspError> {
        let params = self
            .connection
            .wait_for_notification("textDocument/publishDiagnostics", timeout)?;
        let object = params
            .as_object()
            .ok_or_else(|| LspError::Protocol(LspProtocolError::InvalidMessageShape))?;
        let uri = required_str(object.get("uri"))?;
        let version_value = object
            .get("version")
            .and_then(Value::as_i64)
            .ok_or(LspError::MissingDiagnosticVersion)?;
        let version = u64::try_from(version_value)
            .ok()
            .and_then(|value| DocumentVersion::new(value).ok())
            .ok_or(LspError::InvalidDiagnosticVersion(version_value))?;
        let tracked = self
            .documents
            .get(uri)
            .ok_or_else(|| LspError::UntrackedDiagnosticDocument(uri.to_owned()))?;
        if version != tracked.version {
            return Err(LspError::StaleDiagnostics {
                current: tracked.version,
                received: version,
            });
        }
        let diagnostics = object
            .get("diagnostics")
            .and_then(Value::as_array)
            .ok_or_else(|| LspError::Protocol(LspProtocolError::InvalidMessageShape))?
            .iter()
            .map(parse_diagnostic)
            .collect::<Result<Vec<_>, _>>()?;
        Ok(PublishedDiagnostics {
            project_id: tracked.project_id,
            repository_id: tracked.repository_id,
            relative_path: tracked.relative_path.clone(),
            version,
            diagnostics,
        })
    }

    pub fn request_hover(
        &mut self,
        document: &LspDocument,
        position: LspPosition,
    ) -> Result<Value, LspError> {
        if !self.capabilities.hover() {
            return Err(LspError::UnsupportedCapability("textDocument/hover"));
        }
        self.validate_current_document(document)?;
        let uri = document_uri(self.config.workspace_root(), document.relative_path())?;
        self.connection.request(
            "textDocument/hover",
            json!({
                "textDocument": {"uri": uri},
                "position": {"line": position.line(), "character": position.character()},
            }),
            self.config.request_timeout(),
        )
    }

    pub fn restart(&mut self, process_id: ProcessId) -> Result<(), LspError> {
        if process_id == self.config.process_id() {
            return Err(LspError::ProcessIdentityReused(process_id));
        }
        self.connection.shutdown()?;
        self.documents.clear();
        let config = self.config.with_process_id(process_id);
        let mut connection = LspConnection::spawn(&config)?;
        let capabilities = initialize(&mut connection, &config)?;
        self.config = config;
        self.connection = connection;
        self.capabilities = capabilities;
        self.documents.clear();
        self.generation = self
            .generation
            .checked_add(1)
            .ok_or(LspError::GenerationExhausted)?;
        Ok(())
    }

    pub fn close(mut self) -> Result<(), LspError> {
        self.connection.shutdown()
    }

    fn validate_document_boundary(&self, document: &LspDocument) -> Result<(), LspError> {
        if document.project_id() != self.config.project_id() {
            return Err(LspError::ProjectMismatch {
                expected: self.config.project_id(),
                actual: document.project_id(),
            });
        }
        if document.repository_id() != self.config.repository_id() {
            return Err(LspError::RepositoryMismatch {
                expected: self.config.repository_id(),
                actual: document.repository_id(),
            });
        }
        Ok(())
    }

    fn validate_current_document(&self, document: &LspDocument) -> Result<(), LspError> {
        self.validate_document_boundary(document)?;
        let uri = document_uri(self.config.workspace_root(), document.relative_path())?;
        let tracked = self.documents.get(&uri).ok_or(LspError::DocumentNotOpen)?;
        if tracked.version != document.version() {
            return Err(LspError::StaleDocumentVersion {
                current: tracked.version,
                requested: document.version(),
            });
        }
        Ok(())
    }
}

#[derive(Debug, Clone)]
struct TrackedDocument {
    project_id: ProjectId,
    repository_id: RepositoryId,
    relative_path: RepositoryRelativePath,
    version: DocumentVersion,
}

fn initialize(
    connection: &mut LspConnection,
    config: &RustAnalyzerConfig,
) -> Result<RustAnalyzerCapabilities, LspError> {
    let root_uri = path_to_file_uri(config.workspace_root())?;
    connection.set_workspace_folder(root_uri.clone(), "ForgeOS project");
    let result = connection.request(
        "initialize",
        json!({
            "processId": Value::Null,
            "clientInfo": {"name": "ForgeOS", "version": env!("CARGO_PKG_VERSION")},
            "rootUri": root_uri,
            "capabilities": {
                "textDocument": {
                    "publishDiagnostics": {"versionSupport": true},
                    "synchronization": {"didSave": false, "dynamicRegistration": false}
                },
                "workspace": {"configuration": true, "workspaceFolders": true}
            },
            "workspaceFolders": [{"uri": root_uri, "name": "ForgeOS project"}],
        }),
        config.request_timeout(),
    )?;
    let capabilities = parse_capabilities(&result)?;
    if capabilities.text_document_sync().is_none() {
        return Err(LspError::UnsupportedCapability("textDocumentSync"));
    }
    connection.notify("initialized", json!({}))?;
    Ok(capabilities)
}

fn parse_capabilities(result: &Value) -> Result<RustAnalyzerCapabilities, LspError> {
    let capabilities = result
        .get("capabilities")
        .and_then(Value::as_object)
        .ok_or(LspError::InvalidInitializeResult)?;
    let sync_code = match capabilities.get("textDocumentSync") {
        Some(Value::Number(number)) => number.as_u64(),
        Some(Value::Object(sync)) => sync.get("change").and_then(Value::as_u64),
        _ => None,
    };
    let text_document_sync = match sync_code {
        Some(1) => Some(TextDocumentSyncKind::Full),
        Some(2) => Some(TextDocumentSyncKind::Incremental),
        _ => None,
    };
    Ok(RustAnalyzerCapabilities {
        text_document_sync,
        hover: capability_enabled(capabilities.get("hoverProvider")),
        definition: capability_enabled(capabilities.get("definitionProvider")),
        completion: capabilities
            .get("completionProvider")
            .is_some_and(|value| !value.is_null() && value != &Value::Bool(false)),
    })
}

fn capability_enabled(value: Option<&Value>) -> bool {
    value.is_some_and(|value| match value {
        Value::Bool(enabled) => *enabled,
        Value::Null => false,
        _ => true,
    })
}

fn parse_diagnostic(value: &Value) -> Result<LspDiagnostic, LspError> {
    let object = value
        .as_object()
        .ok_or_else(|| LspError::Protocol(LspProtocolError::InvalidMessageShape))?;
    let range = parse_range(
        object
            .get("range")
            .ok_or_else(|| LspError::Protocol(LspProtocolError::InvalidMessageShape))?,
    )?;
    let severity = object
        .get("severity")
        .and_then(Value::as_u64)
        .and_then(|value| u8::try_from(value).ok());
    let code = object.get("code").and_then(|value| match value {
        Value::String(value) => Some(value.clone()),
        Value::Number(value) => Some(value.to_string()),
        _ => None,
    });
    let source = object
        .get("source")
        .and_then(Value::as_str)
        .map(str::to_owned);
    let message = required_str(object.get("message"))?.to_owned();
    Ok(LspDiagnostic {
        range,
        severity,
        code,
        source,
        message,
    })
}

fn parse_range(value: &Value) -> Result<LspRange, LspError> {
    let object = value
        .as_object()
        .ok_or_else(|| LspError::Protocol(LspProtocolError::InvalidMessageShape))?;
    let start = parse_position(
        object
            .get("start")
            .ok_or_else(|| LspError::Protocol(LspProtocolError::InvalidMessageShape))?,
    )?;
    let end = parse_position(
        object
            .get("end")
            .ok_or_else(|| LspError::Protocol(LspProtocolError::InvalidMessageShape))?,
    )?;
    Ok(LspRange::new(start, end))
}

fn parse_position(value: &Value) -> Result<LspPosition, LspError> {
    let object = value
        .as_object()
        .ok_or_else(|| LspError::Protocol(LspProtocolError::InvalidMessageShape))?;
    let line = object
        .get("line")
        .and_then(Value::as_u64)
        .and_then(|value| u32::try_from(value).ok())
        .ok_or_else(|| LspError::Protocol(LspProtocolError::InvalidMessageShape))?;
    let character = object
        .get("character")
        .and_then(Value::as_u64)
        .and_then(|value| u32::try_from(value).ok())
        .ok_or_else(|| LspError::Protocol(LspProtocolError::InvalidMessageShape))?;
    Ok(LspPosition::new(line, character))
}

fn required_str(value: Option<&Value>) -> Result<&str, LspError> {
    value
        .and_then(Value::as_str)
        .ok_or_else(|| LspError::Protocol(LspProtocolError::InvalidMessageShape))
}

fn utf16_document_extent(text: &str) -> Result<(LspPosition, u32), LspError> {
    let mut line = 0_u32;
    let mut character = 0_u32;
    let mut length = 0_u32;
    for value in text.chars() {
        let units = u32::try_from(value.len_utf16())
            .expect("one Unicode scalar occupies at most two UTF-16 code units");
        length = length
            .checked_add(units)
            .ok_or(LspError::DocumentPositionOverflow)?;
        if value == '\n' {
            line = line
                .checked_add(1)
                .ok_or(LspError::DocumentPositionOverflow)?;
            character = 0;
        } else {
            character = character
                .checked_add(units)
                .ok_or(LspError::DocumentPositionOverflow)?;
        }
    }
    Ok((LspPosition::new(line, character), length))
}

fn document_uri(root: &Path, relative: &RepositoryRelativePath) -> Result<String, LspError> {
    path_to_file_uri(&root.join(relative.as_path()))
}

fn path_to_file_uri(path: &Path) -> Result<String, LspError> {
    if !path.is_absolute() {
        return Err(LspError::InvalidDocumentUri);
    }
    let bytes = path_bytes(path)?;
    let mut uri = String::from("file://");
    for byte in bytes {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'/' | b'-' | b'.' | b'_' | b'~') {
            uri.push(byte as char);
        } else {
            use std::fmt::Write as _;
            write!(&mut uri, "%{byte:02X}").expect("writing to String cannot fail");
        }
    }
    Ok(uri)
}

#[cfg(unix)]
fn path_bytes(path: &Path) -> Result<Vec<u8>, LspError> {
    use std::os::unix::ffi::OsStrExt;
    Ok(path.as_os_str().as_bytes().to_vec())
}

#[cfg(not(unix))]
fn path_bytes(path: &Path) -> Result<Vec<u8>, LspError> {
    path.to_str()
        .map(|value| value.as_bytes().to_vec())
        .ok_or(LspError::InvalidDocumentUri)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn utf16_extent_counts_astral_scalars_and_newlines_exactly() {
        let (end, length) = utf16_document_extent("a😀\nβ").expect("representable document");
        assert_eq!(end, LspPosition::new(1, 1));
        assert_eq!(length, 5);
    }
}
