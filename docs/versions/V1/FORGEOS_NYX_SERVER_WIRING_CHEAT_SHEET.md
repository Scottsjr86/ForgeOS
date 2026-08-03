# ForgeOS to Nyx Server wiring cheat sheet

Status: `MANDATORY_NYX_WORK_PREFLIGHT`
Document ID: `FORGEOS-NYX-WIRING-CHEAT-SHEET-V1`
Applies to: every ForgeOS slice that touches Nyx transport, lifecycle, models,
chat, tools, policy, checkpoints, context, memory, runtime control, agent work,
run evidence, or Nyx-facing UI
Capability and ownership companion:
`docs/versions/V1/FORGEOS_NYX_SERVER_CAPABILITY_OWNERSHIP_CHEAT_SHEET.md`
Cross-repository closure authority:
`docs/versions/V1/FORGEOS_V1_NYX_SERVER_DEPENDENCY_CONTRACT.md`
Machine gate map:
`docs/versions/V1/FORGEOS_V1_NYX_SERVER_DEPENDENCIES.json`
Nyx source snapshot inspected for this revision: `Nyx_Server_base_5`

---

## 0. Mandatory use

This file must be read before any ForgeOS Nyx work begins.

A patching model must report this preflight before editing Nyx-facing ForgeOS
source:

```text
NYX_WIRING_REVIEWED=docs/versions/V1/FORGEOS_NYX_SERVER_WIRING_CHEAT_SHEET.md
NYX_OWNERSHIP_REVIEWED=docs/versions/V1/FORGEOS_NYX_SERVER_CAPABILITY_OWNERSHIP_CHEAT_SHEET.md
NYX_CROSS_REPO_CONTRACT_REVIEWED=docs/versions/V1/FORGEOS_V1_NYX_SERVER_DEPENDENCY_CONTRACT.md
NYX_TARGET_REPOSITORY=Forge_OS_V1|Nyx_Server
NYX_SURFACE_TOUCHED=<exact endpoint, DTO, process seam, or Forge adapter>
NYX_CANONICAL_OWNER=FORGE_CORE|FORGE_ADAPTER|NYX_SERVER|REAL_TOOL|LINUX_HOST
NYX_ENGINE_FAMILY=API|AGENT|CONTEXT|MEMORY|OBSERVABILITY|PERSISTENCE|POLICY|REPO|ROUTING|TEMPORAL|TOOLS|WORKFLOW|NONE
NYX_IMPLEMENTATION_STATE=REAL|PARTIAL|CONTRACT_ONLY|PLANNED|MISSING
NYX_SOURCE_FILES_CHECKED=<exact current Nyx source files>
NYX_FORGE_ALLOWED_PATHS=<exact ForgeOS paths>
NYX_DUPLICATE_IMPLEMENTATION_CHECK=PASS
NYX_FORBIDDEN_SUBSTITUTE_CONFIRMED=YES
```

Missing preflight means stop before source edits.

This file is a ForgeOS integration map. It is not authority to change Nyx
behavior. Current Nyx source and real Nyx process output outrank this copied
map. When current Nyx source differs from this file:

```text
inspect the live Nyx source
  -> classify whether Nyx or Forge owns the change
  -> patch the owning repository only
  -> update this cheat sheet in the ForgeOS patch
  -> never preserve stale Forge behavior by cloning Nyx into ForgeOS
```

---

## 1. The seam in one screen

```text
ForgeOS
  owns:
    Forge project identity and canonical project state
    project/repository/revision/worktree/scope envelopes
    Nyx client and service-process adapter
    UI and operator controls
    registered local command declarations
    independent Git, build, test, and patch verification

        HTTP/JSON over configured local TCP
        current Nyx default: http://127.0.0.1:8088
        versioned JSON DTOs and response headers

Nyx_Server
  owns:
    server health and version truth
    backend and model catalogs
    model calls
    sessions, threads, context, tools, policy, checkpoints
    managed model-runtime control
    agent execution and run records
    Nyx memory ledger and Nyx-side audit evidence

External model runtime
  current supported backend shapes:
    Ollama, default http://127.0.0.1:11434
    OpenAI-compatible runtime, default http://127.0.0.1:8080
    optionally Nyx-managed llama-server from allowlisted GGUF weights
```

ForgeOS must talk to Nyx. ForgeOS must not talk around Nyx to Ollama,
llama-server, an OpenAI-compatible runtime, or a remote model provider.

---

## 2. Current transport reality

### 2.1 Nyx public server transport

The inspected Nyx server currently binds an Axum HTTP server to:

```text
NYX_BIND, default 127.0.0.1:8088
```

The public route set is HTTP/JSON under `/v1`.

Current Nyx does not expose a Unix-domain-socket server and does not implement
the Forge-only `FGNYXQ` / `FGNYXR` binary frame protocol.

### 2.2 Current Forge client mismatch

The current Forge crate `crates/forge-nyx-client` implements:

```text
4-byte big-endian frame length
  -> FGNYXQ binary handshake request
  -> Unix socket or TCP exchange
  -> FGNYXR binary handshake response
```

That implementation is fixture-proved only. It has no matching handler in the
inspected Nyx server.

Therefore:

```text
Forge binary fixture passes != ForgeOS and Nyx are integrated
```

The preferred V1 repair is:

```text
Nyx owns a canonical versioned HTTP/JSON health and capability contract
  -> Forge consumes it through forge-nyx-client
  -> Forge classifies unavailable, incompatible, unhealthy, and ready
  -> real Nyx process witness closes the seam
```

A future Nyx-owned binary transport is legal only after Nyx defines and
implements it. ForgeOS may not create the server half inside ForgeOS.

### 2.3 Base URL handling

ForgeOS should treat the Nyx endpoint as a configured origin, not a hardcoded
provider address:

```text
http://127.0.0.1:8088
```

ForgeOS must:

- preserve the configured scheme, host, and port;
- append only documented Nyx paths;
- use bounded connect, read, and total request timeouts;
- cap response body size before JSON decoding;
- reject malformed JSON and incompatible schema claims;
- distinguish connection failure from an HTTP error response;
- never turn transport success alone into readiness;
- never discover or call the backend URL returned by Nyx as a shortcut.

---

## 3. Nyx process startup and ownership

### 3.1 Starting Nyx itself

The `nyx_server` process is the service ForgeOS may later supervise.

ForgeOS service supervision may own:

```text
configured executable path
configured working directory
configured environment
start, stop, restart, and crash observation for nyx_server itself
stdout/stderr capture
PID/process-group lifecycle
readiness polling against Nyx public HTTP
operator-visible unavailable/degraded state
```

ForgeOS service supervision must not own:

```text
Nyx sessions or threads
Nyx backend registry
Nyx model runtime state
Nyx policy
Nyx checkpoints
Nyx run records
Nyx memory ledger
```

The Nyx endpoints `/v1/nyx/runtime/*` manage the model runtime behind Nyx,
currently llama-server. They do not start or stop the `nyx_server` process.
Do not confuse these two process layers.

### 3.2 Nyx boot behavior relevant to ForgeOS

At boot, Nyx currently:

1. parses `NYX_BIND`;
2. resolves the server-owned workspace root;
3. loads strict policy configuration;
4. creates a session and a stateless thread;
5. opens or creates the Nyx memory ledger;
6. configures one primary model backend without calling it;
7. optionally builds a GGUF registry from allowlisted roots;
8. optionally enables model-runtime spawn controls;
9. starts the HTTP listener;
10. responds to control-plane routes even when the model backend is down.

ForgeOS readiness must not require `/v1/models` or chat success when the intended
check is only "is Nyx server alive?" Backend health and server health are
separate states.

### 3.3 Graceful shutdown

The inspected Nyx main process listens for Ctrl-C and uses Axum graceful
shutdown. A ForgeOS manager should first send the configured graceful signal,
wait a bounded grace period, then escalate according to ForgeOS managed-process
law. ForgeOS must record that it stopped Nyx; it must not rewrite Nyx's internal
run or runtime records to make shutdown look clean.

---

## 4. Environment wiring ForgeOS may configure

These variables are current Nyx inputs. ForgeOS may present and pass them. Nyx
owns their interpretation and validation.

### 4.1 Server, workspace, and ledger

| Variable | Current default or shape | ForgeOS meaning |
|---|---|---|
| `NYX_BIND` | `127.0.0.1:8088` | Nyx HTTP listener. Keep loopback for V1 unless Nyx policy explicitly changes. |
| `NYX_WORKSPACE_ROOT` | current Nyx process directory | Server-owned workspace sandbox root. ForgeOS should pass the exact active project/worktree root intended for Nyx. |
| `NYX_LEDGER_DIR` | `.nyx/ledger` | Nyx memory ledger location. Nyx owns content and schema. |
| `NYX_DEBUG_LOG` | enabled only when exactly `1` | Adds bounded request lifecycle debug events. Do not enable silently. |
| `RUST_LOG` | tracing subscriber filter | Optional operator diagnostics. Never parse log prose as protocol truth. |

### 4.2 Primary backend

| Variable | Current default or allowed values | ForgeOS meaning |
|---|---|---|
| `NYX_BACKEND_PRIMARY_KIND` | `ollama`; allowed `ollama` or `openai_compat` | Chooses Nyx's model backend adapter. ForgeOS does not instantiate the adapter. |
| `NYX_BACKEND_PRIMARY_URL` | Ollama: `http://127.0.0.1:11434`; OpenAI-compatible: `http://127.0.0.1:8080` | Backend origin consumed by Nyx only. ForgeOS must not call it around Nyx. |

Nyx boot does not require the backend to be reachable. `/v1/models` and
`/v1/chat/completions` may fail later with backend errors while
`/v1/nyx/health` still reports the server process alive.

### 4.3 Policy

| Variable | Current accepted value | Behavior |
|---|---|---|
| `NYX_POLICY_PROFILE` | absent or `phase0_strict` | Any other value fails startup. |
| `NYX_POLICY_NETWORK` | absent or `deny` | Any weakening value fails startup. |

Current strict policy includes:

```text
outbound network denied by default
safe read tools allowed inside the sandbox
workspace writes require checkpoint
process tools require checkpoint
network tools denied
managed runtime spawn requires explicit enablement
request payloads cannot disable the sandbox
```

ForgeOS must display Nyx policy results. It must not duplicate the policy engine
and call the duplicate authoritative.

### 4.4 GGUF discovery and managed llama-server

| Variable | Current shape | Behavior |
|---|---|---|
| `NYX_MODEL_ROOTS` | colon-separated absolute paths | Non-absolute entries are ignored. Roots constrain GGUF discovery and runtime model resolution. |
| `NYX_RUNTIME_ALLOW_SPAWN` | enabled only when exactly `1` | Required for runtime start and restart. Default is disabled. |
| `NYX_RUNTIME_ALLOW_TCP_LAN` | enabled only when exactly `1` | Allows managed runtime LAN bind. Default is loopback-only. |
| `NYX_LLAMA_SERVER_BIN` | absolute executable path | Required before Nyx can spawn llama-server. |
| `NYX_RUNTIME_DEFAULT_CTX` | positive integer, default `4096` | Default runtime profile context size. |
| `NYX_RUNTIME_DEFAULT_THREADS` | positive integer, default `4` | Default runtime profile thread count. |
| `NYX_RUNTIME_DEFAULT_GPU_LAYERS` | `0` or absent means none | Optional default GPU layer count. |
| `NYX_RUNTIME_DEFAULT_BATCH` | `0` or absent means none | Optional default batch size. |
| `NYX_RUNTIME_DEFAULT_SEED` | optional unsigned integer | Optional deterministic seed. |
| `NYX_RUNTIME_INSTANCE_SPEC_JSON` | serialized `RuntimeInstanceSpec` | Parsed at boot as a reserved configuration surface; current server does not use it to auto-start a runtime. |
| `NYX_RUNTIME_STARTUP_TIMEOUT_MS` | optional milliseconds | Runtime supervisor startup timeout override. |
| `NYX_RUNTIME_SHUTDOWN_TERM_GRACE_MS` | optional milliseconds | Grace before escalation. Legacy alias: `NYX_RUNTIME_SHUTDOWN_TERM_MS`. |
| `NYX_RUNTIME_SHUTDOWN_KILL_GRACE_MS` | optional milliseconds | Kill grace. Legacy alias: `NYX_RUNTIME_SHUTDOWN_KILL_MS`. |
| `NYX_RUNTIME_CWD` | optional path | Runtime child working directory. Nyx owns validation and use. |
| `NYX_RUNTIME_ENV_ALLOWLIST` | optional list interpreted by Nyx | Additional runtime child environment allowlist. Nyx always treats `NYX_` and `LLAMA_` prefixes specially. |
| `NYX_DEBUG_RUNTIME` | presence enables debug path | Development diagnostics only. |
| `NYX_DEBUG_MODEL_REGISTRY` | presence enables debug path | Development diagnostics only. |

ForgeOS must not expose raw arbitrary llama-server arguments as a substitute for
Nyx's typed `RuntimeProfile`, `RuntimeInstanceSpec`, and structured launch hints.

### 4.5 Test-only or development capture variables

The inspected source also contains N1 rerun capture variables used by test or
capture paths:

```text
NYX_N1_RERUN_CAPTURE_PATH
NYX_N1_RERUN_WORKSPACE_ROOT
```

They are not ordinary ForgeOS product configuration and must not become the
production integration path.

---

## 5. HTTP request and response rules

### 5.1 Content type

JSON request bodies should use:

```text
Content-Type: application/json
```

ForgeOS must reject a response that claims success but cannot be decoded as the
expected JSON contract.

### 5.2 Trace correlation

ForgeOS may send:

```text
x-nyx-trace-id: <non-empty caller trace id>
```

Nyx returns:

```text
x-nyx-trace-id: <accepted or generated trace id>
x-nyx-diagnostic-id: <Nyx-generated diagnostic id>
```

ForgeOS should preserve these values in its own operation record and UI. They
are correlation metadata, not proof of success.

### 5.3 Run correlation

Successful chat responses currently place Nyx identity under:

```json
{
  "metadata": {
    "nyx": {
      "session_id": "...",
      "thread_id": "...",
      "run_id": "...",
      "finish_reason": "..."
    }
  }
}
```

The response `id` is also the Nyx run ID.

Server-side chat errors that occur after a run exists may include:

```text
x-nyx-run-id: <run id>
```

and:

```json
error.metadata.nyx.run_id
```

ForgeOS should store the run ID and use Nyx run-introspection routes. ForgeOS
must not generate a replacement run ID and present it as Nyx identity.

### 5.4 Authentication posture

The inspected router has no authentication middleware and currently uses
permissive CORS. This is not permission to expose Nyx publicly.

For V1:

```text
bind Nyx to loopback
  -> treat local process ownership as the current deployment boundary
  -> do not invent an Authorization header contract
  -> do not bind to LAN or public interfaces unless Nyx defines and proves auth
```

When Nyx later adds authentication, ForgeOS must consume the Nyx-owned contract.
ForgeOS may not bolt on a Forge-only auth fiction and call the seam secured.

---

## 6. Complete current public route inventory

The route list below is copied from the inspected Nyx router. A route being
listed means a handler exists in that snapshot. It does not mean every ForgeOS
V1 skill depending on it is closed.

### 6.1 Server health and diagnostics

| Method | Path | Current purpose | ForgeOS use |
|---|---|---|---|
| `GET` | `/v1/nyx/health` | Process health with `status`, crate `version`, and `boot_ts`. | Basic server liveness and version discovery. Current response lacks `schema_version`, `schema_id`, and canonical capability list, so it is not yet the complete desired Forge V1 handshake. |
| `GET` | `/v1/nyx/diagnostics` | Schema-stamped bounded server diagnostics, request lifecycle counts, recent lifecycle events, and debug-log state. | Operator diagnostics. Never use recent log prose as capability truth. |
| `GET` | `/v1/nyx/backends` | Stable catalog of configured Nyx backend name, kind, and base URL without model calls. | Display configuration and diagnose routing. Do not call returned backend URLs. |

Current health example:

```json
{
  "status": "ok",
  "version": "0.1.0",
  "boot_ts": "2026-08-03T00:00:00Z"
}
```

ForgeOS compatibility must eventually validate a Nyx-owned schema/contract
version, not merely the server package version.

### 6.2 OpenAI-shaped compatibility routes

| Method | Path | Current purpose | ForgeOS use |
|---|---|---|---|
| `GET` | `/v1/models` | Calls configured backend adapters, returns sorted OpenAI-shaped model items, and refreshes Nyx model-to-backend routing. | Model picker. Backend failure is not Nyx process death. |
| `POST` | `/v1/chat/completions` | Non-streaming OpenAI-shaped chat request normalized into Nyx `AgentRequest`; builds context, exposes tool catalog, runs one bounded Nyx agent pass, stores run artifacts, and shapes an OpenAI-compatible response. | Main V1 chat seam after model selection and required Nyx gates. |

Current model-list response shape:

```json
{
  "object": "list",
  "data": [
    {
      "id": "model-id",
      "object": "model",
      "owned_by": "nyx"
    }
  ]
}
```

Current chat request shape is intentionally small:

```json
{
  "model": "model-id",
  "messages": [
    {"role": "system", "content": "..."},
    {"role": "user", "content": "..."}
  ],
  "stream": false
}
```

Accepted message content is a string or a supported text-parts array. The
request must contain at least one user message. Model IDs may not be empty or
contain surrounding whitespace.

Current chat response shape:

```json
{
  "id": "run_...",
  "object": "chat.completion",
  "created": 1780000000,
  "model": "model-id",
  "choices": [
    {
      "index": 0,
      "message": {
        "role": "assistant",
        "content": "..."
      },
      "finish_reason": "stop"
    }
  ],
  "metadata": {
    "nyx": {
      "session_id": "session_...",
      "thread_id": "thread_...",
      "run_id": "run_...",
      "finish_reason": "stop"
    }
  }
}
```

Current streaming posture:

```text
stream=false -> supported
stream=true  -> 400 invalid_request
```

ForgeOS must not advertise streaming until Nyx owns a deterministic streaming
contract and the required Nyx skill gates are banked.

### 6.3 Tools and sandbox

| Method | Path | Current purpose | ForgeOS use |
|---|---|---|---|
| `GET` | `/v1/nyx/tools` | Returns the schema-stamped `ToolRegistrySnapshot` directly, with descriptors projected through current policy. | Build tool visibility and approval UI from Nyx truth. |
| `GET` | `/v1/nyx/tools/report` | Operator report with registry diagnostics, safety level, enabled state, checkpoint requirement, side effects, and limits. | Explain why a tool is available, denied, or gated. |
| `GET` | `/v1/nyx/sandbox/report` | Effective workspace root, symlink policy, excludes, byte and traversal limits. | Show and verify the sandbox Nyx will actually use. |

Current built-in tool names are:

```text
repo.read_file
repo.read_snippet
repo.list
repo.grep
repo.search_snippets
```

All current built-ins are safe-read repository tools. Their descriptors include
JSON argument and result schemas and `reads_files` side effects.

ForgeOS rules:

```text
render Nyx's descriptor and effective policy
  -> send user intent through Nyx-owned agent/tool flow
  -> inspect Nyx run and tool-audit records
  -> independently verify important file/Git/build claims
```

ForgeOS must not perform a file read itself and relabel the result as a Nyx tool
execution. ForgeOS may independently read the same file for verification, but
that is Forge evidence and must remain labeled as Forge evidence.

### 6.4 Policy

| Method | Path | Current purpose | ForgeOS use |
|---|---|---|---|
| `GET` | `/v1/nyx/policy` | Returns the current schema-stamped policy snapshot directly. | Canonical policy display and decision context. |
| `GET` | `/v1/nyx/policy/report` | Operator-oriented effective defaults plus recent checkpoint decision context. | Explain policy posture and recent gates. |
| `GET` | `/v1/nyx/runs/:run_id/policy_explain` | Stable explanations of allow, deny, checkpoint, or missing-policy entries from a run record. | Per-run approval and denial explanation. |

ForgeOS must not infer permission from a tool descriptor alone. The descriptor,
effective policy, request-specific decision, checkpoint artifact, and final tool
result are distinct records.

### 6.5 Snapshots and temporal inspection

| Method | Path | Current purpose | ForgeOS use |
|---|---|---|---|
| `GET` | `/v1/nyx/snaps` | Lists stable, read-only snapshots under `.nyx/snaps` within the active workspace. | Nyx snapshot browser. |
| `GET` | `/v1/nyx/snaps/diff?from=<id>&to=<id>` | Produces bounded, hash-verified, path-ordered snapshot differences. | Nyx temporal comparison UI and evidence. |

These are Nyx snapshots, not Forge canonical project snapshots. ForgeOS may
compare them with Forge state but must not silently merge the authorities.

### 6.6 GGUF model registry

| Method | Path | Current purpose | ForgeOS use |
|---|---|---|---|
| `GET` | `/v1/nyx/models/gguf` | Lists stable GGUF model registry entries from allowlisted roots. | Local weight browser and runtime-start input. |
| `POST` | `/v1/nyx/models/gguf/rescan` | Rescans allowlisted roots and returns the updated snapshot. | Explicit operator refresh. |

Registry entries include:

```text
model id
absolute path
human display name
tags
optional SHA-256
optional structured llama.cpp launch hints
```

ForgeOS may display the path and hash returned by Nyx. ForgeOS must not let a
model request substitute an arbitrary path outside Nyx's allowlisted roots.

### 6.7 Managed model runtime

| Method | Path | Input | Current purpose |
|---|---|---|---|
| `GET` | `/v1/nyx/runtime/status` | none | Current managed runtime status or null status. |
| `GET` | `/v1/nyx/runtime/events?n=<1..500>` | optional tail count | Recent runtime lifecycle events. |
| `GET` | `/v1/nyx/runtime/logs?instance_id=<id>&n=<1..5000>` | instance and optional tail count | Redacted runtime log tail. |
| `POST` | `/v1/nyx/runtime/start` | `{"spec": RuntimeInstanceSpec}` | Policy-gated spawn of a configured llama-server runtime. |
| `POST` | `/v1/nyx/runtime/stop?instance_id=<id>` | query parameter | Stops the named runtime instance. |
| `POST` | `/v1/nyx/runtime/restart` | `{"spec": RuntimeInstanceSpec}` | Policy-gated restart through the runtime supervisor. |

`RuntimeInstanceSpec` currently contains:

```json
{
  "schema_version": "nyx.1.0",
  "schema_id": "nyx.runtime_instance_spec.v1",
  "kind": "llama_cpp",
  "model_id": "...",
  "profile_id": "default",
  "transport": {
    "kind": "tcp_loopback",
    "host": "127.0.0.1",
    "port": 0
  }
}
```

or:

```json
{
  "kind": "uds",
  "path": "/absolute/path/to/runtime.sock"
}
```

Runtime start requires:

```text
NYX_RUNTIME_ALLOW_SPAWN=1
NYX_LLAMA_SERVER_BIN configured as an absolute existing path
model ID present in Nyx's GGUF registry
model path under canonical allowlisted roots
known runtime profile ID
allowed runtime transport
```

Nyx may resolve port `0` to an actual loopback port and returns resolved status.
ForgeOS must consume returned runtime status rather than assuming the requested
transport is final.

### 6.8 Memory and run inventory

| Method | Path | Current purpose | ForgeOS use |
|---|---|---|---|
| `GET` | `/v1/nyx/memory/status` | Ledger path/count/status summary and event scope counts. | Nyx persistence diagnostics. |
| `GET` | `/v1/nyx/runs/recent` | Stable ledger-backed recent run summaries. | Run history. |
| `GET` | `/v1/nyx/runs/:run_id` | Run summary plus ledger events. | Run details. |
| `GET` | `/v1/nyx/runs/:run_id/trace` | Cached run record plus related ledger events and trace counts. | Full operator trace. |
| `GET` | `/v1/nyx/runs/:run_id/tool_audit` | Stable tool-call audit entries from cached run record. | Tool review and proof. |
| `GET` | `/v1/nyx/runs/:run_id/file_touches` | Stable file-touch records from cached run record. | Proposed or executed file impact inspection. |
| `GET` | `/v1/nyx/runs/:run_id/context_bundle` | Exact cached `ContextBundleV1` used by the run. | Show what Nyx actually supplied to the model. |
| `GET` | `/v1/nyx/runs/:run_id/context_selection_report` | Exact cached context-selection report. | Explain selection, ordering, exclusions, and provenance. |
| `GET` | `/v1/nyx/runs/:run_id/checkpoints` | Checkpoint records and requests attached to the run. | Approval queue and history. |

Important lifetime split:

```text
ledger-backed run summary and events may survive
in-process run_records/context_bundles/context_selection_reports caches do not
currently imply durable restoration across Nyx restart
```

ForgeOS must treat a 404 from a detail route after restart as "Nyx detail is not
available in this process state," not fabricate the missing detail from chat
history.

### 6.9 Checkpoint resolution

| Method | Path | Input | Current behavior |
|---|---|---|---|
| `POST` | `/v1/nyx/checkpoints/:checkpoint_id/resolve` | decision plus optional actor and reason | Records approve or deny in Nyx session and cached run state. Current N1 does not resume or execute the paused tool. |

Request shape:

```json
{
  "decision": "approve",
  "resolved_by": "operator-id",
  "reason": "bounded explanation"
}
```

or decision `deny`.

Current response includes:

```text
checkpoint_id
run_id
decision record
updated checkpoint record
execution_resumed=false
```

This is crucial. ForgeOS must not regenerate the tool request and execute it
locally after approval. Exact approval-resume semantics belong to a later Nyx
capability and its cross-repository gate.

---

## 7. Error contracts ForgeOS must preserve

### 7.1 OpenAI-shaped compatibility errors

`/v1/models` and `/v1/chat/completions` use:

```json
{
  "error": {
    "message": "public safe message",
    "type": "...",
    "param": null,
    "code": "...",
    "metadata": {
      "nyx": {
        "code": "NyxErrorCode",
        "retryable": false,
        "details": null,
        "run_id": "optional"
      }
    }
  }
}
```

Current HTTP mapping:

| Nyx error | HTTP | Compatibility code |
|---|---:|---|
| invalid request | `400` | `invalid_request` |
| unauthorized | `401` | `unauthorized` |
| backend unavailable | `502` | `model_backend_unavailable` |
| model call failed | `502` | `model_call_failed` |
| tool not found | `400` | `tool_not_found` |
| tool denied | `403` | `tool_denied` |
| tool execution failed | `500` | `tool_execution_failed` |
| checkpoint required | `409` | `checkpoint_required` |
| sandbox violation | `403` | `sandbox_violation` |
| state conflict | `409` | `state_conflict` |
| persistence error | `500` | `persistence_error` |
| internal error | `500` | `internal_error` |

Nyx intentionally redacts internal server error messages and details. ForgeOS
must not replace a safe public error with captured stderr secrets.

### 7.2 Nyx-native errors

Many `/v1/nyx/*` routes use a Nyx-native envelope:

```json
{
  "error": {
    "code": "InvalidRequest",
    "message": "...",
    "details": null
  }
}
```

Runtime lifecycle errors may also include a `policy_decision` sibling.

ForgeOS must decode by route family. It must not assume every Nyx route uses the
OpenAI-shaped error envelope.

### 7.3 Required Forge classification

Forge should classify at least:

```text
UNAVAILABLE
  DNS/connect/refused/timeout before a valid HTTP response

HTTP_ERROR
  valid Nyx HTTP response with non-success status and decodable error

MALFORMED
  response body cannot satisfy the expected route contract

INCOMPATIBLE
  schema/contract version is unsupported or required fields/capabilities conflict

UNHEALTHY_OR_DEGRADED
  Nyx or managed runtime explicitly reports a non-ready health state

READY
  expected Nyx-owned contract validates and required capabilities are present
```

Do not collapse all failures into "Nyx offline."

---

## 8. Schema and DTO ownership

The Nyx protocol crate currently owns public DTOs for:

```text
agent request and response
checkpoint request and decision
context bundle and selection report
policy snapshot and decision
tool descriptor, registry snapshot, call, result, and audit
run record
snapshot list and diff reports
runtime model registry, profile, instance spec, health, status, capabilities, events
OpenAI-shaped model, chat, and error envelopes
IDs for session, thread, run, tool call, checkpoint, model, and runtime instance
```

Current schema epoch is:

```text
nyx.1.0
```

Representative schema IDs include:

```text
nyx.agent_request.v1
nyx.agent_response.v1
nyx.checkpoint_request.v1
nyx.checkpoint_decision.v1
nyx.context_bundle.v1
nyx.context_selection_report.v1
nyx.policy_snapshot.v1
nyx.policy_decision.v1
nyx.tool_descriptor.v1
nyx.tool_registry_snapshot.v1
nyx.tool_call.v1
nyx.tool_result.v1
nyx.run_record.v1
nyx.snap_list_report.v1
nyx.snap_diff_report.v1
nyx.runtime_instance_spec.v1
nyx.runtime_status.v1
nyx.runtime_event.v1
```

A Rust type existing in `nyx_protocol` does not prove a public HTTP route exists.
A public route existing does not prove Forge's required Nyx skill is banked.
A Forge copy of a Nyx DTO does not transfer schema ownership to Forge.

Forge adapter strategy:

```text
prefer a small Forge-owned client DTO that mirrors the exact public Nyx wire
  -> retain unknown namespaced metadata when useful
  -> reject incompatible required fields
  -> keep Nyx schema/version strings visible
  -> do not import Nyx server internals into ForgeOS
```

If a shared protocol crate is ever vendored or published, that requires an
explicit versioned decision. Do not casually add a path dependency from ForgeOS
to a sibling Nyx checkout.

---

## 9. Workspace and repository wiring

Nyx creates one server-owned session at boot using `NYX_WORKSPACE_ROOT` and a
stateless thread. Current `/v1/chat/completions` uses that active session/thread.
There is no current public endpoint for ForgeOS to create or switch sessions or
threads.

For the current V1 seam, ForgeOS should:

```text
start Nyx with NYX_WORKSPACE_ROOT=<exact active Forge worktree root>
  -> verify /v1/nyx/sandbox/report returns the expected canonical workspace root
  -> verify policy and tool catalogs
  -> use chat and run routes
```

When the active Forge project or worktree changes, ForgeOS may need to restart or
reconfigure Nyx until Nyx owns explicit session/workspace switching endpoints.
ForgeOS must not mutate a hidden Forge-side "Nyx session" and pretend Nyx switched.

The current sandbox rejects traversal, absolute or prefixed request paths where
forbidden, excluded directories, unsafe symlink behavior, over-budget reads,
and paths outside the configured workspace.

ForgeOS project identity remains authoritative for which worktree the user
selected. Nyx's sandbox report remains authoritative for what Nyx can currently
read. A mismatch is a blocking integration error.

---

## 10. Model wiring

There are two distinct model catalogs:

### 10.1 Backend model catalog

```text
GET /v1/models
```

This asks configured backend adapters for models available for chat and updates
Nyx's model-to-backend map.

### 10.2 Local GGUF registry

```text
GET /v1/nyx/models/gguf
```

This lists local GGUF files under allowlisted roots for the managed runtime.

These lists may differ. ForgeOS must label them correctly.

A model may appear in the GGUF registry but not yet be available through
`/v1/models` until a runtime is started and the configured backend points at it.
A backend model may appear in `/v1/models` without being a Nyx-managed local GGUF.

Do not merge by display name. Preserve Nyx-provided IDs and source catalog.

---

## 11. Context, tools, and evidence wiring

Current chat execution does more than forward text:

```text
OpenAI-shaped request
  -> Nyx validation and normalization
  -> active session/workspace lookup
  -> Nyx tool registry and policy projection
  -> deterministic repository snippet retrieval
  -> ContextBundleV1 and selection report
  -> one bounded agent pass against the selected backend
  -> optional tool validation, policy decision, and safe-read execution
  -> RunRecord plus ledger events
  -> OpenAI-shaped response with Nyx run identity
```

ForgeOS must provide interfaces to inspect the actual Nyx records instead of
reconstructing a story from assistant prose.

Minimum evidence links for a run UI:

```text
chat response run_id
  -> run detail
  -> run trace
  -> context bundle
  -> context selection report
  -> tool audit
  -> file touches
  -> policy explanation
  -> checkpoints
```

ForgeOS should separately attach its own evidence:

```text
current Forge project and worktree identity
current Git revision
independent file hashes
independent diff
registered command IDs and exact outputs
user approval/rejection
```

Nyx evidence and Forge evidence should cross-check one another. Neither should be
silently relabeled as the other.

---

## 12. What is implemented, contract-only, and missing

### 12.1 Currently implemented public seams in inspected Nyx source

```text
HTTP listener on configured TCP bind
server health and diagnostics
backend catalog
OpenAI-shaped model list
non-streaming OpenAI-shaped chat completion
safe-read repository tool registry and execution
strict policy snapshot and reports
workspace sandbox report
snapshot list and diff
GGUF discovery and rescan
managed llama-server lifecycle and diagnostics
memory status
run summaries, details, trace, context, tool audit, file touches, policy explain
checkpoint inspection and decision recording
request, trace, diagnostic, session, thread, and run identities
```

### 12.2 Types or partial mechanics that are not a complete Forge seam by themselves

```text
Phase-1 protocol DTOs without a dedicated create/update route
runtime instance spec read from environment without auto-start behavior
checkpoint decision recording without exact paused-tool resume
in-process detailed run caches without proven durable rehydration
server health without a canonical schema-stamped capability list
permissive CORS without an authentication contract
```

### 12.3 Not currently implemented in the inspected public seam

```text
Forge FGNYXQ/FGNYXR binary handshake server
Nyx Unix-domain-socket public HTTP or binary server
canonical health response with schema epoch and full capability discovery
streaming chat
public session create/list/switch endpoints
public thread create/list/switch endpoints
checkpoint approval that resumes the exact paused tool call
general write/process/network tool catalog for Forge workflows
remote OpenAI heavyweight-agent workflow required by later Forge skills
Nyx-generated patch proposal endpoint and durable patch artifact contract
Nyx service authentication suitable for non-loopback exposure
```

A model must not fill one of these gaps in ForgeOS. Missing Nyx behavior routes to
the Nyx repository and exact Nyx skills in the dependency contract.

---

## 13. ForgeOS client architecture obligations

The ForgeOS Nyx boundary should remain layered:

```text
forge-nyx-client
  HTTP transport and bounded JSON decode
  version/schema/capability compatibility
  typed route-family errors
  no model-provider dependencies
  no server handlers
  no canonical Nyx state stores

forge-session or service layer
  nyx_server process configuration and lifecycle
  project/worktree-to-workspace binding
  readiness polling
  restart/rebind behavior

Forge Core integration
  links Nyx operation IDs to Forge project/revision/scope
  records Forge-side user intent and independent evidence
  never rewrites Nyx records

Forge World
  displays server, runtime, model, policy, run, checkpoint, and tool state
  issues explicit user commands
  never fabricates green state
```

Do not let `forge-nyx-client` grow:

```text
an Axum server
an Ollama client
an OpenAI provider client
an agent loop
a tool registry
a policy engine
a session database
a checkpoint state machine
a memory ledger
a patch generator
```

Those are Nyx responsibilities.

---

## 14. Required integration order

Use the smallest real seam in this order:

### Stage A: canonical health and capability discovery

```text
Nyx implements and banks the required API skills
  -> Forge client consumes real HTTP/JSON
  -> real-process matrix proves unavailable, malformed, incompatible,
     unhealthy/degraded, and ready states
  -> FORGEOS-V1-NYX-100 may close
```

### Stage B: permissions and exact approval semantics

```text
consume tools and policy catalogs
  -> surface checkpoint request
  -> resolve through Nyx
  -> do not claim exact resume until Nyx implements it
```

### Stage C: managed Nyx service

```text
Forge starts/stops nyx_server
  -> Nyx independently manages its model runtime
  -> Forge verifies both process layers and preserves their distinct states
```

### Stage D: models and conversation

```text
list backend models
  -> select model by Nyx ID
  -> send non-streaming chat
  -> store Nyx run identity
  -> inspect context, tools, policy, and run evidence
```

### Stage E: project-aware work

```text
bind exact Forge worktree as NYX_WORKSPACE_ROOT
  -> verify sandbox report
  -> allow Nyx-owned tools under Nyx policy
  -> independently verify file and Git claims in ForgeOS
```

### Stage F: patch and agent work

Do not activate until the mapped Nyx skills are banked. Nyx may produce an inert
proposal and Nyx-side audit. ForgeOS owns isolated worktrees, exact diff review,
base/hash validation, local apply, build/test verification, and commit.

---

## 15. Real-process acceptance matrix

A ForgeOS Nyx integration test suite should include real Nyx process witnesses,
not only fixtures.

| Case | Required observation |
|---|---|
| no listener | `UNAVAILABLE`; no panic; no provider fallback |
| listener returns non-JSON | `MALFORMED` or `INCOMPATIBLE`; never ready |
| HTTP health without required contract fields | incompatible for capability handshake, even if basic liveness is visible |
| supported contract and healthy server | ready with exact service version and canonical capabilities |
| unsupported contract epoch | incompatible with visible received version |
| Nyx alive, backend down | server alive; model list/chat error retained as backend failure |
| runtime stopped | Nyx alive; runtime status shows no ready runtime |
| runtime degraded | Nyx alive; runtime health preserved as degraded |
| workspace mismatch | block project-aware Nyx work; show expected and reported roots |
| sandbox denial | preserve 403 and Nyx error; do not retry outside Nyx |
| checkpoint required | preserve 409, run ID, checkpoint ID, and approval state |
| checkpoint approved in current N1 | show `execution_resumed=false`; do not execute locally |
| Nyx restart | new boot timestamp; re-evaluate readiness and cache lifetime |
| malformed run detail | keep chat response but mark evidence unavailable/incompatible |
| trace headers | preserve `x-nyx-trace-id` and `x-nyx-diagnostic-id` |
| server error contains internal secret | Forge display uses redacted public contract only |

---

## 16. Cross-repository routing decision

Before fixing any Nyx-facing failure:

```text
Is the failure in Forge HTTP transport, decode, UI, process supervision,
project envelope, worktree handling, or independent validation?
  YES -> patch Forge_OS_V1 only

Is the failure in a Nyx route, DTO authority, health/capability truth, model
backend, session/thread state, context, tool execution, policy, checkpoint,
runtime supervisor, memory, agent loop, run record, or Nyx evidence?
  YES -> patch Nyx_Server only

Is the contract undefined or contradictory across both repos?
  -> choose Nyx as protocol owner for Nyx behavior
  -> patch Nyx first
  -> return a versioned handoff
  -> patch Forge adapter second
```

Required handoff packet:

```text
FORGE_SKILL_ID
NYX_GATE_ID
MISSING_NYX_SKILL_IDS
CURRENT_NYX_LEDGER_STATE
NYX_PUBLIC_SURFACE
FIRST_BLOCKER
TARGET_REPOSITORY
ALLOWED_PATHS
EXPECTED_REQUEST
EXPECTED_RESPONSE
ERROR_CONTRACT
RETURN_WITNESS
```

---

## 17. Source crosswalk for mandatory live verification

When a Nyx-facing Forge slice starts, inspect the current equivalent of these Nyx
files. Paths below are from the inspected Nyx snapshot.

### Server and route registry

```text
crates/nyx_server/src/main.rs
crates/nyx_server/src/http/mod.rs
crates/nyx_server/src/http/openai_chat.rs
crates/nyx_server/src/http/openai_models.rs
crates/nyx_server/src/http/openai_error.rs
crates/nyx_server/src/http/nyx_diag.rs
crates/nyx_server/src/runtime_probe.rs
```

### Public protocol

```text
crates/nyx_protocol/src/openai.rs
crates/nyx_protocol/src/agent.rs
crates/nyx_protocol/src/checkpoint.rs
crates/nyx_protocol/src/context.rs
crates/nyx_protocol/src/errors.rs
crates/nyx_protocol/src/ids.rs
crates/nyx_protocol/src/run_record.rs
crates/nyx_protocol/src/runtime.rs
crates/nyx_protocol/src/session.rs
crates/nyx_protocol/src/temporal.rs
crates/nyx_protocol/src/tools.rs
crates/nyx_protocol/src/phase1.rs
```

### Runtime, policy, state, context, and evidence

```text
crates/nyx_core/src/runtime/*
crates/nyx_core/src/policy/*
crates/nyx_core/src/state/*
crates/nyx_core/src/context/*
crates/nyx_core/src/agent/*
crates/nyx_core/src/checkpoints.rs
crates/nyx_core/src/observability/*
```

### Backends, repository tools, and persistence

```text
crates/nyx_backends/src/*
crates/nyx_tools/src/*
crates/nyx_repo/src/*
crates/nyx_memory/src/*
```

### Nyx workflow status

```text
docs/workflow/skill_trees/NYX-SKILL-ROUTER.md
docs/workflow/skill_trees/NYX-SKILL-ROUTER_STATE.json
docs/workflow/skill_trees/NYX-SKILL-SEALED-LEDGER.jsonl
docs/workflow/skill_trees/NYX-SKILL-GRAPH.json
```

Do not rely on this source crosswalk alone. Search the current Nyx repository for
new routes, moved DTOs, and changed configuration before implementation.

---

## 18. Never-do list

```text
Do not add a Nyx server handler to ForgeOS.
Do not call Ollama, llama-server, or OpenAI around Nyx.
Do not copy Nyx's session, policy, tool, checkpoint, memory, or agent engines.
Do not treat Forge fixtures as a real Nyx witness.
Do not treat HTTP 200 as protocol compatibility.
Do not treat server liveness as backend or runtime readiness.
Do not treat runtime endpoints as nyx_server process control.
Do not invent authentication, streaming, session switching, or approval resume.
Do not use a Forge-generated run ID as Nyx identity.
Do not auto-apply a Nyx or remote-agent patch.
Do not convert Nyx prose into Git, build, test, or release proof.
Do not hide a workspace-root mismatch.
Do not close a Forge Nyx skill before its NYX-GATE passes.
```

---

## 19. Definition of a clean Forge-to-Nyx slice

A clean slice ends with all applicable lines true:

```text
[ ] wiring cheat sheet reviewed and preflight reported
[ ] current Nyx source files inspected
[ ] owning repository chosen correctly
[ ] exact Nyx skill gate checked
[ ] Forge paths remain client/adapter/UI/process/evidence only
[ ] no provider bypass or duplicate Nyx engine added
[ ] route, schema, headers, and error family are typed
[ ] unavailable and malformed paths are preserved
[ ] workspace and process identities remain explicit
[ ] real Nyx process witness exists when closure is claimed
[ ] Forge independently verifies project, Git, patch, build, and test claims
[ ] this cheat sheet is updated if the live seam changed
```

The rule underneath every box is simple:

> ForgeOS integrates Nyx. It does not cosplay as Nyx.
