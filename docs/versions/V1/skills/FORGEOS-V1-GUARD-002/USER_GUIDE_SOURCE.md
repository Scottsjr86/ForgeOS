# FORGEOS-V1-GUARD-002 User Guide Source

Status: `CLOSED_SOURCE`
Capability: cross-subsystem dependency-direction verification

## What the operator can do

The operator can verify that every reviewed ForgeOS authority package reaches only
the other ForgeOS packages allowed by the exact V1 dependency-direction matrix.

## Command

From the ForgeOS repository root:

```bash
cargo run -p forge-guards --bin forge-seam-direction -- --root .
```

## Expected clean result

The closed V1 foundation ends with:

```text
FORGE_SEAM_DIRECTION_SUMMARY status=PASS packages=12 routes=42 forbidden=0 policy=exact-reviewed-subsystem-reachability-v1
```

## Visible states

- `ALLOWED`: the package or route belongs to the exact reviewed graph.
- `FORBIDDEN`: an unknown package or undeclared reachable subsystem was found.
- `PASS`: the complete reviewed graph is present and legal.
- `FAIL`: at least one package or route violates the matrix.
- `FORGE_SEAM_DIRECTION_ERROR`: Cargo evidence could not be obtained or classified.

## Fixed policy

The verifier follows Cargo's real normal and build reachability graph with all
features and targets selected. It does not parse documentation, use a substring
blacklist, or offer a command-line exemption.

An external dependency may exist behind its owning subsystem. It may not create an
undeclared path to another ForgeOS authority package. Renaming an adapter does not
hide the transitive route.

## Errors and recovery

- Forbidden route: move the dependency to its owning direction or narrow the
  adapter boundary.
- Unknown ForgeOS workspace package: register and review the package explicitly or
  remove it from the workspace.
- Missing reviewed package: restore the required authority package before treating
  the graph as valid.
- Cargo failure: repair the workspace or dependency graph, then rerun the same
  command.

## Nyx interaction

None. This is a local structural verifier and does not contact or host
`nyx_server`.

## Current V1 limitations

The verifier proves package-level reachability only. It does not inspect function
calls, runtime state ownership, semantic correctness, or user-facing behavior.
