# FORGEOS-V1-AGENT-100 Closure and Specification

Status: `ACTIVE`
Validation: `OPERATOR_VALIDATION_PENDING`
Cross-repo gate input: `NYX_GATE_INPUT.json`

## Bounded capability

ForgeOS consumes Nyx_Server's versioned remote-agent run API through a thin client.
Nyx owns task and run identity, routing, provider/model attribution, source/worktree
binding, budgets, execution, cancellation, continuation, recorded cost, terminal
state, durable storage, and audit history.

ForgeOS may construct exact requests, submit them, inspect returned records, cancel
or continue the exact accepted queued run, and independently reject malformed or
contradictory records. ForgeOS does not keep a second canonical run ledger.

## Source slice

- `forge-nyx-client` remote-agent protocol DTOs and strict validation;
- thin HTTP client for create, list, read, cancel, and continue;
- fixture-backed behavioral tests plus one bounded OpenAI-compatible provider fixture;
- one ignored real-Nyx integration witness;
- exact Nyx gate receipts and ownership evidence.

## Required proof

```bash
python3 scripts/run_ci.py
```

The operator must also run the ignored real-Nyx client witness against an
independently running Nyx_Server configured with a real or bounded fixture provider.
The witness must create a deferred run, read and list it, cancel it, and verify that
later continuation is rejected.

## Explicit non-claims

This slice does not contact providers directly, execute an agent, mutate a worktree,
apply a patch, infer cost from model prose, or persist competing run state in Forge
Core. Complete, failed, cancelled, and budget-hit server behavior is owned and
system-proved by Nyx_Server.
