# FORGEOS-V1-EDITOR-200 Closure and Specification

Status: `CLOSED`
Capability: Multi-buffer file editing and atomic save
Active slice: `FORGEOS-V1-EDITOR-200-SLICE-001`
Source authority: `Forge_OS_V1_base_41.tar`
Source archive SHA-256: `3c4e4d22a11a82558cde12c1b88ab7985d5b515eb0b9c6510b3af1d1b98ab67b`

## Capability statement

ForgeOS now has a thin product composition boundary that joins editor-owned buffer
state to project-owned repository file authority. Multiple buffers remain independent,
saves retain exact file object preconditions, external changes become explicit
conflicts, and destructive discard requires a confirmation bound to the current local
content generation.

## Public contract

```text
forge_editor::buffers::DiscardConfirmation
forge_editor::buffers::EditorBuffer::discard_confirmation
forge_editor::buffers::BufferRegistry::remove_discarding
forge_app::composition::editor_workspace::EditorWorkspace
forge_app::composition::editor_workspace::EditorSaveResult
forge_app::composition::editor_workspace::EditorWorkspaceError
```

## Intended behavior

- editor state remains owned by `forge-editor`;
- repository boundary checks and atomic file replacement remain owned by
  `forge-project`;
- `forge-app` composes the two public contracts without duplicating either authority;
- each open buffer retains its exact current `FileExpectation`, including filesystem
  object identity, content hash, and length;
- multiple buffers retain independent bytes, cursor state, dirty state, and save state;
- existing files save only against the exact retained revision;
- missing files create atomically through the existing FILE-100 path;
- a successful save updates only the matching buffer generation and exact revision;
- a later edit remains dirty when an earlier in-flight generation completes;
- external changes become explicit conflicts and external bytes are preserved;
- clean close requires no destructive confirmation;
- dirty or conflicted close remains blocked until the caller supplies a confirmation
  token for the exact current content generation;
- a stale confirmation cannot discard newer edits;
- explicit discard and reopen loads current disk bytes through project file authority;
- unedited files and unrelated buffers remain unchanged.

## Regression locks

```text
crates/forge-editor/tests/buffer_state.rs
crates/forge-app/tests/editor_workspace.rs
crates/forge-project/tests/file_access.rs
python3 scripts/run_ci.py
```

The behavioral matrix covers independent multi-buffer edits, atomic save of one buffer
while another remains dirty, external-change conflict, refresh-before-save conflict,
stale discard rejection, explicit discard/reopen, missing-file creation, clean close,
and preservation of unedited repository bytes.

## Operator validation evidence

The operator ran `python3 scripts/run_ci.py`. All three structural guards passed and Rust CI reported 65 suites, 303 passed, 0 failed, and 1 ignored. The capability is closed.

## Explicit non-claims

This skill does not provide rendered editor tabs, autosave, background indexing,
syntax rendering, diagnostics, completion, definition navigation, terminal execution,
Git behavior, Nyx behavior, or collaborative editing. Rust language-intelligence
integration remains `FORGEOS-V1-EDITOR-201`.
