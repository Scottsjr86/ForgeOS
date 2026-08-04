//! Immutable source-backed view projections for Forge World.
//!
//! Projection copies only presentation data from canonical Forge Core state.
//! It never mutates the registry and never derives identity from names, paths,
//! ordering, viewport size, or renderer state.

use forge_core::project_registry::ProjectRegistryState;
use forge_protocol::hashes::ContentHash;
use forge_protocol::identities::{CommandId, ProjectId, RepositoryId};
use std::collections::BTreeMap;
use std::fmt;

/// Exact display path bytes retained for a project root.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DisplayPath {
    bytes: Vec<u8>,
}

impl DisplayPath {
    fn from_exact_bytes(bytes: &[u8]) -> Self {
        Self {
            bytes: bytes.to_vec(),
        }
    }

    /// Exact source-owned path bytes without lossy text conversion.
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }

    /// Stable escaped text suitable for a basic status surface.
    pub fn escaped_text(&self) -> String {
        let mut output = String::new();
        for byte in &self.bytes {
            match *byte {
                b'\\' => output.push_str("\\\\"),
                0x20..=0x7e => output.push(char::from(*byte)),
                _ => {
                    use std::fmt::Write;
                    write!(&mut output, "\\x{:02x}", *byte)
                        .expect("writing escaped path text cannot fail");
                }
            }
        }
        output
    }
}

/// One exact registered-command row projected for presentation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandView {
    command_id: CommandId,
    display_name: String,
    definition_identity: ContentHash,
}

impl CommandView {
    pub const fn command_id(&self) -> CommandId {
        self.command_id
    }

    pub fn display_name(&self) -> &str {
        &self.display_name
    }

    pub const fn definition_identity(&self) -> ContentHash {
        self.definition_identity
    }
}

/// One immutable project row derived from canonical registry state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectView {
    project_id: ProjectId,
    repository_id: RepositoryId,
    display_name: String,
    display_root: DisplayPath,
    is_open: bool,
    last_open_sequence: Option<u64>,
    recent_rank: Option<usize>,
    safe_snapshot_identity: Option<ContentHash>,
    commands: Vec<CommandView>,
}

impl ProjectView {
    pub const fn project_id(&self) -> ProjectId {
        self.project_id
    }

    pub const fn repository_id(&self) -> RepositoryId {
        self.repository_id
    }

    pub fn display_name(&self) -> &str {
        &self.display_name
    }

    pub fn display_root(&self) -> &DisplayPath {
        &self.display_root
    }

    pub const fn is_open(&self) -> bool {
        self.is_open
    }

    pub const fn last_open_sequence(&self) -> Option<u64> {
        self.last_open_sequence
    }

    pub const fn recent_rank(&self) -> Option<usize> {
        self.recent_rank
    }

    pub const fn safe_snapshot_identity(&self) -> Option<ContentHash> {
        self.safe_snapshot_identity
    }

    pub fn commands(&self) -> &[CommandView] {
        &self.commands
    }

    pub fn command(&self, command_id: CommandId) -> Option<&CommandView> {
        self.commands
            .binary_search_by(|command| command.command_id.as_bytes().cmp(command_id.as_bytes()))
            .ok()
            .map(|index| &self.commands[index])
    }
}

/// Deterministic Forge World projection of the canonical project registry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectRegistryProjection {
    source_generation: u64,
    projects: Vec<ProjectView>,
    recent_project_ids: Vec<ProjectId>,
}

impl ProjectRegistryProjection {
    /// Builds a read-only projection without changing the source registry.
    pub fn from_registry(registry: &ProjectRegistryState) -> Self {
        let recent_project_ids = registry.recent_projects();
        let recent_ranks: BTreeMap<_, _> = recent_project_ids
            .iter()
            .enumerate()
            .map(|(rank, project_id)| (*project_id, rank))
            .collect();

        let projects = registry
            .iter()
            .map(|(project_id, entry)| {
                let manifest = entry.manifest();
                let recent_open = entry.recent_open();
                let commands = entry
                    .commands()
                    .iter()
                    .map(|command| CommandView {
                        command_id: command.command_id(),
                        display_name: command.display_name().to_owned(),
                        definition_identity: command.identity(),
                    })
                    .collect();
                ProjectView {
                    project_id: *project_id,
                    repository_id: manifest.repository_id(),
                    display_name: manifest.display_name().to_owned(),
                    display_root: DisplayPath::from_exact_bytes(entry.display_root_bytes()),
                    is_open: recent_open.is_open(),
                    last_open_sequence: recent_open.last_open_sequence(),
                    recent_rank: recent_ranks.get(project_id).copied(),
                    safe_snapshot_identity: entry
                        .last_safe_snapshot()
                        .map(|snapshot| snapshot.identity()),
                    commands,
                }
            })
            .collect();

        Self {
            source_generation: registry.generation(),
            projects,
            recent_project_ids,
        }
    }

    pub const fn source_generation(&self) -> u64 {
        self.source_generation
    }

    pub fn projects(&self) -> &[ProjectView] {
        &self.projects
    }

    pub fn recent_project_ids(&self) -> &[ProjectId] {
        &self.recent_project_ids
    }

    pub fn project(&self, project_id: ProjectId) -> Option<&ProjectView> {
        self.projects
            .binary_search_by(|project| project.project_id.as_bytes().cmp(project_id.as_bytes()))
            .ok()
            .map(|index| &self.projects[index])
    }
}

/// Renderer-owned viewport metadata. It has no canonical-state authority.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Viewport {
    width: u32,
    height: u32,
    scale_milli: u16,
}

impl Viewport {
    pub fn new(width: u32, height: u32, scale_milli: u16) -> Result<Self, ViewportError> {
        if width == 0 || height == 0 {
            return Err(ViewportError::ZeroExtent { width, height });
        }
        if scale_milli == 0 {
            return Err(ViewportError::ZeroScale);
        }
        Ok(Self {
            width,
            height,
            scale_milli,
        })
    }

    pub const fn width(self) -> u32 {
        self.width
    }

    pub const fn height(self) -> u32 {
        self.height
    }

    pub const fn scale_milli(self) -> u16 {
        self.scale_milli
    }
}

/// One presentation frame that borrows an immutable source projection.
#[derive(Debug, Clone, Copy)]
pub struct PresentationFrame<'a> {
    projection: &'a ProjectRegistryProjection,
    viewport: Viewport,
}

impl<'a> PresentationFrame<'a> {
    pub const fn new(projection: &'a ProjectRegistryProjection, viewport: Viewport) -> Self {
        Self {
            projection,
            viewport,
        }
    }

    pub const fn projection(&self) -> &'a ProjectRegistryProjection {
        self.projection
    }

    pub const fn viewport(&self) -> Viewport {
        self.viewport
    }
}

/// Invalid viewport metadata rejected before presentation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ViewportError {
    ZeroExtent { width: u32, height: u32 },
    ZeroScale,
}

impl fmt::Display for ViewportError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ZeroExtent { width, height } => {
                write!(
                    formatter,
                    "viewport extent must be nonzero; found {width}x{height}"
                )
            }
            Self::ZeroScale => formatter.write_str("viewport scale must be nonzero"),
        }
    }
}

impl std::error::Error for ViewportError {}
