# FORGEOS-V1-FILE-200 Closure and Specification

Status: `ACTIVE_OPERATOR_VALIDATION_PENDING`
Capability: Repository file tree and search
Active slice: `FORGEOS-V1-FILE-200-SLICE-001`
Source authority: `Forge_OS_V1_base_40.tar`
Source archive SHA-256: `88aaf0f0553e9179fb1bedab9043b306346fefbf1a33face6679064598373c3c`
Git revision: `bd9bd36167dc46978141bbf84dd4a598c0468ab5`

## Capability statement

ForgeOS now has a manifest-bound, read-only repository browser that projects only
approved roots, preserves exact repository-relative path bytes, safely opens known
files through the existing FILE-100 authority, and performs deterministic exact-text
search without following symlinks or crossing repository filesystem boundaries.

## Public contract

```text
forge_project::repository_view::RepositoryBrowseScope
forge_project::repository_view::RepositoryEntryKind
forge_project::repository_view::RepositoryTreeEntry
forge_project::repository_view::RepositoryScanIssue
forge_project::repository_view::RepositoryTreeSnapshot
forge_project::repository_view::RepositoryBrowser
forge_project::repository_view::RepositoryBrowseError
forge_project::text_search::TextSearchQuery
forge_project::text_search::TextSearchMatch
forge_project::text_search::RepositorySearchIssue
forge_project::text_search::TextSearchReport
```

## Intended behavior

- approved project roots are the only discoverable tree scopes;
- deterministic traversal sorts exact platform path bytes;
- non-UTF-8 child names remain exact and are never replaced for display convenience;
- every discovered child is re-resolved through the canonical repository boundary;
- the object observed through the pinned parent directory must match the resolved object;
- symlinks are reported and skipped rather than followed;
- cross-device mount entries and canonicalization escapes remain rejected by the
  existing repository boundary;
- regular files open through the existing raw FILE-100 read path;
- search uses exact UTF-8 query bytes while reporting byte offsets, one-based line
  numbers, and byte columns;
- no-match results are valid empty reports;
- unreadable, oversized, changed, or unsupported entries remain explicit issues;
- match limits report truncation instead of pretending the result is complete;
- browsing and search do not write repository files or project records.

## Regression locks

```text
crates/forge-project/tests/repository_browse_search.rs
crates/forge-app/tests/repository_navigation.rs
crates/forge-project/src/text_search.rs unit tests
python3 scripts/run_ci.py
```

The behavioral matrix covers deterministic approved-root projection, denied paths,
exact raw file opening, editor-buffer composition, known-content search, no-match
search, match truncation, symlink escape rejection, oversized-file reporting,
non-UTF-8 paths, and repository immutability.

## Operator validation still required

Run the canonical behavior-only CI entrypoint:

```bash
python3 scripts/run_ci.py
```

The skill remains active until the operator returns green structural guards and
Rust tests for this slice.

## Explicit non-claims

This skill does not provide multi-buffer save, external-change conflict UI, language
navigation, ignore-file semantics, fuzzy search, regular expressions, Git-aware file
filtering, or a rendered file-tree panel. Those capabilities belong to later editor,
Git, and Forge World skills.
