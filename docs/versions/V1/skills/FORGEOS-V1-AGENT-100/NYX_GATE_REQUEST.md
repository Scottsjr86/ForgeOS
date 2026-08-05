# FORGEOS-V1-AGENT-100 Nyx Gate Request

Status: `VERIFIED_RETURNED`
Gate ID: `NYX-GATE-FORGEOS-V1-AGENT-100`
Target repository: `Nyx_Server`
Verified return: `docs/versions/V1/skills/FORGEOS-V1-AGENT-100/NYX_GATE_INPUT.json`

## Required Nyx skills

Each skill must be `BANKED` or `RELEASE_EARNED` with at least `PROOF_SYSTEM`:

```text
AGENT-FOUND-002
AGENT-RUN-001
AGENT-BUDGET-001
ROUTING-COST-001
PERSIST-RUN-001
```

## Required general server behavior

Nyx_Server must remain an independent headless AI server for chat, development,
models, sessions, tools, agents, memory, policy, and general API clients. The gate
must expose Nyx-owned, versioned behavior for:

- stable task and run identity;
- explicit provider and model attribution;
- exact source revision, worktree, and scope binding;
- declared token, cost, time, or other supported budget posture;
- complete, failed, cancelled, and budget-hit terminal states;
- cancellation that prevents later continuation;
- durable response, status, cost, and run records across restart;
- isolation between unrelated tasks and worktrees.

Do not add ForgeOS dependencies or a private one-client agent protocol. ForgeOS will
later consume the public Nyx contract as a client and must not contact providers
directly, invent terminal run state, calculate cost from prose, mutate the canonical
worktree, or keep a second agent-run ledger.

## Return evidence

Return to the ForgeOS integration thread only with:

```text
NYX_GATE_ID=NYX-GATE-FORGEOS-V1-AGENT-100
BANKED_SKILLS=[AGENT-FOUND-002,AGENT-RUN-001,AGENT-BUDGET-001,ROUTING-COST-001,PERSIST-RUN-001]
MINIMUM_PROOF_LEVEL=PROOF_SYSTEM
PUBLIC_CONTRACT_VERSION=<actual version>
PUBLIC_ENDPOINTS_OR_SURFACES=<actual Nyx-owned surfaces>
REAL_SERVER_WITNESS_COMMAND=<exact command and result>
CI_COMMAND=<exact command and result>
PROOF_RECEIPTS=<exact paths and SHA-256 values>
STANDALONE_CHAT_DEV_RESULT=<actual result>
NEXT_ACTION=RETURN_TO_FORGEOS_INTEGRATION_THREAD
```
