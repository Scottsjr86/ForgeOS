//! Stable event records carried by the shared protocol.

use crate::identities::EventId;

/// One typed event identity paired with opaque event payload bytes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EventRecord {
    event_id: EventId,
    payload: Vec<u8>,
}

impl EventRecord {
    /// Creates an event record without deriving identity from its payload or display text.
    pub fn new(event_id: EventId, payload: impl Into<Vec<u8>>) -> Self {
        Self {
            event_id,
            payload: payload.into(),
        }
    }

    /// Stable event identity.
    pub const fn event_id(&self) -> EventId {
        self.event_id
    }

    /// Opaque event payload bytes.
    pub fn payload(&self) -> &[u8] {
        &self.payload
    }
}
