# FORGEOS-V1-LSP-100 Closure and Specification

Status: `CLOSED`
Capability: Rust Analyzer process and version-safe JSON-RPC adapter
Closed: `2026-08-02`
Source authority: `Forge_OS_V1_base_24.tar`
Source archive SHA-256: `54ec26d8a66f4d916b81519faf8c107f9cdcc08a8d3dcbeee33245f92a5f5e63`
Git revision: unavailable because the supplied source archive contains no `.git` metadata

## Capability statement

ForgeOS now starts one configured Rust Analyzer-compatible process for one project,
uses framed JSON-RPC, negotiates capabilities, synchronizes exact editor generations,
accepts diagnostics only for tracked document versions, answers the reviewed server
requests, and restarts with a new process identity without retaining stale documents.

## Public contract

```text
forge_bridge::lsp::RustAnalyzerConfig
forge_bridge::lsp::RustAnalyzerClient
forge_bridge::lsp::LspDocument
forge_bridge::lsp::DocumentVersion
forge_bridge::lsp::PublishedDiagnostics
forge_bridge::lsp::LspError
forge_editor::language::RustLanguageDocument
forge_editor::language::PendingLanguageUpdate
forge_editor::language::LanguageDocumentError
```

## Accepted operator evidence

```text
FORGE_SEAM_DIRECTION_SUMMARY status=PASS packages=12 routes=42 forbidden=0 policy=exact-reviewed-subsystem-reachability-v1
FORGE_CORE_PURITY_SUMMARY status=PASS packages=2 allowed=2 forbidden=0 policy=exact-reviewed-production-graph-v1
FORGE_SOURCE_SIZE_SUMMARY status=PASS modules=80 pass=80 warn=0 fail=0 warnings_denied=true
CARGO_TEST_SUMMARY status=PASS suites=47 passed=142 failed=0 ignored=0 measured=0 filtered_out=0
CI RESULT: PASS
```

CI remains limited to behavioral tests, golden locks, and structural guards. It
contains no documentation verification, Git-state check, or formatting gate.

## Negative and failure-path results

Fixtures cover stale and foreign diagnostics, failed notification writes, malformed
framing, missing executable, unexpected server exit, unsupported capabilities,
failed restart, UTF-16 range handling, and non-UTF8 plain-text degradation.

## Explicit non-claims

This closure does not save files, render diagnostics, provide completion or
navigation UI, run registered commands, manage PTYs, inspect Git, connect Nyx,
manage a ForgeOS session, perform recovery, or render Forge World UI.
