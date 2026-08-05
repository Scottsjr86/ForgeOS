//! Nyx-compatible canonical JSON and raw SHA-256 verification.
//!
//! Nyx owns the canonical permission identities. ForgeOS independently recomputes
//! the published hashes so a transport response cannot silently widen or replace
//! the reviewed request. This module does not mint Nyx identities or tokens.

use forge_protocol::hashes::hash_external_contract_bytes;
use serde::Serialize;
use serde_json::{Map, Value};

pub(crate) fn canonical_value(value: &Value) -> Value {
    match value {
        Value::Object(object) => {
            let mut keys = object.keys().collect::<Vec<_>>();
            keys.sort_unstable();
            let mut canonical = Map::new();
            for key in keys {
                canonical.insert(key.clone(), canonical_value(&object[key]));
            }
            Value::Object(canonical)
        }
        Value::Array(items) => Value::Array(items.iter().map(canonical_value).collect()),
        _ => value.clone(),
    }
}

pub(crate) fn canonical_json<T: Serialize>(value: &T) -> Result<Vec<u8>, serde_json::Error> {
    let value = serde_json::to_value(value)?;
    serde_json::to_vec(&canonical_value(&value))
}

pub(crate) fn raw_sha256_json<T: Serialize>(value: &T) -> Result<String, serde_json::Error> {
    canonical_json(value).map(|bytes| raw_sha256_hex(&bytes))
}

pub(crate) fn raw_sha256_hex(bytes: &[u8]) -> String {
    hash_external_contract_bytes(bytes).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn canonical_json_ignores_object_construction_order() {
        let first = json!({"z": 1, "a": {"y": 2, "b": 3}});
        let second = json!({"a": {"b": 3, "y": 2}, "z": 1});
        assert_eq!(
            raw_sha256_json(&first).unwrap(),
            raw_sha256_json(&second).unwrap()
        );
    }
}
