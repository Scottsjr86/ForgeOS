# FORGEOS-V1-NYX-101 Nyx Gate Request

Status: `VERIFIED_RETURNED`
Gate ID: `NYX-GATE-FORGEOS-V1-NYX-101`
Target repository: `Nyx_Server`
Verified return: `docs/versions/V1/skills/FORGEOS-V1-NYX-101/NYX_GATE_INPUT.json`

## Required Nyx skills

Each skill must be `BANKED` or `RELEASE_EARNED` with at least `PROOF_SYSTEM`:

```text
POLICY-CHECKPOINT-001
POLICY-APPROVAL-001
POLICY-TOOL-015
PERSIST-IDEMP-KEY-001
```

## Required general server behavior

Nyx_Server must remain an independent headless AI server for chat, development,
models, sessions, tools, agents, memory, policy, and general API clients. The gate
must expose Nyx-owned, versioned behavior for:

- scoped tool authority and exact request identity;
- immutable checkpoint payloads;
- explicit approval, denial, and expiration;
- one exact resume token bound to the approved payload;
- idempotent replay protection across restart;
- truthful audit records and deterministic rejection of altered requests.

Do not add ForgeOS dependencies or a private one-client permission protocol. ForgeOS
will later consume the public Nyx contract as a client and must not mint checkpoints,
reinterpret approvals, regenerate gated actions, or keep a second permission engine.

## Return evidence

Return to the ForgeOS integration thread only with:

```text
NYX_GATE_ID=NYX-GATE-FORGEOS-V1-NYX-101
BANKED_SKILLS=[POLICY-CHECKPOINT-001,POLICY-APPROVAL-001,POLICY-TOOL-015,PERSIST-IDEMP-KEY-001]
MINIMUM_PROOF_LEVEL=PROOF_SYSTEM
PUBLIC_CONTRACT_VERSION=<actual version>
PUBLIC_ENDPOINTS_OR_SURFACES=<actual Nyx-owned surfaces>
REAL_SERVER_WITNESS_COMMAND=<exact command and result>
CI_COMMAND=<exact command and result>
PROOF_RECEIPTS=<exact paths and SHA-256 values>
STANDALONE_CHAT_DEV_RESULT=<actual result>
NEXT_ACTION=RETURN_TO_FORGEOS_INTEGRATION_THREAD
```
