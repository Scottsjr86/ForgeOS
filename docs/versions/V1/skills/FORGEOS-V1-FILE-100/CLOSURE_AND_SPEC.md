# FORGEOS-V1-FILE-100 Closure and Specification

Status: `CLOSED`
Capability: boundary-safe file access and atomic write
Closed: `2026-08-02`
Source authority: `Forge_OS_V1_base_21.tar`
Source archive SHA-256: `a8633b432379cff72cd432832b60dc10369430be62b5c23d46f1578e663a3c6c`
Git revision: unavailable because the supplied source archive contains no `.git` metadata

## Capability statement

Forge Project now provides one manifest-bound raw file path. Reads preserve exact
bytes. Creates and replacements resolve only through approved project roots, stage
bytes in the target directory, sync before atomic replacement, preserve existing
mode bits, and require an explicit missing or exact-revision precondition.

The accepted path rejects repository mismatches, denied roots, traversal, symlinks,
directories, stale revisions, oversized payloads, and unsafe boundary changes. A
pre-commit failure leaves original bytes intact and removes its staging file.

## Public contract

```text
forge_project::files::ProjectFileAccess
forge_project::files::FileSnapshot
forge_project::files::FileRevision
forge_project::files::FileExpectation
forge_project::files::FileWriteResult
forge_project::files::WriteDurability
forge_project::files::ProjectFileError
```

## Accepted operator evidence

The operator ran the behavior-only CI route and reported:

```text
FORGE_SEAM_DIRECTION_SUMMARY status=PASS packages=12 routes=42 forbidden=0 policy=exact-reviewed-subsystem-reachability-v1
FORGE_CORE_PURITY_SUMMARY status=PASS packages=2 allowed=2 forbidden=0 policy=exact-reviewed-production-graph-v1
FORGE_SOURCE_SIZE_SUMMARY status=PASS modules=71 pass=71 warn=0 fail=0 warnings_denied=true
CARGO_TEST_SUMMARY status=PASS suites=44 passed=105 failed=0 ignored=0 measured=0 filtered_out=0
CI RESULT: PASS
```

CI remains limited to behavioral tests, golden locks, and structural guards. It
contains no documentation verification, Git-state check, or formatting gate.

## Negative and failure-path results

Focused fixtures cover raw non-UTF8 names and bytes, missing-file creation, exact
replacement, stale-revision conflict, denied roots, wrong repository identities,
symlink rejection, directory rejection, oversized bytes, staged failure cleanup,
and preservation of original bytes before commit.

## Regression locks

Later editor and file-tree capabilities must use the manifest-bound file path,
preserve raw bytes, retain exact revision preconditions, reject boundary escape,
and never convert a failed pre-commit write into partial target content.

## Explicit non-claims

This closure does not provide file-tree discovery, search, editor buffers, parsing,
terminal or command execution, Git behavior, project restoration, Nyx, recovery,
or Forge World presentation.
