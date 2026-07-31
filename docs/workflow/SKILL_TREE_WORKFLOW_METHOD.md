# ForgeOS Skill-Tree Workflow Method

Status: `ACTIVE_METHOD`
Method ID: `FORGEOS-SKILL-TREE-METHOD-V1`
Applies to: ForgeOS capability planning, execution, proof, user acceptance, invalidation, and release closure
Canonical V1 worksheet: `docs/versions/V1/FORGEOS_V1_FIRST_ARMOR_SKILL_TREE.md`
Live execution authority: `docs/workflow/WORKFLOW_AUTHORITY.md`
Permanent laws: `docs/GOVERNING_LAWS.md`
Future live router: `docs/versions/V1/V1_EXECUTION_ROUTER.md`

---

## 0. Purpose

This document defines the reusable skill-tree method used to build ForgeOS.

It converts a large product destination into a dependency graph of falsifiable,
user-relevant capabilities and then advances that graph by proving one bounded
capability at a time through the real product path.

The method exists to prevent:

- phase-plan collapse;
- giant flat backlogs;
- unstable foundations;
- hidden prerequisites;
- parallel implementations of the same truth;
- agents declaring work complete because code exists;
- documentation pretending to be functionality;
- tests that bypass the product path;
- proofs being mistaken for user-facing behavior;
- later skills regressing earlier skills;
- multiple active skills rewriting the same subsystem;
- architecture drifting into oversized, unnamed, or catch-all modules;
- release progress being inferred from line count, effort, or visual polish.

The method is permanent. Individual trees, releases, and skill registrations may
change through explicit versioned edits, but the execution law remains:

```text
skill statement
  -> direct prerequisites
  -> real product path
  -> first causal blocker
  -> one bounded source slice
  -> focused tests and behavioral locks
  -> rerun the same product path
  -> user exercises the complete behavior
  -> user approves it within current release scope
  -> regression shields remain green
  -> closure records are written
  -> skill becomes CLOSED
```

The skill tree is the canonical worksheet.

The workflow authority controls execution.

The router selects active work.

Source and real tool behavior remain truth.

---

## 1. Method authority and non-authority

This document defines:

- how a skill tree is designed;
- how tiers are interpreted;
- how skills are written;
- how prerequisites and unlocks are recorded;
- how skill states change;
- how a blocker chooses the next slice;
- how proof and user acceptance interact;
- how parallel work is isolated;
- how closed skills are protected;
- how invalidation propagates;
- how a release apex is earned.

This document does not:

- select the currently active skill;
- override the workflow authority;
- replace the live router;
- award closure;
- replace source inspection;
- replace tests, experiments, or user acceptance;
- create a second backlog;
- authorize source work merely because a skill is `AVAILABLE`;
- authorize future-version work during the current release;
- require documentation CI or prose verification.

When this method conflicts with current source behavior, source behavior wins and
the method application must be reconciled.

When this method conflicts with `docs/GOVERNING_LAWS.md`, the governing laws win.

When this method conflicts with live skill selection, the current router and
`WORKFLOW_AUTHORITY.md` control execution.

---

## 2. Core idea

ForgeOS is planned as a graph of capabilities, not a sequence of implementation
tasks.

Each skill is one plain, falsifiable statement describing something the real
system or user can do.

Good ForgeOS examples:

```text
A developer can register an existing Rust repository and reopen it after a
ForgeOS restart without losing its project identity.

A developer can run a registered test command, cancel it, and inspect its real
exit status and captured output inside ForgeOS.

Nyx can inspect the active repository through approved read tools without gaining
undeclared write authority.

A heavyweight coding task runs in an isolated Git worktree and returns a reviewable
diff without mutating the user's active worktree.

A developer can review, approve, apply, test, and commit an agent patch without
leaving the ForgeOS session.
```

Bad examples:

```text
Build the editor.

Improve Nyx.

Make the UI polished.

Add Git.

Implement the backend.

Finish session management.
```

A skill is not complete because:

- files were added;
- code compiles;
- an agent says it is complete;
- a helper works;
- a mock works;
- one happy-path test passes;
- a screenshot looks correct;
- a proof receipt exists;
- the user guide was written;
- the source change was large;
- the work consumed significant time;
- the feature is scheduled for cleanup later.

A skill is complete only when the complete bounded behavior exists through the
real V1 path, its negative and failure behavior is locked, the user has exercised
and approved it, all regression shields remain green, and closure is recorded
atomically.

---

## 3. Six-tier model

Every ForgeOS release tree uses six tiers.

```text
TIER_0 = atomic foundations, identities, contracts, and guards
TIER_1 = local mechanisms
TIER_2 = functional systems
TIER_3 = complete user or operator workflows
TIER_4 = integrated release capabilities
TIER_5 = release apex
```

Tiers describe capability scale and dependency depth.

They do not determine work order by themselves.

A lower-numbered skill is not automatically selected before all higher-numbered
skills. A skill becomes startable only when all direct prerequisites are
`CLOSED`, and it becomes active only when the router selects it.

### 3.1 Tier 0: atomic foundations

Tier 0 contains narrow foundations that other skills depend on.

ForgeOS examples:

- canonical project identity;
- versioned Nyx protocol messages;
- typed tool and command result records;
- repository boundary identity;
- stable skill identity;
- pure Core dependency boundaries;
- module-size guard;
- seam guard;
- structured router state.

Tier 0 should contain inexpensive, precise capabilities with small blast radii.

Tier 0 is not permission to create speculative abstraction piles. Every foundation
must be required by a declared higher capability.

### 3.2 Tier 1: local mechanisms

Tier 1 combines atomic foundations into narrow local behavior.

ForgeOS examples:

- open one PTY session;
- read one registered file buffer;
- capture one process exit result;
- inspect Git status;
- establish one Nyx client connection;
- persist one project record;
- apply one approved patch;
- restore one workspace layout.

### 3.3 Tier 2: functional systems

Tier 2 combines mechanisms into coherent subsystems.

ForgeOS examples:

- project registry and persistence;
- embedded terminal subsystem;
- Git inspection and mutation subsystem;
- Rust editing and language-intelligence subsystem;
- Nyx lifecycle and conversation subsystem;
- remote-agent worktree subsystem;
- session startup and recovery subsystem.

### 3.4 Tier 3: complete workflows

Tier 3 represents meaningful workflows a user or operator can complete.

ForgeOS examples:

- register and reopen a repository;
- edit and save Rust source with diagnostics;
- run and cancel project commands;
- stage and commit a change;
- ask Nyx a project-aware question;
- review an agent-generated patch;
- recover the last workspace after restart.

### 3.5 Tier 4: integrated release capabilities

Tier 4 connects multiple functional systems through the real release path.

ForgeOS V1 examples:

- bootable dedicated ForgeOS session;
- complete Rust development workflow;
- bounded Nyx-assisted coding workflow;
- isolated heavyweight-agent patch workflow;
- resilient project and workspace recovery;
- user-visible source and command truth;
- real Git and validation workflow;
- V1 packaging and installation readiness.

### 3.6 Tier 5: release apex

Tier 5 defines the complete release claim.

For ForgeOS V1, the apex is not earned until a developer uses ForgeOS to implement,
review, validate, and commit a real ForgeOS or Nyx feature without leaving ForgeOS
for another IDE.

Only the release closure path may award `RELEASE_EARNED`.

No collection of individually closed lower skills automatically earns the apex.
The integrated closure journey must pass.

---

## 4. Skill statement law

Every skill statement must follow this shape:

```text
A concrete subject performs an observable behavior under declared conditions and
produces a result that can be exercised, disproved, and approved.
```

A strong skill statement:

- names the user, system, or subsystem performing the behavior;
- uses an observable behavior verb;
- names the relevant condition;
- names the user-visible or operator-visible result;
- avoids vague improvement language;
- avoids hidden implementation assumptions;
- is narrow enough to fail for one clear reason;
- is broad enough to matter to the current release;
- cannot be satisfied by a mock, screenshot, helper, or prose claim;
- includes failure behavior where failure is part of the product contract.

A skill statement must not be written as:

- a file list;
- a code task;
- a refactor objective;
- an architectural preference without behavior;
- a documentation deliverable;
- a proof-only objective;
- an unbounded theme such as performance, quality, polish, or reliability;
- a future-version capability hidden inside the current release.

Implementation details belong in the skill contract only when they are themselves
part of the public or permanent architecture contract.

Examples:

```text
GOOD
A developer can cancel a running registered command and ForgeOS reports the real
cancelled process state without showing a successful result.

BAD
Add process cancellation support.
```

```text
GOOD
A closed project reopens after ForgeOS restart with the same project identity,
repository boundary, and last valid workspace state.

BAD
Persist projects.
```

---

## 5. Canonical skill states

Every ForgeOS skill has exactly one state from this set:

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

No alternate state vocabulary may be introduced without an explicit method and
worksheet migration.

There is no:

```text
PARTIAL
MOSTLY_DONE
DEFERRED
TEMPORARY_PASS
DOCUMENTED
IMPLEMENTED
GOOD_ENOUGH
ALMOST_CLOSED
```

### 5.1 `LOCKED`

At least one direct prerequisite is not `CLOSED`.

A locked skill may be inspected for planning, but it may not own source work.

### 5.2 `AVAILABLE`

Every direct prerequisite is `CLOSED` and no current invalidation blocks the path.

`AVAILABLE` means structurally startable.

It does not mean active.

The router must still verify:

- release scope;
- source authority;
- lane availability;
- write-path isolation;
- contract ownership;
- no competition with active skills;
- no hidden prerequisite exposed by current source.

### 5.3 `ACTIVE`

The router has selected the skill, assigned a lane and source repository, declared
one active question, recorded one first blocker or capability probe, and granted
one bounded slice.

An active skill must have:

- one owning subsystem;
- one source repository;
- one lane;
- one active question;
- one first blocker or probe question;
- one bounded slice;
- explicit allowed paths;
- explicit forbidden paths;
- required commands;
- pass edge;
- block edge;
- return path.

### 5.4 `BLOCKED`

The real path has been attempted and exactly one earliest causal blocker is known.

A blocked skill is not failed work. It is a precise statement of what prevents the
skill from advancing.

A blocked skill must not accumulate a list of loosely related problems. Only the
first causal blocker is active. Later defects remain observations until the first
blocker is resolved and the originating path is rerun.

### 5.5 `SOURCE_PROVED`

The required source behavior exists and the focused automated locks pass.

This state does not mean the skill is complete.

The behavior may still lack:

- real ForgeOS integration;
- complete user-facing presentation;
- packaging;
- recovery behavior;
- usability;
- user acceptance.

### 5.6 `USER_ACCEPTANCE_READY`

The complete bounded behavior is accessible through the real current-version
product path and is ready for the user to exercise.

The acceptance instructions, expected behavior, options, negative paths, and known
scope limits must be available.

### 5.7 `CLOSED`

The user exercised the complete behavior and explicitly approved it within the
current version scope.

Closure also requires:

- focused tests green;
- required negative and failure behavior green;
- integration path green;
- all previously closed regression shields green;
- source organization compliant;
- no authored source module above 1000 physical lines;
- required closure records present;
- worksheet state updated atomically.

### 5.8 `INVALIDATED`

A source, contract, dependency, environment, packaging, user-acceptance, or
regression change disproved a previously closed capability.

Invalidation returns the skill to the first failed proof or user-facing edge.

Every dependent skill whose claim relies on the invalidated edge must be reevaluated
and may become `LOCKED` or `INVALIDATED`.

### 5.9 `RELEASE_EARNED`

Reserved for the release apex after the declared closure experiment passes.

Lower skills become `CLOSED`, not `RELEASE_EARNED`.

---

## 6. Canonical skill-node contract

Every registered skill must contain at least the following fields in the canonical
worksheet or structured registry:

```yaml
skill_id: FORGEOS-V1-EXAMPLE-001
schema_version: FORGEOS_SKILL_NODE_V1
tree_id: SKILLTREE-FORGEOS-V1-FIRST-ARMOR-0001
tier: 2

statement: >
  A developer can perform one concrete observable behavior through the real
  ForgeOS V1 path and receive the declared result.

status: LOCKED
owning_subsystem: FORGE_TERMINAL
source_repository: FORGEOS
lane: TERMINAL

release_target: FORGEOS_V1_FIRST_ARMOR

prerequisites:
  - FORGEOS-V1-FOUNDATION-001
  - FORGEOS-V1-COMMAND-002

unlocks:
  - FORGEOS-V1-WORKFLOW-003

required_behavior:
  - exact positive behavior
  - exact negative behavior
  - exact failure behavior

must_be_true:
  - real public path performs the capability
  - authoritative subsystem owns the result
  - user can observe and exercise the behavior

must_not_be_true:
  - mock-only path
  - helper-only path
  - hidden fallback
  - second source of truth
  - deferred required behavior

product_path: >
  The exact current-version user or operator path used to exercise the skill.

proof_question: >
  Can the real system perform the skill statement under the declared conditions?

pass_edge: >
  Exact result that makes the bounded capability true.

negative_edge: >
  Exact valid rejection or no-effect result when the capability should not act.

invalid_edge: >
  Conditions that make an apparent pass unusable as closure evidence.

required_tests:
  - focused mechanical test
  - failure-path test
  - regression lock

required_controls:
  - repeat execution when applicable
  - feature-disabled or denied control when applicable

required_artifacts:
  - command result or product artifact when applicable

user_acceptance:
  required: true
  path: >
    Exact steps the user performs.
  approval_question: >
    Does the behavior work completely and acceptably within the declared V1 scope?

regression_shield: >
  Previously closed behavior that must remain unchanged.

module_constraints:
  preferred_lines: 500
  closure_max_lines: 1000
  hard_guard_lines: 1200

closure_records:
  - CLOSURE_AND_SPEC.md
  - USER_GUIDE_SOURCE.md

active_blocker: null
active_slice: null
invalidation_reason: null
notes: []
```

The exact storage syntax may evolve. The semantic fields may not disappear.

A skill must not be registered without a real behavior, owner, direct prerequisites,
completion edges, user acceptance, and regression shield.

---

## 7. Prerequisite law

Prerequisites express immediate causal dependency.

A skill must list only the direct capabilities that must already be `CLOSED` for
its own behavior to be built and accepted on stable ground.

Do not connect every lower-tier skill to every higher-tier skill.

Do not use prerequisites as loose thematic relationships.

Do not hide an implementation step inside a prerequisite description.

A valid prerequisite answers:

> What already-proven capability would this skill otherwise have to recreate,
> assume, or destabilize?

A skill becomes `AVAILABLE` only when:

- every direct prerequisite is `CLOSED`;
- none of those prerequisites is invalidated;
- every cross-subsystem contract required by the skill is stable;
- the skill does not require an undeclared future-version behavior;
- the current source still contains the assumed authoritative paths.

If source inspection reveals a missing prerequisite:

```text
pause the child skill
  -> register or locate the missing prerequisite
  -> validate graph ownership and dependencies
  -> activate the prerequisite only through the router
  -> close it
  -> reevaluate the child skill
```

The child may not implement the missing prerequisite secretly.

---

## 8. Unlock law

A closed skill may make direct dependents structurally `AVAILABLE`.

It may not automatically activate them.

After closure:

1. Reevaluate only direct dependents.
2. Verify all their prerequisites are still `CLOSED`.
3. Verify no active invalidation blocks them.
4. Mark eligible dependents `AVAILABLE`.
5. Leave activation to the router.

No chain reaction may silently activate an entire branch.

No higher-tier node receives progress credit merely because a prerequisite closed.

---

## 9. Tree construction method

A new release tree is built top-down.

### Step 1: define the release apex

Write one Tier 5 statement describing exactly what must be true for the release to
be earned.

The apex must be:

- user-visible or operationally complete;
- integrated across all required subsystems;
- impossible to fake through one local helper;
- testable through one declared closure journey;
- bounded to the current release.

### Step 2: decompose into Tier 4 capabilities

Ask:

> What integrated product capabilities must already be true before the apex can
> pass?

Each Tier 4 node should own one major release obligation.

### Step 3: decompose into Tier 3 workflows

For each integrated capability, ask:

> What complete user or operator workflows must exist?

### Step 4: decompose into Tier 2 systems

For each workflow, ask:

> What functional systems must exist through real public interfaces?

### Step 5: decompose into Tier 1 mechanisms

For each system, ask:

> What local behavior must work before this system can be integrated?

### Step 6: decompose into Tier 0 foundations

For each mechanism, ask:

> What identities, contracts, guards, and atomic state must already be stable?

### Step 7: assign authority

Every skill receives exactly one owning subsystem and one source repository.

Consumers may use and present the result. They may not invent the missing source
behavior.

### Step 8: add direct prerequisites

Add only immediate causal edges.

### Step 9: define completion edges

Every skill receives:

- positive pass edge;
- valid negative edge;
- invalid edge;
- automated tests;
- real product path;
- user acceptance;
- regression shield;
- closure records.

### Step 10: validate the graph

Before activation, verify:

- all skill IDs are unique;
- every prerequisite resolves;
- no cycles exist;
- every non-root skill has at least one prerequisite unless explicitly justified;
- every Tier 5 path reaches Tier 0 foundations;
- every skill has exactly one owner;
- every cross-subsystem edge has a declared seam;
- no V2, V3, or V4 behavior is required by V1;
- no skill is documentation-only;
- no skill can close without user acceptance;
- no two skills claim the same authoritative behavior.

### Step 11: build the initial frontier

Mark only prerequisite-free, current-release, source-coherent nodes `AVAILABLE`.

Everything else begins `LOCKED`.

### Step 12: recommend, do not auto-activate

The planning pass may recommend a small set of suitable first skills.

The router selects the actual active skill or skills.

---

## 10. Active frontier law

The full tree is not the daily backlog.

The active frontier contains only:

```text
ACTIVE_SKILLS
AVAILABLE_FRONTIER
BLOCKED_SKILLS
RECENTLY_CLOSED
INVALIDATED_SKILLS
```

The router may keep at most three skills `ACTIVE` globally.

Parallel activation is legal only when all active skills:

- have all direct prerequisites `CLOSED`;
- are in separate lanes;
- have separate branches or worktrees;
- have non-overlapping writable paths;
- do not alter the same public contract;
- do not depend on one another;
- do not compete for the same subsystem authority;
- do not require the same unstable foundation;
- can be merged without inventing an ordering dependency.

The preferred default is one active skill.

Two or three active skills are an optimization used only when independence is
explicit and verified.

A model may not activate parallel work merely because multiple agents are
available.

---

## 11. Skill activation contract

Before a skill becomes `ACTIVE`, the router must record:

```yaml
skill_id:
lane:
owning_subsystem:
source_repository:
source_revision_or_current_state:
active_question:
originating_product_path:
first_blocker_or_probe:
allowed_paths:
forbidden_paths:
public_contracts_touched:
required_commands:
negative_commands_or_checks:
regression_commands:
pass_edge:
block_edge:
user_acceptance_path:
return_path:
branch_or_worktree:
```

Activation is invalid when any of the following is true:

- a prerequisite is not `CLOSED`;
- the source repository is unavailable;
- the authoritative implementation path is unknown;
- another active skill owns an overlapping write path;
- another active skill alters the same public contract;
- the skill requires a hidden prerequisite;
- the skill belongs to a later version;
- the proposed slice spans multiple authority boundaries without separate handoffs;
- user acceptance cannot be described;
- completion depends on deferred behavior.

---

## 12. First-blocker workflow

The first blocker chooses the source slice.

The operating loop is:

```text
select one AVAILABLE skill
  -> activate it through the router
  -> run or inspect the real product path
  -> find the earliest failed causal edge
  -> record exactly one first blocker
  -> cut the smallest coherent source slice expected to move past it
  -> run focused tests and required guards
  -> rerun the same originating path
  -> either advance or replace the first blocker
```

Do not plan every future implementation detail before running the product path.

Do not fix later defects while an earlier causal blocker remains active.

Do not switch to a neighboring skill merely because the blocker is difficult.

Branch switching is allowed only when:

- the active skill is blocked by an unavailable external dependency;
- the source repository is unavailable;
- an investigation is required;
- another independent `AVAILABLE` skill can proceed without competing;
- the router records the pause and new selection.

### 12.1 Blocker contract

```yaml
blocker_id: FORGEOS-V1-EXAMPLE-001-BLOCK-001
skill_id: FORGEOS-V1-EXAMPLE-001

statement: >
  The earliest source or product-path condition preventing the skill from
  reaching its pass edge.

owning_subsystem: FORGE_TERMINAL
blocker_class: MISSING_SOURCE_BEHAVIOR

discovered_by:
  product_path:
  command_or_action:
  run_or_observation:

resolution_condition: >
  Exact behavior that must become true before the originating path can move
  beyond this blocker.

status: OPEN
```

Allowed blocker classes include:

```text
MISSING_SOURCE_BEHAVIOR
INVALID_SOURCE_BEHAVIOR
MISSING_CONTRACT
CONTRACT_MISMATCH
MISSING_TOOL_ADAPTER
MISSING_NEGATIVE_BEHAVIOR
MISSING_USER_SURFACE
MISSING_RECOVERY_BEHAVIOR
MISSING_ARTIFACT
REPEATABILITY_FAILURE
PERFORMANCE_BLOCKER
PACKAGING_BLOCKER
PERMISSION_BLOCKER
EXTERNAL_SUBSYSTEM_BLOCKER
REGRESSION
PRODUCT_INVALIDATION
```

A blocker must describe a causal condition, not a vague symptom.

Bad:

```text
The terminal is broken.
```

Good:

```text
The PTY reader exits when the child process emits a split UTF-8 sequence, so
ForgeOS loses the remaining command output and cannot report the real process
result.
```

---

## 13. Slice method

A slice is the smallest coherent source change expected to move the originating
product path past the current blocker.

A slice is not:

- an arbitrary amount of work;
- a sprint;
- a collection of nearby cleanup;
- a refactor opportunity;
- a full skill by default;
- a reason to touch every related subsystem.

### 13.1 Slice contract

```yaml
slice_id: FORGEOS-V1-EXAMPLE-001-SLICE-001
skill_id: FORGEOS-V1-EXAMPLE-001
blocker_id: FORGEOS-V1-EXAMPLE-001-BLOCK-001

owning_subsystem: FORGE_TERMINAL
source_repository: FORGEOS
branch_or_worktree:

goal: >
  Exact bounded source change expected to satisfy the blocker resolution
  condition.

allowed_paths:
  - crates/forge-terminal/src/**
  - crates/forge-terminal/tests/**

forbidden_paths:
  - crates/forge-core/**
  - crates/forge-world/**
  - unrelated refactors

public_contracts_touched:
  - forge_protocol::CommandResult

required_commands:
  - formatter
  - focused tests
  - seam guard when applicable
  - module-size guard

pass_condition: >
  Focused tests pass and the originating product path advances beyond the active
  blocker.

block_condition: >
  Exact result that requires recording a new first blocker or returning to the
  router.

return_path: >
  Rerun the originating product path and update only the current skill state.
```

### 13.2 Write-boundary law

Source edits are limited to `allowed_paths`.

If the slice requires a file outside those paths:

1. Stop before editing it.
2. Determine whether the path belongs to the same skill and authority.
3. Expand the slice only through an explicit router update, or create a separate
   handoff when another subsystem owns the work.
4. Recheck competition with active skills.

No agent or human may quietly cross a write boundary because the change appears
small.

### 13.3 Scope discipline

The slice may include:

- the minimum production behavior;
- focused tests;
- required negative and failure behavior;
- required contract versioning;
- the minimum source organization needed to stay under module limits;
- a seam adjustment owned by the same authority.

The slice may not include:

- unrelated cleanup;
- speculative abstraction;
- future-version support;
- visual polish unrelated to acceptance;
- broad dependency upgrades;
- opportunistic renaming;
- a second implementation path;
- weakening existing tests;
- postponing required behavior.

---

## 14. Source organization and module law

Every skill must preserve the ForgeOS source-organization laws.

```text
preferred authored module size: about 500 physical lines or less
skill closure maximum: 1000 physical lines
hard guard failure: 1201 physical lines or more
```

The 1001–1200 range is temporary breathing room during an active split.

A skill cannot close while any authored module remains above 1000 physical lines.

Modules must:

- own one coherent responsibility;
- be named for that exact responsibility;
- live under the owning subsystem;
- route public APIs through the nearest `mod.rs` and crate `lib.rs`;
- keep internal implementation private;
- split along behavior or authority boundaries;
- avoid catch-all production modules.

The line-count law may not be gamed by:

- numbered file shards;
- arbitrary `part_a` and `part_b` files;
- multiple statements compressed onto one line;
- moving production code into tests or scripts;
- hiding implementation in generated files;
- excluding authored files from the verifier without explicit law change.

When a skill would push a module above the preferred range, split it during the
same skill when a coherent boundary exists.

When no coherent split exists yet, the module may temporarily enter the warning
range while the active slice is completed, but closure remains blocked above 1000
lines.

---

## 15. Proof, tests, product behavior, and user acceptance

ForgeOS separates four different obligations.

### 15.1 Mechanical tests

Tests prove:

- local invariants;
- parsing;
- serialization;
- state transitions;
- error classification;
- boundary validation;
- deterministic ordering;
- contract compatibility;
- focused failure behavior;
- regression locks.

A unit test is not automatically product proof.

### 15.2 Product-path execution

The real product path proves:

- the registered public route uses the behavior;
- real tools are invoked;
- authoritative state is consumed;
- integrations are connected;
- failure is visible rather than hidden;
- the user-facing surface reflects real state.

### 15.3 Proof records

Proof records preserve:

- source revision or current source identity;
- commands and exit codes;
- observed product path;
- inputs and outputs;
- artifacts and hashes where applicable;
- controls;
- supported claims;
- explicit non-claims;
- regression results.

Proof records make closure auditable.

They do not make the feature user-facing.

### 15.4 User acceptance

The user must exercise and approve every skill before `CLOSED`.

User acceptance verifies:

- the behavior is reachable;
- the complete bounded workflow functions;
- instructions are sufficient;
- controls and options behave as declared;
- failure behavior is understandable;
- the result is usable within the current release scope;
- no required criterion was hidden behind proof or documentation.

A skill may remain `USER_ACCEPTANCE_READY` indefinitely if the user has not yet
approved it.

It may not become `CLOSED` automatically.

---

## 16. Pass, negative, invalid, and block edges

Every skill must declare four distinct outcome edges.

### 16.1 Pass edge

The exact observable result that makes the bounded skill statement true.

### 16.2 Negative edge

The exact valid rejection, denial, no-effect, empty result, or cancellation when
the behavior should not proceed.

ForgeOS examples:

- an unregistered command is rejected;
- a write request without permission is denied;
- an empty Git staging set produces a stable no-change result;
- cancelling a process does not report success;
- an unavailable local model remains unavailable without fake fallback;
- a patch that fails validation is not applied.

### 16.3 Invalid edge

Conditions that make an apparent pass unusable as closure evidence.

Common invalidators:

- mock-only execution;
- fixture-only execution;
- hidden fallback;
- bypassing the public path;
- stale cached state shown as current;
- Forge World inventing backend truth;
- Nyx claiming a result without tool evidence;
- manual source or database edits;
- test weakening;
- skipped regression commands;
- agent work performed in the active worktree when isolation was required;
- user acceptance performed on a prototype path not shipped in V1.

### 16.4 Block edge

The exact observation that proves the skill has not reached its pass edge and
identifies the first causal blocker or required handoff.

---

## 17. Regression-shield method

Every skill declares what previously closed behavior it must preserve.

Regression shields may include:

- stable project identity;
- project reopen behavior;
- command result semantics;
- Git state correctness;
- permission denial behavior;
- Nyx session recovery;
- protocol compatibility;
- existing user workflow;
- source module boundaries;
- prior performance envelope;
- packaging behavior;
- crash recovery.

A new skill cannot close by breaking an earlier skill and promising to repair it
later.

When a previously closed skill fails:

```text
stop closure
  -> identify the earliest regressed capability
  -> mark it INVALIDATED when the failure is real
  -> pause dependent active work
  -> route the regression as the active blocker
  -> restore the behavior without weakening its locks
  -> rerun dependent paths
```

Regression repair receives no exemption from user acceptance when user-facing
behavior changed.

---

## 18. Test and guard anti-tampering method

A skill may not advance by:

- deleting a failing test;
- weakening an assertion;
- widening a tolerance without an approved contract change;
- changing expected failure into success;
- skipping a test;
- marking a test ignored;
- changing a golden solely to match broken output;
- suppressing a diagnostic;
- adding an allowlist only to silence a guard;
- excluding a source module from the size verifier;
- changing the 1000 or 1200 thresholds;
- replacing an integration test with a helper test;
- changing the acceptance instructions to avoid the broken path.

A legitimate test or guard change requires:

- an explicit contract or migration reason;
- source evidence that the old expectation is obsolete;
- user approval when user-facing behavior changes;
- replacement locks for the new contract;
- regression review of all dependent closed skills.

---

## 19. Allowed automated guards

ForgeOS permits only these non-behavior guard classes:

```text
1. authored Rust module-size guard
2. Forge Core purity guard
3. cross-subsystem seam and dependency guard
4. structured skill-graph and router-integrity guard
```

These guards validate architecture and recorded routing state.

They do not prove product behavior.

Forbidden guard classes include:

- prose wording checks;
- Markdown completeness checks;
- documentation-link CI used as a release gate;
- checkbox-count progress checks;
- scanners that infer closure from words;
- validators that award skill state from document presence alone;
- source comments used as execution authority.

Closure documents are required records and source material for user documentation.
Their presence is checked as structured closure state, not as prose correctness.

---

## 20. Required closure records

Every closed skill creates exactly two skill-specific records.

Suggested location:

```text
docs/skills/<skill-id>/CLOSURE_AND_SPEC.md
docs/skills/<skill-id>/USER_GUIDE_SOURCE.md
```

The live router may define another canonical location, but the two-record law
remains.

### 20.1 `CLOSURE_AND_SPEC.md`

This record contains:

- skill ID and final statement;
- owning subsystem;
- source repository and revision or source identity;
- direct prerequisites;
- final public path;
- final source behavior;
- negative and failure behavior;
- commands run;
- test results;
- guard results;
- product-path result;
- user acceptance result and date or session reference;
- regression shield result;
- module-size result;
- supported claim;
- explicit non-claims;
- known current-version limits;
- invalidation triggers.

### 20.2 `USER_GUIDE_SOURCE.md`

This record contains all substantive user-facing information required to later
produce the onboard Forge Guide and website documentation.

It includes:

- what the capability does;
- where it appears;
- prerequisites visible to the user;
- exact usage steps;
- available options;
- customization;
- keyboard controls;
- permissions;
- expected results;
- negative and failure behavior;
- recovery steps;
- current-version limits;
- troubleshooting;
- interaction with Nyx when applicable.

The wording does not need final publication polish.

It must contain all required substance so later work is presentation and wording,
not rediscovery.

### 20.3 Closure records are not closure substitutes

Writing both documents does not close the skill.

The real behavior and user approval must already exist.

---

## 21. Atomic closure method

Skill closure is one atomic state transition.

All of the following must be true together:

```text
direct prerequisites CLOSED
+ real source behavior complete
+ focused tests green
+ required negative and failure paths green
+ real product path green
+ user acceptance complete
+ user approval explicit
+ prior CLOSED skills remain green
+ module-size closure law passes
+ pure Core and seam guards pass when applicable
+ closure/spec record complete
+ user-guide source record complete
+ worksheet and router state updated together
= CLOSED
```

Do not:

- mark the worksheet closed and fill evidence later;
- create the guide record later;
- accept a promised cleanup;
- close source behavior while the product surface is missing;
- close product behavior before user acceptance;
- close a parent because most children are closed;
- close a skill because the next skill needs it.

---

## 22. Invalidation method

Closed skills remain subject to invalidation.

Invalidation triggers include:

- source regression;
- public contract change;
- schema migration failure;
- dependency update changing behavior;
- packaging or installation failure;
- permission boundary failure;
- user acceptance no longer matching the product;
- test or proof shown to bypass the real path;
- architecture ownership violation;
- module-size closure violation introduced later;
- later evidence disproving the original claim.

When invalidation occurs:

1. Record the exact failed edge.
2. Set the skill to `INVALIDATED`.
3. Identify direct dependents.
4. Reevaluate whether dependents remain true independently.
5. Lock or invalidate dependents whose claims rely on the failed edge.
6. Stop active work that assumes the invalidated capability.
7. Route the earliest repair blocker.
8. Rerun the original acceptance path after repair.
9. Restore closure only after all closure criteria pass again.

Invalidation is not erased from history.

The final closure record must preserve the original closure and subsequent repair
history or link to versioned replacements.

---

## 23. Parallel work method

ForgeOS allows up to three active skills, but independence must be proven before
parallel work begins.

### 23.1 Required isolation

Each active skill must have:

- its own lane;
- its own Git branch or worktree;
- non-overlapping writable paths;
- non-overlapping public contracts;
- no active dependency on another active skill;
- no shared unstable foundation;
- one clear merge owner;
- one clear return path.

### 23.2 Shared-contract freeze

If one active skill discovers that it must change a contract used by another
active skill:

```text
pause both dependent slices
  -> identify or register the shared prerequisite skill
  -> route the prerequisite alone
  -> close and merge it
  -> refresh both worktrees from the new stable foundation
  -> revalidate their blockers
  -> reactivate only if they remain independent
```

### 23.3 Competition examples

Skills directly compete when they:

- edit the same files;
- alter the same public type or protocol;
- change the same database schema;
- own the same process lifecycle;
- create alternate implementations of one behavior;
- depend on unresolved ordering between their patches;
- change one shared test fixture in incompatible ways;
- both claim authority over the same state.

Competing skills may not remain active simultaneously.

---

## 24. Repository and subsystem handoff method

One source slice belongs to one repository and one owning subsystem.

When progress requires another repository or authority:

```text
FROM_REPOSITORY
TO_REPOSITORY
FROM_SKILL
BLOCKER
WHY_THE_TARGET_OWNS_THE_BEHAVIOR
REQUIRED_TARGET_SOURCE
REQUIRED_HANDOFF_INPUTS
PASS_EDGE
BLOCK_EDGE
PATCH_POLICY=SEPARATE_TARGET_REPOSITORY_PATCH
```

Stop before target-source edits until the target repository is supplied or already
present as the current canonical source.

Do not patch ForgeOS and `nyx_server` in one mixed patch.

Do not implement missing Nyx behavior inside ForgeOS.

Do not implement missing Forge Core behavior inside Forge World.

Do not simulate missing real-tool behavior in a visual layer.

---

## 25. Fresh-source and resume method

The newest clean user-supplied archive for the target repository is canonical.

Older archives and older extracted copies are ignored.

Resume sequence:

```text
extract newest supplied archive into an empty directory
  -> verify repository identity and coherent source state
  -> read the fresh-session header
  -> read workflow authority and live router
  -> inspect the active skill, blocker, and slice
  -> inspect current source before assuming the old blocker remains
  -> rerun the originating path when practical
  -> confirm or replace the first blocker
  -> then edit source
```

Do not blindly apply a previously planned patch to changed source.

Do not require a matching archive number or recorded hash.

Do not require the user to generate a new archive after every patch or closure.

The archive is intake source, not a workflow ceremony.

---

## 26. Pause method

Every active or blocked skill must be safely resumable.

A pause record contains:

```yaml
skill_id:
status:
source_repository:
branch_or_worktree:
source_state:
originating_product_path:
last_command_or_user_action:
first_blocker:
active_slice:
allowed_paths:
forbidden_paths:
last_green_commands:
user_acceptance_state:
next_action:
```

A paused skill does not lose its history.

It also does not remain `ACTIVE` indefinitely when the lane is reassigned. The
router records whether the skill is `BLOCKED`, paused outside the active frontier,
or invalidated.

---

## 27. Skill registration method

New skills may be registered only when a real capability gap is discovered that
is not already represented by the canonical tree.

Before registration:

1. Search existing skill IDs and statements.
2. Inspect current source authority.
3. Determine whether the gap is a blocker inside an existing skill rather than a
   new capability.
4. Determine the owning subsystem and release.
5. Verify the capability is required for the active release.
6. Write the falsifiable statement.
7. Add direct prerequisites and direct unlocks.
8. Define all completion edges and user acceptance.
9. Check for overlap with existing skills.
10. Validate the graph for cycles and unresolved IDs.
11. Update the router frontier without automatically activating the new skill.

Do not create a new skill for:

- every source file;
- every bug;
- every test;
- every refactor;
- every blocker;
- every documentation task;
- optional polish;
- future ideas outside the active release.

A blocker normally selects a slice inside its owning skill.

A new skill is justified only when the missing capability is independently
meaningful, reusable, prerequisite-worthy, and requires its own user acceptance.

---

## 28. Skill modification method

A skill statement, prerequisite, owner, or completion edge may change only when:

- current source disproves the old model;
- the release contract changes explicitly;
- a missing authority boundary is discovered;
- two skills are proven duplicates;
- a skill is too broad to accept honestly;
- a skill must be split along real behavior boundaries;
- an invalid assumption requires migration.

Modification procedure:

```text
record the reason
  -> preserve the old skill identity or supersession history
  -> update direct dependencies
  -> migrate current state conservatively
  -> invalidate affected closure when necessary
  -> validate graph integrity
  -> update router frontier
  -> do not award new closure from the edit itself
```

A skill ID may not be silently reused for a different behavior.

When a skill is split, the original becomes superseded or narrowed explicitly.
Previously earned proof is inherited only where the new statement is fully
supported.

---

## 29. Release closure method

Closed skills make release closure possible.

They do not automatically earn release closure.

The release router activates `RELEASE_CLOSURE` only when:

- every required V1 skill is `CLOSED`;
- no required skill is `INVALIDATED`;
- no required user acceptance remains pending;
- the V1 closure experiment is current;
- packaging and supported-host prerequisites are available;
- the source is coherent and all required guards pass.

The closure experiment must exercise the integrated V1 promise:

```text
install or launch ForgeOS on a supported Linux host
  -> select or enter the ForgeOS session
  -> open a real Rust repository
  -> inspect and edit source
  -> use Rust language intelligence
  -> run real project commands
  -> inspect and change Git state
  -> ask Nyx for project-aware assistance
  -> request bounded heavyweight coding work
  -> review and approve the returned diff
  -> apply the patch
  -> run formatting, build, and tests
  -> commit a real ForgeOS or Nyx feature
  -> restart and confirm durable project state
  -> complete the journey without another IDE
```

Only a successful declared closure journey awards `RELEASE_EARNED` to the apex.

---

## 30. No-credit rules

No skill receives completion credit from:

- dormant code;
- unused helpers;
- compile success alone;
- unit tests alone;
- mock-only success;
- fixture-only success;
- screenshots;
- visual effects;
- documentation edits;
- proof receipts without functionality;
- model explanations;
- generated code not used by the product;
- hidden fallback behavior;
- hand-edited state;
- test weakening;
- skipped user acceptance;
- deferred required behavior;
- line count;
- time spent;
- number of prompts;
- number of commits;
- complexity;
- source churn;
- working outside ForgeOS when the skill claims an in-ForgeOS workflow.

A failed experiment that exposes the first real blocker is valuable routing
information.

It does not earn the skill.

---

## 31. Common failure patterns

### 31.1 Flat backlog resurrection

```text
read every open skill
  -> pick the easiest task
  -> lose dependency order
```

Correction: work only from the router-approved active frontier.

### 31.2 Phase adjacency

```text
finish one branch
  -> start the visually next branch
```

Correction: direct prerequisites and router selection decide activation.

### 31.3 Proof-only closure

```text
tests pass
  -> write receipt
  -> mark CLOSED
```

Correction: complete product path and user acceptance remain mandatory.

### 31.4 Hidden prerequisite implementation

```text
child skill needs missing foundation
  -> implement foundation inside child
```

Correction: route the prerequisite explicitly.

### 31.5 Parallel truth

```text
existing subsystem is difficult
  -> build a second path for the new skill
```

Correction: adopt, prove, extend, or explicitly migrate the authoritative path.

### 31.6 File-size theater

```text
module exceeds limit
  -> split into numbered fragments
```

Correction: split by responsibility and authority.

### 31.7 Documentation dance

```text
source problem is hard
  -> create another worksheet, validator, or policy document
```

Correction: the bounded authority set is complete; return to the source blocker.

### 31.8 Visual substitution

```text
Forge World displays a green build indicator
  -> claim build capability
```

Correction: the display must resolve to the real registered command result.

### 31.9 Agent self-certification

```text
agent says patch is correct
  -> apply and close
```

Correction: inspect, approve, run real validation, and complete user acceptance.

### 31.10 Deferred completeness

```text
happy path works
  -> defer errors and recovery
  -> close skill
```

Correction: required negative, failure, and recovery behavior is part of the same
skill unless the canonical tree explicitly assigns an already-declared prerequisite.

---

## 32. Model execution procedure

When an AI model operates ForgeOS work, it must follow this sequence:

```text
1. read docs/ForgeOS_header.md
2. inspect the newest user-supplied source archive
3. read docs/workflow/WORKFLOW_AUTHORITY.md
4. read docs/GOVERNING_LAWS.md
5. read the live V1 router
6. read only the active skill and its direct prerequisites
7. inspect the current authoritative source path
8. rerun or inspect the originating product path
9. confirm the first blocker
10. edit only the declared slice paths
11. run required focused tests and guards
12. rerun the same originating path
13. report pass or the next first blocker
14. prepare user acceptance only when the full behavior is ready
15. close only after explicit user approval
16. update closure records, worksheet, and router atomically
17. stop before activating the next skill unless the user requests continuation
```

The model must not scan the full tree to choose attractive work when the router
already owns selection.

The model must not ask for a new tar merely because the base number changed.

The model must not create new authority documents after migration closure unless a
real governance contradiction requires an explicit user-approved change.

---

## 33. Compact skill execution law

```text
AVAILABLE does not mean ACTIVE.

ACTIVE means one owner, one lane, one question, one blocker, one slice, and one
return path.

Proof does not replace product behavior.

Product behavior does not replace user acceptance.

User acceptance does not waive regressions, module law, guards, or closure records.

CLOSED means the complete bounded behavior was exercised and approved.

RELEASE_EARNED means the complete integrated release journey passed.
```

---

## 34. ForgeOS V1 method application

For ForgeOS V1 First Armor:

- the canonical tree contains 67 skills;
- the tree is the complete V1 worksheet;
- work begins only from `AVAILABLE` nodes selected by the router;
- the default is one active skill;
- up to three may be active only under strict independence;
- every skill requires user acceptance;
- every closed skill creates two records;
- no authored module may exceed 1000 physical lines at closure;
- Nyx remains the sole AI host;
- Forge Core remains pure;
- Forge World remains presentation and interaction authority;
- real tools remain authority for compilation, Git, language behavior, tests, and
  debugging;
- no V2, V3, or V4 feature may become a hidden V1 prerequisite;
- the apex requires ForgeOS to build a real ForgeOS or Nyx feature from inside
  ForgeOS.

The first active skill will be selected by the completed V1 router after the final
fresh-header authority pass.

This method does not select it.

---

## 35. Final completion law

```text
release destination defined
  -> skill tree decomposed top-down
  -> direct prerequisites validated
  -> one startable skill selected by the router
  -> real path attempted
  -> first causal blocker recorded
  -> one bounded slice implemented
  -> focused tests and guards pass
  -> originating path rerun
  -> complete current-version behavior works
  -> user exercises and approves it
  -> prior closed behavior remains green
  -> module and seam laws pass
  -> closure records are complete
  -> skill becomes CLOSED
  -> direct dependents are reevaluated
  -> integrated closure journey eventually passes
  -> release apex becomes RELEASE_EARNED
```

The tree is the map.

The router chooses the quest.

The blocker chooses the slice.

The source performs the behavior.

The user earns the closure.

The release journey earns the armor.
