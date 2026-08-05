# FORGEOS-V1-SESSION-200 Closure and Specification

Status: `CLOSED`
Capability: Dedicated ForgeOS session bootstrap
Closed slice: `FORGEOS-V1-SESSION-200-SLICE-001`
Source authority: `Forge_OS_V1_base_50.tar`

## Capability statement

ForgeOS owns one deterministic display-manager session entry and one shell-free
launcher for the installed `forge-app` composition root. The launcher clears the
ambient environment, reconstructs only the declared session variables, starts from
`/` instead of an arbitrary worktree, and preserves the real child exit state.

## Public contract

```text
forge_session::bootstrap::SessionEnvironment
forge_session::bootstrap::SessionLaunchRequest
forge_session::bootstrap::SessionLaunchOutcome
forge_session::bootstrap::launch_session
forge-session binary: forgeos-session-launcher
forge-session asset: assets/forgeos.desktop
```

## Closure evidence

Operator behavior-only CI passed after the active slice:

```text
FORGE_SEAM_DIRECTION_SUMMARY status=PASS packages=12 routes=42 forbidden=0
FORGE_CORE_PURITY_SUMMARY status=PASS packages=2 allowed=2 forbidden=0
FORGE_SOURCE_SIZE_SUMMARY status=PASS modules=145 pass=145 warn=0 fail=0
CARGO_TEST_SUMMARY status=PASS suites=77 passed=365 failed=0 ignored=2
CI RESULT: PASS
```

## Proved behavior

- the desktop entry names one absolute installed launcher path;
- the launcher names one absolute installed composition-root path by default;
- an explicit absolute composition root may be supplied for testing and packaging;
- no shell command string, shell profile, current directory, or source worktree is
  consulted;
- only the declared session environment reaches the composition root;
- dangerous or unrelated inherited variables do not reach the composition root;
- the composition root always starts with `/` as its current directory;
- normal child exit codes and signalled exits remain truthful;
- invalid session environment, relative executable paths, and spawn failure return
  explicit nonzero launcher failures;
- the session source asset does not modify another installed desktop session.

## Regression locks

```text
crates/forge-session/tests/session_bootstrap.rs
crates/forge-session/tests/session_lifecycle.rs
crates/forge-session/tests/service_plan.rs
python3 scripts/run_ci.py
```

## Explicit non-claims

This capability does not install files into a live display manager, supervise Nyx,
restart services, render the Forge World cockpit, package a distribution, or close
the Tier-3 login journey.
