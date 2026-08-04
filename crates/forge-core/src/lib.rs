//! Pure canonical ForgeOS project, workspace, capability, and mission state.
//!
//! Public routes are explicit. Capability implementations remain owned by
//! their named modules and are added only by the skills that prove them.

pub mod capabilities;
pub mod command_codec;
pub mod commands;
pub mod hashing;
pub mod missions;
pub mod project_registry;
pub mod projects;
pub mod recovery;
pub mod state;
pub mod workspaces;
