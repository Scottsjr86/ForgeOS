//! Forge Core hashing of canonical state and result payloads.
//!
//! Core supplies canonical bytes. The protocol contract supplies stable,
//! domain-separated SHA-256 identity without adding effects or host metadata.

use crate::state::StateRecord;
use forge_protocol::hashes::{ContentHash, HashDomain, hash_canonical_bytes};

/// Stable snapshot identity over the exact current-schema state record bytes.
pub fn state_record_hash(record: &StateRecord) -> ContentHash {
    hash_canonical_bytes(HashDomain::Snapshot, &record.encode())
}

/// Stable result identity over already canonical result payload bytes.
pub fn result_payload_hash(payload: &[u8]) -> ContentHash {
    hash_canonical_bytes(HashDomain::ResultPayload, payload)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn equivalent_state_has_equivalent_hash() {
        let first = StateRecord::new(7, b"same state".to_vec()).unwrap();
        let reopened = StateRecord::decode(&first.encode()).unwrap();
        assert_eq!(state_record_hash(&first), state_record_hash(&reopened));
    }

    #[test]
    fn changed_state_and_result_payload_change_identity() {
        let first = StateRecord::new(7, b"state one".to_vec()).unwrap();
        let second = StateRecord::new(7, b"state two".to_vec()).unwrap();
        assert_ne!(state_record_hash(&first), state_record_hash(&second));
        assert_ne!(result_payload_hash(b"pass"), result_payload_hash(b"fail"));
    }

    #[test]
    fn snapshot_and_result_domains_do_not_alias() {
        let bytes = b"same canonical bytes";
        let record = StateRecord::new(7, bytes.to_vec()).unwrap();
        assert_ne!(
            state_record_hash(&record),
            result_payload_hash(&record.encode())
        );
    }
}
