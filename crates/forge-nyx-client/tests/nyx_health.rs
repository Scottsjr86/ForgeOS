#![cfg(unix)]

use forge_nyx_client::protocol::{
    NyxCapability, NyxHandshakeRequest, NyxHandshakeResponse, NyxHealth, NyxProtocolVersion,
};
use forge_nyx_client::transport::{
    probe_nyx, NyxClientConfig, NyxIncompatibility, NyxProbeOutcome, NyxProbeStatus,
    NyxTransportEndpoint, NyxUnavailableReason,
};
use std::fs;
use std::io::{Read, Write};
use std::os::unix::net::UnixListener;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::thread;
use std::time::Duration;

static NEXT_FIXTURE: AtomicU64 = AtomicU64::new(1);

fn version(major: u16, minor: u16) -> NyxProtocolVersion {
    NyxProtocolVersion::new(major, minor)
}

fn capability(value: &str) -> NyxCapability {
    NyxCapability::new(value).unwrap()
}

fn fixture_socket(label: &str) -> PathBuf {
    let number = NEXT_FIXTURE.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!(
        "forgeos-nyx-100-{}-{label}-{number}.sock",
        std::process::id()
    ))
}

fn config(path: &PathBuf) -> NyxClientConfig {
    NyxClientConfig::new(
        NyxTransportEndpoint::unix_socket(path),
        [version(1, 0), version(1, 1)],
    )
    .unwrap()
    .with_io_timeout(Duration::from_secs(2))
}

fn spawn_fixture(
    path: PathBuf,
    response: impl FnOnce(NyxHandshakeRequest) -> Vec<u8> + Send + 'static,
) -> thread::JoinHandle<()> {
    let _ = fs::remove_file(&path);
    let listener = UnixListener::bind(&path).unwrap();
    thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        let request = read_frame(&mut stream);
        let request = NyxHandshakeRequest::decode(&request).unwrap();
        write_frame(&mut stream, &response(request));
        drop(stream);
        drop(listener);
        let _ = fs::remove_file(path);
    })
}

fn read_frame(stream: &mut impl Read) -> Vec<u8> {
    let mut length = [0_u8; 4];
    stream.read_exact(&mut length).unwrap();
    let length = usize::try_from(u32::from_be_bytes(length)).unwrap();
    let mut bytes = vec![0_u8; length];
    stream.read_exact(&mut bytes).unwrap();
    bytes
}

fn write_frame(stream: &mut impl Write, bytes: &[u8]) {
    stream
        .write_all(&u32::try_from(bytes.len()).unwrap().to_be_bytes())
        .unwrap();
    stream.write_all(bytes).unwrap();
    stream.flush().unwrap();
}

#[test]
fn compatible_healthy_nyx_negotiates_protocol_and_reports_capabilities() {
    let path = fixture_socket("compatible");
    let server = spawn_fixture(path.clone(), |request| {
        assert!(request.supports(version(1, 1)));
        NyxHandshakeResponse::new(
            version(1, 1),
            "nyx-0.1.0",
            NyxHealth::Healthy,
            [capability("tools.read"), capability("chat")],
        )
        .unwrap()
        .encode()
    });

    let outcome = probe_nyx(&config(&path));
    server.join().unwrap();
    assert_eq!(outcome.status(), NyxProbeStatus::Ready);
    let response = outcome.response().unwrap();
    assert_eq!(response.selected_protocol(), version(1, 1));
    assert_eq!(response.service_version(), "nyx-0.1.0");
    assert_eq!(response.health(), NyxHealth::Healthy);
    assert_eq!(
        response
            .capabilities()
            .iter()
            .map(NyxCapability::as_str)
            .collect::<Vec<_>>(),
        vec!["chat", "tools.read"]
    );
}

#[test]
fn missing_configured_endpoint_is_unavailable_not_compatible() {
    let path = fixture_socket("missing");
    let _ = fs::remove_file(&path);
    let outcome = probe_nyx(&config(&path));
    assert!(matches!(
        outcome,
        NyxProbeOutcome::Unavailable {
            reason: NyxUnavailableReason::ConnectFailed(_)
        }
    ));
}

#[test]
fn responding_endpoint_with_unoffered_protocol_is_incompatible() {
    let path = fixture_socket("incompatible");
    let server = spawn_fixture(path.clone(), |_request| {
        NyxHandshakeResponse::new(
            version(2, 0),
            "nyx-2.0.0",
            NyxHealth::Healthy,
            [capability("chat")],
        )
        .unwrap()
        .encode()
    });

    let outcome = probe_nyx(&config(&path));
    server.join().unwrap();
    assert_eq!(outcome.status(), NyxProbeStatus::Incompatible);
    assert_eq!(
        outcome,
        NyxProbeOutcome::Incompatible {
            reason: NyxIncompatibility::UnsupportedSelectedProtocol {
                selected: version(2, 0)
            }
        }
    );
}

#[test]
fn malformed_successful_exchange_is_incompatible_without_crashing() {
    let path = fixture_socket("malformed");
    let server = spawn_fixture(path.clone(), |_request| b"not-a-nyx-response".to_vec());

    let outcome = probe_nyx(&config(&path));
    server.join().unwrap();
    assert_eq!(outcome.status(), NyxProbeStatus::Incompatible);
    assert!(matches!(
        outcome,
        NyxProbeOutcome::Incompatible {
            reason: NyxIncompatibility::MalformedResponse(_)
        }
    ));
}

#[test]
fn unhealthy_nyx_retains_declared_health_and_capabilities() {
    let path = fixture_socket("unhealthy");
    let server = spawn_fixture(path.clone(), |_request| {
        NyxHandshakeResponse::new(
            version(1, 0),
            "nyx-0.1.0",
            NyxHealth::Unhealthy,
            [capability("chat"), capability("models.list")],
        )
        .unwrap()
        .encode()
    });

    let outcome = probe_nyx(&config(&path));
    server.join().unwrap();
    assert_eq!(outcome.status(), NyxProbeStatus::Unhealthy);
    let response = outcome.response().unwrap();
    assert_eq!(response.health(), NyxHealth::Unhealthy);
    assert_eq!(response.selected_protocol(), version(1, 0));
    assert_eq!(response.capabilities().len(), 2);
}
