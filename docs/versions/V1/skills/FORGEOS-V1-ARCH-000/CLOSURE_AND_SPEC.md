# FORGEOS-V1-ARCH-000 Closure and Specification

Status: `CLOSED`
Capability: Rust workspace and authority crate skeleton
Closed: `2026-07-31`
Source authority: `Forge_OS_V1_base_10.tar`
Source archive SHA-256: `f5286887be76ba9a5f930266dac06c010dc5122636af146d0588cdbe255a9dd2`
Git revision: unavailable because the supplied source archive contains no `.git` metadata

## Capability statement

ForgeOS now has one compiling Rust workspace with the twelve declared V1 authority
crates, a behavior-free executable composition root, explicit local dependency
direction, workspace tests, formatting configuration, and no external runtime
implementation smuggled into the architecture root.

## Owning crates and modules

The repository root owns the workspace manifest. The initial authored Rust modules
are the crate roots for `forge-protocol`, `forge-core`, `forge-project`,
`forge-session`, `forge-bridge`, `forge-terminal`, `forge-git`, `forge-editor`,
`forge-nyx-client`, `forge-world`, `forge-app`, and `forge-guards`, plus the four
workspace-skeleton test modules.

Module line counts in the accepted base:

```text
crates/forge-app/src/main.rs                         6
crates/forge-app/tests/workspace_skeleton.rs        15
crates/forge-bridge/src/lib.rs                       3
crates/forge-core/src/lib.rs                         4
crates/forge-core/tests/workspace_skeleton.rs        7
crates/forge-editor/src/lib.rs                       3
crates/forge-git/src/lib.rs                          3
crates/forge-guards/src/lib.rs                       4
crates/forge-guards/tests/workspace_skeleton.rs      6
crates/forge-nyx-client/src/lib.rs                   4
crates/forge-project/src/lib.rs                      3
crates/forge-protocol/src/lib.rs                     3
crates/forge-protocol/tests/workspace_skeleton.rs    6
crates/forge-session/src/lib.rs                      3
crates/forge-terminal/src/lib.rs                     3
crates/forge-world/src/lib.rs                        4
```

## Exercised path and evidence

The operator applied the architecture patch and reported `cargo is green` before
requesting the next slice. This is the explicit user approval record for the crate
boundaries and the operator-run Rust validation on `2026-07-31`.

Operator validation covered the handed-off Rust path:

```bash
cargo fmt --all -- --check
cargo check --workspace
cargo test --workspace
```

Assistant-side patch preparation and regression evidence recorded before handoff:

```text
workspace members: 12
local dependency cycles: none
external runtime dependency scan: pass
git diff --check: pass
git apply --check against fresh extraction: pass
independent applied-tree comparison: pass
```

## Negative and failure-path results

Static inspection found no backward dependency cycle, no Bevy or model-provider
runtime dependency, no Nyx host implementation, and no business logic in
`forge-app`, `main.rs`, scripts, or a monolithic crate. Missing Rust tooling in the
assistant environment was correctly handled as operator validation rather than a
source blocker.

## Supported behavior

- The workspace and every declared V1 authority crate compile and test together on
  the operator host.
- `forge-core` depends only on `forge-protocol`.
- The executable root contains no product business logic.
- Real subsystem implementations remain absent until their owning skills activate.

## Explicit non-claims

This closure does not claim stable IDs, persistence, path security, process
execution, terminals, Git behavior, editing, Nyx communication, Bevy rendering,
session startup, installation, self-hosting, or any user-facing ForgeOS workflow.

## V1 scope limits

The skeleton establishes boundaries only. Module routing, seam guards, contracts,
and every functional capability remain separate V1 work rather than deferred
cleanup.
