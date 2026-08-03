# FORGEOS-V1-GIT-100 User Guide Source

Status: `CLOSED_SOURCE`
Capability: Read-only native Git inspection

## What the operator can do

ForgeOS can inspect the exact branch, revision, worktree status, linked worktrees,
and worktree, staged, or exact-revision diffs of one verified native repository.
Raw path bytes, native failures, and exact binary patch bytes are preserved.

## Validation command

```bash
python3 scripts/run_ci.py
```

## CI boundary

CI is limited to behavior, goldens, Rust tests, and structural guards. Markdown,
Git state, and formatting are not CI authority.

## Current V1 limitations

The adapter is read-only. Staging, commit, restore, worktree mutation, patch
application, project restoration, UI presentation, and complete Git workflows are
owned by later skills.
