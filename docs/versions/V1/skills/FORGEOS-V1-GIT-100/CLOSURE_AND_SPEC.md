# FORGEOS-V1-GIT-100 Closure and Specification

Status: `CLOSED`
Capability: Read-only native Git inspection
Closed: `2026-08-02`
Source authority: `Forge_OS_V1_base_28.tar`
Source archive SHA-256: `5a10bd2a04171232e068ad3e0547068c666cb648a7f8f95e96d39bb4009abc75`
Git revision: unavailable because the supplied source archive contains no `.git` metadata

## Capability statement

ForgeOS now binds one stable `RepositoryId` to one verified native Git worktree
root and inspects attached, detached, and unborn HEAD state, porcelain-v2 status,
porcelain worktree records, typed raw diffs, and exact binary-safe patch bytes
without exposing a mutation method.

## Public contract

```text
forge_bridge::git::NativeGitAdapter
forge_bridge::git::GitReadRequest
forge_bridge::git::GitDiffInvocation
forge_bridge::git::NativeGitOutput
forge_git::repository::GitRepositoryInspector
forge_git::repository::GitInspectError
forge_git::status::GitHead
forge_git::status::GitStatusSnapshot
forge_git::status::GitStatusEntry
forge_git::worktree::GitWorktreeSnapshot
forge_git::diff::DiffScope
forge_git::diff::GitDiff
forge_git::types::GitObjectId
forge_git::types::GitPath
```

## Accepted operator evidence

```text
FORGE_SEAM_DIRECTION_SUMMARY status=PASS packages=12 routes=42 forbidden=0 policy=exact-reviewed-subsystem-reachability-v1
FORGE_CORE_PURITY_SUMMARY status=PASS packages=2 allowed=2 forbidden=0 policy=exact-reviewed-production-graph-v1
FORGE_SOURCE_SIZE_SUMMARY status=PASS modules=92 pass=92 warn=0 fail=0 warnings_denied=true
CARGO_TEST_SUMMARY status=PASS suites=53 passed=207 failed=0 ignored=0 measured=0 filtered_out=0
CI RESULT: PASS
```

CI remains limited to behavioral tests, golden locks, and structural guards. It
contains no documentation verification, Git-state check, or formatting gate.

## Negative and failure-path results

Fixtures cover option injection, attached, detached, and unborn HEAD states,
staged, unstaged, untracked, rename, linked-worktree, worktree/staged/range diff,
non-UTF8 paths, sanitized environment, malformed native output, non-repository and
subdirectory roots, missing Git, replaced roots, native failures, and read-only
state preservation.

## Explicit non-claims

This closure does not stage, unstage, restore, commit, create or remove worktrees,
apply patches, persist project state, mutate refs, connect Nyx, or present Git state
through Forge World.
