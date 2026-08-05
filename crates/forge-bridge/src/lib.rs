//! Explicit ForgeOS adapters and ports for real development tools.
//!
//! Public routes are explicit. Capability implementations remain owned by
//! their named modules and are added only by the skills that prove them.

pub mod adapters;
pub mod git;
pub mod git_mutation;
pub mod lsp;
pub mod parsing;
pub mod patch;
pub mod ports;
pub mod processes;
pub mod pty;
pub mod service_process;
