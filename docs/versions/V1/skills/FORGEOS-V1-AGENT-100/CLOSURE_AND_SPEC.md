# FORGEOS-V1-AGENT-100 Closure and Specification

Status: `CLOSED`
Validation: `OPERATOR_ACCEPTED`
Cross-repo gate input: `NYX_GATE_INPUT.json`

## Closed capability

ForgeOS consumes Nyx_Server's versioned remote-agent run API through a thin client.
Nyx owns task and run identity, routing, provider/model attribution, source/worktree
binding, budgets, execution, cancellation, continuation, recorded cost, terminal
state, durable storage, and audit history.

ForgeOS constructs exact requests, submits them, inspects returned records, controls
the exact accepted queued run, and independently rejects malformed or contradictory
records. ForgeOS keeps no second canonical run ledger.

## Closure evidence

- Forge behavior CI: 81 suites, 397 passed, 0 failed, 4 ignored.
- Real Nyx witness: exact deferred run queued, read/listed, cancelled, and terminal
  with continuation disabled.
- Nyx system gate receipts: `AGENT-FOUND-002`, `AGENT-RUN-001`,
  `AGENT-BUDGET-001`, `ROUTING-COST-001`, and `PERSIST-RUN-001`.

## Explicit non-claims

This capability does not contact providers directly, execute an agent, mutate a
worktree, apply a patch, infer cost from model prose, or persist competing run state
in Forge Core.
