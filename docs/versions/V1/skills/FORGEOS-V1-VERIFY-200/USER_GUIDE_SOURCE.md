# FORGEOS-V1-VERIFY-200 User Guide Source

Status: `ACTIVE_OPERATOR_VALIDATION_PENDING`

## What this capability does

ForgeOS can run a registered validation command and retain exactly what was run,
which project source state it started from, which state existed afterward, how the
process ended, and which exact stdout and stderr bytes belong to the result.

## Current versus historical truth

A green result is current only while the repository still matches the exact
post-run revision and dirty-state identity. Editing, staging, committing, restoring,
or otherwise changing the repository makes the result historical instead of current.

Failed, cancelled, timed-out, and execution-failed runs remain visible as truthful
records but never satisfy current validation.

## History safety

Verification history is append-only and canonically persisted. Restored records must
belong to the same project and repository. ForgeOS does not rewrite an older record
to match newer source.
