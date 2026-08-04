# FORGEOS-V1-VERIFY-200 Closure and Specification

Status: `ACTIVE_OPERATOR_VALIDATION_PENDING`
Capability: Version-bound build and test result records
Active slice: `FORGEOS-V1-VERIFY-200-SLICE-001`
Source authority: `Forge_OS_V1_base_48.tar`

## Capability statement

ForgeOS now runs one immutable registered command between two consistency-checked
project Git inspections and emits one canonical append-only verification record.
The record binds exact command identity, executable, argv, native source revision,
dirty-state identity, terminal outcome, exit code, and content-addressed stdout and
stderr evidence.

## Public contract

```text
forge_core::verification::VerificationSourceState
forge_core::verification::VerificationOutputReference
forge_core::verification::VerificationOutcome
forge_core::verification::VerificationRecord
forge_core::verification::VerificationLedger
forge_app::composition::verification_workspace::ProjectVerificationWorkspace
forge_app::composition::verification_workspace::VerificationApplicability
```

## Intended behavior

- validation begins from one consistency-checked project Git snapshot;
- command execution is rejected before spawn when the configured revision is stale;
- only immutable registered command definitions may run;
- exact executable and argv are retained without shell reconstruction;
- pre-run and post-run native revision plus dirty-state identities are retained;
- passed, failed, timed-out, cancelled, and execution-failed outcomes remain distinct;
- normal exit codes remain exact, including nonzero and absent native codes;
- stdout and stderr remain separate and are referenced by exact content identity;
- a passing record satisfies only the exact post-run source state;
- any later source-state change makes the historical result stale;
- failed or interrupted runs never satisfy current validation;
- history is append-only, canonically ordered, and round-trips through Forge Core
  state records without replacement or deletion;
- restored history must belong to the exact project and repository.

## Regression locks

```text
crates/forge-core/tests/verification_records.rs
crates/forge-app/tests/verification_workspace.rs
python3 scripts/run_ci.py
```

## Operator validation still required

Run the canonical behavior-only CI entrypoint:

```bash
python3 scripts/run_ci.py
```

The skill remains active until that command passes on the operator host.

## Explicit non-claims

This slice does not provide Forge World validation UI, CI pipeline scheduling,
remote-agent proof, Nyx-issued validation, artifact upload, wall-clock duration,
format/build/test semantic inference from display names, or Tier-3 verification
workflow closure.
