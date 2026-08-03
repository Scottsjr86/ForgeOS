# FORGEOS-V1-GIT-101 Closure and Specification

Status: `CLOSED`
Capability: Git mutation and linked-worktree primitives
Closed: `2026-08-02`
Source authority: `Forge_OS_V1_base_30.tar`
Source archive SHA-256: `e202929d2b9d502527c1451a68aadf589c08dbe0720ee197d86f165733a0f581`
Git revision: unavailable because the supplied source archive contains no `.git` metadata

## Capability statement

ForgeOS now performs exact-path staging and unstaging, stale-state-bound commit,
explicitly confirmed worktree restore, linked-worktree creation, and clean
non-primary worktree removal through fixed shell-free native Git invocations.

## Public contract

```text
forge_bridge::git_mutation::GitMutationRequest
forge_bridge::git_mutation::NativeGitMutationAdapter
forge_bridge::git_mutation::NativeGitMutationOutput
forge_git::mutation::GitRepositoryMutator
forge_git::mutation::StageRequest
forge_git::mutation::UnstageRequest
forge_git::mutation::RestoreRequest
forge_git::mutation::CommitRequest
forge_git::mutation::CreateWorktreeRequest
forge_git::mutation::RemoveWorktreeRequest
forge_git::mutation::GitMutationOutcome
forge_git::mutation::GitMutationError
```

## Accepted operator evidence

```text
FORGE_SEAM_DIRECTION_SUMMARY status=PASS packages=12 routes=42 forbidden=0 policy=exact-reviewed-subsystem-reachability-v1
FORGE_CORE_PURITY_SUMMARY status=PASS packages=2 allowed=2 forbidden=0 policy=exact-reviewed-production-graph-v1
FORGE_SOURCE_SIZE_SUMMARY status=PASS modules=95 pass=95 warn=0 fail=0 warnings_denied=true
CARGO_TEST_SUMMARY status=PASS suites=54 passed=226 failed=0 ignored=0 measured=0 filtered_out=0
CI RESULT: PASS
```

CI remains limited to behavioral tests, golden locks, and structural guards. It
contains no documentation verification, Git-state check, or formatting gate.

## Negative and failure-path results

Fixtures cover duplicate and changed path state, stale HEAD and index state,
confirmed restore, hook suppression, linked-worktree target collision, dirty and
primary worktree rejection, repository mismatch, missing Git, native pathspec
failure, and raw non-UTF8 paths. Rejected operations preserve unrelated source and
refs.

## Explicit non-claims

This closure does not apply external patches, merge, rebase, cherry-pick, mutate
Git configuration, force-remove worktrees, persist project state, connect Nyx, or
present source-control state through Forge World.
