# FORGEOS-V1-PROCESS-000 User Guide Source

Status: `CLOSED_SOURCE`
Capability: stable managed-process execution

## What the operator can do

ForgeOS can spawn a real executable from an exact argument vector under a stable
ForgeOS process identity, observe raw stdout and stderr separately, and receive an
explicit terminal result for exit, timeout, cancellation, or failure.

## Validation commands

```bash
cargo test -p forge-protocol
cargo test -p forge-bridge
```

## Visible outcomes

- `Exited`: the native process exited; zero and nonzero statuses remain distinct.
- `TimedOut`: the declared timeout won after no native exit was observed.
- `Cancelled`: cancellation won after no native exit was observed.
- `Failed`: spawn, wait, termination, or output collection failed explicitly.

## Fixed safety behavior

The public request contains an executable and argv array, not shell prose. On
Unix, timeout and cancellation target the process group so child processes do not
outlive the managed execution. Native PIDs are metadata and never replace the
stable ForgeOS `ProcessId`.

## Current V1 limitations

This capability does not choose a repository working directory, allocate a PTY,
register project commands, supervise services, or provide LSP, Git, or Nyx
behavior. Those remain separate routed skills.
