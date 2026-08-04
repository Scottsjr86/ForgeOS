# FORGEOS-V1-TERMINAL-200 Closure and Specification

Status: `ACTIVE_OPERATOR_VALIDATION_PENDING`
Capability: Managed embedded terminal sessions
Active slice: `FORGEOS-V1-TERMINAL-200-SLICE-001`
Source authority: `Forge_OS_V1_base_43.tar`

## Capability statement

ForgeOS now composes project-owned repository boundaries with real native PTYs.
Every terminal is bound to stable project, repository, and terminal identities;
raw output is retained in exact sequence for rendering; and every input, resize,
termination, view, and removal operation must present the same binding.

## Public contract

```text
forge_terminal::managed::ManagedTerminalHandle
forge_terminal::managed::ManagedTerminalSpawnRequest
forge_terminal::managed::ManagedTerminalRegistry
forge_terminal::managed::ManagedTerminalView
forge_app::composition::terminal_workspace::ProjectTerminalWorkspace
forge_app::composition::terminal_workspace::ProjectTerminalLaunch
forge_app::composition::terminal_workspace::TerminalWorkingDirectory
```

## Intended behavior

- multiple native PTYs remain isolated by stable terminal identity;
- every terminal records its owning project and repository identity;
- operations using a forged project or repository binding fail closed;
- working directories are resolved through the existing project boundary;
- only manifest-declared roots may become terminal working directories;
- repository-root access requires explicit repository-root authority;
- symlink or boundary escapes fail before process spawn;
- raw PTY output remains byte-exact and sequence-ordered for rendering;
- resize, input, close-input, exit, termination, and removal use the real PTY;
- removing one exited terminal does not affect any other terminal.

## Regression locks

```text
crates/forge-terminal/tests/pty_sessions.rs
crates/forge-terminal/tests/managed_terminals.rs
crates/forge-app/tests/terminal_workspace.rs
python3 scripts/run_ci.py
```

The behavioral matrix covers multiple concurrent terminals, project/repository
binding, raw transcript rendering, input, resize, clean exit, operator termination,
forged handles, scope rejection, repository-root authority, symlink rejection, and
exited-only removal.

## Operator validation still required

Run the canonical behavior-only CI entrypoint:

```bash
python3 scripts/run_ci.py
```

The skill remains active until that command passes on the operator host.

## Explicit non-claims

This skill does not execute registered project commands, persist command history,
render the final Forge World terminal widget, restore terminals after reboot, own
project paths, or provide shell policy beyond the exact shell-free PTY launch
request. Those behaviors remain later capabilities.
