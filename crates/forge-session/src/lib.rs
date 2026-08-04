//! ForgeOS login-session and managed-service lifecycle.
//!
//! Public routes are explicit. Capability implementations remain owned by
//! their named modules and are added only by the skills that prove them.

pub mod bootstrap;
pub mod lifecycle;
pub mod recovery;
pub mod services;
