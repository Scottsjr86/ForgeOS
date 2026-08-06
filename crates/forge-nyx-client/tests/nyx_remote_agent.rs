use forge_nyx_client::protocol::NyxProtocolVersion;
use forge_nyx_client::remote_agent::{
    NyxRemoteAgentBudget, NyxRemoteAgentProtocolError, NyxRemoteAgentRunCreate,
    NyxRemoteAgentRunState, NyxRemoteAgentScope, NyxRemoteAgentStartMode,
};
use forge_nyx_client::remote_agent_client::{NyxRemoteAgentClient, NyxRemoteAgentClientError};
use forge_nyx_client::transport::{NyxClientConfig, NyxTransportEndpoint};
use serde_json::{Value, json};
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

const REVISION: &str = "db1e5b1da1070995a38624c35036b5afbab179b7";
const WORKTREE_ID: &str = "nyxwt_fedb3ca524283ab4bca3312c";
const WORKSPACE_SHA256: &str = "fedb3ca524283ab4bca3312c77881905585463a5bd2b1b42ab5c9ceeab4b8d84";

fn version() -> NyxProtocolVersion {
    NyxProtocolVersion::new(1, 0)
}

fn client(address: SocketAddr) -> NyxRemoteAgentClient {
    let config = NyxClientConfig::new(NyxTransportEndpoint::tcp(address), [version()])
        .unwrap()
        .with_io_timeout(Duration::from_secs(2));
    NyxRemoteAgentClient::new(config)
}

fn budget(max_model_turns: u32) -> NyxRemoteAgentBudget {
    NyxRemoteAgentBudget::new(
        max_model_turns,
        0,
        1024,
        256,
        1280,
        30_000,
        0,
        1,
        0,
        Some(5000),
    )
    .unwrap()
}

fn request(
    key: &str,
    scope_path: &str,
    start_mode: NyxRemoteAgentStartMode,
) -> NyxRemoteAgentRunCreate {
    NyxRemoteAgentRunCreate::new(
        key,
        "primary",
        "fixture.complete.v1",
        format!("remote task {key}"),
        REVISION,
        WORKTREE_ID,
        NyxRemoteAgentScope::new([scope_path], true).unwrap(),
        budget(2),
        start_mode,
    )
    .unwrap()
}

fn unknown_cost() -> Value {
    json!({
        "schema_version": "nyx.1.0",
        "schema_id": "nyx.route_cost.v1",
        "source": "unknown",
        "currency": null,
        "monetary_microunits": null,
        "prompt_tokens": null,
        "completion_tokens": null,
        "total_tokens": null,
        "energy_millijoules": null,
        "memory_bytes": null,
        "device_use": []
    })
}

fn provider_cost() -> Value {
    json!({
        "schema_version": "nyx.1.0",
        "schema_id": "nyx.route_cost.v1",
        "source": "provider_reported",
        "currency": "USD",
        "monetary_microunits": 1234,
        "prompt_tokens": 11,
        "completion_tokens": 7,
        "total_tokens": 18,
        "energy_millijoules": 55,
        "memory_bytes": 4096,
        "device_use": ["cpu"]
    })
}

fn route() -> Value {
    json!({
        "schema_version": "nyx.1.0",
        "schema_id": "nyx.route_candidate.v1",
        "backend": {
            "schema_version": "nyx.1.0",
            "schema_id": "nyx.backend_identity.v1",
            "backend_id": "primary",
            "kind": "openai_compat",
            "provider": "primary",
            "transport": "http",
            "endpoint_posture": "loopback",
            "configuration_version": "1"
        },
        "model": {
            "schema_version": "nyx.1.0",
            "schema_id": "nyx.routable_model_identity.v1",
            "public_model_id": "fixture.complete.v1",
            "backend_local_name": "fixture.complete.v1",
            "backend_id": "primary",
            "runtime_instance_id": null
        },
        "capabilities": {
            "schema_version": "nyx.1.0",
            "schema_id": "nyx.backend_capabilities.v1",
            "max_context_tokens": null,
            "modalities": ["text"],
            "streaming": false,
            "tool_calling": false,
            "json_mode": false,
            "embedding": false,
            "max_concurrency": null,
            "deterministic": false
        },
        "health": "ready",
        "policy_posture": "loopback_only",
        "measured_signals": ["model_list_success"],
        "declared_cost": unknown_cost()
    })
}

fn record(create: &NyxRemoteAgentRunCreate, state: NyxRemoteAgentRunState) -> Value {
    let request_sha256 = create.request_sha256().unwrap();
    let (
        terminal_at,
        continuation_allowed,
        output,
        error_code,
        error_message,
        cancellation_reason,
        audit,
        cost,
        turns,
    ) = match state {
        NyxRemoteAgentRunState::Queued => (
            Value::Null,
            true,
            Value::Null,
            Value::Null,
            Value::Null,
            Value::Null,
            json!(["accepted", "queued"]),
            unknown_cost(),
            0,
        ),
        NyxRemoteAgentRunState::Running => (
            Value::Null,
            true,
            Value::Null,
            Value::Null,
            Value::Null,
            Value::Null,
            json!(["accepted", "queued", "running"]),
            unknown_cost(),
            0,
        ),
        NyxRemoteAgentRunState::Completed => (
            json!("2026-08-05T20:35:32Z"),
            false,
            json!("fixture-provider:remote task"),
            Value::Null,
            Value::Null,
            Value::Null,
            json!(["accepted", "queued", "running", "completed"]),
            provider_cost(),
            1,
        ),
        NyxRemoteAgentRunState::Failed => (
            json!("2026-08-05T20:35:32Z"),
            false,
            Value::Null,
            json!("ModelCallFailed"),
            json!("fixture provider failed"),
            Value::Null,
            json!(["accepted", "queued", "running", "failed"]),
            unknown_cost(),
            1,
        ),
        NyxRemoteAgentRunState::Cancelled => (
            json!("2026-08-05T20:35:32Z"),
            false,
            Value::Null,
            Value::Null,
            Value::Null,
            json!("client_cancelled_before_execution"),
            json!(["accepted", "queued", "cancelled"]),
            unknown_cost(),
            0,
        ),
        NyxRemoteAgentRunState::BudgetHit => (
            json!("2026-08-05T20:35:32Z"),
            false,
            Value::Null,
            json!("budget_hit"),
            json!("declared max_model_turns is zero"),
            Value::Null,
            json!(["accepted", "queued", "budget_hit_preflight"]),
            unknown_cost(),
            0,
        ),
    };
    let state_text = match state {
        NyxRemoteAgentRunState::Queued => "queued",
        NyxRemoteAgentRunState::Running => "running",
        NyxRemoteAgentRunState::Completed => "completed",
        NyxRemoteAgentRunState::Failed => "failed",
        NyxRemoteAgentRunState::Cancelled => "cancelled",
        NyxRemoteAgentRunState::BudgetHit => "budget_hit",
    };
    json!({
        "schema_version": "nyx.1.0",
        "schema_id": "nyx.remote_agent_run_record.v1",
        "task_id": format!("nyxtask_{}", &request_sha256[..24]),
        "run_id": format!("nyxrun_{}", &request_sha256[24..48]),
        "accepted_operation_id": format!("nyxop_{request_sha256}"),
        "request_sha256": request_sha256,
        "idempotency_key": create.idempotency_key(),
        "request": create,
        "state": state_text,
        "created_at": "2026-08-05T20:35:32Z",
        "updated_at": "2026-08-05T20:35:32Z",
        "terminal_at": terminal_at,
        "provider_id": create.provider_id(),
        "model_id": create.model_id(),
        "route": route(),
        "source": {
            "schema_version": "nyx.1.0",
            "schema_id": "nyx.remote_agent_source_binding.v1",
            "source_revision": create.expected_source_revision(),
            "workspace_root_sha256": WORKSPACE_SHA256,
            "worktree_id": create.expected_worktree_id(),
            "worktree_clean": true,
            "scope": create.scope(),
            "scope_sha256": create.scope().sha256().unwrap()
        },
        "budget": create.budget(),
        "recorded_cost": cost,
        "model_turns": turns,
        "tool_steps": 0,
        "output": output,
        "error_code": error_code,
        "error_message": error_message,
        "cancellation_reason": cancellation_reason,
        "continuation_allowed": continuation_allowed,
        "audit": audit
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
fn create_run_reconciles_nyx_owned_identity_source_budget_and_cost() {
    let create = request(
        "complete-001",
        "scope-a",
        NyxRemoteAgentStartMode::Immediate,
    );
    let expected_body = serde_json::to_value(&create).unwrap();
    let fixture = record(&create, NyxRemoteAgentRunState::Completed);
    let (address, server) = spawn_fixture(1, move |_index, incoming| {
        assert_eq!(incoming.method, "POST");
        assert_eq!(incoming.path, "/v1/nyx/agent/runs");
        assert_eq!(incoming.contract, "1.0");
        assert_eq!(incoming.body, Some(expected_body.clone()));
        response(201, &fixture)
    });
    let result = client(address).create_run(&create).unwrap();
    server.join().unwrap();
    assert_eq!(result.state(), NyxRemoteAgentRunState::Completed);
    assert_eq!(result.source().source_revision(), REVISION);
    assert_eq!(result.source().worktree_id(), WORKTREE_ID);
    assert_eq!(result.recorded_cost().source(), "provider_reported");
    assert_eq!(result.recorded_cost().total_tokens(), Some(18));
    assert_eq!(result.recorded_cost().monetary_microunits(), Some(1234));
}

#[test]
fn idempotent_replay_may_return_200_but_must_return_the_exact_record() {
    let create = request("replay-001", "scope-a", NyxRemoteAgentStartMode::Immediate);
    let fixture = record(&create, NyxRemoteAgentRunState::Completed);
    let (address, server) = spawn_fixture(1, move |_index, _incoming| response(200, &fixture));
    let result = client(address).create_run(&create).unwrap();
    server.join().unwrap();
    assert_eq!(result.request(), &create);
}

#[test]
fn list_requires_stable_unique_run_order() {
    let first = request("list-a", "scope-a", NyxRemoteAgentStartMode::Deferred);
    let second = request("list-b", "scope-b", NyxRemoteAgentStartMode::Deferred);
    let mut runs = vec![
        record(&first, NyxRemoteAgentRunState::Queued),
        record(&second, NyxRemoteAgentRunState::Queued),
    ];
    runs.sort_by(|left, right| left["run_id"].as_str().cmp(&right["run_id"].as_str()));
    let list = json!({
        "schema_version": "nyx.1.0",
        "schema_id": "nyx.remote_agent_run_list.v1",
        "runs": runs
    });
    let (address, server) = spawn_fixture(1, move |_index, incoming| {
        assert_eq!(incoming.method, "GET");
        assert_eq!(incoming.path, "/v1/nyx/agent/runs");
        response(200, &list)
    });
    let result = client(address).runs().unwrap();
    server.join().unwrap();
    assert_eq!(result.runs().len(), 2);
    assert!(result.runs()[0].run_id() < result.runs()[1].run_id());
}

#[test]
fn list_rejects_duplicate_or_reordered_runs() {
    let create = request("duplicate", "scope-a", NyxRemoteAgentStartMode::Deferred);
    let run = record(&create, NyxRemoteAgentRunState::Queued);
    let list = json!({
        "schema_version": "nyx.1.0",
        "schema_id": "nyx.remote_agent_run_list.v1",
        "runs": [run.clone(), run]
    });
    let (address, server) = spawn_fixture(1, move |_index, _incoming| response(200, &list));
    let error = client(address).runs().unwrap_err();
    server.join().unwrap();
    assert!(matches!(
        error,
        NyxRemoteAgentClientError::Protocol(NyxRemoteAgentProtocolError::NonCanonical {
            context: "run list ordering"
        })
    ));
}

#[test]
fn cancel_binds_control_to_the_exact_task_and_request() {
    let create = request("cancel-001", "scope-a", NyxRemoteAgentStartMode::Deferred);
    let queued = record(&create, NyxRemoteAgentRunState::Queued);
    let cancelled = record(&create, NyxRemoteAgentRunState::Cancelled);
    let run_id = queued["run_id"].as_str().unwrap().to_owned();
    let task_id = queued["task_id"].as_str().unwrap().to_owned();
    let request_sha256 = queued["request_sha256"].as_str().unwrap().to_owned();
    let (address, server) = spawn_fixture(1, move |_index, incoming| {
        assert_eq!(incoming.method, "POST");
        assert_eq!(incoming.path, format!("/v1/nyx/agent/runs/{run_id}/cancel"));
        let body = incoming.body.unwrap();
        assert_eq!(body["task_id"], task_id);
        assert_eq!(body["request_sha256"], request_sha256);
        response(200, &cancelled)
    });
    let queued_record: forge_nyx_client::remote_agent::NyxRemoteAgentRunRecord =
        serde_json::from_value(queued).unwrap();
    let result = client(address).cancel(&queued_record).unwrap();
    server.join().unwrap();
    assert_eq!(result.state(), NyxRemoteAgentRunState::Cancelled);
    assert!(!result.continuation_allowed());
}

#[test]
fn continuation_accepts_only_the_same_queued_run_identity() {
    let create = request("continue-001", "scope-a", NyxRemoteAgentStartMode::Deferred);
    let queued = record(&create, NyxRemoteAgentRunState::Queued);
    let completed = record(&create, NyxRemoteAgentRunState::Completed);
    let run_id = queued["run_id"].as_str().unwrap().to_owned();
    let (address, server) = spawn_fixture(1, move |_index, incoming| {
        assert_eq!(
            incoming.path,
            format!("/v1/nyx/agent/runs/{run_id}/continue")
        );
        response(200, &completed)
    });
    let queued_record: forge_nyx_client::remote_agent::NyxRemoteAgentRunRecord =
        serde_json::from_value(queued).unwrap();
    let result = client(address).continue_run(&queued_record).unwrap();
    server.join().unwrap();
    assert_eq!(result.state(), NyxRemoteAgentRunState::Completed);
}

#[test]
fn foreign_control_rejection_is_preserved_as_nyx_server_truth() {
    let create = request("foreign-001", "scope-a", NyxRemoteAgentStartMode::Deferred);
    let queued = record(&create, NyxRemoteAgentRunState::Queued);
    let (address, server) = spawn_fixture(1, move |_index, _incoming| {
        response(
            409,
            &server_error(
                "StateConflict",
                "remote agent control identity does not match the exact accepted request",
            ),
        )
    });
    let queued_record: forge_nyx_client::remote_agent::NyxRemoteAgentRunRecord =
        serde_json::from_value(queued).unwrap();
    let error = client(address).cancel(&queued_record).unwrap_err();
    server.join().unwrap();
    assert!(matches!(
        error,
        NyxRemoteAgentClientError::Rejected {
            status: 409,
            ref error,
            ..
        } if error.code() == "StateConflict"
    ));
}

#[test]
fn tampered_request_hash_is_rejected_before_display() {
    let create = request("tamper-001", "scope-a", NyxRemoteAgentStartMode::Immediate);
    let mut fixture = record(&create, NyxRemoteAgentRunState::Completed);
    fixture["request_sha256"] = json!("0".repeat(64));
    let (address, server) = spawn_fixture(1, move |_index, _incoming| response(201, &fixture));
    let error = client(address).create_run(&create).unwrap_err();
    server.join().unwrap();
    assert!(matches!(
        error,
        NyxRemoteAgentClientError::Protocol(NyxRemoteAgentProtocolError::HashMismatch {
            field: "request_sha256",
            ..
        })
    ));
}

#[test]
fn uppercase_external_hash_is_rejected_as_noncanonical() {
    let create = request(
        "uppercase-001",
        "scope-a",
        NyxRemoteAgentStartMode::Immediate,
    );
    let mut fixture = record(&create, NyxRemoteAgentRunState::Completed);
    fixture["request_sha256"] = json!(create.request_sha256().unwrap().to_ascii_uppercase());
    let (address, server) = spawn_fixture(1, move |_index, _incoming| response(201, &fixture));
    let error = client(address).create_run(&create).unwrap_err();
    server.join().unwrap();
    assert!(matches!(
        error,
        NyxRemoteAgentClientError::Protocol(NyxRemoteAgentProtocolError::InvalidField {
            context: "remote-agent record",
            field: "request_sha256",
            ..
        })
    ));
}

#[test]
fn route_provider_and_model_must_match_the_accepted_request() {
    let create = request(
        "route-mismatch",
        "scope-a",
        NyxRemoteAgentStartMode::Immediate,
    );
    let mut fixture = record(&create, NyxRemoteAgentRunState::Completed);
    fixture["route"]["model"]["public_model_id"] = json!("some-other-model");
    let (address, server) = spawn_fixture(1, move |_index, _incoming| response(201, &fixture));
    let error = client(address).create_run(&create).unwrap_err();
    server.join().unwrap();
    assert!(matches!(
        error,
        NyxRemoteAgentClientError::Protocol(NyxRemoteAgentProtocolError::ImmutableMismatch {
            field: "route provider or model attribution"
        })
    ));
}

#[test]
fn cost_and_execution_counters_cannot_exceed_the_declared_budget() {
    let create = request(
        "budget-overrun",
        "scope-a",
        NyxRemoteAgentStartMode::Immediate,
    );
    let mut fixture = record(&create, NyxRemoteAgentRunState::Completed);
    fixture["recorded_cost"]["total_tokens"] = json!(4096);
    fixture["recorded_cost"]["prompt_tokens"] = json!(2048);
    fixture["recorded_cost"]["completion_tokens"] = json!(2048);
    let (address, server) = spawn_fixture(1, move |_index, _incoming| response(201, &fixture));
    let error = client(address).create_run(&create).unwrap_err();
    server.join().unwrap();
    assert!(matches!(
        error,
        NyxRemoteAgentClientError::Protocol(NyxRemoteAgentProtocolError::InvalidField {
            context: "remote-agent record",
            field: "recorded_cost.prompt_tokens",
            ..
        })
    ));
}

#[test]
fn terminal_records_must_be_truthful_and_cannot_continue() {
    let create = request(
        "terminal-lie",
        "scope-a",
        NyxRemoteAgentStartMode::Immediate,
    );
    let mut fixture = record(&create, NyxRemoteAgentRunState::Cancelled);
    fixture["continuation_allowed"] = json!(true);
    let (address, server) = spawn_fixture(1, move |_index, _incoming| response(201, &fixture));
    let error = client(address).create_run(&create).unwrap_err();
    server.join().unwrap();
    assert!(matches!(
        error,
        NyxRemoteAgentClientError::Protocol(NyxRemoteAgentProtocolError::InvalidField {
            context: "remote-agent record",
            field: "terminal state",
            ..
        })
    ));
}

#[test]
#[ignore = "requires an independently running Nyx_Server remote-agent API"]
fn real_nyx_remote_agent_gate_proves_deferred_create_cancel_and_list_truth() {
    let address = std::env::var("FORGE_NYX_ADDR")
        .expect("set FORGE_NYX_ADDR to the running Nyx_Server address")
        .parse::<SocketAddr>()
        .expect("FORGE_NYX_ADDR must be ip:port");
    let revision = std::env::var("FORGE_NYX_SOURCE_REVISION")
        .expect("set FORGE_NYX_SOURCE_REVISION to Nyx's exact workspace revision");
    let worktree_id = std::env::var("FORGE_NYX_WORKTREE_ID")
        .expect("set FORGE_NYX_WORKTREE_ID to Nyx's exact workspace identity");
    let provider = std::env::var("FORGE_NYX_PROVIDER_ID").unwrap_or_else(|_| "primary".to_owned());
    let model = std::env::var("FORGE_NYX_MODEL_ID")
        .expect("set FORGE_NYX_MODEL_ID to a model exposed by the selected provider");
    let create = NyxRemoteAgentRunCreate::new(
        "forgeos-agent100-real-deferred",
        provider,
        model,
        "ForgeOS real remote-agent contract witness",
        revision,
        worktree_id,
        NyxRemoteAgentScope::new(["scope-a"], true).unwrap(),
        budget(2),
        NyxRemoteAgentStartMode::Deferred,
    )
    .unwrap();
    let client = client(address);
    let queued = client.create_run(&create).unwrap();
    println!("FORGE_NYX_REMOTE_AGENT_QUEUED={queued:#?}");
    assert_eq!(queued.state(), NyxRemoteAgentRunState::Queued);
    let fetched = client.run(queued.run_id()).unwrap();
    assert_eq!(fetched, queued);
    let listed = client.runs().unwrap();
    assert!(listed.runs().iter().any(|run| run == &queued));
    let cancelled = client.cancel(&queued).unwrap();
    println!("FORGE_NYX_REMOTE_AGENT_CANCELLED={cancelled:#?}");
    assert_eq!(cancelled.state(), NyxRemoteAgentRunState::Cancelled);
    assert!(!cancelled.continuation_allowed());
    assert!(matches!(
        client.continue_run(&cancelled),
        Err(NyxRemoteAgentClientError::Rejected { status: 409, .. })
    ));
}
