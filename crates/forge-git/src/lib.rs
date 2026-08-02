//! ForgeOS Git inspection, mutation, and worktree control.
//!
//! Public routes are explicit. `FORGEOS-V1-GIT-100` adds read-only native Git
//! inspection only; mutation remains locked behind later skills.

pub mod diff;
pub mod repository;
pub mod status;
pub mod types;
pub mod worktree;
