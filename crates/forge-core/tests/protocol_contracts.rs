use forge_protocol::envelopes::{CURRENT_PROTOCOL_VERSION, RequestEnvelope};
use forge_protocol::identities::{CommandId, IDENTITY_BYTES, TaskId};

#[test]
fn forge_core_consumes_the_typed_v1_protocol_contract() {
    let task_id = TaskId::from_bytes([0x10; IDENTITY_BYTES]);
    let command_id = CommandId::from_bytes([0x20; IDENTITY_BYTES]);
    let request = RequestEnvelope::new(task_id, command_id, b"core-contract".to_vec())
        .expect("fixture payload should fit the V1 envelope");

    assert_eq!(request.version(), CURRENT_PROTOCOL_VERSION);
    assert_eq!(request.task_id(), task_id);
    assert_eq!(request.command_id(), command_id);
    assert_eq!(
        RequestEnvelope::decode(&request.encode()),
        Ok(request),
        "Forge Core must consume the same published protocol bytes it emits"
    );
}
