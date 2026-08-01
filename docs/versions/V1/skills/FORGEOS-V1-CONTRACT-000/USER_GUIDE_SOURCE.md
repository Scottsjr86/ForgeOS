# FORGEOS-V1-CONTRACT-000 User Guide Source

Status: `CLOSED`
Audience: ForgeOS developers and maintainers

## What this capability does

`forge-protocol` provides the stable V1 identity, error, event, and envelope types
used across ForgeOS authority boundaries. The contract gives callers deterministic
bytes and typed failures instead of display-derived IDs or ad hoc strings.

## Run the contract proof

From the ForgeOS repository root:

```bash
cargo test -p forge-protocol
cargo test -p forge-core
```

The protocol tests cover canonical identity text, duplicate rejection, exact V1
request/result/error bytes, malformed input, unknown versions, and typed-error
round trips. The Core test proves the same public bytes are consumable through
Forge Core's declared protocol dependency.

## Canonical identity rules

- Every public identity type is distinct at compile time.
- The stored value is exactly 16 bytes.
- Canonical text is exactly 32 lowercase hexadecimal characters.
- Uppercase, malformed, short, or long text is invalid.
- Display names, paths, timestamps, list positions, and model text are not IDs.

## Envelope rules

- The schema version and envelope kind are explicit.
- V1 bytes are deterministic and golden-locked.
- Unknown versions and unknown kinds fail closed.
- Truncated, trailing, or malformed payloads are rejected.
- Errors remain typed after serialization and decoding.

## Errors and recovery

A `ProtocolError` is part of the public contract. Do not translate it into generic
success or silently retry with another format. Repair the caller or migrate the
contract explicitly, then rerun the focused protocol and Core tests.

## Nyx interaction

The contract can carry future Nyx-facing requests and results, but this capability
does not connect to or host `nyx_server`.

## Current V1 limitations

The contract defines deterministic values and bytes only. It does not persist
records, canonicalize repository paths, spawn processes, hash artifacts, or enforce
cross-subsystem dependency direction.
