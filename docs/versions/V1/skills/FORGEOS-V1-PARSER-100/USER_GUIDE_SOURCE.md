# FORGEOS-V1-PARSER-100 User Guide Source

Status: `CLOSED_SOURCE`
Capability: incremental Tree-sitter Rust parsing adapter

## What the operator can do

ForgeOS can parse valid and invalid Rust buffer generations, inspect named syntax
spans and parser issues, incrementally update syntax from exact previous bytes, and
reject stale or identity-mismatched parser state without taking ownership of source
bytes.

## Validation command

```bash
python3 scripts/run_ci.py
```

## CI boundary

CI is limited to behavior, goldens, Rust tests, and structural guards. Markdown,
Git state, and formatting are not CI authority.

## Current V1 limitations

The parser does not start Rust Analyzer, provide language-server diagnostics,
completion, navigation, file saving, repository search, terminal, Git, Nyx, session,
recovery, or Forge World behavior.
