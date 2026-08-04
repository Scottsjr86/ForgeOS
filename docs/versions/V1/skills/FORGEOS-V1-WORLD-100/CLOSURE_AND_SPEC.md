# FORGEOS-V1-WORLD-100 Closure and Specification

Status: `ACTIVE_OPERATOR_VALIDATION_PENDING`
Capability: Source-backed view projection and input action routing
Active slice: `FORGEOS-V1-WORLD-100-SLICE-001`
Source authority: `Forge_OS_V1_base_38.tar`
Source archive SHA-256: `f871c859b4cb161b77094e94e695800c44770e0ff0ee19d9a74f8d82d9503be2`
Git revision: unavailable because the supplied source archive contains no `.git` metadata

## Capability statement

Forge World now builds an immutable project view from canonical
`ProjectRegistryState` and emits typed `ForgeUserIntent` values through a
read-only action router. Project, repository, and command identities remain exact
opaque IDs. Display names, path text, list position, viewport dimensions, and
renderer state never become canonical identity or mutate Forge Core.

## Public contract

```text
forge_protocol::intents::ForgeUserIntent
forge_world::presentation::DisplayPath
forge_world::presentation::CommandView
forge_world::presentation::ProjectView
forge_world::presentation::ProjectRegistryProjection
forge_world::presentation::Viewport
forge_world::presentation::PresentationFrame
forge_world::interaction::WorldInputAction
forge_world::interaction::WorldActionRouter
forge_world::interaction::WorldActionError
```

## Intended behavior

- projection consumes a shared immutable `ProjectRegistryState` reference;
- project rows are deterministically ordered by stable project identity;
- recent-project order comes from the canonical registry sequence;
- exact display-root bytes survive projection without lossy conversion;
- registered commands retain exact command identity and definition identity;
- snapshot presence is represented by its source-owned content identity;
- actions are rejected when project or command identity is absent from the
  current projection;
- emitted intents carry the exact source generation observed by the user;
- command intents include project, repository, and command identity;
- rerender and viewport resize change only presentation-owned frame metadata.

## Regression locks

```text
crates/forge-world/tests/source_projection.rs
python3 scripts/run_ci.py
```

The behavioral matrix covers deterministic projection, exact non-UTF-8 path
bytes, duplicate display names and paths, typed project actions, registered
command routing, unknown identity rejection, rerender, and viewport resize.

## Operator validation still required

Run the canonical behavior-only CI entrypoint:

```bash
python3 scripts/run_ci.py
```

The skill remains active until the operator returns green structural guards and
Rust tests for this slice.

## Explicit non-claims

This skill does not render a Bevy shell, own project mutation, execute commands,
inspect Git, manage terminals, present Nyx state, or implement a full workspace
HUD. Those capabilities consume this projection and intent boundary later.
