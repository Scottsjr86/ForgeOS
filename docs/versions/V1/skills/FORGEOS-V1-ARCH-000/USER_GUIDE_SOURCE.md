# FORGEOS-V1-ARCH-000 User Guide Source

Status: `CLOSED`
Audience: ForgeOS developers and build operators

## What this capability does

This capability turns the former documentation-only repository into the initial
ForgeOS Rust workspace. It gives each V1 authority its own crate and keeps the
binary entrypoint free of business logic.

## Access and usage

From the ForgeOS repository root, inspect the workspace members in `Cargo.toml`
and run:

```bash
cargo fmt --all -- --check
cargo check --workspace
cargo test --workspace
```

A successful result means the architecture skeleton formats, resolves, compiles,
and passes its initial tests as one workspace.

## Configuration

- Rust edition: 2024
- Cargo resolver: 3
- Publishing: disabled for workspace crates
- Unsafe Rust: forbidden through workspace lints
- Workspace membership: the twelve declared ForgeOS V1 authority crates

There are no runtime controls, shortcuts, services, model settings, or user-facing
options in this internal foundation capability.

## Expected result

Cargo exits successfully for formatting, checking, and testing. The executable
composition root compiles without implementing product behavior.

## Errors and recovery

- A missing local crate path means the workspace skeleton is incomplete. Restore
  the declared crate rather than deleting it from the workspace.
- A dependency cycle or backward dependency means crate ownership has been
  violated. Remove the backward edge and place the behavior behind its owning
  seam.
- Business logic in `main.rs` or `forge-app` must move to the crate that owns that
  behavior before the architecture can remain accepted.
- Missing Rust tooling on a review host is not a source defect. Run the commands on
  the declared operator host and record the exact results.

## Support implications

Support should ask for the exact failing Cargo command, compiler output, workspace
manifest, and changed crate manifests. A screenshot or model claim is not enough
to diagnose architecture validation.

## Nyx interaction

None. The workspace includes only the ForgeOS-side `forge-nyx-client` boundary.
It does not host, start, or contact `nyx_server`.

## Current V1 limitations

This capability provides structure, not product behavior. No project management,
terminal, Git, editor, Nyx, world, session, persistence, or release workflow is
usable yet.
