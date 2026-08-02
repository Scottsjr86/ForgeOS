# FORGEOS-V1-SESSION-100 Closure and Specification

Status: `CLOSED`
Capability: Deterministic session and managed-service lifecycle contract
Closed: `2026-08-02`
Source authority: `Forge_OS_V1_base_27.tar`
Source archive SHA-256: `37356650de88d9244c413cc05e8657faebcadddf2c0821b6e2494e4390b3f6eb`
Git revision: unavailable because the supplied source archive contains no `.git` metadata

## Capability statement

ForgeOS now owns immutable managed-service definitions with canonical lower-kebab
names, unique explicit startup ranks, validated dependency order, deterministic
startup and reverse shutdown order, bounded pre-readiness restart policy, stable
session and process identity checks, explicit readiness, rollback, stop-failure
preservation, and terminal lifecycle outcomes without sleeps or process-name
discovery.

## Public contract

```text
forge_session::services::ServiceName
forge_session::services::ManagedService
forge_session::services::ServicePlan
forge_session::services::StartupRestartPolicy
forge_session::lifecycle::SessionSupervisor
forge_session::lifecycle::SessionPhase
forge_session::lifecycle::LifecycleAction
forge_session::lifecycle::ServiceStatus
forge_session::lifecycle::ServiceFailure
forge_session::lifecycle::StopReason
forge_session::lifecycle::SupervisorError
```

## Accepted operator evidence

```text
FORGE_SEAM_DIRECTION_SUMMARY status=PASS packages=12 routes=42 forbidden=0 policy=exact-reviewed-subsystem-reachability-v1
FORGE_CORE_PURITY_SUMMARY status=PASS packages=2 allowed=2 forbidden=0 policy=exact-reviewed-production-graph-v1
FORGE_SOURCE_SIZE_SUMMARY status=PASS modules=87 pass=87 warn=0 fail=0 warnings_denied=true
CARGO_TEST_SUMMARY status=PASS suites=52 passed=190 failed=0 ignored=0 measured=0 filtered_out=0
CI RESULT: PASS
```

CI remains limited to behavioral tests, golden locks, and structural guards. It
contains no documentation verification, Git-state check, or formatting gate.

## Negative and failure-path results

Fixtures cover malformed service names, duplicate names and startup ranks, unknown
or later dependencies, wrong process and attempt identity, failed readiness cleanup,
retry exhaustion, rollback, runtime exit, stop failure, invalid transitions, and
independent supervisor isolation.

## Explicit non-claims

This closure does not install a display-manager session, bootstrap the ForgeOS login
session, spawn managed services, persist recovery state, connect Nyx, infer health
from process presence, package a distribution, or render Forge World presentation.
