//! ForgeOS client boundary for the separate nyx_server model host.
//!
//! Nyx_Server owns AI behavior, policy, checkpoints, and durable server state.
//! ForgeOS owns only transport, presentation, project envelopes, and independent
//! verification of the Nyx-owned contracts it consumes.

mod canonical_json;

pub mod conversation;
pub mod conversation_client;
pub mod patches;
pub mod permission;
pub mod permission_client;
pub mod protocol;
pub mod remote_agent;
pub mod remote_agent_client;
pub mod transport;
