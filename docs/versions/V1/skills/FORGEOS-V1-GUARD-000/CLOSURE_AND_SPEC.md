# FORGEOS-V1-GUARD-000 Closure and Specification

Status: `CLOSED`
Capability: Authored Rust source module-size verifier
Closed: `2026-08-01`
Source authority: `Forge_OS_V1_base_12.tar`
Source archive SHA-256: `284ec60229eb36586123de096bc9331415265c7717a7da22f1249b850a52ae1d`
Git revision: unavailable because the supplied source archive contains no `.git` metadata

## Capability statement

ForgeOS now provides one executable `forge-source-size` verifier that discovers
all authored Rust modules in stable relative-path order, counts physical lines,
and applies the exact V1 boundaries:

```text
0-1000     PASS
1001-1200  WARN
1201+      FAIL
```

The verifier has no user-controlled ignore or allowlist file. It excludes only
fixed generated, vendored, third-party, build-output, and version-control source
classes, skips files bearing fixed generated-source markers, and rejects symlinks
inside the scanned authored tree instead of silently omitting them.

## Public contract

The accepted source exposes:

```text
forge_guards::source_size::scan_authored_rust
forge_guards::source_size::classify_line_count
forge-source-size --root <path>
forge-source-size --root <path> --deny-warnings
```

Stable executable records are emitted as:

```text
FORGE_SOURCE_SIZE_MODULE status=<PASS|WARN|FAIL> lines=<count> path=<relative-path>
FORGE_SOURCE_SIZE_SUMMARY status=<PASS|WARN|FAIL> modules=<n> pass=<n> warn=<n> fail=<n> warnings_denied=<bool>
```

`--deny-warnings` preserves the `WARN` classification while returning a failing
exit status, allowing later source skills to enforce the regression rule that no
module above 1000 physical lines may remain at closure.

## Exercised path and evidence

The operator applied the guard patch, ran the handed-off command chain, reported
`all green`, and requested the next slice on `2026-08-01`. That report is the user
acceptance record for the real workspace scan and the five exact boundary fixtures.

Operator-run validation covered:

```bash
cargo fmt --all -- --check
cargo check --workspace
cargo test -p forge-guards
cargo run -p forge-guards --bin forge-source-size -- --root . --deny-warnings
cargo test --workspace
git diff --check
git status --short
```

Assistant-side patch evidence recorded before handoff:

```text
boundary classification mirror: pass at 500 and 1000
boundary classification mirror: warn at 1001 and 1200
boundary classification mirror: fail at 1201
real authored modules inspected statically: 49
real workspace warnings above 1000 lines: none
git diff --check: pass
git apply --check against fresh extraction: pass
independent applied-tree comparison: pass
```

## Negative and failure-path results

- A 1201-line authored module produces `FAIL` and a failing exit status.
- A 1001-line warning produces a failing exit status under `--deny-warnings`
  without being reclassified as `FAIL`.
- Generated, vendored, third-party, target, and VCS source are not counted as
  authored modules.
- A source symlink is rejected rather than followed or silently skipped.
- Unknown or duplicate command options return usage failure.

## Supported behavior

- Every authored `.rs` module below the selected root is inspected through fixed
  policy.
- Results and paths are emitted deterministically.
- Later source slices can hard-gate closure on zero warnings and zero failures.
- Physical-line boundary behavior is locked by executable fixtures.

## Explicit non-claims

This closure does not prove Forge Core purity, cross-subsystem seam direction,
stable protocol identities, persistence, process execution, terminal, Git, editor,
Nyx, Forge World, session, packaging, or any user-facing development journey.

## V1 scope limits

The guard measures module size and source discovery only. It does not evaluate code
quality, architectural correctness, behavior ownership, test adequacy, or whether a
small module is well designed.
