# FORGEOS-V1-LSP-100 User Guide Source

Status: `CLOSED_SOURCE`
Capability: Rust Analyzer process and version-safe JSON-RPC adapter

## What the operator can do

ForgeOS can start a configured Rust Analyzer-compatible service for one canonical
project, synchronize exact UTF-8 editor generations, receive native diagnostics,
answer bounded server requests, restart safely, and expose typed failures without
mutating editor source bytes.

## Validation command

```bash
python3 scripts/run_ci.py
```

## CI boundary

CI is limited to behavior, goldens, Rust tests, and structural guards. Markdown,
Git state, and formatting are not CI authority.

## Current V1 limitations

The adapter does not provide save integration, completion or navigation UI,
registered commands, PTYs, Git, Nyx, session lifecycle, recovery, or Forge World
presentation.
