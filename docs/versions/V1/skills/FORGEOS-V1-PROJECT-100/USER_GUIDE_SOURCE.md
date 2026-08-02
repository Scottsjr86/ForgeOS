# FORGEOS-V1-PROJECT-100 User Guide Source

Status: `CLOSED_SOURCE`
Capability: validated project manifest and repository identity

## What the operator can do

ForgeOS can import a canonical project manifest, bind it to a real repository
directory object, validate declared project roots and Rust profile data, reopen the
same manifest equivalently, and reject malformed or duplicate registration.

## Validation command

```bash
python3 scripts/run_ci.py
```

## Visible outcomes

- Equivalent manifests encode identically.
- Duplicate project and repository identities are rejected.
- Missing, symlinked, or non-directory roots are rejected.
- A moved repository is rebound only when it is the same directory object.
- Unsupported schemas and unknown required fields fail explicitly.
- Core purity and seam direction remain unchanged.

## CI boundary

CI is limited to behavior, goldens, Rust tests, and structural guards. Markdown,
Git state, and formatting are not CI authority and cannot award closure.

## Current V1 limitations

Project registration does not read or mutate source files, persist the full project
registry, run commands, inspect Git, start services, or present Forge World state.
Those remain separate routed capabilities.
