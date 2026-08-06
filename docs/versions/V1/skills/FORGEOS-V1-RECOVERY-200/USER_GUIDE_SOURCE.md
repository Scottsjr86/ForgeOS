# FORGEOS-V1-RECOVERY-200 User Guide Source

Status: `ACTIVE`
Validation: `OPERATOR_VALIDATION_PENDING`

After a controlled crash or restart, ForgeOS can offer one explicit recovery image.
Unsaved editor bytes return in their original buffers. If disk changed meanwhile,
ForgeOS marks the buffer conflicted and preserves both truths instead of overwriting
the file.

Terminals and Nyx services never return as magically alive. ForgeOS shows that they
require restart or revalidation. Interrupted commands and tools remain visible for
inspection but cannot replay.

Abandoned staged recovery bytes and previous-image promotion require explicit safe
choices. Clean startup does not enter recovery mode when no recovery state exists.
