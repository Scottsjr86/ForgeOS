# FORGEOS-V1-EDITOR-201 Closure and Specification

Status: `CLOSED`
Capability: Rust syntax and language-intelligence integration
Active slice: `FORGEOS-V1-EDITOR-201-SLICE-001`
Source authority: `Forge_OS_V1_base_42.tar`
Source archive SHA-256: `8f80a90b5b0ae876a6d7c346621502d0de72163d5402613fc88ee24e75c6535f`

## Capability statement

ForgeOS now binds Tree-sitter syntax state and Rust Analyzer document state to the
same exact editor buffer generation. Diagnostics, definitions, completion results,
and workspace symbols remain native Rust Analyzer results, while syntax parsing
continues independently if the language server is missing or fails.

## Public contract

```text
forge_bridge::lsp::RustAnalyzerClient::request_definition
forge_bridge::lsp::RustAnalyzerClient::request_completion
forge_bridge::lsp::RustAnalyzerClient::request_workspace_symbols
forge_bridge::lsp::DefinitionResult
forge_bridge::lsp::CompletionResult
forge_bridge::lsp::WorkspaceSymbolResult
forge_editor::intelligence::RustBufferIntelligence
forge_editor::intelligence::RustIntelligenceStatus
forge_editor::parsing::PendingBufferParse
```

## Intended behavior

- Tree-sitter and Rust Analyzer remain separate real mechanisms;
- parser and LSP updates are prepared against one exact editor generation;
- committed language state advances only after the exact LSP notification is sent;
- parser state can advance alone when Rust Analyzer is unavailable;
- degraded language state remains explicit and can later resynchronize to the
  current buffer generation;
- diagnostics, definitions, and completion results are rejected when their project,
  repository, path, or document version is stale;
- definition and workspace-symbol locations outside the configured workspace fail
  closed;
- completion and symbol results receive deterministic ordering;
- Rust Analyzer restart generation remains attached to workspace-symbol results;
- editing and syntax parsing remain functional when Rust Analyzer cannot start.

## Regression locks

```text
crates/forge-editor/tests/parser_state.rs
crates/forge-editor/tests/language_server.rs
crates/forge-editor/tests/rust_features.rs
python3 scripts/run_ci.py
```

The behavioral matrix covers exact-generation diagnostics, definition navigation,
basic completion, project symbol search, stale-result rejection, out-of-workspace
location rejection, atomic parser/LSP update, language-server degradation, and later
language resynchronization.

## Closure evidence

The operator ran the canonical behavior-only CI entrypoint with 66 suites, 308 tests
passed, 0 failed, 2 ignored, and all three structural guards green.

The operator also ran the ignored real Rust Analyzer witness against:

```text
/home/devmonster/.vscode/extensions/rust-lang.rust-analyzer-0.3.2981-linux-x64/server/rust-analyzer
```

The witness passed one real temporary Cargo workspace with native diagnostics,
definition navigation, completion, and workspace symbol search.

## Explicit non-claims

This skill does not provide rendered editor widgets, terminal execution, build/test
records, Git behavior, Nyx behavior, refactoring actions, semantic tokens, code
formatting, rename, references, inlay hints, or background indexing policy. The Tier
3 real Rust authoring journey remains `FORGEOS-V1-CODE-300`.
