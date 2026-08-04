//! Typed user intents emitted by presentation surfaces.
//!
//! Intents carry stable identities and the source generation observed by the
//! caller. They request work from the owning subsystem; they do not execute or
//! mutate canonical state inside a UI or presentation crate.

use crate::identities::{CommandId, ProjectId, RepositoryId};

/// One typed request emitted by a ForgeOS presentation surface.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ForgeUserIntent {
    /// Requests a fresh source-backed project projection.
    RefreshProjectProjection { observed_generation: u64 },
    /// Requests that the owning project subsystem mark one project open.
    OpenProject {
        project_id: ProjectId,
        observed_generation: u64,
    },
    /// Requests that the owning project subsystem mark one project closed.
    CloseProject {
        project_id: ProjectId,
        observed_generation: u64,
    },
    /// Requests execution of one exact registered command through its owner.
    InvokeRegisteredCommand {
        project_id: ProjectId,
        repository_id: RepositoryId,
        command_id: CommandId,
        observed_generation: u64,
    },
}

impl ForgeUserIntent {
    /// Canonical project-registry generation visible when the intent was made.
    pub const fn observed_generation(self) -> u64 {
        match self {
            Self::RefreshProjectProjection {
                observed_generation,
            }
            | Self::OpenProject {
                observed_generation,
                ..
            }
            | Self::CloseProject {
                observed_generation,
                ..
            }
            | Self::InvokeRegisteredCommand {
                observed_generation,
                ..
            } => observed_generation,
        }
    }

    /// Project identity targeted by the request, when one is required.
    pub const fn project_id(self) -> Option<ProjectId> {
        match self {
            Self::RefreshProjectProjection { .. } => None,
            Self::OpenProject { project_id, .. }
            | Self::CloseProject { project_id, .. }
            | Self::InvokeRegisteredCommand { project_id, .. } => Some(project_id),
        }
    }
}
