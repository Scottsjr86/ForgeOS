# FORGEOS-V1-TERMINAL-100 User Guide Source

Status: `CLOSED_SOURCE`
Capability: Native PTY spawn, exact byte I/O, resize, exit, and isolated termination

## What the operator can do

ForgeOS can start independent native PTY sessions with stable IDs, exact executable
and argv values, an explicit working directory and dimensions, raw byte input and
output, real resize, native exit information, and explicit termination.

## Validation command

```bash
python3 scripts/run_ci.py
```

## CI boundary

CI is limited to behavior, goldens, Rust tests, and structural guards. Markdown,
Git state, and formatting are not CI authority.

## Current V1 limitations

The PTY primitive does not provide a rendered terminal, ANSI parsing, registered
command policy or execution, command history, project restoration, Git, Nyx,
session lifecycle, recovery, or Forge World presentation.
