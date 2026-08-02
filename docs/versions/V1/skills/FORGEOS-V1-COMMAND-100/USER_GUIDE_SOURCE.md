# FORGEOS-V1-COMMAND-100 User Guide Source

Status: `CLOSED_SOURCE`
Capability: Immutable registered-command definition and exact launch policy

## What the operator can do

ForgeOS can define one immutable project command with stable identity, exact
executable and argv tokens, an explicit repository-bound working directory,
clear-parent declared environment variables, timeout, process-group cancellation,
and authority class. The exact launch payload can be inspected before any process
starts.

## Validation command

```bash
python3 scripts/run_ci.py
```

## CI boundary

CI is limited to behavior, goldens, Rust tests, and structural guards. Markdown,
Git state, and formatting are not CI authority.

## Current V1 limitations

The primitive does not execute registered commands, capture output, preserve command
history, restore terminals, mutate Git, manage session services, connect Nyx,
perform recovery, or render Forge World presentation.
