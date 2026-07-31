# ForgeOS Four-Version High-Level Plan

## 1. Product Definition

**ForgeOS** is a Linux-based developer operating environment where the entire system is organized around building software.

It combines:

* A game-engine-powered spatial interface
* A serious code editor and terminal environment
* Visual project and capability skill trees
* Git, builds, tests, debugging, and experiments
* A local AI assistant hosted by `nyx_server`
* Optional heavyweight OpenAI coding agents
* Proof-driven missions that represent actual software capabilities
* A future custom desktop session and Wayland compositor

The final experience should feel like operating an advanced suit with an intelligent technical partner, not using a normal IDE with a science-fiction wallpaper.

## 2. Final Product Promise

> Boot into ForgeOS, enter a project world, select what the software should learn to do next, and work with Nyx and remote coding agents until that capability is proven through the real system.

## 3. Core Architecture

```text
Linux Kernel and Existing Drivers
                |
        ForgeOS Session Layer
                |
    +-----------+------------+
    |                        |
Forge World              Forge Core
Bevy interface           Project truth
Spatial UI               Skill trees
HUD panels               Missions
Visual state             Experiments
                         Proof receipts
    |                        |
    +-----------+------------+
                |
         Developer Bridge
   Git / PTY / LSP / DAP / Files
   Builds / Tests / Debuggers
                |
           nyx_server
   Local model host and AI runtime
                |
    +-----------+------------+
    |                        |
Local Models          OpenAI Adapter
Routine guidance      Heavy coding work
Navigation            Large-context work
Safe commands         Complex implementation
Explanation           Deep review
```

## 4. Authority Boundaries

### `nyx_server` owns

* Local model hosting
* AI sessions and conversations
* Model routing
* Tool invocation
* Agent permissions
* Repository access
* Context assembly
* Run records
* Human approval checkpoints
* Remote OpenAI requests
* Agent audit history

### Forge Core owns

* Projects
* Repositories
* Releases
* Skill trees
* Skill states
* Missions
* Experiments
* Blockers
* Proof receipts
* Invalidation
* Developer settings
* Workspace state

### Forge World owns

* Spatial presentation
* Project environments
* HUD panels
* Skill-tree visualization
* Animations
* Audio
* User interaction
* Window and panel arrangement

### Real development tools own

* Compilation
* Language semantics
* Debugging
* Source control
* Formatting
* Testing
* Package management

ForgeOS orchestrates these tools. It does not pretend to replace compilers, Git, language servers, or debuggers.

## 5. Non-Negotiable Product Laws

1. **ForgeOS must remain useful with every visual effect disabled.**
2. **Code, terminals, logs, and diffs use crisp 2D surfaces.**
3. **The 3D world represents real project state, never decorative fake state.**
4. **Nyx may propose or perform actions, but it may not invent project truth.**
5. **Remote AI is optional. Core development remains local-first.**
6. **Every agent action is scoped, logged, and reviewable.**
7. **A skill advances only when its real experiment passes.**
8. **Keyboard operation must always be faster than navigating the world manually.**
9. **Each version must be usable to build the next version.**
10. **Do not write a custom kernel, compiler, package manager, or driver stack.**

---

# Version Overview

| Version | Name                   | Core Outcome                                                                 |
| ------- | ---------------------- | ---------------------------------------------------------------------------- |
| V1      | **First Armor**        | Boot into ForgeOS and perform real daily coding work                         |
| V2      | **Living Forge**       | Projects become persistent interactive worlds with skill trees and missions  |
| V3      | **Autonomous Foundry** | Nyx coordinates agents, experiments, proof, and multi-repository development |
| V4      | **Aegis Ascendant**    | Full spatial developer OS with an integrated Jarvis-style experience         |

---

# V1: First Armor

## Product Goal

ForgeOS boots into a dedicated Linux session and is capable of real software development.

It may look raw. It may expose rough edges. It must not be a mockup.

A developer must be able to:

```text
Boot ForgeOS
-> open a repository
-> inspect and edit code
-> run commands
-> build and test
-> inspect Git changes
-> ask Nyx for help
-> invoke a heavyweight coding agent
-> review and apply a patch
-> commit working code
```

## User Experience

The user logs in through the normal Linux display manager and selects:

```text
ForgeOS
```

ForgeOS opens directly into a simple Bevy-powered project hangar.

The environment contains:

* Project selector
* File browser
* Editor surface
* Embedded terminal
* Git panel
* Build and test output
* Nyx assistant panel
* Diff and patch review
* Basic system telemetry

The 3D layer is intentionally restrained.

The user may see project bays, active processes, and basic status indicators, but V1 prioritizes responsiveness and actual coding.

## Required Capabilities

### Boot and Session

* Dedicated ForgeOS login session
* Automatic launch of ForgeOS services
* Reliable logout and shutdown
* Recovery after Forge World crashes
* Optional auto-login development image
* Installer for an existing Linux system
* Later V1 milestone: bootable installation image

### Project Management

* Register an existing repository
* Open recent projects
* Define project commands
* Define repository boundaries
* Persist project settings
* Detect dirty worktrees
* Restore the last workspace

### Editing

V1 supports Rust first.

Required:

* File tree
* Open, edit, save, and close files
* Search within files
* Search repository text
* Syntax highlighting
* Rust Analyzer integration
* Diagnostics
* Go to definition
* Symbol search
* Basic completion
* Multiple open buffers

A terminal editor bridge such as Helix or Neovim is acceptable during early V1, provided ForgeOS controls the surrounding experience. A native editor may replace it before V1 closes.

### Terminal and Commands

* Embedded PTY terminal
* Multiple terminal sessions
* Project working directory management
* Registered build commands
* Registered test commands
* Registered formatting commands
* Command history
* Process cancellation
* Captured command output

### Git

* Status
* Diff
* Stage and unstage
* Commit
* Branch display
* Current revision
* Restore selected changes with confirmation
* Open an isolated worktree for agent tasks

### Nyx Integration

`nyx_server` becomes a required system service.

Nyx must support:

* Local model discovery and selection
* Project-aware chat
* Repository search
* File reading
* Git status and diff inspection
* Safe registered command execution
* Patch reception
* Patch review
* Patch application after approval
* Human checkpoints
* Run and tool audit records
* Remote OpenAI escalation
* Per-run API spending limits
* Session recovery after restart

The existing checkpoint system must be completed so approval resumes the exact suspended action.

### Agent Workflow

The first remote-agent workflow is:

```text
Describe task
-> Nyx builds context packet
-> remote coding agent works in isolated worktree
-> ForgeOS receives patch
-> user reviews diff
-> user approves
-> ForgeOS applies patch
-> registered tests run
-> result is recorded
```

### Basic Visual State

Project bays display:

* Clean or dirty repository
* Build status
* Test status
* Active process count
* Active Nyx run
* Current branch
* Uncommitted changes

## V1 Launch Gate

V1 is complete only when ForgeOS can be used to implement, test, review, and commit a real ForgeOS or Nyx feature without leaving the ForgeOS session for development work.

External browser use is acceptable.

External IDE use is not.

## Explicitly Out of Scope

* Custom Wayland compositor
* Full voice control
* Multi-agent orchestration
* Complex architecture visualization
* Full skill-tree workflow
* Multiplayer collaboration
* Theme marketplace
* Virtual reality
* Broad language support
* Autonomous destructive actions
* Cinematic project worlds

V1 is a functional suit frame with exposed wiring.

---

# V2: Living Forge

## Product Goal

ForgeOS stops being merely a development shell and becomes a persistent visual model of the project.

Projects gain capability trees, missions, experiments, proof, history, and spatial identity.

## User Experience

Each project becomes its own environment.

Examples:

* A simulation project resembles a research facility.
* A game project resembles a world-building foundry.
* A server project resembles a command station.
* A multi-repository system occupies several connected regions.

The environment changes according to real engineering state.

Examples:

* Passing systems receive power.
* Blocked skills show broken paths.
* Dirty repositories expose active work.
* Failing experiments damage connected nodes.
* Completed capabilities become permanent structures.
* Invalidated proof visibly deactivates dependent systems.

## Required Capabilities

### Capability Skill Trees

* Tier 0 through Tier 5 capabilities
* Permanent skill IDs
* Dependency graph
* Locked, available, active, blocked, proved, banked, and invalidated states
* Main release route
* Sidequest branches
* Cross-repository dependencies
* Visual selectable tree

### Missions

A selected skill produces a mission containing:

* Capability statement
* Current status
* Direct prerequisites
* Experiment question
* Current first blocker
* Allowed scope
* Forbidden scope
* Required commands
* Required artifacts
* Pass condition
* Regression shield

### Experiment Runner

* Registered experiment definitions
* Controlled input sets
* Pass edge
* Negative edge
* Invalid edge
* Required controls
* Repeat execution
* Artifact capture
* Hashing
* Comparison
* Result history

### Proof Receipts

Every earned capability records:

* Repository
* Revision
* Commands
* Exit codes
* Inputs
* Outputs
* Artifacts
* Hashes
* Controls
* Repeatability
* Supported claim
* Explicit non-claims
* Missing higher proof

### Persistent Project Memory

Nyx can query:

* Current release target
* Active skill
* Current blocker
* Last experiment
* Architecture authority
* Last known green revision
* Required commands
* Forbidden changes
* Paused missions
* Recent proof receipts

Nyx no longer relies on chat history to understand the project.

### Agent Mission Packets

Nyx generates exact mission packets for humans or remote agents.

Every packet contains:

* Source authority
* Repository revision
* Skill
* Blocker
* Scope
* Relevant files
* Commands
* Tests
* Expected return
* Proof requirements

### Native Developer Surfaces

V2 should move beyond temporary editor bridges.

Required:

* Native Forge editor
* Tree-sitter parsing
* LSP integration
* Basic Debug Adapter Protocol integration
* Breakpoints
* Stack frames
* Variable inspection
* Search and symbol panels
* Persistent layouts

### Language Expansion

V2 should support a limited set of strong language profiles:

* Rust
* Python
* JavaScript or TypeScript
* C or C++

Each profile must declare:

* Language server
* Formatter
* Build system
* Test commands
* Debug adapter
* Project discovery rules

## V2 Launch Gate

V2 is complete when a developer can select a previously unproved capability, receive a mission, implement it manually or through an agent, run its experiment, generate a proof receipt, and watch the capability unlock dependent branches.

The entire loop must occur through ForgeOS.

## Explicitly Out of Scope

* Fully autonomous project management
* Large multi-agent teams
* Full desktop compositor
* Voice-first operation
* Enterprise collaboration
* Remote distributed execution
* Fully cinematic worlds
* Universal language support

V2 turns the suit into a living development machine.

---

# V3: Autonomous Foundry

## Product Goal

ForgeOS becomes an agent command environment.

The developer shifts from manually driving every implementation step to directing missions, reviewing decisions, and controlling proof.

Nyx becomes the persistent technical operator of the system.

## User Experience

The developer selects a release objective or capability.

Nyx can:

* Inspect project state
* Recommend the next available mission
* Diagnose the first blocker
* Prepare implementation context
* Assign work to a local or remote agent
* Create isolated worktrees
* Observe command execution
* Request approval where required
* Run experiments
* Analyze failures
* Generate proof receipts
* Recommend the next move

The human remains release authority.

## Required Capabilities

### Multi-Agent Coordination

* Multiple concurrent agent runs
* One isolated worktree per run
* Agent specialization
* Task dependency tracking
* Merge preparation
* Conflict detection
* Agent result comparison
* Agent performance records
* Explicit agent authority limits

Possible roles:

```text
Planner
Implementer
Test author
Reviewer
Experiment operator
Documentation reconciler
Regression investigator
```

### Advanced Nyx Permissions

Mission-scoped authority:

```text
Observe
Navigate
Run approved commands
Edit declared files
Apply patches
Create worktrees
Spend declared API budget
Request destructive action
```

Every authority grant has:

* Scope
* Expiration
* Repository
* Allowed tools
* Spending limit
* Required approval level

### Invalidation Engine

When source or contracts change, ForgeOS determines:

* Which proof receipts remain valid
* Which capabilities require rerunning
* Which release claims are stale
* Which dependent branches become blocked
* Which user witnesses require renewal

### Architecture Intelligence

ForgeOS identifies:

* Duplicate implementations
* Parallel sources of truth
* Layer violations
* Cyclic dependencies
* Dead commands
* Unregistered execution paths
* UI-owned backend truth
* Stale documentation
* Tests bypassing public paths
* Sidequest implementations replacing mainline systems

### Multi-Repository Projects

* Repository graph
* Authority boundaries
* Cross-repository contracts
* Version compatibility
* Coordinated mission routing
* Repository-specific build environments
* Cross-repository experiments
* Atomic handoff records

### CI and Remote Execution

* CI provider integration
* Remote test runners
* Artifact retrieval
* Build matrix status
* Remote experiment execution
* Reproducibility comparisons
* Release candidate verification

### Project Archaeology

* Capability state at historical revisions
* Git history projected onto the skill tree
* Architecture evolution
* Regression origin tracking
* First appearance of invalid states
* Restore and replay of past missions

### Early Voice Interface

Voice is introduced as an optional command surface.

Examples:

```text
Nyx, open the active blocker.
Run the focused test set.
Show me what changed.
Prepare this mission for a heavyweight agent.
Do not apply anything without review.
```

Voice never becomes mandatory.

## V3 Launch Gate

V3 is complete when ForgeOS can coordinate a medium-sized capability across multiple repositories, using local and remote agents, while preserving authority boundaries, producing proof, and requiring the developer only for decisions that actually need human judgment.

## Explicitly Out of Scope

* Complete replacement of the Linux desktop
* Fully autonomous releases
* Unrestricted system access
* Human-free architectural decisions
* Virtual reality as a requirement
* Visual spectacle ahead of productivity

V3 turns the suit into a staffed foundry.

---

# V4: Aegis Ascendant

## Product Goal

ForgeOS becomes the complete spatial developer operating system.

The user no longer feels like they opened an IDE.

They feel like they entered the operational environment of the software.

This is the full Iron Man and Jarvis version.

## User Experience

The user boots directly into ForgeOS.

Projects occupy persistent locations in a large developer world.

Nyx is omnipresent but not intrusive.

The system understands:

* What project the developer entered
* What they are looking at
* Which mission is active
* Which files and systems matter
* Which agents are working
* Which decisions remain unresolved
* Which proof is missing
* Which release routes are blocked
* What the developer usually does next

The world responds to real state in real time.

## Required Capabilities

### ForgeOS Wayland Session

ForgeOS becomes a complete desktop session and eventually its own Wayland compositor.

It can:

* Host normal Linux application surfaces
* Position native applications inside the spatial environment
* Manage windows as project tools
* Preserve ordinary keyboard and mouse behavior
* Launch browsers, debuggers, profilers, image tools, and other applications
* Recover applications after shell restart
* Support multiple monitors
* Support traditional flat workspace mode

ForgeOS remains Linux underneath.

### Full Spatial Project Worlds

Projects may expose:

* Capability constellations
* Architecture districts
* Runtime topology
* Data flows
* Build pipelines
* Test chambers
* Evidence vaults
* Git history timelines
* Agent workstations
* Release launch systems
* Cross-repository transit routes

Every visual element corresponds to inspectable project data.

### Advanced Nyx Presence

Nyx becomes a full developer companion capable of:

* Natural voice conversation
* Context-aware interruption
* Quiet observation
* Suggesting likely blockers
* Preparing work without applying it
* Monitoring long-running processes
* Coordinating multiple agents
* Explaining architecture spatially
* Replaying project history
* Detecting contradictions
* Managing API and compute budgets
* Learning project-specific operating patterns

Nyx remains bounded by explicit permissions.

### Adaptive Workspace

ForgeOS changes its interface according to the activity.

Examples:

#### Coding mode

* Editor dominates
* Minimal visual effects
* Diagnostics nearby
* Nyx quiet unless needed

#### Debugging mode

* Runtime topology expands
* Stack and variables become spatially connected
* Logs align with components
* Execution paths become visible

#### Architecture mode

* Source files collapse into systems
* Dependencies and authority boundaries become prominent
* Historical changes can be replayed

#### Mission mode

* Capability contract stays visible
* Active blocker is pinned
* Required proof remains in view
* Agent and test activity surrounds the mission

#### Release mode

* Required capabilities form the launch route
* Missing proof blocks visible paths
* Release authority and approvals become explicit
* Final experiments run through a controlled sequence

### Developer Mastery Tree

Separate from product capability trees, ForgeOS may track what the developer has demonstrated.

Examples:

* Language knowledge
* Debugging ability
* Architecture skills
* Tool familiarity
* Release experience

This system must never confuse hours spent or lines written with mastery.

### ForgeOS Capability Tree

Advanced ForgeOS functions unlock through use and readiness:

* Remote runners
* Multi-agent command
* Custom world construction
* Advanced debugging
* Performance visualization
* Fleet management
* Distributed builds
* Release automation

Essential development tools remain available from the start.

### Extensions and Themes

* Project-world templates
* HUD layouts
* Language profiles
* Tool adapters
* Agent providers
* Build-system adapters
* Visual themes
* Sound environments
* Mission templates
* Capability-tree templates

Extensions may customize presentation and integration.

They may not bypass Forge Core authority.

### Collaboration

Optional collaborative capabilities:

* Shared project worlds
* Team mission boards
* Agent activity visibility
* Review rooms
* Architecture walkthroughs
* Release command sessions
* Remote pair development
* Shared proof inspection

## V4 Launch Gate

V4 is complete when ForgeOS can serve as the developer’s primary operating environment for extended daily use and provide a coherent experience across coding, debugging, architecture, agents, proof, releases, and normal Linux applications.

The full experience must remain:

* Fast
* Keyboard-driven
* Recoverable
* Inspectable
* Local-first
* Accessible in flat mode
* Useful without voice
* Useful without remote AI
* Honest about what the project has actually earned

V4 is not merely a game wrapped around an IDE.

It is a developer operating system whose world is shaped by software truth.

---

# Cross-Version Evolution

## Visual Experience

```text
V1: Basic project hangar and HUD
V2: Persistent project worlds and skill trees
V3: Active foundry with agents and architecture state
V4: Full spatial developer operating system
```

## Nyx

```text
V1: Local assistant and remote-agent gateway
V2: Persistent project-aware mission guide
V3: Multi-agent technical operator
V4: Integrated Jarvis-style developer companion
```

## Development Environment

```text
V1: Functional editor, terminal, Git, and Rust workflow
V2: Native IDE surfaces and multiple language profiles
V3: Multi-repository and distributed development
V4: Complete operating environment and compositor
```

## Proof Workflow

```text
V1: Build and test results
V2: Experiments and proof receipts
V3: Invalidation and release governance
V4: Whole-system visual truth and historical replay
```

## Operating-System Integration

```text
V1: Dedicated application session
V2: Deep Linux desktop integration
V3: Experimental ForgeOS session management
V4: ForgeOS Wayland compositor and distribution
```

---

# V1 Build Order

The first version should be built in this order:

```text
1. ForgeOS session launches reliably
2. Bevy shell opens and restores projects
3. Terminal and project commands work
4. Code can be edited and saved
5. Git status, diff, and commit work
6. nyx_server runs as a managed service
7. Nyx can inspect the active repository
8. Nyx can execute approved registered commands
9. Remote agent patches can be received and reviewed
10. ForgeOS is used to complete a real ForgeOS feature
```

Do not begin V2 work until step ten is repeatedly true.

---

# Completion Law

```text
V1 earns daily usability.

V2 earns project awareness.

V3 earns supervised autonomy.

V4 earns the developer operating system.
```

The visual ambition grows only after the underlying development loop proves itself.

The armor must work before it glows.
