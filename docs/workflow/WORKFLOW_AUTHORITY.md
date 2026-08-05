# ForgeOS Workflow Authority

Status: `ACTIVE`
Authority ID: `FORGEOS-WORKFLOW-AUTHORITY-V1`
Applies to: ForgeOS V1 planning, source work, integration, testing, user acceptance, and release closure
Canonical V1 worksheet: `docs/versions/V1/FORGEOS_V1_FIRST_ARMOR_SKILL_TREE.md`
Fresh-session header: `docs/ForgeOS_header.md`
Current program mode: `SLICE`
Current release target: `FORGEOS_V1_FIRST_ARMOR`
Nyx wiring cheat sheet: `docs/versions/V1/FORGEOS_NYX_SERVER_WIRING_CHEAT_SHEET.md`
Nyx capability and ownership cheat sheet: `docs/versions/V1/FORGEOS_NYX_SERVER_CAPABILITY_OWNERSHIP_CHEAT_SHEET.md`
Nyx cross-repo contract: `docs/versions/V1/FORGEOS_V1_NYX_SERVER_DEPENDENCY_CONTRACT.md`

---

## 0. Purpose

This document is the live execution authority for ForgeOS.

It decides:

- which program mode is active;
- whether source work is allowed;
- which skill or documentation authority owns the baton;
- which repository owns the next patch;
- how a skill becomes `ACTIVE`, `BLOCKED`, `SOURCE_PROVED`,
  `USER_ACCEPTANCE_READY`, `CLOSED`, `INVALIDATED`, or `RELEASE_EARNED`;
- how many skills may be active at once;
- when parallel work is legal;
- how the first blocker selects the next slice;
- which guards are allowed;
- what proof is required;
- what may never substitute for working user-facing behavior;
- when V1 may be called complete.

The V1 skill tree defines the complete dependency graph and the capability
contracts. This workflow authority controls execution against that graph.

The workflow is not a flat roadmap, phase ladder, checklist queue, or permission
for a model to pick whichever task looks convenient.

---

## 0.1 Mandatory Nyx wiring preflight

Every slice is Nyx-facing when any of the following is true:

```text
its skill ID begins FORGEOS-V1-NYX- or FORGEOS-V1-AGENT-
it carries a NYX-GATE-* marker
it touches crates/forge-nyx-client
it configures or supervises the nyx_server process
it consumes a Nyx endpoint, DTO, header, error, run, tool, policy, checkpoint,
context, memory, model, runtime, or capability claim
it presents Nyx-derived state in Forge World
```

Before source edits, the patching model must read:

```text
docs/versions/V1/FORGEOS_NYX_SERVER_WIRING_CHEAT_SHEET.md
docs/versions/V1/FORGEOS_NYX_SERVER_CAPABILITY_OWNERSHIP_CHEAT_SHEET.md
docs/versions/V1/FORGEOS_V1_NYX_SERVER_DEPENDENCY_CONTRACT.md
```

and report the exact preflight receipt defined by both cheat sheets. It must
name the canonical owner and inspect the current Nyx source files that own the
surface. If the current
Nyx repository is absent, the model may not invent or alter a Nyx public
contract, claim real compatibility, or close a Nyx-gated Forge skill.

Failure to perform this preflight is a wrong-route stop, not permission to build
a local substitute.

## 1. Authority order

When source, documents, agents, tests, or visual surfaces disagree, use this
order:

```text
1. current source behavior and real tool output
2. canonical Forge Core state and recorded product artifacts
3. docs/ForgeOS_header.md
4. this document
5. docs/GOVERNING_LAWS.md
6. docs/versions/V1/V1_EXECUTION_ROUTER.md
7. docs/versions/V1/FORGEOS_V1_FIRST_ARMOR_SKILL_TREE.md
8. docs/versions/V1/V1_CLOSURE_EXPERIMENT.md when activated
9. docs/High_Level.md
10. the selected skill contract and its direct prerequisites
11. subsystem tests, experiments, receipts, and user acceptance records
12. historical plans, prototypes, notes, and superseded archives
```

Only this document and the live V1 router may activate source work.

The skill tree may say a node is structurally `AVAILABLE`. That does not make it
`ACTIVE`. The router must select it and record its lane, owner, first question,
write boundary, and required return path.

Document order, tier number, version adjacency, source-file proximity, TODO order,
or visual proximity in the skill graph has no scheduling authority.

---

## 2. Current live state

The bounded authority migration is complete and source execution is active:

```text
PROGRAM_MODE=SLICE
ACTIVE_RELEASE_TARGET=FORGEOS_V1_FIRST_ARMOR
BATON_OWNER=FORGEOS-V1-AGENT-100
ACTIVE_REPOSITORY=Forge_OS_V1
ACTIVE_SKILLS=[FORGEOS-V1-AGENT-100]
BLOCKED_SKILLS=[]
AVAILABLE_SKILLS=[FORGEOS-V1-NYX-200]
CLOSED_SKILLS=[FORGEOS-V1-ARCH-000,FORGEOS-V1-ARCH-001,FORGEOS-V1-GUARD-000,FORGEOS-V1-GUARD-001,FORGEOS-V1-GUARD-002,FORGEOS-V1-CONTRACT-000,FORGEOS-V1-PROCESS-000,FORGEOS-V1-PATH-000,FORGEOS-V1-STATE-000,FORGEOS-V1-HASH-000,FORGEOS-V1-PROJECT-100,FORGEOS-V1-FILE-100,FORGEOS-V1-EDITOR-100,FORGEOS-V1-PARSER-100,FORGEOS-V1-LSP-100,FORGEOS-V1-TERMINAL-100,FORGEOS-V1-COMMAND-100,FORGEOS-V1-SESSION-100,FORGEOS-V1-GIT-100,FORGEOS-V1-GIT-101,FORGEOS-V1-PATCH-100,FORGEOS-V1-PROJECT-200,FORGEOS-V1-NYX-100,FORGEOS-V1-NYX-101,FORGEOS-V1-WORLD-100,FORGEOS-V1-RECOVERY-100,FORGEOS-V1-FILE-200,FORGEOS-V1-EDITOR-200,FORGEOS-V1-EDITOR-201,FORGEOS-V1-TERMINAL-200,FORGEOS-V1-COMMAND-200,FORGEOS-V1-GIT-200,FORGEOS-V1-GIT-201,FORGEOS-V1-VERIFY-200,FORGEOS-V1-SESSION-200,FORGEOS-V1-SESSION-201]
ACTIVE_LANE=REMOTE_AGENT
ACTIVE_QUESTION=Can ForgeOS consume Nyx-owned remote-agent task, budget, cost, cancellation, terminal-state, and durable run records without contacting providers or creating a competing run ledger?
FIRST_BLOCKER=FORGE_NYX_CLIENT_HAS_NO_PUBLIC_REMOTE_AGENT_API_ADAPTER_OR_INDEPENDENT_RECORD_RECONCILIATION
ACTIVE_SLICE=FORGEOS-V1-AGENT-100-SLICE-001
SOURCE_IMPLEMENTATION_ALLOWED=YES_FOR_DECLARED_FORGE_CLIENT_PATHS_ONLY
CURRENT_RESULT=AGENT_100_NYX_GATE_VERIFIED_FORGE_CLIENT_SLICE_ACTIVE
RETURN_PATH=RUN_FORGE_BEHAVIOR_CI_AND_THE_INDEPENDENT_REAL_NYX_REMOTE_AGENT_WITNESS
CI_AUTHORITY=EACH_REPOSITORY_OWNED_BEHAVIOR_CI
CI_ALLOWED=[BEHAVIOR_TESTS,GOLDENS,STRUCTURAL_GUARDS]
CI_FORBIDDEN=[DOCUMENTATION,GIT_STATE,FORMATTING,MARKDOWN_STATUS]
NYX_WIRING_CHEAT_SHEET_REVIEW=COMPLETE
NYX_CAPABILITY_OWNERSHIP_REVIEW=COMPLETE
NYX_WIRING_AND_OWNERSHIP_PREFLIGHT_RECEIPT=docs/versions/V1/skills/FORGEOS-V1-AGENT-100/NYX_GATE_INPUT.json
```

Persistent project registry/workspace restoration, Nyx health/versioned public
client protocol, Forge World projection/input routing, recovery, repository
browsing/search, multi-buffer atomic save, Rust language intelligence, managed
project-bound terminal sessions, registered command execution, consistency-checked
Git inspection, safe Git mutation, version-bound verification, dedicated
display-manager session bootstrap, and managed external Nyx service lifecycle are
closed.

The Nyx remote-agent gate is verified from `Nyx_Server_base_18.tar`. The active
ForgeOS slice may add only a thin public HTTP client, exact request envelopes,
independent task/source/budget/cost reconciliation, strict response validation, and
the real-process witness. Nyx_Server remains authoritative for routing, provider and
model execution, cancellation, continuation, cost, persistence, and terminal run
truth.

The authority set is closed. No model may invent additional authority documents,
prose validators, checklists, or migration gates before completing the active source
skill.

## 3. Program modes

Exactly one program mode is active globally.

### `DOCUMENT_MIGRATION`

Allowed:

- create or reconcile the bounded authority set;
- correct paths and live-state fields;
- validate the skill graph structurally;
- define repository and subsystem authority;
- define the release route and closure experiment.

Forbidden:

- product source implementation;
- speculative scaffolding;
- source refactors;
- documentation CI;
- prose verifiers;
- adding unplanned authority layers.

### `CAPABILITY_PROBE`

Use when one real V1 capability or public path must be exercised before a source
slice can be selected.

Allowed:

- run the declared path;
- inspect current source;
- capture output;
- classify `PASS`, `VALID_NEGATIVE`, `BLOCKED`, `INVALID`, or `INCONCLUSIVE`;
- identify one first causal blocker.

Forbidden:

- source edits before the blocker is evidenced;
- activating a different skill because it looks easier;
- broad architecture changes without a demonstrated need.

### `SLICE`

Use when one skill owns one bounded implementation slice.

Allowed:

- edit only the declared paths;
- implement only the behavior needed to move the originating path past its
  current blocker;
- add the focused tests and behavioral locks required by the skill;
- update the two closure documents only when closure is actually earned.

Forbidden:

- opportunistic refactors;
- unrelated cleanup;
- hidden prerequisite implementation;
- parallel truth;
- weakening prior behavior;
- starting the next skill before returning to the originating path.

### `INVESTIGATION`

Use for a bounded reproduction, contradiction audit, permission audit,
performance probe, recovery test, architecture audit, provenance check, or
failure isolation.

Ordinary feature work freezes until the investigation returns a result or one
first blocker.

### `PRODUCT_INTEGRATION`

Use when already proved local capabilities must be connected through the real
ForgeOS product path.

Integration may include Forge Core, Developer Bridge, Nyx, Forge World, session
startup, packaging, recovery, or user-facing workflow surfaces.

Integration may not create a second implementation of a capability that already
has an owner.

### `RELEASE_CLOSURE`

Use only when the router activates the V1 closure experiment.

Allowed:

- clean-host installation;
- session boot;
- self-hosting development journey;
- recovery;
- packaging;
- stranger testing;
- final regression and guard execution.

Release closure cannot manufacture missing skill proof.

---

## 4. Canonical archive and fresh-source law

The newest clean repository archive explicitly supplied by the user is the sole
source authority for that repository in the current conversation.

```text
NEWEST_USER_SUPPLIED_CLEAN_ARCHIVE = CANONICAL_SOURCE
ALL_PREVIOUS_ARCHIVES = SUPERSEDED_AND_IGNORED
```

Operational law:

1. Use only the newest user-supplied clean archive for the target repository.
2. Extract it into a new empty directory.
3. Delete or ignore every older extracted copy and older archive from the active
   work area.
4. Inspect the extracted source and verify that it is the repository and state
   the user said it is.
5. If it is coherent and clean, proceed under the current router.
6. Never demand a renamed archive, incremented base number, regenerated tar, or
   matching recorded hash before continuing.
7. Never predict the next filename, base number, or SHA-256.
8. Never treat an old filename or hash in documentation as a gate.
9. A SHA-256 may be computed and reported as intake evidence. It is not workflow
   authority and it may not stall source work.
10. Source work does not require creating a new tar after every patch, skill, or
    green run. The user decides when to package and supply the next archive.

If the user supplies a clean current archive and says it is the fresh source,
the model verifies the extracted repository and then works from it. There is no
archive reconciliation ceremony and no documentation dance.

The only archive-related stop conditions are:

- the archive is corrupt or cannot be extracted;
- it is the wrong repository for the active baton;
- it is visibly not the state the user claimed;
- required source is absent;
- files needed for the declared work are missing.

In those cases, report the concrete defect. Do not request a different archive
merely because its filename or number does not match prior text.

---

## 5. Repository and authority boundaries

ForgeOS and `nyx_server` are separate source authorities unless a later explicit
versioned vendoring decision changes that boundary.

A patch belongs to exactly one repository.

Before crossing repositories, report:

```text
FROM_REPOSITORY
TO_REPOSITORY
ACTIVE_SKILL
FIRST_BLOCKER
WHY_THE_TARGET_REPOSITORY_OWNS_THE_FIX
REQUIRED_HANDOFF_INPUTS
PATCH_POLICY=SEPARATE_TARGET_REPOSITORY_PATCH
```

If the target repository source has not been supplied, stop before target-source
edits.

No repository may quietly absorb another repository's authority.

- Forge Core owns canonical project and capability state.
- Nyx owns model hosting, AI sessions, permissions, checkpoints, and tool runs.
- Forge World owns interaction and presentation.
- Developer Bridge owns explicit adapters to real tools.
- Real tools own their native execution results.

The detailed seam law belongs in `docs/GOVERNING_LAWS.md`.

### 5.1 Nyx cross-repository closure gate

Every ForgeOS skill carrying a `NYX-GATE-*` marker must satisfy the exact Nyx
requirements in
`docs/versions/V1/FORGEOS_V1_NYX_SERVER_DEPENDENCY_CONTRACT.md`.

```text
Forge client or integration source may be implemented against explicit fixtures
  -> allowed while the Nyx gate is unresolved

Forge skill closure or CLOSED status
  -> forbidden until every listed Nyx skill is BANKED or RELEASE_EARNED
  -> required Nyx proof level and real-server witness must pass
```

If the first blocker belongs to a listed Nyx skill, switch repositories through a
separate Nyx_Server patch. Do not widen `forge-nyx-client`, `forge-session`, or any
Forge crate into a second health server, model host, session store, policy engine,
checkpoint engine, tool engine, agent runtime, or run ledger.

A Forge fixture may prove request bytes, decoding, failure classification, and UI
behavior. It cannot prove that Nyx implements the corresponding server capability.

---

## 6. Skill selection and activation

The canonical worksheet is:

`docs/versions/V1/FORGEOS_V1_FIRST_ARMOR_SKILL_TREE.md`

A skill may be selected only when:

- every direct prerequisite is `CLOSED`;
- the skill is not `INVALIDATED`;
- no active skill owns the same public contract;
- no active skill writes the same source paths;
- no active skill depends on it;
- it does not depend on another active skill;
- its owning subsystem is stable enough for the declared slice;
- the real path or capability probe identifies a concrete reason to activate it;
- the router records its lane, owner, paths, pass edge, block edge, and return
  path.

An unlocked skill is not automatically startable merely because its prerequisites
are closed. It must also be non-competing and safe to execute against the current
source.

The default selection policy is:

```text
lowest valid prerequisite tier
  -> smallest real capability gap
  -> no active-contract conflict
  -> no active-path overlap
  -> shortest route back to a user-visible V1 behavior
```

Manual user selection is allowed only when the selected skill meets the same
prerequisite and conflict rules.

If the user requests a locked skill, report its unresolved direct prerequisites.
Do not silently implement them inside the requested skill.

---

## 7. Active-skill and parallel-work law

At most three skills may be `ACTIVE` at one time.

```text
GLOBAL_ACTIVE_SKILL_LIMIT=3
ACTIVE_SKILL_LIMIT_PER_LANE=1
```

Parallel work is legal only when all of the following are true:

- the skills have no direct or transitive dependency on each other;
- they do not alter the same public contract;
- they do not write overlapping files or directories;
- they do not share one unstable prerequisite;
- they do not require incompatible migrations;
- each uses its own Git branch or worktree;
- each has an independent first blocker and acceptance path;
- merging one cannot invalidate the assumptions of the other.

Parallel skills must declare:

```yaml
skill_id:
lane:
owning_subsystem:
source_repository:
worktree_or_branch:
allowed_paths:
forbidden_paths:
public_contracts_touched:
direct_prerequisites:
first_blocker:
pass_edge:
block_edge:
required_commands:
regression_commands:
return_path:
```

If two active skills begin to compete, both stop. The router must either:

- close or merge one first;
- extract and activate a common prerequisite;
- narrow the write boundaries;
- park one skill.

No model may resolve a collision by letting both continue and hoping Git sorts it
out later.

---

## 8. First-blocker execution loop

Every source skill follows this loop:

```text
select one router-approved AVAILABLE skill
  -> execute or inspect its real path
  -> identify the earliest failed causal edge
  -> record exactly one first blocker
  -> define one narrow implementation slice
  -> edit only allowed paths
  -> run focused tests and required guards
  -> rerun the originating path
  -> close, return a new blocker, or classify the result honestly
```

The experiment or real product path is the scout. The first blocker chooses the
slice.

Do not pre-plan a long chain of implementation slices before running the real
path. Later slices remain hypothetical until the current blocker moves.

A blocked skill must record:

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

Exactly one blocker is primary. Secondary observations may be noted, but they do
not become parallel implementation permission.

---

## 9. Slice write-boundary law

Every active slice must declare:

```yaml
slice_id:
skill_id:
blocker_id:
owning_subsystem:
source_repository:
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

A source edit outside `allowed_paths` is forbidden unless the slice is stopped and
rerouted first.

A slice may expand only when the real blocker proves that another path or contract
is a technical prerequisite. That expansion must be recorded before editing.

A slice may not absorb:

- unrelated cleanup;
- adjacent TODOs;
- speculative abstractions;
- future-version hooks;
- broad dependency upgrades;
- cosmetic restructuring;
- a second capability that merely happens to be nearby.

---

## 10. Completion and user-acceptance law

A skill is not complete because code exists or automated proof exists.

A V1 skill may become `CLOSED` only when:

- every direct prerequisite remains `CLOSED`;
- the real source path implements the complete bounded behavior;
- focused automated tests pass;
- required negative and failure paths pass;
- required guards pass;
- all prior closed-skill regression locks pass;
- the behavior is available through the actual V1 user or operator path;
- the user exercises it;
- the user explicitly approves it within the current version's scope;
- no required criterion is deferred;
- both closure documents exist;
- the worksheet transition is recorded atomically with closure.

Proof receipts support closure. They do not replace user-facing functionality or
user approval.

There is no closure state for:

```text
partially complete
mostly complete
deferred but acceptable
temporary implementation
proof-only complete
backend complete but unusable
UI complete but disconnected
```

If any required behavior is missing, the skill remains `ACTIVE` or `BLOCKED`.

### Required closure documents

Every closed skill receives exactly two source documents:

```text
CLOSURE_AND_SPEC.md
USER_GUIDE_SOURCE.md
```

`CLOSURE_AND_SPEC.md` records the final contract, source path, tests, negative
proof, regression locks, user acceptance result, and known in-scope boundaries.

`USER_GUIDE_SOURCE.md` records every user-facing instruction, option, behavior,
variation, limitation, customization, and troubleshooting fact needed for the
onboard Forge Guide and website documentation.

The second document need not be publication-polished. It must be content-complete
so later work is wording and presentation only.

Documentation does not earn closure. These files record closure already earned by
working behavior.

---

## 11. Regression and anti-shortcut law

No new skill may regress, bypass, replace, weaken, or silently invalidate a
previously closed capability.

A skill may not close by:

- deleting a failing test;
- weakening an assertion;
- increasing a tolerance to hide failure;
- skipping, ignoring, quarantining, or feature-gating away a required test;
- removing a fixture or negative control;
- rewriting a golden solely to match broken output;
- adding an allowlist merely to silence a guard;
- changing a verifier threshold to pass the new source;
- changing an error into success;
- bypassing the public path through a helper;
- substituting model output for tool evidence;
- creating a second implementation;
- deferring a required criterion to a later skill;
- moving required behavior into documentation.

Changing an existing behavioral lock requires an explicit migration skill,
versioned contract, regression plan, and user approval.

When a new change disproves a closed skill, mark that skill `INVALIDATED`, block
its dependents, and return to the first failed proof edge. Do not leave it closed
because it used to work.

---

## 12. Module-size and source-organization law

Authored Rust modules should normally remain near 500 physical lines when that is
a natural behavioral boundary.

```text
PREFERRED_SIZE=ABOUT_500_LINES
CLOSURE_MAXIMUM=1000_LINES
TEMPORARY_HARD_CEILING=1200_LINES
```

The source-size verifier is one of the few allowed non-behavior verifiers.

Required verifier semantics:

```text
0-1000 lines     PASS
1001-1200 lines  WARNING_AND_SKILL_CLOSURE_BLOCKED
1201+ lines      HARD_FAIL
```

The 1200-line ceiling is breathing room during an active split. It is not a target
and not permission to exceed 1000 lines at closure.

The guard must exclude generated, vendored, cache, target, and third-party source.
It must include authored Rust source and authored source test modules.

No file-shard gaming is allowed.

Forbidden examples:

```text
parser_part_1.rs
parser_part_2.rs
parser_part_3.rs
```

when the files do not represent real behavioral or authority boundaries.

Also forbidden:

- compressing multiple statements onto fewer physical lines to cheat the count;
- catch-all production modules such as `utils.rs`, `helpers.rs`, `misc.rs`,
  `stuff.rs`, or unbounded `common.rs`;
- dumping unrelated behavior into a module merely because it is below the limit.

Modules must be tightly scoped, accurately named, nested under meaningful
submodules, and routed through the nearest `mod.rs` and crate `lib.rs`.

---

## 13. Allowed and forbidden guards

Allowed non-behavior guards are limited to:

1. authored Rust module-size guard;
2. Forge Core purity guard;
3. cross-subsystem seam and dependency-direction guards;
4. skill graph and router integrity guard.

The graph and router guard may validate structured state such as:

- unique skill IDs;
- resolved prerequisites;
- no dependency cycles;
- no more than three active skills;
- one active skill per lane;
- no active skill depends on another active skill;
- no active path overlap;
- no active public-contract conflict;
- all active prerequisites are closed;
- every active skill has one blocker and one slice;
- every closed skill has both closure documents;
- invalidated skills block dependents;
- the router names an exact baton owner and repository.

It may not parse prose and decide that behavior is implemented.

Forbidden:

- documentation CI;
- Markdown linters used as completion gates;
- prose-status verifiers;
- checklist parsers;
- validators that award closure from document contents;
- validators that freeze wording as product behavior;
- duplicate guards that enforce the same rule through another mechanism.

Behavior is proved by real execution, tests, integration, and user acceptance.

The canonical ForgeOS CI entrypoint is `python3 scripts/run_ci.py`. Its configuration
may run only behavioral tests, golden locks, and structural guards. Documentation,
Git state, formatting, Markdown, and prose-status validation are permanently outside
CI and cannot become closure gates.

---

## 14. Test and proof law

Tests and product experiments have different jobs.

Tests lock:

- local mechanics;
- validation;
- invariants;
- stable errors;
- ordering;
- persistence;
- protocol compatibility;
- regression behavior.

Capability experiments and user journeys prove:

- the real registered product path uses the behavior;
- required systems integrate;
- failures remain visible;
- user-facing behavior exists;
- the declared result is usable;
- no hidden fallback supplies the result;
- the skill satisfies its V1 claim and no more.

A unit test is not automatically a user-facing capability proof.
A screenshot is not automatically integration proof.
A passing agent patch is not automatically safe.
A proof document is not automatically functionality.

Never claim that a command, build, test, install, boot, recovery path, experiment,
or stranger test passed unless it actually ran.

### 14.1 Assistant toolchain absence and operator validation

The assistant sandbox is not required to contain Rust or the host services needed
by ForgeOS. Missing `rustc`, `cargo`, `rustfmt`, `rustup`, display-manager access,
GPU/session access, or equivalent tooling in the assistant environment does not
block a bounded source slice.

When required validation cannot run in the assistant environment:

```text
source implementation remains authorized
skill state remains ACTIVE
validation state becomes OPERATOR_VALIDATION_PENDING
assistant prepares and apply-checks the patch
assistant reports available checks actually run
assistant hands off exact unavailable commands
user runs them on the canonical development host
```

The assistant must not claim those commands passed before the user reports their
results. The user report must be recorded as operator-executed validation.

A user-reported failure replaces the current blocker with the first actual source,
toolchain, guard, or behavior defect exposed by that command. A user-reported green
chain permits the same skill to advance toward acceptance and closure.

Do not stop source work solely to ask the user to install Rust in the assistant
sandbox, expose a hidden toolchain, regenerate the tar, or begin another document
cycle.

---

## 15. Patch and validation law

Before delivering a source patch:

```text
inspect the newest supplied source
run focused validation
run required negative proof
run prior regression locks affected by the slice
run module-size guard
run core-purity and seam guards when applicable
run git diff --check or equivalent
create one apply-ready patch for one repository
apply-check it against a fresh extraction of the same supplied archive
independently apply and compare when practical
report exact commands and actual results
```

A patch may be delivered without generating a new tar.

The user may apply the patch, run the commands, and later supply any clean fresh
tar they choose. The workflow does not require the assistant to record or predict
that archive's next name or number.

---

## 16. Fresh-session execution sequence

For each new ForgeOS thread:

```text
1. read docs/ForgeOS_header.md
2. read this document
3. identify the current program mode and baton owner
4. use only the newest clean archive supplied for the baton repository
5. remove or ignore all older archive extractions
6. extract the newest archive into an empty directory
7. verify that the repository is coherent, clean, and matches the user's claim
8. inspect current source before relying on prior plans
9. read only the authority and skill documents required by the active mode
10. if DOCUMENT_MIGRATION, create only the declared next authority document
11. if source work is active, read the router and selected skill
12. verify prerequisites, write boundaries, conflicts, and repository ownership
13. execute the real path or capability probe
14. restate the one question, first blocker, active skill, allowed paths, pass edge,
    block edge, and return path
15. edit only after those facts are resolved
```

Do not:

- demand a new tar because an old filename appears in a document;
- compare base numbers as a workflow gate;
- ask for repackaging when the supplied clean source is usable;
- scan every old plan or checklist to choose work;
- create a new worksheet for a blocker;
- reactivate completed documentation migration after source work begins;
- start a source slice from a concept note.

---

## 17. Router handoff contract

The future V1 router must record, at minimum:

```yaml
router_schema: FORGEOS_V1_EXECUTION_ROUTER_V1
program_mode:
active_release_target:
baton_owner:
active_repository:
active_question:
active_skills: []
parked_skills: []
first_blocker:
active_slice:
allowed_paths: []
forbidden_paths: []
required_commands: []
regression_commands: []
pass_edge:
block_edge:
return_path:
next_document_or_action:
```

It must enforce the active-skill and conflict laws in this document.

The router records current execution state. It may mention the supplied archive
for operator orientation, but archive filename, sequence number, and hash are not
routing gates.

---

## 18. Migration exit and current action

The bounded documentation migration is complete because:

- every planned authority document exists;
- every header path resolves;
- the V1 skill graph is structurally valid;
- the router has selected the first legal V1 skill;
- the active skill has no prerequisite or concurrency conflict;
- this authority, the router, the worksheet, and the header agree on source mode.

Current authorized action:

```text
verify the newest clean ForgeOS source
  -> inspect or attempt the initial workspace build path
  -> confirm the declared missing-workspace blocker
  -> implement only FORGEOS-V1-ARCH-000-SLICE-001
  -> run the registered commands
  -> present the workspace boundaries for user acceptance
  -> close only FORGEOS-V1-ARCH-000 if every closure edge passes
```

`DOCUMENT_MIGRATION` may not be reactivated merely because source work becomes
difficult or another planning document would be convenient.

---

## 19. Completion language

Use these meanings exactly:

```text
slice passed
  the bounded source change moved the originating real path past its declared
  blocker and all required mechanical checks pass

skill source proved
  the owning subsystem proves the complete bounded behavior through its real
  source path

skill user acceptance ready
  the behavior is exposed through the real V1 product path and is ready for the
  user to exercise

skill closed
  source, tests, negative paths, regression, user-facing behavior, user approval,
  and both closure documents are complete

skill invalidated
  later evidence or source change disproved a previously closed behavior

V1 release earned
  every required skill is closed and the activated V1 closure experiment passes
```

One definition never implies the next.

---

## 20. Final operating law

```text
The skill tree defines what must become true.
The router selects what may be worked.
The real product path exposes the first blocker.
The slice changes the minimum necessary source.
Tests lock mechanics.
The user approves functionality.
The closure records preserve what was earned.
The release experiment earns V1.
```

No shortcut, document, agent claim, archive number, or visual effect may replace
that chain.
