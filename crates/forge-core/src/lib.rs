//! Pure canonical ForgeOS project, workspace, capability, and mission state.
//!
//! Public routes are explicit. Capability implementations remain owned by
//! their named modules and are added only by the skills that prove them.

pub mod capabilities;
pub mod missions;
pub mod projects;
pub mod workspaces;
