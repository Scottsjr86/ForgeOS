# FORGEOS-V1-NYX-101 User Guide Source

Status: `CLOSED`
Capability: Nyx-owned permission checkpoints and exact immutable resume
Cross-repo gate input: `docs/versions/V1/skills/FORGEOS-V1-NYX-101/NYX_GATE_INPUT.json`

## What the operator can do

ForgeOS can submit one exact, scoped Nyx tool request and display the immutable
checkpoint returned by Nyx. The operator can approve or deny that checkpoint.
Approval returns a Nyx-issued token that may be sent back only with the exact
reviewed request. Nyx executes the approved tool once and rejects altered,
denied, expired, consumed, or replayed requests.

ForgeOS does not keep a second checkpoint database. Reopening, persistence,
idempotency, execution reservation, tool effects, and audit truth remain in the
independent Nyx server.

## Forge validation

```bash
python3 scripts/run_ci.py
```

## Real Nyx witness

From the Nyx repository, create fresh witness directories and start the server:

```bash
rm -rf /tmp/forgeos-nyx101-workspace \
       /tmp/forgeos-nyx101-permissions \
       /tmp/forgeos-nyx101-ledger
mkdir -p /tmp/forgeos-nyx101-workspace \
         /tmp/forgeos-nyx101-permissions \
         /tmp/forgeos-nyx101-ledger

NYX_BIND=127.0.0.1:8088 \
NYX_WORKSPACE_ROOT=/tmp/forgeos-nyx101-workspace \
NYX_PERMISSION_STORE_DIR=/tmp/forgeos-nyx101-permissions \
NYX_LEDGER_DIR=/tmp/forgeos-nyx101-ledger \
  cargo run --locked --quiet -p nyx_server --bin nyx_server
```

From the ForgeOS repository, run:

```bash
FORGE_NYX_ADDR=127.0.0.1:8088 \
FORGE_NYX_WORKSPACE_ROOT=/tmp/forgeos-nyx101-workspace \
  cargo test --locked -p forge-nyx-client --test nyx_permissions \
  real_nyx_permission_gate_proves_exact_approval_and_replay_rejection \
  -- --ignored --nocapture
```

The test passes only when the real Nyx process creates a checkpoint, returns an
approval token, executes the exact request, rejects replay, and publishes the
completed audit event.

## Safety behavior

- malformed or contradictory hashes fail before approval or resume;
- a checkpoint response that changes the submitted request is rejected;
- approval is bound to exact request, scope, and policy hashes;
- denied resolutions expose no token;
- altered requests may be submitted only as a negative test and must be rejected
  by Nyx;
- ForgeOS never treats HTTP success alone as permission;
- ForgeOS never executes `repo.write_file` or any other Nyx tool locally.

## Closure evidence

Forge CI passed with 80 suites, 385 tests passed, 0 failed, and 3 ignored. The independent real-Nyx checkpoint, approval, exact resume, audit, and replay-rejection witness also passed.

## Current V1 limitations

This slice exposes the protocol client and proof path, not the final approval UI,
conversation workflow, command-tool integration, or remote-agent path. Those are
activated separately after this capability closes.
