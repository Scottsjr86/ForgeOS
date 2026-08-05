# FORGEOS-V1-NYX-101 Closure and Specification

Status: `ACTIVE`
Capability: Permission grant, checkpoint, and immutable resume token
Active slice: `FORGEOS-V1-NYX-101-SLICE-001`
Forge source authority: `Forge_OS_V1_base_52.tar`
Forge source archive SHA-256: `962527439cc6f8f8b433adde3ba58ea75b8fa7a802a0e947d1e8ce96c89ef98a`
Nyx source authority inspected: `Nyx_Server_base_16.tar`
Nyx source archive SHA-256: `83fdf8947259b863912986660ed054cc6a208aefcd967245abe5135367ed8f5a`
Verified Nyx gate input: `docs/versions/V1/skills/FORGEOS-V1-NYX-101/NYX_GATE_INPUT.json`

## Capability statement

ForgeOS consumes the Nyx-owned public permission API as a thin client. It may
construct one exact scoped request, ask Nyx to create a checkpoint, present the
immutable hashes and policy decision for review, submit an explicit approve or
deny decision, and return the exact Nyx-issued resume token with the exact
approved request.

Nyx_Server remains authoritative for checkpoint identity, request acceptance,
policy decisions, approval state, expiration, token generation, persistence,
idempotency, execution reservation, tool execution, result identity, and audit
history. ForgeOS stores no competing permission ledger and never executes a
resumed tool around Nyx.

## Verified Nyx cross-repository gate

The following Nyx skills are verified `BANKED` at `PROOF_SYSTEM`:

```text
POLICY-CHECKPOINT-001
POLICY-APPROVAL-001
POLICY-TOOL-015
PERSIST-IDEMP-KEY-001
```

The exact receipts, SHA-256 values, public routes, Nyx source archive identity,
Nyx CI result, real-server witness, and standalone chat/development result are
recorded in `NYX_GATE_INPUT.json`.

## Public ForgeOS contract

```text
forge_protocol::hashes::hash_external_contract_bytes
forge_nyx_client::permission::NyxPermissionScope
forge_nyx_client::permission::NyxScopedToolRequest
forge_nyx_client::permission::NyxPermissionCheckpointCreate
forge_nyx_client::permission::NyxPermissionCheckpoint
forge_nyx_client::permission::NyxPermissionCheckpointStatus
forge_nyx_client::permission::NyxPermissionDecisionKind
forge_nyx_client::permission::NyxPermissionResolution
forge_nyx_client::permission::NyxPermissionResume
forge_nyx_client::permission::NyxPermissionResumeResult
forge_nyx_client::permission::NyxPermissionAuditReport
forge_nyx_client::permission::NyxPermissionProtocolError
forge_nyx_client::permission_client::NyxPermissionClient
forge_nyx_client::permission_client::NyxPermissionClientError
forge_nyx_client::permission_client::NyxPermissionServerError
```

## Intended behavior

- requests use Nyx's `nyx.1.0` canonical schemas and public HTTP routes;
- the shared Forge protocol exposes one narrowly named raw SHA-256 interoperability helper; Forge-owned identities remain domain-separated;
- ForgeOS independently recomputes Nyx-compatible canonical JSON SHA-256 values;
- returned request, payload, scope, policy, effect, approval-condition, and token
  hashes are rejected when they do not match their exact payloads;
- checkpoint creation must return the exact request supplied by ForgeOS;
- approval and denial operate on the immutable checkpoint hashes selected by the
  operator;
- approval returns one Nyx-issued resume token bound to the reviewed request;
- denial returns no token;
- resume submits the exact request and token back to Nyx;
- mutation, denial, expiration, consumption, replay, and server restart behavior
  remain Nyx-owned and are preserved as typed server rejection;
- audit events remain Nyx-owned and must have strictly increasing sequence IDs;
- incompatible public-contract majors, malformed schemas, missing headers, and
  contradictory hashes fail closed.

## Regression locks

```text
crates/forge-nyx-client/src/canonical_json.rs
crates/forge-nyx-client/src/permission.rs
crates/forge-nyx-client/src/permission/
crates/forge-nyx-client/src/permission_client.rs
crates/forge-nyx-client/tests/nyx_permissions.rs
python3 scripts/run_ci.py
```

One ignored test is the explicit independent real-Nyx process witness. It creates,
approves, and resumes one `repo.write_file` request, rejects replay, and verifies
the corresponding Nyx audit event.

## Operator validation required

Run Forge behavior-only CI:

```bash
python3 scripts/run_ci.py
```

Then run the real-process witness against an independently started
`Nyx_Server_base_16` process using the commands in `USER_GUIDE_SOURCE.md`.

The capability remains `ACTIVE` and `OPERATOR_VALIDATION_PENDING` until both
commands pass.

## Explicit non-claims

This slice does not persist permission state in Forge Core, mint checkpoint IDs,
create resume tokens, evaluate Nyx policy, execute tools locally, manage general
Nyx conversations, dispatch agents, or expose a final Forge World approval UI.
Those capabilities remain separate. Nyx_Server continues to operate independently
for chat, development, and general API clients.
