# FORGEOS-V1-PARSER-100 Closure and Specification

Status: `CLOSED`
Capability: incremental Tree-sitter Rust parsing adapter
Closed: `2026-08-02`
Source authority: `Forge_OS_V1_base_23.tar`
Source archive SHA-256: `1b999748b0ed4f3873fe209eae24c3ca35dec0b593e5e76fb0270be4da5b3ec5`
Git revision: unavailable because the supplied source archive contains no `.git` metadata

## Capability statement

ForgeOS now parses exact Rust editor generations with Tree-sitter, keeps parser state
separate from source-byte ownership, exposes named syntax spans and parser issues,
performs incremental updates from exact previous bytes, reports changed ranges, and
rejects stale, cross-buffer, cross-document, or mismatched-source parser state.

## Public contract

```text
forge_bridge::parsing::RustSyntaxParser
forge_bridge::parsing::RustSyntaxSnapshot
forge_bridge::parsing::SyntaxSpan
forge_bridge::parsing::SyntaxIssue
forge_bridge::parsing::SourceRange
forge_editor::parsing::ParsedBuffer
forge_editor::parsing::BufferParseError
```

## Accepted operator evidence

```text
FORGE_SEAM_DIRECTION_SUMMARY status=PASS packages=12 routes=42 forbidden=0 policy=exact-reviewed-subsystem-reachability-v1
FORGE_CORE_PURITY_SUMMARY status=PASS packages=2 allowed=2 forbidden=0 policy=exact-reviewed-production-graph-v1
FORGE_SOURCE_SIZE_SUMMARY status=PASS modules=75 pass=75 warn=0 fail=0 warnings_denied=true
CARGO_TEST_SUMMARY status=PASS suites=46 passed=125 failed=0 ignored=0 measured=0 filtered_out=0
CI RESULT: PASS
```

CI remains limited to behavioral tests, golden locks, and structural guards. It
contains no documentation verification, Git-state check, or formatting gate.

## Negative and failure-path results

Fixtures cover invalid Rust, stale snapshots, non-advancing versions, cross-buffer
and cross-document use, mismatched previous bytes, non-UTF8 source, and transactional
failure that leaves the previously committed parser state intact.

## Explicit non-claims

This closure does not start Rust Analyzer, speak JSON-RPC, synthesize diagnostics,
provide completion or navigation, save files, search repositories, run commands,
inspect Git, manage sessions, connect Nyx, or render Forge World UI.
