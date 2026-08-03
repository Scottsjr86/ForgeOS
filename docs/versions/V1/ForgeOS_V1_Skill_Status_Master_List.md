# ForgeOS V1 Skill Status Master List

> Current status snapshot for the active ForgeOS V1 build.
>
> Source basis: `Forge_OS_V1_base_31.tar`, the canonical V1 skill tree, and the accepted behavior-only CI result through `FORGEOS-V1-PATCH-100`.
> `FORGEOS-V1-PROJECT-200` is currently active and awaiting operator validation.

## Snapshot

| Metric | Count |
|---|---:|
| Total V1 skills | 67 |
| ✅ Completed / closed | 21 |
| 🔨 Active / started | 1 |
| 🟢 Available / ready to start | 4 |
| 🔒 Locked by prerequisites | 41 |

**Raw closed-node count:** 21 of 67, or 31.3%. This is not a release-completion percentage because higher-tier nodes integrate many lower-tier capabilities.

## Status legend

| Status | Plain-English meaning |
|---|---|
| ✅ `CLOSED` | Completed, proved, accepted, and recorded. |
| 🔨 `ACTIVE` | Started now. This is the current baton owner. |
| 🟢 `AVAILABLE` | All direct prerequisites are complete. It can be selected, but has not started. |
| 🔒 `LOCKED` | One or more direct prerequisites are unfinished. |
| `BLOCKED` | Work started but hit an external or technical blocker. None currently. |
| `SOURCE_PROVED` | Source behavior is proved but user acceptance is still pending. None currently. |
| `USER_ACCEPTANCE_READY` | Mechanical proof is complete and waiting for explicit user approval. None currently. |
| `INVALIDATED` | Earlier proof was broken by a later change and must be rerun. None currently. |
| `RELEASE_EARNED` | Final Tier 5 release closure. Not yet earned. |

## Tier map

| Tier | Meaning | Skills | Current state summary |
|---:|---|---:|---|
| 5 | Final release capability | 1 | locked 1 |
| 4 | Integrated V1 capabilities | 8 | locked 8 |
| 3 | Complete user and operator workflows | 13 | locked 13 |
| 2 | Functional V1 systems | 19 | active 1, available 1, locked 17 |
| 1 | Local mechanisms | 16 | closed 11, available 3, locked 2 |
| 0 | Atomic foundations and guards | 10 | closed 10 |

## Current baton

### 🔨 `FORGEOS-V1-PROJECT-200` — Persistent project registry and workspace restoration

- **Tier:** 2
- **Status:** `ACTIVE`
- **Current position:** Source patch prepared; behavior-only CI has not yet been returned by the operator.
- **Direct prerequisite:** `PROJECT-100` ✅
- **Immediate unlock after closure:** `FILE-200`, `TERMINAL-200`, and `GIT-200` become available.

## Available skills right now

- 🟢 `FORGEOS-V1-SESSION-200` — Dedicated ForgeOS session bootstrap
- 🟢 `FORGEOS-V1-NYX-100` — Nyx health and versioned client protocol
- 🟢 `FORGEOS-V1-WORLD-100` — Source-backed view projection and input action routing
- 🟢 `FORGEOS-V1-RECOVERY-100` — Workspace snapshot and crash-journal primitive

---

# Tier 5 — Final release capability

The final release gate. It is not merely “done code”; it requires the complete self-hosting V1 journey and explicit release approval.

| Status | Skill | What it is | Direct prerequisites / blockers |
|---|---|---|---|
| 🔒 LOCKED | `FORGEOS-V1-APEX-001` | First Armor is a bootable self-hosting developer environment | 🔒 `FORGEOS-V1-SELFHOST-400` |


# Tier 4 — Integrated V1 capabilities

Large integrated capabilities that combine the complete lower-tier workflows into a usable V1 product.

| Status | Skill | What it is | Direct prerequisites / blockers |
|---|---|---|---|
| 🔒 LOCKED | `FORGEOS-V1-WORKSTATION-400` | ForgeOS provides a bootable daily development workstation | 🔒 `FORGEOS-V1-SESSION-300`<br>🔒 `FORGEOS-V1-PROJECT-300`<br>🔒 `FORGEOS-V1-CODE-300`<br>🔒 `FORGEOS-V1-TERMINAL-300`<br>🔒 `FORGEOS-V1-GIT-300`<br>🔒 `FORGEOS-V1-WORLD-300`<br>🔒 `FORGEOS-V1-DIST-300` |
| 🔒 LOCKED | `FORGEOS-V1-CODE-400` | ForgeOS supports a complete real Rust authoring workflow | 🔒 `FORGEOS-V1-CODE-300`<br>🔒 `FORGEOS-V1-TERMINAL-300`<br>🔒 `FORGEOS-V1-VERIFY-300` |
| 🔒 LOCKED | `FORGEOS-V1-SOURCE-400` | ForgeOS supports a complete source-control workflow | 🔒 `FORGEOS-V1-GIT-300`<br>🔒 `FORGEOS-V1-PATCH-300` |
| 🔒 LOCKED | `FORGEOS-V1-NYX-400` | Nyx is the integrated local AI host and bounded operator | 🔒 `FORGEOS-V1-NYX-300`<br>🔒 `FORGEOS-V1-NYX-301`<br>🔒 `FORGEOS-V1-RECOVERY-300` |
| 🔒 LOCKED | `FORGEOS-V1-AGENT-400` | ForgeOS integrates one heavyweight remote coding agent safely | 🔒 `FORGEOS-V1-AGENT-300`<br>🔒 `FORGEOS-V1-PATCH-300`<br>🔒 `FORGEOS-V1-VERIFY-300`<br>🔒 `FORGEOS-V1-NYX-400` |
| 🔒 LOCKED | `FORGEOS-V1-VERIFY-400` | ForgeOS closes a verified source-change loop | 🔒 `FORGEOS-V1-CODE-400`<br>🔒 `FORGEOS-V1-SOURCE-400`<br>🔒 `FORGEOS-V1-NYX-400`<br>🔒 `FORGEOS-V1-AGENT-400`<br>🔒 `FORGEOS-V1-VERIFY-300` |
| 🔒 LOCKED | `FORGEOS-V1-WORLD-400` | Forge World truthfully presents the V1 environment | 🔒 `FORGEOS-V1-WORLD-300`<br>🔒 `FORGEOS-V1-RECOVERY-300` |
| 🔒 LOCKED | `FORGEOS-V1-SELFHOST-400` | ForgeOS completes a real ForgeOS feature inside ForgeOS | 🔒 `FORGEOS-V1-WORKSTATION-400`<br>🔒 `FORGEOS-V1-VERIFY-400`<br>🔒 `FORGEOS-V1-WORLD-400` |


# Tier 3 — Complete user and operator workflows

End-to-end workflows a developer or operator can actually perform.

| Status | Skill | What it is | Direct prerequisites / blockers |
|---|---|---|---|
| 🔒 LOCKED | `FORGEOS-V1-SESSION-300` | The user logs into a usable ForgeOS session | 🔒 `FORGEOS-V1-SESSION-200`<br>🔒 `FORGEOS-V1-SESSION-201`<br>🔒 `FORGEOS-V1-WORLD-200` |
| 🔒 LOCKED | `FORGEOS-V1-PROJECT-300` | The user registers, opens, and restores a repository workspace | 🔨 `FORGEOS-V1-PROJECT-200`<br>🔒 `FORGEOS-V1-FILE-200`<br>🔒 `FORGEOS-V1-WORLD-200` |
| 🔒 LOCKED | `FORGEOS-V1-CODE-300` | The user edits real Rust source with language intelligence | 🔒 `FORGEOS-V1-EDITOR-200`<br>🔒 `FORGEOS-V1-EDITOR-201`<br>🔒 `FORGEOS-V1-PROJECT-300` |
| 🔒 LOCKED | `FORGEOS-V1-TERMINAL-300` | The user performs daily terminal and project-command work | 🔒 `FORGEOS-V1-TERMINAL-200`<br>🔒 `FORGEOS-V1-COMMAND-200`<br>🔒 `FORGEOS-V1-PROJECT-300` |
| 🔒 LOCKED | `FORGEOS-V1-GIT-300` | The user performs a real Git inspect, stage, and commit workflow | 🔒 `FORGEOS-V1-GIT-200`<br>🔒 `FORGEOS-V1-GIT-201`<br>🔒 `FORGEOS-V1-PROJECT-300` |
| 🔒 LOCKED | `FORGEOS-V1-NYX-300` | The user receives project-aware assistance from a local model | 🔒 `FORGEOS-V1-NYX-200`<br>🔒 `FORGEOS-V1-NYX-201`<br>🔒 `FORGEOS-V1-PROJECT-300` |
| 🔒 LOCKED | `FORGEOS-V1-NYX-301` | The user controls Nyx tool execution through resumable approval | 🔒 `FORGEOS-V1-NYX-202`<br>🔒 `FORGEOS-V1-TERMINAL-300`<br>🔒 `FORGEOS-V1-GIT-300` |
| 🔒 LOCKED | `FORGEOS-V1-AGENT-300` | The user sends one bounded coding task to a remote agent worktree | 🔒 `FORGEOS-V1-AGENT-200`<br>🔒 `FORGEOS-V1-GIT-300`<br>🔒 `FORGEOS-V1-NYX-300` |
| 🔒 LOCKED | `FORGEOS-V1-PATCH-300` | The user reviews, accepts, or rejects a returned agent patch | 🔒 `FORGEOS-V1-AGENT-201`<br>🔒 `FORGEOS-V1-GIT-300`<br>🔒 `FORGEOS-V1-VERIFY-200` |
| 🔒 LOCKED | `FORGEOS-V1-VERIFY-300` | The user runs and understands real formatting, build, and tests | 🔒 `FORGEOS-V1-VERIFY-200`<br>🔒 `FORGEOS-V1-TERMINAL-300`<br>🔒 `FORGEOS-V1-GIT-300` |
| 🔒 LOCKED | `FORGEOS-V1-WORLD-300` | The user performs the V1 workflow through one coherent cockpit | 🔒 `FORGEOS-V1-SESSION-300`<br>🔒 `FORGEOS-V1-PROJECT-300`<br>🔒 `FORGEOS-V1-CODE-300`<br>🔒 `FORGEOS-V1-TERMINAL-300`<br>🔒 `FORGEOS-V1-GIT-300`<br>🔒 `FORGEOS-V1-NYX-300`<br>🔒 `FORGEOS-V1-VERIFY-300` |
| 🔒 LOCKED | `FORGEOS-V1-RECOVERY-300` | The user resumes work after process or shell failure | 🔒 `FORGEOS-V1-RECOVERY-200`<br>🔒 `FORGEOS-V1-WORLD-300` |
| 🔒 LOCKED | `FORGEOS-V1-DIST-300` | The user installs, updates, and boots ForgeOS on a clean supported host | 🔒 `FORGEOS-V1-DIST-200`<br>🔒 `FORGEOS-V1-SESSION-300`<br>🔒 `FORGEOS-V1-RECOVERY-300` |


# Tier 2 — Functional V1 systems

Functional subsystems assembled from local mechanisms.

| Status | Skill | What it is | Direct prerequisites / blockers |
|---|---|---|---|
| 🔨 ACTIVE | `FORGEOS-V1-PROJECT-200` | Persistent project registry and workspace restoration | ✅ `FORGEOS-V1-PROJECT-100` |
| 🟢 AVAILABLE | `FORGEOS-V1-SESSION-200` | Dedicated ForgeOS session bootstrap | ✅ `FORGEOS-V1-SESSION-100` |
| 🔒 LOCKED | `FORGEOS-V1-SESSION-201` | Managed ForgeOS and Nyx service lifecycle | ✅ `FORGEOS-V1-SESSION-100`<br>🟢 `FORGEOS-V1-NYX-100` |
| 🔒 LOCKED | `FORGEOS-V1-FILE-200` | Repository file tree and search | ✅ `FORGEOS-V1-FILE-100`<br>🔨 `FORGEOS-V1-PROJECT-200` |
| 🔒 LOCKED | `FORGEOS-V1-EDITOR-200` | Multi-buffer file editing and atomic save | ✅ `FORGEOS-V1-EDITOR-100`<br>🔒 `FORGEOS-V1-FILE-200` |
| 🔒 LOCKED | `FORGEOS-V1-EDITOR-201` | Rust syntax and language-intelligence integration | 🔒 `FORGEOS-V1-EDITOR-200`<br>✅ `FORGEOS-V1-PARSER-100`<br>✅ `FORGEOS-V1-LSP-100` |
| 🔒 LOCKED | `FORGEOS-V1-TERMINAL-200` | Managed embedded terminal sessions | ✅ `FORGEOS-V1-TERMINAL-100`<br>🔨 `FORGEOS-V1-PROJECT-200` |
| 🔒 LOCKED | `FORGEOS-V1-COMMAND-200` | Registered project command execution and output history | ✅ `FORGEOS-V1-COMMAND-100`<br>🔒 `FORGEOS-V1-TERMINAL-200` |
| 🔒 LOCKED | `FORGEOS-V1-GIT-200` | Real Git status, branch, revision, and diff inspection | ✅ `FORGEOS-V1-GIT-100`<br>🔨 `FORGEOS-V1-PROJECT-200` |
| 🔒 LOCKED | `FORGEOS-V1-GIT-201` | Safe Git mutation and isolated worktree control | ✅ `FORGEOS-V1-GIT-101`<br>🔒 `FORGEOS-V1-GIT-200`<br>✅ `FORGEOS-V1-PATCH-100` |
| 🔒 LOCKED | `FORGEOS-V1-NYX-200` | Local model selection and Nyx conversation lifecycle | 🟢 `FORGEOS-V1-NYX-100`<br>🔒 `FORGEOS-V1-SESSION-201` |
| 🔒 LOCKED | `FORGEOS-V1-NYX-201` | Project-aware Nyx read tools | 🔒 `FORGEOS-V1-NYX-200`<br>🔒 `FORGEOS-V1-FILE-200`<br>🔒 `FORGEOS-V1-GIT-200` |
| 🔒 LOCKED | `FORGEOS-V1-NYX-202` | Safe registered commands and exact checkpoint resume | 🔒 `FORGEOS-V1-NYX-101`<br>🔒 `FORGEOS-V1-NYX-201`<br>🔒 `FORGEOS-V1-COMMAND-200` |
| 🔒 LOCKED | `FORGEOS-V1-AGENT-200` | OpenAI heavyweight task dispatch | 🔒 `FORGEOS-V1-AGENT-100`<br>🔒 `FORGEOS-V1-NYX-200`<br>🔒 `FORGEOS-V1-GIT-201` |
| 🔒 LOCKED | `FORGEOS-V1-AGENT-201` | Returned patch intake, review, and controlled application | 🔒 `FORGEOS-V1-AGENT-200`<br>✅ `FORGEOS-V1-PATCH-100`<br>🔒 `FORGEOS-V1-GIT-201` |
| 🔒 LOCKED | `FORGEOS-V1-VERIFY-200` | Version-bound build and test result records | 🔒 `FORGEOS-V1-COMMAND-200`<br>🔒 `FORGEOS-V1-GIT-200`<br>✅ `FORGEOS-V1-STATE-000` |
| 🔒 LOCKED | `FORGEOS-V1-WORLD-200` | Basic Bevy shell and truthful status HUD | 🟢 `FORGEOS-V1-WORLD-100`<br>🔨 `FORGEOS-V1-PROJECT-200`<br>🔒 `FORGEOS-V1-TERMINAL-200`<br>🔒 `FORGEOS-V1-GIT-200`<br>🔒 `FORGEOS-V1-NYX-200`<br>🔒 `FORGEOS-V1-VERIFY-200` |
| 🔒 LOCKED | `FORGEOS-V1-RECOVERY-200` | Durable workspace and service recovery | 🔒 `FORGEOS-V1-RECOVERY-100`<br>🔨 `FORGEOS-V1-PROJECT-200`<br>🔒 `FORGEOS-V1-SESSION-201`<br>🔒 `FORGEOS-V1-TERMINAL-200`<br>🔒 `FORGEOS-V1-NYX-200` |
| 🔒 LOCKED | `FORGEOS-V1-DIST-200` | Reproducible ForgeOS session package | 🔒 `FORGEOS-V1-SESSION-200`<br>🔒 `FORGEOS-V1-SESSION-201`<br>🔒 `FORGEOS-V1-WORLD-200` |


# Tier 1 — Local mechanisms

Concrete local mechanisms and adapters that make the systems real.

| Status | Skill | What it is | Direct prerequisites / blockers |
|---|---|---|---|
| ✅ CLOSED | `FORGEOS-V1-PROJECT-100` | Validated project manifest and repository identity | ✅ `FORGEOS-V1-CONTRACT-000`<br>✅ `FORGEOS-V1-STATE-000`<br>✅ `FORGEOS-V1-PATH-000`<br>✅ `FORGEOS-V1-GUARD-002` |
| ✅ CLOSED | `FORGEOS-V1-SESSION-100` | Session and managed-service lifecycle contract | ✅ `FORGEOS-V1-CONTRACT-000`<br>✅ `FORGEOS-V1-PROCESS-000`<br>✅ `FORGEOS-V1-GUARD-002` |
| ✅ CLOSED | `FORGEOS-V1-FILE-100` | Boundary-safe file access and atomic write | ✅ `FORGEOS-V1-PATH-000`<br>✅ `FORGEOS-V1-STATE-000` |
| ✅ CLOSED | `FORGEOS-V1-EDITOR-100` | Editor buffer identity and dirty-state model | ✅ `FORGEOS-V1-FILE-100`<br>✅ `FORGEOS-V1-CONTRACT-000` |
| ✅ CLOSED | `FORGEOS-V1-PARSER-100` | Incremental Tree-sitter parsing adapter | ✅ `FORGEOS-V1-ARCH-001`<br>✅ `FORGEOS-V1-FILE-100` |
| ✅ CLOSED | `FORGEOS-V1-LSP-100` | Rust Analyzer process and JSON-RPC adapter | ✅ `FORGEOS-V1-CONTRACT-000`<br>✅ `FORGEOS-V1-PROCESS-000`<br>✅ `FORGEOS-V1-GUARD-002` |
| ✅ CLOSED | `FORGEOS-V1-TERMINAL-100` | PTY spawn, I/O, resize, and termination | ✅ `FORGEOS-V1-PROCESS-000`<br>✅ `FORGEOS-V1-PATH-000`<br>✅ `FORGEOS-V1-GUARD-002` |
| ✅ CLOSED | `FORGEOS-V1-COMMAND-100` | Registered command definition and execution policy | ✅ `FORGEOS-V1-PROCESS-000`<br>✅ `FORGEOS-V1-PATH-000`<br>✅ `FORGEOS-V1-CONTRACT-000` |
| ✅ CLOSED | `FORGEOS-V1-GIT-100` | Read-only Git adapter | ✅ `FORGEOS-V1-PATH-000`<br>✅ `FORGEOS-V1-PROCESS-000`<br>✅ `FORGEOS-V1-GUARD-002` |
| ✅ CLOSED | `FORGEOS-V1-GIT-101` | Git mutation and worktree primitives | ✅ `FORGEOS-V1-GIT-100`<br>✅ `FORGEOS-V1-CONTRACT-000` |
| 🟢 AVAILABLE | `FORGEOS-V1-NYX-100` | Nyx health and versioned client protocol | ✅ `FORGEOS-V1-CONTRACT-000`<br>✅ `FORGEOS-V1-PROCESS-000`<br>✅ `FORGEOS-V1-GUARD-002` |
| 🔒 LOCKED | `FORGEOS-V1-NYX-101` | Permission grant, checkpoint, and immutable resume token | 🟢 `FORGEOS-V1-NYX-100`<br>✅ `FORGEOS-V1-STATE-000`<br>✅ `FORGEOS-V1-HASH-000` |
| 🔒 LOCKED | `FORGEOS-V1-AGENT-100` | Remote-agent task and budget record | 🟢 `FORGEOS-V1-NYX-100`<br>✅ `FORGEOS-V1-PATH-000`<br>✅ `FORGEOS-V1-STATE-000`<br>✅ `FORGEOS-V1-HASH-000` |
| ✅ CLOSED | `FORGEOS-V1-PATCH-100` | Patch identity, base validation, and safe application primitive | ✅ `FORGEOS-V1-PATH-000`<br>✅ `FORGEOS-V1-STATE-000`<br>✅ `FORGEOS-V1-HASH-000` |
| 🟢 AVAILABLE | `FORGEOS-V1-WORLD-100` | Source-backed view projection and input action routing | ✅ `FORGEOS-V1-ARCH-001`<br>✅ `FORGEOS-V1-CONTRACT-000`<br>✅ `FORGEOS-V1-GUARD-001`<br>✅ `FORGEOS-V1-GUARD-002` |
| 🟢 AVAILABLE | `FORGEOS-V1-RECOVERY-100` | Workspace snapshot and crash-journal primitive | ✅ `FORGEOS-V1-STATE-000`<br>✅ `FORGEOS-V1-PROCESS-000`<br>✅ `FORGEOS-V1-HASH-000` |


# Tier 0 — Atomic foundations and guards

The bedrock: architecture, contracts, guards, process/path/state foundations, and hashing.

| Status | Skill | What it is | Direct prerequisites / blockers |
|---|---|---|---|
| ✅ CLOSED | `FORGEOS-V1-ARCH-000` | Rust workspace and authority crate skeleton | None |
| ✅ CLOSED | `FORGEOS-V1-ARCH-001` | Scoped module hierarchy and public routing | ✅ `FORGEOS-V1-ARCH-000` |
| ✅ CLOSED | `FORGEOS-V1-GUARD-000` | Authored source module-size verifier | ✅ `FORGEOS-V1-ARCH-000` |
| ✅ CLOSED | `FORGEOS-V1-GUARD-001` | Forge Core purity guard | ✅ `FORGEOS-V1-ARCH-000`<br>✅ `FORGEOS-V1-ARCH-001` |
| ✅ CLOSED | `FORGEOS-V1-GUARD-002` | Cross-subsystem seam direction guards | ✅ `FORGEOS-V1-ARCH-001`<br>✅ `FORGEOS-V1-CONTRACT-000` |
| ✅ CLOSED | `FORGEOS-V1-CONTRACT-000` | Stable IDs, typed errors, and versioned envelopes | ✅ `FORGEOS-V1-ARCH-000` |
| ✅ CLOSED | `FORGEOS-V1-STATE-000` | Atomic versioned local persistence | ✅ `FORGEOS-V1-CONTRACT-000` |
| ✅ CLOSED | `FORGEOS-V1-PATH-000` | Canonical repository path and boundary identity | ✅ `FORGEOS-V1-CONTRACT-000` |
| ✅ CLOSED | `FORGEOS-V1-PROCESS-000` | Stable process lifecycle and cancellation model | ✅ `FORGEOS-V1-CONTRACT-000` |
| ✅ CLOSED | `FORGEOS-V1-HASH-000` | Stable artifact and request hashing | ✅ `FORGEOS-V1-CONTRACT-000` |


---

## Completed foundation chain

1. ✅ `FORGEOS-V1-ARCH-000` — Rust workspace and authority crate skeleton
2. ✅ `FORGEOS-V1-ARCH-001` — Scoped module hierarchy and public routing
3. ✅ `FORGEOS-V1-GUARD-000` — Authored source module-size verifier
4. ✅ `FORGEOS-V1-GUARD-001` — Forge Core purity guard
5. ✅ `FORGEOS-V1-CONTRACT-000` — Stable IDs, typed errors, and versioned envelopes
6. ✅ `FORGEOS-V1-GUARD-002` — Cross-subsystem seam direction guards
7. ✅ `FORGEOS-V1-PROCESS-000` — Stable process lifecycle and cancellation model
8. ✅ `FORGEOS-V1-PATH-000` — Canonical repository path and boundary identity
9. ✅ `FORGEOS-V1-STATE-000` — Atomic versioned local persistence
10. ✅ `FORGEOS-V1-HASH-000` — Stable artifact and request hashing
11. ✅ `FORGEOS-V1-PROJECT-100` — Validated project manifest and repository identity
12. ✅ `FORGEOS-V1-FILE-100` — Boundary-safe file access and atomic write
13. ✅ `FORGEOS-V1-EDITOR-100` — Editor buffer identity and dirty-state model
14. ✅ `FORGEOS-V1-PARSER-100` — Incremental Tree-sitter parsing adapter
15. ✅ `FORGEOS-V1-LSP-100` — Rust Analyzer process and JSON-RPC adapter
16. ✅ `FORGEOS-V1-TERMINAL-100` — PTY spawn, I/O, resize, and termination
17. ✅ `FORGEOS-V1-COMMAND-100` — Registered command definition and execution policy
18. ✅ `FORGEOS-V1-SESSION-100` — Session and managed-service lifecycle contract
19. ✅ `FORGEOS-V1-GIT-100` — Read-only Git adapter
20. ✅ `FORGEOS-V1-GIT-101` — Git mutation and worktree primitives
21. ✅ `FORGEOS-V1-PATCH-100` — Patch identity, base validation, and safe application primitive
22. 🔨 `FORGEOS-V1-PROJECT-200` — Persistent project registry and workspace restoration

## Reading this list correctly

- A higher-tier skill being locked is normal. It is not “behind schedule”; its ingredients do not exist yet.
- `AVAILABLE` does not mean selected. The execution router chooses one active skill using prerequisites, dependency depth, user value, blast radius, and conflict rules.
- Closing a lower-tier skill may unlock several branches at once. The workflow can switch branches before climbing tiers.
- Tier adjacency does not select work. The live router does.
- The final V1 is not earned until the Tier 5 apex becomes `RELEASE_EARNED` through the real boot, development, recovery, and self-hosting journey.

## Update rule

This file is a mandatory status mirror named by `docs/ForgeOS_header.md`. It must be updated in the same patch whenever a skill changes state or the baton/frontier changes. It reports authority; it does not create authority.

When a skill changes state, update:

1. Its row in this document.
2. The snapshot counts.
3. Any dependent row whose prerequisites have all become `CLOSED`.
4. The current baton section.

Do not mark a skill complete because neighboring code exists. Use the canonical closure record and operator proof.
