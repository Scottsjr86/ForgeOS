# FORGEOS-V1-PROJECT-100 Closure and Specification

Status: `CLOSED`
Capability: validated project manifest and repository identity
Closed: `2026-08-02`
Source authority: `Forge_OS_V1_base_20.tar`
Source archive SHA-256: `5923e493f18fdeac68f521fd7e81bb4426fe780fc76857cb8a0ec55aa1bbfb00`
Git revision: unavailable because the supplied source archive contains no `.git` metadata

## Capability statement

Forge Core owns deterministic versioned project-manifest bytes. Forge Project binds
those manifests to verified repository directory objects while enforcing unique
project and repository identities, declared allowed roots, moved-root identity, and
explicit rejection of malformed or duplicate input.

## Public contract

```text
forge_core::projects::ProjectManifest
forge_core::projects::ManifestCommand
forge_core::projects::ProjectSetting
forge_core::projects::LanguageProfile
forge_core::projects::AllowedProjectRoot
forge_core::projects::ProjectManifestError
forge_project::registry::ProjectRegistry
forge_project::registry::RegisteredProject
forge_project::registry::ProjectRegistryError
```

## Accepted operator evidence

The operator ran the behavior-only CI route and reported:

```text
FORGE_SEAM_DIRECTION_SUMMARY status=PASS packages=12 routes=42 forbidden=0 policy=exact-reviewed-subsystem-reachability-v1
FORGE_CORE_PURITY_SUMMARY status=PASS packages=2 allowed=2 forbidden=0 policy=exact-reviewed-production-graph-v1
FORGE_SOURCE_SIZE_SUMMARY status=PASS modules=68 pass=68 warn=0 fail=0 warnings_denied=true
CARGO_TEST_SUMMARY status=PASS suites=43 passed=95 failed=0 ignored=0 measured=0 filtered_out=0
CI RESULT: PASS
```

The CI route contains only behavioral tests, golden locks, and structural guards.
It contains no documentation verification, Git checks, or formatting gate.

## Negative and compatibility results

Tests cover exact V1 manifest bytes, equivalent reopen behavior, unknown optional
fields, unknown required fields, unsupported schema, malformed bytes, duplicate
project IDs, duplicate repository IDs, duplicate commands, missing or invalid
allowed roots, repository replacement, and relocation of the same directory object.

## Regression locks

Project identity is never derived from display name, path text, list order, or
filesystem discovery order. Unchanged manifests remain byte stable. Repository
relocation is accepted only when filesystem object identity remains the same.

## Explicit non-claims

This closure does not provide repository file reads or writes, project-registry
persistence, terminal or command execution, Git behavior, sessions, LSP, Nyx,
recovery orchestration, or Forge World presentation.
