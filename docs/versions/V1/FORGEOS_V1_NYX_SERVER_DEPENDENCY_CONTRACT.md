# ForgeOS V1 to Nyx Server cross-repository dependency contract

Status: `ACTIVE_CLOSURE_AUTHORITY`
Contract ID: `FORGEOS-V1-NYX-SERVER-CROSS-REPO-CONTRACT`
Wiring cheat sheet: `docs/versions/V1/FORGEOS_NYX_SERVER_WIRING_CHEAT_SHEET.md`
Capability and ownership cheat sheet: `docs/versions/V1/FORGEOS_NYX_SERVER_CAPABILITY_OWNERSHIP_CHEAT_SHEET.md`
Machine mirror: `docs/versions/V1/FORGEOS_V1_NYX_SERVER_DEPENDENCIES.json`
Forge skill authority: `docs/versions/V1/FORGEOS_V1_FIRST_ARMOR_SKILL_TREE.md`
Forge execution authority: `docs/versions/V1/V1_EXECUTION_ROUTER.md`

---

## 0. Why this exists

ForgeOS and `Nyx_Server` are separate repositories with separate authority. The
ForgeOS crate `forge-nyx-client` is a client boundary, not a spare room where a
patching model may grow a second Nyx server.

This contract names the exact Nyx skill evidence required before a ForgeOS V1
skill that consumes Nyx may close. It also states which side owns each behavior.

The hard rule is:

```text
Nyx capability absent or unbanked
  -> ForgeOS may implement only its client/adapter/fixture side
  -> ForgeOS skill remains ACTIVE, BLOCKED, or SOURCE_PROVED
  -> ForgeOS skill may not become CLOSED
  -> any missing Nyx behavior is patched in Nyx_Server
```

A copied DTO, mock server, fixture, direct model call, local conversation store,
Forge-owned checkpoint engine, or fake run record does not satisfy a Nyx gate.

### 0.1 Current seam finding: Nyx public API gate is proved

`Nyx_Server_base_13.tar` provides Nyx-owned public contract version `1.0` through
`GET /v1/nyx/version`, `GET /v1/nyx/health`, and
`GET /v1/nyx/capabilities`. The required Nyx skills are banked with
`PROOF_SYSTEM`, their receipt hashes match the cross-repository handoff, and the
real-server witness plus standalone chat/development regression both pass.

Therefore:

```text
Nyx public API gate proved
  -> ForgeOS may replace its private fixture protocol with the Nyx-owned HTTP contract
  -> ForgeOS must validate schema IDs, contract headers, versions, health, capabilities,
     engine readiness, and provider readiness
  -> FORGEOS-V1-NYX-100 remains ACTIVE until Forge CI and a real Nyx client witness pass
```

The verified gate input is recorded at
`docs/versions/V1/skills/FORGEOS-V1-NYX-100/NYX_GATE_INPUT.json`. Nyx_Server
remains independent and authoritative for all server truth. ForgeOS owns only its
client adapter, classification, and independent witness path.

---

## 0.2 Mandatory wiring and ownership review

Before a model edits any ForgeOS path for a skill listed in this contract, it
must complete the preflight in
`docs/versions/V1/FORGEOS_NYX_SERVER_WIRING_CHEAT_SHEET.md` and
`docs/versions/V1/FORGEOS_NYX_SERVER_CAPABILITY_OWNERSHIP_CHEAT_SHEET.md`, then
inspect the current Nyx source that owns the exact surface.

The gate table answers "which Nyx proof must exist." The wiring cheat sheet
answers "how ForgeOS is allowed to connect." The ownership cheat sheet answers
"which repository and Nyx engine own the behavior now and at full maturity."
All three are required. None authorizes a Forge-owned replacement for missing
Nyx code.

## 1. State and proof law

Every listed Nyx skill must be `BANKED` or `RELEASE_EARNED` in the Nyx
append-only ledger. The associated proof must meet the listed minimum level and
exercise the real Nyx server/engine path needed by the Forge closure witness.

`LOCALLY_PROVED`, code presence, compilation, a passing Forge fixture, a document,
or a model claim is not enough for Forge closure.

Current Nyx status is never copied into this file as authority. Resolve it from:

```text
Nyx_Server/docs/workflow/skill_trees/NYX-SKILL-SEALED-LEDGER.jsonl
Nyx_Server/docs/workflow/skill_trees/NYX-SKILL-ROUTER_STATE.json
```

---

## 2. Exact closure gates

| ForgeOS V1 skill | Gate marker | Required Nyx skills | Minimum Nyx evidence |
|---|---|---|---|
| `FORGEOS-V1-NYX-100` | `NYX-GATE-FORGEOS-V1-NYX-100` | `API-FOUND-008`<br>`API-VERSION-010`<br>`API-SYS-044`<br>`API-SYS-047` | `BANKED` / `PROOF_SYSTEM` |
| `FORGEOS-V1-NYX-101` | `NYX-GATE-FORGEOS-V1-NYX-101` | `POLICY-CHECKPOINT-001`<br>`POLICY-APPROVAL-001`<br>`POLICY-TOOL-015`<br>`PERSIST-IDEMP-KEY-001` | `BANKED` / `PROOF_SYSTEM` |
| `FORGEOS-V1-SESSION-201` | `NYX-GATE-FORGEOS-V1-SESSION-201` | `API-FOUND-008` | `BANKED` / `PROOF_SYSTEM` |
| `FORGEOS-V1-NYX-200` | `NYX-GATE-FORGEOS-V1-NYX-200` | `API-SYS-020`<br>`API-SYS-021`<br>`API-SYS-022`<br>`API-SYS-027`<br>`ROUTING-REG-012`<br>`PERSIST-SESSION-001` | `BANKED` / `PROOF_SYSTEM` |
| `FORGEOS-V1-NYX-201` | `NYX-GATE-FORGEOS-V1-NYX-201` | `REPO-READ-001`<br>`REPO-SYS-024`<br>`TOOL-SYS-022`<br>`TOOL-CAP-031` | `BANKED` / `PROOF_SYSTEM` |
| `FORGEOS-V1-NYX-202` | `NYX-GATE-FORGEOS-V1-NYX-202` | `TOOL-CAP-034`<br>`AGENT-CAP-031`<br>`PERSIST-CAP-035` | `BANKED` / `PROOF_SYSTEM` |
| `FORGEOS-V1-NYX-300` | `NYX-GATE-FORGEOS-V1-NYX-300` | `API-CAP-030`<br>`REPO-CAP-030`<br>`AGENT-CAP-030` | `BANKED` / `PROOF_USER` |
| `FORGEOS-V1-NYX-301` | `NYX-GATE-FORGEOS-V1-NYX-301` | `AGENT-CAP-031`<br>`PERSIST-CAP-035`<br>`API-CAP-034` | `BANKED` / `PROOF_USER` |
| `FORGEOS-V1-NYX-400` | `NYX-GATE-FORGEOS-V1-NYX-400` | `AGENT-INTEG-040`<br>`AGENT-INTEG-041`<br>`API-CAP-033`<br>`ROUTING-INTEG-042`<br>`PERSIST-CAP-035` | `BANKED` / `PROOF_USER` |
| `FORGEOS-V1-AGENT-100` | `NYX-GATE-FORGEOS-V1-AGENT-100` | `AGENT-FOUND-002`<br>`AGENT-RUN-001`<br>`AGENT-BUDGET-001`<br>`ROUTING-COST-001`<br>`PERSIST-RUN-001` | `BANKED` / `PROOF_SYSTEM` |
| `FORGEOS-V1-AGENT-200` | `NYX-GATE-FORGEOS-V1-AGENT-200` | `AGENT-CAP-030`<br>`AGENT-CAP-034`<br>`ROUTING-CAP-037`<br>`POLICY-SYS-030` | `BANKED` / `PROOF_SYSTEM` |
| `FORGEOS-V1-AGENT-300` | `NYX-GATE-FORGEOS-V1-AGENT-300` | `AGENT-CAP-030`<br>`AGENT-CAP-034`<br>`ROUTING-CAP-037`<br>`TOOL-SYS-043` | `BANKED` / `PROOF_USER` |
| `FORGEOS-V1-AGENT-400` | `NYX-GATE-FORGEOS-V1-AGENT-400` | `AGENT-INTEG-040`<br>`AGENT-INTEG-041`<br>`ROUTING-CAP-037`<br>`TOOL-SYS-043`<br>`POLICY-CAP-043` | `BANKED` / `PROOF_USER` |

`FORGEOS-V1-AGENT-201` and `FORGEOS-V1-PATCH-300` add no new direct Nyx gate.
They consume the already-gated returned artifact and own ForgeOS-side hash/base
validation, review, rejection, application, and local verification.

---

## 3. Repository ownership firewall

### ForgeOS may own

```text
client transport and protocol adapters
Forge project/revision/worktree/scope envelopes
service-process supervision
user controls and presentation
registered-command declarations
Git isolation, patch review, local apply, validation, and commit
independent comparison of Nyx claims against source and real tools
```

### Nyx_Server must own

```text
health and capability truth
model/backend catalogs and route selection
model hosting and model calls
sessions, threads, conversations, and run identity
context assembly and repository tool execution
policy, permissions, checkpoints, approvals, and resume semantics
agent loops, budgets, remote provider calls, and run records
inert patch proposal generation and Nyx-side audit evidence
```

### Forbidden ForgeOS substitutes

```text
server handlers or model runtime inside forge-nyx-client
calling Ollama/OpenAI/other model providers around Nyx
Forge-owned canonical Nyx session or run stores
Forge-owned tool/policy/checkpoint engines
reading files on Nyx's behalf and presenting the result as a Nyx tool call
regenerating a gated command after approval
Nyx patch auto-application or provider prose treated as local proof
```

---

## 4. Handoff rule

When a ForgeOS Nyx-consuming skill hits a blocker, classify it before editing:

```text
missing/incorrect client transport, adapter, UI, process supervision,
project envelope, worktree, review, or local validation
  -> patch Forge_OS_V1 only

missing/incorrect health, API contract, model/session behavior, tool execution,
policy, checkpoint, agent, routing, persistence, or Nyx evidence
  -> patch Nyx_Server only
```

The handoff must name:

```text
FORGE_SKILL_ID
NYX_GATE_ID
MISSING_NYX_SKILL_IDS
CURRENT_NYX_LEDGER_STATE
FIRST_BLOCKER
TARGET_REPOSITORY
ALLOWED_PATHS
RETURN_WITNESS
```

Do not solve a missing Nyx skill by widening an allowed ForgeOS path.
