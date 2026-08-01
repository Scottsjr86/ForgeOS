# FORGEOS-V1-GUARD-001 Closure and Specification

Status: `CLOSED`
Capability: Forge Core transitive dependency-purity guard
Closed: `2026-08-01`
Source authority: `Forge_OS_V1_base_13.tar`
Source archive SHA-256: `b79f1bf22dbcf800f26f60e8d9a3a0cf1cb5ac754b3269efa41a09810aecbe2b`
Git revision: unavailable because the supplied source archive contains no `.git` metadata

## Capability statement

ForgeOS now provides one executable `forge-core-purity` verifier that inspects the
real Cargo normal and build dependency graph rooted at `forge-core` and rejects
all packages outside the exact reviewed pure graph.

The closed V1 graph is:

```text
forge-core
forge-protocol
```

The policy is default-deny. An unknown package fails even when its name looks
harmless, and a generically named adapter cannot hide an effectful transitive
package.

## Public contract

The accepted source exposes:

```text
forge_guards::core_purity::inspect_core_dependencies
forge_guards::core_purity::inspect_core_dependencies_with_cargo
forge_guards::core_purity::PurityReport
forge_guards::core_purity::PackageViolation
forge-core-purity --root <repository-root>
```

Stable executable records are emitted as:

```text
FORGE_CORE_PURITY_PACKAGE status=<ALLOWED|FORBIDDEN> package=<cargo-package-name>
FORGE_CORE_PURITY_SUMMARY status=<PASS|FAIL> packages=<n> allowed=<n> forbidden=<n> policy=exact-reviewed-production-graph-v1
```

Package records are sorted deterministically.

## Exercised path and evidence

The operator applied the guard patch, ran the complete handed-off command chain,
reported every command green, supplied the exact real-workspace guard summaries,
and requested the next slice on `2026-08-01`.

Operator-run validation covered:

```bash
cargo fmt --all -- --check
cargo check --workspace
cargo test -p forge-guards
cargo run -p forge-guards --bin forge-core-purity -- --root .
cargo run -p forge-guards --bin forge-source-size -- --root . --deny-warnings
cargo test --workspace
git diff --check
git status --short
```

The reported real-workspace summaries were:

```text
FORGE_CORE_PURITY_SUMMARY status=PASS packages=2 allowed=2 forbidden=0 policy=exact-reviewed-production-graph-v1
FORGE_SOURCE_SIZE_SUMMARY status=PASS modules=51 pass=51 warn=0 fail=0 warnings_denied=true
```

Assistant-side patch evidence before handoff included manifest parsing, a static
mirror of the exact two-package graph, direct and transitive rejection review,
source-size regression review, authority-state coherence, `git diff --check`,
fresh-base apply checking, and independent applied-tree comparison.

## Negative and failure-path results

Focused executable fixtures reject representative direct dependencies for:

```text
effect runtime
Forge World or UI
Nyx transport
Git
PTY or terminal
filesystem adapter
LSP
DAP
network provider
session runtime
```

A transitive fixture also rejects both nodes in:

```text
forge-core -> generic-adapter -> forge-world
```

The verifier rejects invalid repository roots, missing workspace manifests,
Cargo invocation failure, Cargo graph failure, malformed Cargo output, a missing
`forge-core` graph root, and every package not named in the exact reviewed graph.

## Regression locks

Every later dependency change must run:

```bash
cargo run -p forge-guards --bin forge-core-purity -- --root .
```

Any new normal or build package reachable from `forge-core` invalidates this
closure until the package is intentionally reviewed, the policy is updated, the
negative fixtures remain effective, and the complete acceptance path is rerun.

## Explicit non-claims

This closure does not claim stable protocol IDs, versioned envelopes, persistence,
path safety, process execution, terminal behavior, Git behavior, editor behavior,
Nyx operation, Forge World behavior, or any user-facing development journey.

The guard classifies dependency purity only. It does not prove that accepted code
is logically correct, deterministic, secure, or complete.
