# FORGEOS-V1-ARCH-001 Closure and Specification

Status: `CLOSED`
Capability: Scoped module hierarchy and public routing
Closed: `2026-07-31`
Source authority: `Forge_OS_V1_base_11.tar`
Source archive SHA-256: `00d4ebc349e46ddf4f0998885f4fbfe83818fc96d1ff94391c08fc7ad1c02bc5`
Git revision: unavailable because the supplied source archive contains no `.git` metadata

## Capability statement

Every production crate now routes its public API through an explicit `lib.rs`,
behavior namespaces are represented by named modules, and the executable root
delegates through `forge_app::composition::run()` without owning subsystem logic.
No generic catch-all production module or uncontrolled public re-export is present.

## Public route shape

The accepted source exposes the following named routes:

```text
forge_app::composition
forge_bridge::{adapters, ports}
forge_core::{capabilities, missions, projects, workspaces}
forge_editor::{buffers, language}
forge_git::{repository, worktree}
forge_guards::{core_purity, seams, source_size}
forge_nyx_client::{protocol, transport}
forge_project::{persistence, registry}
forge_protocol::{envelopes, errors, events, identities}
forge_session::{lifecycle, services}
forge_terminal::{commands, pty}
forge_world::{interaction, presentation}
```

`crates/forge-app/src/main.rs` contains only the executable entrypoint and delegates
to the explicit composition route. The composition module remains behavior-free.

## Exercised path and evidence

The operator applied the scoped-routing patch and reported `all green` before
requesting the next slice. This is the explicit user approval record for route
naming, module organization, and operator-run Rust validation on `2026-07-31`.

Operator validation covered the handed-off Rust path:

```bash
cargo fmt --all -- --check
cargo check --workspace
cargo test --workspace
```

Assistant-side patch preparation and regression evidence recorded before handoff:

```text
workspace members: 12
dependency edges: 30
public route modules: 28
dependency cycles: none
uncontrolled pub use: none
generic catch-all modules: none
authored source modules over 1000 lines: none
git diff --check: pass
git apply --check against fresh extraction: pass
independent applied-tree comparison: pass
```

## Negative and failure-path results

Static inspection found no business logic in crate roots, `main.rs`, or the
composition module; no wildcard or uncontrolled public re-export; no generic
catch-all module; no dependency cycle; and no source module above the future guard
warning boundary.

## Supported behavior

- Representative public module imports compile through the real Cargo workspace.
- Every production crate exposes named, ownership-specific public routes.
- The executable root delegates through one explicit composition module.
- Existing crate dependency direction remains unchanged.

## Explicit non-claims

This closure does not claim source-size enforcement, Forge Core purity enforcement,
seam-direction enforcement, stable IDs, persistence, process execution, terminal,
Git, editor, Nyx, world, session, packaging, or any user-facing workflow.

## V1 scope limits

The modules are routing boundaries only. Empty namespace modules do not prove their
named functional capabilities, and future behavior must land in the owning module
under its own active skill and proof contract.
