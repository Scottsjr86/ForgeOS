//! Top-level ForgeOS composition route.
//!
//! Subsystem wiring will be added only by the capabilities that own it.

pub mod command_workspace;
pub mod editor_workspace;
pub mod git_mutation_workspace;
pub mod git_workspace;
pub mod terminal_workspace;

/// Enters the ForgeOS composition root.
///
/// The architecture slice deliberately performs no product behavior.
pub fn run() {}
