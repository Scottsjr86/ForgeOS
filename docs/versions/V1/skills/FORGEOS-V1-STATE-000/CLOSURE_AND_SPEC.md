# FORGEOS-V1-STATE-000 Closure and Specification

Status: `CLOSED`
Capability: atomic versioned local persistence
Closed: `2026-08-02`
Source authority: `Forge_OS_V1_base_18.tar`
Source archive SHA-256: `3433d3311dcc864dc65e2f3cd707397549bfcb6a3936e9e14f0308c3fd64bcf3`
Git revision: unavailable because the supplied source archive contains no `.git` metadata

## Capability statement

Forge Core now owns exact versioned local-state record bytes, explicit schema
interpretation, corruption detection, and the reviewed V0-to-V1 migration path.
Forge Project owns the Linux filesystem effect through synced staging, atomic
replacement, visible interrupted writes, retained previous-state bytes, and
explicit recovery.

The accepted path never guesses schemas, silently resets invalid state, overwrites
an existing record during create, follows state-file symlinks, or discards the
previous valid record before a replacement is proven and published.

## Public contract

The accepted source exposes:

```text
forge_core::state::StateRecord
forge_core::state::StateRecordError
forge_core::state::MigratedStateRecord
forge_core::state::migrate_legacy_v0
forge_core::state::encode_legacy_v0_fixture
forge_project::persistence::AtomicStateStore
forge_project::persistence::OpenedStateRecord
forge_project::persistence::StateStoreError
```

## Exercised path and evidence

The operator applied the state patch, ran the handed-off Cargo and guard command
chain, reported all Cargo checks green and all three structural guards passing,
and advanced the workflow on `2026-08-02`.

Operator-run validation covered:

```bash
cargo fmt --all
cargo fmt --all -- --check
cargo check --workspace
cargo test -p forge-core
cargo test -p forge-project
cargo run -p forge-guards --bin forge-seam-direction -- --root .
cargo run -p forge-guards --bin forge-core-purity -- --root .
cargo run -p forge-guards --bin forge-source-size -- --root . --deny-warnings
cargo test --workspace
git diff --check
git status --short
```

## Negative and failure-path results

Focused fixtures cover corrupt, truncated, trailing, legacy, unsupported-version,
reserved-type, oversized, missing, duplicate-create, symlink, interrupted-stage,
injected pre-commit failure, retained-previous-state, and explicit recovery paths.

## Regression locks

Later persistent records must use explicit schemas and canonical Core bytes. File
adapters must preserve valid current state on failed writes, expose interrupted
state, and require an explicit recovery decision. The integrity checksum remains
corruption detection only and must not impersonate the SHA-256 identity contract.

## Explicit non-claims

This closure does not provide SHA-256 artifact identity, project manifests,
repository file access, workspace recovery journals, Git, terminal, command,
session, LSP, Nyx, agent, or Forge World behavior.
