# FORGEOS-V1-NYX-100 Closure and Specification

Status: `CLOSED`
Capability: Nyx health and versioned client protocol
Closed: `2026-08-03`
Closed slice: `FORGEOS-V1-NYX-100-SLICE-002`
Mandatory wiring review: `docs/versions/V1/FORGEOS_NYX_SERVER_WIRING_CHEAT_SHEET.md`
Mandatory capability ownership review: `docs/versions/V1/FORGEOS_NYX_SERVER_CAPABILITY_OWNERSHIP_CHEAT_SHEET.md`
Cross-repo gate authority: `docs/versions/V1/FORGEOS_V1_NYX_SERVER_DEPENDENCY_CONTRACT.md`
Verified Nyx gate input: `docs/versions/V1/skills/FORGEOS-V1-NYX-100/NYX_GATE_INPUT.json`
Forge source authority: `Forge_OS_V1_base_38.tar`
Forge source archive SHA-256: `f871c859b4cb161b77094e94e695800c44770e0ff0ee19d9a74f8d82d9503be2`
Nyx source authority inspected: `Nyx_Server_base_13.tar`
Nyx source archive SHA-256: `800e499b0d9b7d9aa60d4c55920f9e9dc7be48f1c4a52fbd794800c8f9c3b26d`
Git revision: unavailable because the supplied source archives contain no `.git` metadata

## Capability statement

ForgeOS probes one explicitly configured local TCP or Unix endpoint through the
Nyx-owned public HTTP contract. It requests `/v1/nyx/version`,
`/v1/nyx/health`, and `/v1/nyx/capabilities`, sends the highest supported
`x-nyx-contract-version`, validates the returned contract header and canonical
schema IDs, and produces a typed ready, unavailable, incompatible, or unhealthy
result.

Nyx_Server remains authoritative for server version, protocol version, health,
capabilities, engine readiness, and provider readiness. ForgeOS performs no
provider call, server-side capability calculation, permission decision, model
selection, conversation mutation, or service lifecycle operation in this skill.

## Nyx cross-repository evidence

The following Nyx skills are verified `BANKED` at `PROOF_SYSTEM`:

```text
API-FOUND-008
API-VERSION-010
API-SYS-044
API-SYS-047
```

The exact receipt hashes, Nyx CI result, public contract version, endpoint paths,
real-server witness hash, and standalone chat/development regression result are
recorded in `NYX_GATE_INPUT.json`.

This satisfies the Nyx-owned side of `NYX-GATE-FORGEOS-V1-NYX-100`. The
operator subsequently passed Forge behavior-only CI and the independent real
Nyx process witness, so the Forge capability is closed.

## Public ForgeOS contract

```text
forge_nyx_client::protocol::NyxProtocolVersion
forge_nyx_client::protocol::NyxHealth
forge_nyx_client::protocol::NyxAvailability
forge_nyx_client::protocol::NyxCapability
forge_nyx_client::protocol::NyxEngineReadiness
forge_nyx_client::protocol::NyxProviderReadiness
forge_nyx_client::protocol::NyxServiceReport
forge_nyx_client::protocol::NyxProtocolError
forge_nyx_client::transport::NyxTransportEndpoint
forge_nyx_client::transport::NyxClientConfig
forge_nyx_client::transport::NyxProbeOutcome
forge_nyx_client::transport::NyxProbeStatus
forge_nyx_client::transport::NyxUnavailableReason
forge_nyx_client::transport::NyxIncompatibility
forge_nyx_client::transport::probe_nyx
```

## Intended behavior

- compatible major versions may negotiate to Nyx's canonical published minor;
- incompatible majors are decoded from Nyx's canonical `426 Upgrade Required`
  error envelope;
- missing endpoints preserve typed transport failures;
- HTTP success without the Nyx contract header is incompatible;
- header/body contract disagreement is incompatible;
- malformed HTTP, JSON, schema IDs, versions, or readiness payloads fail closed;
- healthy and fully ready Nyx returns `Ready`;
- degraded or unavailable Nyx retains its declared capabilities and readiness
  details while returning `Unhealthy`;
- capability, engine, and provider identifiers are duplicate-rejected and
  projected in deterministic order;
- health summary counts must match the detailed readiness inventories;
- the same HTTP contract may travel over configured TCP or Unix transport, but
  the current Nyx_Server deployment exposes TCP HTTP at `NYX_BIND`.

## Regression locks

```text
crates/forge-nyx-client/tests/nyx_health.rs
python3 scripts/run_ci.py
```

The test matrix covers healthy compatibility, unavailable transport,
incompatible-major rejection, malformed responses, degraded readiness, schema
mismatch, contract-header mismatch, and Unix HTTP transport. One ignored test is
the explicit real Nyx process witness.

## Accepted operator evidence

The operator ran the canonical behavior-only CI entrypoint and returned:

```text
FORGE_SEAM_DIRECTION_SUMMARY status=PASS packages=12 routes=42 forbidden=0 policy=exact-reviewed-subsystem-reachability-v1
FORGE_CORE_PURITY_SUMMARY status=PASS packages=2 allowed=2 forbidden=0 policy=exact-reviewed-production-graph-v1
FORGE_SOURCE_SIZE_SUMMARY status=PASS modules=107 pass=107 warn=0 fail=0 warnings_denied=true
CARGO_TEST_SUMMARY status=PASS suites=58 passed=274 failed=0 ignored=1 measured=0 filtered_out=0
CI RESULT: PASS
```

The operator also started Nyx_Server independently and ran:

```bash
FORGE_NYX_ADDR=127.0.0.1:8088 \
  cargo test --locked -p forge-nyx-client --test nyx_health \
  real_nyx_public_api_gate -- --ignored --nocapture
```

The real process returned public contract `1.0`, server version `0.1.0`, schema
`nyx.1.0`, truthful `Degraded` health, twelve engine records, one provider
record, and six capability records. ForgeOS classified the compatible but not
model-ready server as `Unhealthy` and the test passed:

```text
test real_nyx_public_api_gate ... ok
test result: ok. 1 passed; 0 failed; 0 ignored; 8 filtered out
```

This proves the client preserves Nyx-owned readiness truth instead of treating
transport success or process presence as readiness.

## Explicit non-claims

This skill does not manage the Nyx process, select models, create conversations,
grant tools, persist permission checkpoints, resume suspended actions, dispatch
remote agents, or present Nyx state through Forge World. Those remain separate,
Nyx-gated capabilities.
