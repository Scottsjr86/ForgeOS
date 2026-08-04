# FORGEOS-V1-WORLD-100 User Guide Source

Status: `ACTIVE_OPERATOR_VALIDATION_PENDING`
Capability: Source-backed view projection and input action routing

## What the operator can verify

Forge World can receive canonical project registry state as an immutable view,
project stable project and command identities for presentation, and translate
user actions into typed requests for the owning subsystem.

The presentation layer cannot directly rename, open, close, execute, or otherwise
mutate project state. It emits a typed intent carrying the exact source generation
that was visible when the action occurred.

## Identity and display behavior

- project identity comes from `ProjectId`, never display name or row position;
- repository identity comes from `RepositoryId`, never a path string;
- command identity comes from `CommandId`, never button text;
- non-UTF-8 root bytes remain exact and have a stable escaped display form;
- recent ordering comes from canonical registry sequence values;
- renderer viewport changes do not alter canonical state.

## Validation command

```bash
python3 scripts/run_ci.py
```

## Current V1 limitations

This boundary does not yet provide the Bevy shell, project browser, editor,
terminal, Git cockpit, Nyx panel, or status HUD. It supplies the honest data and
input seam those surfaces must use.
