# FORGEOS-V1-TERMINAL-100 Closure and Specification

Status: `CLOSED`
Capability: Native PTY spawn, exact byte I/O, resize, exit, and isolated termination
Closed: `2026-08-02`
Source authority: `Forge_OS_V1_base_25.tar`
Source archive SHA-256: `bfae16d0d73aeeff2dd2b2c2a3d8728f473252bb3597702159ece321589d643a`
Git revision: unavailable because the supplied source archive contains no `.git` metadata

## Capability statement

ForgeOS now owns real native Linux PTY sessions with stable terminal identity,
shell-free executable and argv launch, validated absolute working directories,
exact raw byte input and output, kernel resize, native exit inspection, explicit
operator termination, and isolation between concurrent terminals.

## Public contract

```text
forge_bridge::pty::NativePtyProcess
forge_bridge::pty::NativePtyLaunch
forge_bridge::pty::NativePtySize
forge_bridge::pty::NativePtyExit
forge_bridge::pty::NativePtyTermination
forge_bridge::pty::NativePtyDrain
forge_bridge::pty::PtyAdapterError
forge_terminal::pty::PtyDimensions
forge_terminal::pty::PtySpawnRequest
forge_terminal::pty::PtySession
forge_terminal::pty::PtyRegistry
forge_terminal::pty::PtyOutputChunk
forge_terminal::pty::PtyLifecycle
forge_terminal::pty::PtyExit
forge_terminal::pty::PtyError
```

## Accepted operator evidence

```text
FORGE_SEAM_DIRECTION_SUMMARY status=PASS packages=12 routes=42 forbidden=0 policy=exact-reviewed-subsystem-reachability-v1
FORGE_CORE_PURITY_SUMMARY status=PASS packages=2 allowed=2 forbidden=0 policy=exact-reviewed-production-graph-v1
FORGE_SOURCE_SIZE_SUMMARY status=PASS modules=82 pass=82 warn=0 fail=0 warnings_denied=true
CARGO_TEST_SUMMARY status=PASS suites=48 passed=152 failed=0 ignored=0 measured=0 filtered_out=0
CI RESULT: PASS
```

CI remains limited to behavioral tests, golden locks, and structural guards. It
contains no documentation verification, Git-state check, or formatting gate.

## Negative and failure-path results

Fixtures cover duplicate and unknown terminal IDs, invalid dimensions, noncanonical
working directories, missing executables, post-exit input and resize, removal while
running, PTY EOF normalization, explicit termination, and concurrent-session
isolation.

## Explicit non-claims

This closure does not render a terminal UI, parse ANSI, define or execute registered
project commands, preserve command history, restore project terminals, inspect Git,
connect Nyx, manage a ForgeOS session, perform recovery, or render Forge World UI.
