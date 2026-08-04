# FORGEOS-V1-GIT-200 User Guide Source

Status: `CLOSED`

## What this capability does

ForgeOS can inspect the Git repository registered to the active project and return
one exact view of branch, revision, status, unstaged changes, and staged changes.
Paths and patch bytes come from native Git machine output rather than lossy human
text parsing.

## What the view shows

The view distinguishes staged, unstaged, untracked, renamed, deleted, and conflicted
paths. Worktree and staged patches remain separate. An unchanged repository produces
the same stable inspection identity on repeated reads.

## Safety behavior

ForgeOS reads the complete Git surface twice. If another process changes the
repository between those passes, the mixed result is rejected rather than displayed
as current truth. A foreign repository identity, replaced root object, repository
subdirectory, malformed Git output, or native Git failure is also explicit.

## What comes next

`FORGEOS-V1-GIT-201` binds explicit selections from this accepted view to safe
stage, unstage, commit, confirmed restore, and isolated worktree operations.
