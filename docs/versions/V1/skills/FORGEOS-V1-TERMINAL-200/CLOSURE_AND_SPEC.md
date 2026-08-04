# FORGEOS-V1-TERMINAL-200 Closure and Specification

Status: `CLOSED`
Capability: Managed embedded terminal sessions
Closed by operator evidence from `Forge_OS_V1_base_44.tar`

## Capability statement

ForgeOS composes project-owned repository boundaries with real native PTYs. Every
terminal is bound to stable project, repository, and terminal identities; raw output
is retained in exact sequence for rendering; and every input, resize, termination,
view, and removal operation must present the same binding.

## Accepted operator evidence

```text
FORGE_SEAM_DIRECTION_SUMMARY status=PASS packages=12 routes=42 forbidden=0 policy=exact-reviewed-subsystem-reachability-v1
FORGE_CORE_PURITY_SUMMARY status=PASS packages=2 allowed=2 forbidden=0 policy=exact-reviewed-production-graph-v1
FORGE_SOURCE_SIZE_SUMMARY status=PASS modules=128 pass=128 warn=0 fail=0 warnings_denied=true
CARGO_TEST_SUMMARY status=PASS suites=68 passed=317 failed=0 ignored=2 measured=0 filtered_out=0
CI RESULT: PASS
```

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

## Proven behavior

- multiple native PTYs remain isolated by stable terminal identity;
- every terminal records its owning project and repository identity;
- forged project or repository bindings fail closed;
- working directories resolve through the existing project boundary;
- only manifest-declared roots may become terminal working directories;
- repository-root access requires explicit repository-root authority;
- symlink and boundary escapes fail before process spawn;
- raw PTY output remains byte-exact and sequence-ordered for rendering;
- resize, input, close-input, exit, termination, and removal use the real PTY;
- removing or terminating one terminal does not affect another.

## Explicit non-claims

This skill does not execute registered project commands, persist command history,
render the final Forge World terminal widget, restore terminals after reboot, own
project paths, or provide command approval policy. Those behaviors remain separate
capabilities.
