# FORGEOS-V1-PATH-000 User Guide Source

Status: `CLOSED_SOURCE`
Capability: canonical repository path and boundary identity

## What the operator can do

ForgeOS can open a real repository root under a stable `RepositoryId`, preserve the
path shown to the operator separately from the canonical enforcement root, resolve
existing repository children, and rebind the same directory object after an honest
move.

## Validation commands

```bash
cargo test -p forge-protocol
cargo test -p forge-project
```

## Visible outcomes

- Valid in-root children resolve to verified canonical paths.
- A moved but identical directory object retains its repository identity.
- Traversal, absolute, aliased, wrong-repository, symlinked, replaced, missing,
  non-directory, outside-root, and unexpected-mount cases fail explicitly.
- Non-UTF8 Unix path components remain exact operating-system strings.

## Fixed safety behavior

Repository identity is not a path string. Every existing-child resolution
revalidates the root object, walks components without following symlinks, and
confirms the final canonical object remains inside the verified root.

## Current V1 limitations

This capability resolves existing paths only. It does not create or replace files,
register projects, run commands, inspect Git, allocate terminals, or persist project
state. Those remain separate routed skills.
