# FORGEOS-V1-GIT-201 User Guide Source

Status: `CLOSED`

## What this capability does

ForgeOS can take the exact Git view currently shown for a registered project and
perform explicit stage, unstage, commit, confirmed restore, linked-worktree create,
and clean linked-worktree removal operations.

## Selection safety

A mutation is not authorized by a path string alone. The selected path must exist in
the accepted Git snapshot, the project and repository identities must still match,
and the complete inspection identity must still be current. If another process
changes the repository after selection, ForgeOS rejects the action and requires a
fresh view.

## Destructive-action boundaries

Restore requires an explicit confirmation and accepts only tracked worktree changes.
Commit uses the exact staged patch and HEAD visible in the selected snapshot.
Worktree removal accepts only a registered non-primary worktree whose HEAD still
matches and whose worktree is clean.

## Result

Every successful action returns the native Git outcome and a new
consistency-checked project snapshot. ForgeOS does not pretend the old view remains
current after mutation.
