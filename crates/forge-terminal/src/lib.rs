//! ForgeOS PTY and registered-command execution.
//!
//! Public routes are explicit. Capability implementations remain owned by
//! their named modules and are added only by the skills that prove them.

pub mod commands;
pub mod execution;
pub mod managed;
pub mod pty;
