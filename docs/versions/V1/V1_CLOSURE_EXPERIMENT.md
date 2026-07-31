# ForgeOS V1 First Armor Closure Experiment

Status: `DEFINED_INACTIVE`
Experiment ID: `FORGEOS-V1-CLOSURE-0001`
Release target: `FORGEOS_V1_FIRST_ARMOR`
Release apex: `FORGEOS-V1-APEX-001`
Activation authority: `docs/versions/V1/V1_EXECUTION_ROUTER.md`
Canonical worksheet: `docs/versions/V1/FORGEOS_V1_FIRST_ARMOR_SKILL_TREE.md`
Workflow authority: `docs/workflow/WORKFLOW_AUTHORITY.md`
Skill-tree method: `docs/workflow/SKILL_TREE_WORKFLOW_METHOD.md`
Permanent laws: `docs/GOVERNING_LAWS.md`
Mission authority: `docs/MISSION_FORGEOS.md`
Fresh-session authority: `docs/ForgeOS_header.md`

---

## 0. Purpose

This document defines the only V1 release-closure experiment permitted to award
`RELEASE_EARNED` to `FORGEOS-V1-APEX-001`.

The experiment answers one bounded question:

```text
Can ForgeOS First Armor be installed on its declared supported Linux host,
entered as a real login session, and used as the complete development
environment for two real verified source changes, including local Nyx guidance,
one bounded Nyx action, one denied action, one remote OpenAI coding task in an
isolated worktree, restart recovery, Git commit, and full V1 regression replay,
without using another IDE or hidden operator repair?
```

This is a release experiment, not a planning checklist, demonstration script,
screenshot exercise, documentation audit, or substitute for per-skill closure.

The experiment verifies that all already-closed V1 capabilities aggregate into one
usable product journey. It does not reopen the skill tree merely to create more
work, and it does not permit missing V1 behavior to be relabeled as a release
experiment task.

---

## 1. Authority and release-credit law

The following authority order applies inside this experiment:

```text
current source and real tool behavior
  -> docs/GOVERNING_LAWS.md
  -> docs/workflow/WORKFLOW_AUTHORITY.md
  -> docs/versions/V1/V1_EXECUTION_ROUTER.md
  -> this closure experiment
  -> canonical V1 skill contracts and closure records
  -> operator notes and descriptive artifacts
```

This experiment alone may award V1 release credit, but it may do so only after all
non-apex V1 skills have independently earned closure.

This experiment may not:

- close an unfinished lower-tier skill;
- waive a missing user-acceptance path;
- replace a failed skill regression with a release-level exception;
- change a skill contract to make the release pass;
- weaken a test, guard, negative path, or user-facing requirement;
- treat proof artifacts as the user-facing function;
- grant release credit from model prose, screenshots, logs, or documents alone;
- pull V2, V3, or V4 features into V1;
- require archive renaming, archive numbering, hash bookkeeping, or tar
  regeneration as a release criterion.

A failure identifies one first blocker and returns the baton to the owning skill or
subsystem. It awards no partial release credit.

---

## 2. Activation gate

The router may activate this experiment only when all of the following are true:

```text
PROGRAM_MODE=RELEASE_CLOSURE
ACTIVE_RELEASE_TARGET=FORGEOS_V1_FIRST_ARMOR
ACTIVE_CLOSURE_EXPERIMENT=FORGEOS-V1-CLOSURE-0001
SOURCE_WORK_AUTHORIZED=YES
FORGEOS-V1-APEX-001=LOCKED
ALL_OTHER_REGISTERED_V1_SKILLS=CLOSED
CLOSED_NON_APEX_SKILL_COUNT=66
ACTIVE_SOURCE_SKILLS=[]
INVALIDATED_V1_SKILLS=[]
UNRESOLVED_V1_PREREQUISITES=[]
```

Before activation, verify:

- all 66 non-apex skill IDs are `CLOSED` in the canonical worksheet and router;
- every closed skill has both required closure records;
- no skill closure depends on deferred behavior;
- no active worktree contains unreviewed source changes;
- the authoritative ForgeOS and required Nyx repositories are clean;
- the consolidated V1 regression command set is declared;
- the supported host profile is declared by the closed distribution skill;
- the installation artifact is produced from the exact closure-candidate source;
- the two closure source changes are registered and were not preimplemented;
- the user has approved the two bounded change candidates for this experiment.

If any item is false, the experiment remains inactive.

---

## 3. Closure fixture identities

The following fixture names are permanent. Their concrete values are resolved from
closed V1 contracts and recorded at activation.

### 3.1 Supported host profile

`SUPPORTED_HOST_PROFILE_V1` is the exact first Linux host profile declared by the
closure record for `FORGEOS-V1-DIST-200` and inherited by
`FORGEOS-V1-DIST-300` and `FORGEOS-V1-WORKSTATION-400`.

It must identify at minimum:

```yaml
fixture_id: SUPPORTED_HOST_PROFILE_V1
architecture:
distribution:
distribution_version:
display_manager:
graphics_session:
minimum_cpu:
minimum_memory:
minimum_storage:
gpu_requirement:
required_system_packages:
unsupported_host_conditions:
```

The experiment may run on physical hardware or a virtual machine only when that
execution environment satisfies the declared supported profile.

A VM is not automatically a weaker proof. It becomes invalid when the VM bypasses
a required session, GPU, input, service, package, or persistence behavior.

### 3.2 Clean host fixture

`CLEAN_HOST_V1` is a clean installation or clean VM snapshot of
`SUPPORTED_HOST_PROFILE_V1` with:

- no prior ForgeOS installation;
- no prior ForgeOS user configuration;
- no registered ForgeOS projects;
- no Nyx session state from an earlier run;
- no preinstalled ForgeOS development extension or hidden helper;
- no source patch prepared outside the experiment;
- network and credentials configured only as declared by the fixture;
- a documented recovery route back to the normal host session.

The snapshot or machine preparation method is descriptive metadata. The user must
still perform the real installation and login journey.

### 3.3 Closure source fixture

`FORGEOS_V1_CLOSURE_SOURCE` is a clean source checkout at the exact closure
candidate revision.

It must declare:

```yaml
fixture_id: FORGEOS_V1_CLOSURE_SOURCE
repository: Forge_OS_V1
revision:
branch:
worktree_clean: true
submodule_or_external_dependency_state:
nyx_repository_revision:
installation_artifact_source_revision:
```

The newest clean user-supplied repository archive remains the source authority for
planning and patch turns before the experiment. The closure run itself operates on
real Git repositories and exact revisions inside the supported host.

Archive names and numeric suffixes have no release authority.

### 3.4 Project fixture

`FORGEOS_V1_SELFHOST_PROJECT` is the real ForgeOS source repository registered as
a ForgeOS project after installation.

It must use the real V1 paths for:

- project registration;
- file browsing;
- editor access;
- Rust language intelligence;
- terminal and registered commands;
- Git state and diffs;
- Nyx project context;
- agent worktree creation;
- validation result capture;
- workspace restore.

A duplicate fixture-only project that bypasses the real ForgeOS source is invalid.

### 3.5 Local model fixture

`NYX_LOCAL_MODEL_V1` is one supported local model configuration closed under the
Nyx V1 skills.

It must declare:

```yaml
fixture_id: NYX_LOCAL_MODEL_V1
backend:
model_identifier:
model_revision_or_digest:
context_limit:
tool_support:
hardware_profile:
expected_startup_limit:
```

The local model may explain and propose. Project facts must come from Forge Core,
Nyx tool results, Git, files, commands, or other authoritative sources.

### 3.6 Remote coding fixture

`OPENAI_CODING_PROVIDER_V1` is the exact OpenAI-backed heavy coding path closed by
`FORGEOS-V1-AGENT-400`.

It must declare:

```yaml
fixture_id: OPENAI_CODING_PROVIDER_V1
provider: OpenAI
model_or_agent:
invocation_path:
maximum_run_budget:
maximum_tool_authority:
worktree_policy: ISOLATED_REQUIRED
patch_application_policy: USER_APPROVAL_REQUIRED
```

Secrets must never appear in closure artifacts, logs, screenshots, or guide
source.

---

## 4. Closure source-change registration

The closure experiment uses exactly two real source changes.

```text
CHANGE_A = manual ForgeOS source change
CHANGE_B = agent-assisted ForgeOS or Nyx source change
```

At least one change must modify ForgeOS source. The second may modify ForgeOS or
`nyx_server` when the Nyx repository is registered and the cross-repository seam
is already a closed V1 behavior.

Each change must be registered before the first closure journey begins.

```yaml
change_id:
target_repository:
starting_revision:
capability_or_defect_statement:
user_or_operator_visible_result:
why_the_change_is_real:
why_the_change_is_inside_v1_scope:
why_it_does_not_redefine_a_closed_skill:
allowed_paths:
forbidden_paths:
public_contracts_touched:
required_commands:
negative_or_failure_path:
regression_commands:
expected_commit_count:
```

A valid closure change must:

- alter real authored source;
- produce an observable user or operator result;
- use the public V1 development path;
- be small enough for one bounded development journey;
- remain inside the existing V1 product contract;
- preserve every closed skill;
- satisfy the module-size and seam laws;
- be unimplemented at the registered starting revision;
- require genuine editing, validation, diff review, and commit.

A closure change is invalid when it is:

- documentation-only;
- comment-only;
- whitespace-only;
- generated-file-only;
- test-only with no real behavior;
- fixture-only;
- a prewritten patch hidden from the user;
- already present in source;
- a deliberate regression created merely to repair it;
- a V2, V3, or V4 feature;
- a broad refactor with no bounded observable outcome;
- a change that requires weakening an existing lock;
- a change that exists solely to satisfy this experiment mechanically.

If a registered change is discovered to be preimplemented, invalid, or outside
scope, stop before editing and register a new valid candidate. Do not silently
substitute work during the run.

---

## 5. Permitted and forbidden external tools

### 5.1 Permitted outside ForgeOS

The following may be used only where the product contract requires them:

- the host display manager for selecting the ForgeOS session;
- the operating system package installer before entering ForgeOS;
- firmware, boot, VM, or host controls needed to start the supported host;
- a browser for external documentation, provider authentication, or API account
  configuration;
- a password manager for entering credentials;
- emergency host recovery after the experiment has already been classified
  `FAILED` or `INVALID`.

### 5.2 Forbidden during the development journeys

After entering the ForgeOS session, the following are forbidden for source work:

- another IDE or code editor;
- an external terminal used to edit, build, test, Git-operate, or repair source;
- an external Git client;
- manual patch application outside ForgeOS;
- direct file mutation through the host desktop;
- a separate chat application used as the authoritative Nyx or agent interface;
- hidden scripts that precompute success state;
- operator changes to databases, audit records, or result files;
- manual service repair that is not an exposed ForgeOS recovery path.

If any forbidden tool is required, the current journey fails. Repair may occur
after classification, but the affected journey must be repeated from its declared
start boundary.

---

## 6. Required command and regression registry

Before activation, the router must register the exact command set used by the
closure experiment.

```yaml
closure_command_registry:
  formatter:
  build:
  focused_change_a_tests:
  focused_change_b_tests:
  forgeos_regression:
  nyx_regression:
  module_size_guard:
  forge_core_purity_guard:
  seam_guard:
  router_integrity_guard:
  package_build:
  package_verify:
```

The registry may reference consolidated scripts or tool profiles only when those
paths execute the real commands and preserve exact exit codes and output.

The closure runner may not parse Markdown prose to discover commands or award
behavior. Command registration is structured execution data.

A command result is valid only when it records:

- exact command identity;
- working directory;
- source revision before execution;
- start and completion state;
- exit code;
- cancellation state;
- stdout and stderr location;
- resulting source revision or dirty-state identity;
- whether the displayed Forge World status matches the authoritative result.

A stale success from an earlier revision may not satisfy a later step.

---

## 7. Preflight phase

The experiment begins with preflight, not installation.

### 7.1 Authority preflight

Verify:

- this document is `ACTIVE` in the router;
- all 66 non-apex V1 skills are closed;
- no V1 skill is invalidated;
- no unrelated skill is active;
- both closure changes are registered;
- the supported host profile is resolved;
- the command registry is complete;
- the user understands the allowed and forbidden external-tool boundary;
- the user is prepared to perform both journeys personally.

### 7.2 Source preflight

Verify through real Git and source inspection:

- ForgeOS source is clean;
- required Nyx source is clean;
- current revisions match the activation record;
- Change A and Change B are absent;
- no hidden patch file already implements either change;
- no agent worktree exists for Change B;
- no source module begins above the 1200-line hard ceiling;
- no unresolved dependency or build failure exists before the run.

### 7.3 Package preflight

Build the installation artifact from the exact closure source revision and verify:

- package identity resolves to the source revision;
- installation metadata identifies V1 First Armor;
- the package contains the declared ForgeOS session entry;
- required managed services are included or declared dependencies;
- no development-only path outside the package is required for launch;
- package verification passes before transfer to the clean host.

### 7.4 Clean-host preflight

Verify the clean host fixture has no prior ForgeOS state.

If any preflight edge fails, stop. Record the first blocker. Do not begin the
installation journey and do not award partial credit.

---

## 8. Journey A: clean install and manual source change

Journey A proves that First Armor can become a real workstation from a clean host
and support a complete manual Rust development loop.

### 8.1 Install ForgeOS

The user:

1. boots `CLEAN_HOST_V1`;
2. installs the exact ForgeOS closure package through the declared installation
   path;
3. receives a visible success or actionable failure result;
4. verifies the ForgeOS session is offered by the display manager;
5. does not manually edit installed session or service files.

Pass edge:

```text
installation completes through the supported path
  -> package identity matches closure source
  -> ForgeOS session is visible
  -> required services are installed
  -> normal host recovery remains available
```

### 8.2 Enter the ForgeOS session

The user:

1. selects ForgeOS in the display manager;
2. logs in without launching ForgeOS from another desktop session;
3. observes Forge World start;
4. observes Forge Core and Nyx service health;
5. confirms keyboard and pointer input work;
6. confirms a failure surface exists for any unhealthy service.

Pass edge:

```text
display-manager selection
  -> ForgeOS session starts
  -> required services reach truthful states
  -> project hangar is usable
  -> no external IDE or terminal bootstrap is required
```

### 8.3 Register the real ForgeOS source project

The user:

1. registers `FORGEOS_V1_SELFHOST_PROJECT` through ForgeOS;
2. confirms repository identity, branch, revision, and dirty state;
3. confirms declared commands are visible;
4. closes and reopens the project once;
5. verifies the same identity and command configuration returns.

The user also attempts one invalid registration outside the approved project
boundary. ForgeOS must reject it with an actionable result and without creating a
partial project record.

### 8.4 Verify the baseline

Before editing, the user runs through ForgeOS:

1. the formatter check or no-change formatter path;
2. the build command;
3. the relevant baseline focused tests;
4. the approved structural guards applicable to the current source.

The UI must display the exact current result. A baseline failure stops the journey
and routes to the owning closed skill as an invalidation or current blocker.

### 8.5 Use local Nyx for project inspection

The user:

1. selects `NYX_LOCAL_MODEL_V1`;
2. asks Nyx a project-specific question whose answer requires approved repository
   tools;
3. inspects the tool calls and source references;
4. confirms Nyx does not present unsupported model memory as project truth;
5. asks Nyx to summarize the registered Change A contract.

The answer must resolve to current source, project state, or tool evidence.

### 8.6 Approve one bounded Nyx action

The user asks Nyx to perform one safe registered action relevant to Change A,
such as repository search, focused command execution, or other already-closed V1
tool behavior.

The user:

1. sees the exact requested action and scope;
2. approves it;
3. observes execution through the real registered tool;
4. confirms the audit record contains request, approval, execution, and result;
5. confirms checkpoint approval resumed the exact suspended action.

Nyx may not reformulate approval into a broader request.

### 8.7 Deny one Nyx action

The user causes one action to require approval, then denies it.

Pass edge:

```text
action requests approval
  -> user denies
  -> action does not execute
  -> project state remains unchanged
  -> denial is visible and audited
  -> Nyx does not silently retry an equivalent action
```

### 8.8 Implement Change A manually

Using only ForgeOS development surfaces, the user:

1. opens the registered source files;
2. navigates using the file tree and symbol or search surfaces;
3. receives real Rust syntax and language-server diagnostics;
4. edits the bounded source behavior;
5. saves the files;
6. resolves in-scope diagnostics;
7. does not use another editor.

The user must exercise at least:

- open and save;
- repository search;
- go-to-definition or equivalent symbol navigation;
- visible diagnostics;
- multiple buffers when the change spans more than one file.

### 8.9 Validate Change A

The user runs:

1. formatter;
2. focused Change A tests;
3. build;
4. declared negative or failure-path test;
5. required structural guards;
6. applicable regression commands.

All results must be attached to the current source state. A failed command must
immediately replace any prior green aggregate status.

### 8.10 Review and commit Change A

The user:

1. inspects real Git status;
2. reviews the exact diff;
3. confirms no forbidden path changed;
4. stages only intended files;
5. commits with a truthful message;
6. independently verifies the resulting commit and clean state through ForgeOS;
7. records `CHANGE_A_COMMIT`.

Journey A passes only when the committed behavior works through its real user or
operator path and the user accepts that bounded result.

---

## 9. Restart boundary

After Journey A:

1. stop or complete all active commands;
2. close the project through ForgeOS;
3. log out of the ForgeOS session;
4. perform a cold login boundary as declared by the supported host profile;
5. select ForgeOS again through the display manager;
6. allow managed services to recover without external repair.

A cold login must recreate the session from persisted product state. Merely closing
and reopening one panel does not satisfy this boundary.

After login, verify:

- Forge World starts;
- Forge Core restores project identity;
- the active project or recent-project state is available;
- branch and revision identify `CHANGE_A_COMMIT`;
- the worktree is clean;
- command history and result provenance remain coherent;
- Nyx session or declared recoverable context returns according to its V1 contract;
- prior approval does not become standing authority for future actions;
- no stale active process is shown as running.

Failure routes to the first owning recovery, session, project, Nyx, or state skill.

---

## 10. Journey B: agent-assisted source change after cold login

Journey B proves that the restored environment can safely coordinate the declared
OpenAI coding path without giving the remote agent authority over the main
worktree or release result.

### 10.1 Reopen and verify the restored project

The user opens `FORGEOS_V1_SELFHOST_PROJECT` and verifies:

- repository identity;
- branch;
- `CHANGE_A_COMMIT`;
- clean worktree;
- registered commands;
- local Nyx availability;
- remote provider configuration status;
- current API budget limit.

### 10.2 Prepare the bounded Change B mission

The user asks Nyx to prepare Change B from its registered contract.

The mission packet must contain:

- exact repository and starting revision;
- Change B capability or defect statement;
- allowed and forbidden paths;
- relevant source context;
- public contracts touched;
- required commands;
- negative or failure path;
- regression commands;
- expected result format;
- maximum API budget;
- explicit non-authority to award completion.

The user reviews the packet before dispatch.

### 10.3 Create an isolated agent worktree

ForgeOS creates a dedicated worktree or equivalent isolated branch for Change B.

Pass edge:

```text
main worktree starts clean
  -> isolated worktree is created from declared revision
  -> agent authority is limited to that worktree and allowed paths
  -> main worktree remains unchanged during agent execution
```

The user verifies both worktree identities through ForgeOS.

### 10.4 Dispatch the OpenAI coding task

The user approves the exact remote request.

ForgeOS and Nyx must expose:

- provider and model or agent identity;
- request scope;
- source revision;
- worktree;
- allowed tools;
- maximum budget;
- current spend when available;
- run status;
- cancellation control;
- returned result and limitations.

The remote agent may propose and implement inside its isolated worktree. It may not
modify the main worktree, approve its own patch, increase its budget silently, or
mark the skill or release earned.

### 10.5 Remote-provider failure control

Before or after the successful run, exercise one declared remote-unavailable or
cancelled-run path without corrupting source state.

A valid control may use:

- a deliberately cancelled request;
- a provider-unavailable fixture supported by the closed skill;
- a budget rejection below the requested maximum;
- a declared invalid credential test account that cannot expose a real secret.

Pass edge:

```text
remote task cannot complete
  -> failure is visible and actionable
  -> main worktree remains unchanged
  -> local Nyx remains available
  -> manual ForgeOS development remains available
  -> no false patch or success state appears
```

The successful Change B run must then begin from a clean declared state.

### 10.6 Inspect the agent result

When the successful agent run completes, the user:

1. reviews the exact changed files;
2. reviews the complete diff;
3. reviews commands the agent reports running;
4. compares reported results with authoritative command records;
5. verifies no forbidden path changed;
6. verifies no new module exceeds the closure limit;
7. verifies no duplicate source of truth was introduced;
8. rejects any unsupported claim.

Agent prose does not prove behavior.

### 10.7 Approve or reject the patch

The user must exercise both review outcomes during V1 closure history:

- one returned patch or patch revision is rejected or sent back when it does not
  meet the declared contract, or a dedicated safe rejection fixture is used;
- the final acceptable patch is explicitly approved.

Rejection must leave the main worktree unchanged.

Approval must apply only the exact reviewed patch through the declared ForgeOS
path.

### 10.8 Apply the approved patch

ForgeOS applies the approved Change B patch to the main worktree.

Pass edge:

```text
reviewed patch identity
  -> explicit user approval
  -> exact patch applied to declared starting revision
  -> resulting main-worktree diff matches reviewed content
  -> isolated worktree remains separately identifiable
```

A context mismatch, changed starting revision, or altered patch identity must block
application and require a new review.

### 10.9 Validate Change B independently

After patch application, ForgeOS runs the real local commands, regardless of any
agent-reported success:

1. formatter;
2. focused Change B tests;
3. build;
4. declared negative or failure-path test;
5. all approved structural guards;
6. applicable ForgeOS and Nyx regressions.

The user exercises the real Change B result and approves the bounded behavior.

### 10.10 Review and commit Change B

The user:

1. inspects current Git status;
2. reviews the final exact diff;
3. stages intended files;
4. commits the verified result;
5. verifies commit identity and clean state through ForgeOS;
6. records `CHANGE_B_COMMIT`.

Journey B passes only after local independent validation and user acceptance.

---

## 11. Mandatory integrated control matrix

The following controls must pass during the closure run. Some occur inside the two
journeys; others may run immediately before final release evaluation.

### 11.1 Local-only control

Disable remote OpenAI access through the real ForgeOS configuration.

Verify:

- ForgeOS still starts;
- the editor, terminal, Git, builds, and tests remain usable;
- local Nyx remains usable;
- remote-only functions report their unavailable state honestly;
- no local action silently sends data remotely.

Re-enable remote access only through the declared configuration path.

### 11.2 Nyx denial control

Deny one bounded action and verify no execution, mutation, retry, or standing
authority results.

### 11.3 Stale-result control

After a successful validation result, make a controlled source edit that changes
the source-state identity without committing it.

Verify:

- the prior result remains historically visible;
- the current source is no longer presented as verified by that result;
- aggregate green state becomes stale, unknown, or otherwise truthful;
- rerunning validation associates the new result with the new source state.

Restore or incorporate the controlled edit through normal ForgeOS actions before
continuing.

### 11.4 Dirty-worktree control

Create one bounded uncommitted change and verify:

- ForgeOS displays the dirty state;
- project close or risky operations follow their declared prompt behavior;
- restart restoration does not pretend the worktree is clean;
- Git status agrees with the visible state.

### 11.5 Command-failure control

Run one registered command or fixture expected to fail.

Verify:

- nonzero exit status is preserved;
- output remains inspectable;
- cancellation and failure are distinct;
- the UI does not retain a green current result;
- Nyx does not summarize failure as success.

### 11.6 Service-recovery control

Exercise one declared Forge World or managed-service recovery path already closed
under V1.

The control must not corrupt:

- source files;
- project registration;
- Git state;
- command records;
- Nyx audit history;
- user settings required by the V1 contract.

### 11.7 Invalid project-boundary control

Attempt one project or file operation outside the declared approved root and
verify rejection without partial registration or mutation.

### 11.8 Patch mismatch control

Attempt to apply a reviewed patch against a deliberately changed source-state
identity or use the closed mismatch fixture.

Verify the application is blocked and requires renewed review.

### 11.9 Module-limit control

Run the authored Rust module-size guard against final source.

Release requires:

```text
all authored Rust modules <= 1000 physical lines
no numbered or meaningless shards created to game the limit
no compressed multi-statement formatting used to game physical lines
module names and responsibilities remain coherent
```

The 1200-line breathing ceiling is not acceptable at skill or release closure.
Any module above 1000 blocks release.

### 11.10 Core and seam controls

Run:

- Forge Core purity guard;
- cross-subsystem seam and dependency-direction guard;
- skill graph and router integrity guard.

All must pass without new allowlists created merely for closure.

---

## 12. Final regression replay

After `CHANGE_B_COMMIT`, run the complete declared V1 regression set from the clean
committed source state.

The final regression replay must cover:

- every closed Tier 0 structural foundation;
- every closed Tier 1 local mechanism;
- every closed Tier 2 system;
- every closed Tier 3 user or operator workflow;
- every closed Tier 4 integrated capability;
- the self-hosting capability;
- ForgeOS package verification;
- Nyx local-only behavior;
- remote-agent isolation and review behavior;
- restart and restore behavior;
- all four approved non-behavior guard classes.

A consolidated regression suite may satisfy multiple locks only when each relevant
result remains attributable and failures cannot be hidden by aggregate success.

Final source requirements:

```text
ForgeOS worktree clean
required Nyx worktree clean
CHANGE_A_COMMIT reachable in history
CHANGE_B_COMMIT current or reachable as declared
all current commands green
all required controls green
no V1 skill invalidated
no active source skill remains
no module above 1000 physical lines
```

---

## 13. User acceptance procedure

The user is the V1 functional acceptance authority.

After both journeys and final regression replay, present a bounded acceptance
summary that states:

- what was installed;
- which supported host profile was used;
- what Change A did;
- what Change B did;
- which actions were manual;
- which actions Nyx performed;
- which action was denied;
- which work the OpenAI path performed;
- how isolation and approval were enforced;
- what happened across cold login;
- which failures and negative controls were exercised;
- which current limitations remain outside V1 scope;
- which later-version capabilities are not claimed.

The user then personally confirms or rejects each of these questions:

```text
1. Could I install ForgeOS through the declared V1 path?
2. Could I enter ForgeOS as a real login session without another IDE?
3. Could I register and restore the real ForgeOS project?
4. Could I edit and navigate real Rust source effectively enough for V1?
5. Could I run and understand real build, test, and failure results?
6. Could I inspect and commit real Git changes safely?
7. Did local Nyx help through real project tools rather than invented state?
8. Did approval and denial behave exactly as shown?
9. Was the remote coding task isolated, budgeted, reviewable, and user-approved?
10. Did ForgeOS recover coherently after cold login?
11. Did the second real source change complete entirely inside ForgeOS?
12. Did the final regression replay preserve all closed V1 behavior?
13. Is First Armor usable enough for its declared raw V1 scope?
14. Do I explicitly approve ForgeOS V1 First Armor for release?
```

Only an explicit affirmative answer to question 14 can authorize release credit.

The user may reject release even when all automated checks pass. That rejection
becomes the current release blocker and must be stated concretely.

No model or agent may infer approval from silence, earlier enthusiasm, successful
commands, or completion of the operator steps.

---

## 14. Required closure records and release package

Before the apex transition, create the two required records for
`FORGEOS-V1-APEX-001` at:

```text
docs/versions/V1/skills/FORGEOS-V1-APEX-001/CLOSURE_AND_SPEC.md
docs/versions/V1/skills/FORGEOS-V1-APEX-001/USER_GUIDE_SOURCE.md
```

### 14.1 Apex `CLOSURE_AND_SPEC.md`

It must contain:

- exact closure experiment ID and version;
- supported host profile;
- installation artifact identity;
- ForgeOS and Nyx source revisions;
- Change A registration and commit;
- Change B registration and commit;
- exact commands run;
- all control results;
- restart boundary and restoration result;
- worktree isolation result;
- API budget and redacted provider audit summary;
- final regression result;
- final module-size result;
- user acceptance answers;
- explicit user release approval;
- supported V1 claim;
- explicit V1 non-claims;
- known supported-host limitations;
- confirmation that no criterion was deferred.

### 14.2 Apex `USER_GUIDE_SOURCE.md`

It must contain all facts needed for the onboard Forge Guide and website V1 guide:

- installation requirements;
- supported host profile;
- login-session selection;
- first launch;
- project registration;
- editor use;
- terminal and registered commands;
- Git workflow;
- local Nyx setup and use;
- approval and denial behavior;
- remote OpenAI configuration;
- budget controls;
- isolated worktree workflow;
- patch review and application;
- restart and project restoration;
- local-only operation;
- normal failure states and recovery;
- uninstall or safe fallback path;
- V1 limitations;
- explicit later-version exclusions.

These documents record earned behavior. They do not create it and they are not
verified by prose CI.

### 14.3 Release artifact set

The closure package must preserve enough structured evidence to audit what ran,
without pretending the evidence is the product.

Required artifacts:

```text
closure_activation_record
supported_host_profile
installation_artifact_identity
installation_result
session_start_and_health_record
project_registration_record
change_a_registration
change_a_diff_and_commit_identity
change_b_registration
remote_agent_scope_and_budget_record
isolated_worktree_identity
reviewed_patch_identity
change_b_diff_and_commit_identity
command_result_index
control_matrix_result
final_regression_result
structural_guard_results
restart_restore_result
redacted_nyx_and_agent_audit_index
user_acceptance_record
release_claim_and_non_claims
```

Screenshots or video may accompany the package for demonstration. They are never
required proof and never replace authoritative records or user acceptance.

Secrets, tokens, credential values, private model prompts containing secrets, and
unredacted environment dumps are forbidden in the package.

---

## 15. Pass, negative, block, invalid, and abort classifications

### 15.1 `PASS`

The experiment passes only when:

```text
all activation prerequisites remain true
  -> clean-host installation passes
  -> real ForgeOS login session passes
  -> Journey A passes and CHANGE_A_COMMIT exists
  -> cold-login restoration passes
  -> Journey B passes and CHANGE_B_COMMIT exists
  -> all mandatory controls pass
  -> full V1 regression replay passes
  -> all final source states are clean
  -> apex closure records are complete
  -> user explicitly approves V1
```

### 15.2 `VALID_NEGATIVE`

Expected denials and failures may produce valid negative results, including:

- invalid project registration rejected;
- Nyx action denied without execution;
- remote provider unavailable without source mutation;
- patch mismatch blocked;
- command failure shown truthfully;
- stale success invalidated after source change.

A valid negative proves the safety or error path. It does not compensate for a
failed required positive path.

### 15.3 `BLOCKED`

Use `BLOCKED` when the first causal blocker is identified and the experiment
cannot proceed honestly.

The blocker must name:

```yaml
blocker_id:
closure_step:
statement:
blocker_class:
owning_skill:
owning_subsystem:
source_repository:
source_revision:
discovered_by:
resolution_condition:
status: OPEN
```

### 15.4 `INVALID`

The run is invalid when any of the following occurs:

- Change A or Change B was preimplemented;
- source was edited outside ForgeOS during a journey;
- another IDE or forbidden terminal performed source work;
- a patch was manually applied outside the declared review path;
- hidden operator repair changed product state;
- a clean-host claim used prior ForgeOS state;
- agent work occurred in the main worktree;
- source revision or patch identity cannot be established;
- a result was fabricated, hand-edited, or associated with the wrong revision;
- required secrets were exposed in artifacts;
- a test, guard, or requirement was weakened during closure;
- a V2+ feature was substituted for missing V1 behavior;
- user approval was inferred rather than explicitly given.

An invalid run earns nothing and must restart from the earliest invalidated
boundary after repair.

### 15.5 `ABORTED`

Use `ABORTED` when the user intentionally stops the run before a pass or blocker is
classified.

Aborting is not failure and earns no release credit. Preserve resumable state only
when the source and experiment boundaries remain trustworthy.

---

## 16. First-blocker routing table

Use the earliest failed edge, not the loudest symptom.

| Closure edge | Primary owning skill or branch |
| --- | --- |
| Package cannot install or identify source | `FORGEOS-V1-DIST-200`, `FORGEOS-V1-DIST-300` |
| ForgeOS session absent or cannot start | `FORGEOS-V1-SESSION-200`, `FORGEOS-V1-SESSION-300` |
| Managed services fail at login | `FORGEOS-V1-NYX-200`, `FORGEOS-V1-SESSION-300`, recovery branch |
| Project registration or restore fails | `FORGEOS-V1-PROJECT-200`, `FORGEOS-V1-PROJECT-300` |
| Editor or Rust intelligence fails | code and editor branch ending at `FORGEOS-V1-CODE-400` |
| Terminal or registered command path fails | terminal and command branch |
| Git state, diff, worktree, or commit fails | source-control branch ending at `FORGEOS-V1-SOURCE-400` |
| Local Nyx cannot ground project answers | Nyx branch ending at `FORGEOS-V1-NYX-400` |
| Approval does not resume exact action | Nyx checkpoint skill owning that path |
| Denial executes or silently retries | Nyx permission or checkpoint skill |
| Remote request bypasses isolation or budget | agent branch ending at `FORGEOS-V1-AGENT-400` |
| Patch review or identity fails | patch branch ending at `FORGEOS-V1-PATCH-300` |
| Validation state is stale or false | verify branch ending at `FORGEOS-V1-VERIFY-400` |
| Forge World shows invented state | world branch ending at `FORGEOS-V1-WORLD-400` |
| Cold-login restore fails | recovery and workstation branches |
| Self-hosting source change cannot complete | `FORGEOS-V1-SELFHOST-400` or earliest failed prerequisite |
| Module, Core, seam, or router guard fails | owning structural Tier 0 skill |
| User rejects usability or release | exact user-facing owning skill, or apex when integration-only |

When a closed skill no longer satisfies its contract, mark it `INVALIDATED` before
routing repair. Do not leave it closed while patching around the failure.

After repair:

```text
rerun owning skill closure path
  -> restore CLOSED state with user acceptance when required
  -> rerun affected dependent regressions
  -> restart this closure experiment from the earliest affected boundary
```

---

## 17. Release award and atomic transition

After `PASS` and explicit user approval, perform one atomic release transition.

```text
FORGEOS-V1-APEX-001:
  LOCKED -> RELEASE_EARNED

PROGRAM_MODE:
  RELEASE_CLOSURE -> RELEASE_EARNED

ACTIVE_CLOSURE_EXPERIMENT:
  FORGEOS-V1-CLOSURE-0001 -> COMPLETE

ACTIVE_SKILLS:
  []

INVALIDATED_V1_SKILLS:
  []
```

The atomic transition must update:

- the apex section in the canonical worksheet;
- the router release state;
- the header current campaign;
- the apex closure records;
- the supported release claim and non-claims;
- the exact release source revisions;
- the installation artifact identity.

The transition may not occur before all source, controls, regressions, records, and
user approval are complete.

A release tag, package name, archive name, marketing announcement, or version
string does not award release credit by itself.

---

## 18. Supported V1 release claim

On pass, ForgeOS may claim only:

> ForgeOS V1 First Armor is a Linux-based developer operating environment that can
> be installed on its declared supported host, entered as a real login session,
> and used to perform bounded real Rust development with project restoration,
> editing, language intelligence, terminal commands, Git, local Nyx assistance,
> a reviewed OpenAI coding-agent workflow, validation, restart recovery, and
> committed source changes without another IDE.

The exact supported host, installation limitations, model requirements, provider
requirements, and known V1 limits must accompany the claim.

---

## 19. Explicit V1 non-claims

A passed closure experiment does not claim:

- a custom Linux kernel;
- a custom Wayland compositor;
- support for every Linux distribution or hardware configuration;
- broad language support beyond the closed V1 Rust profile;
- full cinematic project worlds;
- a complete runtime capability-tree game system;
- multi-agent orchestration;
- autonomous release authority;
- unrestricted shell authority for Nyx or remote agents;
- voice-first operation;
- distributed builds;
- enterprise collaboration;
- multiplayer development spaces;
- extension or theme marketplaces;
- virtual reality support;
- correctness of every model answer;
- elimination of normal software defects;
- V2, V3, or V4 completion.

The closure experiment proves First Armor, not the entire ForgeOS destination.

---

## 20. Repeatability and post-release invalidation

The two-journey closure run is the minimum V1 release experiment.

A later release build or supported-host change must rerun the affected closure
edges. It may reuse unaffected closed-skill evidence only when the governing laws
and invalidation graph support that reuse.

After V1 is earned, any current evidence that disproves a required V1 behavior
must:

1. identify the exact failed edge;
2. invalidate the owning skill and affected dependents;
3. suspend the affected release claim;
4. route one blocker and one slice;
5. repeat changed user acceptance;
6. rerun the affected closure journey or full experiment before restoring release
   status.

Release history remains preserved. Current release truth may not remain green
because an older version once passed.

---

## 21. Closure sequence summary

```text
all 66 non-apex V1 skills CLOSED
  -> closure experiment activated
  -> clean source and package preflight
  -> clean supported host installation
  -> real ForgeOS login session
  -> real project registration and baseline
  -> local Nyx grounded inspection
  -> one approved Nyx action
  -> one denied Nyx action
  -> manual Change A inside ForgeOS
  -> local validation and CHANGE_A_COMMIT
  -> cold logout and login boundary
  -> restored project and Nyx state
  -> bounded OpenAI Change B in isolated worktree
  -> failure control, review, rejection path, and exact approval
  -> patch applied through ForgeOS
  -> independent local validation and CHANGE_B_COMMIT
  -> local-only, stale-result, dirty-state, failure, recovery, boundary,
     mismatch, module, Core, seam, and router controls
  -> complete V1 regression replay
  -> apex closure records completed
  -> user explicitly approves First Armor
  -> FORGEOS-V1-APEX-001 becomes RELEASE_EARNED
```

A failure returns one first blocker.

A valid negative proves only its declared negative edge.

Evidence supports the journey.

The user approves the product.

The armor works before it glows.
