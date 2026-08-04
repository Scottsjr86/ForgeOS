//! Typed Forge World input routing.
//!
//! The router validates actions against one immutable projection and emits a
//! typed protocol intent. It has no mutable Forge Core reference and cannot
//! directly alter project, command, Git, Nyx, or session state.

use crate::presentation::ProjectRegistryProjection;
use forge_protocol::identities::{CommandId, ProjectId};
use forge_protocol::intents::ForgeUserIntent;
use std::fmt;

/// One user action originating from a Forge World control surface.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorldInputAction {
    RefreshProjects,
    OpenProject(ProjectId),
    CloseProject(ProjectId),
    InvokeRegisteredCommand {
        project_id: ProjectId,
        command_id: CommandId,
    },
}

/// Read-only router bound to one exact projected source generation.
#[derive(Debug, Clone, Copy)]
pub struct WorldActionRouter<'a> {
    projection: &'a ProjectRegistryProjection,
}

impl<'a> WorldActionRouter<'a> {
    pub const fn new(projection: &'a ProjectRegistryProjection) -> Self {
        Self { projection }
    }

    pub fn route(&self, action: WorldInputAction) -> Result<ForgeUserIntent, WorldActionError> {
        let observed_generation = self.projection.source_generation();
        match action {
            WorldInputAction::RefreshProjects => Ok(ForgeUserIntent::RefreshProjectProjection {
                observed_generation,
            }),
            WorldInputAction::OpenProject(project_id) => {
                self.require_project(project_id)?;
                Ok(ForgeUserIntent::OpenProject {
                    project_id,
                    observed_generation,
                })
            }
            WorldInputAction::CloseProject(project_id) => {
                self.require_project(project_id)?;
                Ok(ForgeUserIntent::CloseProject {
                    project_id,
                    observed_generation,
                })
            }
            WorldInputAction::InvokeRegisteredCommand {
                project_id,
                command_id,
            } => {
                let project = self.require_project(project_id)?;
                if project.command(command_id).is_none() {
                    return Err(WorldActionError::UnknownProjectCommand {
                        project_id,
                        command_id,
                    });
                }
                Ok(ForgeUserIntent::InvokeRegisteredCommand {
                    project_id,
                    repository_id: project.repository_id(),
                    command_id,
                    observed_generation,
                })
            }
        }
    }

    fn require_project(
        &self,
        project_id: ProjectId,
    ) -> Result<&crate::presentation::ProjectView, WorldActionError> {
        self.projection
            .project(project_id)
            .ok_or(WorldActionError::UnknownProject(project_id))
    }
}

/// Exact reason a presentation action was not emitted as an intent.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorldActionError {
    UnknownProject(ProjectId),
    UnknownProjectCommand {
        project_id: ProjectId,
        command_id: CommandId,
    },
}

impl fmt::Display for WorldActionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownProject(project_id) => {
                write!(
                    formatter,
                    "project {project_id} is absent from the current projection"
                )
            }
            Self::UnknownProjectCommand {
                project_id,
                command_id,
            } => write!(
                formatter,
                "command {command_id} is absent from projected project {project_id}"
            ),
        }
    }
}

impl std::error::Error for WorldActionError {}
