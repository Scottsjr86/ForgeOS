# FORGEOS-V1-GUARD-001 User Guide Source

Status: `CLOSED_SOURCE`
Capability: Forge Core dependency-purity verification

## What the operator can do

The operator can verify that `forge-core` reaches only packages intentionally
reviewed as pure protocol dependencies through Cargo normal and build edges.

## Command

From the ForgeOS repository root:

```bash
cargo run -p forge-guards --bin forge-core-purity -- --root .
```

## Expected clean result

The closed V1 foundation currently ends with:

```text
FORGE_CORE_PURITY_SUMMARY status=PASS packages=2 allowed=2 forbidden=0 policy=exact-reviewed-production-graph-v1
```

The two accepted packages are `forge-core` and `forge-protocol`.

## Visible states

- `ALLOWED`: the package belongs to the exact reviewed pure graph.
- `FORBIDDEN`: the package is reachable from Core but is not reviewed.
- `PASS`: no forbidden package is reachable.
- `FAIL`: at least one forbidden package is reachable.
- `FORGE_CORE_PURITY_ERROR`: the graph could not be obtained or classified.

## Fixed policy

The verifier reads Cargo's real normal and build dependency graph. It does not
parse documentation, infer safety from package-name substrings, or allow unknown
packages by default.

There is no command-line package exemption. A new Core dependency requires an
intentional architecture review and updated proof, not a local ignore switch.

## Errors and recovery

- Forbidden direct package: remove the dependency or move effectful behavior to
  its owning adapter or subsystem.
- Forbidden transitive package: inspect the full chain and remove the hidden
  dependency from the Core graph.
- Missing manifest or invalid root: run from the repository root or pass the
  correct root.
- Cargo failure: repair the manifest or dependency graph, then rerun the same
  command.

Do not rename an effectful crate to make it look generic. The guard follows the
real graph and still rejects it.

## Nyx interaction

None. This is a local structural verifier and does not contact or host
`nyx_server`.

## Current V1 limitations

The guard proves only the reviewed package boundary. It does not inspect function
calls, runtime effects hidden inside an accepted package, feature-specific graph
changes not selected by the command, or semantic correctness.
