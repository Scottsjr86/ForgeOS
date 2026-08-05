use forge_app::composition::nyx_service::{
    ManagedNyxService, ManagedNyxServiceError, NyxServiceConfig,
};
use forge_bridge::processes::ProcessExecutionContext;
use forge_nyx_client::protocol::NyxProtocolVersion;
use forge_nyx_client::transport::{NyxClientConfig, NyxProbeOutcome, NyxTransportEndpoint};
use forge_protocol::identities::{ProcessId, SessionId};
use forge_session::service_runtime::{ManagedServiceReadiness, ManagedServiceRuntimeState};
use forge_session::services::StartupRestartPolicy;
use std::fs;
use std::net::{SocketAddr, TcpListener};
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::thread;
use std::time::{Duration, Instant};

static NEXT_FIXTURE: AtomicU64 = AtomicU64::new(1);

fn process(byte: u8) -> ProcessId {
    ProcessId::from_bytes([byte; 16])
}

fn session(byte: u8) -> SessionId {
    SessionId::from_bytes([byte; 16])
}

struct Fixture {
    root: PathBuf,
    script: PathBuf,
    marker: PathBuf,
    address: SocketAddr,
}

impl Fixture {
    fn new(exit_after_contract: bool) -> Self {
        let unique = NEXT_FIXTURE.fetch_add(1, Ordering::Relaxed);
        let root = std::env::temp_dir().join(format!(
            "forgeos-nyx-service-{}-{unique}",
            std::process::id()
        ));
        fs::create_dir_all(&root).unwrap();
        let script = root.join("nyx_fixture.py");
        let marker = root.join("starts.txt");
        fs::write(&script, python_fixture()).unwrap();
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        drop(listener);
        fs::write(
            root.join("mode.txt"),
            if exit_after_contract { "exit" } else { "stay" },
        )
        .unwrap();
        Self {
            root,
            script,
            marker,
            address,
        }
    }

    fn manager(&self, restarts: u16) -> ManagedNyxService {
        let client = NyxClientConfig::new(
            NyxTransportEndpoint::tcp(self.address),
            [NyxProtocolVersion::new(1, 0)],
        )
        .unwrap()
        .with_io_timeout(Duration::from_millis(100));
        let context = ProcessExecutionContext::new(&self.root).with_environment([(
            "PATH",
            std::env::var("PATH").unwrap_or_else(|_| "/usr/bin:/bin".to_owned()),
        )]);
        let config = NyxServiceConfig::new(
            "python3",
            [
                self.script.to_string_lossy().into_owned(),
                self.address.port().to_string(),
                self.marker.to_string_lossy().into_owned(),
                self.root.join("mode.txt").to_string_lossy().into_owned(),
            ],
            context,
            client,
            StartupRestartPolicy::OnFailure {
                max_restarts: restarts,
            },
        )
        .unwrap()
        .with_readiness_policy(100, Duration::from_millis(10))
        .unwrap();
        ManagedNyxService::new(session(7), config)
    }

    fn starts(&self) -> usize {
        fs::read_to_string(&self.marker)
            .unwrap_or_default()
            .lines()
            .count()
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

#[test]
fn degraded_nyx_control_plane_is_managed_as_truthful_degraded_readiness() {
    let fixture = Fixture::new(false);
    let mut service = fixture.manager(1);
    let started = service.start(process(1)).unwrap();
    assert_eq!(started.session_id(), session(7));
    assert!(matches!(
        started.outcome(),
        NyxProbeOutcome::Unhealthy { .. }
    ));
    assert_eq!(
        service.state(),
        &ManagedServiceRuntimeState::Ready {
            attempt: 1,
            process_id: process(1),
            readiness: ManagedServiceReadiness::Degraded,
        }
    );
    assert!(matches!(
        service.probe().unwrap(),
        NyxProbeOutcome::Unhealthy { .. }
    ));
    service.stop().unwrap();
    assert_eq!(service.state(), &ManagedServiceRuntimeState::Stopped);
    assert_eq!(fixture.starts(), 1);
}

#[test]
fn an_existing_nyx_endpoint_prevents_duplicate_server_spawn() {
    let fixture = Fixture::new(false);
    let mut owner = fixture.manager(1);
    owner.start(process(1)).unwrap();

    let mut duplicate = fixture.manager(1);
    assert!(matches!(
        duplicate.start(process(2)),
        Err(ManagedNyxServiceError::EndpointAlreadyServing(_))
    ));
    assert_eq!(fixture.starts(), 1);
    owner.stop().unwrap();
}

#[test]
fn crashes_consume_the_bounded_restart_budget_with_new_process_identity() {
    let fixture = Fixture::new(true);
    let mut service = fixture.manager(1);
    service.start(process(1)).unwrap();
    let first_exit = wait_for_exit(&mut service);
    assert_eq!(first_exit.code(), Some(17));
    assert!(matches!(
        service.state(),
        ManagedServiceRuntimeState::RestartPending {
            next_attempt: 2,
            ..
        }
    ));

    service.start(process(2)).unwrap();
    let second_exit = wait_for_exit(&mut service);
    assert_eq!(second_exit.code(), Some(17));
    assert!(matches!(
        service.state(),
        ManagedServiceRuntimeState::Failed { .. }
    ));
    assert_eq!(fixture.starts(), 2);
}

#[test]
fn stopping_one_managed_nyx_does_not_modify_unrelated_local_state() {
    let fixture = Fixture::new(false);
    let sentinel = fixture.root.join("local-editor-state.bin");
    fs::write(&sentinel, b"dirty-local-buffer").unwrap();
    let mut service = fixture.manager(0);
    service.start(process(1)).unwrap();
    service.stop().unwrap();
    assert_eq!(fs::read(&sentinel).unwrap(), b"dirty-local-buffer");
}

fn wait_for_exit(service: &mut ManagedNyxService) -> forge_protocol::processes::ProcessExit {
    let deadline = Instant::now() + Duration::from_secs(3);
    loop {
        if let Some(exit) = service.poll_native_exit().unwrap() {
            return exit;
        }
        assert!(Instant::now() < deadline, "fixture Nyx did not exit");
        thread::sleep(Duration::from_millis(10));
    }
}

fn python_fixture() -> &'static str {
    r#"from http.server import BaseHTTPRequestHandler, HTTPServer
import json
import os
import sys
import threading

port = int(sys.argv[1])
marker = sys.argv[2]
mode_path = sys.argv[3]
with open(marker, "a", encoding="utf-8") as handle:
    handle.write("start\n")
with open(mode_path, "r", encoding="utf-8") as handle:
    exit_after_contract = handle.read().strip() == "exit"

VERSION = {
    "schema_version": "nyx.1.0",
    "schema_id": "nyx.api_version_manifest.v1",
    "server_version": "0.1.0",
    "protocol_schema_version": "nyx.1.0",
    "public_contract_version": "1.0",
    "supported_contract_versions": ["1.0"],
    "compatible_major_versions": [1],
    "deprecation_posture": "no_deprecated_public_contracts"
}
HEALTH = {
    "schema_version": "nyx.1.0",
    "schema_id": "nyx.server_health.v1",
    "status": "degraded",
    "live": True,
    "control_plane_ready": True,
    "model_requests_ready": False,
    "server_version": "0.1.0",
    "protocol_schema_version": "nyx.1.0",
    "public_contract_version": "1.0",
    "boot_ts": "fixture",
    "engine_ready_count": 1,
    "engine_total_count": 2,
    "provider_ready_count": 0,
    "provider_total_count": 1
}
CAPABILITIES = {
    "schema_version": "nyx.1.0",
    "schema_id": "nyx.api_capability_manifest.v1",
    "public_contract_version": "1.0",
    "server_version": "0.1.0",
    "protocol_schema_version": "nyx.1.0",
    "capabilities": [
        {
            "schema_version": "nyx.1.0",
            "schema_id": "nyx.api_capability_descriptor.v1",
            "capability_id": "nyx.native.health",
            "supported_version": "1.0",
            "required_engine": "api",
            "availability": "ready",
            "endpoint_ids": ["nyx.health.get", "nyx.readiness.get"]
        }
    ],
    "engines": [
        {
            "engine": "agent",
            "availability": "degraded",
            "live": True,
            "ready": False,
            "reason": "fixture agent not ready",
            "source_schema_id": "nyx.engine_health.agent.v1"
        },
        {
            "engine": "api",
            "availability": "ready",
            "live": True,
            "ready": True,
            "reason": "fixture api ready",
            "source_schema_id": "nyx.engine_health.api.v1"
        }
    ],
    "providers": [
        {
            "provider_id": "primary",
            "kind": "openai_compat",
            "configured": True,
            "availability": "degraded",
            "ready": False,
            "probe_posture": "fixture",
            "reason": "not ready"
        }
    ]
}

class ReusableHTTPServer(HTTPServer):
    allow_reuse_address = True

class Handler(BaseHTTPRequestHandler):
    request_count = 0

    def do_GET(self):
        if self.path == "/v1/nyx/version":
            body = VERSION
        elif self.path == "/v1/nyx/health":
            body = HEALTH
        elif self.path == "/v1/nyx/capabilities":
            body = CAPABILITIES
        else:
            self.send_response(404)
            self.end_headers()
            return
        encoded = json.dumps(body, separators=(",", ":")).encode("utf-8")
        self.send_response(200)
        self.send_header("Content-Type", "application/json")
        self.send_header("Content-Length", str(len(encoded)))
        self.send_header("x-nyx-contract-version", "1.0")
        self.send_header("Connection", "close")
        self.end_headers()
        self.wfile.write(encoded)
        self.wfile.flush()
        Handler.request_count += 1
        if exit_after_contract and Handler.request_count >= 3:
            threading.Timer(0.05, lambda: os._exit(17)).start()

    def log_message(self, format, *args):
        pass

ReusableHTTPServer(("127.0.0.1", port), Handler).serve_forever()
"#
}
