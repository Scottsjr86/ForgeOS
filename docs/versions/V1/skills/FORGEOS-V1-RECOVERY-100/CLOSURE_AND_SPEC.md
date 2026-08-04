# FORGEOS-V1-RECOVERY-100 Closure and Specification

Status: `ACTIVE_OPERATOR_VALIDATION_PENDING`
Capability: Workspace snapshot and crash-journal primitive
Active slice: `FORGEOS-V1-RECOVERY-100-SLICE-001`
Source authority: `Forge_OS_V1_base_39.tar`
Source archive SHA-256: `5abb16e390a109d1f35b1add7e4896ce8b9b340364ac51286ee49a58660b072e`
Git revision: unavailable because the supplied source archive contains no `.git` metadata

## Capability statement

ForgeOS now has a canonical recovery image containing one versioned safe workspace
snapshot, non-replayable interrupted-action evidence, and historical process records
that require revalidation after restart. The image uses the existing checksummed
Forge Core state envelope and stable SHA-256 identity. Filesystem publication reuses
the existing atomic state store and requires exact sequential generations.

## Public contract

```text
forge_core::recovery::WorkspaceRecoveryRecord
forge_core::recovery::InterruptedAction
forge_core::recovery::InterruptedEffectState
forge_core::recovery::RecordedProcess
forge_core::recovery::RecoveredProcessState
forge_session::recovery::SessionRecoveryEvidence
forge_session::lifecycle::SessionSupervisor::recovery_evidence
forge_project::recovery_store::WorkspaceRecoveryStore
forge_project::recovery_store::RecoveryAssessment
forge_project::recovery_store::RecoveryImageStatus
forge_project::recovery_store::RecoveryChoice
```

## Intended behavior

- the safe workspace payload remains versioned and content-addressed;
- the complete recovery record is checksummed and domain-separated by SHA-256;
- interrupted actions carry exact request identities and never become replayable;
- a process observed active before failure reopens only as `RequiresRevalidation`;
- confirmed stopped services retain no stale process identity;
- recovery images publish atomically through exact sequential generations;
- abandoned staging is visible and can be explicitly discarded only with valid current data;
- corrupt or missing current data may explicitly restore a valid previous image;
- a valid current image blocks replacement by an older previous image;
- assessment does not mutate or silently choose recovery data.

## Regression locks

```text
crates/forge-core/tests/workspace_recovery.rs
crates/forge-project/tests/workspace_recovery_store.rs
crates/forge-session/tests/recovery_evidence.rs
python3 scripts/run_ci.py
```

## Operator validation still required

Run the canonical behavior-only CI entrypoint:

```bash
python3 scripts/run_ci.py
```

The skill remains active until the operator returns green structural guards and
Rust tests for this slice.

## Explicit non-claims

This skill does not restore editor buffers into a running shell, restart services,
recover terminal processes, reconnect Nyx conversations, replay commands, or provide
the final crash-recovery UI. Those integrated behaviors belong to later recovery,
session, terminal, Nyx, and Forge World skills.
