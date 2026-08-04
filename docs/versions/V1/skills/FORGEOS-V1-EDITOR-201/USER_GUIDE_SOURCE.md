# FORGEOS-V1-EDITOR-201 User Guide Source

Status: `CLOSED`

## What this capability does

ForgeOS can attach real Rust syntax and Rust Analyzer intelligence to an open Rust
buffer without allowing either mechanism to invent editor state. The current buffer
generation controls which diagnostics, definitions, and completion results are valid.

## Available language features

- current Tree-sitter syntax spans and syntax issues;
- version-bound Rust Analyzer diagnostics;
- go-to-definition locations inside the active workspace;
- basic completion items;
- workspace symbol search;
- explicit degraded status when Rust Analyzer is unavailable;
- later resynchronization to the current buffer without losing edits.

## Safety behavior

Results from another project, repository, file, or document generation are rejected.
Definition and symbol locations outside the active workspace are rejected. Rust
Analyzer failure never makes the editor buffer read-only and does not erase current
Tree-sitter syntax state.

## What comes later

Rendered diagnostics, editor navigation controls, terminal-backed build and test
results, project restoration through the full cockpit, refactoring actions, formatting,
and the complete real Rust authoring journey remain later capabilities.
