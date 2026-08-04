# FORGEOS-V1-COMMAND-200 User Guide Source

Status: `ACTIVE_OPERATOR_VALIDATION_PENDING`

## What this capability does

ForgeOS can run registered format, build, test, or custom project commands without
turning them into mutable shell text. Each run uses the exact executable, arguments,
working directory, declared environment, timeout, and cancellation policy stored in
the command definition.

## What the record shows

Each completed run preserves the project, repository, source revision, command ID,
command-definition hash, process ID, exact stdout and stderr bytes, and one truthful
terminal outcome: exited, timed out, cancelled, or failed.

## Safety behavior

A stale command definition, duplicate process identity, foreign repository, missing
declared environment value, symlink escape, or working directory outside the project
scope is rejected before a valid command run can be recorded. Arbitrary shell strings
are not an input to this capability.

## What comes later

Durable history, version-bound build/test receipts, Forge World presentation, and
Nyx approval/checkpoint behavior remain later skills.
