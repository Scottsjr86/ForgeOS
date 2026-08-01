# FORGEOS-V1-GUARD-000 User Guide Source

Status: `CLOSED`
Audience: ForgeOS developers and maintainers

## What this capability does

`forge-source-size` prevents authored Rust modules from quietly becoming thousand-
line junk drawers. It reports every scanned module, warns after 1000 physical lines,
and fails at 1201 or more.

## Run the guard

From the ForgeOS repository root:

```bash
cargo run -p forge-guards --bin forge-source-size -- --root .
```

For the mandatory later-skill closure gate:

```bash
cargo run -p forge-guards --bin forge-source-size -- --root . --deny-warnings
```

The second form returns failure for either `WARN` or `FAIL`, while preserving the
reported classification.

## Expected result

A clean current workspace ends with a summary shaped like:

```text
FORGE_SOURCE_SIZE_SUMMARY status=PASS modules=<n> pass=<n> warn=0 fail=0 warnings_denied=true
```

## Fixed policy

The guard scans authored `.rs` files in stable relative-path order. It has no
project ignore file or command-line exclusion option. Fixed generated, vendored,
third-party, target, and Git directories are excluded, and fixed generated-source
markers exclude generated files.

## Errors and recovery

- `WARN`: split or reorganize the module before closing the active source skill.
- `FAIL`: the module is at least 1201 physical lines and must be reduced.
- `FORGE_SOURCE_SIZE_ERROR`: fix the invalid root, I/O problem, or source symlink,
  then rerun the same command.
- Usage failure: run `forge-source-size --help` and correct the command.

Do not add an ignore rule to silence authored source. Fix the ownership boundary.

## Nyx interaction

None. This is a local structural verifier and does not contact or host `nyx_server`.

## Current V1 limitations

The verifier counts physical lines. It does not measure complexity, coupling,
duplicate logic, test quality, or semantic cohesion.
