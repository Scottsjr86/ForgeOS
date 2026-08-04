# FORGEOS-V1-FILE-200 User Guide Source

Status: `ACTIVE_OPERATOR_VALIDATION_PENDING`

## What this capability does

ForgeOS can enumerate the approved parts of a registered repository, open an exact
file from that tree, and search known text across safe regular files. Results identify
files by stable repository identity plus exact repository-relative path, not by list
position or display text.

## Safety behavior

- paths outside the project manifest's approved roots are rejected;
- `..`, absolute paths, repeated separators, and other noncanonical requests are
  rejected before filesystem access;
- symlinks are shown as rejected issues and are never followed;
- unexpected mounts remain outside the repository boundary;
- unreadable, oversized, changed, and special filesystem entries remain visible as
  explicit issues rather than disappearing silently;
- search is read-only and never rewrites bytes or project state.

## Search behavior

V1 search is exact and case-sensitive. A result reports its repository-relative path,
byte offset, byte length, one-based line number, and byte column. A valid query with no
matches returns an empty result. A configured match limit returns the matches collected
so far with `truncated=true`.

## What comes later

The rendered file-tree panel, multi-buffer save, external conflict handling, language
navigation, fuzzy search, ignore rules, and Git-aware filtering remain later skills.
