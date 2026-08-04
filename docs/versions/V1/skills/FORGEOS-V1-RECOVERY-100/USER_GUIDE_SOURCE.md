# FORGEOS-V1-RECOVERY-100 User Guide Source

Status: `ACTIVE_OPERATOR_VALIDATION_PENDING`
Capability: Workspace snapshot and crash-journal primitive

## What the operator can verify

ForgeOS can persist a safe workspace recovery image, identify interrupted
publication, distinguish unresolved actions from restorable workspace data, and
present explicit recovery choices without automatically changing current state.

Historical process identities are diagnostic only. After restart, any process that
was previously running or ready is marked as requiring revalidation. ForgeOS does
not claim that the process survived merely because its old identity was recorded.

## Recovery choices

- `KeepCurrent` appears when the current recovery image is valid.
- `DiscardInterruptedWrite` appears only when valid current data exists beside an
  abandoned staged publication.
- `RestorePrevious` appears only when current data is missing or invalid and the
  retained previous image is valid.
- An older previous image cannot replace a valid current image.

Interrupted actions are inspection records. They are never executable resume tokens
and are never replayed automatically.

## Validation command

```bash
python3 scripts/run_ci.py
```

## Current V1 limitations

This primitive does not yet reconstruct the full workspace, restart services,
restore terminals, reconnect Nyx, or present recovery controls in Forge World. It
supplies the safe data and decision boundary those later capabilities must consume.
