# FORGEOS-V1-PATCH-100 User Guide Source

Status: `CLOSED_SOURCE`
Capability: Stable patch validation and all-or-nothing application

## What the operator can do

ForgeOS can validate and apply exact declared add, modify, and delete text patches
against an exact repository and base revision. Unsafe paths, hidden files, stale
content, binary data, unsupported metadata, and partial application fail without
being accepted as success.

## Validation command

```bash
python3 scripts/run_ci.py
```

## CI boundary

CI is limited to behavior, goldens, Rust tests, and structural guards. Markdown,
Git state, and formatting are not CI authority.

## Current V1 limitations

Complete Git integration, external agent workflow, patch review UI, durable
verification records, and Forge World presentation remain owned by later skills.
