//! Canonical persistent project-registry state.
//!
//! Forge Core owns the exact registry bytes and mutation rules. Filesystem
//! validation, atomic publication, and repository reopening remain in
//! `forge-project`.

use crate::command_codec::{decode_registered_command, CommandDecodeError};
use crate::commands::RegisteredCommand;
use crate::projects::{ProjectManifest, ProjectManifestError};
use crate::state::{StateRecord, StateRecordError};
use forge_protocol::hashes::{hash_canonical_bytes, ContentHash, HashDomain};
use forge_protocol::identities::{CommandId, ProjectId, RepositoryId, IDENTITY_BYTES};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
const REGISTRY_MAGIC: [u8; 8] = *b"FGPREG\0\0";
const REGISTRY_SCHEMA_VERSION: u16 = 1;
const SNAPSHOT_MAGIC: [u8; 8] = *b"FGWSNAP\0";
const MAX_PROJECTS: usize = 1024;
const MAX_DISPLAY_ROOT_BYTES: usize = 16 * 1024;
const MAX_COMMANDS_PER_PROJECT: usize = 256;
const MAX_COMMAND_BYTES: usize = 256 * 1024;
const MAX_COMMAND_NAME_BYTES: usize = 128;
const MAX_SNAPSHOT_BYTES: usize = 4 * 1024 * 1024;
/// State-record type reserved for the canonical V1 project registry.
pub const PROJECT_REGISTRY_RECORD_TYPE: u16 = 0x0102;
/// Exact Linux filesystem object identity captured for one repository root.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PersistedRepositoryObject {
    device: u64,
    object: u64,
}
impl PersistedRepositoryObject {
    pub const fn new(device: u64, object: u64) -> Self {
        Self { device, object }
    }
    pub const fn device(self) -> u64 {
        self.device
    }
    pub const fn object(self) -> u64 {
        self.object
    }
}
/// Deterministic recent-open state. Sequence values are registry-local and do
/// not derive from wall-clock time.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RecentOpenState {
    is_open: bool,
    last_open_sequence: Option<u64>,
}
impl RecentOpenState {
    pub const fn never_opened() -> Self {
        Self {
            is_open: false,
            last_open_sequence: None,
        }
    }
    pub const fn is_open(self) -> bool {
        self.is_open
    }
    pub const fn last_open_sequence(self) -> Option<u64> {
        self.last_open_sequence
    }
    const fn opened(sequence: u64) -> Self {
        Self {
            is_open: true,
            last_open_sequence: Some(sequence),
        }
    }
    const fn closed(self) -> Self {
        Self {
            is_open: false,
            last_open_sequence: self.last_open_sequence,
        }
    }
}
/// One versioned safe workspace payload retained with a project record.
/// Recovery policy and process replay are deliberately outside this contract.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SafeWorkspaceSnapshot {
    schema_version: u16,
    payload: Vec<u8>,
    identity: ContentHash,
}
impl SafeWorkspaceSnapshot {
    pub fn new(schema_version: u16, payload: Vec<u8>) -> Result<Self, ProjectRegistryStateError> {
        if schema_version == 0 {
            return Err(ProjectRegistryStateError::ReservedSnapshotSchema);
        }
        if payload.len() > MAX_SNAPSHOT_BYTES {
            return Err(ProjectRegistryStateError::SnapshotTooLarge {
                maximum: MAX_SNAPSHOT_BYTES,
                actual: payload.len(),
            });
        }
        let identity = snapshot_identity(schema_version, &payload);
        Ok(Self {
            schema_version,
            payload,
            identity,
        })
    }
    pub const fn schema_version(&self) -> u16 {
        self.schema_version
    }
    pub fn payload(&self) -> &[u8] {
        &self.payload
    }
    pub const fn identity(&self) -> ContentHash {
        self.identity
    }
}
/// Exact registered-command definition retained by the project registry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PersistedCommandDefinition {
    command_id: CommandId,
    repository_id: RepositoryId,
    display_name: String,
    canonical_bytes: Vec<u8>,
    identity: ContentHash,
}
impl PersistedCommandDefinition {
    pub fn from_registered(command: &RegisteredCommand) -> Result<Self, ProjectRegistryStateError> {
        let bytes = command.canonical_bytes();
        validate_command_bytes(&bytes)?;
        Ok(Self {
            command_id: command.command_id(),
            repository_id: command.repository_id(),
            display_name: command.display_name().to_owned(),
            identity: command.definition_identity(),
            canonical_bytes: bytes,
        })
    }
    pub const fn command_id(&self) -> CommandId {
        self.command_id
    }
    pub const fn repository_id(&self) -> RepositoryId {
        self.repository_id
    }
    pub fn display_name(&self) -> &str {
        &self.display_name
    }
    pub fn canonical_bytes(&self) -> &[u8] {
        &self.canonical_bytes
    }
    pub const fn identity(&self) -> ContentHash {
        self.identity
    }
    /// Reconstructs the exact typed command definition from its canonical bytes.
    pub fn decode_registered(&self) -> Result<RegisteredCommand, CommandDecodeError> {
        decode_registered_command(&self.canonical_bytes)
    }
}
/// One canonical persistent project record.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PersistentProjectEntry {
    manifest: ProjectManifest,
    display_root_bytes: Vec<u8>,
    repository_object: PersistedRepositoryObject,
    recent_open: RecentOpenState,
    last_safe_snapshot: Option<SafeWorkspaceSnapshot>,
    commands: Vec<PersistedCommandDefinition>,
}
impl PersistentProjectEntry {
    pub fn new(
        manifest: ProjectManifest,
        display_root_bytes: Vec<u8>,
        repository_object: PersistedRepositoryObject,
        mut commands: Vec<PersistedCommandDefinition>,
    ) -> Result<Self, ProjectRegistryStateError> {
        commands.sort_by(|left, right| left.command_id.as_bytes().cmp(right.command_id.as_bytes()));
        let entry = Self {
            manifest,
            display_root_bytes,
            repository_object,
            recent_open: RecentOpenState::never_opened(),
            last_safe_snapshot: None,
            commands,
        };
        entry.validate()?;
        Ok(entry)
    }
    pub fn manifest(&self) -> &ProjectManifest {
        &self.manifest
    }
    pub fn display_root_bytes(&self) -> &[u8] {
        &self.display_root_bytes
    }
    pub const fn repository_object(&self) -> PersistedRepositoryObject {
        self.repository_object
    }
    pub const fn recent_open(&self) -> RecentOpenState {
        self.recent_open
    }
    pub fn last_safe_snapshot(&self) -> Option<&SafeWorkspaceSnapshot> {
        self.last_safe_snapshot.as_ref()
    }
    pub fn commands(&self) -> &[PersistedCommandDefinition] {
        &self.commands
    }
    fn validate(&self) -> Result<(), ProjectRegistryStateError> {
        validate_display_root(&self.display_root_bytes)?;
        validate_commands(&self.manifest, &self.commands)?;
        if let Some(snapshot) = &self.last_safe_snapshot {
            let actual = snapshot_identity(snapshot.schema_version, &snapshot.payload);
            if actual != snapshot.identity {
                return Err(ProjectRegistryStateError::SnapshotIdentityMismatch {
                    expected: snapshot.identity,
                    actual,
                });
            }
        }
        Ok(())
    }
}
/// Canonical deterministic registry state for every known V1 project.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectRegistryState {
    generation: u64,
    next_open_sequence: u64,
    projects: BTreeMap<ProjectId, PersistentProjectEntry>,
    repositories: BTreeMap<RepositoryId, ProjectId>,
}
impl ProjectRegistryState {
    pub fn empty() -> Self {
        Self {
            generation: 0,
            next_open_sequence: 1,
            projects: BTreeMap::new(),
            repositories: BTreeMap::new(),
        }
    }
    pub const fn generation(&self) -> u64 {
        self.generation
    }
    pub const fn next_open_sequence(&self) -> u64 {
        self.next_open_sequence
    }
    pub fn len(&self) -> usize {
        self.projects.len()
    }
    pub fn is_empty(&self) -> bool {
        self.projects.is_empty()
    }
    pub fn get(&self, project_id: ProjectId) -> Option<&PersistentProjectEntry> {
        self.projects.get(&project_id)
    }
    pub fn iter(&self) -> impl ExactSizeIterator<Item = (&ProjectId, &PersistentProjectEntry)> {
        self.projects.iter()
    }
    pub fn register(
        &mut self,
        entry: PersistentProjectEntry,
    ) -> Result<(), ProjectRegistryStateError> {
        entry.validate()?;
        if self.projects.len() >= MAX_PROJECTS {
            return Err(ProjectRegistryStateError::TooManyProjects {
                maximum: MAX_PROJECTS,
            });
        }
        let project_id = entry.manifest.project_id();
        let repository_id = entry.manifest.repository_id();
        if self.projects.contains_key(&project_id) {
            return Err(ProjectRegistryStateError::DuplicateProjectId(project_id));
        }
        if let Some(existing_project) = self.repositories.get(&repository_id) {
            return Err(ProjectRegistryStateError::DuplicateRepositoryId {
                repository_id,
                existing_project: *existing_project,
            });
        }
        self.projects.insert(project_id, entry);
        self.repositories.insert(repository_id, project_id);
        self.advance_generation()?;
        Ok(())
    }
    pub fn rename(
        &mut self,
        project_id: ProjectId,
        display_name: impl Into<String>,
    ) -> Result<(), ProjectRegistryStateError> {
        let entry = self
            .projects
            .get_mut(&project_id)
            .ok_or(ProjectRegistryStateError::UnknownProject(project_id))?;
        let old = &entry.manifest;
        let renamed = ProjectManifest::new(
            old.project_id(),
            old.repository_id(),
            display_name,
            old.allowed_roots().to_vec(),
            old.commands().to_vec(),
            old.language_profile(),
            old.settings().to_vec(),
        )?;
        entry.manifest = renamed;
        entry.validate()?;
        self.advance_generation()?;
        Ok(())
    }
    pub fn mark_open(&mut self, project_id: ProjectId) -> Result<u64, ProjectRegistryStateError> {
        if !self.projects.contains_key(&project_id) {
            return Err(ProjectRegistryStateError::UnknownProject(project_id));
        }
        let sequence = self.next_open_sequence;
        self.next_open_sequence = self
            .next_open_sequence
            .checked_add(1)
            .ok_or(ProjectRegistryStateError::OpenSequenceOverflow)?;
        self.projects
            .get_mut(&project_id)
            .expect("project existence was checked before sequence allocation")
            .recent_open = RecentOpenState::opened(sequence);
        self.advance_generation()?;
        Ok(sequence)
    }
    pub fn mark_closed(&mut self, project_id: ProjectId) -> Result<(), ProjectRegistryStateError> {
        let entry = self
            .projects
            .get_mut(&project_id)
            .ok_or(ProjectRegistryStateError::UnknownProject(project_id))?;
        entry.recent_open = entry.recent_open.closed();
        self.advance_generation()?;
        Ok(())
    }
    pub fn set_last_safe_snapshot(
        &mut self,
        project_id: ProjectId,
        snapshot: SafeWorkspaceSnapshot,
    ) -> Result<(), ProjectRegistryStateError> {
        let entry = self
            .projects
            .get_mut(&project_id)
            .ok_or(ProjectRegistryStateError::UnknownProject(project_id))?;
        entry.last_safe_snapshot = Some(snapshot);
        entry.validate()?;
        self.advance_generation()?;
        Ok(())
    }
    pub fn clear_last_safe_snapshot(
        &mut self,
        project_id: ProjectId,
    ) -> Result<(), ProjectRegistryStateError> {
        let entry = self
            .projects
            .get_mut(&project_id)
            .ok_or(ProjectRegistryStateError::UnknownProject(project_id))?;
        entry.last_safe_snapshot = None;
        self.advance_generation()?;
        Ok(())
    }
    pub fn relocate(
        &mut self,
        project_id: ProjectId,
        display_root_bytes: Vec<u8>,
        repository_object: PersistedRepositoryObject,
    ) -> Result<(), ProjectRegistryStateError> {
        validate_display_root(&display_root_bytes)?;
        let entry = self
            .projects
            .get_mut(&project_id)
            .ok_or(ProjectRegistryStateError::UnknownProject(project_id))?;
        entry.display_root_bytes = display_root_bytes;
        entry.repository_object = repository_object;
        entry.validate()?;
        self.advance_generation()?;
        Ok(())
    }
    pub fn remove(
        &mut self,
        project_id: ProjectId,
    ) -> Result<PersistentProjectEntry, ProjectRegistryStateError> {
        let entry = self
            .projects
            .remove(&project_id)
            .ok_or(ProjectRegistryStateError::UnknownProject(project_id))?;
        self.repositories.remove(&entry.manifest.repository_id());
        self.advance_generation()?;
        Ok(entry)
    }
    pub fn recent_projects(&self) -> Vec<ProjectId> {
        let mut recent: Vec<_> = self
            .projects
            .iter()
            .filter_map(|(project_id, entry)| {
                entry
                    .recent_open
                    .last_open_sequence
                    .map(|sequence| (*project_id, sequence))
            })
            .collect();
        recent.sort_by(|(left_id, left_sequence), (right_id, right_sequence)| {
            right_sequence
                .cmp(left_sequence)
                .then_with(|| left_id.as_bytes().cmp(right_id.as_bytes()))
        });
        recent
            .into_iter()
            .map(|(project_id, _)| project_id)
            .collect()
    }
    pub fn to_state_record(&self) -> Result<StateRecord, ProjectRegistryStateError> {
        StateRecord::new(PROJECT_REGISTRY_RECORD_TYPE, self.encode())
            .map_err(ProjectRegistryStateError::State)
    }
    pub fn from_state_record(record: &StateRecord) -> Result<Self, ProjectRegistryStateError> {
        if record.record_type() != PROJECT_REGISTRY_RECORD_TYPE {
            return Err(ProjectRegistryStateError::WrongRecordType {
                expected: PROJECT_REGISTRY_RECORD_TYPE,
                found: record.record_type(),
            });
        }
        Self::decode(record.payload())
    }
    /// Exact deterministic V1 registry payload bytes.
    pub fn encode(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&REGISTRY_MAGIC);
        bytes.extend_from_slice(&REGISTRY_SCHEMA_VERSION.to_be_bytes());
        bytes.extend_from_slice(&self.generation.to_be_bytes());
        bytes.extend_from_slice(&self.next_open_sequence.to_be_bytes());
        bytes.extend_from_slice(&(self.projects.len() as u32).to_be_bytes());
        for entry in self.projects.values() {
            put_bytes(&mut bytes, &entry.manifest.encode());
            put_bytes(&mut bytes, &entry.display_root_bytes);
            bytes.extend_from_slice(&entry.repository_object.device.to_be_bytes());
            bytes.extend_from_slice(&entry.repository_object.object.to_be_bytes());
            bytes.push(u8::from(entry.recent_open.is_open));
            bytes.extend_from_slice(
                &entry
                    .recent_open
                    .last_open_sequence
                    .unwrap_or(0)
                    .to_be_bytes(),
            );
            match &entry.last_safe_snapshot {
                None => bytes.push(0),
                Some(snapshot) => {
                    bytes.push(1);
                    bytes.extend_from_slice(&snapshot.schema_version.to_be_bytes());
                    put_bytes(&mut bytes, &snapshot.payload);
                    bytes.extend_from_slice(snapshot.identity.as_bytes());
                }
            }
            bytes.extend_from_slice(&(entry.commands.len() as u16).to_be_bytes());
            for command in &entry.commands {
                bytes.extend_from_slice(command.command_id.as_bytes());
                bytes.extend_from_slice(command.repository_id.as_bytes());
                put_text(&mut bytes, &command.display_name);
                bytes.extend_from_slice(command.identity.as_bytes());
                put_bytes(&mut bytes, &command.canonical_bytes);
            }
        }
        bytes
    }
    pub fn decode(bytes: &[u8]) -> Result<Self, ProjectRegistryStateError> {
        let mut cursor = Cursor::new(bytes);
        if cursor.take(REGISTRY_MAGIC.len())? != REGISTRY_MAGIC.as_slice() {
            return Err(ProjectRegistryStateError::InvalidMagic);
        }
        let schema = cursor.u16()?;
        if schema != REGISTRY_SCHEMA_VERSION {
            return Err(ProjectRegistryStateError::UnsupportedSchemaVersion(schema));
        }
        let generation = cursor.u64()?;
        let next_open_sequence = cursor.u64()?;
        if next_open_sequence == 0 {
            return Err(ProjectRegistryStateError::InvalidNextOpenSequence);
        }
        let count = cursor.u32()? as usize;
        if count > MAX_PROJECTS {
            return Err(ProjectRegistryStateError::TooManyProjects {
                maximum: MAX_PROJECTS,
            });
        }

        let mut state = Self {
            generation,
            next_open_sequence,
            projects: BTreeMap::new(),
            repositories: BTreeMap::new(),
        };
        let mut maximum_open_sequence = 0;
        let mut open_sequences = BTreeSet::new();
        for _ in 0..count {
            let manifest = ProjectManifest::decode(cursor.bytes()?)?;
            let display_root_bytes = cursor.bytes()?.to_vec();
            let repository_object = PersistedRepositoryObject::new(cursor.u64()?, cursor.u64()?);
            let is_open = match cursor.u8()? {
                0 => false,
                1 => true,
                found => return Err(ProjectRegistryStateError::InvalidOpenFlag(found)),
            };
            let raw_sequence = cursor.u64()?;
            let last_open_sequence = if raw_sequence == 0 {
                None
            } else {
                if !open_sequences.insert(raw_sequence) {
                    return Err(ProjectRegistryStateError::DuplicateOpenSequence(
                        raw_sequence,
                    ));
                }
                maximum_open_sequence = maximum_open_sequence.max(raw_sequence);
                Some(raw_sequence)
            };
            if is_open && last_open_sequence.is_none() {
                return Err(ProjectRegistryStateError::OpenProjectMissingSequence(
                    manifest.project_id(),
                ));
            }
            let snapshot = match cursor.u8()? {
                0 => None,
                1 => {
                    let schema_version = cursor.u16()?;
                    let payload = cursor.bytes()?.to_vec();
                    let expected = ContentHash::from_bytes(cursor.array::<32>()?);
                    let snapshot = SafeWorkspaceSnapshot::new(schema_version, payload)?;
                    if snapshot.identity != expected {
                        return Err(ProjectRegistryStateError::SnapshotIdentityMismatch {
                            expected,
                            actual: snapshot.identity,
                        });
                    }
                    Some(snapshot)
                }
                found => return Err(ProjectRegistryStateError::InvalidSnapshotFlag(found)),
            };
            let command_count = cursor.u16()? as usize;
            if command_count > MAX_COMMANDS_PER_PROJECT {
                return Err(ProjectRegistryStateError::TooManyCommands {
                    maximum: MAX_COMMANDS_PER_PROJECT,
                    actual: command_count,
                });
            }
            let mut commands = Vec::with_capacity(command_count);
            for _ in 0..command_count {
                let command_id = CommandId::from_bytes(cursor.array::<IDENTITY_BYTES>()?);
                let repository_id = RepositoryId::from_bytes(cursor.array::<IDENTITY_BYTES>()?);
                let display_name = cursor.text(MAX_COMMAND_NAME_BYTES)?;
                let expected = ContentHash::from_bytes(cursor.array::<32>()?);
                let canonical_bytes = cursor.bytes()?.to_vec();
                validate_command_bytes(&canonical_bytes)?;
                let actual = hash_canonical_bytes(HashDomain::ToolRequest, &canonical_bytes);
                if actual != expected {
                    return Err(ProjectRegistryStateError::CommandIdentityMismatch {
                        command_id,
                        expected,
                        actual,
                    });
                }
                commands.push(PersistedCommandDefinition {
                    command_id,
                    repository_id,
                    display_name,
                    canonical_bytes,
                    identity: expected,
                });
            }
            let entry = PersistentProjectEntry {
                manifest,
                display_root_bytes,
                repository_object,
                recent_open: RecentOpenState {
                    is_open,
                    last_open_sequence,
                },
                last_safe_snapshot: snapshot,
                commands,
            };
            entry.validate()?;
            let project_id = entry.manifest.project_id();
            let repository_id = entry.manifest.repository_id();
            if state.projects.insert(project_id, entry).is_some() {
                return Err(ProjectRegistryStateError::DuplicateProjectId(project_id));
            }
            if let Some(existing_project) = state.repositories.insert(repository_id, project_id) {
                return Err(ProjectRegistryStateError::DuplicateRepositoryId {
                    repository_id,
                    existing_project,
                });
            }
        }
        if !cursor.is_finished() {
            return Err(ProjectRegistryStateError::TrailingBytes(cursor.remaining()));
        }
        if maximum_open_sequence >= next_open_sequence {
            return Err(ProjectRegistryStateError::InvalidNextOpenSequence);
        }
        Ok(state)
    }
    fn advance_generation(&mut self) -> Result<(), ProjectRegistryStateError> {
        self.generation = self
            .generation
            .checked_add(1)
            .ok_or(ProjectRegistryStateError::GenerationOverflow)?;
        Ok(())
    }
}
fn validate_display_root(bytes: &[u8]) -> Result<(), ProjectRegistryStateError> {
    if bytes.is_empty() || bytes[0] != b'/' {
        return Err(ProjectRegistryStateError::DisplayRootNotAbsolute);
    }
    if bytes.len() > MAX_DISPLAY_ROOT_BYTES {
        return Err(ProjectRegistryStateError::DisplayRootTooLong {
            maximum: MAX_DISPLAY_ROOT_BYTES,
            actual: bytes.len(),
        });
    }
    if bytes.contains(&0) {
        return Err(ProjectRegistryStateError::DisplayRootContainsNul);
    }
    Ok(())
}
fn validate_command_bytes(bytes: &[u8]) -> Result<(), ProjectRegistryStateError> {
    if bytes.is_empty() {
        return Err(ProjectRegistryStateError::EmptyCommandDefinition);
    }
    if bytes.len() > MAX_COMMAND_BYTES {
        return Err(ProjectRegistryStateError::CommandDefinitionTooLarge {
            maximum: MAX_COMMAND_BYTES,
            actual: bytes.len(),
        });
    }
    Ok(())
}
fn validate_commands(
    manifest: &ProjectManifest,
    commands: &[PersistedCommandDefinition],
) -> Result<(), ProjectRegistryStateError> {
    if commands.len() > MAX_COMMANDS_PER_PROJECT {
        return Err(ProjectRegistryStateError::TooManyCommands {
            maximum: MAX_COMMANDS_PER_PROJECT,
            actual: commands.len(),
        });
    }
    if commands.len() != manifest.commands().len() {
        return Err(ProjectRegistryStateError::CommandSetMismatch {
            manifest: manifest.commands().len(),
            definitions: commands.len(),
        });
    }
    for pair in commands.windows(2) {
        if pair[0].command_id.as_bytes() > pair[1].command_id.as_bytes() {
            return Err(ProjectRegistryStateError::NonCanonicalCommandOrder);
        }
    }
    let mut ids = BTreeSet::new();
    for command in commands {
        validate_command_bytes(&command.canonical_bytes)?;
        let decoded = command.decode_registered().map_err(|source| {
            ProjectRegistryStateError::CommandDecode {
                command_id: command.command_id,
                source,
            }
        })?;
        if decoded.command_id() != command.command_id
            || decoded.repository_id() != command.repository_id
            || decoded.display_name() != command.display_name
        {
            return Err(ProjectRegistryStateError::CommandMetadataMismatch(
                command.command_id,
            ));
        }
        if command.repository_id != manifest.repository_id() {
            return Err(ProjectRegistryStateError::CommandRepositoryMismatch {
                command_id: command.command_id,
                expected: manifest.repository_id(),
                found: command.repository_id,
            });
        }
        let actual = hash_canonical_bytes(HashDomain::ToolRequest, &command.canonical_bytes);
        if actual != command.identity {
            return Err(ProjectRegistryStateError::CommandIdentityMismatch {
                command_id: command.command_id,
                expected: command.identity,
                actual,
            });
        }
        if !ids.insert(command.command_id) {
            return Err(ProjectRegistryStateError::DuplicateCommandId(
                command.command_id,
            ));
        }
        let reference = manifest
            .commands()
            .iter()
            .find(|reference| reference.command_id() == command.command_id)
            .ok_or(ProjectRegistryStateError::CommandMissingFromManifest(
                command.command_id,
            ))?;
        if reference.display_name() != command.display_name {
            return Err(ProjectRegistryStateError::CommandNameMismatch {
                command_id: command.command_id,
                manifest: reference.display_name().to_owned(),
                definition: command.display_name.clone(),
            });
        }
    }
    Ok(())
}
fn snapshot_identity(schema_version: u16, payload: &[u8]) -> ContentHash {
    let mut bytes = Vec::with_capacity(SNAPSHOT_MAGIC.len() + 2 + 4 + payload.len());
    bytes.extend_from_slice(&SNAPSHOT_MAGIC);
    bytes.extend_from_slice(&schema_version.to_be_bytes());
    bytes.extend_from_slice(&(payload.len() as u32).to_be_bytes());
    bytes.extend_from_slice(payload);
    hash_canonical_bytes(HashDomain::Snapshot, &bytes)
}
fn put_text(bytes: &mut Vec<u8>, text: &str) {
    bytes.extend_from_slice(&(text.len() as u16).to_be_bytes());
    bytes.extend_from_slice(text.as_bytes());
}
fn put_bytes(bytes: &mut Vec<u8>, value: &[u8]) {
    bytes.extend_from_slice(&(value.len() as u32).to_be_bytes());
    bytes.extend_from_slice(value);
}

struct Cursor<'a> {
    bytes: &'a [u8],
    offset: usize,
}

impl<'a> Cursor<'a> {
    const fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, offset: 0 }
    }
    fn take(&mut self, count: usize) -> Result<&'a [u8], ProjectRegistryStateError> {
        let end = self
            .offset
            .checked_add(count)
            .ok_or(ProjectRegistryStateError::LengthOverflow)?;
        if end > self.bytes.len() {
            return Err(ProjectRegistryStateError::Truncated {
                needed: count,
                remaining: self.remaining(),
            });
        }
        let value = &self.bytes[self.offset..end];
        self.offset = end;
        Ok(value)
    }
    fn array<const N: usize>(&mut self) -> Result<[u8; N], ProjectRegistryStateError> {
        self.take(N)?
            .try_into()
            .map_err(|_| ProjectRegistryStateError::LengthOverflow)
    }
    fn u8(&mut self) -> Result<u8, ProjectRegistryStateError> {
        Ok(self.take(1)?[0])
    }
    fn u16(&mut self) -> Result<u16, ProjectRegistryStateError> {
        Ok(u16::from_be_bytes(self.array()?))
    }
    fn u32(&mut self) -> Result<u32, ProjectRegistryStateError> {
        Ok(u32::from_be_bytes(self.array()?))
    }
    fn u64(&mut self) -> Result<u64, ProjectRegistryStateError> {
        Ok(u64::from_be_bytes(self.array()?))
    }
    fn bytes(&mut self) -> Result<&'a [u8], ProjectRegistryStateError> {
        let length = self.u32()? as usize;
        self.take(length)
    }
    fn text(&mut self, maximum: usize) -> Result<String, ProjectRegistryStateError> {
        let length = self.u16()? as usize;
        if length > maximum {
            return Err(ProjectRegistryStateError::TextTooLong {
                maximum,
                actual: length,
            });
        }
        let bytes = self.take(length)?;
        let text =
            std::str::from_utf8(bytes).map_err(|_| ProjectRegistryStateError::InvalidUtf8)?;
        if text.as_bytes().contains(&0) {
            return Err(ProjectRegistryStateError::TextContainsNul);
        }
        Ok(text.to_owned())
    }
    const fn remaining(&self) -> usize {
        self.bytes.len() - self.offset
    }
    const fn is_finished(&self) -> bool {
        self.offset == self.bytes.len()
    }
}
/// Exact reason canonical persistent project state was rejected.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProjectRegistryStateError {
    State(StateRecordError),
    Manifest(ProjectManifestError),
    InvalidMagic,
    UnsupportedSchemaVersion(u16),
    WrongRecordType {
        expected: u16,
        found: u16,
    },
    Truncated {
        needed: usize,
        remaining: usize,
    },
    TrailingBytes(usize),
    LengthOverflow,
    InvalidUtf8,
    TextContainsNul,
    TextTooLong {
        maximum: usize,
        actual: usize,
    },
    DisplayRootNotAbsolute,
    DisplayRootContainsNul,
    DisplayRootTooLong {
        maximum: usize,
        actual: usize,
    },
    TooManyProjects {
        maximum: usize,
    },
    DuplicateProjectId(ProjectId),
    DuplicateRepositoryId {
        repository_id: RepositoryId,
        existing_project: ProjectId,
    },
    UnknownProject(ProjectId),
    GenerationOverflow,
    OpenSequenceOverflow,
    InvalidNextOpenSequence,
    InvalidOpenFlag(u8),
    OpenProjectMissingSequence(ProjectId),
    DuplicateOpenSequence(u64),
    InvalidSnapshotFlag(u8),
    ReservedSnapshotSchema,
    SnapshotTooLarge {
        maximum: usize,
        actual: usize,
    },
    SnapshotIdentityMismatch {
        expected: ContentHash,
        actual: ContentHash,
    },
    TooManyCommands {
        maximum: usize,
        actual: usize,
    },
    EmptyCommandDefinition,
    CommandDefinitionTooLarge {
        maximum: usize,
        actual: usize,
    },
    CommandSetMismatch {
        manifest: usize,
        definitions: usize,
    },
    DuplicateCommandId(CommandId),
    NonCanonicalCommandOrder,
    CommandMissingFromManifest(CommandId),
    CommandRepositoryMismatch {
        command_id: CommandId,
        expected: RepositoryId,
        found: RepositoryId,
    },
    CommandNameMismatch {
        command_id: CommandId,
        manifest: String,
        definition: String,
    },
    CommandIdentityMismatch {
        command_id: CommandId,
        expected: ContentHash,
        actual: ContentHash,
    },
    CommandDecode {
        command_id: CommandId,
        source: CommandDecodeError,
    },
    CommandMetadataMismatch(CommandId),
}
impl fmt::Display for ProjectRegistryStateError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::State(source) => write!(formatter, "state record rejected: {source}"),
            Self::Manifest(source) => write!(formatter, "project manifest rejected: {source}"),
            Self::InvalidMagic => formatter.write_str("invalid project-registry magic"),
            Self::UnsupportedSchemaVersion(found) => {
                write!(formatter, "unsupported project-registry schema {found}")
            }
            Self::WrongRecordType { expected, found } => {
                write!(formatter, "expected state record type {expected:#06x}, found {found:#06x}")
            }
            Self::Truncated { needed, remaining } => {
                write!(formatter, "registry payload needs {needed} bytes, only {remaining} remain")
            }
            Self::TrailingBytes(count) => write!(formatter, "registry payload has {count} trailing bytes"),
            Self::LengthOverflow => formatter.write_str("registry payload length overflow"),
            Self::InvalidUtf8 => formatter.write_str("registry text is not valid UTF-8"),
            Self::TextContainsNul => formatter.write_str("registry text contains NUL"),
            Self::TextTooLong { maximum, actual } => {
                write!(formatter, "registry text has {actual} bytes, maximum is {maximum}")
            }
            Self::DisplayRootNotAbsolute => formatter.write_str("display root is not an absolute Linux path"),
            Self::DisplayRootContainsNul => formatter.write_str("display root contains NUL"),
            Self::DisplayRootTooLong { maximum, actual } => {
                write!(formatter, "display root has {actual} bytes, maximum is {maximum}")
            }
            Self::TooManyProjects { maximum } => write!(formatter, "project registry exceeds {maximum} projects"),
            Self::DuplicateProjectId(project_id) => write!(formatter, "duplicate project ID {project_id}"),
            Self::DuplicateRepositoryId { repository_id, existing_project } => write!(
                formatter,
                "repository {repository_id} is already owned by project {existing_project}"
            ),
            Self::UnknownProject(project_id) => write!(formatter, "unknown project {project_id}"),
            Self::GenerationOverflow => formatter.write_str("project-registry generation overflow"),
            Self::OpenSequenceOverflow => formatter.write_str("recent-open sequence overflow"),
            Self::InvalidNextOpenSequence => formatter.write_str("invalid next recent-open sequence"),
            Self::InvalidOpenFlag(found) => write!(formatter, "invalid project-open flag {found}"),
            Self::OpenProjectMissingSequence(project_id) => {
                write!(formatter, "open project {project_id} has no recent-open sequence")
            }
            Self::DuplicateOpenSequence(sequence) => {
                write!(formatter, "duplicate recent-open sequence {sequence}")
            }
            Self::InvalidSnapshotFlag(found) => write!(formatter, "invalid snapshot flag {found}"),
            Self::ReservedSnapshotSchema => formatter.write_str("workspace snapshot schema zero is reserved"),
            Self::SnapshotTooLarge { maximum, actual } => {
                write!(formatter, "workspace snapshot has {actual} bytes, maximum is {maximum}")
            }
            Self::SnapshotIdentityMismatch { expected, actual } => write!(
                formatter,
                "workspace snapshot identity mismatch: expected {expected}, found {actual}"
            ),
            Self::TooManyCommands { maximum, actual } => {
                write!(formatter, "project has {actual} commands, maximum is {maximum}")
            }
            Self::EmptyCommandDefinition => formatter.write_str("registered command definition is empty"),
            Self::CommandDefinitionTooLarge { maximum, actual } => write!(
                formatter,
                "registered command definition has {actual} bytes, maximum is {maximum}"
            ),
            Self::CommandSetMismatch { manifest, definitions } => write!(
                formatter,
                "manifest declares {manifest} commands, but registry carries {definitions} definitions"
            ),
            Self::DuplicateCommandId(command_id) => write!(formatter, "duplicate command ID {command_id}"),
            Self::NonCanonicalCommandOrder => formatter.write_str("registered commands are not in canonical ID order"),
            Self::CommandMissingFromManifest(command_id) => {
                write!(formatter, "command {command_id} is absent from the project manifest")
            }
            Self::CommandRepositoryMismatch { command_id, expected, found } => write!(
                formatter,
                "command {command_id} targets repository {found}, expected {expected}"
            ),
            Self::CommandNameMismatch { command_id, manifest, definition } => write!(
                formatter,
                "command {command_id} name mismatch: manifest {manifest:?}, definition {definition:?}"
            ),
            Self::CommandIdentityMismatch { command_id, expected, actual } => write!(
                formatter,
                "command {command_id} identity mismatch: expected {expected}, found {actual}"
            ),
            Self::CommandDecode { command_id, source } => {
                write!(formatter, "command {command_id} bytes rejected: {source}")
            }
            Self::CommandMetadataMismatch(command_id) => {
                write!(formatter, "command {command_id} metadata disagrees with canonical bytes")
            }
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
