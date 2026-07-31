# ForgeOS V1 First Armor Skill Tree

Status: `CANONICAL_V1_WORKSHEET`
Tree ID: `SKILLTREE-FORGEOS-V1-FIRST-ARMOR-0001`
Release target: `FORGEOS_V1_FIRST_ARMOR`
Product authority: `docs/High_Level.md`
Fresh-session authority: `docs/ForgeOS_header.md`
Future execution authority: `docs/workflow/WORKFLOW_AUTHORITY.md`
Future router authority: `docs/versions/V1/V1_EXECUTION_ROUTER.md`

---

## 0. Canonical purpose

This document is the complete canonical capability worksheet for ForgeOS V1.
It defines every capability that must be closed before **First Armor** can be
called complete.

V1 is not a concept demo, fake desktop, cinematic prototype, or game-shaped
mockup. V1 is earned only when a developer can boot into the ForgeOS session,
open a real Rust repository, edit source, use language intelligence, run real
commands, inspect and mutate Git state, work with `nyx_server`, request a
heavyweight remote coding task, review and apply the returned patch, run the
required validation, and commit the verified result without leaving ForgeOS for
another IDE.

The V1 product path is:

```text
install ForgeOS on a supported Linux host
  -> select the ForgeOS session at login
  -> enter the Forge World shell
  -> register or reopen a real Rust repository
  -> inspect and edit source through real files
  -> use Rust language intelligence
  -> run real terminal and registered project commands
  -> inspect, stage, and commit through real Git
  -> ask Nyx for project-aware local assistance
  -> grant bounded tool authority when needed
  -> request heavyweight OpenAI coding work in an isolated worktree
  -> review the exact returned diff
  -> approve and apply the patch
  -> rerun real formatting, build, and test commands
  -> commit a real ForgeOS or Nyx feature
  -> repeat after restart
  -> earn ForgeOS V1 First Armor
```

The skill tree is top-down so the destination remains visible. Daily execution
will still begin only from an `AVAILABLE` node whose direct prerequisites are
closed.

---

## 1. V1 completion law

A skill is not complete because:

- source exists;
- a crate compiles;
- a unit test passes;
- an agent says the work is complete;
- a proof receipt exists;
- a screenshot looks convincing;
- a document says the skill is closed;
- a hidden helper performs the behavior;
- a mock or fixture performs the behavior;
- the behavior works only outside ForgeOS;
- unfinished criteria were deferred to another skill;
- the user has not exercised and approved the behavior.

A V1 skill may enter `CLOSED` only when all of the following are true:

```text
direct prerequisites are CLOSED
  -> the real source path exists
  -> focused mechanical tests pass
  -> the public or operator-facing behavior works
  -> required negative and failure paths work
  -> the user exercises the behavior
  -> the user approves the behavior within V1 scope
  -> all previously CLOSED skills remain green
  -> no authored source module exceeds 1000 physical lines
  -> required closure and user-guide source documents exist
  -> the exact worksheet node is updated to CLOSED
```

Proof supports closure. Proof does not replace functionality or user approval.

There is no `PARTIAL`, `MOSTLY_DONE`, `DEFERRED`, `TEMPORARILY_ACCEPTED`, or
`CLOSE_ENOUGH` state. If a required criterion is missing, the node remains
`ACTIVE` or `BLOCKED`.

---

## 2. Skill states

Every node has exactly one current state:

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

### `LOCKED`

One or more direct prerequisites are not `CLOSED`.

### `AVAILABLE`

Every direct prerequisite is `CLOSED`. The router may select the node.

### `ACTIVE`

The node owns one current implementation slice.

### `BLOCKED`

The real path was attempted and exactly one first causal blocker is recorded.

### `SOURCE_PROVED`

The required source behavior and automated locks pass, but the user has not yet
completed acceptance.

### `USER_ACCEPTANCE_READY`

The behavior is packaged and presented through the real V1 path and is ready for
the user to exercise.

### `CLOSED`

The user exercised and approved the complete bounded behavior, all regression
shields passed, and the two required skill documents were recorded.

### `INVALIDATED`

A later source, contract, tool, packaging, or regression change disproved a
previously closed capability. The node returns to the first failed edge.

### `RELEASE_EARNED`

Reserved for `FORGEOS-V1-APEX-001` after the full V1 closure journey passes.

---

## 2.1 Tree inventory

```text
Tier 5 final release capability:       1 node
Tier 4 integrated V1 capabilities:     8 nodes
Tier 3 complete user workflows:       13 nodes
Tier 2 functional systems:            19 nodes
Tier 1 local mechanisms:              16 nodes
Tier 0 atomic foundations and guards: 10 nodes
                                      --------
Total canonical V1 skills:            67 nodes
```

Every Tier 5 path resolves to the single Tier 0 root
`FORGEOS-V1-ARCH-000`. The dependency graph contains no cycles and no unresolved
prerequisite IDs at the time this worksheet is created. The future router must
revalidate those properties after every registration change.

---

## 3. Source architecture and module laws

These laws apply to every skill in this tree.

### 3.1 Module size law

All authored source modules must remain tightly scoped.

```text
preferred module size: approximately 500 physical lines or less
target maximum: 1000 physical lines
hard temporary ceiling: 1200 physical lines
```

The repository will contain one source-size verifier with these semantics:

```text
0-1000 lines     PASS
1001-1200 lines  BREATHING_ROOM_WARNING
1201+ lines      HARD_FAIL
```

The 1200-line ceiling exists only to allow a module to be split safely during an
active slice. It is not permission to treat 1200 lines as acceptable design.

No skill may close while any authored source module remains above 1000 physical
lines. The same verifier provides the warning used to block closure. A second
document or prose verifier must not be invented.

The verifier covers authored Rust source files, including separate source test
modules. Generated, vendored, target, cache, and third-party source are excluded.

### 3.2 Module scope and naming law

Every module must:

- own one coherent responsibility;
- use a name that states that responsibility;
- live under the subsystem that owns its truth;
- expose its public surface through the nearest `mod.rs` and crate `lib.rs`;
- keep internal implementation private unless another subsystem has a declared
  contract need;
- split by behavior or authority boundary rather than arbitrary file size alone.

Production catch-all modules such as `utils.rs`, `helpers.rs`, `misc.rs`,
`stuff.rs`, or an unbounded `common.rs` are forbidden.

This naming and scope rule is enforced through source review and closure, not a
document checker.

### 3.3 Expected V1 workspace shape

The exact crate names may be versioned during the first architecture skill, but
the authority separation must remain equivalent to:

```text
crates/
  forge-protocol/       shared versioned messages, IDs, errors, and events
  forge-core/           pure canonical project and workspace state
  forge-project/        project registration and persistence adapters
  forge-session/        ForgeOS login-session and service lifecycle
  forge-bridge/         explicit adapters to real development tools
  forge-terminal/       PTY and registered command execution
  forge-git/            Git inspection, mutation, and worktree control
  forge-editor/         file buffers, parsing, and language intelligence
  forge-nyx-client/     nyx_server protocol and lifecycle integration
  forge-world/          Bevy shell, HUD, and user interaction
  forge-app/            composition root and executable
  forge-guards/         source-size, core-purity, and seam guards only
```

`nyx_server` remains its own host and AI authority. ForgeOS integrates through a
versioned client contract. ForgeOS must not create a parallel model host.

### 3.4 Pure core law

`forge-core` may depend only on pure domain and protocol crates.

It must not import or directly call:

- Bevy or renderer APIs;
- PTY or shell libraries;
- Git process adapters;
- filesystem implementation APIs;
- LSP or DAP clients;
- `nyx_server` transport clients;
- OpenAI clients;
- desktop-session or display-manager APIs;
- host package-management APIs.

Effects enter through declared commands and ports. Results return through
versioned domain outcomes and events.

### 3.5 Seam guard law

Automated seam guards may enforce:

- crate dependency direction;
- Forge Core purity;
- Forge World not owning canonical project truth;
- Nyx integration remaining behind the declared client seam;
- real tool adapters remaining outside Forge Core;
- no forbidden cross-crate imports;
- no parallel source-of-truth implementations.

These guards validate architecture. They do not prove user behavior.

### 3.6 No documentation CI law

There will be no documentation CI, documentation prose verifier, checklist
parser, status-word scanner, or test that closes capabilities by reading
Markdown.

The only non-behavior automated guards permitted by this plan are:

- the source module-size verifier;
- declared seam guards;
- the Forge Core purity guard;
- graph integrity checks required by the future skill router.

Documents record decisions, closure, and user guidance. They do not manufacture
product behavior.

---

## 4. Regression and no-shortcut laws

Every node inherits these guards in addition to its node-specific guards.

### 4.1 No regression

A new skill may not weaken, bypass, disable, reinterpret, or silently replace a
previously `CLOSED` skill.

If a new implementation invalidates an earlier behavior, the earlier node becomes
`INVALIDATED` immediately and the newer skill cannot close until the regression
is repaired and both paths are user-approved again where applicable.

### 4.2 No parallel truth

Before adding a new path:

```text
inspect existing source
  -> identify current authority
  -> adopt existing behavior
  -> prove existing behavior
  -> extend through a versioned contract
  -> migrate explicitly when required
  -> remove or reject the replaced path
```

A second project registry, second workspace state model, second Nyx host, second
Git abstraction, second command runner, second editor buffer authority, or
second session lifecycle is forbidden.

### 4.3 No hidden fallback

A skill is invalid if the public path succeeds only because of:

- hardcoded sample data;
- hidden fixture substitution;
- silent fallback to another IDE;
- precomputed Git or build output;
- scripted agent dialogue;
- renderer-owned status;
- a different command than the one shown to the user;
- an unreviewed shell escape;
- an undeclared remote service;
- manual state edits outside the product path.

### 4.4 No deferred criteria

If a criterion is required by a node, it is completed in that node. It may not be
moved into a later node merely to declare progress.

A node may depend on an explicit prerequisite. It may not depend on an unnamed
future cleanup, hidden migration, planned rewrite, temporary mock, or promised
polish pass.

### 4.5 User approval is mandatory

Every node has a declared user acceptance route.

For internal foundation nodes, the user acts as the operator and observes the
real result through a command, failure injection, state inspection, or integrated
surface. For user-facing nodes, the user performs the actual workflow.

No model, test suite, or documentation author may approve on the user's behalf.

---

## 5. Required closure records for every skill

Every skill that reaches `CLOSED` must create exactly these two records:

```text
docs/versions/V1/skills/<SKILL_ID>/CLOSURE_AND_SPEC.md
docs/versions/V1/skills/<SKILL_ID>/USER_GUIDE_SOURCE.md
```

### `CLOSURE_AND_SPEC.md`

Must contain:

- exact capability statement;
- source revision and repository state;
- owning crates and modules;
- module line counts for touched authored modules;
- real public or operator path exercised;
- automated test commands and results;
- negative and failure-path results;
- regression commands and results;
- user acceptance procedure;
- explicit user approval record;
- supported behavior;
- explicit non-claims;
- known version-scope limits that are truly out of V1, not deferred V1 work.

### `USER_GUIDE_SOURCE.md`

Must contain all information needed to create the onboard Forge Guide and public
website documentation for the skill:

- what the capability does;
- who uses it;
- how to access it;
- step-by-step usage;
- controls and shortcuts;
- options and configuration;
- variations;
- expected results;
- errors and recovery;
- safety or approval prompts;
- current V1 limitations;
- interaction with Nyx where applicable.

It does not need publication polish. It must be complete enough that later work is
presentation and wording only.

For an internal-only skill, the user guide source must still explain the operator
observable, configuration, failure message, and support implications. It may not
be replaced with “internal only” and no useful information.

The two documents record an already working capability. Their existence alone
never closes the node.

---

## 6. Canonical V1 apex

### `FORGEOS-V1-APEX-001` — First Armor is a bootable self-hosting developer environment

- **Tier:** 5
- **Owner:** ForgeOS release authority
- **Direct prerequisites:** `FORGEOS-V1-SELFHOST-400`
- **Initial state:** `LOCKED`
- **Capability:** A developer installs ForgeOS on a supported Linux host, selects
  the ForgeOS session at login, completes real Rust development work with Forge
  World, real developer tools, Nyx, and a remote coding agent, survives restart,
  and commits a verified ForgeOS or Nyx change without using another IDE.
- **Must be true:** The complete journey passes twice on two real source changes,
  with the second journey beginning after a cold login and using restored project
  state.
- **Must not be true:** The journey may not depend on a hidden external IDE,
  manually pre-applied patch, fake terminal, scripted AI result, precomputed Git
  state, or operator repair outside ForgeOS.
- **User acceptance:** The user personally performs both journeys, confirms the
  environment is usable for the bounded V1 workflow, and explicitly approves V1.
- **Regression shield:** Every `CLOSED` V1 node remains closed under the final
  clean-host and self-hosting runs.
- **Closure result:** State becomes `RELEASE_EARNED`, not merely `CLOSED`.

---

# Tier 4 — Integrated V1 capabilities

## `FORGEOS-V1-WORKSTATION-400` — ForgeOS provides a bootable daily development workstation

- **Owner:** `forge-app`, `forge-session`, `forge-world`
- **Prerequisites:** `FORGEOS-V1-SESSION-300`, `FORGEOS-V1-PROJECT-300`,
  `FORGEOS-V1-CODE-300`, `FORGEOS-V1-TERMINAL-300`, `FORGEOS-V1-GIT-300`,
  `FORGEOS-V1-WORLD-300`, `FORGEOS-V1-DIST-300`
- **Initial state:** `LOCKED`
- **Must be true:** A supported machine can enter ForgeOS from the display manager
  and perform the complete local coding workflow for an extended session.
- **Must not be true:** ForgeOS may not be only a full-screen application launched
  manually from another IDE session for this claim.
- **User acceptance:** The user completes a normal development session from login
  through logout and approves responsiveness, navigation, and basic usability.
- **Regression shield:** Session, project, editor, terminal, Git, and Forge World
  behaviors remain independently usable.

## `FORGEOS-V1-CODE-400` — ForgeOS supports a complete real Rust authoring workflow

- **Owner:** `forge-editor`, `forge-bridge`, `forge-world`
- **Prerequisites:** `FORGEOS-V1-CODE-300`, `FORGEOS-V1-TERMINAL-300`,
  `FORGEOS-V1-VERIFY-300`
- **Initial state:** `LOCKED`
- **Must be true:** The user navigates, edits, saves, searches, receives Rust
  diagnostics, and verifies the change through real registered commands.
- **Must not be true:** Editing may not be delegated to an external IDE or reduced
  to opening files without language intelligence.
- **User acceptance:** The user implements a bounded Rust behavior using the
  ForgeOS editing surfaces and approves the workflow.
- **Regression shield:** Saving, diagnostics, command execution, and file identity
  remain stable across reopen.

## `FORGEOS-V1-SOURCE-400` — ForgeOS supports a complete source-control workflow

- **Owner:** `forge-git`, `forge-world`
- **Prerequisites:** `FORGEOS-V1-GIT-300`, `FORGEOS-V1-PATCH-300`
- **Initial state:** `LOCKED`
- **Must be true:** The user can inspect revision and branch, review diffs, stage
  selected changes, commit, and manage the isolated agent worktree path.
- **Must not be true:** ForgeOS may not display cached or invented Git state or
  mutate the repository without showing the exact intended operation.
- **User acceptance:** The user creates and commits a real source change and
  verifies the commit using an independent Git command from inside ForgeOS.
- **Regression shield:** Existing clean/dirty detection and patch review remain
  accurate after commit and reopen.

## `FORGEOS-V1-NYX-400` — Nyx is the integrated local AI host and bounded operator

- **Owner:** `nyx_server`, `forge-nyx-client`
- **Prerequisites:** `FORGEOS-V1-NYX-300`, `FORGEOS-V1-NYX-301`,
  `FORGEOS-V1-RECOVERY-300`
- **Initial state:** `LOCKED`
- **Must be true:** Nyx starts as a managed service, uses a selected local model,
  understands the active project through approved tools, performs bounded actions,
  resumes approved checkpoints, and recovers its session after restart.
- **Must not be true:** ForgeOS may not host a second AI runtime, silently grant
  shell authority, fabricate project state, or treat model prose as tool evidence.
- **User acceptance:** The user asks project-specific questions, approves one safe
  action, denies one action, restarts, and confirms the session and audit remain
  coherent.
- **Regression shield:** Local-only operation remains available when remote OpenAI
  access is disabled.

## `FORGEOS-V1-AGENT-400` — ForgeOS integrates one heavyweight remote coding agent safely

- **Owner:** `nyx_server`, `forge-nyx-client`, `forge-git`
- **Prerequisites:** `FORGEOS-V1-AGENT-300`, `FORGEOS-V1-PATCH-300`,
  `FORGEOS-V1-VERIFY-300`, `FORGEOS-V1-NYX-400`
- **Initial state:** `LOCKED`
- **Must be true:** A bounded task is packaged, executed through the configured
  OpenAI path in an isolated worktree, returned as a reviewable result, and never
  applied without declared approval.
- **Must not be true:** The remote agent may not modify the authoritative worktree
  directly, exceed its budget silently, award completion, or bypass review.
- **User acceptance:** The user submits a real bounded task, reviews the response,
  approves or rejects it, and confirms cost and audit visibility.
- **Regression shield:** Manual development and local Nyx remain usable when the
  remote provider is unavailable.

## `FORGEOS-V1-VERIFY-400` — ForgeOS closes a verified source-change loop

- **Owner:** `forge-core`, `forge-bridge`, `forge-terminal`, `forge-git`
- **Prerequisites:** `FORGEOS-V1-CODE-400`, `FORGEOS-V1-SOURCE-400`,
  `FORGEOS-V1-NYX-400`, `FORGEOS-V1-AGENT-400`, `FORGEOS-V1-VERIFY-300`
- **Initial state:** `LOCKED`
- **Must be true:** A source change moves from edit or agent patch through review,
  formatting, build, focused tests, broader required tests, and commit with exact
  command and revision records.
- **Must not be true:** A green visual state may not survive a failing command,
  mismatched revision, cancelled process, or stale result.
- **User acceptance:** The user completes one manual change and one agent-assisted
  change through the full loop and approves the results.
- **Regression shield:** Earlier successful results remain associated with their
  exact revisions and never overwrite current failures.

## `FORGEOS-V1-WORLD-400` — Forge World truthfully presents the V1 environment

- **Owner:** `forge-world`
- **Prerequisites:** `FORGEOS-V1-WORLD-300`, `FORGEOS-V1-RECOVERY-300`
- **Initial state:** `LOCKED`
- **Must be true:** Project, branch, dirty state, command state, build/test state,
  active Nyx state, and errors are visible and resolve to source-owned records.
- **Must not be true:** Animation, cached UI state, or scene scripts may not invent
  engineering truth or block keyboard-first work.
- **User acceptance:** The user compares every visible status against the real
  underlying tool and approves the basic V1 presentation.
- **Regression shield:** The environment remains usable with visual effects reduced
  or disabled.

## `FORGEOS-V1-SELFHOST-400` — ForgeOS completes a real ForgeOS feature inside ForgeOS

- **Owner:** All V1 subsystems
- **Prerequisites:** `FORGEOS-V1-WORKSTATION-400`, `FORGEOS-V1-VERIFY-400`,
  `FORGEOS-V1-WORLD-400`
- **Initial state:** `LOCKED`
- **Must be true:** A real, previously unimplemented ForgeOS or Nyx behavior is
  selected, implemented manually or through the bounded agent path, tested,
  reviewed, committed, and reopened entirely inside ForgeOS.
- **Must not be true:** The feature may not be documentation-only, fixture-only,
  preimplemented, externally edited, manually patched outside ForgeOS, or exempt
  from normal validation.
- **User acceptance:** The user performs the self-hosting journey and confirms that
  ForgeOS was sufficient for the complete bounded development task.
- **Regression shield:** The self-hosting change cannot break any earlier V1 skill;
  the full V1 regression set must remain green.

---

# Tier 3 — Complete user and operator workflows

## `FORGEOS-V1-SESSION-300` — The user logs into a usable ForgeOS session

- **Owner:** `forge-session`, `forge-app`
- **Prerequisites:** `FORGEOS-V1-SESSION-200`, `FORGEOS-V1-SESSION-201`,
  `FORGEOS-V1-WORLD-200`
- **Initial state:** `LOCKED`
- **Must be true:** Selecting ForgeOS in the display manager starts the required
  services and opens the shell without manual terminal setup.
- **Must not be true:** The claim may not depend on starting ForgeOS from an
  already-running desktop session.
- **User acceptance:** The user logs in, sees service health, logs out, and repeats.
- **Regression shield:** Normal host login recovery remains available if ForgeOS
  fails.

## `FORGEOS-V1-PROJECT-300` — The user registers, opens, and restores a repository workspace

- **Owner:** `forge-project`, `forge-core`, `forge-world`
- **Prerequisites:** `FORGEOS-V1-PROJECT-200`, `FORGEOS-V1-FILE-200`,
  `FORGEOS-V1-WORLD-200`
- **Initial state:** `LOCKED`
- **Must be true:** A real repository can be registered, validated, opened, closed,
  and restored with its project identity and declared commands intact.
- **Must not be true:** ForgeOS may not silently register paths outside the approved
  root or treat moved repositories as the same path without identity checks.
- **User acceptance:** The user registers two repositories, reopens each, and
  approves the restored state.
- **Regression shield:** Existing project records survive unrelated registrations.

## `FORGEOS-V1-CODE-300` — The user edits real Rust source with language intelligence

- **Owner:** `forge-editor`, `forge-world`
- **Prerequisites:** `FORGEOS-V1-EDITOR-200`, `FORGEOS-V1-EDITOR-201`,
  `FORGEOS-V1-PROJECT-300`
- **Initial state:** `LOCKED`
- **Must be true:** The user opens multiple Rust files, edits, saves, searches,
  navigates symbols, and receives diagnostics from the real language server.
- **Must not be true:** The editor may not lose unrelated buffers, silently rewrite
  bytes, or show synthetic diagnostics.
- **User acceptance:** The user intentionally introduces and fixes a Rust error,
  follows a definition, saves, reopens, and approves the behavior.
- **Regression shield:** Original line endings and untouched file bytes remain
  unchanged.

## `FORGEOS-V1-TERMINAL-300` — The user performs daily terminal and project-command work

- **Owner:** `forge-terminal`, `forge-world`
- **Prerequisites:** `FORGEOS-V1-TERMINAL-200`, `FORGEOS-V1-COMMAND-200`,
  `FORGEOS-V1-PROJECT-300`
- **Initial state:** `LOCKED`
- **Must be true:** Multiple terminals run in the correct working directory and
  registered commands can be launched, observed, cancelled, and revisited.
- **Must not be true:** Terminal output may not be fabricated, truncated without
  indication, or detached from its actual process identity.
- **User acceptance:** The user runs an interactive shell, a long command, a failing
  command, and a cancelled command.
- **Regression shield:** One terminal failure or cancellation does not kill other
  sessions or corrupt project state.

## `FORGEOS-V1-GIT-300` — The user performs a real Git inspect, stage, and commit workflow

- **Owner:** `forge-git`, `forge-world`
- **Prerequisites:** `FORGEOS-V1-GIT-200`, `FORGEOS-V1-GIT-201`,
  `FORGEOS-V1-PROJECT-300`
- **Initial state:** `LOCKED`
- **Must be true:** Real branch, revision, status, diff, staging, unstage, commit,
  restore confirmation, and worktree behavior are available.
- **Must not be true:** Destructive operations may not execute without explicit
  confirmation, and the UI may not infer success from process exit alone when
  repository state disagrees.
- **User acceptance:** The user stages only selected changes, commits them, restores
  one disposable change with confirmation, and verifies the result.
- **Regression shield:** Unselected changes and unrelated worktrees remain intact.

## `FORGEOS-V1-NYX-300` — The user receives project-aware assistance from a local model

- **Owner:** `nyx_server`, `forge-nyx-client`, `forge-world`
- **Prerequisites:** `FORGEOS-V1-NYX-200`, `FORGEOS-V1-NYX-201`,
  `FORGEOS-V1-PROJECT-300`
- **Initial state:** `LOCKED`
- **Must be true:** Nyx answers using the selected local model and active repository
  context gathered through recorded read tools.
- **Must not be true:** Nyx may not claim it read source it did not access, cross
  repository boundaries silently, or present model memory as current source.
- **User acceptance:** The user asks Nyx to locate and explain real source, then
  checks the cited files and approves accuracy within the bounded task.
- **Regression shield:** Chat without repository access remains clearly labeled and
  does not inherit stale project claims.

## `FORGEOS-V1-NYX-301` — The user controls Nyx tool execution through resumable approval

- **Owner:** `nyx_server`, `forge-nyx-client`
- **Prerequisites:** `FORGEOS-V1-NYX-202`, `FORGEOS-V1-TERMINAL-300`,
  `FORGEOS-V1-GIT-300`
- **Initial state:** `LOCKED`
- **Must be true:** Nyx requests permission for gated actions, records approval or
  denial, and resumes the exact suspended action after approval.
- **Must not be true:** Approval may not cause the model to regenerate or substitute
  a different command, path, patch, or tool request.
- **User acceptance:** The user approves one request, denies one, lets one expire,
  and verifies the audit trail and exact resumed operation.
- **Regression shield:** Read-only Nyx behavior remains available when all write and
  command permissions are denied.

## `FORGEOS-V1-AGENT-300` — The user sends one bounded coding task to a remote agent worktree

- **Owner:** `nyx_server`, `forge-nyx-client`, `forge-git`
- **Prerequisites:** `FORGEOS-V1-AGENT-200`, `FORGEOS-V1-GIT-300`,
  `FORGEOS-V1-NYX-300`
- **Initial state:** `LOCKED`
- **Must be true:** The user sees scope, source revision, allowed files, required
  commands, provider, and budget before dispatch; work occurs in an isolated
  worktree.
- **Must not be true:** The task may not silently expand scope, modify the primary
  worktree, or continue beyond the declared spend limit.
- **User acceptance:** The user dispatches a real bounded task and verifies worktree
  isolation, provider status, and returned records.
- **Regression shield:** Cancelling or failing a remote task leaves the primary
  repository unchanged.

## `FORGEOS-V1-PATCH-300` — The user reviews, accepts, or rejects a returned agent patch

- **Owner:** `forge-git`, `forge-nyx-client`, `forge-world`
- **Prerequisites:** `FORGEOS-V1-AGENT-201`, `FORGEOS-V1-GIT-300`,
  `FORGEOS-V1-VERIFY-200`
- **Initial state:** `LOCKED`
- **Must be true:** The exact patch, files, hunks, base revision, agent claims, and
  required validation are visible before application.
- **Must not be true:** A patch may not auto-apply, hide generated changes, apply to
  a mismatched base, or treat agent tests as authoritative local proof.
- **User acceptance:** The user rejects one patch or hunk set, approves another,
  applies it, and verifies the resulting Git diff.
- **Regression shield:** Rejected patches leave source untouched; accepted patches
  preserve unrelated work.

## `FORGEOS-V1-VERIFY-300` — The user runs and understands real formatting, build, and tests

- **Owner:** `forge-terminal`, `forge-core`, `forge-world`
- **Prerequisites:** `FORGEOS-V1-VERIFY-200`, `FORGEOS-V1-TERMINAL-300`,
  `FORGEOS-V1-GIT-300`
- **Initial state:** `LOCKED`
- **Must be true:** Registered formatting, build, focused test, and required broader
  test commands report exact command, exit state, output, duration, and revision.
- **Must not be true:** ForgeOS may not label cancelled, stale, mismatched-revision,
  or partially executed validation as green.
- **User acceptance:** The user observes one pass, one failure, one cancellation,
  repairs the source, and reruns to green.
- **Regression shield:** Prior run history remains inspectable and never overwrites
  current results.

## `FORGEOS-V1-WORLD-300` — The user performs the V1 workflow through one coherent cockpit

- **Owner:** `forge-world`, `forge-app`
- **Prerequisites:** `FORGEOS-V1-SESSION-300`, `FORGEOS-V1-PROJECT-300`,
  `FORGEOS-V1-CODE-300`, `FORGEOS-V1-TERMINAL-300`, `FORGEOS-V1-GIT-300`,
  `FORGEOS-V1-NYX-300`, `FORGEOS-V1-VERIFY-300`
- **Initial state:** `LOCKED`
- **Must be true:** Project selection, editor, terminal, Git, Nyx, validation, and
  basic system status are reachable quickly through consistent keyboard and pointer
  controls.
- **Must not be true:** The user may not be forced to traverse decorative 3D space,
  wait through mandatory animation, or open another IDE to complete the workflow.
- **User acceptance:** The user completes a representative coding session and
  approves layout, focus, shortcuts, and state visibility.
- **Regression shield:** Flat or reduced-effects mode retains full V1 functionality.

## `FORGEOS-V1-RECOVERY-300` — The user resumes work after process or shell failure

- **Owner:** `forge-session`, `forge-project`, `forge-world`, `forge-nyx-client`
- **Prerequisites:** `FORGEOS-V1-RECOVERY-200`, `FORGEOS-V1-WORLD-300`
- **Initial state:** `LOCKED`
- **Must be true:** After controlled shell, Nyx-client, or terminal-process failure,
  ForgeOS restarts and restores the last safe project, buffers, terminals metadata,
  and service state without corrupting source.
- **Must not be true:** Recovery may not silently replay destructive commands,
  discard unsaved work without warning, or mark interrupted validation as passed.
- **User acceptance:** The user performs declared failure injections and approves
  the recovered state and warnings.
- **Regression shield:** Normal clean shutdown and startup remain unchanged.

## `FORGEOS-V1-DIST-300` — The user installs, updates, and boots ForgeOS on a clean supported host

- **Owner:** packaging and `forge-session`
- **Prerequisites:** `FORGEOS-V1-DIST-200`, `FORGEOS-V1-SESSION-300`,
  `FORGEOS-V1-RECOVERY-300`
- **Initial state:** `LOCKED`
- **Must be true:** A documented package installs the session and services on a clean
  supported Linux host, survives one update, and can be removed without damaging
  the host desktop.
- **Must not be true:** Installation may not depend on the developer worktree,
  unrecorded manual copying, machine-specific absolute paths, or hidden local files.
- **User acceptance:** The user performs clean install, login, update, second login,
  and uninstall or rollback on a test machine or clean VM.
- **Regression shield:** The host's normal desktop session remains usable throughout.

---

# Tier 2 — Functional V1 systems

## `FORGEOS-V1-PROJECT-200` — Persistent project registry and workspace restoration

- **Owner:** `forge-project`, `forge-core`
- **Prerequisites:** `FORGEOS-V1-PROJECT-100`
- **Initial state:** `LOCKED`
- **Must be true:** Projects persist with stable identity, repository root, display
  name, registered commands, recent-open state, and last safe workspace snapshot.
- **Must not be true:** Display name, path string alone, or list order may not become
  canonical project identity.
- **User acceptance:** The user registers, renames, closes, reopens, and removes a
  disposable project while verifying source remains untouched.
- **Regression shield:** Other project records remain byte-for-byte equivalent where
  their state did not change.

## `FORGEOS-V1-SESSION-200` — Dedicated ForgeOS session bootstrap

- **Owner:** `forge-session`
- **Prerequisites:** `FORGEOS-V1-SESSION-100`
- **Initial state:** `LOCKED`
- **Must be true:** A display-manager session entry launches the ForgeOS composition
  root with the expected environment and returns a real failure status when startup
  fails.
- **Must not be true:** The session may not assume the user's shell profile, current
  worktree, or hardcoded home path.
- **User acceptance:** The user selects the session and observes successful startup
  and one intentional startup failure with a recoverable message.
- **Regression shield:** Other installed desktop sessions are not modified.

## `FORGEOS-V1-SESSION-201` — Managed ForgeOS and Nyx service lifecycle

- **Owner:** `forge-session`, `forge-nyx-client`
- **Prerequisites:** `FORGEOS-V1-SESSION-100`, `FORGEOS-V1-NYX-100`
- **Initial state:** `LOCKED`
- **Must be true:** Required services start in declared order, expose health, restart
  within policy, and stop cleanly on logout.
- **Must not be true:** Forge World may not infer service health from process presence
  alone or spawn duplicate Nyx instances.
- **User acceptance:** The user starts, stops, crashes, and restarts managed services
  and verifies exact state.
- **Regression shield:** A Nyx failure does not destroy the local editor, terminal,
  project, or Git workflow.

## `FORGEOS-V1-FILE-200` — Repository file tree and search

- **Owner:** `forge-editor`, `forge-project`
- **Prerequisites:** `FORGEOS-V1-FILE-100`, `FORGEOS-V1-PROJECT-200`
- **Initial state:** `LOCKED`
- **Must be true:** The user browses the approved repository tree, opens files, finds
  text, and receives explicit errors for unreadable or out-of-bound paths.
- **Must not be true:** Symlink, `..`, mount, or canonicalization tricks may not escape
  the registered repository boundary.
- **User acceptance:** The user searches known content, tests no-match behavior, and
  attempts one rejected escape.
- **Regression shield:** Search never mutates repository files or project records.

## `FORGEOS-V1-EDITOR-200` — Multi-buffer file editing and atomic save

- **Owner:** `forge-editor`
- **Prerequisites:** `FORGEOS-V1-EDITOR-100`, `FORGEOS-V1-FILE-200`
- **Initial state:** `LOCKED`
- **Must be true:** Multiple buffers preserve independent dirty state, save atomically,
  detect external changes, and warn before destructive close.
- **Must not be true:** Save may not truncate on failure, overwrite a newer external
  change silently, or normalize unrelated bytes.
- **User acceptance:** The user edits several files, exercises save, discard, conflict,
  and reopen behavior.
- **Regression shield:** Unedited files and buffers remain unchanged.

## `FORGEOS-V1-EDITOR-201` — Rust syntax and language-intelligence integration

- **Owner:** `forge-editor`, `forge-bridge`
- **Prerequisites:** `FORGEOS-V1-EDITOR-200`, `FORGEOS-V1-PARSER-100`,
  `FORGEOS-V1-LSP-100`
- **Initial state:** `LOCKED`
- **Must be true:** Tree-sitter and Rust Analyzer provide syntax, diagnostics,
  definition navigation, symbol search, and basic completion for the active project.
- **Must not be true:** ForgeOS may not synthesize diagnostics or silently show stale
  responses from another project or document version.
- **User acceptance:** The user verifies each declared feature against a real Rust
  workspace and a deliberate error.
- **Regression shield:** Editing remains functional if the language server is down,
  with the degraded state clearly shown.

## `FORGEOS-V1-TERMINAL-200` — Managed embedded terminal sessions

- **Owner:** `forge-terminal`
- **Prerequisites:** `FORGEOS-V1-TERMINAL-100`, `FORGEOS-V1-PROJECT-200`
- **Initial state:** `LOCKED`
- **Must be true:** ForgeOS creates, renders, resizes, writes to, reads from, and closes
  multiple real PTYs associated with the correct project.
- **Must not be true:** A terminal may not become a fake log view or share process
  identity with another terminal.
- **User acceptance:** The user runs interactive input, resize-sensitive output,
  multiple sessions, and clean termination.
- **Regression shield:** Closing one terminal leaves other processes alive unless
  explicitly linked.

## `FORGEOS-V1-COMMAND-200` — Registered project command execution and output history

- **Owner:** `forge-terminal`, `forge-core`
- **Prerequisites:** `FORGEOS-V1-COMMAND-100`, `FORGEOS-V1-TERMINAL-200`
- **Initial state:** `LOCKED`
- **Must be true:** Declared format, build, test, and custom commands run with exact
  argv, working directory, environment policy, process identity, output, exit state,
  and cancellation.
- **Must not be true:** Nyx or Forge World may not substitute arbitrary shell strings
  for registered commands without a separately approved operator action.
- **User acceptance:** The user configures and runs passing, failing, long, and
  cancelled commands and inspects their records.
- **Regression shield:** Command history remains attached to the correct project and
  revision.

## `FORGEOS-V1-GIT-200` — Real Git status, branch, revision, and diff inspection

- **Owner:** `forge-git`
- **Prerequisites:** `FORGEOS-V1-GIT-100`, `FORGEOS-V1-PROJECT-200`
- **Initial state:** `LOCKED`
- **Must be true:** Git state is read from the registered repository and represented
  without losing staged, unstaged, untracked, rename, delete, or conflict meaning.
- **Must not be true:** ForgeOS may not infer clean state from an empty cached diff or
  hide unsupported states.
- **User acceptance:** The user creates each supported change class and verifies the
  displayed result against native Git.
- **Regression shield:** Inspection remains read-only.

## `FORGEOS-V1-GIT-201` — Safe Git mutation and isolated worktree control

- **Owner:** `forge-git`
- **Prerequisites:** `FORGEOS-V1-GIT-101`, `FORGEOS-V1-GIT-200`,
  `FORGEOS-V1-PATCH-100`
- **Initial state:** `LOCKED`
- **Must be true:** Stage, unstage, commit, confirmed restore, branch-safe worktree
  create, and worktree cleanup operate on explicit paths and revisions.
- **Must not be true:** Destructive Git operations may not execute from ambiguous
  selection, stale status, or unconfirmed broad scope.
- **User acceptance:** The user performs every mutation on a disposable repository
  and verifies native Git results.
- **Regression shield:** Unselected files, branches, and worktrees remain unchanged.

## `FORGEOS-V1-NYX-200` — Local model selection and Nyx conversation lifecycle

- **Owner:** `nyx_server`, `forge-nyx-client`
- **Prerequisites:** `FORGEOS-V1-NYX-100`, `FORGEOS-V1-SESSION-201`
- **Initial state:** `LOCKED`
- **Must be true:** ForgeOS discovers Nyx, lists available local models, selects one,
  creates a session, streams responses, handles errors, and restores conversation
  identity.
- **Must not be true:** ForgeOS may not bypass Nyx to call a local model directly or
  hide which model produced a response.
- **User acceptance:** The user changes models, sends messages, restarts the client,
  and verifies session continuity and model attribution.
- **Regression shield:** A missing model returns a clear error without corrupting
  other sessions.

## `FORGEOS-V1-NYX-201` — Project-aware Nyx read tools

- **Owner:** `nyx_server`, `forge-nyx-client`, `forge-project`
- **Prerequisites:** `FORGEOS-V1-NYX-200`, `FORGEOS-V1-FILE-200`,
  `FORGEOS-V1-GIT-200`
- **Initial state:** `LOCKED`
- **Must be true:** Nyx receives the active project contract and may use bounded file
  search, file read, Git status, and Git diff tools with recorded inputs and outputs.
- **Must not be true:** Nyx may not read outside the registered project, claim hidden
  context, or use stale tool results as current state.
- **User acceptance:** The user asks questions requiring each read tool and verifies
  the audit against the repository.
- **Regression shield:** Denying repository tools degrades to general chat without
  leaking previous project content.

## `FORGEOS-V1-NYX-202` — Safe registered commands and exact checkpoint resume

- **Owner:** `nyx_server`, `forge-nyx-client`, `forge-terminal`
- **Prerequisites:** `FORGEOS-V1-NYX-101`, `FORGEOS-V1-NYX-201`,
  `FORGEOS-V1-COMMAND-200`
- **Initial state:** `LOCKED`
- **Must be true:** Nyx may request only declared commands or explicit operator shell
  actions, gated requests suspend with an immutable payload, and approval resumes
  that exact payload.
- **Must not be true:** Approval may not rerun model planning, alter argv, broaden
  path scope, or convert denial into a new equivalent request without disclosure.
- **User acceptance:** The user verifies approve, deny, expire, and restart behavior
  against exact request hashes.
- **Regression shield:** Nyx read tools remain operational under a deny-all execute
  policy.

## `FORGEOS-V1-AGENT-200` — OpenAI heavyweight task dispatch

- **Owner:** `nyx_server`, `forge-nyx-client`, `forge-git`
- **Prerequisites:** `FORGEOS-V1-AGENT-100`, `FORGEOS-V1-NYX-200`,
  `FORGEOS-V1-GIT-201`
- **Initial state:** `LOCKED`
- **Must be true:** A versioned task packet includes project, revision, worktree,
  capability or task statement, scope, constraints, required commands, provider,
  model, and budget; the response is retained with provider status.
- **Must not be true:** Dispatch may not omit source revision, use the primary
  worktree, exceed budget silently, or represent provider prose as local proof.
- **User acceptance:** The user inspects and sends a real packet, cancels one packet,
  and confirms spend and run records.
- **Regression shield:** Provider failure never mutates the authoritative worktree.

## `FORGEOS-V1-AGENT-201` — Returned patch intake, review, and controlled application

- **Owner:** `forge-nyx-client`, `forge-git`, `forge-world`
- **Prerequisites:** `FORGEOS-V1-AGENT-200`, `FORGEOS-V1-PATCH-100`,
  `FORGEOS-V1-GIT-201`
- **Initial state:** `LOCKED`
- **Must be true:** Returned files and patches are hashed, base-checked, rendered as
  exact diffs, reviewable by file and hunk, rejectable, and applicable only after
  approval.
- **Must not be true:** Hidden files, binary changes, mismatched bases, or unsupported
  patch forms may not be silently accepted.
- **User acceptance:** The user reviews one valid and one invalid return and confirms
  correct application and rejection.
- **Regression shield:** Patch intake never changes source before approval.

## `FORGEOS-V1-VERIFY-200` — Version-bound build and test result records

- **Owner:** `forge-core`, `forge-terminal`
- **Prerequisites:** `FORGEOS-V1-COMMAND-200`, `FORGEOS-V1-GIT-200`,
  `FORGEOS-V1-STATE-000`
- **Initial state:** `LOCKED`
- **Must be true:** Each validation result records command ID, exact argv, revision,
  dirty-state identity, start/end state, exit code, output reference, and cancellation
  or timeout.
- **Must not be true:** Results from another revision, worktree, or interrupted run
  may not satisfy the current validation state.
- **User acceptance:** The user compares recorded results with real command output
  for pass, fail, and cancellation.
- **Regression shield:** Historical records remain immutable.

## `FORGEOS-V1-WORLD-200` — Basic Bevy shell and truthful status HUD

- **Owner:** `forge-world`, `forge-app`
- **Prerequisites:** `FORGEOS-V1-WORLD-100`, `FORGEOS-V1-PROJECT-200`,
  `FORGEOS-V1-TERMINAL-200`, `FORGEOS-V1-GIT-200`, `FORGEOS-V1-NYX-200`,
  `FORGEOS-V1-VERIFY-200`
- **Initial state:** `LOCKED`
- **Must be true:** The shell presents project selection, editor region, terminal,
  Git, Nyx, validation, service health, branch, dirty state, and active process state
  through source-backed projections.
- **Must not be true:** Forge World may not write canonical state directly, invent
  success, or hide a failed subsystem behind animation.
- **User acceptance:** The user cross-checks each status and uses keyboard navigation
  across the full shell.
- **Regression shield:** Disabling effects leaves every V1 control accessible.

## `FORGEOS-V1-RECOVERY-200` — Durable workspace and service recovery

- **Owner:** `forge-session`, `forge-project`, `forge-nyx-client`
- **Prerequisites:** `FORGEOS-V1-RECOVERY-100`, `FORGEOS-V1-PROJECT-200`,
  `FORGEOS-V1-SESSION-201`, `FORGEOS-V1-TERMINAL-200`, `FORGEOS-V1-NYX-200`
- **Initial state:** `LOCKED`
- **Must be true:** Safe workspace state, dirty buffers, service state, terminal
  metadata, and current project recover after declared crashes or restart.
- **Must not be true:** Recovery may not replay commands, resurrect terminated
  processes as alive, or conceal data loss.
- **User acceptance:** The user performs controlled crashes and checks restored and
  intentionally non-restored state.
- **Regression shield:** Clean startup remains faster and simpler than recovery mode.

## `FORGEOS-V1-DIST-200` — Reproducible ForgeOS session package

- **Owner:** packaging and `forge-session`
- **Prerequisites:** `FORGEOS-V1-SESSION-200`, `FORGEOS-V1-SESSION-201`,
  `FORGEOS-V1-WORLD-200`
- **Initial state:** `LOCKED`
- **Must be true:** A versioned package installs binaries, assets, service definitions,
  session entry, defaults, and uninstall metadata on the supported host.
- **Must not be true:** Packaging may not include developer caches, absolute user
  paths, unlicensed assets, or source-worktree dependencies.
- **User acceptance:** The user inspects package contents and installs it in a clean
  environment.
- **Regression shield:** Upgrade preserves supported user project records and settings.

---

# Tier 1 — Local mechanisms

## `FORGEOS-V1-PROJECT-100` — Validated project manifest and repository identity

- **Owner:** `forge-project`, `forge-core`
- **Prerequisites:** `FORGEOS-V1-CONTRACT-000`, `FORGEOS-V1-STATE-000`,
  `FORGEOS-V1-PATH-000`, `FORGEOS-V1-GUARD-002`
- **Initial state:** `LOCKED`
- **Must be true:** A versioned manifest validates display name, canonical repository
  identity, allowed roots, commands, language profile, and settings.
- **Must not be true:** Unknown required fields, invalid paths, duplicate IDs, or
  unsupported schema versions may not be accepted silently.
- **User acceptance:** The user imports valid, malformed, duplicate, and moved project
  fixtures and approves the outcomes.
- **Regression shield:** Reopening a valid unchanged manifest yields equivalent state.

## `FORGEOS-V1-SESSION-100` — Session and managed-service lifecycle contract

- **Owner:** `forge-session`
- **Prerequisites:** `FORGEOS-V1-CONTRACT-000`, `FORGEOS-V1-PROCESS-000`,
  `FORGEOS-V1-GUARD-002`
- **Initial state:** `LOCKED`
- **Must be true:** Startup order, readiness, restart policy, shutdown order, and
  failure states are explicit and testable.
- **Must not be true:** Service order may not depend on timing sleeps or process-name
  guessing.
- **User acceptance:** The user runs the lifecycle harness and observes ordered start,
  failure, restart, and stop.
- **Regression shield:** Adding a service cannot reorder unrelated services silently.

## `FORGEOS-V1-FILE-100` — Boundary-safe file access and atomic write

- **Owner:** `forge-editor`, `forge-project`
- **Prerequisites:** `FORGEOS-V1-PATH-000`, `FORGEOS-V1-STATE-000`
- **Initial state:** `LOCKED`
- **Must be true:** Reads and atomic writes resolve through approved project roots and
  detect changed-on-disk conflicts.
- **Must not be true:** Symlink escapes, path traversal, partial writes, or hidden
  encoding replacement may not occur.
- **User acceptance:** The user runs read, write, conflict, denied-path, and failure
  injection cases.
- **Regression shield:** Failed writes preserve original bytes.

## `FORGEOS-V1-EDITOR-100` — Editor buffer identity and dirty-state model

- **Owner:** `forge-editor`
- **Prerequisites:** `FORGEOS-V1-FILE-100`, `FORGEOS-V1-CONTRACT-000`
- **Initial state:** `LOCKED`
- **Must be true:** Buffer identity, content version, cursor state, dirty state, disk
  version, save outcome, and conflict state are explicit.
- **Must not be true:** File path aliases may not create silent duplicate authorities
  for the same file.
- **User acceptance:** The user exercises two buffers for one file alias and verifies
  conflict prevention.
- **Regression shield:** Buffer bookkeeping never modifies file content by itself.

## `FORGEOS-V1-PARSER-100` — Incremental Tree-sitter parsing adapter

- **Owner:** `forge-editor`, `forge-bridge`
- **Prerequisites:** `FORGEOS-V1-ARCH-001`, `FORGEOS-V1-FILE-100`
- **Initial state:** `LOCKED`
- **Must be true:** Parser state updates against exact buffer versions and exposes
  syntax spans and parse errors without owning source bytes.
- **Must not be true:** Parse state from an older buffer version may not be shown as
  current.
- **User acceptance:** The user edits valid and invalid Rust and observes correct
  incremental syntax behavior.
- **Regression shield:** Parser failure does not prevent plain-text editing or save.

## `FORGEOS-V1-LSP-100` — Rust Analyzer process and JSON-RPC adapter

- **Owner:** `forge-editor`, `forge-bridge`
- **Prerequisites:** `FORGEOS-V1-CONTRACT-000`, `FORGEOS-V1-PROCESS-000`,
  `FORGEOS-V1-GUARD-002`
- **Initial state:** `LOCKED`
- **Must be true:** ForgeOS starts Rust Analyzer, manages document versions, routes
  requests and responses, handles restart, and reports unsupported capabilities.
- **Must not be true:** Responses may not cross project or document versions silently.
- **User acceptance:** The user observes startup, diagnostics, restart, and one
  intentional protocol error.
- **Regression shield:** LSP failure leaves editor buffers intact.

## `FORGEOS-V1-TERMINAL-100` — PTY spawn, I/O, resize, and termination

- **Owner:** `forge-terminal`, `forge-bridge`
- **Prerequisites:** `FORGEOS-V1-PROCESS-000`, `FORGEOS-V1-PATH-000`,
  `FORGEOS-V1-GUARD-002`
- **Initial state:** `LOCKED`
- **Must be true:** PTYs have stable IDs, declared working directories, bidirectional
  bytes, resize, exit status, and explicit termination.
- **Must not be true:** PTY output may not be normalized into misleading text or
  shared between sessions.
- **User acceptance:** The user runs an interactive test program and verifies I/O,
  resize, and termination.
- **Regression shield:** Terminating one PTY does not affect unrelated processes.

## `FORGEOS-V1-COMMAND-100` — Registered command definition and execution policy

- **Owner:** `forge-terminal`, `forge-core`
- **Prerequisites:** `FORGEOS-V1-PROCESS-000`, `FORGEOS-V1-PATH-000`,
  `FORGEOS-V1-CONTRACT-000`
- **Initial state:** `LOCKED`
- **Must be true:** Commands use stable IDs, exact argv arrays, declared working
  directories, environment policy, timeout, cancellation, and authority class.
- **Must not be true:** Registered commands may not be mutable shell prose or inherit
  undeclared environment secrets.
- **User acceptance:** The user validates accepted and rejected command definitions
  and inspects the exact launch payload.
- **Regression shield:** Existing command IDs cannot silently change meaning.

## `FORGEOS-V1-GIT-100` — Read-only Git adapter

- **Owner:** `forge-git`, `forge-bridge`
- **Prerequisites:** `FORGEOS-V1-PATH-000`, `FORGEOS-V1-PROCESS-000`,
  `FORGEOS-V1-GUARD-002`
- **Initial state:** `LOCKED`
- **Must be true:** Branch, revision, status, worktree list, and diff are represented
  through typed outcomes with native error preservation.
- **Must not be true:** Parsing may not collapse distinct Git states or assume English
  human-formatted output when a stable machine format exists.
- **User acceptance:** The user compares typed results with native Git across prepared
  repository states.
- **Regression shield:** The adapter performs no mutation.

## `FORGEOS-V1-GIT-101` — Git mutation and worktree primitives

- **Owner:** `forge-git`, `forge-bridge`
- **Prerequisites:** `FORGEOS-V1-GIT-100`, `FORGEOS-V1-CONTRACT-000`
- **Initial state:** `LOCKED`
- **Must be true:** Stage, unstage, commit, confirmed restore, create worktree, and
  remove worktree use explicit typed requests and report resulting repository state.
- **Must not be true:** Broad destructive defaults, implicit `--force`, or ambiguous
  pathspecs are forbidden.
- **User acceptance:** The user exercises each primitive in a disposable repository.
- **Regression shield:** Failed mutations leave unrelated source and refs unchanged.

## `FORGEOS-V1-NYX-100` — Nyx health and versioned client protocol

- **Owner:** `forge-nyx-client`, `nyx_server`
- **Prerequisites:** `FORGEOS-V1-CONTRACT-000`, `FORGEOS-V1-PROCESS-000`,
  `FORGEOS-V1-GUARD-002`
- **Initial state:** `LOCKED`
- **Must be true:** ForgeOS discovers Nyx through configured transport, negotiates a
  supported protocol, reads health and capabilities, and distinguishes unavailable,
  incompatible, and unhealthy states.
- **Must not be true:** ForgeOS may not infer compatibility from HTTP success alone or
  call model providers around Nyx.
- **User acceptance:** The user connects compatible, unavailable, and incompatible
  Nyx fixtures or instances.
- **Regression shield:** Nyx incompatibility does not crash the ForgeOS shell.

## `FORGEOS-V1-NYX-101` — Permission grant, checkpoint, and immutable resume token

- **Owner:** `nyx_server`, `forge-nyx-client`
- **Prerequisites:** `FORGEOS-V1-NYX-100`, `FORGEOS-V1-STATE-000`,
  `FORGEOS-V1-HASH-000`
- **Initial state:** `LOCKED`
- **Must be true:** Tool requests carry scoped authority, immutable payload identity,
  expiration, decision, and exact resume token.
- **Must not be true:** Approval may not authorize a different request or survive
  payload mutation.
- **User acceptance:** The user verifies request hashes and decision behavior.
- **Regression shield:** Denied and expired requests cannot execute later.

## `FORGEOS-V1-AGENT-100` — Remote-agent task and budget record

- **Owner:** `forge-nyx-client`, `nyx_server`
- **Prerequisites:** `FORGEOS-V1-NYX-100`, `FORGEOS-V1-PATH-000`,
  `FORGEOS-V1-STATE-000`, `FORGEOS-V1-HASH-000`
- **Initial state:** `LOCKED`
- **Must be true:** Task identity, provider, model, source revision, worktree, scope,
  budget, status, response, and cost are durable and inspectable.
- **Must not be true:** A task may not continue after cancellation or exceed its
  declared budget without a new approval.
- **User acceptance:** The user inspects complete, failed, cancelled, and budget-hit
  task records.
- **Regression shield:** One task cannot mutate another task's record or worktree.

## `FORGEOS-V1-PATCH-100` — Patch identity, base validation, and safe application primitive

- **Owner:** `forge-git`, `forge-nyx-client`
- **Prerequisites:** `FORGEOS-V1-PATH-000`, `FORGEOS-V1-STATE-000`,
  `FORGEOS-V1-HASH-000`
- **Initial state:** `LOCKED`
- **Must be true:** Patches have stable identity, declared base revision, file table,
  content hash, validation result, and atomic apply outcome.
- **Must not be true:** Mismatched-base, path-escaping, malformed, hidden binary, or
  partially applicable patches may not be accepted silently.
- **User acceptance:** The user applies valid fixtures and observes rejection of each
  invalid class.
- **Regression shield:** Failed application preserves the original worktree.

## `FORGEOS-V1-WORLD-100` — Source-backed view projection and input action routing

- **Owner:** `forge-world`, `forge-protocol`
- **Prerequisites:** `FORGEOS-V1-ARCH-001`, `FORGEOS-V1-CONTRACT-000`,
  `FORGEOS-V1-GUARD-001`, `FORGEOS-V1-GUARD-002`
- **Initial state:** `LOCKED`
- **Must be true:** Forge World receives immutable view models and emits typed user
  intents without owning domain mutation.
- **Must not be true:** UI widgets, scenes, or animations may not directly write
  canonical project, Git, command, or Nyx state.
- **User acceptance:** The user triggers actions and verifies resulting core commands
  and refreshed projections.
- **Regression shield:** Re-render and window resize do not change canonical state.

## `FORGEOS-V1-RECOVERY-100` — Workspace snapshot and crash-journal primitive

- **Owner:** `forge-project`, `forge-session`
- **Prerequisites:** `FORGEOS-V1-STATE-000`, `FORGEOS-V1-PROCESS-000`,
  `FORGEOS-V1-HASH-000`
- **Initial state:** `LOCKED`
- **Must be true:** Snapshots are versioned, atomic, checksummed, and distinguish safe
  restorable state from interrupted actions and live processes.
- **Must not be true:** Recovery records may not claim processes are alive, replay
  side effects, or overwrite a newer valid snapshot silently.
- **User acceptance:** The user corrupts and interrupts prepared snapshots and checks
  recovery choices.
- **Regression shield:** Invalid recovery data does not damage current project data.

---

# Tier 0 — Atomic foundations and guards

## `FORGEOS-V1-ARCH-000` — Rust workspace and authority crate skeleton

- **Owner:** repository root
- **Prerequisites:** none
- **Current state:** `ACTIVE`
- **Must be true:** The Rust workspace contains the declared V1 authority crates,
  composition root, test locations, and dependency direction needed by this tree.
- **Must not be true:** Business logic may not begin in `forge-app`, `main.rs`, Bevy
  scenes, scripts, or an undifferentiated monolithic crate.
- **User acceptance:** The user inspects the workspace, runs the initial build, and
  approves the crate boundaries before later skills unlock.
- **Regression shield:** The existing documentation-only base remains intact except
  for the new source skeleton and required configuration.

## `FORGEOS-V1-ARCH-001` — Scoped module hierarchy and public routing

- **Owner:** all production crates
- **Prerequisites:** `FORGEOS-V1-ARCH-000`
- **Initial state:** `LOCKED`
- **Must be true:** Each crate routes public APIs through `lib.rs`; nested behavior is
  organized through named submodules and `mod.rs` where directories are used.
- **Must not be true:** Production logic may not accumulate in crate roots, giant
  `main.rs` files, generic catch-all modules, or uncontrolled public re-exports.
- **User acceptance:** The user inspects representative imports and module paths and
  approves the organization and naming.
- **Regression shield:** Existing public routes remain stable unless versioned and
  migrated explicitly.

## `FORGEOS-V1-GUARD-000` — Authored source module-size verifier

- **Owner:** `forge-guards`
- **Prerequisites:** `FORGEOS-V1-ARCH-000`
- **Initial state:** `LOCKED`
- **Must be true:** One executable verifier counts physical lines in every authored
  Rust source module, warns at 1001-1200, and fails at 1201 or more.
- **Must not be true:** The verifier may not treat 1200 as the design target, scan
  Markdown for status, skip authored modules through arbitrary allowlists, or count
  generated and vendored source as authored code.
- **User acceptance:** The user runs fixtures at 500, 1000, 1001, 1200, and 1201 lines
  and approves exact outcomes.
- **Regression shield:** Every later source skill runs this guard; no skill closes
  while any warning above 1000 remains.

## `FORGEOS-V1-GUARD-001` — Forge Core purity guard

- **Owner:** `forge-guards`, `forge-core`
- **Prerequisites:** `FORGEOS-V1-ARCH-000`, `FORGEOS-V1-ARCH-001`
- **Initial state:** `LOCKED`
- **Must be true:** An automated guard rejects forbidden effect, UI, Nyx transport,
  Git, PTY, filesystem-adapter, LSP, DAP, provider, and session dependencies from
  Forge Core.
- **Must not be true:** The guard may not bless behavior, parse documentation, or
  allow a generic adapter crate to smuggle effects into Core.
- **User acceptance:** The user introduces representative forbidden dependencies and
  observes rejection, then removes them and observes pass.
- **Regression shield:** Every later dependency change runs the guard.

## `FORGEOS-V1-GUARD-002` — Cross-subsystem seam direction guards

- **Owner:** `forge-guards`
- **Prerequisites:** `FORGEOS-V1-ARCH-001`, `FORGEOS-V1-CONTRACT-000`
- **Initial state:** `LOCKED`
- **Must be true:** Guards enforce the declared dependency graph between Forge Core,
  Forge World, Developer Bridge, Nyx client, session, editor, terminal, and Git.
- **Must not be true:** Forge World may not own project truth, ForgeOS may not host a
  parallel Nyx runtime, and adapters may not depend backward into presentation.
- **User acceptance:** The user tests representative legal and illegal seam fixtures.
- **Regression shield:** All later crate graph changes must remain legal.

## `FORGEOS-V1-CONTRACT-000` — Stable IDs, typed errors, and versioned envelopes

- **Owner:** `forge-protocol`, `forge-core`
- **Prerequisites:** `FORGEOS-V1-ARCH-000`
- **Initial state:** `LOCKED`
- **Must be true:** Project, repository, process, terminal, command, session, task,
  patch, result, and event identities are stable typed values with versioned request,
  result, and error envelopes.
- **Must not be true:** Display names, list indexes, raw paths, timestamps, or model
  wording may not become canonical IDs.
- **User acceptance:** The user runs round-trip, unknown-version, duplicate-ID, and
  typed-error fixtures and approves the API shape.
- **Regression shield:** Published V1 contract meanings cannot silently change.

## `FORGEOS-V1-STATE-000` — Atomic versioned local persistence

- **Owner:** `forge-core`, `forge-project`
- **Prerequisites:** `FORGEOS-V1-CONTRACT-000`
- **Initial state:** `LOCKED`
- **Must be true:** Canonical local records write atomically, reopen through explicit
  schema versions, reject corrupt or unsupported data, and preserve the previous
  valid state on failure.
- **Must not be true:** Partial writes, implicit schema guessing, UI-owned persistence,
  or silent reset-to-default are forbidden.
- **User acceptance:** The user exercises create, reopen, migration fixture, corrupt
  file, interrupted write, and recovery.
- **Regression shield:** Equivalent unchanged state reopens equivalently.

## `FORGEOS-V1-PATH-000` — Canonical repository path and boundary identity

- **Owner:** `forge-project`, `forge-protocol`
- **Prerequisites:** `FORGEOS-V1-CONTRACT-000`
- **Initial state:** `LOCKED`
- **Must be true:** Repository roots and child paths resolve canonically, preserve
  display paths separately, and reject escapes through traversal, symlink, or
  unexpected mount behavior.
- **Must not be true:** User-provided path strings may not be trusted without
  canonical boundary verification.
- **User acceptance:** The user runs normal, moved, symlink, traversal, and denied
  path cases.
- **Regression shield:** Valid in-root paths continue to resolve after unrelated
  project changes.

## `FORGEOS-V1-PROCESS-000` — Stable process lifecycle and cancellation model

- **Owner:** `forge-bridge`, `forge-protocol`
- **Prerequisites:** `FORGEOS-V1-CONTRACT-000`
- **Initial state:** `LOCKED`
- **Must be true:** Spawn request, process identity, running state, output channels,
  exit, timeout, cancellation, and failure are explicit and race-safe.
- **Must not be true:** PID alone may not serve as durable identity, and cancellation
  may not be reported as normal success.
- **User acceptance:** The user runs fast, failing, timed-out, cancelled, and child-
  process fixtures.
- **Regression shield:** One process outcome cannot overwrite another process record.

## `FORGEOS-V1-HASH-000` — Stable artifact and request hashing

- **Owner:** `forge-protocol`, `forge-core`
- **Prerequisites:** `FORGEOS-V1-CONTRACT-000`
- **Initial state:** `LOCKED`
- **Must be true:** Files, patches, tool requests, snapshots, and result payloads can
  receive stable SHA-256 identities over declared canonical bytes.
- **Must not be true:** Timestamps, host paths, unstable map ordering, or display text
  may not change identity unless they are part of the declared content.
- **User acceptance:** The user verifies identical, reordered, changed, and corrupt
  fixtures.
- **Regression shield:** Existing hash contracts remain byte stable within V1.

---

## 7. Direct dependency index

```text
FORGEOS-V1-APEX-001
  <- FORGEOS-V1-SELFHOST-400

FORGEOS-V1-SELFHOST-400
  <- FORGEOS-V1-WORKSTATION-400
  <- FORGEOS-V1-VERIFY-400
  <- FORGEOS-V1-WORLD-400

FORGEOS-V1-WORKSTATION-400
  <- SESSION-300, PROJECT-300, CODE-300, TERMINAL-300, GIT-300, WORLD-300, DIST-300

FORGEOS-V1-VERIFY-400
  <- CODE-400, SOURCE-400, NYX-400, AGENT-400, VERIFY-300

FORGEOS-V1-WORLD-400
  <- WORLD-300, RECOVERY-300
```

The detailed node sections above are canonical when this compact index and a node
section ever disagree.

---

## 8. Initial frontier

The repository currently contains planning documents only. No product behavior is
assumed from names or plans.

Current frontier:

```text
ACTIVE_SKILL=FORGEOS-V1-ARCH-000
ACTIVE_BLOCKER=THE_REPOSITORY_DOES_NOT_YET_CONTAIN_THE_V1_RUST_WORKSPACE_AND_AUTHORITY_CRATE_SKELETON
ACTIVE_SLICE=FORGEOS-V1-ARCH-000-SLICE-001

AVAILABLE
  none while the root skill is active

LOCKED
  every other node
```

After `FORGEOS-V1-ARCH-000` closes, the router may expose independent foundation
nodes according to direct prerequisites and conflict rules. It must not activate
them automatically.

---

## 9. Skill worksheet update fields

When a node changes state, update its section and maintain the following record in
the future router or skill registration document:

```yaml
skill_id: FORGEOS-V1-...
state: LOCKED
owner: null
source_repository: Forge_OS_V1
source_revision: null
active_slice: null
first_blocker: null
user_acceptance_status: NOT_READY
closure_and_spec: null
user_guide_source: null
last_verified_at: null
invalidated_by: null
```

This worksheet records capability status. It does not select work independently of
the future workflow authority and router.

---

## 10. V1 closure experiment preview

The separate future closure document will define exact fixtures and operator steps,
but it must preserve this acceptance shape:

```text
clean supported Linux host or clean VM
  -> install ForgeOS package
  -> choose ForgeOS session at login
  -> register the ForgeOS repository
  -> open and edit a real Rust module
  -> receive real Rust diagnostics
  -> run formatter, build, and focused tests
  -> inspect Git status and diff
  -> ask local Nyx to inspect the active source
  -> approve one bounded Nyx command
  -> dispatch one bounded OpenAI coding task to an isolated worktree
  -> review and apply the exact returned patch
  -> rerun real validation locally
  -> commit the result
  -> restart or cold-login
  -> restore the project and records
  -> implement and commit a second real source change
  -> verify every V1 regression shield
  -> user explicitly approves First Armor
```

A failure identifies one first blocker. It does not earn partial release credit.

---

## 11. V1 explicit exclusions

These are not deferred V1 criteria. They are outside the First Armor product
contract and must not be pulled into an active V1 skill:

- custom Wayland compositor;
- standalone ForgeOS Linux distribution image;
- full cinematic project worlds;
- full product capability-tree runtime;
- multi-agent orchestration;
- broad language support beyond Rust;
- voice-first operation;
- remote distributed builds;
- enterprise collaboration;
- multiplayer project spaces;
- architecture archaeology;
- extension or theme marketplace;
- virtual reality;
- autonomous release authority;
- unrestricted shell or destructive AI authority.

V1 may create seams that later versions extend. It may not implement hollow stubs
or fake controls for these excluded capabilities.

---

## 12. Completion statement

ForgeOS V1 is complete only when:

```text
all Tier 0 foundations are CLOSED
  -> all Tier 1 mechanisms are CLOSED
  -> all Tier 2 systems are CLOSED
  -> all Tier 3 workflows are CLOSED
  -> all Tier 4 integrations are CLOSED
  -> self-hosting passes
  -> the full journey repeats after restart
  -> the user approves the product
  -> FORGEOS-V1-APEX-001 becomes RELEASE_EARNED
```

Until then, First Armor remains unearned.

The armor must work before it glows.
