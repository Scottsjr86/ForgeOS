# ForgeOS V1 Execution Router and Skill Registry

Status: `ACTIVE_ROUTER`
Router ID: `FORGEOS-V1-EXECUTION-ROUTER-V1`
Release target: `FORGEOS_V1_FIRST_ARMOR`
Canonical worksheet: `docs/versions/V1/FORGEOS_V1_FIRST_ARMOR_SKILL_TREE.md`
Workflow authority: `docs/workflow/WORKFLOW_AUTHORITY.md`
Skill-tree method: `docs/workflow/SKILL_TREE_WORKFLOW_METHOD.md`
Permanent laws: `docs/GOVERNING_LAWS.md`
Mission authority: `docs/MISSION_FORGEOS.md`
Fresh-session authority: `docs/ForgeOS_header.md`
Future closure authority: `docs/versions/V1/V1_CLOSURE_EXPERIMENT.md`
Nyx wiring cheat sheet: `docs/versions/V1/FORGEOS_NYX_SERVER_WIRING_CHEAT_SHEET.md`
Nyx capability and ownership cheat sheet: `docs/versions/V1/FORGEOS_NYX_SERVER_CAPABILITY_OWNERSHIP_CHEAT_SHEET.md`
Nyx cross-repo closure contract: `docs/versions/V1/FORGEOS_V1_NYX_SERVER_DEPENDENCY_CONTRACT.md`
Nyx cross-repo machine map: `docs/versions/V1/FORGEOS_V1_NYX_SERVER_DEPENDENCIES.json`

---

## 0. Purpose

This document is the live execution router and skill-registration authority for
ForgeOS V1 First Armor.

It has two jobs:

1. register the exact canonical V1 skills that may participate in execution; and
2. select, isolate, pause, close, invalidate, and hand off the small active
   frontier without creating a second roadmap.

The canonical skill worksheet defines what each capability means, its direct
prerequisites, its required behavior, its forbidden shortcuts, its user
acceptance, and its regression shield.

This router defines what may be worked now.

```text
V1 skill tree
  owns capability contracts and dependencies

workflow authority
  owns execution law

this router
  owns live selection, concurrency, write isolation, and baton state

source and real tools
  own current behavior

user acceptance
  owns bounded V1 functional approval

V1 closure experiment
  alone may award the release apex
```

No skill becomes active because it is nearby in the document, lower in tier,
visually attractive, easy to implement, mentioned by a model, or requested
without closed prerequisites.

---

## 0.1 Nyx-facing routing preflight

Before this router may authorize a Nyx-facing source edit, the execution packet
must contain:

```text
required_reading:
  - docs/versions/V1/FORGEOS_NYX_SERVER_WIRING_CHEAT_SHEET.md
  - docs/versions/V1/FORGEOS_NYX_SERVER_CAPABILITY_OWNERSHIP_CHEAT_SHEET.md
  - docs/versions/V1/FORGEOS_V1_NYX_SERVER_DEPENDENCY_CONTRACT.md
nyx_wiring_review: COMPLETE
nyx_ownership_review: COMPLETE
nyx_requested_behavior: <plain-language behavior>
nyx_canonical_owner: FORGE_CORE|FORGE_ADAPTER|NYX_SERVER|REAL_TOOL|LINUX_HOST
nyx_engine_family: API|AGENT|CONTEXT|MEMORY|OBSERVABILITY|PERSISTENCE|POLICY|REPO|ROUTING|TEMPORAL|TOOLS|WORKFLOW|NONE
nyx_surface_touched: <exact route, DTO, process seam, or adapter>
nyx_implementation_state: REAL|PARTIAL|CONTRACT_ONLY|PLANNED|MISSING
nyx_source_files_checked:
  - <current Nyx source paths>
nyx_target_repository: Forge_OS_V1|Nyx_Server
nyx_forge_role: <client, process supervisor, UI, envelope provider, verifier, or none>
nyx_duplicate_implementation_check: PASS
nyx_forbidden_substitute_confirmed: true
```

This applies to every `FORGEOS-V1-NYX-*`, every `FORGEOS-V1-AGENT-*`, every
`NYX-GATE-*`, `crates/forge-nyx-client`, Nyx service supervision, and any UI or
record derived from Nyx. Missing fields mean the packet is incomplete and source
edits stop.

The wiring and ownership cheat sheets are mandatory integration maps, not
substitutes for current Nyx source. Contract changes require the current Nyx repository and a
Nyx-owned patch first.

## 1. Non-authority

This router does not:

- redefine a skill contract;
- replace the canonical worksheet;
- invent missing prerequisites;
- award closure from prose;
- treat proof as user-facing functionality;
- activate a future-version feature;
- bless a test-only or mock-only path;
- authorize edits outside a registered slice;
- allow an agent to promote its own result;
- require archive renaming, archive-number matching, hash bookkeeping, or tar
  regeneration as a workflow gate;
- select work from old plans, TODO order, file adjacency, or roadmap numbering.

If the router conflicts with the current source, inspect the source and reconcile
the router before editing. If the router conflicts with
`docs/GOVERNING_LAWS.md`, the governing laws win.

---

## 2. Current release mode and source-activation gate

Current bounded state:

```text
PROGRAM_MODE=CAPABILITY_PROBE
ACTIVE_RELEASE_TARGET=FORGEOS_V1_FIRST_ARMOR
CANONICAL_SKILL_TREE=docs/versions/V1/FORGEOS_V1_FIRST_ARMOR_SKILL_TREE.md
REGISTERED_SKILL_COUNT=67
GLOBAL_ACTIVE_SKILL_LIMIT=3
ACTIVE_SKILL_LIMIT_PER_LANE=1
CLOSED_SKILLS=[FORGEOS-V1-ARCH-000,FORGEOS-V1-ARCH-001,FORGEOS-V1-GUARD-000,FORGEOS-V1-GUARD-001,FORGEOS-V1-GUARD-002,FORGEOS-V1-CONTRACT-000,FORGEOS-V1-PROCESS-000,FORGEOS-V1-PATH-000,FORGEOS-V1-STATE-000,FORGEOS-V1-HASH-000,FORGEOS-V1-PROJECT-100,FORGEOS-V1-FILE-100,FORGEOS-V1-EDITOR-100,FORGEOS-V1-PARSER-100,FORGEOS-V1-LSP-100,FORGEOS-V1-TERMINAL-100,FORGEOS-V1-COMMAND-100,FORGEOS-V1-SESSION-100,FORGEOS-V1-GIT-100,FORGEOS-V1-GIT-101,FORGEOS-V1-PATCH-100,FORGEOS-V1-PROJECT-200,FORGEOS-V1-NYX-100,FORGEOS-V1-NYX-101,FORGEOS-V1-WORLD-100,FORGEOS-V1-RECOVERY-100,FORGEOS-V1-FILE-200,FORGEOS-V1-EDITOR-200,FORGEOS-V1-EDITOR-201,FORGEOS-V1-TERMINAL-200,FORGEOS-V1-COMMAND-200,FORGEOS-V1-GIT-200,FORGEOS-V1-GIT-201,FORGEOS-V1-VERIFY-200,FORGEOS-V1-SESSION-200,FORGEOS-V1-SESSION-201]
AVAILABLE_SKILLS=[FORGEOS-V1-NYX-200]
BLOCKED_SKILLS=[FORGEOS-V1-AGENT-100]
ACTIVE_SKILLS=[]
ACTIVE_BATON_OWNER=FORGEOS-V1-AGENT-100
ACTIVE_REPOSITORY=Nyx_Server
ACTIVE_LANE=REMOTE_AGENT
ACTIVE_SLICE=NONE_UNTIL_NYX_GATE_IS_PROVED
SOURCE_WORK_AUTHORIZED=NO_UNTIL_NEWEST_NYX_SERVER_ARCHIVE_IS_VERIFIED
QUEUED_FIRST_SKILL=NONE
NEXT_ACTION=COMPLETE_NYX_GATE_FORGEOS_V1_AGENT_100_IN_NYX_SERVER
FINAL_ACTIVATION_REQUIRED=YES_AFTER_GATE_RETURN
NYX_GATE_REQUEST=docs/versions/V1/skills/FORGEOS-V1-AGENT-100/NYX_GATE_REQUEST.md
CI_ENTRYPOINT=TARGET_REPOSITORY_OWNED_CI
CI_ALLOWED=[BEHAVIOR_TESTS,GOLDENS,STRUCTURAL_GUARDS]
CI_FORBIDDEN=[DOCUMENTATION,GIT_STATE,FORMATTING,MARKDOWN_STATUS]
```

All Tier-0 foundations, all three structural guards, project registration,
boundary-safe file access, editor buffer identity, incremental Rust parsing, the
Rust Analyzer process/JSON-RPC foundation, native PTY support, immutable
registered-command policy, deterministic session lifecycle, native Git inspection
and mutation primitives, stable patch application, persistent project registry and
workspace restoration, Nyx public health/version discovery, source-backed Forge
World projection, recovery, repository browsing/search, multi-buffer atomic save,
Rust language intelligence, managed project terminals, registered command execution,
consistent Git inspection, safe Git mutation, version-bound verification, dedicated
session bootstrap, managed external Nyx lifecycle, and Nyx-owned permission,
checkpoint, exact resume, replay rejection, and audit consumption are closed.

The operator ran Forge behavior CI with 80 suites, 385 tests passed, 0 failed, and
3 ignored, then ran the independent real Nyx permission witness successfully. The
witness created one exact `repo.write_file` checkpoint, approved and consumed it,
verified the returned hashes and audit sequence, and rejected replay. Therefore
`FORGEOS-V1-NYX-101` is closed.

The router selects `FORGEOS-V1-AGENT-100` next because it is the lowest valid
dependency-depth frontier node. Its required behavior remains Nyx-owned. ForgeOS
may not start a client slice until Nyx_Server returns system proof for
`AGENT-FOUND-002`, `AGENT-RUN-001`, `AGENT-BUDGET-001`, `ROUTING-COST-001`, and
`PERSIST-RUN-001`.

## 3. Current-source intake law

The newest clean user-supplied ForgeOS archive is the only source authority for
the current turn.

```text
newest clean supplied archive
  -> extract into an empty directory
  -> inspect repository identity and current source
  -> verify that it is internally coherent enough for the requested work
  -> ignore and discard all older supplied archives
  -> continue from current source
```

Archive names such as `base_5`, `base_6`, or later are convenient human labels.
They have no scheduling or proof authority.

A current clean archive must not be rejected because:

- its numeric suffix does not match a document;
- an earlier archive hash is absent;
- a prior archive was not recorded in the router;
- the repository was not repackaged after a documentation-only change;
- a model predicted a different next filename.

Stop only when the supplied archive is corrupt, is the wrong repository, lacks a
required already-declared source dependency, contains unresolved conflicting
work, or does not match the user's claim that it is the current clean source.

---

## 4. Canonical skill registration

The canonical registry contains exactly the 67 skill IDs declared by
`FORGEOS_V1_FIRST_ARMOR_SKILL_TREE.md`.

A skill may participate in routing only when:

- its ID appears in this registry;
- its full contract exists in the canonical worksheet;
- every direct prerequisite ID resolves;
- the dependency graph remains acyclic;
- its owner and release tier are unambiguous;
- it belongs to V1 First Armor;
- it has not been silently replaced or renamed.

### 4.1 Tier 5 registry

```text
FORGEOS-V1-APEX-001
```

### 4.2 Tier 4 registry

```text
FORGEOS-V1-WORKSTATION-400
FORGEOS-V1-CODE-400
FORGEOS-V1-SOURCE-400
FORGEOS-V1-NYX-400
FORGEOS-V1-AGENT-400
FORGEOS-V1-VERIFY-400
FORGEOS-V1-WORLD-400
FORGEOS-V1-SELFHOST-400
```

### 4.3 Tier 3 registry

```text
FORGEOS-V1-SESSION-300
FORGEOS-V1-PROJECT-300
FORGEOS-V1-CODE-300
FORGEOS-V1-TERMINAL-300
FORGEOS-V1-GIT-300
FORGEOS-V1-NYX-300
FORGEOS-V1-NYX-301
FORGEOS-V1-AGENT-300
FORGEOS-V1-PATCH-300
FORGEOS-V1-VERIFY-300
FORGEOS-V1-WORLD-300
FORGEOS-V1-RECOVERY-300
FORGEOS-V1-DIST-300
```

### 4.4 Tier 2 registry

```text
FORGEOS-V1-PROJECT-200
FORGEOS-V1-SESSION-200
FORGEOS-V1-SESSION-201
FORGEOS-V1-FILE-200
FORGEOS-V1-EDITOR-200
FORGEOS-V1-EDITOR-201
FORGEOS-V1-TERMINAL-200
FORGEOS-V1-COMMAND-200
FORGEOS-V1-GIT-200
FORGEOS-V1-GIT-201
FORGEOS-V1-NYX-200
FORGEOS-V1-NYX-201
FORGEOS-V1-NYX-202
FORGEOS-V1-AGENT-200
FORGEOS-V1-AGENT-201
FORGEOS-V1-VERIFY-200
FORGEOS-V1-WORLD-200
FORGEOS-V1-RECOVERY-200
FORGEOS-V1-DIST-200
```

### 4.5 Tier 1 registry

```text
FORGEOS-V1-PROJECT-100
FORGEOS-V1-SESSION-100
FORGEOS-V1-FILE-100
FORGEOS-V1-EDITOR-100
FORGEOS-V1-PARSER-100
FORGEOS-V1-LSP-100
FORGEOS-V1-TERMINAL-100
FORGEOS-V1-COMMAND-100
FORGEOS-V1-GIT-100
FORGEOS-V1-GIT-101
FORGEOS-V1-NYX-100
FORGEOS-V1-NYX-101
FORGEOS-V1-AGENT-100
FORGEOS-V1-PATCH-100
FORGEOS-V1-WORLD-100
FORGEOS-V1-RECOVERY-100
```

### 4.6 Tier 0 registry

```text
FORGEOS-V1-ARCH-000
FORGEOS-V1-ARCH-001
FORGEOS-V1-GUARD-000
FORGEOS-V1-GUARD-001
FORGEOS-V1-GUARD-002
FORGEOS-V1-CONTRACT-000
FORGEOS-V1-STATE-000
FORGEOS-V1-PATH-000
FORGEOS-V1-PROCESS-000
FORGEOS-V1-HASH-000
```

### 4.7 Registration-change law

Adding, removing, splitting, merging, or renaming a V1 skill requires one
explicit registration change that:

1. states why the existing graph cannot truthfully represent the capability;
2. updates the canonical worksheet and this registry together;
3. preserves permanent IDs for unchanged capability meaning;
4. migrates all direct prerequisite and unlock references;
5. revalidates uniqueness, resolution, and acyclicity;
6. identifies every active, closed, or invalidated skill affected;
7. does not award closure or release credit;
8. receives user approval before becoming canonical.

A temporary implementation inconvenience is not sufficient reason to mutate the
skill graph.

---

## 5. Skill state authority

Every registered skill has exactly one live state:

```text
LOCKED
AVAILABLE
ACTIVE
BLOCKED
SOURCE_PROVED
USER_ACCEPTANCE_READY
CLOSED
INVALIDATED
RELEASE_EARNED
```

The canonical worksheet defines these meanings. This router controls legal live
transitions.

Allowed transitions:

```text
LOCKED -> AVAILABLE
AVAILABLE -> ACTIVE
ACTIVE -> BLOCKED
BLOCKED -> ACTIVE
ACTIVE -> SOURCE_PROVED
SOURCE_PROVED -> USER_ACCEPTANCE_READY
USER_ACCEPTANCE_READY -> CLOSED
CLOSED -> INVALIDATED
INVALIDATED -> ACTIVE
CLOSED -> RELEASE_EARNED only for the apex through closure authority
```

Also legal:

```text
ACTIVE -> AVAILABLE
BLOCKED -> AVAILABLE
```

only when a skill is deliberately parked with no source claim, no hidden active
slice, and a resumable pause record.

Forbidden transitions include:

```text
LOCKED -> ACTIVE
AVAILABLE -> CLOSED
ACTIVE -> CLOSED
SOURCE_PROVED -> CLOSED without user acceptance
CLOSED -> RELEASE_EARNED for a non-apex skill
INVALIDATED -> CLOSED without rerunning the failed path
```

No `PARTIAL`, `DEFERRED`, `BACKEND_DONE`, `PROOF_ONLY`, or `CLOSE_ENOUGH` state
exists.

---

## 6. Lane registration

Every active skill occupies exactly one primary lane.

```text
ARCHITECTURE_AND_CONTRACTS
SESSION_AND_DISTRIBUTION
PROJECT_AND_PERSISTENCE
EDITOR_AND_LANGUAGE
TERMINAL_AND_COMMANDS
GIT_AND_PATCHES
NYX_LOCAL_AI
REMOTE_AGENT
FORGE_WORLD
VERIFICATION_AND_RELEASE
```

Lane assignment follows the skill's owning truth, not the file currently being
edited.

Examples:

```text
FORGEOS-V1-ARCH-000     -> ARCHITECTURE_AND_CONTRACTS
FORGEOS-V1-SESSION-100  -> SESSION_AND_DISTRIBUTION
FORGEOS-V1-PROJECT-100  -> PROJECT_AND_PERSISTENCE
FORGEOS-V1-EDITOR-100   -> EDITOR_AND_LANGUAGE
FORGEOS-V1-TERMINAL-100 -> TERMINAL_AND_COMMANDS
FORGEOS-V1-GIT-100      -> GIT_AND_PATCHES
FORGEOS-V1-NYX-100      -> NYX_LOCAL_AI
FORGEOS-V1-AGENT-100    -> REMOTE_AGENT
FORGEOS-V1-WORLD-100    -> FORGE_WORLD
FORGEOS-V1-VERIFY-200   -> VERIFICATION_AND_RELEASE
```

One active skill per lane is permitted. A skill touching multiple lanes still
occupies one primary lane and declares every secondary contract it touches.

---

## 7. Global concurrency law

```text
GLOBAL_ACTIVE_SKILL_LIMIT=3
ACTIVE_SKILL_LIMIT_PER_LANE=1
```

The normal operating target is one active skill.

A second or third active skill is allowed only when independence is proved before
activation. Parallelism is an optimization, never a quota.

Two skills may not be active together when any of the following is true:

- one directly or transitively depends on the other;
- both alter the same public contract;
- both write the same file or overlapping directory;
- both depend on an unstable shared prerequisite;
- both require incompatible migrations;
- one changes assumptions used by the other's tests or acceptance path;
- both alter the same canonical state schema;
- both alter Forge Core authority boundaries;
- both alter the same Nyx protocol surface;
- merging either first could invalidate the other;
- either skill lacks exact write boundaries.

Every parallel skill requires a separate branch or worktree.

If a conflict appears after activation:

```text
stop both affected slices
  -> preserve their work without merging
  -> identify the shared prerequisite or ownership collision
  -> park one, narrow both, or activate the common prerequisite
  -> resume only after the router becomes conflict-free
```

Git conflict resolution is not a substitute for architectural independence.

---

## 8. Prerequisite and frontier law

A skill becomes `AVAILABLE` only when every direct prerequisite in the canonical
worksheet is `CLOSED` and remains valid.

An `AVAILABLE` skill is startable only when:

- source inspection confirms its assumptions;
- no active skill conflicts with it;
- its user-facing or operator-facing path is identifiable;
- its first probe can expose a real blocker;
- its owner, lane, repository, and write boundaries can be declared;
- its work belongs to V1;
- starting it does not hide a missing lower foundation.

Default selection order:

```text
closed prerequisites
  -> lowest valid dependency depth
  -> shortest bounded route to user-visible V1 behavior
  -> smallest coherent blast radius
  -> no contract or path collision
  -> source reality confirms the need
```

The user may manually select any startable skill. A locked or conflicting skill
must not be activated even by explicit request. Report the exact prerequisite or
conflict instead.

When a skill closes, reevaluate only:

- its direct unlocks;
- skills invalidated by its changed contracts;
- the current active frontier.

Do not rescan the entire tree and automatically activate a new node.

---

## 9. Activation packet

Before a skill enters `ACTIVE`, this router must contain one complete activation
packet:

```yaml
skill_id:
state: ACTIVE
lane:
owning_subsystem:
source_repository:
source_revision:
worktree_or_branch:
direct_prerequisites:
originating_path_or_probe:
first_blocker:
active_slice:
allowed_paths:
forbidden_paths:
public_contracts_touched:
required_commands:
regression_commands:
pass_edge:
block_edge:
user_acceptance_path:
return_path:
parallel_compatibility:
```

A field may be `NONE` only when the contract genuinely does not require it.
Missing fields do not grant freedom. They block activation.

### 9.1 Write-boundary law

`allowed_paths` names the exact writable area for the current slice.

Everything else is forbidden by default.

If source inspection proves another path is required:

```text
stop editing
  -> record the dependency or seam reason
  -> expand the slice explicitly or activate a prerequisite
  -> recheck active-skill conflicts
  -> then edit
```

Do not edit first and justify the expansion afterward.

---

## 10. First-blocker and slice law

Every active skill has exactly one first causal blocker and one active slice.

```text
run or inspect the real path
  -> locate the earliest failed causal edge
  -> record one blocker
  -> cut one bounded slice expected to move that edge
  -> run focused tests and guards
  -> rerun the same originating path
  -> close, expose the next blocker, or invalidate the attempt
```

A blocker is not:

- a list of every missing future feature;
- a broad architectural dissatisfaction;
- a convenient refactor opportunity;
- a documentation gap;
- a request to implement several prerequisites at once.

Required blocker record:

```yaml
blocker_id:
skill_id:
statement:
blocker_class:
owning_subsystem:
discovered_by:
source_revision:
resolution_condition:
status: OPEN
```

Required slice record:

```yaml
slice_id:
skill_id:
blocker_id:
goal:
allowed_paths:
forbidden_paths:
public_contracts_touched:
required_commands:
regression_commands:
pass_condition:
block_condition:
return_path:
```

A new blocker replaces the old blocker only after the old edge has actually
moved.

---

## 11. Guard and regression routing

Every source slice must run the guards and regression commands relevant to its
seams.

The only permitted non-behavior guard classes are:

1. authored Rust module-size guard, with a preferred module size near 500
   physical lines, skill closure blocked above 1000 physical lines, temporary
   breathing room through 1200 physical lines, and a hard verifier failure at
   1201 or more physical lines;
2. Forge Core purity guard;
3. cross-subsystem seam and dependency-direction guard;
4. skill graph and router integrity guard.

The router-integrity guard may enforce structured facts only:

- exactly 67 registered V1 IDs unless an approved registration migration changes
  the count;
- globally unique IDs;
- resolved direct prerequisites;
- no dependency cycles;
- no more than three active skills;
- one active skill per lane;
- no dependency relation between active skills;
- no overlapping active write paths;
- no active public-contract conflict;
- every active skill has closed prerequisites;
- every active skill has one blocker and one slice;
- every closed skill has both required closure records;
- invalidated skills block their dependents;
- one exact baton owner and source repository are named.

It may not parse Markdown prose and decide that product behavior exists.
Documentation CI, prose-status validators, checklist parsers, and any
verifier that awards behavior from document wording are forbidden.

A later skill may not close by weakening any already-closed skill's test,
negative path, acceptance, guard, schema meaning, or real product behavior.

### 11.1 Validation execution authority

Each active skill records one validation execution state:

```text
ASSISTANT_VALIDATED
OPERATOR_VALIDATION_PENDING
OPERATOR_VALIDATED
```

Use `ASSISTANT_VALIDATED` only for commands actually executed in the assistant
environment. Use `OPERATOR_VALIDATION_PENDING` when the assistant has prepared and
apply-checked the bounded patch but cannot run required host tooling. Use
`OPERATOR_VALIDATED` only after the user returns the exact command results.

The lack of Rust or host tooling in the assistant environment may not move an
otherwise actionable skill to `BLOCKED`. The skill remains `ACTIVE` until the
operator results expose a real blocker or satisfy the required validation.

---

## 12. Source-proved, user acceptance, and closure routing

A skill moves to `SOURCE_PROVED` only when:

- the complete bounded source behavior exists;
- the real registered path exercises it;
- focused automated tests pass;
- required negative and failure paths pass;
- relevant non-behavior guards pass;
- earlier closed-skill regressions remain green;
- no authored source module exceeds 1000 physical lines;
- the behavior is ready to present through the real V1 path.

It moves to `USER_ACCEPTANCE_READY` only when the exact user or operator steps are
prepared and no manual hidden repair is required.

It moves to `CLOSED` only when:

- the user personally exercises the behavior;
- the user explicitly approves the behavior within V1 scope;
- no required criterion is deferred;
- `CLOSURE_AND_SPEC.md` exists for the skill;
- `USER_GUIDE_SOURCE.md` exists for the skill;
- closure is recorded atomically in the worksheet and router;
- direct dependents are reevaluated but not auto-activated.

Proof documents do not replace functionality. Documentation does not close a
skill. User approval does not excuse missing automated locks or broken negative
paths.

---

## 13. Required per-skill closure records

Every closed skill owns exactly two capability documents under a stable skill
path chosen when the first skill closes:

```text
CLOSURE_AND_SPEC.md
USER_GUIDE_SOURCE.md
```

`CLOSURE_AND_SPEC.md` records:

- final bounded capability contract;
- authoritative source path;
- public path;
- tests and commands actually run;
- negative and failure behavior;
- relevant guard results;
- regression locks;
- user acceptance performed;
- exact non-claims;
- final module boundaries.

`USER_GUIDE_SOURCE.md` records all information needed later for the onboard Forge
Guide and website:

- what the user can do;
- how to access the behavior;
- inputs and controls;
- visible states and outputs;
- options and customization;
- limitations within V1;
- expected failures and recovery;
- accessibility and keyboard behavior where applicable;
- troubleshooting and support facts.

The second document may be rough in wording. It must be complete enough that
later publication requires presentation polish, not rediscovery of product
behavior.

---

## 14. Invalidation law

A closed skill becomes `INVALIDATED` when current evidence shows that its declared
behavior, negative path, public path, schema contract, user acceptance, or
regression shield no longer holds.

Invalidation immediately:

- blocks direct and transitive dependents from new activation;
- prevents affected release claims;
- records the exact failed edge;
- preserves prior closure records as history;
- routes the invalidated skill back through one current blocker and slice;
- forbids pretending the old user approval applies to changed behavior.

Fixing an invalidated skill requires rerunning its complete current closure path,
including user acceptance when the user-facing contract changed.

---

## 15. Pause, resume, and baton handoff

Every active or blocked skill must be resumable without chat history.

Pause record:

```yaml
skill_id:
state:
lane:
source_repository:
source_revision:
worktree_or_branch:
originating_path_or_probe:
last_run:
first_blocker:
active_slice:
allowed_paths:
last_green_commands:
uncommitted_state:
next_action:
```

Resume law:

```text
newest clean supplied source
  -> inspect current implementation and Git state
  -> locate the recorded skill and worktree
  -> rerun or recheck the originating path
  -> confirm, replace, or clear the blocker
  -> revalidate active-skill conflicts
  -> then continue editing
```

A handoff between repositories or authority owners must report:

```text
FROM_OWNER
TO_OWNER
SKILL_ID
WHY
PASS_OR_BLOCK_EDGE
REQUIRED_SOURCE_REPOSITORY
REQUIRED_INPUTS
ALLOWED_PATHS
RETURN_PATH
```

If the required target repository is absent, stop before target-source edits.
Do not manufacture a local substitute or absorb the other repository's authority.

For every `NYX-GATE-*` marker, source implementation and skill closure are separate
decisions. ForgeOS-side client work may proceed against explicit fixtures. Closure
may not proceed until the Nyx contract reports all required Nyx skills `BANKED` or
`RELEASE_EARNED` at the required proof level and the real-server witness passes.

---

## 16. Current registered frontier

The Nyx permission client is closed. The baton is blocked at the Nyx-owned
remote-agent task, budget, cost, cancellation, and durable run-record gate.

```yaml
skill_id: FORGEOS-V1-AGENT-100
state: BLOCKED
lane: REMOTE_AGENT
owning_subsystem: nyx_server, forge-nyx-client
source_repository: Nyx_Server
source_revision: newest clean Nyx_Server archive supplied in the Nyx work thread
worktree_or_branch: target-repository-owned single-skill worktree
direct_prerequisites:
  - FORGEOS-V1-NYX-100
  - FORGEOS-V1-PATH-000
  - FORGEOS-V1-STATE-000
  - FORGEOS-V1-HASH-000
originating_path_or_probe: >
  Inspect the current Nyx agent, routing, provider-cost, cancellation, persistence,
  and run-introspection paths; prove complete, failed, cancelled, and budget-hit
  task records through a real independently running Nyx_Server process.
first_blocker: >
  ForgeOS is forbidden from owning canonical remote-agent run state, and the newest
  Nyx_Server source has not yet returned system proof for AGENT-FOUND-002,
  AGENT-RUN-001, AGENT-BUDGET-001, ROUTING-COST-001, and PERSIST-RUN-001.
active_slice: NONE_UNTIL_NYX_GATE_IS_PROVED
allowed_paths:
  - Nyx_Server paths selected by the Nyx workflow router
  - Nyx-owned public protocol, agent, routing, budget, cost, cancellation,
    persistence, witness, receipt, and workflow-status paths required by the gate
forbidden_paths:
  - Forge_OS_V1 source during the Nyx gate
  - Forge-owned canonical agent-run ledger
  - direct ForgeOS provider calls
  - ForgeOS-calculated provider status or cost from prose
  - private ForgeOS-only Nyx agent protocol
must_pass:
  - all five required Nyx skills are BANKED or RELEASE_EARNED
  - minimum proof level is PROOF_SYSTEM
  - task identity binds provider, model, source revision, worktree, scope, and budget
  - complete, failed, cancelled, and budget-hit terminal states are durable
  - cancellation prevents later continuation
  - recorded cost and budget posture remain truthful
  - one task cannot mutate another task record or worktree
  - a real Nyx_Server witness passes
  - Nyx repository behavior CI passes
  - standalone Nyx chat and development operation remains green
must_not_claim:
  - ForgeOS-side agent client integration
  - FORGEOS-V1-AGENT-100 closure
  - remote provider dispatch from ForgeOS
  - agent patch intake or application
  - project-aware Nyx tools
return_path: >
  Return to the ForgeOS integration thread with the newest clean Nyx_Server archive,
  exact receipt paths and SHA-256 values, public contract version and surfaces, real
  server witness command/result, repository CI result, and standalone chat/dev result.
```

## 17. Cross-repository gate handling for Nyx remote-agent run authority

`FORGEOS-V1-AGENT-100` may not receive a ForgeOS source slice until the current Nyx
repository proves all required gate skills. The Nyx implementation must remain a
general server capability for chat, development, CLI, editor, script, bot, and other
clients. It may not depend on ForgeOS or encode Forge project truth as Nyx-owned
canonical state.

After the gate returns, ForgeOS may activate a thin client-adapter slice that submits
and inspects Nyx-owned task records while preserving exact project, revision,
worktree, scope, provider, model, budget, status, response, and cost identity.
ForgeOS still may not contact providers directly, invent terminal status, calculate
cost from prose, or maintain a competing run ledger.

## 18. Final closure routing

No lower skill, aggregate score, completion percentage, or collection of green
tests may award V1.

The final route is:

```text
all required lower capabilities CLOSED
  -> FORGEOS-V1-SELFHOST-400 CLOSED
  -> activate docs/versions/V1/V1_CLOSURE_EXPERIMENT.md
  -> execute the declared clean-host and self-hosting journeys
  -> user performs and approves the final V1 journey
  -> all closed-skill regressions remain valid
  -> FORGEOS-V1-APEX-001 becomes RELEASE_EARNED
```

Only the closure authority may award `RELEASE_EARNED`.

A release remains unearned if the journey requires:

- another IDE;
- hidden manual patching;
- fake terminal or Git state;
- scripted model output standing in for agent execution;
- operator repair outside ForgeOS;
- an unrecorded permission escalation;
- deferred V1 behavior;
- a closed skill that has become invalidated.

---

## 19. Fresh-session router sequence

Every source-capable ForgeOS turn must:

```text
1. read docs/ForgeOS_header.md
2. identify the baton repository from the live router
3. extract the newest clean user-supplied archive for that repository into an empty directory
4. ignore all older archives and inspect current source and Git-equivalent state
5. read docs/MISSION_FORGEOS.md
6. read docs/GOVERNING_LAWS.md
7. read docs/workflow/WORKFLOW_AUTHORITY.md section 2
8. read docs/workflow/SKILL_TREE_WORKFLOW_METHOD.md
9. read this router
10. read docs/versions/V1/ForgeOS_V1_Skill_Status_Master_List.md as a status mirror
11. read ci/master.yaml and preserve its behavior-only CI boundary
12. read only the active skill's full canonical worksheet contract
13. verify source work is authorized
14. verify every direct prerequisite is CLOSED
15. verify active-skill count, lane, dependency, path, contract, and worktree isolation
16. inspect or run the originating path
17. restate the one skill, one blocker, one slice, pass edge, block edge, allowed
    paths, forbidden paths, public contracts, required CI command, and return path
18. edit only the declared source and authority paths
19. run or hand off only the active repository's declared behavior CI
20. update the mandatory status mirror in the same repository patch as any state transition
```

After every skill-state transition, update the master status mirror in the same patch.
The mirror reports the router and worksheet state; it never selects work.

Do not scan unrelated skills to choose convenient work. Do not reopen closed skills
without evidence of invalidation. Do not create a new worksheet for a source
blocker. Do not stall on archive labels.

---

## 20. Router completion law

The router is healthy when:

```text
all registered IDs resolve
no dependency cycle exists
one exact release target is active
zero to three non-competing skills are active
one active skill exists per lane at most
every active skill has closed prerequisites
every active skill has one blocker and one slice
active write paths and public contracts do not overlap
every closed skill has source, locks, user approval, and two closure records
invalidated skills block dependents
one exact baton owner and source repository are named
no document claims behavior that source has not earned
```

The router is not progress.

It keeps progress from eating itself.
