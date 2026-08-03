# FORGEOS-V1-PATCH-100 Closure and Specification

Status: `CLOSED`
Capability: Stable patch identity, validation, and all-or-nothing application
Closed: `2026-08-02`
Source authority: `Forge_OS_V1_base_31.tar`
Source archive SHA-256: `6507a74c14c041a9f151e5e3366275324003d52b2ada0cff17a6382883157ee3`
Git revision: unavailable because the supplied source archive contains no `.git` metadata

## Capability statement

ForgeOS now accepts one exact text patch envelope, validates stable payload and
structured identities, verifies repository and base revision, rejects hostile or
undeclared patch content, checks every current-file precondition, applies through a
fixed native Git path, verifies every declared result hash, and restores all touched
files after apply or verification failure.

## Public contract

```text
forge_protocol::patches::PatchBaseRevision
forge_protocol::patches::PatchFileAction
forge_protocol::patches::PatchFileRecord
forge_protocol::patches::PatchEnvelope
forge_nyx_client::patches::NyxPatchOffer
forge_bridge::patch::NativePatchAdapter
forge_bridge::patch::NativePatchOperation
forge_git::patches::GitPatchApplier
forge_git::patches::PatchValidationResult
forge_git::patches::PatchApplyOutcome
forge_git::patches::PatchApplyError
```

## Accepted operator evidence

```text
FORGE_SEAM_DIRECTION_SUMMARY status=PASS packages=12 routes=42 forbidden=0 policy=exact-reviewed-subsystem-reachability-v1
FORGE_CORE_PURITY_SUMMARY status=PASS packages=2 allowed=2 forbidden=0 policy=exact-reviewed-production-graph-v1
FORGE_SOURCE_SIZE_SUMMARY status=PASS modules=101 pass=101 warn=0 fail=0 warnings_denied=true
CARGO_TEST_SUMMARY status=PASS suites=55 passed=246 failed=0 ignored=0 measured=0 filtered_out=0
CI RESULT: PASS
```

CI remains limited to behavioral tests, golden locks, and structural guards. It
contains no documentation verification, Git-state check, or formatting gate.

## Negative and failure-path results

Fixtures cover transport tampering, repository and base mismatch, declared-table
mismatch, hidden traditional secondary diffs, traversal, symlinks, binary data,
mode and rename metadata, partial applicability, stale before-state, after-hash
failure, rollback, concurrent application, and missing Git. Rejected or failed
application preserves original and unrelated worktree state.

## Explicit non-claims

This closure does not complete integrated Git workflow, agent dispatch, returned
patch review, verification records, project restoration, Nyx model behavior, or
Forge World presentation.
