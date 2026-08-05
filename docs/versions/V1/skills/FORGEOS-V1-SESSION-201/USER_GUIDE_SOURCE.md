# FORGEOS-V1-SESSION-201 User Guide Source

Status: `ACTIVE_OPERATOR_VALIDATION_PENDING`

## What this capability provides

ForgeOS can own one Nyx process for the lifetime of a ForgeOS session. It starts Nyx
without a shell, waits for Nyx's public health contract, displays degraded model
readiness honestly, observes crashes, applies a bounded restart policy, and stops the
exact process group during logout.

## Readiness meaning

A running PID is not enough. ForgeOS considers the service available only after Nyx
reports both:

```text
live=true
control_plane_ready=true
```

Nyx may still report degraded model or provider readiness. That state is preserved
instead of repainted as healthy.

## Duplicate protection

Before starting Nyx, ForgeOS probes the configured endpoint. A compatible,
incompatible, malformed, or otherwise occupied endpoint blocks another spawn. A
second start against the already managed process is also rejected.

## Failure isolation

A Nyx crash changes only the Nyx managed-service ledger. Local repository browsing,
editing, terminals, commands, Git, and verification remain available. Restarting Nyx
requires a new stable process identity and cannot exceed the configured retry budget.

## Ownership cutline

ForgeOS supervises the process and consumes health. Nyx_Server still owns server
behavior, capability truth, model routing, conversations, tools, policy, agents, and
persistence, and remains independently usable by non-ForgeOS clients.
