//! Shared ForgeOS messages, identities, errors, events, and seam contracts.
//!
//! Public routes are explicit. Canonical identifiers are typed opaque values;
//! request, result, and error traffic uses the deterministic V1 envelope codec.

pub mod envelopes;
pub mod errors;
pub mod events;
pub mod hashes;
pub mod identities;
pub mod intents;
pub mod patches;
pub mod paths;
pub mod processes;

mod sha256;
mod wire;
