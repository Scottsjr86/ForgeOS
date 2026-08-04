//! Top-level ForgeOS composition route.
//!
//! Subsystem wiring will be added only by the capabilities that own it.

pub mod editor_workspace;
pub mod terminal_workspace;

/// Enters the ForgeOS composition root.
///
/// The architecture slice deliberately performs no product behavior.
pub fn run() {}
