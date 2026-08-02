# FORGEOS-V1-PATH-000 Closure and Specification

Status: `CLOSED`
Capability: canonical repository path and boundary identity
Closed: `2026-08-01`
Source authority: `Forge_OS_V1_base_17.tar`
Source archive SHA-256: `7aa7fcf5d6cc692c5079255a7b4eabe3c200500e42eaac3bece13f27a727b0e1`
Git revision: unavailable because the supplied source archive contains no `.git` metadata

## Capability statement

ForgeOS now binds one stable `RepositoryId` to one verified directory object while
keeping operator display paths separate from canonical enforcement paths. Existing
children resolve through a canonical repository-relative request without lossy
Unicode conversion or silent lexical cleanup.

The accepted boundary rejects absolute paths, parent traversal, alias components,
wrong repository identities, root and child symlinks, replaced roots, missing or
non-directory path components, object changes during resolution, outside-root
canonical results, and unexpected filesystem-device crossings. The same directory
object may be rebound after an honest move without changing repository identity.

## Public contract

The accepted source exposes:

```text
forge_protocol::paths::RepositoryRelativePath
forge_protocol::paths::RepositoryPathRequest
forge_protocol::paths::RepositoryPathError
forge_project::paths::FileSystemObjectId
forge_project::paths::RepositoryBoundary
forge_project::paths::ResolvedRepositoryPath
forge_project::paths::RepositoryBoundaryError
```

## Exercised path and evidence

The operator applied the path patch, ran the complete handed-off command chain,
reported Cargo green and every structural guard passing, and requested the next
slice on `2026-08-01`.

Operator-run validation covered:

```bash
cargo fmt --all
cargo fmt --all -- --check
cargo check --workspace
cargo test -p forge-protocol
cargo test -p forge-project
cargo run -p forge-guards --bin forge-seam-direction -- --root .
cargo run -p forge-guards --bin forge-core-purity -- --root .
cargo run -p forge-guards --bin forge-source-size -- --root . --deny-warnings
cargo test --workspace
git diff --check
git status --short
```

## Negative and failure-path results

Focused fixtures cover normal child resolution, honest repository relocation,
absolute and traversal rejection, alias rejection, repository-ID mismatch, root and
child symlink rejection, root replacement, missing children, non-directory
intermediates, non-UTF8 Unix names, and unexpected filesystem-device rejection.

## Regression locks

Every later path-consuming capability must route through the stable repository
identity and canonical boundary contract. Display paths may not become authority,
lossy text conversion remains forbidden, and all three structural guards plus the
complete workspace suite remain mandatory.

## Explicit non-claims

This closure does not provide project manifests, registration, atomic file writes,
file conflict detection, PTYs, commands, Git behavior, persistence, sessions, LSP,
Nyx transport, or Forge World presentation.
