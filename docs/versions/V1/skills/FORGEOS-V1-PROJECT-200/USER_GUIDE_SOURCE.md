# FORGEOS-V1-PROJECT-200 User Guide Source

Status: `CLOSED_SOURCE`
Capability: Persistent project registry and workspace restoration

## What the operator can do

ForgeOS can register multiple validated repositories, rename project display names,
mark projects open or closed, retain deterministic recent-project order, save one
versioned safe workspace payload, reopen the registry after restart, relocate a
repository only when the same directory object moved, and remove a project record
without deleting or editing repository source.

## Identity and safety behavior

- project identity comes from the stable project ID, not the display name;
- repository identity comes from the stable repository ID plus the real directory
  object, not the path string alone;
- registered command definitions are retained as exact canonical bytes;
- recent-open order uses a registry-local monotonic sequence, never wall-clock time;
- corrupt or unsupported state fails closed;
- duplicate identities and failed mutations do not publish partial registry bytes;
- copied or replaced repository roots are rejected;
- source files are outside registry ownership and remain untouched on removal.

## Validation command

```bash
python3 scripts/run_ci.py
```

## Expected failures and recovery

A missing, copied, or replaced repository root prevents registry reopening until the
correct original directory object is restored or an explicit same-object relocation
is performed. Corrupt registry state is reported as an error and is not silently
replaced. An abandoned staged write is reported and can be explicitly discarded.

## Current V1 limitations

This capability stores project and workspace truth but does not yet expose the file
tree, editor workspace, embedded terminal, Git cockpit, recovery replay, or Forge
World controls. Those surfaces consume this registry in later skills.
