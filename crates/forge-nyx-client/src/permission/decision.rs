//! Nyx-owned permission decisions and immutable resume responses.

use super::support::{
    NyxPermissionProtocolError, ensure_schema, validate_hash, validate_route_id, validate_text,
    verify_hash, verify_immutable_checkpoint,
};
use super::{
    NyxPermissionCheckpoint, NyxPermissionCheckpointStatus, NyxScopedToolRequest, SCHEMA_DECISION,
    SCHEMA_RESOLVE, SCHEMA_RESUME, SCHEMA_RESUME_RESULT, SCHEMA_VERSION_V1,
};
use crate::canonical_json::raw_sha256_hex;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NyxPermissionDecisionKind {
    Approve,
    Deny,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NyxPermissionDecision {
    schema_version: String,
    schema_id: String,
    decision: NyxPermissionDecisionKind,
    decided_at_unix_ms: u64,
    decided_by: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    reason: Option<String>,
    #[serde(default)]
    conditions: Vec<String>,
    expected_request_sha256: String,
    expected_scope_sha256: String,
    expected_policy_decision_sha256: String,
}

impl NyxPermissionDecision {
    pub fn for_checkpoint(
        checkpoint: &NyxPermissionCheckpoint,
        decision: NyxPermissionDecisionKind,
        reason: Option<String>,
    ) -> Result<Self, NyxPermissionProtocolError> {
        checkpoint.validate()?;
        if checkpoint.status != NyxPermissionCheckpointStatus::Pending {
            return Err(NyxPermissionProtocolError::InvalidField {
                context: "permission decision",
                field: "checkpoint.status",
                detail: "must be pending".to_owned(),
            });
        }
        Ok(Self {
            schema_version: SCHEMA_VERSION_V1.to_owned(),
            schema_id: SCHEMA_DECISION.to_owned(),
            decision,
            decided_at_unix_ms: 0,
            decided_by: "forgeos-client-intent".to_owned(),
            reason,
            conditions: vec!["exact_scope_only".to_owned(), "single_execution".to_owned()],
            expected_request_sha256: checkpoint.request_sha256.clone(),
            expected_scope_sha256: checkpoint.scope_sha256.clone(),
            expected_policy_decision_sha256: checkpoint.policy_decision_sha256.clone(),
        })
    }

    pub const fn decision(&self) -> NyxPermissionDecisionKind {
        self.decision
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NyxPermissionResolution {
    schema_version: String,
    schema_id: String,
    checkpoint: NyxPermissionCheckpoint,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    resume_token: Option<String>,
}

impl NyxPermissionResolution {
    pub fn checkpoint(&self) -> &NyxPermissionCheckpoint {
        &self.checkpoint
    }

    pub fn resume_token(&self) -> Option<&str> {
        self.resume_token.as_deref()
    }

    pub fn validate_against(
        &self,
        before: &NyxPermissionCheckpoint,
        decision: NyxPermissionDecisionKind,
    ) -> Result<(), NyxPermissionProtocolError> {
        ensure_schema(
            "permission resolution",
            &self.schema_version,
            &self.schema_id,
            SCHEMA_RESOLVE,
        )?;
        before.validate()?;
        self.checkpoint.validate()?;
        verify_immutable_checkpoint(before, &self.checkpoint)?;
        match decision {
            NyxPermissionDecisionKind::Approve => {
                if self.checkpoint.status != NyxPermissionCheckpointStatus::Approved {
                    return Err(NyxPermissionProtocolError::UnexpectedStatus {
                        expected: NyxPermissionCheckpointStatus::Approved,
                        found: self.checkpoint.status,
                    });
                }
                let token = self.resume_token.as_deref().ok_or(
                    NyxPermissionProtocolError::MissingField {
                        context: "permission resolution",
                        field: "resume_token",
                    },
                )?;
                let declared = self.checkpoint.resume_token_sha256.as_deref().ok_or(
                    NyxPermissionProtocolError::MissingField {
                        context: "permission checkpoint",
                        field: "resume_token_sha256",
                    },
                )?;
                verify_hash(
                    "resume_token_sha256",
                    &raw_sha256_hex(token.as_bytes()),
                    declared,
                )
            }
            NyxPermissionDecisionKind::Deny => {
                if self.checkpoint.status != NyxPermissionCheckpointStatus::Denied {
                    return Err(NyxPermissionProtocolError::UnexpectedStatus {
                        expected: NyxPermissionCheckpointStatus::Denied,
                        found: self.checkpoint.status,
                    });
                }
                if self.resume_token.is_some() {
                    return Err(NyxPermissionProtocolError::ImmutableMismatch {
                        field: "resume_token",
                    });
                }
                Ok(())
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NyxPermissionResume {
    schema_version: String,
    schema_id: String,
    checkpoint_id: String,
    resume_token: String,
    request: NyxScopedToolRequest,
}

impl NyxPermissionResume {
    pub fn new(resolution: &NyxPermissionResolution) -> Result<Self, NyxPermissionProtocolError> {
        let checkpoint = resolution.checkpoint();
        checkpoint.validate()?;
        if checkpoint.status != NyxPermissionCheckpointStatus::Approved {
            return Err(NyxPermissionProtocolError::UnexpectedStatus {
                expected: NyxPermissionCheckpointStatus::Approved,
                found: checkpoint.status,
            });
        }
        let token = resolution
            .resume_token()
            .ok_or(NyxPermissionProtocolError::MissingField {
                context: "permission resolution",
                field: "resume_token",
            })?;
        let declared =
            checkpoint
                .resume_token_sha256()
                .ok_or(NyxPermissionProtocolError::MissingField {
                    context: "permission checkpoint",
                    field: "resume_token_sha256",
                })?;
        verify_hash(
            "resume_token_sha256",
            &raw_sha256_hex(token.as_bytes()),
            declared,
        )?;
        Ok(Self {
            schema_version: SCHEMA_VERSION_V1.to_owned(),
            schema_id: SCHEMA_RESUME.to_owned(),
            checkpoint_id: checkpoint.checkpoint_id.clone(),
            resume_token: token.to_owned(),
            request: checkpoint.request.clone(),
        })
    }

    pub fn from_exact_parts(
        checkpoint_id: impl Into<String>,
        resume_token: impl Into<String>,
        request: NyxScopedToolRequest,
    ) -> Result<Self, NyxPermissionProtocolError> {
        let resume = Self {
            schema_version: SCHEMA_VERSION_V1.to_owned(),
            schema_id: SCHEMA_RESUME.to_owned(),
            checkpoint_id: checkpoint_id.into(),
            resume_token: resume_token.into(),
            request,
        };
        resume.validate()?;
        Ok(resume)
    }

    pub fn request(&self) -> &NyxScopedToolRequest {
        &self.request
    }

    pub fn checkpoint_id(&self) -> &str {
        &self.checkpoint_id
    }

    fn validate(&self) -> Result<(), NyxPermissionProtocolError> {
        ensure_schema(
            "permission resume",
            &self.schema_version,
            &self.schema_id,
            SCHEMA_RESUME,
        )?;
        validate_route_id("checkpoint_id", &self.checkpoint_id)?;
        validate_text("permission resume", "resume_token", &self.resume_token)?;
        self.request.validate()
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NyxPermissionResumeResult {
    schema_version: String,
    schema_id: String,
    checkpoint_id: String,
    execution_id: String,
    request_sha256: String,
    status: NyxPermissionCheckpointStatus,
    tool_result: Value,
    audit_sequence: u64,
}

impl NyxPermissionResumeResult {
    pub fn checkpoint_id(&self) -> &str {
        &self.checkpoint_id
    }

    pub fn execution_id(&self) -> &str {
        &self.execution_id
    }

    pub fn request_sha256(&self) -> &str {
        &self.request_sha256
    }

    pub const fn status(&self) -> NyxPermissionCheckpointStatus {
        self.status
    }

    pub fn tool_result(&self) -> &Value {
        &self.tool_result
    }

    pub const fn audit_sequence(&self) -> u64 {
        self.audit_sequence
    }

    pub fn validate_against(
        &self,
        resume: &NyxPermissionResume,
    ) -> Result<(), NyxPermissionProtocolError> {
        ensure_schema(
            "permission resume result",
            &self.schema_version,
            &self.schema_id,
            SCHEMA_RESUME_RESULT,
        )?;
        resume.validate()?;
        validate_route_id("checkpoint_id", &self.checkpoint_id)?;
        validate_text(
            "permission resume result",
            "execution_id",
            &self.execution_id,
        )?;
        validate_hash(
            "permission resume result",
            "request_sha256",
            &self.request_sha256,
        )?;
        if self.status != NyxPermissionCheckpointStatus::Consumed {
            return Err(NyxPermissionProtocolError::UnexpectedStatus {
                expected: NyxPermissionCheckpointStatus::Consumed,
                found: self.status,
            });
        }
        if self.checkpoint_id != resume.checkpoint_id {
            return Err(NyxPermissionProtocolError::ImmutableMismatch {
                field: "checkpoint_id",
            });
        }
        verify_hash(
            "request_sha256",
            &resume.request.request_sha256()?,
            &self.request_sha256,
        )
    }
}
