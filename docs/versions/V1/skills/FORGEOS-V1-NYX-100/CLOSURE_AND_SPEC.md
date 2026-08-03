# FORGEOS-V1-NYX-100 Closure and Specification

Status: `SOURCE_PROVED_CROSS_REPO_GATE_OPEN`
Capability: Nyx health and versioned client protocol
Forge client source proved: `2026-08-03`
Source authority: `Forge_OS_V1_base_33.tar`
Source archive SHA-256: `5ddd6d4c79a5faf209e835d5fb2374af0a50adabffe2c41b005f41dfedb62833`
Git revision: unavailable because the supplied source archive contains no `.git` metadata

## Capability statement

ForgeOS now probes one explicitly configured local Unix-socket or TCP Nyx endpoint,
sends deterministic framed handshake bytes, negotiates only an offered protocol
version, decodes the declared service version, health, and canonical capability set,
and classifies the result as ready, unavailable, incompatible, or unhealthy. A
responding socket is never treated as compatibility by itself, malformed bytes fail
closed without a panic, and ForgeOS does not bypass Nyx to contact model providers.

## Public contract

```text
forge_nyx_client::protocol::NyxProtocolVersion
forge_nyx_client::protocol::NyxCapability
forge_nyx_client::protocol::NyxHealth
forge_nyx_client::protocol::NyxHandshakeRequest
forge_nyx_client::protocol::NyxHandshakeResponse
forge_nyx_client::protocol::NyxProtocolError
forge_nyx_client::transport::NyxTransportEndpoint
forge_nyx_client::transport::NyxClientConfig
forge_nyx_client::transport::NyxProbeOutcome
forge_nyx_client::transport::NyxProbeStatus
forge_nyx_client::transport::NyxUnavailableReason
forge_nyx_client::transport::NyxIncompatibility
forge_nyx_client::transport::probe_nyx
```

## Accepted operator evidence

The operator ran the canonical behavior-only CI entrypoint and returned:

```text
FORGE_SEAM_DIRECTION_SUMMARY status=PASS packages=12 routes=42 forbidden=0 policy=exact-reviewed-subsystem-reachability-v1
FORGE_CORE_PURITY_SUMMARY status=PASS packages=2 allowed=2 forbidden=0 policy=exact-reviewed-production-graph-v1
FORGE_SOURCE_SIZE_SUMMARY status=PASS modules=107 pass=107 warn=0 fail=0 warnings_denied=true
CARGO_TEST_SUMMARY status=PASS suites=58 passed=274 failed=0 ignored=0 measured=0 filtered_out=0
CI RESULT: PASS
```

This proves the Forge client and fixture matrix only. It does not close the capability:
`NYX-GATE-FORGEOS-V1-NYX-100` still requires `API-FOUND-008`, `API-VERSION-010`,
`API-SYS-044`, and `API-SYS-047` at `BANKED` or `RELEASE_EARNED` with at least
`PROOF_SYSTEM`, plus a real Nyx process consuming the same Nyx-owned public contract.
CI remains limited to behavioral tests, golden locks, and structural guards.

## Proved behavior

- request and response frames use deterministic versioned bytes;
- offered protocol versions are canonical, ordered, and duplicate-free;
- healthy compatible fixtures return exact service version and capabilities;
- missing Unix and TCP endpoints classify unavailable with typed transport reasons;
- an endpoint selecting an unoffered protocol classifies incompatible;
- malformed response bytes classify incompatible without crashing;
- degraded and unhealthy fixtures retain health and capabilities while classifying
  unhealthy;
- oversized frames and incomplete I/O fail closed;
- no provider call, model selection, conversation, permission, or service-lifecycle
  behavior is hidden behind the probe.

## Regression locks

```text
crates/forge-nyx-client/tests/nyx_health.rs
python3 scripts/run_ci.py
```

## Explicit non-claims

The private `FGNYXQ` / `FGNYXR` fixture protocol is not implemented by the current
Nyx_Server HTTP/JSON API and is not a real-server closure witness.

This source proof does not grant tools, persist permission checkpoints, select models,
create conversations, manage the Nyx process, dispatch remote agents, or present Nyx
state through Forge World. Those remain separate skills.
