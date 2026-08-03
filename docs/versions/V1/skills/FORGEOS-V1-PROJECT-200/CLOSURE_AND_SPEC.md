# FORGEOS-V1-PROJECT-200 Closure and Specification

Status: `CLOSED`
Capability: Persistent project registry and workspace restoration
Closed: `2026-08-02`
Source authority: `Forge_OS_V1_base_32.tar`
Source archive SHA-256: `c3888552959f9c4a54e2e9cb78540d5c0da1f24e310d59f4d8557c4ef85e180c`
Git revision: unavailable because the supplied source archive contains no `.git` metadata

## Capability statement

ForgeOS now persists multiple validated projects in one deterministic, versioned,
atomically published registry. Each record retains stable project and repository
identity, the exact validated manifest and registered-command definitions, the real
repository directory object, deterministic recent-open state, and one versioned safe
workspace payload. Reopening revalidates repository identity, same-object relocation
is explicit, corrupt state fails closed, and registry removal never removes or edits
repository source.

## Public contract

```text
forge_core::command_codec::decode_registered_command
forge_core::project_registry::ProjectRegistryState
forge_core::project_registry::PersistentProjectEntry
forge_core::project_registry::PersistedCommandDefinition
forge_core::project_registry::PersistedRepositoryObject
forge_core::project_registry::RecentOpenState
forge_core::project_registry::SafeWorkspaceSnapshot
forge_project::registry_store::PersistentProjectRegistry
forge_project::registry_store::RestoredProject
forge_project::registry_store::PersistentProjectRegistryError
```

## Accepted operator evidence

The operator ran the canonical behavior-only CI entrypoint and returned:

```text
FORGE_SEAM_DIRECTION_SUMMARY status=PASS packages=12 routes=42 forbidden=0 policy=exact-reviewed-subsystem-reachability-v1
FORGE_CORE_PURITY_SUMMARY status=PASS packages=2 allowed=2 forbidden=0 policy=exact-reviewed-production-graph-v1
FORGE_SOURCE_SIZE_SUMMARY status=PASS modules=106 pass=106 warn=0 fail=0 warnings_denied=true
CARGO_TEST_SUMMARY status=PASS suites=57 passed=266 failed=0 ignored=0 measured=0 filtered_out=0
CI RESULT: PASS
```

This satisfies the declared user-acceptance path for the active capability. CI remains
limited to behavioral tests, golden locks, and structural guards.

## Proved behavior

- canonical registry bytes round-trip and remain deterministic;
- multiple projects retain unique project and repository identity;
- exact manifest and registered-command bytes survive reopen;
- recent-open order uses registry-local sequence values rather than wall-clock time;
- one safe workspace payload survives restart with its content identity;
- unrelated project records remain equivalent after a selected-project mutation;
- duplicate registration and invalid inputs publish no partial state;
- the same repository directory object may be relocated explicitly;
- copied, replaced, or mismatched repository roots fail reopening;
- corrupt current state fails closed rather than creating an empty replacement;
- interrupted staging remains visible until explicitly discarded;
- removing a project removes only registry state and preserves repository source.

## Regression locks

```text
crates/forge-core/tests/project_registry_state.rs
crates/forge-project/tests/project_registry_persistence.rs
python3 scripts/run_ci.py
```

## Explicit non-claims

This closure does not provide repository file browsing or search, editor save,
managed terminals, project-command execution, Git presentation, process recovery,
Nyx behavior, or Forge World workspace presentation. Those remain separate skills.
