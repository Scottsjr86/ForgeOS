# FORGEOS-V1-FILE-100 User Guide Source

Status: `CLOSED_SOURCE`
Capability: boundary-safe file access and atomic write

## What the operator can do

ForgeOS can read exact repository file bytes and explicitly create or atomically
replace approved files through a validated project manifest and repository
boundary. Every replacement declares the disk revision it expects.

## Validation command

```bash
python3 scripts/run_ci.py
```

## Visible outcomes

- Raw bytes and non-UTF8 path components are preserved.
- Missing files require an explicit create expectation.
- Existing files require the exact observed revision.
- Changed-on-disk content is reported as a conflict before replacement.
- Denied roots, repository mismatches, symlinks, and directories fail explicitly.
- Injected pre-commit failure preserves original bytes and removes staging.

## CI boundary

CI is limited to behavior, goldens, Rust tests, and structural guards. Markdown,
Git state, and formatting are not CI authority and cannot award closure.

## Current V1 limitations

The file primitive does not build a repository tree, search source, own editor
buffers, parse syntax, execute commands, inspect Git, or present UI state. Those
remain separate routed capabilities.
