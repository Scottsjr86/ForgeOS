# FORGEOS-V1-HASH-000 User Guide Source

Status: `CLOSED_SOURCE`
Capability: stable artifact and request hashing

## What the operator can do

ForgeOS can assign stable SHA-256 identities to declared canonical file, patch,
tool-request, snapshot, and result bytes. Structured field insertion order does
not change identity, while changed bytes or a changed semantic domain do.

## Validation command

```bash
python3 scripts/run_ci.py
```

## Visible outcomes

- Standard SHA-256 vectors pass.
- Lowercase 64-character digest text round-trips exactly.
- Uppercase, malformed, and wrong-length digest text is rejected.
- Reordered declared fields produce the same identity.
- Duplicate fields are rejected.
- Changed or corrupt payloads fail verification.
- Forge Core purity and seam direction remain unchanged.

## CI boundary

CI is limited to behavior, goldens, Rust tests, and structural guards. Markdown,
Git state, and formatting are not CI authority and cannot award closure.

## Current V1 limitations

Hash identity does not apply patches, open repositories, persist recovery journals,
or grant Nyx or agent authority. Those remain separate routed capabilities.
