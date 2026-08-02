# FORGEOS-V1-SESSION-100 User Guide Source

Status: `CLOSED_SOURCE`
Capability: Deterministic session and managed-service lifecycle contract

## What the operator can do

ForgeOS can define one explicitly ordered service plan, drive a stable session
through dependency-gated start requests and exact process-bound readiness, apply a
bounded pre-readiness retry policy, roll back already-ready services after terminal
failure, and stop services in deterministic reverse order while preserving native
failures.

## Validation command

```bash
python3 scripts/run_ci.py
```

## CI boundary

CI is limited to behavior, goldens, Rust tests, and structural guards. Markdown,
Git state, and formatting are not CI authority.

## Current V1 limitations

The primitive does not launch a login session, spawn the declared services, manage
Nyx health, persist workspace recovery, package ForgeOS, or render lifecycle state
through Forge World.
