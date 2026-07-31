# ForgeOS fresh-session forge header

Status: `ACTIVE`
Purpose: copy-paste or read this file first in every new ForgeOS build,
capability, integration, analysis, or documentation thread.
Mission authority: `PENDING_CREATION: docs/MISSION_FORGEOS.md`
Live execution authority: `docs/workflow/WORKFLOW_AUTHORITY.md`
Governing laws: `PENDING_CREATION: docs/GOVERNING_LAWS.md`
Skill-tree method: `PENDING_CREATION: docs/workflow/SKILL_TREE_WORKFLOW_METHOD.md`
V1 skill tree: `docs/versions/V1/FORGEOS_V1_FIRST_ARMOR_SKILL_TREE.md`
V1 execution router: `PENDING_CREATION: docs/versions/V1/V1_EXECUTION_ROUTER.md`
V1 closure experiment: `PENDING_CREATION: docs/versions/V1/V1_CLOSURE_EXPERIMENT.md`
Active release target: `FORGEOS_V1_FIRST_ARMOR`
Four-version product contract: `docs/High_Level.md`

---

## 0. Project identity

ForgeOS is a Linux-based developer operating environment in which the whole
system is organized around building software.

The permanent destination is the complete spatial developer operating system:
a game-engine-powered command environment, a serious coding workstation, a
proof-driven project world, and a bounded local AI companion hosted by
`nyx_server`.

The active release destination is **ForgeOS V1: First Armor**, the raw but real
bootable development environment that must work before later visual,
autonomous, or cinematic versions are activated:

```text
Linux host and dedicated ForgeOS session
  -> Forge Core project and workspace truth
  -> repository, file, terminal, Git, build, and test control
  -> Rust editing and language intelligence
  -> nyx_server local-model assistance
  -> bounded OpenAI heavyweight-agent handoff
  -> reviewable patch, command, and audit trail
  -> one real ForgeOS feature completed inside ForgeOS
  -> V1 First Armor earned
  -> persistent project worlds and capability missions
  -> supervised multi-agent foundry
  -> full spatial developer operating system
```

ForgeOS must become a real development environment, not a game-shaped mockup.
Decorative terminals, fake build lights, scripted agent dialogue, screenshots
standing in for execution, HUD-owned project state, and visual skill nodes that
do not resolve to real capabilities are forbidden substitutes.

The permanent product law is:

> The armor must work before it glows.

---

## 1. Authority order

When documents, source, tools, and visual surfaces disagree, use this order:

```text
1. current source behavior and real tool outputs
2. canonical Forge Core state and recorded artifacts
3. docs/ForgeOS_header.md
4. docs/workflow/WORKFLOW_AUTHORITY.md section 2
5. docs/GOVERNING_LAWS.md when created
6. docs/versions/V1/V1_EXECUTION_ROUTER.md when created
7. docs/versions/V1/FORGEOS_V1_FIRST_ARMOR_SKILL_TREE.md
8. docs/versions/V1/V1_CLOSURE_EXPERIMENT.md when activated
9. docs/MISSION_FORGEOS.md when created
10. docs/High_Level.md
11. the one selected independent capability node
12. subsystem contracts, tests, experiments, receipts, and operator proof
13. historical plans, concept notes, prototypes, and legacy archives
```

Only `WORKFLOW_AUTHORITY.md` and the live V1 router select active work. The
four-version plan defines what each version must become and what later-version
features must not leak into V1. It does not select the next source slice.

The V1 skill tree defines dependencies and proof requirements. It may identify
which nodes are `LOCKED`, `AVAILABLE`, `ACTIVE`, `BLOCKED`, `BANKED`, or
`RELEASE_EARNED`. It may not silently promote the next node merely because a
neighboring node closed.

A roadmap number, unchecked box, version adjacency, tree tier, document order,
engine number, UI screen order, TODO location, prototype label, or visually
nearby skill node has no scheduling authority.

When a document claims behavior that the current real path does not perform,
the behavior remains unproved and the document must be reconciled.

---

## 2. Permanent ownership laws

```text
FORGE CORE / RUST
  owns canonical project identity, repository registration, workspace state,
  release targets, capability graphs, skill states, missions, experiments,
  blockers, slices, proof receipts, invalidation, project settings, and the
  durable state from which ForgeOS claims are made.

NYX_SERVER
  owns model hosting, model discovery, AI sessions, conversations, context
  assembly, tool routing, agent permissions, checkpoints, run records, tool
  audit, remote OpenAI requests, and bounded agent execution. Nyx may inspect,
  propose, explain, and operate within granted authority. It may not invent,
  rewrite, bless, or silently advance Forge Core project truth.

FORGE WORLD / BEVY
  owns spatial presentation, project environments, HUD panels, interaction,
  skill-tree visualization, animation, audio, native ForgeOS layout, and user
  witness surfaces. It may display and control source-owned state. It may not
  fabricate build status, Git state, capability proof, agent results, project
  history, or release readiness.

DEVELOPER BRIDGE
  owns explicit adapters between Forge Core and real development tools,
  including files, PTYs, Git, LSP, DAP, compilers, formatters, test runners,
  debuggers, package managers, and external applications. It reports tool-owned
  results without silently translating failure into success.

REAL DEVELOPMENT TOOLS
  own compilation, language semantics, formatting, source-control behavior,
  debugging behavior, test execution, package resolution, and their native
  diagnostics. ForgeOS orchestrates these tools. It does not pretend to replace
  them with visual simulations or model prose.

LINUX HOST AND SESSION
  own the kernel, hardware drivers, filesystems, networking, audio, process
  isolation, login management, and host services. ForgeOS may provide a
  dedicated session and later a compositor or distribution. It does not rewrite
  the kernel, compiler toolchains, package manager, or driver stack.

PYTHON AND SCRIPTS
  may provide repository utilities, fixtures, orchestration, derived analysis,
  packaging helpers, and development support. They may not become a hidden
  second implementation of Forge Core, Nyx, an official capability experiment,
  or a product path claimed as native ForgeOS behavior.
```

All seams are explicit contracts. No repository, crate, process, visual layer,
or model may quietly absorb another subsystem's authority.

Forge World is a witness and control surface. Nyx is an operator and model host.
Forge Core is project-state authority. Real tools remain authority for their own
execution results.

---

## 3. Canonical state, repeatability, and evidence laws

Canonical project state and proof artifacts must never derive identity from:

```text
wall-clock time
hidden randomness
unstable map ordering
filesystem discovery order
thread or worker arrival order
renderer timing
window-layout timing
model wording
locale
host-specific path formatting
environment-specific diagnostic formatting
```

Timestamps may exist as descriptive metadata. They may not replace stable IDs or
become the sole identity of a capability, mission, experiment, repository,
artifact, or receipt.

Canonical JSON, manifests, project records, and proof receipts require stable
ordering, versioned schemas, and SHA-256 identity where applicable.

External AI output is inherently variable. Therefore:

```text
model output is a proposal
recorded tool execution is evidence
Forge Core state is canonical project truth
human or declared release authority awards release credit
```

An agent saying that code is correct is not proof. A patch existing is not proof.
A passing helper is not proof of the public ForgeOS path. A rendered green status
is not proof unless it resolves to the recorded command, revision, and result it
claims to display.

Public commands and official ForgeOS surfaces must exercise real registered
behavior. Fixtures, demos, mocks, screenshots, sample projects, and derived
visualizations may support development. They may not masquerade as the real V1
execution path.

---

## 4. Product-capability operating loop

Work begins from one of two bounded question forms.

### 4.1 Capability quest

```text
ask: Can the current ForgeOS release do this one required thing honestly?
  -> identify the exact public product path
  -> verify direct prerequisites and source authority
  -> execute or inspect the current real path
  -> capture commands, outputs, revisions, and artifacts
  -> classify PASS, VALID_NEGATIVE, BLOCKED, INVALID, or INCONCLUSIVE
  -> identify exactly one first causal blocker
  -> select one independent AVAILABLE capability node
  -> cut the smallest coherent implementation slice
  -> run focused mechanical tests
  -> rerun the originating capability experiment or product path
  -> obtain the required Forge World witness when presentation is part of the claim
  -> record proof or the next blocker
  -> close only the bounded capability actually earned
```

### 4.2 Integrated developer-journey check

```text
ask: Can a developer complete this bounded workflow without leaving ForgeOS?
  -> begin from the declared clean or known source state
  -> use the real ForgeOS session and registered developer tools
  -> exercise Forge Core, Developer Bridge, Nyx, and Forge World as required
  -> identify the first source, integration, usability, packaging, recovery,
     permission, or release blocker
  -> select one independent capability node
  -> implement and prove that node
  -> repeat the exact same journey step
  -> close only that step or report the next blocker
```

At most one central routing question, one primary blocker, one active capability,
and one implementation slice are active per work lane.

The V1 launch gate must never become a second flat roadmap. Every new question
must name the exact V1 capability gap it closes or remain parked.

Mechanical closure does not imply integrated closure. Integrated closure does not
imply daily usability. Daily usability does not imply release closure. A valid
local model response does not prove safe tool operation. A real command result can
still be shown incorrectly by Forge World. A passing agent patch can still bypass
the authoritative product path.

---

## 5. Permanent product trajectory

The long-term product focus is a truthful developer operating environment that
combines project cognition, bounded agent assistance, and spatial orientation.

### Capability truth and agent supervision

```text
Can ForgeOS maintain an exact, durable account of what a software project can
actually do while human and AI contributors change the source?
```

The product must distinguish task completion from capability proof, agent claim
from tool evidence, local proof from integration, banked work from release credit,
and current truth from invalidated historical receipts.

### Spatial project embodiment

```text
Can a game-engine-powered environment make architecture, dependencies, active
work, failures, agents, and release state easier to understand without making
coding slower than a conventional developer workspace?
```

The product must distinguish useful spatial memory from decorative spectacle,
project state from animation, immediate keyboard access from forced navigation,
and dense 2D work surfaces from 3D structural views.

### Combined product hypothesis

```text
A persistent source-grounded project world, paired with a bounded local AI
operator and optional heavyweight agents, may let developers build ambitious
software with greater continuity, orientation, and confidence than disconnected
IDEs, terminals, chats, roadmaps, and agent sessions.
```

These are permanent product compasses. They do not select the next capability or
activate later-version work.

---

## 6. Current V1 gap-closing campaign

The initial ForgeOS planning and authority campaign is:

```text
PROGRAM_MODE=DOCUMENT_MIGRATION
ACTIVE_QUESTION_CLASS=V1_PLANNING_AUTHORITY
ACTIVE_RELEASE_TARGET=FORGEOS_V1_FIRST_ARMOR
ACTIVE_RELEASE_GATE=BOUNDED_AUTHORITY_SET_AND_FIRST_ROUTER_ACTIVATION
ACTIVE_V1_CONTRIBUTION=WORKFLOW_AUTHORITY_AND_HEADER_RECONCILIATION
ACTIVE_CAPABILITY_ID=NONE_DOCUMENT_MIGRATION_ACTIVE
QUESTION=Can ForgeOS finish the bounded authority set without creating a documentation treadmill, then activate the first legal V1 source skill against the newest clean user-supplied archive?
CURRENT_RESULT=WORKFLOW_AUTHORITY_CREATED_AND_HEADER_PATHS_RECONCILED
BATON_OWNER=FORGEOS_DOCUMENTATION
ACTIVE_SLICE=FORGEOS-WORKFLOW-AUTHORITY-001
CANONICAL_FORGEOS_SOURCE=NEWEST_USER_SUPPLIED_CLEAN_FORGEOS_ARCHIVE
CANONICAL_NYX_SOURCE=NEWEST_USER_SUPPLIED_CLEAN_NYX_ARCHIVE
OLDER_ARCHIVE_POLICY=SUPERSEDED_IGNORE_OR_DELETE
ARCHIVE_FILENAME_AND_HASH_POLICY=ORIENTATION_ONLY_NOT_A_WORKFLOW_GATE
ACTIVE_NYX_ROLE=AI_MODEL_HOST_AND_BOUNDED_AGENT_RUNTIME
NYX_SOURCE_IN_FORGEOS_ARCHIVE=NO_UNLESS_EXPLICITLY_VENDOR_LOCKED
REPOSITORY_SWITCH_NOTICE=REQUIRED
PATCH_ARTIFACT_SCOPE=ONE_REPOSITORY_ONLY
FRESH_CHAT_INPUT=HEADER_PLUS_NEWEST_SUPPLIED_ACTIVE_REPOSITORY_ARCHIVE
SOURCE_START_POLICY=VERIFY_FRESH_SOURCE_THEN_EXECUTE_ROUTER_NO_TAR_OR_DOC_DANCE
WRONG_REPOSITORY_POLICY=NOTIFY_AND_STOP_BEFORE_SOURCE_EDITS
NEXT_REQUIRED_DOCUMENT=docs/GOVERNING_LAWS.md
```

The four-version product boundary is already defined by
`docs/High_Level.md`:

```text
V1 earns daily usability.
V2 earns persistent project awareness.
V3 earns supervised autonomy.
V4 earns the full spatial developer operating system.
```

The V1 First Armor skill tree now exists and is the canonical V1 worksheet.
The next required authority document is `docs/GOVERNING_LAWS.md`.

After the bounded authority set is complete, the V1 router selects the first
`AVAILABLE` node. No source node is active until the final header pass changes the
program mode and the router explicitly records the skill, lane, repository, write
boundary, pass edge, block edge, and return path.

The V1 closure journey is:

```text
boot into ForgeOS
  -> open a Rust repository
  -> inspect and edit code
  -> run registered commands
  -> build and test
  -> inspect Git changes
  -> ask Nyx for project-aware help
  -> invoke a bounded heavyweight coding agent
  -> review and apply the returned patch
  -> rerun proof commands
  -> commit the verified result
  -> complete one real ForgeOS or Nyx feature without leaving ForgeOS
```

The exact pass edges, block edges, activation rules, and closure experiment belong
in the V1 execution router and V1 closure experiment after those documents are
created.

---

## 7. Program modes

Read the exact current mode from `docs/workflow/WORKFLOW_AUTHORITY.md`.

### `DOCUMENT_MIGRATION`

Only project identity, authority boundaries, workflow conversion, legacy
quarantine, graph construction, and migration-exit proof are allowed. Do not begin
source implementation from a concept note or unvalidated roadmap.

### `CAPABILITY_PROBE`

Run, inspect, or triage one declared capability experiment or public product path.
Select no source slice until the first causal blocker is supported by real output.

### `SLICE`

Patch only the named capability and its real technical prerequisites. Seal only
what the public path, tests, tool outputs, artifacts, witness, negative proof, and
claim cutline support. Return immediately to the originating experiment or journey.

### `INVESTIGATION`

Freeze ordinary feature work. Execute the declared control, reproduction,
provenance audit, performance probe, architecture audit, recovery test, permission
test, or contradiction analysis.

### `PRODUCT_INTEGRATION`

Integrate already proved capabilities through Forge Core, Developer Bridge, Nyx,
Forge World, session startup, packaging, installation, recovery, or user-journey
surfaces. Repeat the same real path after each slice. Do not create a second source
of truth or activate the next V1 requirement by adjacency.

### `RELEASE_CLOSURE`

Run only the activated release acceptance route, clean-machine installation,
self-hosting build, recovery checks, stranger test, packaging proof, and declared
release guards. Release closure may not invent missing capability proof.

---

## 8. Archive, repository, and patch laws

The newest clean archive explicitly supplied by the user for a repository is the
sole source authority for that repository in the current conversation.

```text
NEWEST_USER_SUPPLIED_CLEAN_ARCHIVE = CANONICAL_SOURCE
ALL_PREVIOUS_ARCHIVES = SUPERSEDED_AND_IGNORED
```

At intake:

```text
use only the newest supplied archive
extract into an empty directory
remove or ignore older extractions
verify that the source is coherent and matches the user's claim
inspect current source before editing
source behavior outranks stale documentation
```

Never:

```text
predict the next archive filename, base number, or SHA-256
require a renamed or incremented archive before continuing
compare archive sequence numbers as a routing gate
stall source work because documentation mentions an older archive
require a new tar after every patch, skill, or green run
```

A SHA-256 may be computed and reported as intake evidence. It is not workflow
authority. Archive names, numbers, and hashes may help operator orientation but
may not block work when the newest supplied source is clean and verified.

ForgeOS and `nyx_server` remain separate source authorities unless an explicit,
versioned vendoring decision changes that boundary.

A ForgeOS archive may contain the Nyx protocol client, service adapter,
integration tests, and declared compatibility contract. It does not automatically
become authority for Nyx runtime source.

A Nyx archive may contain tool contracts and ForgeOS protocol compatibility. It
does not become authority for Forge Core, Forge World, session, editor, or release
state.

One patch artifact belongs to one repository. Cross-repository work requires
separate patches, separate validation, and explicit handoff.

Before crossing a repository boundary, report:

```text
FROM_REPOSITORY
TO_REPOSITORY
WHY
ACTIVE_CAPABILITY
FIRST_BLOCKER
PASS_OR_BLOCK_EDGE
REQUIRED_HANDOFF_INPUTS
PATCH_POLICY=SEPARATE_TARGET_REPOSITORY_PATCH
```

If the target repository source is absent, stop before source edits and request
the newest clean user-supplied archive for that repository.

Before delivery:

```text
run focused validation
run negative proof where applicable
run broader guards required by the seam
run git diff --check or equivalent
create an apply-ready unified patch
run git apply --check against a fresh extraction of the same supplied archive
apply independently when practical
compare the independently applied copy
provide exact commands and actual results
```

Creating a new source tar is optional and controlled by the user. It is not part
of skill closure or patch acceptance.

Never claim a build, test, experiment, install, boot, session, or stranger check
passed unless it actually ran.

### 8.1 Official capability execution lane

Official ForgeOS capability experiments live under `experiments/`, not hidden in
`scripts/`.

```text
experiments/<domain>/<experiment-id>/
  experiment manifest or one declared executable entrypoint
  controlled inputs and fixtures
  declared pass, negative, and invalid edges
  required command sequence
  artifact and result schema
  primary run and repeat run when repeatability is required
```

```text
scripts/
  developer and repository utilities only
  never the hidden implementation of an official ForgeOS capability
```

An experiment may invoke real public ForgeOS, Nyx, Git, compiler, test, LSP, DAP,
PTY, session, or packaging paths. It may not edit state behind those paths and then
claim the public product performed the behavior.

Evidence packages, when required, must include:

```text
exact experiment definition or entrypoint
source repository and revision
base archive identity when applicable
focused command log
primary and repeat results when required
artifacts and SHA-256 manifest
result classification
supported claim
explicit non-claims
missing higher proof
```

### 8.2 Green-patch operator ritual

For a Rust-owned ForgeOS or Nyx patch, the default operator ritual is:

```bash
git apply <repository>.patch \
  && cargo fmt --all \
  && cargo test --workspace
```

Use repository-declared focused checks before the broad workspace test when the
active capability requires them. If the repository defines stronger canonical
commands, those commands take precedence.

If a command reports a defect, repair the defect and rerun the same command chain
until green or until a new first blocker is honestly recorded.

Once green:

```text
commit with a truthful message when the operator chooses
push only when the operator chooses
run the originating official capability experiment or V1 journey step
package a new source archive only when the user chooses
```

Hand off only the inputs required by the independent audit or next repository
owner. No audit or next slice may demand archive renumbering as a substitute for
verifying the supplied source.

---

## 9. Completion language

Use these meanings exactly:

```text
slice sealed
  the selected implementation slice moved the originating capability past its
  declared blocker and its required mechanical checks pass

capability locally proved
  the owning subsystem proves the behavior through its real local path

capability system integrated
  ForgeOS consumes the behavior through the real registered product path

capability user witnessed
  Forge World or another declared surface presents the exact source-owned result
  without inventing, repairing, or silently recomputing truth

capability banked
  all proof required by the capability contract exists, but release acceptance
  has not yet awarded release credit

capability release earned
  the activated release route formally adopts the banked capability

valid negative result
  the declared mechanism was exercised and the expected rejection, no-effect, or
  absent-result edge occurred correctly

subsystem complete
  every declared completion gate for that subsystem is proved

product integration complete
  required proved capabilities are integrated into one usable product journey

V1 sealed
  the V1 First Armor capability graph, self-hosting closure journey, installation,
  recovery, packaging, and clean-machine stranger test pass through real paths

open product question
  broader usability, scalability, generality, market fit, or later-version behavior
  remains unresolved
```

One definition does not imply the next.

A clean build does not make V1 usable. A usable editor does not prove Nyx safety. A
working Nyx tool call does not prove the full agent patch path. A glowing skill node
does not mean the capability is banked. A banked capability does not receive release
credit until the release route adopts it.

---

## 10. Fresh-session boot sequence

For every new ForgeOS build, audit, planning, or integration thread:

```text
1. read this header
2. read docs/workflow/WORKFLOW_AUTHORITY.md
3. identify the current program mode and baton owner
4. use only the newest clean archive supplied for the baton repository
5. delete or ignore every older archive extraction in the active work area
6. extract the newest archive into an empty directory
7. verify that the repository is coherent, clean, and matches the user's claim
8. inspect current source before relying on prior plans
9. during DOCUMENT_MIGRATION, create only the declared next authority document
10. after source activation, read the V1 router and selected skill
11. verify prerequisites, active-skill conflicts, repository ownership, and write
    boundaries
12. if the baton points to nyx_server, require the newest separate clean Nyx archive
    unless an explicit vendored-source contract proves otherwise
13. open the active experiment, public product path, or journey step
14. inspect current source, commands, run records, patches, receipts, and artifacts
15. restate the one V1 gap, active capability, first blocker, owner, allowed paths,
    pass edge, block edge, and return path
16. only then edit source
```

The newest verified archive is canonical regardless of its filename or base
number. Do not demand a new archive, updated header token, matching hash, or
repackaging ritual before working from clean current source.

During the bounded documentation-migration state, this header, the workflow
authority, `docs/High_Level.md`, and the V1 skill tree determine the next document.
They are not authority to begin implementation before the final router/header pass
activates the first source capability.

Do not scan unrelated roadmap sections, future-version features, project-world
concept art, Nyx engine lists, or neighboring skill nodes to choose work.

Do not create a new workflow document for every blocker. Record the blocker on the
active capability and let the same experiment choose the next slice.

When proof is missing, name the missing proof and leave the capability open.

---

## 11. Anti-resurrection law

The following may exist only as historical plans, identity labels, architecture
references, product milestones, or visual navigation aids:

```text
old phase numbers
prototype build order
engine-number order
UI-screen order
crate order
Nyx subsystem order
Tier 0 through Tier 5 adjacency
V1 requirement list order
V1 to V4 release adjacency
concept-art geography
project-world room order
backlog priority inherited from file position
TODO order
commit order
legacy roadmap sequence
```

None may select work, raise proof level, close a capability, or activate a later
version.

V1 may use crude presentation while the real development loop is earned. V2
project worlds, V3 multi-agent autonomy, and V4 compositor theater remain locked
until their explicit release boundaries activate them.

The full spatial developer operating system remains.

The fake ladder is gone.
