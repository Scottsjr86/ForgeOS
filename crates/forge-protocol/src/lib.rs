//! Shared ForgeOS messages, identities, errors, events, and seam contracts.
//!
//! Public routes are explicit. Canonical identifiers are typed opaque values;
//! request, result, and error traffic uses the deterministic V1 envelope codec.

pub mod envelopes;
pub mod errors;
pub mod events;
pub mod identities;
pub mod processes;

mod wire;
