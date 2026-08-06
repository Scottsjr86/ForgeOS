use forge_nyx_client::protocol::{NyxAvailability, NyxHealth, NyxProtocolVersion};
use forge_nyx_client::transport::{
    NyxClientConfig, NyxIncompatibility, NyxProbeOutcome, NyxProbeStatus, NyxTransportEndpoint,
    NyxUnavailableReason, probe_nyx,
};
use serde_json::{Value, json};
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

#[cfg(unix)]
use std::fs;
#[cfg(unix)]
use std::os::unix::net::UnixListener;
#[cfg(unix)]
use std::path::PathBuf;
#[cfg(unix)]
use std::sync::atomic::{AtomicU64, Ordering};

#[cfg(unix)]
static NEXT_FIXTURE: AtomicU64 = AtomicU64::new(1);

fn version(major: u16, minor: u16) -> NyxProtocolVersion {
    NyxProtocolVersion::new(major, minor)
}

fn tcp_config(address: SocketAddr, versions: &[NyxProtocolVersion]) -> NyxClientConfig {
    NyxClientConfig::new(NyxTransportEndpoint::tcp(address), versions.iter().copied())
        .unwrap()
        .with_io_timeout(Duration::from_secs(2))
}

fn version_body(contract: &str, server: &str, majors: &[u16]) -> Value {
    json!({
        "schema_version": "nyx.1.0",
        "schema_id": "nyx.api_version_manifest.v1",
        "server_version": server,
        "protocol_schema_version": "nyx.1.0",
        "public_contract_version": contract,
        "supported_contract_versions": [contract],
        "compatible_major_versions": majors,
        "deprecation_posture": "no_deprecated_public_contracts"
    })
}

fn health_body(status: &str, model_ready: bool) -> Value {
    json!({
        "schema_version": "nyx.1.0",
        "schema_id": "nyx.server_health.v1",
        "status": status,
        "live": true,
        "control_plane_ready": true,
        "model_requests_ready": model_ready,
        "server_version": "0.1.0",
        "protocol_schema_version": "nyx.1.0",
        "public_contract_version": "1.0",
        "boot_ts": "fixture",
        "engine_ready_count": if model_ready { 2 } else { 1 },
        "engine_total_count": 2,
        "provider_ready_count": if model_ready { 1 } else { 0 },
        "provider_total_count": 1
    })
}

fn capabilities_body(provider_ready: bool) -> Value {
    json!({
        "schema_version": "nyx.1.0",
        "schema_id": "nyx.api_capability_manifest.v1",
        "public_contract_version": "1.0",
        "server_version": "0.1.0",
        "protocol_schema_version": "nyx.1.0",
        "capabilities": [
            {
                "schema_version": "nyx.1.0",
                "schema_id": "nyx.api_capability_descriptor.v1",
                "capability_id": "nyx.openai.chat_completions",
                "supported_version": "1.0",
                "required_engine": "agent",
                "availability": "ready",
                "endpoint_ids": ["openai.chat.completions.post"]
            },
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
                "availability": if provider_ready { "ready" } else { "degraded" },
                "live": true,
                "ready": provider_ready,
                "reason": "fixture agent readiness",
                "source_schema_id": "nyx.engine_health.agent.v1"
            },
            {
                "engine": "api",
                "availability": "ready",
                "live": true,
                "ready": true,
                "reason": "fixture api readiness",
                "source_schema_id": "nyx.engine_health.api.v1"
            }
        ],
        "providers": [
            {
                "provider_id": "primary",
                "kind": "openai_compat",
                "configured": true,
                "availability": if provider_ready { "ready" } else { "degraded" },
                "ready": provider_ready,
                "probe_posture": "fixture",
                "reason": if provider_ready { "ready" } else { "not ready" }
            }
        ]
    })
}

fn error_body() -> Value {
    json!({
        "schema_version": "nyx.1.0",
        "schema_id": "nyx.api_error_envelope.v1",
        "error": {
            "code": "incompatible_api_major_version",
            "message": "unsupported",
            "retryable": false,
            "supported_contract_versions": ["1.0"],
            "request_id": "fixture-request",
            "trace_id": null,
            "diagnostic_id": null
        }
    })
}

fn json_response(status: u16, contract: Option<&str>, body: &Value) -> Vec<u8> {
    raw_response(
        status,
        contract,
        "application/json",
        body.to_string().as_bytes(),
    )
}

fn raw_response(status: u16, contract: Option<&str>, content_type: &str, body: &[u8]) -> Vec<u8> {
    let reason = match status {
        200 => "OK",
        426 => "Upgrade Required",
        500 => "Internal Server Error",
        _ => "Status",
    };
    let mut headers = format!(
        "HTTP/1.1 {status} {reason}\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nConnection: close\r\n",
        body.len()
    );
    if let Some(contract) = contract {
        headers.push_str(&format!("x-nyx-contract-version: {contract}\r\n"));
    }
    headers.push_str("\r\n");
    let mut response = headers.into_bytes();
    response.extend_from_slice(body);
    response
}

fn spawn_tcp_fixture(
    connection_count: usize,
    handler: impl Fn(&str, &str) -> Vec<u8> + Send + Sync + 'static,
) -> (SocketAddr, thread::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let handler = Arc::new(handler);
    let join = thread::spawn(move || {
        for _ in 0..connection_count {
            let (mut stream, _) = listener.accept().unwrap();
            let request = read_http_request(&mut stream);
            let (path, contract) = request_path_and_contract(&request);
            let response = handler(path, contract);
            stream.write_all(&response).unwrap();
            stream.flush().unwrap();
        }
    });
    (address, join)
}

fn read_http_request(stream: &mut impl Read) -> Vec<u8> {
    let mut bytes = Vec::new();
    let mut byte = [0_u8; 1];
    while !bytes.ends_with(b"\r\n\r\n") {
        stream.read_exact(&mut byte).unwrap();
        bytes.push(byte[0]);
        assert!(bytes.len() < 32 * 1024, "fixture request too large");
    }
    bytes
}

fn request_path_and_contract(request: &[u8]) -> (&str, &str) {
    let text = std::str::from_utf8(request).unwrap();
    let mut lines = text.split("\r\n");
    let request_line = lines.next().unwrap();
    let path = request_line.split_whitespace().nth(1).unwrap();
    let contract = lines
        .find_map(|line| {
            let (name, value) = line.split_once(':')?;
            name.eq_ignore_ascii_case("x-nyx-contract-version")
                .then_some(value.trim())
        })
        .unwrap();
    (path, contract)
}

fn public_api_handler(path: &str, requested: &str, provider_ready: bool) -> Vec<u8> {
    assert_eq!(requested, "1.1");
    match path {
        "/v1/nyx/version" => json_response(200, Some("1.0"), &version_body("1.0", "0.1.0", &[1])),
        "/v1/nyx/health" => json_response(
            200,
            Some("1.0"),
            &health_body(
                if provider_ready { "ready" } else { "degraded" },
                provider_ready,
            ),
        ),
        "/v1/nyx/capabilities" => {
            json_response(200, Some("1.0"), &capabilities_body(provider_ready))
        }
        other => panic!("unexpected fixture path {other}"),
    }
}

#[test]
fn compatible_healthy_nyx_uses_public_http_contract() {
    let (address, server) = spawn_tcp_fixture(3, |path, requested| {
        public_api_handler(path, requested, true)
    });
    let outcome = probe_nyx(&tcp_config(address, &[version(1, 0), version(1, 1)]));
    server.join().unwrap();

    assert_eq!(outcome.status(), NyxProbeStatus::Ready);
    let response = outcome.response().unwrap();
    assert_eq!(response.selected_protocol(), version(1, 0));
    assert_eq!(response.server_version(), "0.1.0");
    assert_eq!(response.protocol_schema_version(), "nyx.1.0");
    assert_eq!(response.health(), NyxHealth::Healthy);
    assert!(response.live());
    assert!(response.control_plane_ready());
    assert!(response.model_requests_ready());
    assert_eq!(response.engine_ready_count(), 2);
    assert_eq!(response.engine_total_count(), 2);
    assert_eq!(response.provider_ready_count(), 1);
    assert_eq!(response.provider_total_count(), 1);
    assert_eq!(
        response
            .capabilities()
            .iter()
            .map(|item| item.capability_id())
            .collect::<Vec<_>>(),
        vec!["nyx.native.health", "nyx.openai.chat_completions"]
    );
    assert_eq!(
        response.capabilities()[0].availability(),
        NyxAvailability::Ready
    );
    assert_eq!(response.engines()[0].engine(), "agent");
    assert!(response.providers()[0].ready());
}

#[test]
fn missing_configured_endpoint_is_unavailable() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    drop(listener);

    let outcome = probe_nyx(&tcp_config(address, &[version(1, 0)]));
    assert!(matches!(
        outcome,
        NyxProbeOutcome::Unavailable {
            reason: NyxUnavailableReason::ConnectFailed(_)
        }
    ));
}

#[test]
fn incompatible_major_is_rejected_from_nyx_error_contract() {
    let (address, server) = spawn_tcp_fixture(1, |_path, requested| {
        assert_eq!(requested, "2.0");
        json_response(426, Some("1.0"), &error_body())
    });
    let outcome = probe_nyx(&tcp_config(address, &[version(2, 0)]));
    server.join().unwrap();

    assert_eq!(outcome.status(), NyxProbeStatus::Incompatible);
    assert_eq!(
        outcome,
        NyxProbeOutcome::Incompatible {
            reason: NyxIncompatibility::RejectedContract {
                requested: version(2, 0),
                supported: vec![version(1, 0)]
            }
        }
    );
}

#[test]
fn malformed_success_response_is_incompatible_without_crashing() {
    let (address, server) = spawn_tcp_fixture(1, |_path, _requested| {
        raw_response(200, Some("1.0"), "application/json", b"not-json")
    });
    let outcome = probe_nyx(&tcp_config(address, &[version(1, 0)]));
    server.join().unwrap();

    assert!(matches!(
        outcome,
        NyxProbeOutcome::Incompatible {
            reason: NyxIncompatibility::MalformedResponse {
                path: "/v1/nyx/version",
                ..
            }
        }
    ));
}

#[test]
fn degraded_nyx_retains_capabilities_and_readiness_truth() {
    let (address, server) = spawn_tcp_fixture(3, |path, requested| {
        public_api_handler(path, requested, false)
    });
    let outcome = probe_nyx(&tcp_config(address, &[version(1, 1)]));
    server.join().unwrap();

    assert_eq!(outcome.status(), NyxProbeStatus::Unhealthy);
    let response = outcome.response().unwrap();
    assert_eq!(response.health(), NyxHealth::Degraded);
    assert!(!response.model_requests_ready());
    assert_eq!(response.capabilities().len(), 2);
    assert_eq!(
        response.engines()[0].availability(),
        NyxAvailability::Degraded
    );
    assert_eq!(
        response.providers()[0].availability(),
        NyxAvailability::Degraded
    );
    assert!(!response.providers()[0].ready());
}

#[test]
fn wrong_nyx_schema_is_incompatible() {
    let (address, server) = spawn_tcp_fixture(1, |_path, _requested| {
        let mut body = version_body("1.0", "0.1.0", &[1]);
        body["schema_id"] = json!("not.nyx.version.v1");
        json_response(200, Some("1.0"), &body)
    });
    let outcome = probe_nyx(&tcp_config(address, &[version(1, 0)]));
    server.join().unwrap();

    assert!(matches!(
        outcome,
        NyxProbeOutcome::Incompatible {
            reason: NyxIncompatibility::MalformedResponse { .. }
        }
    ));
}

#[test]
fn response_header_must_match_body_contract() {
    let (address, server) = spawn_tcp_fixture(1, |_path, _requested| {
        json_response(200, Some("1.1"), &version_body("1.0", "0.1.0", &[1]))
    });
    let outcome = probe_nyx(&tcp_config(address, &[version(1, 0)]));
    server.join().unwrap();

    assert_eq!(
        outcome,
        NyxProbeOutcome::Incompatible {
            reason: NyxIncompatibility::ContractHeaderMismatch {
                path: "/v1/nyx/version",
                header: version(1, 1),
                body: version(1, 0)
            }
        }
    );
}

#[cfg(unix)]
fn fixture_socket(label: &str) -> PathBuf {
    let number = NEXT_FIXTURE.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!(
        "forgeos-nyx-http-{}-{label}-{number}.sock",
        std::process::id()
    ))
}

#[cfg(unix)]
#[test]
fn unix_socket_transport_carries_the_same_public_http_contract() {
    let path = fixture_socket("compatible");
    let _ = fs::remove_file(&path);
    let listener = UnixListener::bind(&path).unwrap();
    let socket_path = path.clone();
    let server = thread::spawn(move || {
        for _ in 0..3 {
            let (mut stream, _) = listener.accept().unwrap();
            let request = read_http_request(&mut stream);
            let (path, contract) = request_path_and_contract(&request);
            let response = public_api_handler(path, contract, true);
            stream.write_all(&response).unwrap();
            stream.flush().unwrap();
        }
        drop(listener);
        let _ = fs::remove_file(socket_path);
    });

    let config = NyxClientConfig::new(
        NyxTransportEndpoint::unix_socket(&path),
        [version(1, 0), version(1, 1)],
    )
    .unwrap();
    let outcome = probe_nyx(&config);
    server.join().unwrap();
    assert_eq!(outcome.status(), NyxProbeStatus::Ready);
}

#[test]
#[ignore = "requires an independently running Nyx_Server public API"]
fn real_nyx_public_api_gate() {
    let address = std::env::var("FORGE_NYX_ADDR")
        .expect("set FORGE_NYX_ADDR to the running Nyx_Server address")
        .parse::<SocketAddr>()
        .expect("FORGE_NYX_ADDR must be ip:port");
    let outcome = probe_nyx(&tcp_config(address, &[version(1, 0)]));
    println!("FORGE_NYX_REAL_SERVER_OUTCOME={outcome:#?}");
    let response = outcome
        .response()
        .expect("real Nyx must return a compatible health and capability report");
    assert_eq!(response.selected_protocol(), version(1, 0));
    assert!(!response.server_version().is_empty());
    assert!(
        response
            .capabilities()
            .iter()
            .any(|capability| capability.capability_id() == "nyx.native.health")
    );
}
