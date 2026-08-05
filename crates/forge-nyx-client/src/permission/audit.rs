//! Nyx-owned permission audit records decoded by the Forge client.

use super::support::{
    ensure_schema, validate_hash, validate_route_id, validate_text, NyxPermissionProtocolError,
};
use super::{NyxPermissionCheckpointStatus, SCHEMA_AUDIT_EVENT, SCHEMA_AUDIT_REPORT};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NyxPermissionAuditEventKind {
    CheckpointCreated,
    CheckpointReplayed,
    Approved,
    Denied,
    Expired,
    ResumeRejected,
    ExecutionReserved,
    ExecutionCompleted,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NyxPermissionAuditEvent {
    schema_version: String,
    schema_id: String,
    sequence: u64,
    event: NyxPermissionAuditEventKind,
    checkpoint_id: String,
    request_id: String,
    request_sha256: String,
    status: NyxPermissionCheckpointStatus,
    occurred_at_unix_ms: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    actor_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    reason: Option<String>,
}

impl NyxPermissionAuditEvent {
    pub const fn sequence(&self) -> u64 {
        self.sequence
    }

    pub const fn event(&self) -> NyxPermissionAuditEventKind {
        self.event
    }

    pub fn checkpoint_id(&self) -> &str {
        &self.checkpoint_id
    }

    pub fn request_sha256(&self) -> &str {
        &self.request_sha256
    }

    fn validate(&self) -> Result<(), NyxPermissionProtocolError> {
        ensure_schema(
            "permission audit event",
            &self.schema_version,
            &self.schema_id,
            SCHEMA_AUDIT_EVENT,
        )?;
        validate_route_id("checkpoint_id", &self.checkpoint_id)?;
        validate_text("permission audit event", "request_id", &self.request_id)?;
        validate_hash(
            "permission audit event",
            "request_sha256",
            &self.request_sha256,
        )?;
        if self.sequence == 0 {
            return Err(NyxPermissionProtocolError::InvalidField {
                context: "permission audit event",
                field: "sequence",
                detail: "must be greater than zero".to_owned(),
            });
        }
        if let Some(actor_id) = &self.actor_id {
            validate_text("permission audit event", "actor_id", actor_id)?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NyxPermissionAuditReport {
    schema_version: String,
    schema_id: String,
    events: Vec<NyxPermissionAuditEvent>,
}

impl NyxPermissionAuditReport {
    pub fn events(&self) -> &[NyxPermissionAuditEvent] {
        &self.events
    }

    pub fn validate(&self) -> Result<(), NyxPermissionProtocolError> {
        ensure_schema(
            "permission audit report",
            &self.schema_version,
            &self.schema_id,
            SCHEMA_AUDIT_REPORT,
        )?;
        let mut previous = None;
        for event in &self.events {
            event.validate()?;
            if previous.is_some_and(|value| event.sequence <= value) {
                return Err(NyxPermissionProtocolError::NonCanonical {
                    context: "permission audit sequence",
                });
            }
            previous = Some(event.sequence);
        }
        Ok(())
    }
}
