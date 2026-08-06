# FORGEOS-V1-RECOVERY-200 Closure and Specification

Status: `ACTIVE`
Validation: `OPERATOR_VALIDATION_PENDING`
Active slice: `FORGEOS-V1-RECOVERY-200-SLICE-001`

## Bounded capability

ForgeOS captures one canonical workspace recovery image and explicitly restores
only state that can be recovered honestly. Dirty editor bytes return without
writing repository files. Terminal and Nyx process records return only as non-live
metadata. Interrupted actions remain inspect-only and may never replay.

## Source slice

- canonical versioned durable workspace payload in Forge Core;
- exact dirty/conflicted editor bytes, cursor, generation, path, and disk baselines;
- terminal working-directory and exit metadata with running terminals downgraded to `RequiresRestart`;
- Nyx lifecycle metadata downgraded to stopped, failed, restart-pending, or requiring revalidation;
- atomic generation-guarded publication through the existing recovery store;
- explicit staged-write discard and previous-image promotion choices;
- project/session/repository binding checks;
- fixture-backed controlled crash and restore tests.

## Required proof

```bash
python3 scripts/run_ci.py
```

The tests must prove dirty-buffer restoration, conflict presentation after external
disk changes, non-live terminal/service state, non-replayable interrupted actions,
sequential generations, explicit staged-write cleanup, and safe previous-image
promotion.

## Explicit non-claims

This slice does not replay commands, restart terminals or Nyx automatically,
replace repository files during restore, hide conflicts, or create another
persistence implementation.
