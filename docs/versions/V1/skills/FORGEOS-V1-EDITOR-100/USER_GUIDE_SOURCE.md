# FORGEOS-V1-EDITOR-100 User Guide Source

Status: `CLOSED_SOURCE`
Capability: editor buffer identity and dirty-state model

## What the operator can do

ForgeOS can represent one authoritative in-memory buffer for each canonical project
document, track local edits and cursor state, detect external disk conflicts, prepare
exact save intents, and prevent destructive close when work is dirty or conflicted.

## Validation command

```bash
python3 scripts/run_ci.py
```

## CI boundary

CI is limited to behavior, goldens, Rust tests, and structural guards. Markdown,
Git state, and formatting are not CI authority.

## Current V1 limitations

The buffer model does not parse syntax, call Rust Analyzer, write files, search the
repository, run commands, inspect Git, or render a user interface.
