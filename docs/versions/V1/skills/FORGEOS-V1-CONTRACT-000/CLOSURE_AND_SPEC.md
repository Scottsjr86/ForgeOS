# FORGEOS-V1-CONTRACT-000 Closure and Specification

Status: `CLOSED`
Capability: Stable typed identities and deterministic versioned protocol envelopes
Closed: `2026-08-01`
Source authority: `Forge_OS_V1_base_14.tar`
Source archive SHA-256: `ce6096027606dd1e2ca831bdaa2e76604e8b1708cf6938c681817f65e41ece95`
Git revision: unavailable because the supplied source archive contains no `.git` metadata

## Capability statement

ForgeOS now provides ten distinct stable identity types, typed protocol failures,
event records, and deterministic V1 request, result, and error envelopes through
the public `forge-protocol` contract. Forge Core consumes the same locked public
bytes without adding another protocol implementation.

Canonical identity is an opaque 16-byte value with an exact 32-character lowercase
hexadecimal representation. Display names, paths, timestamps, indexes, and model
wording do not participate in identity.

## Public contract

The accepted source exposes typed:

```text
ProjectId
RepositoryId
ProcessId
TerminalId
CommandId
SessionId
TaskId
PatchId
ResultId
EventId
ProtocolError
RequestEnvelope
ResultEnvelope
ErrorEnvelope
EventRecord
```

The V1 envelope kind and schema version are encoded explicitly. Decoders reject
unknown versions, malformed bytes, invalid canonical identity text, and trailing or
truncated payload data instead of guessing.

## Exercised path and evidence

The operator applied the contract patch, ran the complete handed-off command chain,
reported every command green, supplied the exact real-workspace guard summaries,
and explicitly requested the next slice on `2026-08-01`.

Operator-run validation covered:

```bash
cargo fmt --all
cargo check --workspace
cargo test -p forge-protocol
cargo test -p forge-core
cargo run -p forge-guards --bin forge-core-purity -- --root .
cargo run -p forge-guards --bin forge-source-size -- --root . --deny-warnings
cargo test --workspace
git diff --check
git status --short
```

The reported real-workspace summaries were:

```text
FORGE_CORE_PURITY_SUMMARY status=PASS packages=2 allowed=2 forbidden=0 policy=exact-reviewed-production-graph-v1
FORGE_SOURCE_SIZE_SUMMARY status=PASS modules=54 pass=54 warn=0 fail=0 warnings_denied=true
```

Assistant-side patch evidence before handoff included exact wire-byte mirroring,
manifest parsing, typed-identity inspection, malformed and unknown-version fixture
review, Core graph review, source-size review, `git diff --check`, fresh-base apply
checking, and independent applied-tree comparison.

## Negative and failure-path results

Executable fixtures prove that:

- uppercase, short, long, and malformed canonical identity text is rejected;
- duplicate typed identities fail deterministically;
- request, result, and error envelopes round-trip exact locked V1 bytes;
- unknown schema versions fail closed;
- malformed, truncated, trailing, and wrong-kind bytes are rejected;
- typed protocol errors survive an error-envelope round trip;
- Forge Core consumes the same public bytes rather than inventing a parallel format.

## Regression locks

Every later protocol change must preserve the published V1 meanings and rerun:

```bash
cargo test -p forge-protocol
cargo test -p forge-core
cargo run -p forge-guards --bin forge-core-purity -- --root .
cargo run -p forge-guards --bin forge-source-size -- --root . --deny-warnings
```

Wire-byte drift, identity aliasing, newly accepted malformed input, or a new package
in the Forge Core graph invalidates this closure until explicitly reviewed and
reproved.

## Explicit non-claims

This closure does not claim persistence, canonical path boundaries, process
execution, artifact hashing, cross-subsystem seam enforcement, terminal behavior,
Git behavior, editor behavior, Nyx operation, Forge World behavior, session startup,
packaging, or a user-facing development journey.
