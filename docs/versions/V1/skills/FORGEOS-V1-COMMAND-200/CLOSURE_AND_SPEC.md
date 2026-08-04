# FORGEOS-V1-COMMAND-200 Closure and Specification

Status: `ACTIVE_OPERATOR_VALIDATION_PENDING`
Capability: Registered project command execution and exact output history
Active slice: `FORGEOS-V1-COMMAND-200-SLICE-001`
Source authority: `Forge_OS_V1_base_44.tar`

## Capability statement

ForgeOS now joins immutable registered-command definitions to real managed process
execution. Every run is shell-free, bound to one validated project and repository,
uses the declared working directory and clear-parent environment policy, and records
its exact source revision, command-definition identity, process identity, stdout,
stderr, exit state, timeout, or cancellation outcome.

## Public contract

```text
forge_bridge::processes::ProcessExecutionContext
forge_terminal::execution::CommandSourceBinding
forge_terminal::execution::CommandRunRecord
forge_terminal::execution::CommandRunRegistry
forge_terminal::execution::CommandRunError
forge_app::composition::command_workspace::ProjectCommandWorkspace
forge_app::composition::command_workspace::ProjectCommandWorkspaceError
```

## Intended behavior

- only immutable `RegisteredCommand` definitions may execute;
- executable and argv enter the operating system without shell interpolation;
- command definitions are checked by exact content identity before launch;
- working directories resolve through the registered repository boundary;
- undeclared parent environment variables are removed;
- declared literal and inherited environment values are exact and sorted;
- passing, failing, timed-out, and cancelled processes remain distinct;
- stdout and stderr remain exact separate byte streams;
- duplicate process IDs fail before a second launch;
- every record retains project, repository, revision, command, definition, and
  process identity;
- foreign repositories and broader undeclared directories fail before spawn.

## Regression locks

```text
crates/forge-terminal/tests/registered_commands.rs
crates/forge-terminal/tests/command_execution.rs
crates/forge-app/tests/command_workspace.rs
python3 scripts/run_ci.py
```

## Operator validation still required

Run the canonical behavior-only CI entrypoint:

```bash
python3 scripts/run_ci.py
```

The skill remains active until that command passes on the operator host.

## Explicit non-claims

This slice does not accept arbitrary shell strings, make command history durable
across reboot, create verification receipts, render the Forge World command UI,
inspect Git, or implement Nyx permission checkpoints and resume semantics.
