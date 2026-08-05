# ForgeOS Nyx Server capability and ownership cheat sheet

Status: `MANDATORY_NYX_OWNERSHIP_PREFLIGHT`
Document ID: `FORGEOS-NYX-CAPABILITY-OWNERSHIP-CHEAT-SHEET-V1`
Applies to: every ForgeOS slice that proposes, consumes, displays, supervises,
records, tests, or documents behavior that belongs to Nyx Server
Wiring companion:
`docs/versions/V1/FORGEOS_NYX_SERVER_WIRING_CHEAT_SHEET.md`
Cross-repository closure authority:
`docs/versions/V1/FORGEOS_V1_NYX_SERVER_DEPENDENCY_CONTRACT.md`
Machine gate map:
`docs/versions/V1/FORGEOS_V1_NYX_SERVER_DEPENDENCIES.json`
Nyx source snapshot inspected for this revision: `Nyx_Server_base_5`

---

## 0. Mandatory use

This file must be read before any ForgeOS work that touches AI, models, chat,
context, repository intelligence, tools, policy, approvals, checkpoints, agent
runs, workflows, memory, routing, temporal state, persistence, observability,
plugins, proposals, or Nyx-facing UI.

The wiring sheet answers:

```text
How does ForgeOS connect to Nyx?
```

This ownership sheet answers:

```text
Which system owns the behavior, now and at full Nyx maturity?
```

A patching model must report both reviews before editing:

```text
NYX_WIRING_REVIEWED=docs/versions/V1/FORGEOS_NYX_SERVER_WIRING_CHEAT_SHEET.md
NYX_OWNERSHIP_REVIEWED=docs/versions/V1/FORGEOS_NYX_SERVER_CAPABILITY_OWNERSHIP_CHEAT_SHEET.md
NYX_CROSS_REPO_CONTRACT_REVIEWED=docs/versions/V1/FORGEOS_V1_NYX_SERVER_DEPENDENCY_CONTRACT.md
NYX_REQUESTED_BEHAVIOR=<plain-language behavior being added or changed>
NYX_CANONICAL_OWNER=FORGE_CORE|FORGE_ADAPTER|NYX_SERVER|REAL_TOOL|LINUX_HOST
NYX_ENGINE_FAMILY=<API|AGENT|CONTEXT|MEMORY|OBSERVABILITY|PERSISTENCE|POLICY|REPO|ROUTING|TEMPORAL|TOOLS|WORKFLOW|NONE>
NYX_CURRENT_STATE=REAL|PARTIAL|CONTRACT_ONLY|PLANNED|MISSING
NYX_FORGE_ROLE=<client, process supervisor, UI, envelope provider, verifier, or none>
NYX_SOURCE_FILES_CHECKED=<exact current Nyx paths>
NYX_FORGE_ALLOWED_PATHS=<exact ForgeOS paths>
NYX_DUPLICATE_IMPLEMENTATION_CHECK=PASS
```

Missing receipt means stop before source edits.

This file is a copied integration authority inside ForgeOS. It does not override
current Nyx source, Nyx skill status, Nyx proof receipts, or the Nyx router.
When this file and the current Nyx repository disagree:

```text
current Nyx source and real Nyx outputs win
  -> identify the owning Nyx engine
  -> patch Nyx Server when Nyx behavior is absent or wrong
  -> patch ForgeOS only when the adapter, supervision, envelope, UI, or verifier is wrong
  -> update this copied cheat sheet after the owning contract changes
```

---

## 1. Prime directive

`nyx_server` is a separate product repository and process. It is not a ForgeOS
crate waiting to happen.

```text
ForgeOS owns the development environment and project truth.
Nyx owns AI behavior and AI-side state.
Real tools own their native results.
ForgeOS integrates, constrains, displays, and verifies.
```

When ForgeOS needs a capability that Nyx is intended to provide but Nyx has not
implemented yet, the legal result is:

```text
Forge adapter may be prepared against an explicit versioned fixture
  -> Forge skill remains ACTIVE, BLOCKED, or SOURCE_PROVED
  -> Nyx repository receives the missing capability patch
  -> real cross-process witness is required before Forge closure
```

The illegal result is:

```text
missing Nyx capability
  -> add a convenient local copy inside forge-nyx-client, forge-session,
     forge-world, forge-app, forge-core, a script, or a fixture server
  -> call the Forge skill complete
```

That is not integration. It is a second Nyx wearing ForgeOS armor.

---

## 2. The full system split

### 2.1 Forge Core owns

```text
project identity
repository registration
workspace identity and canonical Forge workspace state
release targets
Forge capability and skill state
missions, experiments, blockers, slices, and proof receipts
source revision and worktree identity supplied to Nyx
allowed project scope supplied to Nyx
registered Forge command declarations
decisions to accept, reject, or apply returned proposals
independent verification of files, Git, builds, tests, patches, and commits
```

Nyx may read or operate against a granted projection of this truth. Nyx may not
invent it, silently widen it, or become its canonical store.

### 2.2 Forge adapters and UI own

```text
Nyx client transport
Nyx service-process configuration and supervision
bounded request timeouts and response limits
mapping Forge project envelopes into Nyx request contracts
operator controls for selecting, approving, cancelling, and inspecting
truthful display of Nyx-owned states and evidence
independent Forge-side verification and reconciliation
```

Adapters may cache for transport convenience only when the cache is explicitly
non-authoritative and invalidation is defined. They may not grow domain logic.

### 2.3 Nyx Server owns

```text
public AI API and compatibility contracts
model backend and runtime catalogs
model selection and calls
AI sessions, threads, and accepted operations
context construction
AI memory
repository observation used by AI
Nyx tool registry and execution
policy decisions
approval and checkpoint law
bounded agent execution
workflow execution
routing and fallback
Nyx temporal history and replay
Nyx durable state and migrations
run records, traces, diagnostics, incidents, and exports
plugins and proposal-only self-forging
```

### 2.4 Real tools own

```text
filesystem effects
Git semantics and repository mutation
compiler, formatter, test runner, debugger, and package-manager results
process exit status and native diagnostics
external model runtime behavior beneath the Nyx adapter
```

Nyx may invoke real tools through governed tool adapters. ForgeOS must verify
important effects independently before promoting Forge project truth.

### 2.5 Linux host owns

```text
kernel, drivers, filesystems, networking, process isolation, signals,
resource controls, login/session management, and host services
```

ForgeOS and Nyx both consume host facilities. Neither may counterfeit them with
application state.

---

## 3. State language used by this sheet

```text
REAL
  Product source exists in the inspected Nyx base and the behavior has a real
  path. This does not automatically mean the corresponding Nyx skill is BANKED.

PARTIAL
  Useful source exists, but durability, public routing, restart behavior,
  breadth, or proof required by the full claim is incomplete.

CONTRACT_ONLY
  DTOs, interfaces, placeholders, or documentation exist without a complete
  source-owned product path.

PLANNED
  The capability belongs to the Nyx skill tree and future product ceiling but
  is not claimed as implemented in the inspected base.

MISSING
  The expected seam does not exist in the inspected base and no Forge model may
  create a substitute inside ForgeOS.
```

Skill completion is never inferred from these labels. Resolve live completion
from the Nyx append-only sealed ledger, router state, source, experiments, and
proof receipts.

---

## 4. Current Nyx baseline relevant to ForgeOS

The inspected Nyx base currently provides a meaningful but incomplete server.
The wiring sheet contains the exact route and environment inventory. The
ownership-level baseline is:

### 4.1 Real or substantially real now

```text
Axum HTTP server on configured local TCP
OpenAI-shaped model listing
non-streaming OpenAI-shaped chat completion
Ollama and OpenAI-compatible backend adapters
optional managed llama-server runtime and GGUF discovery
server, backend, runtime, tool, policy, sandbox, memory, run, and snapshot diagnostics
session and thread identity substrate
in-process session and thread stores
ContextBundle construction and selection reports
sandboxed repository inventory, reads, search, and snippets
safe-read tool registry and execution
strict policy snapshots and tool decisions
durable permission checkpoints, explicit approval/denial, immutable resume, replay protection, and audit
bounded agent and tool-loop substrate
append-only memory ledger, rotation, reopen, and derived index rebuild
snapshot manifests, list, diff, and content reads
RunRecord contracts, traces, file touches, tool audits, and redacted diagnostics
```

### 4.2 Partial or process-local now

```text
durable sessions and threads
durable run, context, and response rehydration beyond the permission journal
complete backend and route decision engine
full model-runtime fleet management
full write, process, and network tool families
streaming chat and streaming tool calls
complete workflow engine
complete temporal replay, restore, branch, and rollback
complete metrics and incident system
portable flight-recorder export and hostile external verification
```

### 4.3 Missing public seam that ForgeOS must not fabricate

```text
public session and thread lifecycle API sufficient for Forge product use
agent-run suspended-action continuation beyond the general permission API
complete remote heavyweight-agent workflow
Nyx-owned durable patch proposal and returned-artifact contract
production authentication for non-loopback exposure
```

---

## 5. Nyx API Engine

Skill family: `API-*`
Strategic ceiling anchors: `API-APEX-001` through `API-APEX-008`

### 5.1 Nyx owns

```text
HTTP routing
request parsing and validation
authentication and transport limits
OpenAI compatibility normalization
response shaping
SSE framing and cancellation propagation
version negotiation
capability discovery
correlation headers
public error mapping
```

### 5.2 Current inspected baseline

```text
REAL:
  local HTTP server
  /v1 health and diagnostic families
  OpenAI-shaped /v1/models
  non-streaming /v1/chat/completions
  Nyx-native diagnostics and control routes described in the wiring sheet

PARTIAL OR MISSING:
  canonical Forge-compatible health and capability schema
  streaming chat and tool calls
  embeddings
  complete public session, thread, workflow, proposal, replay, and export plane
  non-loopback authentication contract
```

### 5.3 Nyx will own at full maturity

```text
complete OpenAI client compatibility
complete Nyx-native headless control plane
secure bounded public request surface
durable identity across retry, concurrency, stream, and restart
governed actions, workflows, plugins, and proposal application
transparent semantic and multi-agent inspection
live and offline flight-recorder auditability
honest capability discovery with no fake support
```

### 5.4 ForgeOS may

```text
configure the Nyx base URL
send versioned requests
bound transport resources
map typed errors into Forge state
display Nyx-declared capabilities
exercise external-client acceptance
```

### 5.5 ForgeOS may not

```text
add Nyx server routes inside ForgeOS
own canonical Nyx DTO evolution
shape model responses with a second compatibility layer that changes meaning
implement sessions, tools, policy, workflows, or run records in HTTP client code
claim a fixture server is Nyx
```

---

## 6. Nyx Agent Engine

Skill family: `AGENT-*`
Strategic ceiling anchors: `AGENT-APEX-001` through `AGENT-APEX-008`

### 6.1 Nyx owns

```text
bounded run progression
run state transitions and terminal posture
model-turn sequencing
typed engine-seam orchestration
shared run budgets
retry and repair decisions
checkpoint waiting and lawful continuation
cancellation propagation
persona and role coordination
bounded evaluator and multi-agent loops
evidence-backed proposal generation
```

### 6.2 Current inspected baseline

```text
REAL OR PARTIAL:
  single-pass agent substrate
  tool-call parsing and bounded tool loop
  policy decision and checkpoint integration
  run records, context, tool audit, and file-touch evidence
  cancellation, budget, and error contracts in protocol/source

PLANNED OR INCOMPLETE:
  complete agent-run restart-safe continuation beyond the permission API
  complete multi-step repository agent product path
  remote heavyweight-agent orchestration
  personas and bounded multi-agent evaluators
  proposal-only self-forging with validation and rollback
```

### 6.3 Nyx will own at full maturity

```text
one bounded source-owned execution path
durable pause, resume, cancel, retry, stream, and restart
integrated models, tools, workflows, memory, semantics, and replay
bounded personas and multi-agent evaluators
proposal-only self-forging
evidence-backed adaptive strategies
complete inspectable and replayable agent evidence
```

### 6.4 ForgeOS may

```text
submit a bounded task envelope
provide project, revision, worktree, scope, command, and budget constraints
display run state and evidence
request cancellation or approval
independently inspect the returned worktree, patch, and validation results
```

### 6.5 ForgeOS may not

```text
run a second hidden model loop in forge-app or forge-world
invent agent progress messages
rewrite or regenerate a paused action after approval
apply returned changes without explicit Forge-owned review and verification
store canonical Nyx run state in Forge Core
```

---

## 7. Nyx Context Engine

Skill family: `CONTEXT-*`
Strategic ceiling anchors: `CONTEXT-APEX-001` through `CONTEXT-APEX-008`

### 7.1 Nyx owns

```text
normalization of model-visible candidates
source prioritization and final ordering
total and per-source budgets
pinned and required reserves
compression and truncation decisions
deterministic and semantic scoring integration
inclusion, exclusion, and fallback rationale
exact ContextBundle construction
dry-run context previews
context refresh and long-run compaction policy
context-strategy evaluation and proposal evidence
```

### 7.2 Current inspected baseline

```text
REAL OR PARTIAL:
  ContextBundle contracts
  context builder
  selection reports
  repository, message, tool, and policy-related source inputs
  per-run context diagnostic views

PLANNED OR INCOMPLETE:
  full semantic retrieval integration
  durable context rehydration
  long-run compaction and refresh
  persona and multi-agent least-context isolation
  strategy comparison, proposal, rollback, and export
```

### 7.3 Nyx will own at full maturity

```text
exact immutable context for every model call
one provenance and budget law across all sources
explainable semantic and adaptive selection with deterministic fallback
refresh, compaction, restart, branching, and replay
least-context persona and multi-agent isolation
evidence-backed context strategy proposals
dry-run inspection, comparison, and export
```

### 7.4 ForgeOS may

```text
supply canonical project envelopes and operator-selected pins
request a context preview
show included and excluded sources
show token and source budgets
verify cited Forge files against current project truth
```

### 7.5 ForgeOS may not

```text
silently inject source text outside the Nyx ContextBundle
assemble a second hidden prompt in Forge UI code
claim Nyx saw a file that was not in the recorded context
store model wording as project truth
```

---

## 8. Nyx Memory Engine

Skill family: `MEMORY-*`
Strategic ceiling anchors: `MEMORY-APEX-001` through `MEMORY-APEX-008`

### 8.1 Nyx owns

```text
append-only memory event truth
deterministic materialization and indexes
scoped memory records and bundles
deterministic and semantic retrieval
retrieval explanations and score breakdowns
memory provenance, staleness, feedback, and graph projections
compaction and curation analysis
memory mutation proposal change sets
memory diagnostics and export contracts
```

### 8.2 Current inspected baseline

```text
REAL:
  append-only JSONL memory ledger
  stable event identity
  reopen and rotation
  optional fsync
  partial trailing-line recovery
  derived index rebuild
  memory status diagnostics

PARTIAL OR PLANNED:
  richer scoped retrieval
  semantic embeddings and score explanation
  feedback, curation, graph projections, and staleness policy
  proposal-only memory mutation
  portable exports and historical verification
```

### 8.3 Nyx will own at full maturity

```text
durable scoped memory across time and source change
fully explainable deterministic and semantic retrieval
proposal-only reversible source-preserving curation
bounded visible feedback rules
least-scope privacy across consumers
bounded storage, indexes, graphs, and retrieval at scale
externally verifiable historical memory use
```

### 8.4 ForgeOS may

```text
choose whether a Forge task permits Nyx memory use
show memory scope and provenance
request deletion or mutation through Nyx-governed policy and proposal paths
verify that project truth still comes from Forge source rather than memory
```

### 8.5 ForgeOS may not

```text
create a competing AI memory database
write directly into the Nyx ledger
promote recalled model text into canonical Forge state
hide memory use from the operator
```

---

## 9. Nyx Repository Engine

Skill family: `REPO-*`
Strategic ceiling anchors: `REPO-APEX-001` through `REPO-APEX-008`

### 9.1 Nyx owns

```text
Nyx workspace identity and sandboxed repository observation
normalized paths, inventory, metadata, hashes, and source spans
read, path, text, snippet, symbol, graph, semantic, and history queries
deterministic and incremental derived indexes
language-adapter and analyzer-version custody
read-only Git metadata used by Nyx
repository provenance, staleness, limits, and diagnostics
impact, architecture, test, workset, and proposal evidence reports
```

### 9.2 Current inspected baseline

```text
REAL:
  configured workspace root
  sandbox path enforcement
  repository inventory
  bounded file reads
  text search
  snippets
  safe-read tool integration

PARTIAL OR PLANNED:
  durable incremental index
  symbol, dependency, call, and target graphs
  semantic retrieval
  history and semantic change analysis
  impact, test, workset, and architecture guidance
```

### 9.3 Nyx will own at full maturity

```text
strict complete bounded workspace observation
explainable deterministic and semantic repository queries
source-bound symbol, dependency, call, and target graph
history, semantic change, impact, tests, and uncertainty
restart-safe reproducible indexes at scale
context, test, workset, architecture, and proposal guidance
full replayable repository provenance
```

### 9.4 ForgeOS may

```text
provide the canonical registered repository and worktree root
provide revision and allowed-scope envelopes
show Nyx citations and search evidence
compare Nyx-observed hashes against Forge and Git truth
```

### 9.5 ForgeOS may not

```text
let Nyx silently choose a different repository
implement a parallel AI repository index in forge-nyx-client
allow repository reads outside declared scope
confuse Nyx read observations with Forge canonical project state
```

---

## 10. Nyx Tool Engine

Skill family: `TOOLS-*`
Strategic ceiling anchors: `TOOL-APEX-001` through `TOOL-APEX-008`

### 10.1 Nyx owns

```text
canonical tool descriptors, schemas, versions, safety, effects, and limits
tool registry construction, validation, snapshots, projection, diff, and reload
argument validation and bounded dispatch
result validation, output limits, effect reconciliation, and tool-local audit
safe-read and safe-compute implementations
structured write, patch, and allowlisted process tool mechanics
local plugin descriptors, packages, validation, install plans, and lifecycle
tool and plugin proposal payloads and catalog-change previews
```

### 10.2 Current inspected baseline

```text
REAL:
  tool protocol contracts
  registry
  safe repository read tools
  bounded executor
  policy attachment
  redacted audit records
  file-touch evidence

PARTIAL OR PLANNED:
  complete write and patch tools
  allowlisted process tools
  bounded network tools
  plugin lifecycle
  tool catalog migration, replay, rollback, and proposal application
```

### 10.3 Nyx will own at full maturity

```text
one canonical versioned registry and catalog
one bounded validated execution path
least-power read, compute, mutation, process, and network law
local plugins and workflows with complete lifecycle
proposal-only tool and plugin self-forging
restart, replay, rollback, and clean reproduction
complete externally verifiable provenance
```

### 10.4 ForgeOS may

```text
register Forge-owned command declarations through a Nyx contract
supply allowed working directory and project scope
show requested arguments, safety class, effects, and result
independently verify file, process, Git, build, and test effects
```

### 10.5 ForgeOS may not

```text
execute a model-requested command directly from UI code
create a second hidden tool registry
mark a tool allowed without Nyx Policy evidence
trust model prose instead of the real tool result
allow a fixture tool to satisfy product closure
```

---

## 11. Nyx Policy Engine

Skill family: `POLICY-*`
Strategic ceiling anchors: `POLICY-APEX-001` through `POLICY-APEX-008`

### 11.1 Nyx owns

```text
effective policy snapshots and rule evaluation
allow, deny, checkpoint, disclosure, and policy-proof decisions
layer precedence and narrow-only override law
actor, role, persona, resource, and capability scopes
tool, repo, memory, network, process, routing, temporal, workflow, plugin, and proposal gates
checkpoint and approval validity rules
quotas, retention, redaction, debug disclosure, and export posture
policy simulation, diff, migration, replay, diagnostics, and incidents
```

### 11.2 Current inspected baseline

```text
REAL:
  strict policy configuration
  deterministic policy snapshots
  tool decisions
  checkpoint posture
  policy reports and per-run explanations
  redaction rules

PARTIAL OR PLANNED:
  complete actor and persona capability matrix
  durable approvals and exact resume binding
  every future tool, workflow, plugin, proposal, routing, temporal, and export gate
  policy migrations, incidents, replay, and hostile proof
```

### 11.3 Nyx will own at full maturity

```text
one deterministic effective policy per accepted operation
one fail-closed gate for every capability and side effect
exact preview, approval binding, re-evaluation, and no duplicate apply
least privilege, quotas, privacy, and redaction across all actors
reversible replayable policy and proposal history
adaptive intelligence without silent authority expansion
complete externally verifiable policy proof
```

### 11.4 ForgeOS may

```text
present policy prompts and decisions
capture the human operator decision through the Nyx contract
provide Forge-side scope constraints that Nyx may only narrow
refuse to continue even when Nyx allows an action
```

### 11.5 ForgeOS may not

```text
replace Nyx Policy with a checkbox in Forge UI
broaden a denied or checkpointed action
mutate a pending action after approval
store an approval without its Nyx identity, digest, scope, and expiry
claim an action was approved from button state alone
```

---

## 12. Nyx Routing Engine

Skill family: `ROUTING-*`
Strategic ceiling anchors: `ROUTING-APEX-001` through `ROUTING-APEX-008`

### 12.1 Nyx owns

```text
backend, model, embedder, and runtime-profile catalogs
capability normalization and eligibility
backend health and readiness facts used for routing
task-class and lane selection
deterministic scoring and tie-breaks
fallback plans, attempt budgets, and circuit posture
local-first and explicit-remote route strategy
route decisions, rejections, diagnostics, replay, and strategy proposals
```

### 12.2 Current inspected baseline

```text
REAL OR PARTIAL:
  backend target contracts
  Ollama adapter
  OpenAI-compatible adapter
  configured primary backend
  model listing
  runtime health and GGUF registry substrate

PLANNED OR INCOMPLETE:
  complete canonical catalog and capability normalization
  deterministic multi-candidate scoring
  explicit lanes for chat, tools, JSON, stream, embeddings, context, and roles
  full retry, fallback, circuit, affinity, admission, and load behavior
  strategy proposals and replay
```

### 12.3 Nyx will own at full maturity

```text
canonical backend, model, embedder, profile, capability, and health catalog
deterministic explainable policy-constrained selection
specialized chat, tool, JSON, stream, embed, context, and role lanes
governed local runtimes and explicit remote backends
bounded retries, fallback, circuit breaking, affinity, admission, and load
proposal-only adaptive routing with validation and rollback
full replayable route and runtime provenance
```

### 12.4 ForgeOS may

```text
show available Nyx-declared models and profiles
request a model or lane when policy allows
show the selected route and fallback posture
configure approved local endpoints and explicit remote credentials through Nyx
```

### 12.5 ForgeOS may not

```text
call Ollama, llama-server, or OpenAI around Nyx
select a hidden fallback provider
store a second model registry
reinterpret backend health as Nyx server health
silently turn a local request into a remote request
```

---

## 13. Nyx Temporal Engine

Skill family: `TEMPORAL-*`
Strategic ceiling anchors: `TEMPORAL-APEX-001` through `TEMPORAL-APEX-008`

### 13.1 Nyx owns

```text
snapshot, diff, history, timeline, replay, branch, restore-plan, and rollback-anchor truth
immutable ancestry and temporal operation identities
structural change ground truth and semantic evolution overlays
replay reconstruction, cursor, comparison, drift, and side-effect posture
restore and rollback previews, plans, verification, and history
temporal migrations, retention references, integrity, diagnostics, and archives
```

### 13.2 Current inspected baseline

```text
REAL OR PARTIAL:
  snapshot protocol contracts
  snapshot manifests and content reads
  snapshot list and diff diagnostics
  temporal engine seam

PLANNED OR INCOMPLETE:
  full timeline
  deterministic offline replay
  restore and rollback plan execution
  isolated branches
  semantic evolution
  failure bisection
  complete archival and external reproduction
```

### 13.3 Nyx will own at full maturity

```text
one immutable source-bound temporal spine
deterministic snapshots, diffs, timelines, and evolution
safe previewed verified restore and rollback
dry-run, step, seek, resume, compare, and offline replay
isolated branches and deterministic failure bisection
semantic evolution without replacing source truth
every dangerous change anchored, validated, and exactly revertible
```

### 13.4 ForgeOS may

```text
request and display Nyx temporal reports
provide current Forge repository and worktree identity
compare Nyx temporal claims against Git and filesystem truth
require Forge-owned approval before applying a restore or rollback effect
```

### 13.5 ForgeOS may not

```text
implement Nyx replay as UI event playback
call a copied snapshot DTO a temporal engine
let Nyx temporal state replace Git or Forge canonical project history
perform restore effects without real tools, policy, checkpoints, and verification
```

---

## 14. Nyx Persistence Engine

Skill family: `PERSIST-*`
Strategic ceiling anchors: `PERSIST-APEX-001` through `PERSIST-APEX-008`

### 14.1 Nyx owns

```text
durable object envelopes, keys, manifests, checksums, and references
storage roots, namespaces, backends, atomic commits, transactions, locks, and leases
durable sessions, threads, accepted operations, idempotency, runs, context, and responses
durable checkpoints, proposals, approvals, workflows, plugins, policy, routing, indexes, and temporal artifacts
schema registry, migrations, quarantine, repair, retention, archive, backup, import, and export
storage health, pressure, integrity, inventory, portability, and scale reports
```

### 14.2 Current inspected baseline

```text
REAL ON DISK:
  append-only memory ledger
  deterministic memory rotation and reopen
  optional fsync and partial trailing-line recovery
  derived memory index rebuild
  temporal snapshot manifests and content reads
  permission checkpoint, idempotency, token-consumption, and audit journal

IN MEMORY ONLY:
  sessions
  threads
  general run records
  context bundles

CONTRACT ONLY OR PLANNED:
  general Persistence Engine
  complete migrations, repair, retention, backup, restore, and portability
```

### 14.3 Nyx will own at full maturity

```text
one complete source-owned durable state spine
exactly-once crash-safe operations and concurrency
transactional migration, quarantine, repair, and recovery
safe retention, pressure, privacy, and isolation
reopen, rebuild, replay, and reproduction from stored truth
complete inventory and forensic visibility
clean-environment portability and isolated restore
```

### 14.4 ForgeOS may

```text
configure a Nyx data root through the supported process contract
show Nyx storage health and migration requirements
back up or package Nyx data only through declared Nyx export and shutdown law
keep Forge project truth in Forge-owned stores
```

### 14.5 ForgeOS may not

```text
create Forge-owned canonical copies of Nyx sessions or runs
write Nyx JSON or database rows directly
perform ad hoc Nyx schema migration from Forge startup code
claim in-memory Nyx state is restart durable
```

---

## 15. Nyx Observability Engine

Skill family: `OBS-*`
Strategic ceiling anchors: `OBS-APEX-001` through `OBS-APEX-008`

### 15.1 Nyx owns

```text
events and correlation
traces and reports
metrics and health
incidents
proof references
redaction receipts
diagnostic queries and live tails
exports and verification
```

### 15.2 Current inspected baseline

```text
REAL:
  RunRecordV1 and stable vector ordering
  request, trace, diagnostic, run, session, thread, tool, checkpoint, and context identities
  request lifecycle diagnostics and trace headers
  redaction-safe debug mirror
  server and engine diagnostic reports
  per-run trace, policy, tool, file-touch, context, selection, and checkpoint views
  accepted-failure records and sanitized errors

PARTIAL OR PROCESS-LOCAL:
  run, context, and selection lookup
  lifecycle ring buffers
  recent-run derivation

PLANNED:
  durable flight recorder
  full metrics and incidents
  replay comparisons
  proposal and self-forging proof reports
  portable exports and hostile external verification
```

### 15.3 Nyx will own at full maturity

```text
one durable correlated flight-recorder spine
complete source-backed explanation of every decision
effect, policy, approval, and rollback reconciliation
bounded redacted operations, health, metrics, and incidents
replay, migration, archive, and external reproduction
explainable adaptive and self-forging evidence
hostile external verification of supported claims
```

### 15.4 ForgeOS may

```text
display Nyx run records and diagnostics
correlate Forge request IDs with Nyx IDs
retain Forge-side references to Nyx evidence
verify Nyx file-touch and tool-effect claims against real Forge tools
show missing evidence as missing
```

### 15.5 ForgeOS may not

```text
fabricate a run record from UI events
upgrade logs into canonical Nyx evidence
hide failed attempts or missing fields
let a green dashboard replace source-backed proof
rewrite Nyx evidence to fit a Forge closure claim
```

---

## 16. Nyx Workflow Engine

Skill family: `WORKFLOW-*`
Strategic ceiling anchors: `WORKFLOW-APEX-001` through `WORKFLOW-APEX-008`

### 16.1 Nyx owns

```text
immutable workflow descriptors and versions
graph validation, planning, readiness, mappings, and workflow run state
manual, scheduled, event, nested, and plugin triggers
per-step orchestration and workflow-level limits
workflow traces, outputs, artifacts, diagnostics, and replay explanations
workflow packs, migrations, proposals, apply plans, and revert plans
```

### 16.2 Current inspected baseline

```text
REAL SUBSTRATE:
  official engine identity and authority boundary
  contract-only engine interface
  dependency and forbidden-edge laws
  placeholder diagnostics
  tool, checkpoint, policy, and RunRecord substrate

NOT YET REAL AS A WORKFLOW PRODUCT:
  WorkflowDescriptor
  registry
  graph validator and planner
  parameter mapping
  runner
  schedules and event triggers
  durable workflow state
  workflow replay and branching
  workflow plugin packs
  workflow proposal, apply, and revert
```

### 16.3 Nyx will own at full maturity

```text
canonical immutable multi-scope workflow registry
one bounded durable workflow state machine
unified trigger, idempotency, policy, and effect law
restart, migration, replay, branch, and reproduction
proposal-only workflow generation and evolution
published execution and evaluation scale envelopes
complete inspectable and verifiable provenance
```

### 16.4 ForgeOS may

```text
show available Nyx workflows
supply project-scoped parameters and constraints
start, pause, approve, cancel, and inspect through Nyx contracts
show step state and returned artifacts
independently verify Forge project effects
```

### 16.5 ForgeOS may not

```text
hide a workflow runner inside Forge command routing
turn shell scripts or GUI macros into fake Nyx workflows
keep canonical workflow continuation only in Forge state
install model-generated workflows without Nyx proposal and policy law
re-execute live effects during a supposed replay
```

---

## 17. Cross-cutting capability ownership index

| Capability requested by ForgeOS | Canonical owner | ForgeOS role | Wrong implementation |
|---|---|---|---|
| Nyx server health | Nyx API | client, readiness display | process-presence equals health |
| Nyx schema and capability discovery | Nyx API | compatibility check | Forge invents capability list |
| OpenAI-compatible models | Nyx API + Routing | display and selection request | Forge calls provider directly |
| Local model discovery | Nyx Routing + Runtime | display | Forge scans provider as canonical truth |
| Model invocation | Nyx backend and Agent paths | request and consume | Forge embeds provider SDK |
| Non-streaming chat | Nyx API + Agent | client and UI | Forge owns a second chat loop |
| Streaming chat | Nyx API + Agent | stream client and UI | Forge chunks a finished response and calls it streaming |
| Embeddings | Nyx API + Routing | request | Forge owns hidden embedder |
| Session lifecycle | Nyx state + Persistence | create, select, display | Forge session store becomes AI session truth |
| Thread lifecycle | Nyx state + Persistence | create, select, display | Forge stores canonical conversation thread |
| Accepted operation identity | Nyx Persistence + API | correlate | Forge synthesizes missing Nyx identity |
| Context assembly | Nyx Context | provide authorized candidates and pins | Forge builds hidden prompt |
| Repository observation for AI | Nyx Repo | provide root, revision, scope | Forge AI index replaces Nyx Repo |
| Source citations | Nyx Repo + Context + Observability | display and verify | Forge invents citations |
| Tool registry | Nyx Tools | display and expose Forge declarations | Forge creates hidden model tool registry |
| Safe reads | Nyx Tools + Repo + Policy | permit and verify | Forge UI reads for model outside Nyx audit |
| File writes and patches | Nyx Tools + Policy + Temporal | approve, verify, apply through real tools | model writes directly from UI |
| Process and command execution | Nyx Tools + Policy | grant allowlisted command envelope | arbitrary shell execution from response text |
| Network tools | Nyx Tools + Policy | configure and approve | direct hidden network calls |
| Policy decision | Nyx Policy | display and optionally narrow | Forge broadens or replaces decision |
| Human approval | Nyx Policy + Persistence | collect explicit decision | button state without bound action digest |
| Checkpoint | Nyx Policy + Agent + Persistence | display and decide | Forge mutates paused action |
| Exact resume | Nyx Agent + Persistence | request continuation | model regenerates action after approval |
| Agent run state | Nyx Agent | display and control | Forge fabricates progress |
| Budgets and retries | Nyx Agent + Routing + Policy | provide maximum constraints | Forge silently resets budget |
| Remote heavyweight task | Nyx Agent + Routing + Policy | submit bounded envelope | Forge bypasses Nyx to provider |
| Personas and multi-agent | Nyx Agent + Context + Policy | select and observe | Forge spawns ad hoc agent loops |
| Workflow registry | Nyx Workflow | display and start | Forge command list becomes workflow truth |
| Schedules and triggers | Nyx Workflow + Temporal + Persistence | configure | Forge timer executes hidden workflow |
| AI memory | Nyx Memory | permit scope and display provenance | Forge stores model memories as project state |
| Backend selection | Nyx Routing | request preference and display result | Forge chooses hidden provider |
| Fallback and circuit state | Nyx Routing | display | Forge retries around Nyx |
| llama-server lifecycle behind Nyx | Nyx Runtime and Routing | display/control through Nyx | Forge starts provider runtime behind Nyx's back |
| nyx_server process lifecycle | Forge service supervision | start, stop, restart, readiness poll | Nyx runtime route confused with server process |
| Snapshots and diffs | Nyx Temporal | request and display | Forge copies DTOs and claims replay |
| Replay, branch, restore, rollback | Nyx Temporal + Policy + Tools | approve, display, independently verify | UI animation or Git command alone called Nyx replay |
| Durable Nyx state | Nyx Persistence | configure root and inspect health | Forge writes Nyx storage directly |
| Migrations and repair | Nyx Persistence | invoke supported operation | Forge startup rewrites Nyx files |
| Run records | Nyx Observability | display and correlate | Forge reconstructs record from logs |
| Traces and tool audit | Nyx Observability | display and verify | Forge omits failures |
| Metrics and incidents | Nyx Observability | display | Forge infers health from one request |
| Proposal-only self-forging | Nyx Agent + Workflow + Tools + Policy | review and accept or reject | Forge auto-applies model-generated internals |
| Plugin lifecycle | Nyx Tools + Workflow + Policy + Persistence | install/control through Nyx | Forge loads plugin into client process |
| Exports and external proof | Nyx Observability + Persistence | request and verify | Forge emits unverifiable summary |
| Forge project skill state | Forge Core | canonical owner | Nyx advances Forge skill |
| Forge patch acceptance | Forge Core + user + real tools | canonical owner | Nyx auto-applies or blesses patch |
| Git/build/test truth | real tools, recorded by Forge | canonical verifier | Nyx prose equals passing tool result |

---

## 18. ForgeOS responsibilities that remain legal and required

The anti-duplication law does not mean ForgeOS is a dumb terminal. ForgeOS has
real integration work to do.

### 18.1 Client boundary

```text
versioned transport
bounded timeouts and body sizes
schema and capability compatibility
stable typed error mapping
correlation propagation
cancellation propagation
no provider dependencies
no server handlers
no Nyx canonical stores
```

### 18.2 Service supervision

```text
configured nyx_server executable
working directory and environment
start, stop, restart, crash observation
process-group cleanup
stdout and stderr capture
readiness polling through Nyx public API
operator-visible unavailable, incompatible, degraded, and ready states
```

### 18.3 Forge project envelope

```text
project ID
repository ID
workspace/worktree ID
canonical path
source revision
allowed files and directories
registered commands
required validation
budget ceiling
user-selected task and requested outcome
```

Nyx may narrow this envelope. It may never widen it.

### 18.4 Operator experience

```text
model and session selection controls
conversation and run views
context and citation inspection
tool request and policy decision views
approval, denial, expiry, cancel, and retry controls
patch and artifact review
Nyx diagnostics and evidence views
clear missing, partial, stale, and incompatible states
```

### 18.5 Independent verification

```text
confirm returned file paths are in scope
confirm file bytes and hashes
inspect Git status and diff through real Git
run declared format, build, and test commands
compare file touches to observed effects
reject stale revision or worktree results
record Forge-owned acceptance or rejection
```

---

## 19. Forbidden duplicate implementations inside ForgeOS

The following shapes are invalid even if tests are green:

```text
crates/forge-nyx-client/src/server.rs
  -> Nyx HTTP or binary handlers implemented in ForgeOS

crates/forge-nyx-client/src/session_store.rs
  -> canonical AI sessions or conversation history

crates/forge-nyx-client/src/agent_loop.rs
  -> model-turn or tool-call orchestration

crates/forge-session/src/nyx_runtime.rs
  -> backend or model runtime state that belongs behind Nyx

crates/forge-core/src/nyx_policy.rs
  -> Nyx allow, deny, checkpoint, or approval decisions

crates/forge-world/src/nyx_memory.rs
  -> model memory or hidden prompt state owned by the UI

crates/forge-app/src/direct_ollama.rs
crates/forge-app/src/direct_openai.rs
  -> provider bypass

scripts/fake_nyx_server.py
scripts/nyx_agent.py
  -> product behavior hidden in a helper script

tests/mock_nyx_product.rs
  -> mock-only behavior promoted to closure
```

Fixtures remain legal only for Forge adapter tests. A fixture must be clearly
named, isolated to tests, and incapable of satisfying a real-process closure
requirement.

---

## 20. Contract evolution law

Nyx public behavior evolves from the Nyx repository first.

```text
1. identify the missing or changing Nyx behavior
2. identify the owning Nyx engine and skill
3. patch and prove Nyx under its router
4. publish the versioned Nyx contract and evidence
5. update Forge wiring and ownership copies
6. adapt forge-nyx-client
7. run Forge fixture tests
8. run a real Nyx process witness
9. run cross-repository negative and restart controls
10. only then close the Forge consumer skill
```

ForgeOS may not define a unilateral public contract and demand that Nyx catch up
unless the user explicitly chooses that contract work and the matching Nyx patch
is produced first or in the same coordinated handoff.

Breaking changes require:

```text
explicit version or schema change
compatibility posture
migration or dual-read window when required
negative test for old and new clients
no silent reinterpretation of existing fields
```

---

## 21. Repository routing decision

Use this decision tree before editing:

```text
Does the change alter model, session, context, memory, repo intelligence,
tool, policy, checkpoint, agent, workflow, routing, temporal, persistence,
observability, plugin, proposal, or Nyx API behavior?
  YES -> Nyx_Server owns the source patch.

Does the change alter Forge transport, timeout, process supervision, project
envelope mapping, UI presentation, or independent verification?
  YES -> Forge_OS_V1 owns the source patch.

Does the change alter both?
  YES -> produce separate repository patches with an explicit contract order.
         Do not create a mixed archive or copy source between repositories.

Is the current Nyx repository unavailable?
  YES -> Forge may prepare only a fixture-backed adapter if the contract is
         already authoritative. It may not invent missing Nyx behavior or claim
         real integration.
```

---

## 22. Proof and closure law

A Forge Nyx-facing skill may not close because:

```text
Forge client source exists
Forge fixture tests pass
Nyx has a similarly named DTO
Nyx has a placeholder engine interface
one HTTP request returned 200
one model answer looked correct
Forge UI displayed a green state
Nyx skill documentation describes future behavior
```

Closure requires all applicable evidence:

```text
required Nyx skills BANKED or RELEASE_EARNED
minimum proof level from the cross-repo dependency contract
one shared Nyx-owned versioned contract
real Nyx server process
real Forge client process or product path
positive behavior
malformed, denied, unavailable, incompatible, timeout, and stale controls
restart and rehydration when durability is claimed
exact identity and correlation reconciliation
real effect verification when tools mutate state
no provider bypass
no duplicate Nyx implementation in ForgeOS
```

---

## 23. Nyx skill-tree family crosswalk

```text
API-*           public transport, compatibility, request and response contracts
AGENT-*         bounded AI run progression and multi-agent orchestration
CONTEXT-*       model-visible context construction and explanation
MEMORY-*        AI memory ledger, retrieval, curation, and provenance
OBS-*           run records, traces, reports, metrics, incidents, and proof
PERSIST-*       durable state, transactions, migrations, recovery, and export
POLICY-*        allow, deny, checkpoint, approval, privacy, quotas, and redaction
REPO-*          sandboxed repository observation, indexing, semantics, and impact
ROUTING-*       backend, model, lane, fallback, circuit, and runtime selection
TEMPORAL-*      snapshots, history, replay, branch, restore, and rollback
TOOLS-*         tool and plugin registry, validation, execution, effects, and audit
WORKFLOW-*      durable workflow registry, planning, triggers, continuation, and replay
```

The exact prerequisite list for every Forge V1 Nyx consumer remains in:

```text
docs/versions/V1/FORGEOS_V1_NYX_SERVER_DEPENDENCY_CONTRACT.md
docs/versions/V1/FORGEOS_V1_NYX_SERVER_DEPENDENCIES.json
```

This sheet names ownership. The dependency contract names closure gates. The
wiring sheet names concrete transport and source seams. All three are required.

---

## 24. Nyx source crosswalk for live verification

Inspect the current equivalents of these paths before claiming behavior:

```text
Public server and routes
  crates/nyx_server/src/main.rs
  crates/nyx_server/src/http/mod.rs
  crates/nyx_server/src/http/nyx_diag.rs
  crates/nyx_server/src/http/openai_models.rs
  crates/nyx_server/src/http/openai_chat.rs
  crates/nyx_server/src/http/openai_error.rs

Protocol and DTO ownership
  crates/nyx_protocol/src/openai.rs
  crates/nyx_protocol/src/session.rs
  crates/nyx_protocol/src/context.rs
  crates/nyx_protocol/src/tools.rs
  crates/nyx_protocol/src/checkpoint.rs
  crates/nyx_protocol/src/agent.rs
  crates/nyx_protocol/src/memory.rs
  crates/nyx_protocol/src/runtime.rs
  crates/nyx_protocol/src/temporal.rs
  crates/nyx_protocol/src/run_record.rs
  crates/nyx_protocol/src/errors.rs
  crates/nyx_protocol/src/ids.rs
  crates/nyx_protocol/src/engine.rs

Agent, context, policy, tools, runtime, state, and evidence
  crates/nyx_core/src/agent/**
  crates/nyx_core/src/context/**
  crates/nyx_core/src/policy/**
  crates/nyx_core/src/tools/**
  crates/nyx_core/src/runtime/**
  crates/nyx_core/src/state/**
  crates/nyx_core/src/temporal/**
  crates/nyx_core/src/observability/**
  crates/nyx_core/src/checkpoints.rs

Repository intelligence
  crates/nyx_repo/src/sandbox.rs
  crates/nyx_repo/src/inventory.rs
  crates/nyx_repo/src/fs_io.rs
  crates/nyx_repo/src/search.rs
  crates/nyx_repo/src/snippet.rs

Tool registry and audit
  crates/nyx_tools/src/registry.rs
  crates/nyx_tools/src/audit.rs
  crates/nyx_tools/src/redaction.rs

Memory durability
  crates/nyx_memory/src/ledger.rs
  crates/nyx_memory/src/index.rs
  crates/nyx_memory/src/rotation.rs

Backend adapters
  crates/nyx_backends/src/traits.rs
  crates/nyx_backends/src/target.rs
  crates/nyx_backends/src/ollama.rs
  crates/nyx_backends/src/openai_compat.rs

Nyx workflow and skill authority
  docs/workflow/header.md
  docs/workflow/skill_trees/NYX-SKILL-ROUTER.md
  docs/workflow/skill_trees/NYX-SKILL-ROUTER_STATE.json
  docs/workflow/skill_trees/NYX-SKILL-SEALED-LEDGER.jsonl
  docs/workflow/skill_trees/NYX-SKILL-GRAPH.json
  docs/workflow/skill_trees/NYX-SKILL-API.md
  docs/workflow/skill_trees/NYX-SKILL-AGENT.md
  docs/workflow/skill_trees/NYX-SKILL-CONTEXT.md
  docs/workflow/skill_trees/NYX-SKILL-MEMORY.md
  docs/workflow/skill_trees/NYX-SKILL-OBSERVABILITY.md
  docs/workflow/skill_trees/NYX-SKILL-PERSISTENCE.md
  docs/workflow/skill_trees/NYX-SKILL-POLICY.md
  docs/workflow/skill_trees/NYX-SKILL-REPO.md
  docs/workflow/skill_trees/NYX-SKILL-ROUTING.md
  docs/workflow/skill_trees/NYX-SKILL-TEMPORAL.md
  docs/workflow/skill_trees/NYX-SKILL-TOOLS.md
  docs/workflow/skill_trees/NYX-SKILL-WORKFLOW.md
```

Do not rely on path names alone. Read the current source and execute the current
Nyx experiment or product path.

---

## 25. Patching-model stop conditions

Stop and route to Nyx Server when any of these is true:

```text
The Forge patch needs a new Nyx endpoint.
The Forge patch needs a new Nyx DTO or changes DTO meaning.
The Forge patch needs session, thread, run, context, memory, or checkpoint state.
The Forge patch needs a model or backend call.
The Forge patch needs tool, policy, approval, agent, workflow, or replay logic.
The Forge patch needs durable Nyx storage or migration.
The Forge patch needs Nyx run evidence that the current server does not emit.
The Forge patch needs a capability that is PLANNED or MISSING in this sheet.
```

Stop and route to ForgeOS when any of these is true:

```text
Nyx already exposes the behavior but Forge cannot call it correctly.
Forge timeout, cancellation, error, or compatibility handling is wrong.
Forge starts or stops the nyx_server process incorrectly.
Forge maps project, repository, worktree, revision, or scope incorrectly.
Forge UI lies about Nyx state or evidence.
Forge verification fails to reconcile real effects.
```

---

## 26. Clean Nyx-facing Forge patch checklist

```text
[ ] wiring sheet reviewed
[ ] ownership sheet reviewed
[ ] cross-repo dependency contract reviewed
[ ] exact requested behavior named
[ ] canonical owner named
[ ] current Nyx implementation state verified from source
[ ] relevant Nyx skill family and exact gate identified
[ ] current Nyx source files inspected
[ ] Forge paths limited to client, supervision, envelope, UI, or verification
[ ] no provider bypass
[ ] no server handler in ForgeOS
[ ] no canonical Nyx state store in ForgeOS
[ ] no model or agent loop in ForgeOS
[ ] no policy, checkpoint, tool, workflow, memory, routing, temporal, or run-record clone
[ ] fixture clearly test-only
[ ] real Nyx witness planned or executed as required
[ ] negative controls included
[ ] restart and durability claims proved when applicable
[ ] Forge independently verifies real project effects
[ ] separate Nyx patch produced when Nyx behavior changes
```

---

## 27. One-screen law

```text
Need AI behavior?
  Nyx owns it.

Need Forge project truth, UI, service supervision, or independent verification?
  ForgeOS owns it.

Need filesystem, Git, compiler, test, debugger, package, or process truth?
  The real tool owns it.

Nyx missing a capability?
  Patch Nyx.

Forge cannot consume an existing Nyx capability?
  Patch Forge.

Both must change?
  Two patches, one versioned contract, no copied implementation.

Fixture passes?
  Good adapter evidence, not integration closure.

Model suggests putting Nyx logic in Forge because it is convenient?
  Reject the patch. Convenience is how architecture turns into soup.
```
