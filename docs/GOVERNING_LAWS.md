# ForgeOS Governing Laws

Status: `ACTIVE`
Authority ID: `FORGEOS-GOVERNING-LAWS-V1`
Applies to: all ForgeOS repositories, versions, capabilities, slices, agents,
interfaces, tests, experiments, packaging, and release claims
Fresh-session header: `docs/ForgeOS_header.md`
Live execution authority: `docs/workflow/WORKFLOW_AUTHORITY.md`
Canonical V1 worksheet: `docs/versions/V1/FORGEOS_V1_FIRST_ARMOR_SKILL_TREE.md`
Nyx capability ownership authority: `docs/versions/V1/FORGEOS_NYX_SERVER_CAPABILITY_OWNERSHIP_CHEAT_SHEET.md`

---

## 0. Purpose

This document defines the permanent laws of ForgeOS.

It governs:

- subsystem authority;
- repository authority;
- source architecture;
- module size and scope;
- allowed automated guards;
- dependency and seam direction;
- canonical state and identity;
- real-tool integration;
- Nyx authority and limits;
- Forge World truthfulness;
- agent permissions;
- testing and anti-tampering;
- user acceptance;
- regression protection;
- skill closure;
- documentation records;
- packaging and release truth.

This document does not select active work. It does not activate a skill, choose a
slice, assign a repository, or award release credit.

The live workflow authority and execution router perform those jobs.

When a source implementation, agent proposal, visual surface, test, or lower
authority document violates these laws, the violating work is invalid even when
it compiles or appears to function.

---

## 1. Permanent product law

ForgeOS is a real developer operating environment whose spatial experience is
built on truthful software state.

```text
real source
  -> real tools
  -> recorded outcomes
  -> canonical Forge Core state
  -> truthful Forge World presentation
  -> user-observed behavior
```

ForgeOS must remain useful with every visual effect disabled.

Code, terminals, logs, diffs, diagnostics, and dense technical information use
clear, efficient two-dimensional surfaces. Spatial presentation may organize,
connect, and illuminate those surfaces. It may not make ordinary development
slower merely to preserve a theme.

The permanent product law is:

> The armor must work before it glows.

The following are forbidden substitutes for working product behavior:

- decorative terminals;
- fake build or test indicators;
- scripted agent dialogue presented as live reasoning;
- screenshots standing in for execution;
- renderer-owned project state;
- skill nodes disconnected from real capabilities;
- mock-only success presented as user functionality;
- external-IDE behavior presented as ForgeOS behavior;
- model prose presented as source or tool truth.

---

## 2. Authority map

### 2.1 Forge Core

Forge Core owns canonical project and development-workflow truth.

It owns:

- project identity;
- repository registration;
- workspace identity and durable workspace state;
- release targets;
- capability graphs;
- skill states;
- missions;
- experiments;
- blockers;
- active slices;
- proof receipts;
- invalidation;
- developer settings that belong to ForgeOS project state;
- canonical records consumed by Forge World and Nyx.

Forge Core must remain deterministic and effect-free except through declared
ports and adapters.

Forge Core may not own:

- rendering;
- terminal processes;
- Git process execution;
- filesystem implementation details;
- LSP or DAP transport;
- model transport;
- OpenAI transport;
- display-manager or compositor calls;
- package-manager execution;
- host process supervision.

### 2.2 `nyx_server`

Detailed current and future Nyx ownership is cataloged in
`docs/versions/V1/FORGEOS_NYX_SERVER_CAPABILITY_OWNERSHIP_CHEAT_SHEET.md`. Any
Nyx-facing patch must review that catalog before source edits.

`nyx_server` is the sole ForgeOS AI host and bounded AI operator.

It owns:

- local model hosting and discovery;
- AI sessions and conversation state;
- model-provider routing;
- context assembly;
- tool requests;
- agent permission requests;
- resumable human checkpoints;
- AI run records;
- tool audit records;
- remote OpenAI requests;
- remote-agent handoff and result intake;
- model and agent execution policy inside its declared authority.

Nyx may inspect, explain, propose, route, and operate within granted authority.

Nyx may not:

- become canonical project-state authority;
- mark a skill `CLOSED` or `RELEASE_EARNED`;
- invent build, test, Git, repository, or capability state;
- bypass Forge Core to mutate canonical state;
- bypass the Developer Bridge to operate real tools;
- silently expand its permissions;
- treat model output as tool evidence;
- approve its own gated operation;
- create a parallel ForgeOS project model;
- be reimplemented inside ForgeOS as a second model host.

### 2.3 Forge World

Forge World owns presentation and user interaction.

It owns:

- the Bevy spatial shell;
- project environments;
- HUD surfaces;
- skill-tree visualization;
- visual status and animation;
- audio;
- input routing;
- workspace and panel arrangement;
- user-facing control surfaces;
- visual witness of source-owned state.

Forge World may display immutable view models and emit typed user intents.

Forge World may not:

- own canonical project truth;
- directly mutate Forge Core state;
- fabricate Git, build, test, Nyx, process, or capability state;
- infer health merely because a process exists;
- hide a failed or unknown state behind a green visual;
- recompute a source-owned result and present it as canonical;
- silently recover by using another IDE or external hidden path.

### 2.4 Developer Bridge

The Developer Bridge owns explicit integration with real development tools.

It owns adapters for:

- files and directories;
- PTYs and terminal processes;
- registered commands;
- Git;
- LSP;
- DAP;
- compilers;
- formatters;
- test runners;
- debuggers;
- package managers;
- external developer applications;
- host process and service interaction assigned to the bridge.

The Developer Bridge reports tool-owned results without changing their meaning.

It may normalize transport and versioned protocol details. It may not silently
translate failure into success, remove diagnostics, invent missing output, or
substitute a different command than the one recorded.

### 2.5 Real development tools

Real tools own their native behavior and results.

Examples include:

- Rust and Cargo;
- Git;
- rust-analyzer and other language servers;
- GDB, LLDB, and debug adapters;
- formatters;
- test frameworks;
- package managers;
- shell and terminal processes.

ForgeOS orchestrates these tools. It does not claim to replace their semantics
with visual simulations or AI interpretation.

### 2.6 Linux host and session

The Linux host owns:

- the kernel;
- hardware drivers;
- filesystems;
- networking;
- audio;
- process isolation;
- login and session infrastructure;
- host services;
- the installed package ecosystem.

ForgeOS may provide a dedicated session and later a compositor or distribution.
It does not write a new kernel, driver stack, compiler toolchain, or package
manager as part of the ForgeOS product plan.

### 2.7 Scripts and helper languages

Python, shell, and other helper languages may provide:

- repository utilities;
- fixtures;
- local orchestration;
- packaging helpers;
- derived analysis;
- development support.

They may not become:

- a hidden second Forge Core;
- a hidden Nyx implementation;
- the authoritative implementation of a native ForgeOS capability;
- an official product path claimed as Rust-owned behavior;
- a substitute for the registered experiment or public path.

---

## 3. Repository authority law

Every repository has one source authority.

The newest clean repository archive explicitly supplied by the user is the sole
source authority for that repository in the current conversation.

All older archives and extracted copies are superseded and ignored.

Archive names, base numbers, and SHA-256 values are intake context only. They are
not scheduling authority and may not stall work after the current source is
verified.

A patch belongs to exactly one repository.

ForgeOS and `nyx_server` remain separate source authorities unless an explicit,
versioned, user-approved vendoring or merge decision changes that boundary.

No repository may quietly absorb another repository's responsibilities.

Cross-repository work requires:

- an explicit baton handoff;
- the target repository's current source;
- a target-repository-only patch;
- declared input and output contracts;
- independent validation in each repository.

---

## 4. Canonical identity and state law

Canonical identity must be stable, explicit, and versioned.

Canonical ForgeOS state must never derive identity from:

- wall-clock time;
- hidden randomness;
- unstable map ordering;
- filesystem discovery order;
- worker arrival order;
- renderer timing;
- window-layout timing;
- model wording;
- locale;
- host-specific display formatting.

Timestamps may exist as descriptive metadata. They may not be the sole identity
of a project, repository, mission, capability, experiment, run, blocker, slice,
artifact, or proof receipt.

Canonical records require:

- stable IDs;
- stable ordering;
- versioned schemas;
- explicit migration when schemas change;
- SHA-256 identity for canonical artifacts where practical;
- source revision or equivalent source-state identity;
- unambiguous ownership.

External AI output is inherently variable.

Therefore:

```text
model output = proposal
real tool execution = evidence
Forge Core record = canonical project truth
user or declared release authority = closure authority
```

---

## 5. Rust workspace and module law

### 5.1 Scoped module law

Every authored source module must:

- own one coherent responsibility;
- use a name that states that responsibility;
- live under the subsystem that owns its truth;
- keep implementation private unless another subsystem has a declared contract;
- expose public APIs through the nearest `mod.rs` and crate `lib.rs`;
- split by behavior, state ownership, or seam boundary;
- remain understandable without unrelated subsystem knowledge.

Production catch-all modules are forbidden.

Forbidden names include:

```text
utils.rs
helpers.rs
misc.rs
stuff.rs
junk.rs
shared.rs without a bounded contract
common.rs without a bounded contract
manager.rs without naming what it manages
service.rs without naming what service it provides
```

A generic name is allowed only when its containing module path makes the exact
responsibility unambiguous and the owned behavior remains narrow.

### 5.2 Nested organization law

Large subsystems must use nested modules organized by responsibility.

Public routing follows:

```text
implementation module
  -> nearest mod.rs
  -> crate lib.rs
  -> versioned public contract
```

Callers should not import deep private implementation paths across subsystem
boundaries.

### 5.3 Module size law

Authored source modules follow these limits:

```text
preferred size:       approximately 500 physical lines or less
target maximum:       1000 physical lines
temporary ceiling:    1200 physical lines
```

Verifier behavior:

```text
0-1000 lines      PASS
1001-1200 lines   BREATHING_ROOM_WARNING
1201+ lines       HARD_FAIL
```

The 1200-line range exists only to allow safe splitting during an active slice.
It is not an acceptable steady state or a design target.

No skill may close while any authored source module remains above 1000 physical
lines.

The verifier covers authored Rust source, including separately authored source
test modules. It excludes generated, vendored, cached, target, and third-party
source.

### 5.4 No line-count gaming

The module-size law may not be bypassed by:

- compressing multiple statements onto one physical line;
- removing normal formatting;
- hiding source in macros solely to reduce file lines;
- moving behavior into generated code without a real generation requirement;
- creating numbered shards such as `parser_part_1.rs` and `parser_part_2.rs`;
- splitting by arbitrary line ranges;
- moving unrelated code into a generic helper module;
- creating one-line include wrappers around oversized authored source.

Every split must produce coherent, named responsibilities and improve the
architecture rather than merely satisfy the counter.

### 5.5 Workspace authority shape

The exact crate names may evolve through explicit migrations, but V1 authority
must remain equivalent to:

```text
forge-protocol       shared versioned IDs, commands, events, errors, and messages
forge-core           pure canonical project and capability state
forge-project        project registration and persistence adapters
forge-session        session and managed-service lifecycle
forge-bridge         explicit adapters to real development tools
forge-terminal       PTY and registered-command execution
forge-git            Git inspection, mutation, and worktree control
forge-editor         buffers, parsing, language intelligence, and editor state
forge-nyx-client     nyx_server protocol and lifecycle integration
forge-world          Bevy shell, HUD, spatial presentation, and user interaction
forge-app            composition root and executable
forge-guards         approved structural guards only
```

The composition root may connect subsystems. It may not become a second domain
layer containing business logic that belongs elsewhere.

---

## 6. Pure Forge Core law

Forge Core may depend only on pure domain and protocol crates.

It must not import or directly call:

- Bevy or renderer APIs;
- PTY or shell libraries;
- Git process adapters;
- filesystem implementation APIs;
- LSP or DAP clients;
- Nyx transport clients;
- OpenAI clients;
- desktop-session or display-manager APIs;
- host package-manager APIs;
- network clients;
- process supervisors;
- system clocks as identity sources.

Effects enter Forge Core through declared commands and ports.

Effect results return through versioned domain outcomes and events.

Forge Core logic must remain testable without:

- a running desktop session;
- a renderer;
- Git installed;
- a local model;
- network access;
- external services;
- a real filesystem, except where a pure serialized fixture is explicitly the
  subject of the test.

---

## 7. Dependency and seam law

Dependencies must point from composition and adapters toward pure contracts and
core domain logic, never backward from the core into effects or presentation.

The legal conceptual direction is:

```text
forge-app
  -> Forge World / session / project / tool adapters / Nyx client
  -> forge-protocol and Forge Core ports
  -> forge-core
```

Required laws:

- Forge World depends on immutable view contracts, not private Core state.
- Forge World emits typed intents, not direct state mutation.
- Developer Bridge implements declared ports; Core does not import adapters.
- Nyx integration remains behind the declared Nyx client contract.
- ForgeOS does not call model providers around Nyx.
- Real-tool adapters do not depend on Forge World.
- Presentation does not become a shared dependency of domain crates.
- The composition root wires dependencies but does not own canonical behavior.
- Shared protocol crates contain contracts, not hidden subsystem implementation.

A seam is invalid when it:

- leaks private implementation types across authority boundaries;
- allows two owners to mutate the same canonical state;
- reports an effect without its source identity;
- silently retries through a different implementation;
- translates failure into success;
- creates circular authority;
- requires the consumer to reconstruct source truth.

---

## 8. Allowed automated guard law

ForgeOS permits only these non-behavior automated guard classes:

1. authored Rust module-size verification;
2. Forge Core purity verification;
3. declared cross-subsystem seam and dependency-direction verification;
4. structured skill-graph and router integrity verification.

These guards may verify:

- physical source line counts;
- forbidden dependency edges;
- forbidden imports;
- Core purity;
- no parallel Nyx host;
- no presentation-owned canonical state path;
- no unresolved skill prerequisite IDs;
- no dependency cycles;
- active-skill limits;
- active-skill path or contract conflicts;
- required structured router fields;
- required closure-record file presence for a closed structured skill record.

These guards do not prove product behavior.

A guard may not:

- parse prose to decide that a behavior exists;
- award skill closure;
- infer user acceptance;
- scan Markdown status words as product truth;
- turn document wording into CI authority;
- replace a behavioral test;
- grow into a general architecture oracle;
- silently rewrite source or project state.

There will be no documentation CI, documentation prose verifier, checklist
parser, status-word scanner, heading validator, Markdown test, Git-state gate, or
formatting gate that claims to prove a product capability. Canonical CI is limited
to behavior tests, golden locks, and structural guards declared in `ci/master.yaml`
and executed by `scripts/run_ci.py`.

---

## 9. Real-path and no-shortcut law

Every capability must work through the real registered ForgeOS path named by its
contract.

A skill is invalid when its claimed behavior depends on:

- a hidden helper not used by the product;
- fixture-only execution;
- hardcoded sample data;
- precomputed command, Git, build, or test output;
- manual artifact editing;
- silent fallback to another IDE;
- a different command than the user was shown;
- an undeclared shell escape;
- an undeclared remote service;
- renderer-owned status;
- scripted agent responses;
- bypassing Nyx for model calls;
- bypassing the Developer Bridge for real-tool calls;
- bypassing Forge Core for canonical state changes;
- a mock standing in for the user-facing implementation.

Mocks and fixtures may test mechanics. They may not be the only evidence for a
user-facing or integrated capability.

Proof artifacts support a claim. They do not replace the behavior being claimed.

---

## 10. No parallel truth law

Before adding a new implementation path:

```text
inspect current source
  -> identify the existing authority
  -> adopt existing behavior where valid
  -> prove the existing path
  -> extend through a versioned contract
  -> migrate explicitly when required
  -> remove or reject the replaced path
```

Forbidden parallel authorities include:

- a second project registry;
- a second workspace-state model;
- a second capability-state store;
- a second Nyx host;
- a second Git abstraction for the same product path;
- a second command runner for the same authority;
- a second editor-buffer authority;
- a second session lifecycle;
- a UI-owned copy of canonical state;
- a test-only implementation presented as production behavior.

Temporary migration overlap is allowed only when:

- one migration skill explicitly owns it;
- both paths are named;
- write authority remains singular;
- compatibility behavior is tested;
- the old path has an explicit removal condition;
- closure removes or permanently quarantines the superseded path.

---

## 11. Agent and permission law

AI agents are workers, not authorities.

Every agent action must be:

- attributable to one run;
- scoped to one repository or declared multi-repository handoff;
- bounded by declared tools;
- bounded by declared writable paths;
- bounded by declared public contracts;
- logged;
- reviewable;
- resumable after a human checkpoint where applicable;
- limited by an explicit spending policy for remote inference.

Permission classes must remain distinguishable:

```text
OBSERVE
NAVIGATE
RUN_APPROVED_COMMANDS
EDIT_DECLARED_PATHS
APPLY_REVIEWED_PATCH
CREATE_ISOLATED_WORKTREE
REQUEST_DESTRUCTIVE_ACTION
SPEND_DECLARED_REMOTE_BUDGET
EXTERNAL_PUBLISH_OR_DEPLOY
```

A higher permission may not be inferred from a lower one.

Human approval must resume the exact suspended action with immutable payload
identity. Approval may not ask the model to reconstruct or reinterpret the
operation after the fact.

Nyx and remote agents may not:

- approve their own checkpoints;
- expand allowed paths after editing begins;
- hide tool calls;
- mutate outside the active worktree;
- spend beyond the declared budget;
- claim commands ran when they did not;
- merge or publish without declared authority;
- award skill or release closure.

Destructive actions always require explicit human approval unless a future
version defines a narrower preapproved sandbox whose destruction cannot affect
canonical or user-owned data.

---

## 12. Registered command law

Product-level command execution must use declared command identities or an
explicit operator shell.

A registered command must define:

- stable command ID;
- display name;
- executable and arguments or a versioned command template;
- working-directory policy;
- environment policy;
- timeout policy;
- cancellation behavior;
- authority class;
- output capture policy;
- expected result classification;
- whether human approval is required.

Nyx may request registered commands by ID and bounded parameters.

Nyx may not synthesize arbitrary shell strings and present them as registered
commands.

The operator may use an explicit shell. The product must clearly distinguish an
operator shell command from a registered command and preserve its exact text and
result.

Displayed command text, executed command identity, working directory, and
captured result must agree.

---

## 13. Testing law

Tests and product-path experiments have different jobs.

### 13.1 Tests prove

- local mechanics;
- validation;
- invariants;
- ordering;
- error handling;
- serialization;
- compatibility;
- contract behavior;
- regression locks;
- failure classification.

### 13.2 Product-path experiments prove

- the real ForgeOS path uses the capability;
- integrated behavior exists;
- the source-owned result reaches the user-facing surface;
- required controls separate real behavior from coincidence or fallback;
- the user can complete the declared workflow;
- the claim remains within its stated scope.

A unit test is not automatically an integrated product proof.

A successful demo is not automatically a regression lock.

### 13.3 Required negative and failure behavior

Every skill must declare the negative and failure paths required by its scope.

Examples include:

- missing repository;
- dirty worktree;
- denied permission;
- unavailable Nyx service;
- incompatible protocol;
- failed command;
- cancelled process;
- malformed patch;
- failed patch apply;
- failed test;
- session restart;
- unavailable local model;
- exhausted remote budget.

A capability is incomplete when its expected failure behavior is missing,
misclassified, hidden, or destructive.

### 13.4 Assistant and operator validation law

ForgeOS development may be coordinated from an assistant environment that does not
contain the Rust toolchain, desktop-session services, GPU stack, local model
runtime, or other host dependencies required by the active skill.

Absence of those tools in the assistant environment is not a product defect and is
not a valid reason to block source implementation when the user can execute the
required validation on the canonical development host.

The permitted split is:

```text
assistant
  -> inspects source
  -> writes the bounded patch
  -> runs available structural, diff, graph, and patch-application checks
  -> declares exactly what was and was not executed
  -> hands off exact validation commands

operator
  -> applies the patch
  -> runs the unavailable compiler, formatter, test, guard, boot, display-session,
     hardware, or integration commands
  -> returns exact results
```

The router records this intermediate state as:

```text
OPERATOR_VALIDATION_PENDING
```

That state keeps the skill `ACTIVE`. It awards no proof and no closure, but it may
not be treated as `BLOCKED_ENVIRONMENT_TOOLCHAIN_MISSING` merely because the
assistant sandbox lacks the toolchain.

The assistant may rely on user-returned logs and results as operator-executed
evidence, but must label them honestly and must never state that it personally ran
those commands.

If operator validation fails, the failure becomes the next first blocker. If it
passes, the skill may continue through its remaining user-acceptance and closure
requirements.

---

## 14. Test and guard anti-tampering law

A new skill may not close by weakening the system that judges it.

Forbidden closure tactics include:

- deleting a failing test;
- skipping or ignoring a failing test;
- weakening an assertion;
- increasing a tolerance without an approved contract change;
- reducing fixture coverage;
- changing a golden solely to match broken output;
- reclassifying a failure as expected without a contract change;
- adding an allowlist solely to silence a guard;
- raising a verifier threshold;
- excluding authored source from line counting without a real generated or
  third-party classification;
- bypassing a seam guard through a generic crate;
- changing a test to call a helper instead of the real public path;
- disabling prior regression commands;
- hiding failure output from the user.

Changing an existing behavioral lock requires an explicit migration or contract
change owned by an active skill and approved by the user.

The change must explain:

- why the prior contract is no longer correct;
- what replaces it;
- which previously closed skills are affected;
- whether any skill becomes `INVALIDATED`;
- how compatibility and user behavior are preserved or intentionally migrated.

---

## 15. Regression law

A new skill may not weaken, bypass, disable, reinterpret, or silently replace a
previously closed skill.

Every skill inherits the requirement:

```text
all previously CLOSED skills remain green
```

When a source or contract change disproves an earlier skill:

1. the earlier skill becomes `INVALIDATED` immediately;
2. dependent skills lose any closure that relies on the invalidated edge;
3. the active skill may not close while the regression remains;
4. the original user acceptance path is repeated when user behavior changed;
5. closure records are amended through an explicit invalidation or migration
   record, never silently rewritten as though the old proof never existed.

A regression discovered during unrelated work is not parked as optional cleanup.
It becomes the current blocker when it invalidates the active path or existing
release truth.

---

## 16. No deferred completion law

ForgeOS has no partial closure state.

The following are not valid completion states:

```text
PARTIAL
MOSTLY_DONE
DEFERRED
TEMPORARILY_ACCEPTED
CLOSE_ENOUGH
FUNCTIONAL_EXCEPT
POLISH_LATER when polish is part of the skill contract
```

If a criterion is required by a skill, it is completed in that skill.

A skill may depend on a named prerequisite. It may not depend on:

- unnamed future cleanup;
- a hidden migration;
- a promised rewrite;
- a temporary mock that remains in the public path;
- a later documentation pass for missing user instructions;
- a later usability pass when usability belongs to the skill;
- a later failure-path pass;
- a later module split required to meet the 1000-line closure law.

A missing criterion leaves the skill `ACTIVE` or `BLOCKED`.

---

## 17. User-facing functionality and acceptance law

Every skill must declare one real user acceptance route.

For internal foundation skills, the user acts as the operator and witnesses the
real result through:

- a command;
- a controlled failure;
- state inspection;
- an integrated product surface;
- a legal and illegal seam fixture;
- a restart or recovery path;
- a source or artifact comparison.

For user-facing skills, the user performs the actual workflow inside ForgeOS.

A skill may reach source-proved or acceptance-ready state through tests and
recorded evidence. It may reach `CLOSED` only after the user exercises and
approves the behavior within the current version's scope.

No model, agent, test suite, document, screenshot, or proof receipt may approve
on the user's behalf.

User approval means the declared behavior is functionally acceptable within the
current version. It does not mean every later-version feature exists.

Proof supports acceptance. Proof does not count as the user-facing function.

---

## 18. Skill closure law

A skill may become `CLOSED` only when all of the following are true:

```text
direct prerequisites are CLOSED
  -> the real source path exists
  -> focused mechanical tests pass
  -> required negative and failure paths pass
  -> the public or operator-facing behavior works
  -> the user exercises the behavior
  -> the user approves the behavior within current-version scope
  -> all previously CLOSED skills remain green
  -> approved structural guards pass
  -> no authored source module exceeds 1000 physical lines
  -> no parallel source of truth was introduced
  -> required closure records exist and are complete
  -> the canonical worksheet and router close the exact skill atomically
```

Closure is atomic.

The worksheet may not say `CLOSED` before the source, tests, user acceptance,
guards, and records are complete.

A patch, successful compile, green unit test, proof receipt, screenshot, or agent
claim is insufficient by itself.

Release credit is separate from skill closure. Only the release authority and
activated closure experiment may award `RELEASE_EARNED`.

---

## 19. Required closure records

Every closed V1 skill must create exactly these two records:

```text
docs/versions/V1/skills/<SKILL_ID>/CLOSURE_AND_SPEC.md
docs/versions/V1/skills/<SKILL_ID>/USER_GUIDE_SOURCE.md
```

### 19.1 `CLOSURE_AND_SPEC.md`

This record must contain:

- exact skill ID and capability statement;
- owning subsystem and repository;
- direct prerequisites;
- source revision or verified source-state identity;
- files and public contracts changed;
- final module ownership and responsibility;
- tests and exact commands run;
- negative and failure paths exercised;
- approved structural guards run;
- real product or operator path exercised;
- user acceptance procedure;
- user approval result;
- regression commands and results;
- proof artifacts and hashes where applicable;
- supported claim;
- explicit non-claims;
- known limits that are outside the skill's declared scope;
- confirmation that no required criterion was deferred;
- closure date as descriptive metadata only.

This record documents why closure was earned. It does not award closure by
existing.

### 19.2 `USER_GUIDE_SOURCE.md`

This record must contain all information needed to produce the built-in Forge
Guide and public website documentation for the skill.

Where applicable, it must include:

- what the capability does;
- who it is for;
- how to access it;
- required setup;
- step-by-step use;
- available options;
- variations;
- customization;
- keyboard controls;
- permission prompts;
- expected outputs;
- status meanings;
- normal failure states;
- recovery steps;
- limitations within the current version;
- safety or destructive-action warnings;
- interaction with Nyx;
- interaction with real tools;
- examples grounded in the real product.

The prose does not need final marketing polish at skill closure. It must be
factually complete enough that later work is presentation, organization, and
wording only.

Missing user information blocks closure when the skill has user-facing behavior,
configuration, options, or recovery requirements.

### 19.3 No documentation verifier

Closure-record completeness is reviewed as part of skill closure and may be
represented by structured required fields in the future router.

There will be no prose-quality CI, Markdown content parser, wording verifier, or
document-driven behavior test.

---

## 20. Parallel work law

At most three skills may be active globally and at most one skill may be active
per lane.

Parallel skills are legal only when:

- all direct prerequisites are closed;
- neither skill directly or transitively depends on another active skill;
- they do not alter the same public contract;
- they do not write overlapping paths;
- they do not share one unstable prerequisite;
- they do not require incompatible migrations;
- each uses an isolated branch or worktree;
- each has its own first blocker and acceptance path;
- merging one cannot invalidate the assumptions of the other.

When two skills discover a shared missing prerequisite:

1. both child skills pause;
2. the shared prerequisite becomes a registered skill or existing node;
3. only the prerequisite may proceed;
4. the children are revalidated after the prerequisite closes.

Only one active skill may change a given public contract or authority boundary.

Parallel work is an optimization, never permission to build on unstable ground.

---

## 21. Persistence, migration, and compatibility law

Canonical persisted state must use versioned schemas.

Every schema change must declare:

- source version;
- target version;
- migration direction;
- whether rollback is supported;
- compatibility expectations;
- failure behavior;
- backup or recovery requirements;
- affected skills and user documentation.

Silent destructive migration is forbidden.

A migration may not:

- drop unknown data without an explicit contract;
- rewrite user projects merely because they were opened;
- hide a partial failure;
- leave two writable canonical formats;
- claim success before persistence and reopen behavior are verified.

Where V1 promises reopen, restart, or recovery, the actual persisted path must be
used in acceptance.

---

## 22. Error and observability law

Every externally meaningful operation must produce an inspectable result.

Errors must be:

- typed or stably classified;
- attributable to a subsystem and operation;
- associated with the relevant project, repository, command, run, or service;
- visible to the user when action is required;
- preserved in run history where the contract requires it;
- distinguishable from cancellation, denial, timeout, incompatibility, and
  internal failure.

Forge World may summarize errors. The exact source result must remain reachable.

Logs and diagnostics may not expose secrets, model credentials, authentication
tokens, or private external payloads beyond declared safe fields.

Observability may report behavior. It may not become a second state authority.

---

## 23. Security and trust-boundary law

ForgeOS is local-first, but local execution is not automatically trusted.

Trust boundaries include:

- user-owned repositories;
- project configuration;
- Nyx tool requests;
- remote model output;
- downloaded patches;
- external commands;
- plugins and extensions;
- environment variables;
- credentials;
- build scripts from opened repositories.

Required laws:

- remote model output is untrusted input;
- patches are reviewed before application unless an explicit scoped policy says
  otherwise;
- tool authority is least-privilege and mission-scoped;
- credentials are never inserted into model context without explicit policy;
- secrets are redacted from logs and proof artifacts;
- repository boundaries are enforced;
- external publishing, deployment, and spending require explicit authority;
- opened repositories may not silently gain system-wide privileges;
- destructive operations expose their target and consequence before approval.

Security mechanisms must not fabricate success or hide blocked behavior.

---

## 24. Performance and usability law

ForgeOS is a working environment, not an animation showcase.

Every user-facing capability must remain responsive enough for sustained daily
use within its declared V1 scope.

Required laws:

- keyboard access is never slower than mandatory spatial navigation;
- common actions do not require walking through virtual space;
- animations are interruptible or bypassable;
- visual effects may be reduced or disabled;
- long-running operations remain cancellable when the underlying tool permits;
- background work does not freeze the editor or terminal;
- unknown or stale status is displayed honestly;
- recovery does not require discarding unrelated user work;
- flat, dense work surfaces remain available for coding, terminal, Git, logs,
  and diagnostics.

A visually impressive path that is materially worse than the direct development
path does not satisfy the capability.

---

## 25. V1 and later-version boundary law

V1 exists to earn a raw but real daily development environment.

V2, V3, and V4 capabilities may shape versioned seams. They may not enter V1 as:

- hollow stubs;
- speculative frameworks;
- unused abstractions;
- fake visual surfaces;
- placeholder autonomous-agent systems;
- premature compositor work;
- broad multi-language support unrelated to the active V1 path;
- marketplace, collaboration, voice-first, VR, or cinematic-world work.

A V1 skill may create a narrow extension seam when the current capability
requires it. The seam must have a present V1 owner, present V1 consumer, and
present V1 tests.

Future possibility is not present justification.

---

## 26. Release truth law

Skill closure, product aggregation, and release closure are separate claims.

```text
skill CLOSED
  != integrated user journey complete
  != V1 RELEASE_EARNED
```

V1 may be called complete only when:

- every required V1 skill is closed;
- integrated V1 routes remain green;
- the declared V1 closure experiment passes;
- ForgeOS installs or launches through the supported V1 distribution path;
- the ForgeOS session boots;
- a real ForgeOS or Nyx feature is implemented, reviewed, tested, and committed
  from inside ForgeOS without another IDE;
- required recovery behavior passes;
- approved structural guards pass;
- the user approves the full V1 journey;
- release records state exact supported behavior and non-claims.

Release authority may not close a missing skill by waiver, documentation, or
intent.

---

## 27. Change law for this document

These laws may evolve only through an explicit governing-law change.

A governing-law change must state:

- the exact law being changed;
- why the old law is insufficient or incorrect;
- the replacement text;
- affected skills, repositories, guards, tests, and closure records;
- whether existing closed skills require revalidation;
- whether the change expands or reduces authority;
- user approval.

A source slice may not casually edit this document to make its implementation
legal.

When a proposed implementation conflicts with these laws, the default action is
to change the implementation, not weaken the law.

---

## 28. Compact permanent law

```text
one source of truth per authority
  -> pure Core
  -> explicit seams
  -> real tools own real results
  -> Nyx operates but does not rule
  -> Forge World presents but does not invent
  -> tightly scoped modules
  -> prefer approximately 500 lines
  -> close nothing above 1000 authored lines
  -> hard fail above 1200
  -> no line-count gaming
  -> no parallel truth
  -> no hidden fallback
  -> no test weakening
  -> no deferred completion
  -> no regression of closed behavior
  -> real user acceptance is mandatory
  -> two complete closure records per skill
  -> no documentation CI
  -> proof supports behavior; proof is not behavior
  -> release credit is earned only through the real closure journey
```

ForgeOS earns trust by making the real path visible, bounded, testable, and
useful.
