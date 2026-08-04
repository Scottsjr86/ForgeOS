# FORGEOS-V1-SESSION-200 Closure and Specification

Status: `ACTIVE_OPERATOR_VALIDATION_PENDING`
Capability: Dedicated ForgeOS session bootstrap
Active slice: `FORGEOS-V1-SESSION-200-SLICE-001`
Source authority: `Forge_OS_V1_base_49.tar`

## Capability statement

ForgeOS now owns one deterministic display-manager session entry and one shell-free
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

## Intended behavior

- the desktop entry names one absolute installed launcher path;
- the launcher names one absolute installed composition-root path by default;
- an explicit absolute composition root may be supplied for testing and packaging;
- no shell command string, shell profile, current directory, or source worktree is
  consulted;
- `HOME`, `XDG_RUNTIME_DIR`, display transport, identity, locale, and selected XDG
  paths are copied only through the declared whitelist;
- dangerous or unrelated inherited variables such as `LD_PRELOAD`, `BASH_ENV`,
  `SHELL`, and `PWD` do not reach the composition root;
- the composition root always starts with `/` as its current directory;
- a normal child exit code is returned exactly by the session launcher;
- a signalled child maps to the conventional `128 + signal` launcher status;
- invalid session environment, relative executable paths, and spawn failure return
  explicit nonzero launcher failures;
- this slice adds a new ForgeOS session source asset and never edits another installed
  desktop session.

## Regression locks

```text
crates/forge-session/tests/session_bootstrap.rs
crates/forge-session/tests/session_lifecycle.rs
crates/forge-session/tests/service_plan.rs
python3 scripts/run_ci.py
```

## Operator validation still required

Run the canonical behavior-only CI entrypoint:

```bash
python3 scripts/run_ci.py
```

The skill remains active until that command passes on the operator host.

## Explicit non-claims

This slice does not install files into `/usr/share/xsessions`, configure SDDM or
another display manager, provide a compositor, supervise Nyx, restart services,
render the Forge World cockpit, package a distribution, or close the Tier-3 login
journey. Installation and clean-host login remain later distribution and session
acceptance work.
