# FORGEOS-V1-ARCH-001 User Guide Source

Status: `CLOSED`
Audience: ForgeOS developers and maintainers

## What this capability does

This capability gives each ForgeOS crate a predictable public route shape and keeps
the executable entrypoint from becoming a business-logic junk drawer. Developers
can locate subsystem ownership from the crate and named module path rather than
searching a monolithic root.

## Access and usage

Import behavior through the owning crate and named route. Representative paths are:

```rust
use forge_core::projects;
use forge_protocol::identities;
use forge_terminal::commands;
use forge_world::presentation;
```

Run the architecture checks from the repository root:

```bash
cargo fmt --all -- --check
cargo check --workspace
cargo test --workspace
```

## Adding behavior

- Put behavior in the crate that owns the capability.
- Use a specific named module for the capability or domain concept.
- Keep `lib.rs` focused on documentation and explicit `pub mod` routes.
- Keep `main.rs` limited to delegation into the composition route.
- Do not use wildcard re-exports or generic modules such as `common`, `misc`, or
  `utils` to conceal unclear ownership.

## Expected result

Public routes compile, imports remain explicit, and module ownership can be read
without opening implementation details.

## Errors and recovery

- A broken import usually means the route was renamed without migration. Restore the
  public route or version and migrate it explicitly.
- Behavior accumulating in `lib.rs`, `main.rs`, or `composition` must move to its
  owning module before the architecture remains accepted.
- A concept that does not have an obvious owner is an architecture question, not a
  reason to create a catch-all module.

## Nyx interaction

None. `forge-nyx-client::{protocol, transport}` are ForgeOS-side route names only;
this capability does not start, host, or contact `nyx_server`.

## Current V1 limitations

The route names are structural. Most modules intentionally contain no product
behavior yet and receive implementations only through their own capability skills.
