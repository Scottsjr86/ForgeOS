# FORGEOS-V1-GIT-101 User Guide Source

Status: `CLOSED_SOURCE`
Capability: Git mutation and linked-worktree primitives

## What the operator can do

ForgeOS can stage and unstage exact literal paths, restore explicitly confirmed
files, commit an exact staged state, create a linked worktree from an exact object,
and remove only a registered clean non-primary worktree.

## Validation command

```bash
python3 scripts/run_ci.py
```

## CI boundary

CI is limited to behavior, goldens, Rust tests, and structural guards. Markdown,
Git state, and formatting are not CI authority.

## Current V1 limitations

External patch validation and application, complete Git subsystem integration,
project restoration, UI presentation, merge, rebase, and other broad source-control
workflows remain owned by later skills.
