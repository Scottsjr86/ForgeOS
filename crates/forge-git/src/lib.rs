//! ForgeOS Git inspection, mutation, and worktree control.
//!
//! Public routes are explicit. Read-only inspection and typed mutation/worktree
//! primitives remain separate contracts with fixed native command surfaces.

pub mod diff;
pub mod mutation;
mod patch_format;
pub mod patches;
pub mod repository;
pub mod status;
pub mod types;
pub mod worktree;
