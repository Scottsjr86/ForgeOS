# FORGEOS-V1-NYX-100 User Guide Source

Status: `SOURCE_PROVED_CROSS_REPO_GATE_OPEN`
Capability: Nyx health and versioned client protocol

## What the operator can do

ForgeOS can probe one configured local Nyx endpoint over a Unix socket or TCP,
negotiate an explicitly supported protocol version, and inspect the returned service
version, health, and declared capabilities. The result distinguishes ready,
unavailable, incompatible, and unhealthy service states.

The current probe's private `FGNYXQ` / `FGNYXR` protocol is covered by Forge fixtures
but has no matching handler in Nyx_Server's HTTP/JSON API. Treat these instructions as
Forge client validation, not cross-repository closure, until both repositories consume
one Nyx-owned public contract and `NYX-GATE-FORGEOS-V1-NYX-100` passes.

## Compatibility and safety behavior

- transport success alone is not compatibility;
- Nyx must select one protocol version ForgeOS actually offered;
- malformed or oversized response frames fail closed;
- unavailable endpoints preserve the transport failure class;
- unhealthy responses preserve their declared health and capabilities;
- no model provider is contacted around Nyx;
- probe failures do not crash or corrupt the local ForgeOS workflow.

## Validation command

```bash
python3 scripts/run_ci.py
```

## Expected failures and recovery

A missing endpoint reports unavailable. A responding service with an unsupported
protocol or malformed response reports incompatible. A degraded or unhealthy service
reports unhealthy and retains its declared metadata. Correct the endpoint or service,
then run the same probe again.

## Current V1 limitations

The health probe does not manage the Nyx process, select a local model, create a
conversation, grant tools, resume checkpoints, or dispatch remote agents. Those
capabilities are activated separately.
