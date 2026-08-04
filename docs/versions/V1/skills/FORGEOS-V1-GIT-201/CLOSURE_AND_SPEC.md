# FORGEOS-V1-GIT-201 Closure and Specification

Status: `ACTIVE_OPERATOR_VALIDATION_PENDING`
Capability: Safe Git mutation and isolated worktree control
Active slice: `FORGEOS-V1-GIT-201-SLICE-001`
Source authority: `Forge_OS_V1_base_47.tar`

## Capability statement

ForgeOS now binds the existing native Git mutation primitives to one registered
project and one exact consistency-checked inspection selected by the user. Empty,
duplicate, foreign, stale, or inapplicable selections fail before mutation. Every
successful operation returns its native outcome plus a new accepted project Git
snapshot.

## Public contract

```text
forge_app::composition::git_mutation_workspace::ProjectGitMutationWorkspace
forge_app::composition::git_mutation_workspace::ProjectGitMutationResult
forge_app::composition::git_mutation_workspace::ProjectGitMutationWorkspaceError
```

The composition surface delegates native behavior to:

```text
forge_git::mutation::GitRepositoryMutator
forge_git::mutation::StageRequest
forge_git::mutation::UnstageRequest
forge_git::mutation::RestoreRequest
forge_git::mutation::CommitRequest
forge_git::mutation::CreateWorktreeRequest
forge_git::mutation::RemoveWorktreeRequest
```

## Intended behavior

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

## Operator validation still required

Run the canonical behavior-only CI entrypoint:

```bash
python3 scripts/run_ci.py
```

The skill remains active until that command passes on the operator host.

## Explicit non-claims

This slice does not provide arbitrary pathspecs, shell execution, broad reset or
clean, force operations, merge, rebase, history rewriting, durable mutation history,
version-bound validation receipts, Nyx repository tools, or Forge World Git UI.
