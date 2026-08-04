# FORGEOS-V1-NYX-100 User Guide Source

Status: `CLOSED`
Capability: Nyx health and versioned client protocol
Cross-repo gate input: `docs/versions/V1/skills/FORGEOS-V1-NYX-100/NYX_GATE_INPUT.json`

## What the operator can do

ForgeOS can probe one configured local Nyx endpoint through Nyx's public HTTP
contract. It reads the published version, health, capabilities, engine readiness,
and provider readiness, then reports one of four states:

```text
Ready
Unavailable
Incompatible
Unhealthy
```

Nyx_Server remains a separate, independently runnable server. ForgeOS neither
hosts it inside the client crate nor contacts model providers around it.

## Compatibility and safety behavior

- `x-nyx-contract-version` carries the highest Forge-supported version;
- compatible major versions may return Nyx's canonical published minor version;
- incompatible majors return `Incompatible` from Nyx's versioned error envelope;
- missing endpoints return `Unavailable` with the native transport error class;
- missing headers, malformed JSON, wrong schemas, and contradictory readiness
  data return `Incompatible`;
- degraded or unavailable server health returns `Unhealthy` while retaining the
  exact capability, engine, and provider details supplied by Nyx;
- transport success alone never means the server is compatible or ready.

## Forge validation

```bash
python3 scripts/run_ci.py
```

## Real Nyx witness

Start Nyx_Server from its own repository:

```bash
NYX_BIND=127.0.0.1:8088 cargo run --locked --quiet -p nyx_server --bin nyx_server
```

From ForgeOS, run:

```bash
FORGE_NYX_ADDR=127.0.0.1:8088 \
  cargo test --locked -p forge-nyx-client --test nyx_health \
  real_nyx_public_api_gate -- --ignored --nocapture
```

The witness passes when the real Nyx process returns a compatible versioned
health and capability report. A degraded server remains visibly degraded.

## Current V1 limitations

The probe does not manage the Nyx process, select a model, create a conversation,
grant tools, resolve checkpoints, or dispatch agents. Those capabilities are
activated and proved separately.
