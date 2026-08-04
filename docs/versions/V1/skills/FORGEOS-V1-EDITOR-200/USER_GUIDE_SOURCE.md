# FORGEOS-V1-EDITOR-200 User Guide Source

Status: `ACTIVE_OPERATOR_VALIDATION_PENDING`

## What this capability does

ForgeOS can keep several repository files open at once, preserve independent unsaved
changes, and save one exact buffer generation through the project-owned atomic file
path. Saving one file does not mark another file clean or alter unrelated repository
bytes.

## Save and conflict behavior

- an existing file is saved only when its exact retained repository revision still
  matches;
- a new file is created atomically from a buffer whose disk baseline is missing;
- an external change is reported as a conflict and is never overwritten silently;
- refresh checks current disk identity without replacing local bytes;
- save results apply only to the generation that requested the save;
- durability uncertainty remains visible in the save result.

## Close, discard, and reopen behavior

Clean buffers may close directly. Dirty and conflicted buffers require explicit user
confirmation. The confirmation includes the current content generation, so an edit
made after the warning invalidates the old confirmation. Explicit discard and reopen
removes the confirmed local generation and reloads the current repository bytes.

## What comes later

Rendered editor tabs and conflict controls, Rust syntax and diagnostics, completion,
definition navigation, project-wide symbol search, autosave policy, and recovery UI
remain later capabilities.
