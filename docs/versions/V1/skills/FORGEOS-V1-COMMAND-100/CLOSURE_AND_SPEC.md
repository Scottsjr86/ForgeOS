# FORGEOS-V1-COMMAND-100 Closure and Specification

Status: `CLOSED`
Capability: Immutable registered-command definition and exact launch policy
Closed: `2026-08-02`
Source authority: `Forge_OS_V1_base_26.tar`
Source archive SHA-256: `104b15ab148a83963c4423acfdfa243168db2df3f5acf78fff8796b0a7386e5d`
Git revision: unavailable because the supplied source archive contains no `.git` metadata

## Capability statement

ForgeOS now owns immutable registered-command definitions with stable command and
repository identity, exact executable and argv tokens, repository-bound working
directory declarations, clear-parent declared environment policy, explicit timeout,
process-group cancellation policy, authority class, stable definition identity, and
an inspectable shell-free launch payload prepared without spawning a process.

## Public contract

```text
forge_core::commands::RegisteredCommand
forge_core::commands::CommandRegistry
forge_core::commands::CommandRegistration
forge_core::commands::CommandEnvironmentPolicy
forge_core::commands::CommandEnvironmentVariable
forge_core::commands::CommandWorkingDirectory
forge_core::commands::CommandTimeout
forge_core::commands::CommandCancellationPolicy
forge_core::commands::CommandAuthorityClass
forge_terminal::commands::CommandDirectoryBinding
forge_terminal::commands::CommandLaunchPayload
forge_terminal::commands::ResolvedCommandEnvironmentVariable
forge_terminal::commands::CommandLaunchError
```

## Accepted operator evidence

```text
FORGE_SEAM_DIRECTION_SUMMARY status=PASS packages=12 routes=42 forbidden=0 policy=exact-reviewed-subsystem-reachability-v1
FORGE_CORE_PURITY_SUMMARY status=PASS packages=2 allowed=2 forbidden=0 policy=exact-reviewed-production-graph-v1
FORGE_SOURCE_SIZE_SUMMARY status=PASS modules=85 pass=85 warn=0 fail=0 warnings_denied=true
CARGO_TEST_SUMMARY status=PASS suites=50 passed=172 failed=0 ignored=0 measured=0 filtered_out=0
CI RESULT: PASS
```

CI remains limited to behavioral tests, golden locks, and structural guards. It
contains no documentation verification, Git-state check, or formatting gate.

## Negative and failure-path results

Fixtures cover malformed command text, duplicate environment declarations, silent
command-ID meaning changes, invalid replacement identity, repository and working-
directory crossing, missing or malformed inherited environment values, undeclared
secret exclusion, invalid timeout, and literal shell-looking argv tokens.

## Explicit non-claims

This closure does not spawn registered commands, capture command output, persist
command history, restore project terminals, mutate files or Git, manage sessions,
connect Nyx, perform recovery, or render Forge World UI.
