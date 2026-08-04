# FORGEOS-V1-GIT-200 Closure and Specification

Status: `ACTIVE_OPERATOR_VALIDATION_PENDING`
Capability: Real Git status, branch, revision, and diff inspection
Active slice: `FORGEOS-V1-GIT-200-SLICE-001`
Source authority: `Forge_OS_V1_base_46.tar`

## Capability statement

ForgeOS now binds fixed native Git inspection to one registered project repository.
One accepted view contains exact branch and revision truth, typed porcelain-v2 status,
exact worktree and staged raw-diff records, exact binary-safe patch bytes, and one
stable SHA-256 identity. The complete surface is captured twice and rejected when
the repository changes between passes, preventing a torn UI snapshot.

## Public contract

```text
forge_git::inspection::GitInspectionSnapshot
forge_git::inspection::GitRepositoryInspectionError
forge_git::repository::GitRepositoryInspector::inspect_consistent
forge_app::composition::git_workspace::ProjectGitSnapshot
forge_app::composition::git_workspace::ProjectGitWorkspace
forge_app::composition::git_workspace::ProjectGitWorkspaceError
```

## Intended behavior

- only the registered repository root may be inspected;
- project and repository identities remain attached to every accepted view;
- branch, detached state, unborn state, and exact native revision remain distinct;
- staged, unstaged, untracked, rename/copy, deletion, and conflict meaning survives;
- worktree and staged diffs remain distinct and preserve exact patch bytes;
- repeated unchanged views produce one deterministic inspection identity;
- a repository mutation during multi-command capture fails closed;
- no inspection operation changes HEAD, index, worktree, branches, or worktrees.

## Regression locks

```text
crates/forge-git/tests/read_only_git.rs
crates/forge-git/tests/consistent_inspection.rs
crates/forge-app/tests/git_workspace.rs
python3 scripts/run_ci.py
```

## Operator validation still required

Run the canonical behavior-only CI entrypoint:

```bash
python3 scripts/run_ci.py
```

The skill remains active until that command passes on the operator host.

## Explicit non-claims

This slice does not stage, unstage, restore, commit, create or remove worktrees,
cache Git state across restart, create build/test verification receipts, expose Nyx
repository tools, or render the Forge World source-control UI.
