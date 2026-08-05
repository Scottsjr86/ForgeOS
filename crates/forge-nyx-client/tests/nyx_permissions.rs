use forge_nyx_client::permission::{
    NyxPermissionAuditEventKind, NyxPermissionCheckpointCreate, NyxPermissionCheckpointStatus,
    NyxPermissionDecisionKind, NyxPermissionResume, NyxPermissionScope, NyxScopedToolRequest,
};
use forge_nyx_client::permission_client::{NyxPermissionClient, NyxPermissionClientError};
use forge_nyx_client::protocol::NyxProtocolVersion;
use forge_nyx_client::transport::{NyxClientConfig, NyxTransportEndpoint};
use serde_json::{json, Value};
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

const WORKSPACE: &str = "/tmp/forge-nyx-permission-fixture";
const REQUEST_SHA256: &str = "b3edb536116a83f86c563db5e3199a7b771604e31d18dbbb8743e9097497355e";
const PAYLOAD_SHA256: &str = "283a46a1ccbbc80b6fb04798c9077ad92feb64430c1c9810d6568526090ad2a3";
const SCOPE_SHA256: &str = "ea4466e6ce04895e7907f351ba5ed5f8dbc21b3af18228a0e9087e5c1dae2578";
const POLICY_SHA256: &str = "8607c949db93f056e60c26afb0dcfb4e02b8f16842a2e670d6c42c3971d14f9e";
const EFFECTS_SHA256: &str = "6c81b70d0f67781422b4d16e5981a9c4e99a15ee884a50c15216fc09139c2292";
const CONDITIONS_SHA256: &str = "ca83278531aaac9babc7d9bd2b98c43dac414d7fdfa93c82b7c9ec2bd889a686";
const RESUME_TOKEN: &str = "nyxrt_fixture_exact_token_0123456789abcdef";
const RESUME_TOKEN_SHA256: &str =
    "4988fa6d68d1f23d7a87553986ba287749665732734e306c48d9e188c53b7ac8";

fn version() -> NyxProtocolVersion {
    NyxProtocolVersion::new(1, 0)
}

fn client(address: SocketAddr) -> NyxPermissionClient {
    let config = NyxClientConfig::new(NyxTransportEndpoint::tcp(address), [version()])
        .unwrap()
        .with_io_timeout(Duration::from_secs(2));
    NyxPermissionClient::new(config)
}

fn fixed_scope(workspace: &str) -> NyxPermissionScope {
    NyxPermissionScope::new(workspace)
        .unwrap()
        .with_allowed_paths(["approved.txt"])
        .unwrap()
        .with_declared_side_effects(["writes_files"])
        .unwrap()
}

fn fixed_request(workspace: &str) -> NyxScopedToolRequest {
    NyxScopedToolRequest::new(
        "forge-permission-request-1",
        "sess_000001",
        "forge-tool-call-1",
        "repo.write_file",
        json!({"path": "approved.txt", "content": "approved-once\n"}),
        fixed_scope(workspace),
        "forge-idem-1",
    )
    .unwrap()
}

fn policy_decision() -> Value {
    json!({
        "schema_version": "nyx.1.0",
        "schema_id": "nyx.policy_decision.v1",
        "kind": "checkpoint",
        "policy_snapshot_id": "nyx.policy_snapshot.phase0.v1"
    })
}

fn pending_checkpoint(request: &NyxScopedToolRequest) -> Value {
    json!({
        "schema_version": "nyx.1.0",
        "schema_id": "nyx.permission_checkpoint.v1",
        "checkpoint_id": "permckpt_000001_b3edb536116a",
        "request": request,
        "accepted_operation_id": format!("nyxop_{REQUEST_SHA256}"),
        "request_sha256": REQUEST_SHA256,
        "payload_sha256": PAYLOAD_SHA256,
        "scope_sha256": SCOPE_SHA256,
        "policy_decision": policy_decision(),
        "policy_decision_sha256": POLICY_SHA256,
        "predicted_effects": ["writes_files"],
        "predicted_effects_sha256": EFFECTS_SHA256,
        "created_at_unix_ms": 100,
        "expires_at_unix_ms": 30_100,
        "status": "pending",
        "audit_sequence": 1
    })
}

fn approved_checkpoint(request: &NyxScopedToolRequest) -> Value {
    let mut checkpoint = pending_checkpoint(request);
    checkpoint["approval_conditions"] = json!(["exact_scope_only", "single_execution"]);
    checkpoint["approval_conditions_sha256"] = json!(CONDITIONS_SHA256);
    checkpoint["status"] = json!("approved");
    checkpoint["decided_at_unix_ms"] = json!(200);
    checkpoint["decided_by"] = json!("local_anonymous");
    checkpoint["decision_reason"] = json!("reviewed exact payload and scope");
    checkpoint["resume_token_sha256"] = json!(RESUME_TOKEN_SHA256);
    checkpoint["audit_sequence"] = json!(2);
    checkpoint
}

fn denied_checkpoint(request: &NyxScopedToolRequest) -> Value {
    let mut checkpoint = pending_checkpoint(request);
    checkpoint["approval_conditions"] = json!(["exact_scope_only", "single_execution"]);
    checkpoint["approval_conditions_sha256"] = json!(CONDITIONS_SHA256);
    checkpoint["status"] = json!("denied");
    checkpoint["decided_at_unix_ms"] = json!(200);
    checkpoint["decided_by"] = json!("local_anonymous");
    checkpoint["decision_reason"] = json!("operator denied exact request");
    checkpoint["audit_sequence"] = json!(2);
    checkpoint
}

fn resolution(checkpoint: Value, token: Option<&str>) -> Value {
    json!({
        "schema_version": "nyx.1.0",
        "schema_id": "nyx.permission_resolve.v1",
        "checkpoint": checkpoint,
        "resume_token": token
    })
}

fn resume_result() -> Value {
    json!({
        "schema_version": "nyx.1.0",
        "schema_id": "nyx.permission_resume_result.v1",
        "checkpoint_id": "permckpt_000001_b3edb536116a",
        "execution_id": "permexec_000003_b3edb536116a",
        "request_sha256": REQUEST_SHA256,
        "status": "consumed",
        "tool_result": {
            "schema_version": "nyx.1.0",
            "schema_id": "nyx.tool_result.v1",
            "ok": true,
            "output": {"path": "approved.txt", "bytes_written": 14}
        },
        "audit_sequence": 4
    })
}

fn server_error(code: &str, message: &str) -> Value {
    json!({"error": {"code": code, "message": message, "details": null}})
}

#[derive(Debug)]
struct FixtureRequest {
    method: String,
    path: String,
    contract: String,
    body: Option<Value>,
}

fn spawn_fixture(
    count: usize,
    handler: impl Fn(usize, FixtureRequest) -> Vec<u8> + Send + Sync + 'static,
) -> (SocketAddr, thread::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let handler = Arc::new(handler);
    let join = thread::spawn(move || {
        for index in 0..count {
            let (mut stream, _) = listener.accept().unwrap();
            let request = read_request(&mut stream);
            let response = handler(index, request);
            stream.write_all(&response).unwrap();
            stream.flush().unwrap();
        }
    });
    (address, join)
}

fn read_request(stream: &mut impl Read) -> FixtureRequest {
    let mut header = Vec::new();
    let mut byte = [0_u8; 1];
    while !header.ends_with(b"\r\n\r\n") {
        stream.read_exact(&mut byte).unwrap();
        header.push(byte[0]);
        assert!(header.len() < 64 * 1024, "fixture request header too large");
    }
    let text = std::str::from_utf8(&header).unwrap();
    let mut lines = text.split("\r\n");
    let request_line = lines.next().unwrap();
    let mut request_parts = request_line.split_whitespace();
    let method = request_parts.next().unwrap().to_owned();
    let path = request_parts.next().unwrap().to_owned();
    let mut contract = None;
    let mut content_length = 0_usize;
    for line in lines {
        let Some((name, value)) = line.split_once(':') else {
            continue;
        };
        if name.eq_ignore_ascii_case("x-nyx-contract-version") {
            contract = Some(value.trim().to_owned());
        }
        if name.eq_ignore_ascii_case("content-length") {
            content_length = value.trim().parse().unwrap();
        }
    }
    let mut body = vec![0_u8; content_length];
    stream.read_exact(&mut body).unwrap();
    FixtureRequest {
        method,
        path,
        contract: contract.expect("contract header"),
        body: (!body.is_empty()).then(|| serde_json::from_slice(&body).unwrap()),
    }
}

fn response(status: u16, body: &Value) -> Vec<u8> {
    let reason = match status {
        200 => "OK",
        201 => "Created",
        400 => "Bad Request",
        403 => "Forbidden",
        404 => "Not Found",
        409 => "Conflict",
        _ => "Status",
    };
    let body = body.to_string();
    let mut bytes = format!(
        "HTTP/1.1 {status} {reason}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nx-nyx-contract-version: 1.0\r\nConnection: close\r\n\r\n",
        body.len()
    )
    .into_bytes();
    bytes.extend_from_slice(body.as_bytes());
    bytes
}

#[test]
fn create_checkpoint_reconciles_nyx_owned_immutable_hashes() {
    let request = fixed_request(WORKSPACE);
    assert_eq!(request.request_sha256().unwrap(), REQUEST_SHA256);
    assert_eq!(request.payload_sha256().unwrap(), PAYLOAD_SHA256);
    assert_eq!(request.scope_sha256().unwrap(), SCOPE_SHA256);
    let expected = serde_json::to_value(&request).unwrap();
    let fixture_request = request.clone();
    let (address, server) = spawn_fixture(1, move |_index, request| {
        assert_eq!(request.method, "POST");
        assert_eq!(request.path, "/v1/nyx/permissions/checkpoints");
        assert_eq!(request.contract, "1.0");
        assert_eq!(request.body.as_ref().unwrap()["request"], expected);
        response(201, &pending_checkpoint(&fixture_request))
    });

    let checkpoint = client(address)
        .create_checkpoint(&NyxPermissionCheckpointCreate::new(request, 30_000).unwrap())
        .unwrap();
    server.join().unwrap();
    assert_eq!(checkpoint.status(), NyxPermissionCheckpointStatus::Pending);
    assert_eq!(checkpoint.request_sha256(), REQUEST_SHA256);
    assert_eq!(
        checkpoint.accepted_operation_id(),
        format!("nyxop_{REQUEST_SHA256}")
    );
}

#[test]
fn approval_and_exact_resume_are_bound_to_one_nyx_token() {
    let request = fixed_request(WORKSPACE);
    let fixture_request = request.clone();
    let (address, server) = spawn_fixture(3, move |index, incoming| {
        assert_eq!(incoming.contract, "1.0");
        match index {
            0 => response(201, &pending_checkpoint(&fixture_request)),
            1 => {
                assert_eq!(incoming.method, "POST");
                assert_eq!(
                    incoming.path,
                    "/v1/nyx/permissions/checkpoints/permckpt_000001_b3edb536116a/resolve"
                );
                let body = incoming.body.unwrap();
                assert_eq!(body["decision"], "approve");
                assert_eq!(body["expected_request_sha256"], REQUEST_SHA256);
                assert_eq!(body["expected_scope_sha256"], SCOPE_SHA256);
                assert_eq!(body["expected_policy_decision_sha256"], POLICY_SHA256);
                assert_eq!(
                    body["conditions"],
                    json!(["exact_scope_only", "single_execution"])
                );
                response(
                    200,
                    &resolution(approved_checkpoint(&fixture_request), Some(RESUME_TOKEN)),
                )
            }
            2 => {
                let body = incoming.body.unwrap();
                assert_eq!(incoming.path, "/v1/nyx/permissions/resume");
                assert_eq!(body["resume_token"], RESUME_TOKEN);
                assert_eq!(
                    body["request"],
                    serde_json::to_value(&fixture_request).unwrap()
                );
                response(200, &resume_result())
            }
            _ => unreachable!(),
        }
    });

    let client = client(address);
    let checkpoint = client
        .create_checkpoint(&NyxPermissionCheckpointCreate::new(request, 30_000).unwrap())
        .unwrap();
    let resolution = client
        .resolve_checkpoint(
            &checkpoint,
            NyxPermissionDecisionKind::Approve,
            Some("reviewed exact payload and scope".to_owned()),
        )
        .unwrap();
    assert_eq!(resolution.resume_token(), Some(RESUME_TOKEN));
    assert_eq!(
        resolution.checkpoint().decided_by(),
        Some("local_anonymous")
    );
    let result = client.resume_approved(&resolution).unwrap();
    server.join().unwrap();
    assert_eq!(result.status(), NyxPermissionCheckpointStatus::Consumed);
    assert_eq!(result.request_sha256(), REQUEST_SHA256);
}

#[test]
fn altered_payload_cannot_reuse_an_approved_token() {
    let request = fixed_request(WORKSPACE);
    let fixture_request = request.clone();
    let (address, server) = spawn_fixture(3, move |index, incoming| match index {
        0 => response(201, &pending_checkpoint(&fixture_request)),
        1 => response(
            200,
            &resolution(approved_checkpoint(&fixture_request), Some(RESUME_TOKEN)),
        ),
        2 => {
            assert_eq!(
                incoming.body.unwrap()["request"]["arguments"]["content"],
                "mutated\n"
            );
            response(
                409,
                &server_error(
                    "StateConflict",
                    "resumed payload or scope does not match immutable approved request",
                ),
            )
        }
        _ => unreachable!(),
    });
    let client = client(address);
    let checkpoint = client
        .create_checkpoint(&NyxPermissionCheckpointCreate::new(request.clone(), 30_000).unwrap())
        .unwrap();
    let approved = client
        .resolve_checkpoint(&checkpoint, NyxPermissionDecisionKind::Approve, None)
        .unwrap();
    let altered = NyxScopedToolRequest::new(
        request.request_id(),
        request.session_id(),
        request.tool_call_id(),
        request.tool_name(),
        json!({"path": "approved.txt", "content": "mutated\n"}),
        request.scope().clone(),
        request.idempotency_key(),
    )
    .unwrap();
    let resume = NyxPermissionResume::from_exact_parts(
        checkpoint.checkpoint_id(),
        approved.resume_token().unwrap(),
        altered,
    )
    .unwrap();
    let error = client.resume_exact(&resume).unwrap_err();
    server.join().unwrap();
    assert!(matches!(
        error,
        NyxPermissionClientError::Rejected {
            status: 409,
            ref error,
            ..
        } if error.code() == "StateConflict"
    ));
}

#[test]
fn denial_returns_no_token_and_later_resume_is_rejected() {
    let request = fixed_request(WORKSPACE);
    let fixture_request = request.clone();
    let (address, server) = spawn_fixture(3, move |index, _incoming| match index {
        0 => response(201, &pending_checkpoint(&fixture_request)),
        1 => response(200, &resolution(denied_checkpoint(&fixture_request), None)),
        2 => response(
            409,
            &server_error(
                "StateConflict",
                "resume rejected for checkpoint status Denied",
            ),
        ),
        _ => unreachable!(),
    });
    let client = client(address);
    let checkpoint = client
        .create_checkpoint(&NyxPermissionCheckpointCreate::new(request.clone(), 30_000).unwrap())
        .unwrap();
    let denied = client
        .resolve_checkpoint(
            &checkpoint,
            NyxPermissionDecisionKind::Deny,
            Some("operator denied exact request".to_owned()),
        )
        .unwrap();
    assert_eq!(denied.resume_token(), None);
    let forced = NyxPermissionResume::from_exact_parts(
        checkpoint.checkpoint_id(),
        "nyxrt_not_authorized",
        request,
    )
    .unwrap();
    assert!(matches!(
        client.resume_exact(&forced),
        Err(NyxPermissionClientError::Rejected { status: 409, .. })
    ));
    server.join().unwrap();
}

#[test]
fn tampered_checkpoint_hash_is_rejected_before_display_or_resume() {
    let request = fixed_request(WORKSPACE);
    let fixture_request = request.clone();
    let (address, server) = spawn_fixture(1, move |_index, _incoming| {
        let mut checkpoint = pending_checkpoint(&fixture_request);
        checkpoint["request_sha256"] = json!("0".repeat(64));
        response(201, &checkpoint)
    });
    let error = client(address)
        .create_checkpoint(&NyxPermissionCheckpointCreate::new(request, 30_000).unwrap())
        .unwrap_err();
    server.join().unwrap();
    assert!(matches!(
        error,
        NyxPermissionClientError::Protocol(
            forge_nyx_client::permission::NyxPermissionProtocolError::HashMismatch {
                field: "request_sha256",
                ..
            }
        )
    ));
}

#[test]
fn uppercase_external_hash_is_rejected_as_noncanonical() {
    let request = fixed_request(WORKSPACE);
    let fixture_request = request.clone();
    let (address, server) = spawn_fixture(1, move |_index, _incoming| {
        let mut checkpoint = pending_checkpoint(&fixture_request);
        checkpoint["request_sha256"] = json!(REQUEST_SHA256.to_ascii_uppercase());
        response(201, &checkpoint)
    });
    let error = client(address)
        .create_checkpoint(&NyxPermissionCheckpointCreate::new(request, 30_000).unwrap())
        .unwrap_err();
    server.join().unwrap();
    assert!(matches!(
        error,
        NyxPermissionClientError::Protocol(
            forge_nyx_client::permission::NyxPermissionProtocolError::InvalidField {
                context: "permission checkpoint",
                field: "request_sha256",
                ..
            }
        )
    ));
}

#[test]
fn audit_report_rejects_zero_sequence() {
    let audit = json!({
        "schema_version": "nyx.1.0",
        "schema_id": "nyx.permission_audit_report.v1",
        "events": [{
            "schema_version": "nyx.1.0",
            "schema_id": "nyx.permission_audit_event.v1",
            "sequence": 0,
            "event": "checkpoint_created",
            "checkpoint_id": "permckpt_000001_b3edb536116a",
            "request_id": "forge-permission-request-1",
            "request_sha256": REQUEST_SHA256,
            "status": "pending",
            "occurred_at_unix_ms": 100
        }]
    });
    let (address, server) = spawn_fixture(1, move |_index, _incoming| response(200, &audit));
    let error = client(address).audit().unwrap_err();
    server.join().unwrap();
    assert!(matches!(
        error,
        NyxPermissionClientError::Protocol(
            forge_nyx_client::permission::NyxPermissionProtocolError::InvalidField {
                context: "permission audit event",
                field: "sequence",
                ..
            }
        )
    ));
}

#[test]
fn audit_report_requires_strictly_increasing_nyx_sequence() {
    let audit = json!({
        "schema_version": "nyx.1.0",
        "schema_id": "nyx.permission_audit_report.v1",
        "events": [
            {
                "schema_version": "nyx.1.0",
                "schema_id": "nyx.permission_audit_event.v1",
                "sequence": 1,
                "event": "checkpoint_created",
                "checkpoint_id": "permckpt_000001_b3edb536116a",
                "request_id": "forge-permission-request-1",
                "request_sha256": REQUEST_SHA256,
                "status": "pending",
                "occurred_at_unix_ms": 100,
                "actor_id": "local_anonymous"
            },
            {
                "schema_version": "nyx.1.0",
                "schema_id": "nyx.permission_audit_event.v1",
                "sequence": 2,
                "event": "approved",
                "checkpoint_id": "permckpt_000001_b3edb536116a",
                "request_id": "forge-permission-request-1",
                "request_sha256": REQUEST_SHA256,
                "status": "approved",
                "occurred_at_unix_ms": 200,
                "actor_id": "local_anonymous"
            }
        ]
    });
    let (address, server) = spawn_fixture(1, move |_index, incoming| {
        assert_eq!(incoming.method, "GET");
        assert_eq!(incoming.path, "/v1/nyx/permissions/audit");
        response(200, &audit)
    });
    let report = client(address).audit().unwrap();
    server.join().unwrap();
    assert_eq!(report.events().len(), 2);
    assert_eq!(
        report.events()[1].event(),
        NyxPermissionAuditEventKind::Approved
    );
}

#[test]
fn audit_report_rejects_duplicate_or_reordered_sequence() {
    let audit = json!({
        "schema_version": "nyx.1.0",
        "schema_id": "nyx.permission_audit_report.v1",
        "events": [
            {
                "schema_version": "nyx.1.0",
                "schema_id": "nyx.permission_audit_event.v1",
                "sequence": 2,
                "event": "checkpoint_created",
                "checkpoint_id": "permckpt_000001_b3edb536116a",
                "request_id": "forge-permission-request-1",
                "request_sha256": REQUEST_SHA256,
                "status": "pending",
                "occurred_at_unix_ms": 100
            },
            {
                "schema_version": "nyx.1.0",
                "schema_id": "nyx.permission_audit_event.v1",
                "sequence": 2,
                "event": "approved",
                "checkpoint_id": "permckpt_000001_b3edb536116a",
                "request_id": "forge-permission-request-1",
                "request_sha256": REQUEST_SHA256,
                "status": "approved",
                "occurred_at_unix_ms": 200
            }
        ]
    });
    let (address, server) = spawn_fixture(1, move |_index, _incoming| response(200, &audit));
    let error = client(address).audit().unwrap_err();
    server.join().unwrap();
    assert!(matches!(
        error,
        NyxPermissionClientError::Protocol(
            forge_nyx_client::permission::NyxPermissionProtocolError::NonCanonical {
                context: "permission audit sequence"
            }
        )
    ));
}

#[test]
#[ignore = "requires an independently running Nyx_Server permission API"]
fn real_nyx_permission_gate_proves_exact_approval_and_replay_rejection() {
    let address = std::env::var("FORGE_NYX_ADDR")
        .expect("set FORGE_NYX_ADDR to the running Nyx_Server address")
        .parse::<SocketAddr>()
        .expect("FORGE_NYX_ADDR must be ip:port");
    let workspace = std::env::var("FORGE_NYX_WORKSPACE_ROOT")
        .expect("set FORGE_NYX_WORKSPACE_ROOT to Nyx's exact workspace root");
    let session_id =
        std::env::var("FORGE_NYX_SESSION_ID").unwrap_or_else(|_| "sess_000001".to_owned());
    let request = NyxScopedToolRequest::new(
        "forgeos-nyx101-real-request",
        session_id,
        "forgeos-nyx101-real-tool-call",
        "repo.write_file",
        json!({
            "path": "forgeos-nyx101-real.txt",
            "content": "ForgeOS consumed the Nyx-owned permission contract.\n"
        }),
        NyxPermissionScope::new(&workspace)
            .unwrap()
            .with_allowed_paths(["forgeos-nyx101-real.txt"])
            .unwrap()
            .with_declared_side_effects(["writes_files"])
            .unwrap(),
        "forgeos-nyx101-real-idempotency",
    )
    .unwrap();
    let client = client(address);
    let checkpoint = client
        .create_checkpoint(&NyxPermissionCheckpointCreate::new(request, 30_000).unwrap())
        .unwrap();
    println!("FORGE_NYX_PERMISSION_CHECKPOINT={checkpoint:#?}");
    let resolution = client
        .resolve_checkpoint(
            &checkpoint,
            NyxPermissionDecisionKind::Approve,
            Some("ForgeOS operator reviewed exact Nyx request hashes".to_owned()),
        )
        .unwrap();
    let resume = NyxPermissionResume::new(&resolution).unwrap();
    let result = client.resume_exact(&resume).unwrap();
    println!("FORGE_NYX_PERMISSION_RESULT={result:#?}");
    assert_eq!(result.status(), NyxPermissionCheckpointStatus::Consumed);
    let replay = client.resume_exact(&resume).unwrap_err();
    assert!(matches!(
        replay,
        NyxPermissionClientError::Rejected { status: 409, .. }
    ));
    let audit = client.audit().unwrap();
    assert!(audit.events().iter().any(|event| {
        event.checkpoint_id() == checkpoint.checkpoint_id()
            && event.event() == NyxPermissionAuditEventKind::ExecutionCompleted
    }));
}
