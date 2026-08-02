# FORGEOS-V1-HASH-000 Closure and Specification

Status: `CLOSED`
Capability: stable artifact and request hashing
Closed: `2026-08-02`
Source authority: `Forge_OS_V1_base_19.tar`
Source archive SHA-256: `b8fefe29556b977062607b8bda6c067cdf1f9e215f84783d21a16b0c8a65b197`
Git revision: unavailable because the supplied source archive contains no `.git` metadata

## Capability statement

Forge Protocol now owns a std-only SHA-256 implementation, strict lowercase digest
text, semantic domain separation, deterministic structured-field ordering, and
verification of canonical bytes. Forge Core applies that contract to canonical
state records and result payloads without expanding the reviewed Core dependency
graph.

## Public contract

```text
forge_protocol::hashes::HashDomain
forge_protocol::hashes::ContentHash
forge_protocol::hashes::CanonicalHashInput
forge_protocol::hashes::HashContractError
forge_protocol::hashes::hash_canonical_bytes
forge_protocol::hashes::verify_canonical_bytes
forge_core::hashing::state_record_hash
forge_core::hashing::result_payload_hash
```

## Accepted operator evidence

The operator ran the behavior-only CI route and reported:

```text
FORGE_SEAM_DIRECTION_SUMMARY status=PASS packages=12 routes=42 forbidden=0 policy=exact-reviewed-subsystem-reachability-v1
FORGE_CORE_PURITY_SUMMARY status=PASS packages=2 allowed=2 forbidden=0 policy=exact-reviewed-production-graph-v1
FORGE_SOURCE_SIZE_SUMMARY status=PASS modules=66 pass=66 warn=0 fail=0 warnings_denied=true
CARGO_TEST_SUMMARY status=PASS suites=41 passed=85 failed=0 ignored=0 measured=0 filtered_out=0
CI RESULT: PASS
```

The official CI route contains only behavioral tests, golden locks, and structural
guards. It contains no documentation verification, Git checks, or formatting gate.

## Negative and compatibility results

Tests cover standard SHA-256 vectors, identical input, reordered structured fields,
changed bytes, cross-domain separation, malformed digest text, duplicate fields,
and deliberate corruption. The V1 structured-request golden remains byte locked.

## Regression locks

Canonical identities may depend only on declared canonical bytes and semantic
domain. Host paths, timestamps, locale, display labels, and unstable map order may
not enter identity accidentally. Existing V1 hash contracts must remain byte stable.

## Explicit non-claims

This closure does not provide project manifests, repository file writes, patch
application, recovery journals, Nyx permissions, remote-agent records, terminal,
Git, session, LSP, or Forge World behavior.
