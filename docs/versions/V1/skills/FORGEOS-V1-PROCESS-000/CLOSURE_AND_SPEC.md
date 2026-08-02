# FORGEOS-V1-PROCESS-000 Closure and Specification

Status: `CLOSED`
Capability: stable process lifecycle and cancellation model
Closed: `2026-08-01`
Source authority: `Forge_OS_V1_base_16.tar`
Source archive SHA-256: `b19609101f41b8a383a7f73408f656cc41adc33e3e59197811c93bb68b7217ce`
Git revision: unavailable because the supplied source archive contains no `.git` metadata

## Capability statement

ForgeOS now provides one real, shell-free managed-process path. A stable ForgeOS
`ProcessId` is assigned before spawn and remains distinct from the native PID.
The runner preserves raw stdout and stderr independently, reports native zero and
nonzero exits, and distinguishes spawn failure, wait failure, timeout,
cancellation, termination failure, and output failure.

On Unix, each child starts in its own process group. Timeout and cancellation
terminate the process group so descendants do not survive the owning execution.
One lifecycle accepts exactly one terminal outcome, and concurrent executions
retain their own identities and output.

## Public contract

The accepted source exposes:

```text
forge_protocol::processes::ProcessSpawnRequest
forge_protocol::processes::ProcessLifecycle
forge_protocol::processes::ProcessOutcome
forge_protocol::processes::ProcessExecution
forge_protocol::processes::ProcessOutput
forge_protocol::processes::ProcessOutputChunk
forge_bridge::processes::CancellationToken
forge_bridge::processes::ProcessRunner
```

## Exercised path and evidence

The operator applied the process patch, ran the complete handed-off command chain,
reported all commands green, supplied the exact structural summaries, and
requested continuation on `2026-08-01`.

Operator-run validation covered:

```bash
cargo fmt --all
cargo check --workspace
cargo test -p forge-protocol
cargo test -p forge-bridge
cargo run -p forge-guards --bin forge-seam-direction -- --root .
cargo run -p forge-guards --bin forge-core-purity -- --root .
cargo run -p forge-guards --bin forge-source-size -- --root . --deny-warnings
cargo test --workspace
git diff --check
git status --short
```

The reported real-workspace summaries were:

```text
FORGE_SEAM_DIRECTION_SUMMARY status=PASS packages=12 routes=42 forbidden=0 policy=exact-reviewed-subsystem-reachability-v1
FORGE_SOURCE_SIZE_SUMMARY status=PASS modules=59 pass=59 warn=0 fail=0 warnings_denied=true
FORGE_CORE_PURITY_SUMMARY status=PASS packages=2 allowed=2 forbidden=0 policy=exact-reviewed-production-graph-v1
```

## Negative and failure-path results

Focused fixtures cover real zero and nonzero exits, missing executables, timeout,
pre-spawn cancellation, cancellation after spawn, separate raw output channels,
concurrent identity isolation, duplicate terminal transition rejection, and Unix
descendant cleanup.

## Regression locks

Every later process-owning capability must retain the stable `ProcessId`, preserve
raw output channel identity, keep nonzero native exits distinct from adapter
failure, and rerun the three structural guards plus the complete workspace suite.

## Explicit non-claims

This closure does not provide working-directory policy, repository boundary
verification, PTY behavior, registered commands, session orchestration, LSP, Git,
Nyx transport, persistence, hashing, or Forge World presentation.
