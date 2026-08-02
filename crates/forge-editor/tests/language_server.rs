#![cfg(unix)]

use forge_bridge::lsp::{
    LspError, LspPosition, LspProtocolError, RustAnalyzerClient, RustAnalyzerConfig,
};
use forge_editor::buffers::{
    BufferId, BufferRegistry, DiskVersion, DocumentKey, EditorBuffer,
};
use forge_editor::language::{LanguageDocumentError, RustLanguageDocument};
use forge_protocol::identities::{ProcessId, ProjectId, RepositoryId};
use forge_protocol::paths::RepositoryRelativePath;
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

static FIXTURE_COUNTER: AtomicU64 = AtomicU64::new(1);

const SERVER: &str = r#"
import json
import sys

mode = sys.argv[1]

def read_message():
    length = None
    while True:
        line = sys.stdin.buffer.readline()
        if not line:
            return None
        if line in (b"\r\n", b"\n"):
            break
        name, value = line.decode("ascii").strip().split(":", 1)
        if name.lower() == "content-length":
            length = int(value.strip())
    if length is None:
        raise RuntimeError("missing content length")
    return json.loads(sys.stdin.buffer.read(length))

def send(message):
    body = json.dumps(message, separators=(",", ":")).encode("utf-8")
    sys.stdout.buffer.write((f"Content-Length: {len(body)}\r\n\r\n").encode("ascii"))
    sys.stdout.buffer.write(body)
    sys.stdout.buffer.flush()

def diagnostics(uri, version):
    send({
        "jsonrpc": "2.0",
        "id": 899,
        "method": "workspace/workspaceFolders",
        "params": None,
    })
    folders = read_message()
    result = None if folders is None else folders.get("result")
    if folders is None or folders.get("id") != 899 or not isinstance(result, list) or len(result) != 1:
        raise RuntimeError("client did not report its workspace folder")
    if not result[0].get("uri", "").startswith("file://") or result[0].get("name") != "ForgeOS project":
        raise RuntimeError("client reported an invalid workspace folder")
    send({
        "jsonrpc": "2.0",
        "id": 900,
        "method": "workspace/configuration",
        "params": {"items": [{"section": "rust-analyzer"}]},
    })
    response = read_message()
    if response is None or response.get("id") != 900 or response.get("result") != [None]:
        raise RuntimeError("client did not answer workspace/configuration")
    target_uri = uri + ".other" if mode == "wrong_uri" else uri
    payload = {
        "jsonrpc": "2.0",
        "method": "textDocument/publishDiagnostics",
        "params": {
            "uri": target_uri,
            "diagnostics": [{
                "range": {
                    "start": {"line": 0, "character": 0},
                    "end": {"line": 0, "character": 2},
                },
                "severity": 1,
                "code": "fixture",
                "source": "forge-fixture",
                "message": "fixture diagnostic",
            }],
        },
    }
    if mode != "missing_version":
        payload["params"]["version"] = version - 1 if mode == "stale_change" and version > 1 else version
    send(payload)

while True:
    message = read_message()
    if message is None:
        break
    method = message.get("method")
    if method == "initialize":
        send({
            "jsonrpc": "2.0",
            "id": message["id"],
            "result": {
                "capabilities": {
                    "textDocumentSync": 2,
                    "hoverProvider": False,
                    "definitionProvider": True,
                    "completionProvider": {"resolveProvider": False},
                }
            },
        })
    elif method == "initialized":
        pass
    elif method == "textDocument/didOpen":
        if mode == "malformed":
            sys.stdout.buffer.write(b"Content-Length: nope\r\n\r\n{}")
            sys.stdout.buffer.flush()
            break
        if mode == "exit":
            sys.exit(17)
        doc = message["params"]["textDocument"]
        diagnostics(doc["uri"], doc["version"])
    elif method == "textDocument/didChange":
        doc = message["params"]["textDocument"]
        change = message["params"]["contentChanges"][0]
        if "range" not in change or "rangeLength" not in change:
            raise RuntimeError("incremental change omitted its replacement range")
        diagnostics(doc["uri"], doc["version"])
    elif method == "textDocument/didClose":
        pass
    elif method == "shutdown":
        send({"jsonrpc": "2.0", "id": message["id"], "result": None})
    elif method == "exit":
        break
    elif "id" in message:
        send({
            "jsonrpc": "2.0",
            "id": message["id"],
            "error": {"code": -32601, "message": "fixture method not found"},
        })
"#;

struct Fixture {
    root: PathBuf,
    script: PathBuf,
}

impl Fixture {
    fn new() -> Self {
        let counter = FIXTURE_COUNTER.fetch_add(1, Ordering::Relaxed);
        let root = std::env::temp_dir().join(format!(
            "forgeos-lsp-{}-{counter}",
            std::process::id()
        ));
        fs::create_dir_all(root.join("src")).expect("create fixture root");
        let root = fs::canonicalize(root).expect("canonical fixture root");
        let script = root.join("server.py");
        fs::write(&script, SERVER).expect("write fixture server");
        Self { root, script }
    }

    fn config(&self, mode: &str, process_id: ProcessId) -> RustAnalyzerConfig {
        RustAnalyzerConfig::new(
            process_id,
            project_id(1),
            repository_id(2),
            "python3",
            [
                "-u".to_owned(),
                self.script.to_string_lossy().into_owned(),
                mode.to_owned(),
            ],
            self.root.clone(),
        )
        .expect("fixture config")
        .with_request_timeout(Duration::from_secs(2))
        .expect("fixture timeout")
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

fn process_id(byte: u8) -> ProcessId {
    ProcessId::from_bytes([byte; 16])
}

fn project_id(byte: u8) -> ProjectId {
    ProjectId::from_bytes([byte; 16])
}

fn repository_id(byte: u8) -> RepositoryId {
    RepositoryId::from_bytes([byte; 16])
}

fn buffer_id(byte: u8) -> BufferId {
    BufferId::from_bytes([byte; 16])
}

fn document(repository: u8, path: &str) -> DocumentKey {
    DocumentKey::new(
        repository_id(repository),
        RepositoryRelativePath::new(path).expect("canonical test path"),
    )
}

fn open_buffer<'a>(
    registry: &'a mut BufferRegistry,
    id: BufferId,
    document: DocumentKey,
    bytes: &[u8],
) -> &'a mut EditorBuffer {
    registry
        .open(id, document, bytes, DiskVersion::for_bytes(bytes))
        .expect("open fixture buffer")
}

#[test]
fn missing_server_is_a_typed_spawn_failure() {
    let fixture = Fixture::new();
    let config = RustAnalyzerConfig::new(
        process_id(1),
        project_id(1),
        repository_id(2),
        "/forgeos/fixture/definitely-not-rust-analyzer",
        [] as [&str; 0],
        fixture.root.clone(),
    )
    .expect("structurally valid missing server config");
    assert!(matches!(
        RustAnalyzerClient::start(config),
        Err(LspError::Spawn(_))
    ));
}

#[test]
fn starts_and_negotiates_reviewed_capabilities() {
    let fixture = Fixture::new();
    let client = RustAnalyzerClient::start(fixture.config("normal", process_id(1)))
        .expect("start fixture server");
    assert!(client.system_pid() > 0);
    assert!(client.capabilities().incremental_document_sync());
    assert!(!client.capabilities().full_document_sync());
    assert!(!client.capabilities().hover());
    assert!(client.capabilities().definition());
    assert!(client.capabilities().completion());
    client.close().expect("graceful close");
}

#[test]
fn diagnostics_are_bound_to_the_exact_editor_generation() {
    let fixture = Fixture::new();
    let mut registry = BufferRegistry::new();
    let buffer = open_buffer(
        &mut registry,
        buffer_id(1),
        document(2, "src/lib.rs"),
        b"fn broken() {}\n",
    );
    let (language, payload) =
        RustLanguageDocument::open(project_id(1), buffer).expect("language document");
    let mut client = RustAnalyzerClient::start(fixture.config("normal", process_id(1)))
        .expect("start fixture server");
    client.open_document(&payload).expect("open document");
    let diagnostics = client
        .wait_for_diagnostics(Duration::from_secs(2))
        .expect("diagnostics");
    language
        .validate_diagnostics(&diagnostics)
        .expect("current diagnostics");
    assert_eq!(diagnostics.diagnostics().len(), 1);
    assert_eq!(diagnostics.diagnostics()[0].message(), "fixture diagnostic");
    assert_eq!(diagnostics.diagnostics()[0].source(), Some("forge-fixture"));
    client.close().expect("graceful close");
}

#[test]
fn stale_diagnostics_are_rejected_after_an_edit() {
    let fixture = Fixture::new();
    let mut registry = BufferRegistry::new();
    let buffer = open_buffer(
        &mut registry,
        buffer_id(1),
        document(2, "src/lib.rs"),
        b"fn one() {}\n",
    );
    let (mut language, first) =
        RustLanguageDocument::open(project_id(1), buffer).expect("language document");
    let mut client = RustAnalyzerClient::start(fixture.config("stale_change", process_id(1)))
        .expect("start fixture server");
    client.open_document(&first).expect("open document");
    client
        .wait_for_diagnostics(Duration::from_secs(2))
        .expect("initial diagnostics");

    buffer
        .replace_range(3..6, b"two")
        .expect("edit fixture buffer");
    let pending = language
        .prepare_update(buffer)
        .expect("prepare language document");
    let second = pending.lsp_document().clone();
    client
        .change_document(&first, pending.lsp_document())
        .expect("send document change");
    language.commit_update(pending).expect("commit language update");
    assert!(matches!(
        client.wait_for_diagnostics(Duration::from_secs(2)),
        Err(LspError::StaleDiagnostics { current, received })
            if current == second.version() && received == first.version()
    ));
}

#[test]
fn prepared_update_does_not_advance_language_state_until_committed() {
    let mut registry = BufferRegistry::new();
    let buffer = open_buffer(
        &mut registry,
        buffer_id(1),
        document(2, "src/lib.rs"),
        b"fn one() {}\n",
    );
    let (mut language, _) =
        RustLanguageDocument::open(project_id(1), buffer).expect("language document");
    let original_content = language.content_version();
    let original_lsp = language.lsp_version();

    buffer
        .replace_range(3..6, b"two")
        .expect("edit fixture buffer");
    let pending = language
        .prepare_update(buffer)
        .expect("prepare language update");
    assert_eq!(language.content_version(), original_content);
    assert_eq!(language.lsp_version(), original_lsp);

    language.commit_update(pending).expect("commit language update");
    assert_eq!(language.content_version(), buffer.content_version());
    assert!(language.lsp_version() > original_lsp);
}

#[test]
fn project_and_repository_mismatches_never_cross_the_wire() {
    let fixture = Fixture::new();
    let mut registry = BufferRegistry::new();
    let buffer = open_buffer(
        &mut registry,
        buffer_id(1),
        document(3, "src/lib.rs"),
        b"fn one() {}\n",
    );
    let (_, payload) =
        RustLanguageDocument::open(project_id(9), buffer).expect("language document");
    let mut client = RustAnalyzerClient::start(fixture.config("normal", process_id(1)))
        .expect("start fixture server");
    assert!(matches!(
        client.open_document(&payload),
        Err(LspError::ProjectMismatch { .. })
    ));
}

#[test]
fn restart_requires_a_new_process_identity_and_clears_document_state() {
    let fixture = Fixture::new();
    let mut registry = BufferRegistry::new();
    let buffer = open_buffer(
        &mut registry,
        buffer_id(1),
        document(2, "src/lib.rs"),
        b"fn one() {}\n",
    );
    let (_, payload) =
        RustLanguageDocument::open(project_id(1), buffer).expect("language document");
    let mut client = RustAnalyzerClient::start(fixture.config("normal", process_id(1)))
        .expect("start fixture server");
    client.open_document(&payload).expect("open document");
    client
        .wait_for_diagnostics(Duration::from_secs(2))
        .expect("diagnostics");

    assert!(matches!(
        client.restart(process_id(1)),
        Err(LspError::ProcessIdentityReused(id)) if id == process_id(1)
    ));
    client.restart(process_id(2)).expect("restart server");
    assert_eq!(client.process_id(), process_id(2));
    assert_eq!(client.generation(), 2);
    assert_eq!(
        client
            .change_document(&payload, &payload)
            .unwrap_err()
            .to_string(),
        "LSP document is not open"
    );
    client.open_document(&payload).expect("reopen after restart");
    client
        .wait_for_diagnostics(Duration::from_secs(2))
        .expect("post-restart diagnostics");
    client.close().expect("graceful close");
}

#[test]
fn failed_restart_clears_document_state_before_retry() {
    let fixture = Fixture::new();
    let mut registry = BufferRegistry::new();
    let buffer = open_buffer(
        &mut registry,
        buffer_id(1),
        document(2, "src/lib.rs"),
        b"fn one() {}\n",
    );
    let (_, payload) =
        RustLanguageDocument::open(project_id(1), buffer).expect("language document");
    let mut client = RustAnalyzerClient::start(fixture.config("normal", process_id(1)))
        .expect("start fixture server");
    client.open_document(&payload).expect("open document");
    client
        .wait_for_diagnostics(Duration::from_secs(2))
        .expect("diagnostics");

    fs::remove_file(&fixture.script).expect("remove restart executable payload");
    assert!(client.restart(process_id(2)).is_err());
    assert!(matches!(
        client.change_document(&payload, &payload),
        Err(LspError::DocumentNotOpen)
    ));
}

#[test]
fn unsupported_capabilities_are_reported_without_guessing() {
    let fixture = Fixture::new();
    let mut registry = BufferRegistry::new();
    let buffer = open_buffer(
        &mut registry,
        buffer_id(1),
        document(2, "src/lib.rs"),
        b"fn one() {}\n",
    );
    let (_, payload) =
        RustLanguageDocument::open(project_id(1), buffer).expect("language document");
    let mut client = RustAnalyzerClient::start(fixture.config("normal", process_id(1)))
        .expect("start fixture server");
    client.open_document(&payload).expect("open document");
    assert!(matches!(
        client.request_hover(&payload, LspPosition::new(0, 1)),
        Err(LspError::UnsupportedCapability("textDocument/hover"))
    ));
}

#[test]
fn malformed_protocol_is_explicit_and_does_not_mutate_the_buffer() {
    let fixture = Fixture::new();
    let mut registry = BufferRegistry::new();
    let buffer = open_buffer(
        &mut registry,
        buffer_id(1),
        document(2, "src/lib.rs"),
        b"fn one() {}\n",
    );
    let original = buffer.bytes().to_vec();
    let (_, payload) =
        RustLanguageDocument::open(project_id(1), buffer).expect("language document");
    let mut client = RustAnalyzerClient::start(fixture.config("malformed", process_id(1)))
        .expect("start fixture server");
    client.open_document(&payload).expect("open document");
    assert!(matches!(
        client.wait_for_diagnostics(Duration::from_secs(2)),
        Err(LspError::Protocol(LspProtocolError::InvalidContentLength))
    ));
    assert_eq!(buffer.bytes(), original);
}

#[test]
fn missing_and_unknown_diagnostic_versions_are_rejected() {
    for mode in ["missing_version", "wrong_uri"] {
        let fixture = Fixture::new();
        let mut registry = BufferRegistry::new();
        let buffer = open_buffer(
            &mut registry,
            buffer_id(1),
            document(2, "src/lib.rs"),
            b"fn one() {}\n",
        );
        let (_, payload) =
            RustLanguageDocument::open(project_id(1), buffer).expect("language document");
        let mut client = RustAnalyzerClient::start(fixture.config(mode, process_id(1)))
            .expect("start fixture server");
        client.open_document(&payload).expect("open document");
        let error = client
            .wait_for_diagnostics(Duration::from_secs(2))
            .expect_err("invalid diagnostics must fail");
        assert!(matches!(
            (mode, error),
            ("missing_version", LspError::MissingDiagnosticVersion)
                | ("wrong_uri", LspError::UntrackedDiagnosticDocument(_))
        ));
    }
}

#[test]
fn non_utf8_rust_remains_editable_when_language_state_is_unavailable() {
    let mut registry = BufferRegistry::new();
    let buffer = open_buffer(
        &mut registry,
        buffer_id(1),
        document(2, "src/lib.rs"),
        &[0xff, b'a'],
    );
    assert_eq!(
        RustLanguageDocument::open(project_id(1), buffer).unwrap_err(),
        LanguageDocumentError::InvalidUtf8 { valid_up_to: 0 }
    );
    buffer
        .replace_range(1..2, b"b")
        .expect("plain-text editing remains available");
    assert_eq!(buffer.bytes(), &[0xff, b'b']);
}

#[test]
fn language_state_cannot_cross_buffer_identity() {
    let mut first_registry = BufferRegistry::new();
    let first = open_buffer(
        &mut first_registry,
        buffer_id(1),
        document(2, "src/lib.rs"),
        b"fn one() {}\n",
    );
    let (language, _) =
        RustLanguageDocument::open(project_id(1), first).expect("language document");

    let mut second_registry = BufferRegistry::new();
    let second = open_buffer(
        &mut second_registry,
        buffer_id(2),
        document(2, "src/lib.rs"),
        b"fn two() {}\n",
    );
    assert!(matches!(
        language.prepare_update(second),
        Err(LanguageDocumentError::BufferMismatch { expected, actual })
            if expected == buffer_id(1) && actual == buffer_id(2)
    ));
}

#[test]
fn unexpected_server_exit_is_not_reported_as_timeout() {
    let fixture = Fixture::new();
    let mut registry = BufferRegistry::new();
    let buffer = open_buffer(
        &mut registry,
        buffer_id(1),
        document(2, "src/lib.rs"),
        b"fn one() {}\n",
    );
    let (_, payload) =
        RustLanguageDocument::open(project_id(1), buffer).expect("language document");
    let mut client = RustAnalyzerClient::start(fixture.config("exit", process_id(1)))
        .expect("start fixture server");
    client.open_document(&payload).expect("open document");
    assert!(matches!(
        client.wait_for_diagnostics(Duration::from_secs(2)),
        Err(LspError::ServerExited(_))
    ));
}
