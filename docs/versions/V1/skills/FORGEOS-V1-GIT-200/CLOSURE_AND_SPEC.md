# FORGEOS-V1-GIT-200 Closure and Specification

Status: `CLOSED`
Capability: Real Git status, branch, revision, and diff inspection
Active slice: `FORGEOS-V1-GIT-200-SLICE-001`
Source authority: `Forge_OS_V1_base_47.tar`

## Capability statement

ForgeOS binds fixed native Git inspection to one registered project repository.
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

## Closure evidence

Operator behavior-only CI passed:

```text
CARGO_TEST_SUMMARY status=PASS suites=72 passed=336 failed=0 ignored=2 measured=0 filtered_out=0
CI RESULT: PASS
```

All three structural guards passed with zero forbidden seams, zero core-purity
violations, and 136 source modules under the enforced size limit.

## Explicit non-claims

This slice does not stage, unstage, restore, commit, create or remove worktrees,
cache Git state across restart, create build/test verification receipts, expose Nyx
repository tools, or render the Forge World source-control UI.
