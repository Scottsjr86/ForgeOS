# FORGEOS-V1-GIT-201 Closure and Specification

Status: `CLOSED`
Capability: Safe Git mutation and isolated worktree control
Closed slice: `FORGEOS-V1-GIT-201-SLICE-001`
Source authority: `Forge_OS_V1_base_48.tar`

## Closure evidence

Operator behavior-only CI passed with:

```text
FORGE_SEAM_DIRECTION_SUMMARY status=PASS packages=12 routes=42 forbidden=0
FORGE_CORE_PURITY_SUMMARY status=PASS packages=2 allowed=2 forbidden=0
FORGE_SOURCE_SIZE_SUMMARY status=PASS modules=138 pass=138 warn=0 fail=0
CARGO_TEST_SUMMARY status=PASS suites=73 passed=345 failed=0 ignored=2
CI RESULT: PASS
```

## Capability statement

ForgeOS binds the existing native Git mutation primitives to one registered project
and one exact consistency-checked inspection selected by the user. Empty,
duplicate, foreign, stale, or inapplicable selections fail before mutation. Every
successful operation returns its native outcome plus a new accepted project Git
snapshot.

## Public contract

```text
forge_app::composition::git_mutation_workspace::ProjectGitMutationWorkspace
forge_app::composition::git_mutation_workspace::ProjectGitMutationResult
forge_app::composition::git_mutation_workspace::ProjectGitMutationWorkspaceError
```

## Proved behavior

- project, repository, and repository-object identity are revalidated before use;
- every mutation begins from one accepted `ProjectGitSnapshot`;
- a changed inspection identity rejects the selection as stale;
- stage and unstage accept only exact paths present in that snapshot;
- confirmed restore accepts only exact tracked worktree changes;
- commit binds the exact selected HEAD and staged-patch identity;
- linked worktrees start from the selected exact HEAD on one validated new branch;
- linked-worktree removal accepts only an exact registered clean worktree;
- non-UTF8 path bytes remain exact through native Git;
- successful operations return a new consistency-checked project snapshot;
- unselected files, branches, and worktrees remain unchanged.

## Regression locks

```text
crates/forge-git/tests/git_mutation.rs
crates/forge-app/tests/git_mutation_workspace.rs
python3 scripts/run_ci.py
```

## Explicit non-claims

This capability does not provide arbitrary pathspecs, shell execution, broad reset
or clean, force operations, merge, rebase, history rewriting, durable mutation
history, version-bound validation receipts, Nyx repository tools, or Forge World
Git UI.
