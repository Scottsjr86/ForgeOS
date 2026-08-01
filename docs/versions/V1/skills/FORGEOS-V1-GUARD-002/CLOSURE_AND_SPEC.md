# FORGEOS-V1-GUARD-002 Closure and Specification

Status: `CLOSED`
Capability: cross-subsystem dependency-direction guard
Closed: `2026-08-01`
Source authority: `Forge_OS_V1_base_15.tar`
Source archive SHA-256: `c36b73a782595f263bf8f9cc42495131885373e9b5c51f7849175524e1d63dbf`
Git revision: unavailable because the supplied source archive contains no `.git` metadata

## Capability statement

ForgeOS now provides one executable `forge-seam-direction` verifier that inspects
the real Cargo normal and build dependency graph for every reviewed V1 authority
package and compares all ForgeOS package reachability against an exact reviewed
matrix.

The closed policy contains twelve reviewed packages and forty-two legal reachable
relations. It is default-deny for unknown ForgeOS workspace packages, missing
reviewed packages, backward authority paths, and transitive adapter smuggling.

## Public contract

The accepted source exposes:

```text
forge_guards::seams::inspect_seam_directions
forge_guards::seams::inspect_seam_directions_with_cargo
forge_guards::seams::SeamReport
forge_guards::seams::SeamRelation
forge_guards::seams::SeamViolation
forge-seam-direction --root <repository-root>
```

Stable executable records are emitted as:

```text
FORGE_SEAM_DIRECTION_PACKAGE status=<ALLOWED|FORBIDDEN> package=<cargo-package-name>
FORGE_SEAM_DIRECTION_ROUTE status=<ALLOWED|FORBIDDEN> root=<package> target=<package>
FORGE_SEAM_DIRECTION_SUMMARY status=<PASS|FAIL> packages=<n> routes=<n> forbidden=<n> policy=exact-reviewed-subsystem-reachability-v1
```

Package and route records are sorted deterministically.

## Exercised path and evidence

The operator applied the seam-direction patch, ran the complete handed-off command
chain, reported all commands green, supplied the exact real-workspace summaries,
and requested continuation on `2026-08-01`.

Operator-run validation covered:

```bash
cargo fmt --all -- --check
cargo check --workspace
cargo test -p forge-guards
cargo run -p forge-guards --bin forge-seam-direction -- --root .
cargo run -p forge-guards --bin forge-core-purity -- --root .
cargo run -p forge-guards --bin forge-source-size -- --root . --deny-warnings
cargo test --workspace
git diff --check
git status --short
```

The reported real-workspace summaries were:

```text
FORGE_SEAM_DIRECTION_SUMMARY status=PASS packages=12 routes=42 forbidden=0 policy=exact-reviewed-subsystem-reachability-v1
FORGE_CORE_PURITY_SUMMARY status=PASS packages=2 allowed=2 forbidden=0 policy=exact-reviewed-production-graph-v1
FORGE_SOURCE_SIZE_SUMMARY status=PASS modules=56 pass=56 warn=0 fail=0 warnings_denied=true
```

Assistant-side patch evidence before handoff included TOML parsing, a static mirror
of the twelve-package and forty-two-route graph, source-size review, authority-state
coherence, `git diff --check`, fresh-base apply checking, and independent applied-
tree comparison.

## Negative and failure-path results

Focused executable fixtures reject representative backward relations including:

```text
forge-world -> forge-project
forge-world -> forge-bridge
forge-bridge -> forge-world
forge-project -> forge-world
forge-session -> forge-world
forge-nyx-client -> forge-project
```

A transitive fixture rejects:

```text
forge-bridge -> generic-adapter -> forge-world
```

The verifier also rejects unknown ForgeOS workspace packages, missing reviewed
packages, invalid repository roots, missing workspace manifests, Cargo invocation
failure, Cargo graph failure, malformed Cargo output, and duplicate workspace
package identities.

## Regression locks

Every later crate-graph change must run:

```bash
cargo run -p forge-guards --bin forge-seam-direction -- --root .
cargo run -p forge-guards --bin forge-core-purity -- --root .
```

Any new ForgeOS workspace package or changed reachable subsystem relation
invalidates this closure until the matrix is intentionally reviewed, negative
fixtures remain effective, and the complete acceptance path is rerun.

## Explicit non-claims

This closure does not prove persistence, path safety, process execution, terminal
behavior, Git behavior, editor behavior, Nyx compatibility, session behavior, or
Forge World correctness.

The guard proves package reachability direction only. It does not inspect runtime
calls, semantic correctness, effect behavior hidden inside an accepted package, or
user-facing workflow completion.
