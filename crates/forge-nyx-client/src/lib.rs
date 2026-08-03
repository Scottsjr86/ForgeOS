//! ForgeOS client boundary for the separate nyx_server model host.
//!
//! Public routes are explicit. Capability implementations remain owned by
//! their named modules and are added only by the skills that prove them.

pub mod patches;
pub mod permissions;
pub mod protocol;
pub mod transport;
