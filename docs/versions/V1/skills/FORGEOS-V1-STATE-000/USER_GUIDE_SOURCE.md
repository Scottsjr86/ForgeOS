# FORGEOS-V1-STATE-000 User Guide Source

Status: `CLOSED_SOURCE`
Capability: atomic versioned local persistence

## What the operator can do

ForgeOS can create one current-schema local record, reopen equivalent canonical
state, atomically replace it while retaining the previous valid record, detect an
interrupted staged write, explicitly migrate the reviewed V0 fixture, and
explicitly restore the retained previous record.

## Validation commands

```bash
cargo test -p forge-core
cargo test -p forge-project
```

## Visible outcomes

- Current records reopen with equivalent type and payload bytes.
- Corrupt, truncated, trailing, legacy, unsupported, and reserved records fail
  explicitly.
- Injected failures before publication preserve the valid current record.
- Interrupted staging is reported rather than hidden or auto-applied.
- Recovery occurs only through the explicit previous-state operation.
- State and companion-file symlinks are rejected.

## Fixed safety behavior

Forge Core owns schema and canonical bytes. Forge Project owns filesystem effects.
No UI layer or helper script may silently invent defaults, guess a schema, or
replace invalid state behind the operator's back.

## Current V1 limitations

The state checksum detects accidental corruption but is not a durable artifact
identity. SHA-256 identity, project manifests, file access, recovery journals, and
higher product workflows remain separately routed capabilities.
