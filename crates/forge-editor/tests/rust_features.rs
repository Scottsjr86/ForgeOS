#![cfg(unix)]

use forge_bridge::lsp::{LspError, LspPosition, RustAnalyzerClient, RustAnalyzerConfig};
use forge_editor::buffers::{BufferId, BufferRegistry, DiskVersion, DocumentKey, EditorBuffer};
use forge_editor::intelligence::{RustBufferIntelligence, RustIntelligenceStatus};
use forge_editor::language::LanguageDocumentError;
use forge_protocol::identities::{ProcessId, ProjectId, RepositoryId};
use forge_protocol::paths::RepositoryRelativePath;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

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

def publish(uri, version):
    send({
        "jsonrpc": "2.0",
        "method": "textDocument/publishDiagnostics",
        "params": {
            "uri": uri,
            "version": version,
            "diagnostics": [{
                "range": {
                    "start": {"line": 0, "character": 0},
                    "end": {"line": 0, "character": 2},
                },
                "severity": 1,
                "source": "forge-feature-fixture",
                "message": "deliberate fixture error",
            }],
        },
    })

open_uri = None
open_version = None
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
                    "definitionProvider": True,
                    "completionProvider": {"resolveProvider": False},
                    "workspaceSymbolProvider": True,
                }
            },
        })
    elif method == "initialized":
        pass
    elif method == "textDocument/didOpen":
        document = message["params"]["textDocument"]
        open_uri = document["uri"]
        open_version = document["version"]
        publish(open_uri, open_version)
    elif method == "textDocument/didChange":
        document = message["params"]["textDocument"]
        open_version = document["version"]
        publish(open_uri, open_version)
    elif method == "textDocument/definition":
        target = "file:///tmp/forgeos-outside.rs" if mode == "outside" else open_uri
        send({
            "jsonrpc": "2.0",
            "id": message["id"],
            "result": [{
                "targetUri": target,
                "targetRange": {
                    "start": {"line": 0, "character": 0},
                    "end": {"line": 0, "character": 12},
                },
                "targetSelectionRange": {
                    "start": {"line": 0, "character": 7},
                    "end": {"line": 0, "character": 11},
                },
            }],
        })
    elif method == "textDocument/completion":
        send({
            "jsonrpc": "2.0",
            "id": message["id"],
            "result": {
                "isIncomplete": False,
                "items": [
                    {"label": "zeta", "kind": 3, "sortText": "20"},
                    {"label": "alpha", "kind": 3, "sortText": "10", "insertText": "alpha()"},
                ],
            },
        })
    elif method == "workspace/symbol":
        target = "file:///tmp/forgeos-outside.rs" if mode == "outside_symbol" else open_uri
        send({
            "jsonrpc": "2.0",
            "id": message["id"],
            "result": [
                {
                    "name": "Zulu",
                    "kind": 12,
                    "containerName": "crate",
                    "location": {
                        "uri": target,
                        "range": {
                            "start": {"line": 2, "character": 0},
                            "end": {"line": 2, "character": 4},
                        },
                    },
                },
                {
                    "name": "Alpha",
                    "kind": 12,
                    "location": {
                        "uri": target,
                        "range": {
                            "start": {"line": 0, "character": 0},
                            "end": {"line": 0, "character": 5},
                        },
                    },
                },
            ],
        })
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
            "forgeos-rust-features-{}-{counter}",
            std::process::id()
        ));
        fs::create_dir_all(root.join("src")).expect("create fixture root");
        let root = fs::canonicalize(root).expect("canonical fixture root");
        let script = root.join("server.py");
        fs::write(&script, SERVER).expect("write fixture server");
        Self { root, script }
    }

    fn config(&self, mode: &str, process: ProcessId) -> RustAnalyzerConfig {
        RustAnalyzerConfig::new(
            process,
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

fn process_id(seed: u8) -> ProcessId {
    ProcessId::from_bytes([seed; 16])
}

fn project_id(seed: u8) -> ProjectId {
    ProjectId::from_bytes([seed; 16])
}

fn repository_id(seed: u8) -> RepositoryId {
    RepositoryId::from_bytes([seed; 16])
}

fn buffer_id(seed: u8) -> BufferId {
    BufferId::from_bytes([seed; 16])
}

fn document(path: &str) -> DocumentKey {
    DocumentKey::new(
        repository_id(2),
        RepositoryRelativePath::new(path).expect("canonical path"),
    )
}

fn open_buffer<'a>(
    registry: &'a mut BufferRegistry,
    id: BufferId,
    bytes: &[u8],
) -> &'a mut EditorBuffer {
    registry
        .open_existing(
            id,
            document("src/lib.rs"),
            DiskVersion::for_bytes(bytes),
            bytes.to_vec(),
        )
        .expect("open fixture buffer");
    registry.get_mut(id).expect("fixture buffer")
}

#[test]
fn definition_completion_and_symbols_are_typed_sorted_and_generation_bound() {
    let fixture = Fixture::new();
    let mut registry = BufferRegistry::new();
    let buffer = open_buffer(&mut registry, buffer_id(1), b"pub fn alpha() {}\n");
    let intelligence =
        RustBufferIntelligence::open(project_id(1), buffer).expect("intelligence state");
    let mut client =
        RustAnalyzerClient::start(fixture.config("normal", process_id(1))).expect("start fixture");
    assert!(client.capabilities().definition());
    assert!(client.capabilities().completion());
    assert!(client.capabilities().workspace_symbol());
    client
        .open_document(intelligence.lsp_document())
        .expect("open language document");

    let diagnostics = client
        .wait_for_diagnostics(Duration::from_secs(2))
        .expect("fixture diagnostics");
    intelligence
        .validate_diagnostics(&diagnostics)
        .expect("current diagnostics");

    let definition = client
        .request_definition(intelligence.lsp_document(), LspPosition::new(0, 8))
        .expect("definition result");
    intelligence
        .validate_definition(&definition)
        .expect("current definition");
    assert_eq!(definition.locations().len(), 1);
    assert_eq!(
        definition.locations()[0].relative_path().as_path(),
        Path::new("src/lib.rs")
    );
    assert_eq!(
        definition.locations()[0].range().start(),
        LspPosition::new(0, 7)
    );

    let completion = client
        .request_completion(intelligence.lsp_document(), LspPosition::new(0, 10))
        .expect("completion result");
    intelligence
        .validate_completion(&completion)
        .expect("current completion");
    assert_eq!(completion.items()[0].label(), "alpha");
    assert_eq!(completion.items()[0].insert_text(), Some("alpha()"));
    assert_eq!(completion.items()[1].label(), "zeta");

    let symbols = client
        .request_workspace_symbols("a")
        .expect("workspace symbols");
    assert_eq!(symbols.project_id(), project_id(1));
    assert_eq!(symbols.repository_id(), repository_id(2));
    assert_eq!(symbols.client_generation(), client.generation());
    assert_eq!(symbols.symbols()[0].name(), "Alpha");
    assert_eq!(symbols.symbols()[1].name(), "Zulu");
    client.close().expect("close fixture");
}

#[test]
fn navigation_results_outside_the_workspace_fail_closed() {
    let fixture = Fixture::new();
    let mut registry = BufferRegistry::new();
    let buffer = open_buffer(&mut registry, buffer_id(1), b"pub fn alpha() {}\n");
    let intelligence =
        RustBufferIntelligence::open(project_id(1), buffer).expect("intelligence state");
    let mut client =
        RustAnalyzerClient::start(fixture.config("outside", process_id(1))).expect("start fixture");
    client
        .open_document(intelligence.lsp_document())
        .expect("open language document");
    client
        .wait_for_diagnostics(Duration::from_secs(2))
        .expect("fixture diagnostics");
    assert!(matches!(
        client.request_definition(intelligence.lsp_document(), LspPosition::new(0, 8)),
        Err(LspError::ResultOutsideWorkspace(_))
    ));
    client.close().expect("close fixture");
}

#[test]
fn stale_feature_results_are_rejected_after_atomic_intelligence_update() {
    let fixture = Fixture::new();
    let mut registry = BufferRegistry::new();
    let buffer = open_buffer(&mut registry, buffer_id(1), b"pub fn alpha() {}\n");
    let mut intelligence =
        RustBufferIntelligence::open(project_id(1), buffer).expect("intelligence state");
    let mut client =
        RustAnalyzerClient::start(fixture.config("normal", process_id(1))).expect("start fixture");
    client
        .open_document(intelligence.lsp_document())
        .expect("open language document");
    client
        .wait_for_diagnostics(Duration::from_secs(2))
        .expect("fixture diagnostics");
    let stale_definition = client
        .request_definition(intelligence.lsp_document(), LspPosition::new(0, 8))
        .expect("definition result");
    let stale_completion = client
        .request_completion(intelligence.lsp_document(), LspPosition::new(0, 10))
        .expect("completion result");

    let previous = buffer.bytes().to_vec();
    buffer.replace_range(7..12, b"bravo").expect("edit buffer");
    let pending = intelligence
        .prepare_update(buffer, &previous)
        .expect("prepare exact update");
    client
        .change_document(intelligence.lsp_document(), pending.lsp_document())
        .expect("send exact update");
    intelligence
        .commit_update(pending)
        .expect("commit exact update");
    let diagnostics = client
        .wait_for_diagnostics(Duration::from_secs(2))
        .expect("updated diagnostics");
    intelligence
        .validate_diagnostics(&diagnostics)
        .expect("updated diagnostics are current");
    assert!(matches!(
        intelligence.validate_definition(&stale_definition),
        Err(forge_editor::intelligence::RustIntelligenceError::Language(
            LanguageDocumentError::StaleFeatureResult { .. }
        ))
    ));
    assert!(matches!(
        intelligence.validate_completion(&stale_completion),
        Err(forge_editor::intelligence::RustIntelligenceError::Language(
            LanguageDocumentError::StaleFeatureResult { .. }
        ))
    ));
    assert!(intelligence.syntax_snapshot(buffer).is_ok());
    client.close().expect("close fixture");
}

#[test]
fn server_failure_keeps_current_syntax_and_marks_language_degraded() {
    let fixture = Fixture::new();
    let mut registry = BufferRegistry::new();
    let buffer = open_buffer(&mut registry, buffer_id(1), b"pub fn alpha() {}\n");
    let mut intelligence =
        RustBufferIntelligence::open(project_id(1), buffer).expect("intelligence state");
    let previous = buffer.bytes().to_vec();
    buffer
        .replace_range(7..12, b"bravo")
        .expect("edit buffer while server is unavailable");
    let pending = intelligence
        .prepare_update(buffer, &previous)
        .expect("prepare parser and language update");

    let missing = RustAnalyzerConfig::new(
        process_id(9),
        project_id(1),
        repository_id(2),
        "/forgeos/fixture/missing-rust-analyzer",
        [] as [&str; 0],
        fixture.root.clone(),
    )
    .expect("valid missing server config");
    assert!(matches!(
        RustAnalyzerClient::start(missing),
        Err(LspError::Spawn(_))
    ));

    intelligence
        .commit_syntax_only(pending)
        .expect("syntax remains usable without LSP");
    assert!(intelligence.syntax_snapshot(buffer).is_ok());
    assert!(matches!(
        intelligence.status(),
        RustIntelligenceStatus::LanguageDegraded {
            syntax_version,
            language_version,
        } if syntax_version == buffer.content_version() && language_version.get() == 1
    ));
    let resync = intelligence
        .prepare_language_resync(buffer)
        .expect("later resync can target current buffer");
    assert_eq!(
        resync.lsp_document().version().get() as u64,
        buffer.content_version().get()
    );
}

#[test]
fn workspace_symbol_results_outside_the_workspace_fail_closed() {
    let fixture = Fixture::new();
    let mut registry = BufferRegistry::new();
    let buffer = open_buffer(&mut registry, buffer_id(1), b"pub fn alpha() {}\n");
    let intelligence =
        RustBufferIntelligence::open(project_id(1), buffer).expect("intelligence state");
    let mut client = RustAnalyzerClient::start(fixture.config("outside_symbol", process_id(1)))
        .expect("start fixture");
    client
        .open_document(intelligence.lsp_document())
        .expect("open language document");
    client
        .wait_for_diagnostics(Duration::from_secs(2))
        .expect("fixture diagnostics");
    assert!(matches!(
        client.request_workspace_symbols("alpha"),
        Err(LspError::ResultOutsideWorkspace(_))
    ));
    client.close().expect("close fixture");
}

#[test]
#[ignore = "requires operator rust-analyzer and a real Rust workspace"]
fn real_rust_analyzer_proves_diagnostics_definition_completion_and_symbols() {
    let counter = FIXTURE_COUNTER.fetch_add(1, Ordering::Relaxed);
    let root = std::env::temp_dir().join(format!(
        "forgeos-real-rust-analyzer-{}-{counter}",
        std::process::id()
    ));
    fs::create_dir_all(root.join("src")).expect("create real workspace");
    fs::write(
        root.join("Cargo.toml"),
        "[package]\nname = \"forgeos_ra_witness\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    )
    .expect("write Cargo.toml");
    let source = b"pub fn helper() -> u32 { 42 }\npub fn broken() -> u32 {\n    let value = helper();\n    let _ = std::mem::\n    value + \"bad\"\n}\n";
    fs::write(root.join("src/lib.rs"), source).expect("write Rust source");
    let root = fs::canonicalize(&root).expect("canonical real workspace");
    let executable =
        std::env::var("FORGE_RUST_ANALYZER").unwrap_or_else(|_| "rust-analyzer".into());
    let config = RustAnalyzerConfig::new(
        process_id(7),
        project_id(1),
        repository_id(2),
        executable,
        [] as [&str; 0],
        root.clone(),
    )
    .expect("real rust-analyzer config")
    .with_request_timeout(Duration::from_secs(30))
    .expect("real request timeout");
    let mut registry = BufferRegistry::new();
    let buffer = open_buffer(&mut registry, buffer_id(1), source);
    let intelligence =
        RustBufferIntelligence::open(project_id(1), buffer).expect("real intelligence state");
    let mut client = RustAnalyzerClient::start(config).expect("start real rust-analyzer");
    client
        .open_document(intelligence.lsp_document())
        .expect("open real document");
    let diagnostics_deadline = Instant::now() + Duration::from_secs(30);
    let diagnostics = loop {
        let remaining = diagnostics_deadline
            .checked_duration_since(Instant::now())
            .expect("real rust-analyzer did not publish non-empty diagnostics in time");
        let diagnostics = client
            .wait_for_diagnostics(remaining)
            .expect("real diagnostics");
        if !diagnostics.diagnostics().is_empty() {
            break diagnostics;
        }
    };
    intelligence
        .validate_diagnostics(&diagnostics)
        .expect("real diagnostics are current");

    let definition = client
        .request_definition(intelligence.lsp_document(), LspPosition::new(2, 18))
        .expect("real definition");
    intelligence
        .validate_definition(&definition)
        .expect("real definition is current");
    assert!(!definition.locations().is_empty());

    let completion = client
        .request_completion(intelligence.lsp_document(), LspPosition::new(3, 22))
        .expect("real completion");
    intelligence
        .validate_completion(&completion)
        .expect("real completion is current");
    assert!(!completion.items().is_empty());

    let symbols = client
        .request_workspace_symbols("helper")
        .expect("real workspace symbols");
    assert!(symbols
        .symbols()
        .iter()
        .any(|symbol| symbol.name() == "helper"));
    client.close().expect("close real rust-analyzer");
    fs::remove_dir_all(root).expect("remove real workspace");
}
