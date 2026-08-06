use forge_nyx_client::conversation::{
    NyxConversationControl, NyxConversationCreate, NyxConversationEventKind,
    NyxConversationMessageCreate, NyxConversationProtocolError, NyxModelReadiness,
    NyxOpenAiChatMessage, NyxOpenAiChatRequest, NyxOpenAiChatRole, NyxOpenAiStreamEventKind,
    NyxSessionControl, NyxSessionCreate,
};
use forge_nyx_client::conversation_client::{NyxConversationClient, NyxConversationClientError};
use forge_nyx_client::protocol::NyxProtocolVersion;
use forge_nyx_client::transport::{NyxClientConfig, NyxTransportEndpoint};
use serde_json::{Value, json};
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener};
use std::sync::Arc;
use std::thread;

const TOKEN: &str = "forgeos-conversation-test-token";
const ROOT: &str = "/tmp/forgeos-conversation-workspace";
const SESSION_ID: &str = "sess_000101";
const CONVERSATION_ID: &str = "thr_000201";
const RUN_ID: &str = "run_000301";

fn client(address: SocketAddr) -> NyxConversationClient {
    let config = NyxClientConfig::new(
        NyxTransportEndpoint::tcp(address),
        [NyxProtocolVersion::new(1, 0)],
    )
    .unwrap()
    .with_bearer_token(TOKEN)
    .unwrap();
    NyxConversationClient::new(config)
}

fn model_catalog() -> Value {
    json!({
        "schema_version": "nyx.1.0",
        "schema_id": "nyx.model_catalog.v1",
        "ordering": "model_id_asc_then_backend_id_asc",
        "models": [{
            "schema_version": "nyx.1.0",
            "schema_id": "nyx.routable_model.v1",
            "model_id": "mock.echo.v1",
            "display_name": "Mock Echo",
            "backend_id": "primary",
            "backend_kind": "mock",
            "capabilities": {
                "max_context_tokens": 8192,
                "tool_calling": false,
                "json_mode": false,
                "streaming": false
            },
            "readiness": "ready",
            "readiness_reason": "bounded_backend_model_listing_succeeded",
            "availability": "local",
            "default_constraints_profile": "phase0_default"
        }]
    })
}

fn session_view(session_id: &str, selected: bool, closed: bool) -> Value {
    json!({
        "schema_version": "nyx.1.0",
        "schema_id": "nyx.session_view.v1",
        "state": {
            "session_id": session_id,
            "created_at": "2026-08-06T00:00:00Z",
            "workspace": {
                "root": ROOT,
                "symlink_policy": "deny_escape",
                "exclude_dirs": [".git", "node_modules", "target"],
                "max_file_bytes": 10485760,
                "max_list_entries": 20000,
                "max_list_depth": 25
            },
            "network_policy": "deny",
            "active_thread_id": CONVERSATION_ID
        },
        "name": "ForgeOS",
        "updated_at": "2026-08-06T00:00:01Z",
        "closed_at": closed.then_some("2026-08-06T00:00:02Z"),
        "selected": selected
    })
}

fn conversation_view(conversation_id: &str, session_id: &str, closed: bool) -> Value {
    json!({
        "schema_version": "nyx.1.0",
        "schema_id": "nyx.conversation_view.v1",
        "thread": {
            "thread_id": conversation_id,
            "session_id": session_id,
            "created_at": "2026-08-06T00:00:00Z",
            "title": "Conversation"
        },
        "updated_at": "2026-08-06T00:00:01Z",
        "closed_at": closed.then_some("2026-08-06T00:00:02Z")
    })
}

fn message(
    message_id: &str,
    sequence: u64,
    role: &str,
    content: &str,
    run_id: Option<&str>,
    model: Option<Value>,
) -> Value {
    json!({
        "schema_version": "nyx.1.0",
        "schema_id": "nyx.conversation_message.v1",
        "message_id": message_id,
        "session_id": SESSION_ID,
        "conversation_id": CONVERSATION_ID,
        "sequence": sequence,
        "role": role,
        "content": content,
        "created_at": "2026-08-06T00:00:03Z",
        "run_id": run_id,
        "model": model
    })
}

fn attribution() -> Value {
    json!({
        "model_id": "mock.echo.v1",
        "backend_id": "primary",
        "backend_kind": "mock"
    })
}

fn conversation_response() -> Value {
    let model = attribution();
    json!({
        "schema_version": "nyx.1.0",
        "schema_id": "nyx.conversation_response.v1",
        "run_id": RUN_ID,
        "session_id": SESSION_ID,
        "conversation_id": CONVERSATION_ID,
        "model": model.clone(),
        "user_message": message("msg_000001", 0, "user", "hello", None, None),
        "assistant_message": message(
            "msg_000002",
            1,
            "assistant",
            "echo: hello",
            Some(RUN_ID),
            Some(model.clone()),
        ),
        "events": [
            {
                "schema_version": "nyx.1.0",
                "schema_id": "nyx.conversation_event.v1",
                "run_id": RUN_ID,
                "session_id": SESSION_ID,
                "conversation_id": CONVERSATION_ID,
                "sequence": 0,
                "choice_index": 0,
                "event_kind": "response_created",
                "payload": {"model": model}
            },
            {
                "schema_version": "nyx.1.0",
                "schema_id": "nyx.conversation_event.v1",
                "run_id": RUN_ID,
                "session_id": SESSION_ID,
                "conversation_id": CONVERSATION_ID,
                "sequence": 1,
                "choice_index": 0,
                "event_kind": "output_text_delta",
                "payload": {"text": "echo: hello"}
            },
            {
                "schema_version": "nyx.1.0",
                "schema_id": "nyx.conversation_event.v1",
                "run_id": RUN_ID,
                "session_id": SESSION_ID,
                "conversation_id": CONVERSATION_ID,
                "sequence": 2,
                "choice_index": 0,
                "event_kind": "response_completed",
                "payload": {"finish_reason": "stop"}
            }
        ]
    })
}

#[derive(Debug)]
struct FixtureRequest {
    method: String,
    path: String,
    contract: String,
    authorization: Option<String>,
    accept: Option<String>,
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
        assert!(header.len() < 64 * 1024);
    }
    let text = std::str::from_utf8(&header).unwrap();
    let mut lines = text.split("\r\n");
    let mut request_parts = lines.next().unwrap().split_whitespace();
    let method = request_parts.next().unwrap().to_owned();
    let path = request_parts.next().unwrap().to_owned();
    let mut contract = None;
    let mut authorization = None;
    let mut accept = None;
    let mut content_length = 0_usize;
    for line in lines {
        let Some((name, value)) = line.split_once(':') else {
            continue;
        };
        if name.eq_ignore_ascii_case("x-nyx-contract-version") {
            contract = Some(value.trim().to_owned());
        }
        if name.eq_ignore_ascii_case("authorization") {
            authorization = Some(value.trim().to_owned());
        }
        if name.eq_ignore_ascii_case("accept") {
            accept = Some(value.trim().to_owned());
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
        contract: contract.unwrap(),
        authorization,
        accept,
        body: (!body.is_empty()).then(|| serde_json::from_slice(&body).unwrap()),
    }
}

fn openai_stream_body(sequence_override: Option<u64>) -> String {
    let frames = [
        json!({
            "id": RUN_ID,
            "object": "chat.completion.chunk",
            "created": 1785974400_i64,
            "model": "mock.echo.v1",
            "choices": [{"index": 0, "delta": {"role": "assistant"}, "finish_reason": null}],
            "metadata": {"nyx": {
                "schema_id": "nyx.openai_chat_stream_event.v1",
                "sequence": 0,
                "event": "response.created"
            }}
        }),
        json!({
            "id": RUN_ID,
            "object": "chat.completion.chunk",
            "created": 1785974400_i64,
            "model": "mock.echo.v1",
            "choices": [{"index": 0, "delta": {"content": "echo: hello"}, "finish_reason": null}],
            "metadata": {"nyx": {
                "schema_id": "nyx.openai_chat_stream_event.v1",
                "sequence": sequence_override.unwrap_or(1),
                "event": "response.output_text.delta"
            }}
        }),
        json!({
            "id": RUN_ID,
            "object": "chat.completion.chunk",
            "created": 1785974400_i64,
            "model": "mock.echo.v1",
            "choices": [{"index": 0, "delta": {}, "finish_reason": "stop"}],
            "metadata": {"nyx": {
                "schema_id": "nyx.openai_chat_stream_event.v1",
                "sequence": 2,
                "event": "response.completed"
            }}
        }),
    ];
    let mut body = String::new();
    for frame in frames {
        body.push_str("data: ");
        body.push_str(&frame.to_string());
        body.push_str("\n\n");
    }
    body.push_str("data: [DONE]\n\n");
    body
}

fn sse_response(status: u16, stream_schema: Option<&str>, body: &str) -> Vec<u8> {
    sse_response_with_optional_contract(status, stream_schema, Some("1.0"), body)
}

fn sse_response_with_optional_contract(
    status: u16,
    stream_schema: Option<&str>,
    contract: Option<&str>,
    body: &str,
) -> Vec<u8> {
    let schema_header = stream_schema
        .map(|schema| format!("x-nyx-stream-schema: {schema}\r\n"))
        .unwrap_or_default();
    let contract_header = contract
        .map(|version| format!("x-nyx-contract-version: {version}\r\n"))
        .unwrap_or_default();
    let mut bytes = format!(
        "HTTP/1.1 {status} OK\r\nContent-Type: text/event-stream\r\nContent-Length: {}\r\n{contract_header}{schema_header}Connection: close\r\n\r\n",
        body.len()
    )
    .into_bytes();
    bytes.extend_from_slice(body.as_bytes());
    bytes
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

fn assert_common(request: &FixtureRequest, method: &str, path: &str) {
    assert_eq!(request.method, method);
    assert_eq!(request.path, path);
    assert_eq!(request.contract, "1.0");
    assert_eq!(
        request.authorization.as_deref(),
        Some("Bearer forgeos-conversation-test-token")
    );
    let expected_accept = if path == "/v1/chat/completions" {
        "text/event-stream"
    } else {
        "application/json"
    };
    assert_eq!(request.accept.as_deref(), Some(expected_accept));
}

#[test]
fn model_catalog_is_typed_ordered_and_bearer_authenticated() {
    let payload = model_catalog();
    let (address, server) = spawn_fixture(1, move |_index, request| {
        assert_common(&request, "GET", "/v1/nyx/models");
        response(200, &payload)
    });
    let catalog = client(address).models().unwrap();
    server.join().unwrap();
    let model = catalog.find("mock.echo.v1").unwrap();
    assert_eq!(model.readiness(), NyxModelReadiness::Ready);
    assert_eq!(model.backend_id(), "primary");
    assert_eq!(model.capabilities().max_context_tokens(), Some(8192));
}

#[test]
fn model_catalog_rejects_duplicate_identity() {
    let mut payload = model_catalog();
    payload["models"] = json!([payload["models"][0].clone(), payload["models"][0].clone()]);
    let (address, server) = spawn_fixture(1, move |_index, _request| response(200, &payload));
    let error = client(address).models().unwrap_err();
    server.join().unwrap();
    assert!(matches!(
        error,
        NyxConversationClientError::Protocol(NyxConversationProtocolError::NonCanonical {
            context: "model catalog"
        })
    ));
}

#[test]
fn session_creation_preserves_request_and_server_identity() {
    let request = NyxSessionCreate::new("forge-session-create", "ForgeOS", ROOT);
    let expected = serde_json::to_value(&request).unwrap();
    let payload = session_view(SESSION_ID, true, false);
    let (address, server) = spawn_fixture(1, move |_index, incoming| {
        assert_common(&incoming, "POST", "/v1/nyx/sessions");
        assert_eq!(incoming.body, Some(expected.clone()));
        response(201, &payload)
    });
    let session = client(address).create_session(&request).unwrap();
    server.join().unwrap();
    assert_eq!(session.state().session_id(), SESSION_ID);
    assert_eq!(session.state().workspace().root(), ROOT);
    assert!(session.selected());
}

#[test]
fn session_creation_rejects_workspace_attribution_mismatch() {
    let request = NyxSessionCreate::new("create-workspace-mismatch", "ForgeOS", ROOT);
    let expected = serde_json::to_value(&request).unwrap();
    let mut payload = session_view(SESSION_ID, true, false);
    payload["state"]["workspace"]["root"] = json!("/tmp/foreign-workspace");
    let (address, server) = spawn_fixture(1, move |_index, incoming| {
        assert_common(&incoming, "POST", "/v1/nyx/sessions");
        assert_eq!(incoming.body, Some(expected.clone()));
        response(201, &payload)
    });
    let error = client(address).create_session(&request).unwrap_err();
    server.join().unwrap();
    assert!(matches!(
        error,
        NyxConversationClientError::Protocol(NyxConversationProtocolError::ImmutableMismatch {
            field: "session request attribution"
        })
    ));
}

#[test]
fn session_list_rejects_selection_disagreement() {
    let payload = json!({
        "schema_version": "nyx.1.0",
        "schema_id": "nyx.session_list.v1",
        "ordering": "session_id_asc",
        "active_session_id": SESSION_ID,
        "sessions": [session_view(SESSION_ID, false, false)]
    });
    let (address, server) = spawn_fixture(1, move |_index, _request| response(200, &payload));
    let error = client(address).sessions().unwrap_err();
    server.join().unwrap();
    assert!(matches!(
        error,
        NyxConversationClientError::Protocol(NyxConversationProtocolError::ImmutableMismatch {
            field: "active session selection"
        })
    ));
}

#[test]
fn conversation_creation_preserves_session_and_title() {
    let request = NyxConversationCreate::new("conversation-create", "Conversation");
    let expected = serde_json::to_value(&request).unwrap();
    let payload = conversation_view(CONVERSATION_ID, SESSION_ID, false);
    let (address, server) = spawn_fixture(1, move |_index, incoming| {
        assert_common(
            &incoming,
            "POST",
            "/v1/nyx/sessions/sess_000101/conversations",
        );
        assert_eq!(incoming.body, Some(expected.clone()));
        response(201, &payload)
    });
    let conversation = client(address)
        .create_conversation(SESSION_ID, &request)
        .unwrap();
    server.join().unwrap();
    assert_eq!(conversation.thread().session_id(), SESSION_ID);
    assert_eq!(conversation.thread().thread_id(), CONVERSATION_ID);
    assert_eq!(conversation.thread().title(), "Conversation");
}

#[test]
fn conversation_list_is_bound_to_the_requested_session() {
    let payload = json!({
        "schema_version": "nyx.1.0",
        "schema_id": "nyx.conversation_list.v1",
        "session_id": SESSION_ID,
        "ordering": "conversation_id_asc",
        "conversations": [conversation_view(CONVERSATION_ID, SESSION_ID, false)]
    });
    let (address, server) = spawn_fixture(1, move |_index, request| {
        assert_common(
            &request,
            "GET",
            "/v1/nyx/sessions/sess_000101/conversations",
        );
        response(200, &payload)
    });
    let list = client(address).conversations(SESSION_ID).unwrap();
    server.join().unwrap();
    assert_eq!(list.session_id(), SESSION_ID);
    assert_eq!(
        list.conversations()[0].thread().thread_id(),
        CONVERSATION_ID
    );
}

#[test]
fn openai_stream_preserves_server_frames_and_done_terminator() {
    let request = NyxOpenAiChatRequest::new(
        "mock.echo.v1",
        [NyxOpenAiChatMessage::new(NyxOpenAiChatRole::User, "hello")],
    );
    let expected = serde_json::to_value(&request).unwrap();
    let body = openai_stream_body(None);
    let (address, server) = spawn_fixture(1, move |_index, incoming| {
        assert_common(&incoming, "POST", "/v1/chat/completions");
        assert_eq!(incoming.body, Some(expected.clone()));
        sse_response(200, Some("nyx.openai_chat_stream_event.v1"), &body)
    });
    let stream = client(address).stream_chat(&request).unwrap();
    server.join().unwrap();
    assert_eq!(stream.selected_protocol(), NyxProtocolVersion::new(1, 0));
    assert_eq!(stream.frames().len(), 3);
    assert_eq!(
        stream.frames()[0].event_kind(),
        NyxOpenAiStreamEventKind::ResponseCreated
    );
    assert_eq!(
        stream.frames()[2].event_kind(),
        NyxOpenAiStreamEventKind::ResponseCompleted
    );
    assert_eq!(stream.rendered_text(), "echo: hello");
}

#[test]
fn openai_stream_accepts_nyx_projection_without_contract_response_header() {
    let request = NyxOpenAiChatRequest::new(
        "mock.echo.v1",
        [NyxOpenAiChatMessage::new(NyxOpenAiChatRole::User, "hello")],
    );
    let body = openai_stream_body(None);
    let (address, server) = spawn_fixture(1, move |_index, incoming| {
        assert_common(&incoming, "POST", "/v1/chat/completions");
        sse_response_with_optional_contract(
            200,
            Some("nyx.openai_chat_stream_event.v1"),
            None,
            &body,
        )
    });
    let stream = client(address).stream_chat(&request).unwrap();
    server.join().unwrap();
    assert_eq!(stream.selected_protocol(), NyxProtocolVersion::new(1, 0));
    assert_eq!(stream.rendered_text(), "echo: hello");
}

#[test]
fn openai_stream_rejects_sequence_gaps() {
    let request = NyxOpenAiChatRequest::new(
        "mock.echo.v1",
        [NyxOpenAiChatMessage::new(NyxOpenAiChatRole::User, "hello")],
    );
    let body = openai_stream_body(Some(9));
    let (address, server) = spawn_fixture(1, move |_index, _incoming| {
        sse_response(200, Some("nyx.openai_chat_stream_event.v1"), &body)
    });
    let error = client(address).stream_chat(&request).unwrap_err();
    server.join().unwrap();
    assert!(matches!(
        error,
        NyxConversationClientError::Protocol(NyxConversationProtocolError::ImmutableMismatch {
            field: "OpenAI stream identity or ordering"
        })
    ));
}

#[test]
fn openai_stream_requires_nyx_schema_header() {
    let request = NyxOpenAiChatRequest::new(
        "mock.echo.v1",
        [NyxOpenAiChatMessage::new(NyxOpenAiChatRole::User, "hello")],
    );
    let body = openai_stream_body(None);
    let (address, server) =
        spawn_fixture(1, move |_index, _incoming| sse_response(200, None, &body));
    let error = client(address).stream_chat(&request).unwrap_err();
    server.join().unwrap();
    assert!(matches!(
        error,
        NyxConversationClientError::Protocol(NyxConversationProtocolError::UnsupportedSchema {
            context: "OpenAI stream header",
            ..
        })
    ));
}

#[test]
fn openai_stream_requires_done_terminator() {
    let request = NyxOpenAiChatRequest::new(
        "mock.echo.v1",
        [NyxOpenAiChatMessage::new(NyxOpenAiChatRole::User, "hello")],
    );
    let body = openai_stream_body(None).replace("data: [DONE]\n\n", "");
    let (address, server) = spawn_fixture(1, move |_index, _incoming| {
        sse_response(200, Some("nyx.openai_chat_stream_event.v1"), &body)
    });
    let error = client(address).stream_chat(&request).unwrap_err();
    server.join().unwrap();
    assert!(matches!(
        error,
        NyxConversationClientError::Protocol(NyxConversationProtocolError::NonCanonical {
            context: "OpenAI SSE completion"
        })
    ));
}

#[test]
fn openai_stream_rejects_model_attribution_mismatch() {
    let request = NyxOpenAiChatRequest::new(
        "mock.echo.v1",
        [NyxOpenAiChatMessage::new(NyxOpenAiChatRole::User, "hello")],
    );
    let body = openai_stream_body(None)
        .replace("\"model\":\"mock.echo.v1\"", "\"model\":\"foreign.model\"");
    let (address, server) = spawn_fixture(1, move |_index, _incoming| {
        sse_response(200, Some("nyx.openai_chat_stream_event.v1"), &body)
    });
    let error = client(address).stream_chat(&request).unwrap_err();
    server.join().unwrap();
    assert!(matches!(
        error,
        NyxConversationClientError::Protocol(NyxConversationProtocolError::ImmutableMismatch {
            field: "OpenAI stream identity or ordering"
        })
    ));
}

#[test]
fn native_message_response_preserves_model_and_ordered_terminal_events() {
    let request = NyxConversationMessageCreate::new("message-001", "mock.echo.v1", "hello");
    let expected = serde_json::to_value(&request).unwrap();
    let payload = conversation_response();
    let (address, server) = spawn_fixture(1, move |_index, incoming| {
        assert_common(
            &incoming,
            "POST",
            "/v1/nyx/sessions/sess_000101/conversations/thr_000201/messages",
        );
        assert_eq!(incoming.body, Some(expected.clone()));
        response(200, &payload)
    });
    let result = client(address)
        .send_message(SESSION_ID, CONVERSATION_ID, &request)
        .unwrap();
    server.join().unwrap();
    assert_eq!(result.model().model_id(), "mock.echo.v1");
    assert_eq!(result.assistant_message().content(), "echo: hello");
    assert_eq!(result.events().len(), 3);
    assert_eq!(
        result.events().last().unwrap().event_kind(),
        NyxConversationEventKind::ResponseCompleted
    );
}

#[test]
fn multiline_message_content_is_preserved() {
    let request = NyxConversationMessageCreate::new(
        "message-multiline",
        "mock.echo.v1",
        "first line\nsecond line",
    );
    let expected = serde_json::to_value(&request).unwrap();
    let mut payload = conversation_response();
    payload["user_message"]["content"] = json!("first line\nsecond line");
    let (address, server) = spawn_fixture(1, move |_index, incoming| {
        assert_eq!(incoming.body, Some(expected.clone()));
        response(200, &payload)
    });
    let result = client(address)
        .send_message(SESSION_ID, CONVERSATION_ID, &request)
        .unwrap();
    server.join().unwrap();
    assert_eq!(result.user_message().content(), "first line\nsecond line");
}

#[test]
fn native_message_response_requires_response_created_first() {
    let request = NyxConversationMessageCreate::new("message-created", "mock.echo.v1", "hello");
    let mut payload = conversation_response();
    payload["events"][0]["event_kind"] = json!("output_text_delta");
    let (address, server) = spawn_fixture(1, move |_index, _incoming| response(200, &payload));
    let error = client(address)
        .send_message(SESSION_ID, CONVERSATION_ID, &request)
        .unwrap_err();
    server.join().unwrap();
    assert!(matches!(
        error,
        NyxConversationClientError::Protocol(NyxConversationProtocolError::NonCanonical {
            context: "conversation terminal events"
        })
    ));
}

#[test]
fn native_message_response_rejects_reordered_events() {
    let request = NyxConversationMessageCreate::new("message-002", "mock.echo.v1", "hello");
    let mut payload = conversation_response();
    payload["events"][1]["sequence"] = json!(9);
    let (address, server) = spawn_fixture(1, move |_index, _incoming| response(200, &payload));
    let error = client(address)
        .send_message(SESSION_ID, CONVERSATION_ID, &request)
        .unwrap_err();
    server.join().unwrap();
    assert!(matches!(
        error,
        NyxConversationClientError::Protocol(NyxConversationProtocolError::ImmutableMismatch {
            field: "event identity or ordering"
        })
    ));
}

#[test]
fn missing_model_failure_remains_nyx_server_truth() {
    let request = NyxConversationMessageCreate::new("message-003", "missing.model", "hello");
    let error = json!({
        "error": {
            "code": "model_not_found",
            "message": "requested model is not available",
            "details": {"available_models": ["mock.echo.v1"]}
        }
    });
    let (address, server) = spawn_fixture(1, move |_index, _incoming| response(400, &error));
    let result = client(address)
        .send_message(SESSION_ID, CONVERSATION_ID, &request)
        .unwrap_err();
    server.join().unwrap();
    assert!(matches!(
        result,
        NyxConversationClientError::Rejected {
            status: 400,
            ref error,
            ..
        } if error.code() == "model_not_found"
    ));
}

#[test]
fn message_history_rejects_cross_session_identity() {
    let payload = json!({
        "schema_version": "nyx.1.0",
        "schema_id": "nyx.conversation_message_list.v1",
        "session_id": "sess_foreign",
        "conversation_id": CONVERSATION_ID,
        "ordering": "sequence_asc_then_message_id_asc",
        "messages": []
    });
    let (address, server) = spawn_fixture(1, move |_index, _incoming| response(200, &payload));
    let error = client(address)
        .messages(SESSION_ID, CONVERSATION_ID)
        .unwrap_err();
    server.join().unwrap();
    assert!(matches!(
        error,
        NyxConversationClientError::Protocol(NyxConversationProtocolError::ImmutableMismatch {
            field: "message list identity"
        })
    ));
}

#[test]
fn close_and_restore_controls_use_exact_server_owned_identity() {
    let closed = session_view(SESSION_ID, false, true);
    let restored = session_view(SESSION_ID, true, false);
    let (address, server) = spawn_fixture(2, move |index, request| {
        let suffix = if index == 0 { "close" } else { "restore" };
        assert_common(
            &request,
            "POST",
            &format!("/v1/nyx/sessions/{SESSION_ID}/{suffix}"),
        );
        response(200, if index == 0 { &closed } else { &restored })
    });
    let client = client(address);
    let closed = client
        .close_session(SESSION_ID, &NyxSessionControl::new("close-001"))
        .unwrap();
    assert!(closed.is_closed());
    let restored = client
        .restore_session(SESSION_ID, &NyxSessionControl::new("restore-001"))
        .unwrap();
    server.join().unwrap();
    assert!(!restored.is_closed());
    assert!(restored.selected());
}

#[test]
fn conversation_control_and_path_validation_fail_closed() {
    let config = NyxClientConfig::new(
        NyxTransportEndpoint::tcp("127.0.0.1:9".parse().unwrap()),
        [NyxProtocolVersion::new(1, 0)],
    )
    .unwrap();
    let client = NyxConversationClient::new(config);
    let error = client
        .select_conversation("bad/id", &NyxConversationControl::new("select-001"))
        .unwrap_err();
    assert!(matches!(
        error,
        NyxConversationClientError::Protocol(NyxConversationProtocolError::InvalidField {
            context: "conversation path",
            field: "conversation_id",
            ..
        })
    ));
}

#[test]
fn bearer_token_rejects_header_injection() {
    let error = NyxClientConfig::new(
        NyxTransportEndpoint::tcp("127.0.0.1:9".parse().unwrap()),
        [NyxProtocolVersion::new(1, 0)],
    )
    .unwrap()
    .with_bearer_token("token\r\nX-Evil: yes")
    .unwrap_err();
    assert!(format!("{error}").contains("bearer_token"));
}

#[test]
#[ignore = "requires a separately running real Nyx_Server local conversation gate"]
fn real_nyx_conversation_gate_proves_models_streaming_session_message_and_restore() {
    let address: SocketAddr = std::env::var("FORGE_NYX_ADDR")
        .expect("FORGE_NYX_ADDR")
        .parse()
        .expect("valid FORGE_NYX_ADDR");
    let token = std::env::var("FORGE_NYX_TOKEN").expect("FORGE_NYX_TOKEN");
    let workspace = std::env::var("FORGE_NYX_WORKSPACE_ROOT").expect("FORGE_NYX_WORKSPACE_ROOT");
    let model_id =
        std::env::var("FORGE_NYX_MODEL_ID").unwrap_or_else(|_| "mock.echo.v1".to_owned());
    let config = NyxClientConfig::new(
        NyxTransportEndpoint::tcp(address),
        [NyxProtocolVersion::new(1, 0)],
    )
    .unwrap()
    .with_bearer_token(token)
    .unwrap();
    let client = NyxConversationClient::new(config);

    let catalog = client.models().unwrap();
    let model = catalog.find(&model_id).expect("configured model");
    assert_eq!(model.readiness(), NyxModelReadiness::Ready);

    let session = client
        .create_session(&NyxSessionCreate::new(
            "forgeos-nyx200-real-session",
            "ForgeOS NYX-200",
            workspace,
        ))
        .unwrap();
    let session_id = session.state().session_id().to_owned();
    let conversations = client.conversations(&session_id).unwrap();
    let conversation_id = conversations.conversations()[0]
        .thread()
        .thread_id()
        .to_owned();
    let stream = client
        .stream_chat(&NyxOpenAiChatRequest::new(
            model_id.clone(),
            [NyxOpenAiChatMessage::new(
                NyxOpenAiChatRole::User,
                "ForgeOS consumed the Nyx-owned OpenAI stream contract.",
            )],
        ))
        .unwrap();
    println!("FORGE_NYX_CONVERSATION_STREAM={stream:#?}");
    assert_eq!(stream.frames()[0].sequence(), 0);
    assert_eq!(
        stream.frames().last().unwrap().event_kind(),
        NyxOpenAiStreamEventKind::ResponseCompleted
    );

    let response = client
        .send_message(
            &session_id,
            &conversation_id,
            &NyxConversationMessageCreate::new(
                "forgeos-nyx200-real-message",
                model_id,
                "ForgeOS consumed the Nyx-owned conversation contract.",
            ),
        )
        .unwrap();
    println!("FORGE_NYX_CONVERSATION_RESPONSE={response:#?}");
    assert_eq!(response.events()[0].sequence(), 0);
    assert!(response.events().last().unwrap().event_kind().is_terminal());
    let history = client.messages(&session_id, &conversation_id).unwrap();
    assert!(history.messages().len() >= 2);

    let closed = client
        .close_session(
            &session_id,
            &NyxSessionControl::new("forgeos-nyx200-real-close"),
        )
        .unwrap();
    assert!(closed.is_closed());
    let restored = client
        .restore_session(
            &session_id,
            &NyxSessionControl::new("forgeos-nyx200-real-restore"),
        )
        .unwrap();
    println!("FORGE_NYX_CONVERSATION_RESTORED={restored:#?}");
    assert!(!restored.is_closed());
    assert_eq!(restored.state().session_id(), session_id);
}
