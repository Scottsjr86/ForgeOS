//! ForgeOS project registration and persistence adapters.
//!
//! Public routes are explicit. Capability implementations remain owned by
//! their named modules and are added only by the skills that prove them.

pub mod files;
pub mod paths;
pub mod persistence;
pub mod recovery_store;
pub mod registry;
pub mod registry_store;
pub mod repository_view;
pub mod text_search;
