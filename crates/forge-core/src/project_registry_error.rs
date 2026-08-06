//! Formatting and source-error conversions for project-registry failures.

use super::ProjectRegistryStateError;
use crate::projects::ProjectManifestError;
use crate::state::StateRecordError;
use std::fmt;

impl fmt::Display for ProjectRegistryStateError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::State(source) => write!(formatter, "state record rejected: {source}"),
            Self::Manifest(source) => write!(formatter, "project manifest rejected: {source}"),
            Self::InvalidMagic => formatter.write_str("invalid project-registry magic"),
            Self::UnsupportedSchemaVersion(found) => {
                write!(formatter, "unsupported project-registry schema {found}")
            }
            Self::WrongRecordType { expected, found } => write!(
                formatter,
                "expected state record type {expected:#06x}, found {found:#06x}"
            ),
            Self::Truncated { needed, remaining } => write!(
                formatter,
                "registry payload needs {needed} bytes, only {remaining} remain"
            ),
            Self::TrailingBytes(count) => {
                write!(formatter, "registry payload has {count} trailing bytes")
            }
            Self::LengthOverflow => formatter.write_str("registry payload length overflow"),
            Self::InvalidUtf8 => formatter.write_str("registry text is not valid UTF-8"),
            Self::TextContainsNul => formatter.write_str("registry text contains NUL"),
            Self::TextTooLong { maximum, actual } => write!(
                formatter,
                "registry text has {actual} bytes, maximum is {maximum}"
            ),
            Self::DisplayRootNotAbsolute => {
                formatter.write_str("display root is not an absolute Linux path")
            }
            Self::DisplayRootContainsNul => formatter.write_str("display root contains NUL"),
            Self::DisplayRootTooLong { maximum, actual } => write!(
                formatter,
                "display root has {actual} bytes, maximum is {maximum}"
            ),
            Self::TooManyProjects { maximum } => {
                write!(formatter, "project registry exceeds {maximum} projects")
            }
            Self::DuplicateProjectId(project_id) => {
                write!(formatter, "duplicate project ID {project_id}")
            }
            Self::DuplicateRepositoryId {
                repository_id,
                existing_project,
            } => write!(
                formatter,
                "repository {repository_id} is already owned by project {existing_project}"
            ),
            Self::UnknownProject(project_id) => write!(formatter, "unknown project {project_id}"),
            Self::GenerationOverflow => formatter.write_str("project-registry generation overflow"),
            Self::OpenSequenceOverflow => formatter.write_str("recent-open sequence overflow"),
            Self::InvalidNextOpenSequence => {
                formatter.write_str("invalid next recent-open sequence")
            }
            Self::InvalidOpenFlag(found) => write!(formatter, "invalid project-open flag {found}"),
            Self::OpenProjectMissingSequence(project_id) => write!(
                formatter,
                "open project {project_id} has no recent-open sequence"
            ),
            Self::DuplicateOpenSequence(sequence) => {
                write!(formatter, "duplicate recent-open sequence {sequence}")
            }
            Self::InvalidSnapshotFlag(found) => write!(formatter, "invalid snapshot flag {found}"),
            Self::ReservedSnapshotSchema => {
                formatter.write_str("workspace snapshot schema zero is reserved")
            }
            Self::SnapshotTooLarge { maximum, actual } => write!(
                formatter,
                "workspace snapshot has {actual} bytes, maximum is {maximum}"
            ),
            Self::SnapshotIdentityMismatch { expected, actual } => write!(
                formatter,
                "workspace snapshot identity mismatch: expected {expected}, found {actual}"
            ),
            Self::TooManyCommands { maximum, actual } => write!(
                formatter,
                "project has {actual} commands, maximum is {maximum}"
            ),
            Self::EmptyCommandDefinition => {
                formatter.write_str("registered command definition is empty")
            }
            Self::CommandDefinitionTooLarge { maximum, actual } => write!(
                formatter,
                "registered command definition has {actual} bytes, maximum is {maximum}"
            ),
            Self::CommandSetMismatch {
                manifest,
                definitions,
            } => write!(
                formatter,
                "manifest declares {manifest} commands, but registry carries {definitions} definitions"
            ),
            Self::DuplicateCommandId(command_id) => {
                write!(formatter, "duplicate command ID {command_id}")
            }
            Self::NonCanonicalCommandOrder => {
                formatter.write_str("registered commands are not in canonical ID order")
            }
            Self::CommandMissingFromManifest(command_id) => write!(
                formatter,
                "command {command_id} is absent from the project manifest"
            ),
            Self::CommandRepositoryMismatch {
                command_id,
                expected,
                found,
            } => write!(
                formatter,
                "command {command_id} targets repository {found}, expected {expected}"
            ),
            Self::CommandNameMismatch {
                command_id,
                manifest,
                definition,
            } => write!(
                formatter,
                "command {command_id} name mismatch: manifest {manifest:?}, definition {definition:?}"
            ),
            Self::CommandIdentityMismatch {
                command_id,
                expected,
                actual,
            } => write!(
                formatter,
                "command {command_id} identity mismatch: expected {expected}, found {actual}"
            ),
            Self::CommandDecode { command_id, source } => {
                write!(formatter, "command {command_id} bytes rejected: {source}")
            }
            Self::CommandMetadataMismatch(command_id) => write!(
                formatter,
                "command {command_id} metadata disagrees with canonical bytes"
            ),
        }
    }
}

impl std::error::Error for ProjectRegistryStateError {}

impl From<StateRecordError> for ProjectRegistryStateError {
    fn from(source: StateRecordError) -> Self {
        Self::State(source)
    }
}

impl From<ProjectManifestError> for ProjectRegistryStateError {
    fn from(source: ProjectManifestError) -> Self {
        Self::Manifest(source)
    }
}
