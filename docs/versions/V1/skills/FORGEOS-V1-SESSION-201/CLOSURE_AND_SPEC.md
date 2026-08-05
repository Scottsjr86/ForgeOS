# FORGEOS-V1-SESSION-201 Closure and Specification

Status: `CLOSED`
Capability: Managed ForgeOS and Nyx service lifecycle
Active slice: `FORGEOS-V1-SESSION-201-SLICE-001`
Source authority: `Forge_OS_V1_base_51.tar`

## Capability statement

ForgeOS now supervises the separate Nyx server as one externally owned managed
service. Native process mechanics remain in `forge-bridge`, canonical lifecycle and
bounded restart state remain in `forge-session`, and readiness comes only from the
Nyx-owned public health contract consumed by `forge-nyx-client`.

## Public contract

```text
forge_bridge::service_process::ManagedServiceProcess
forge_session::service_runtime::ManagedServiceRuntime
forge_session::service_runtime::ManagedServiceRuntimeState
forge_app::composition::nyx_service::ManagedNyxService
forge_app::composition::nyx_service::NyxServiceConfig
```

## Intended behavior

- one exact shell-free Nyx executable and argv are launched in a declared context;
- ForgeOS proves the configured endpoint is not already serving before spawn;
- duplicate managed or externally running Nyx instances are rejected;
- process presence alone never becomes readiness;
- Nyx `live` plus `control_plane_ready` establishes service readiness;
- degraded model/provider readiness remains truthfully degraded while the control
  plane can still support session management;
- incompatible or malformed public contracts fail readiness;
- unexpected native exit consumes only the declared bounded restart budget;
- every restart requires a new stable ForgeOS process identity;
- logout stops the exact owned process group and records `Stopped`;
- Nyx failure remains isolated from editor, terminal, project, and Git state.

## Nyx cross-repository gate

`API-FOUND-008` is already banked with `PROOF_SYSTEM` in the Nyx public API gate.
The exact reused receipt and Nyx source identity are recorded in `NYX_GATE_INPUT.json`.
No Nyx source change is required by this ForgeOS slice.

## Regression locks

```text
crates/forge-session/tests/service_runtime.rs
crates/forge-app/tests/nyx_service_lifecycle.rs
crates/forge-nyx-client/tests/nyx_health.rs
python3 scripts/run_ci.py
```

## Operator validation

The operator ran:

```bash
python3 scripts/run_ci.py
```

Result: `PASS` with 79 suites, 374 tests passed, 0 failed, 2 ignored, and all three structural guards green. The bounded managed-service capability is closed.

## Explicit non-claims

This slice does not select models, create conversations, store Nyx sessions, expose
Nyx tools, implement policy or checkpoints, call providers, install a system service,
or close the Tier-3 login journey. Nyx_Server remains independently runnable for chat
and development clients.
