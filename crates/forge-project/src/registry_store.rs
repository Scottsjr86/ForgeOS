//! Atomic persistent project registry and workspace restoration.
//!
//! The canonical registry bytes remain owned by Forge Core. This module binds
//! them to real repository directory objects and publishes each mutation through
//! the existing atomic state store. Repository source is never modified.

use crate::paths::FileSystemObjectId;
use crate::persistence::{AtomicStateStore, StateStoreError};
use crate::registry::{ProjectRegistry, ProjectRegistryError, RegisteredProject};
use forge_core::commands::RegisteredCommand;
use forge_core::project_registry::{
    PersistedCommandDefinition, PersistedRepositoryObject, PersistentProjectEntry,
    ProjectRegistryState, ProjectRegistryStateError, RecentOpenState, SafeWorkspaceSnapshot,
};
use forge_core::projects::ProjectManifest;
use forge_protocol::identities::ProjectId;
use std::ffi::OsString;
use std::fmt;
use std::path::{Path, PathBuf};

#[cfg(unix)]
use std::os::unix::ffi::{OsStrExt, OsStringExt};

/// One atomically stored registry plus its validated live repository bindings.
#[derive(Debug)]
pub struct PersistentProjectRegistry {
    store: AtomicStateStore,
    state: ProjectRegistryState,
    runtime: ProjectRegistry,
    interrupted_write_present: bool,
}

impl PersistentProjectRegistry {
    /// Creates an empty durable registry. Existing state is never overwritten.
    pub fn create(target: impl AsRef<Path>) -> Result<Self, PersistentProjectRegistryError> {
        ensure_supported_platform()?;
        let store = AtomicStateStore::new(target)?;
        let state = ProjectRegistryState::empty();
        store.create(&state.to_state_record()?)?;
        Ok(Self {
            store,
            state,
            runtime: ProjectRegistry::new(),
            interrupted_write_present: false,
        })
    }

    /// Reopens exact canonical state and revalidates every registered repository.
    /// Missing, replaced, copied, or otherwise invalid roots fail the whole reopen.
    pub fn open(target: impl AsRef<Path>) -> Result<Self, PersistentProjectRegistryError> {
        ensure_supported_platform()?;
        let store = AtomicStateStore::new(target)?;
        let opened = store.open_current()?;
        let state = ProjectRegistryState::from_state_record(opened.record())?;
        let runtime = build_runtime(&state)?;
        Ok(Self {
            store,
            state,
            runtime,
            interrupted_write_present: opened.interrupted_write_present(),
        })
    }

    pub fn state(&self) -> &ProjectRegistryState {
        &self.state
    }

    pub fn len(&self) -> usize {
        self.state.len()
    }

    pub fn is_empty(&self) -> bool {
        self.state.is_empty()
    }

    pub const fn interrupted_write_present(&self) -> bool {
        self.interrupted_write_present
    }

    pub fn project(&self, project_id: ProjectId) -> Option<RestoredProject<'_>> {
        Some(RestoredProject {
            entry: self.state.get(project_id)?,
            registered: self.runtime.get(project_id)?,
        })
    }

    pub fn recent_projects(&self) -> Vec<ProjectId> {
        self.state.recent_projects()
    }

    /// Registers one validated project and all of its exact command definitions.
    pub fn register(
        &mut self,
        manifest: ProjectManifest,
        commands: &[RegisteredCommand],
        display_root: impl AsRef<Path>,
    ) -> Result<(), PersistentProjectRegistryError> {
        let display_root = display_root.as_ref();
        let mut probe = ProjectRegistry::new();
        let registered = probe.import_manifest(manifest.clone(), display_root)?;
        let root_object = persisted_object(registered.boundary().root_object());
        let display_root_bytes = path_bytes(display_root)?;
        let definitions = commands
            .iter()
            .map(PersistedCommandDefinition::from_registered)
            .collect::<Result<Vec<_>, _>>()?;
        let entry =
            PersistentProjectEntry::new(manifest, display_root_bytes, root_object, definitions)?;
        let mut candidate = self.state.clone();
        candidate.register(entry)?;
        self.commit_candidate(candidate)
    }

    pub fn rename(
        &mut self,
        project_id: ProjectId,
        display_name: impl Into<String>,
    ) -> Result<(), PersistentProjectRegistryError> {
        let mut candidate = self.state.clone();
        candidate.rename(project_id, display_name)?;
        self.commit_candidate(candidate)
    }

    /// Marks a project open and returns its deterministic registry-local sequence.
    pub fn mark_open(
        &mut self,
        project_id: ProjectId,
    ) -> Result<u64, PersistentProjectRegistryError> {
        let mut candidate = self.state.clone();
        let sequence = candidate.mark_open(project_id)?;
        self.commit_candidate(candidate)?;
        Ok(sequence)
    }

    pub fn mark_closed(
        &mut self,
        project_id: ProjectId,
    ) -> Result<(), PersistentProjectRegistryError> {
        let mut candidate = self.state.clone();
        candidate.mark_closed(project_id)?;
        self.commit_candidate(candidate)
    }

    pub fn set_last_safe_snapshot(
        &mut self,
        project_id: ProjectId,
        snapshot: SafeWorkspaceSnapshot,
    ) -> Result<(), PersistentProjectRegistryError> {
        let mut candidate = self.state.clone();
        candidate.set_last_safe_snapshot(project_id, snapshot)?;
        self.commit_candidate(candidate)
    }

    pub fn clear_last_safe_snapshot(
        &mut self,
        project_id: ProjectId,
    ) -> Result<(), PersistentProjectRegistryError> {
        let mut candidate = self.state.clone();
        candidate.clear_last_safe_snapshot(project_id)?;
        self.commit_candidate(candidate)
    }

    /// Rebinds a moved repository only when the same filesystem object is found.
    pub fn relocate(
        &mut self,
        project_id: ProjectId,
        new_display_root: impl AsRef<Path>,
    ) -> Result<(), PersistentProjectRegistryError> {
        let current = self
            .runtime
            .get(project_id)
            .ok_or(PersistentProjectRegistryError::UnknownProject(project_id))?;
        let relocated = current.boundary().relocate(new_display_root.as_ref())?;
        let mut candidate = self.state.clone();
        candidate.relocate(
            project_id,
            path_bytes(new_display_root.as_ref())?,
            persisted_object(relocated.root_object()),
        )?;
        self.commit_candidate(candidate)
    }

    /// Removes only the registry record. Repository source is never deleted.
    pub fn remove(
        &mut self,
        project_id: ProjectId,
    ) -> Result<PersistentProjectEntry, PersistentProjectRegistryError> {
        let mut candidate = self.state.clone();
        let removed = candidate.remove(project_id)?;
        self.commit_candidate(candidate)?;
        Ok(removed)
    }

    /// Clears a visible abandoned staging record only after current state reopens.
    pub fn discard_interrupted_write(&mut self) -> Result<bool, PersistentProjectRegistryError> {
        let removed = self.store.discard_interrupted_write()?;
        if removed {
            self.interrupted_write_present = false;
        }
        Ok(removed)
    }

    fn commit_candidate(
        &mut self,
        candidate: ProjectRegistryState,
    ) -> Result<(), PersistentProjectRegistryError> {
        let runtime = build_runtime(&candidate)?;
        self.store.replace(&candidate.to_state_record()?)?;
        self.state = candidate;
        self.runtime = runtime;
        self.interrupted_write_present = false;
        Ok(())
    }
}

/// Read-only combined canonical and validated runtime project view.
#[derive(Debug, Clone, Copy)]
pub struct RestoredProject<'a> {
    entry: &'a PersistentProjectEntry,
    registered: &'a RegisteredProject,
}

impl<'a> RestoredProject<'a> {
    pub const fn entry(self) -> &'a PersistentProjectEntry {
        self.entry
    }

    pub const fn registered(self) -> &'a RegisteredProject {
        self.registered
    }

    pub fn manifest(self) -> &'a ProjectManifest {
        self.entry.manifest()
    }

    pub const fn recent_open(self) -> RecentOpenState {
        self.entry.recent_open()
    }

    pub fn last_safe_snapshot(self) -> Option<&'a SafeWorkspaceSnapshot> {
        self.entry.last_safe_snapshot()
    }
}

fn build_runtime(
    state: &ProjectRegistryState,
) -> Result<ProjectRegistry, PersistentProjectRegistryError> {
    let mut runtime = ProjectRegistry::new();
    for (_, entry) in state.iter() {
        let path = path_from_bytes(entry.display_root_bytes())?;
        let registered = runtime.import_manifest(entry.manifest().clone(), &path)?;
        let found = persisted_object(registered.boundary().root_object());
        let expected = entry.repository_object();
        if found != expected {
            return Err(PersistentProjectRegistryError::RepositoryObjectMismatch {
                path,
                expected,
                found,
            });
        }
    }
    Ok(runtime)
}

const fn persisted_object(object: FileSystemObjectId) -> PersistedRepositoryObject {
    PersistedRepositoryObject::new(object.device(), object.object())
}

#[cfg(unix)]
fn path_bytes(path: &Path) -> Result<Vec<u8>, PersistentProjectRegistryError> {
    if !path.is_absolute() {
        return Err(PersistentProjectRegistryError::DisplayRootNotAbsolute(
            path.to_path_buf(),
        ));
    }
    let bytes = path.as_os_str().as_bytes().to_vec();
    if bytes.contains(&0) {
        return Err(PersistentProjectRegistryError::DisplayRootContainsNul);
    }
    Ok(bytes)
}

#[cfg(not(unix))]
fn path_bytes(_path: &Path) -> Result<Vec<u8>, PersistentProjectRegistryError> {
    Err(PersistentProjectRegistryError::UnsupportedPlatform)
}

#[cfg(unix)]
fn path_from_bytes(bytes: &[u8]) -> Result<PathBuf, PersistentProjectRegistryError> {
    if bytes.contains(&0) {
        return Err(PersistentProjectRegistryError::DisplayRootContainsNul);
    }
    let path = PathBuf::from(OsString::from_vec(bytes.to_vec()));
    if !path.is_absolute() {
        return Err(PersistentProjectRegistryError::DisplayRootNotAbsolute(path));
    }
    Ok(path)
}

#[cfg(not(unix))]
fn path_from_bytes(_bytes: &[u8]) -> Result<PathBuf, PersistentProjectRegistryError> {
    Err(PersistentProjectRegistryError::UnsupportedPlatform)
}

fn ensure_supported_platform() -> Result<(), PersistentProjectRegistryError> {
    if cfg!(unix) {
        Ok(())
    } else {
        Err(PersistentProjectRegistryError::UnsupportedPlatform)
    }
}

/// Exact reason durable project registration or restoration failed.
#[derive(Debug)]
pub enum PersistentProjectRegistryError {
    UnsupportedPlatform,
    State(ProjectRegistryStateError),
    Store(StateStoreError),
    Registry(ProjectRegistryError),
    Boundary(crate::paths::RepositoryBoundaryError),
    UnknownProject(ProjectId),
    DisplayRootNotAbsolute(PathBuf),
    DisplayRootContainsNul,
    RepositoryObjectMismatch {
        path: PathBuf,
        expected: PersistedRepositoryObject,
        found: PersistedRepositoryObject,
    },
}

impl fmt::Display for PersistentProjectRegistryError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedPlatform => {
                formatter.write_str("persistent project registry requires Unix filesystem identity")
            }
            Self::State(source) => write!(
                formatter,
                "canonical project-registry state rejected: {source}"
            ),
            Self::Store(source) => write!(formatter, "project-registry store failed: {source}"),
            Self::Registry(source) => write!(formatter, "project registration failed: {source}"),
            Self::Boundary(source) => write!(formatter, "repository relocation failed: {source}"),
            Self::UnknownProject(project_id) => write!(formatter, "unknown project {project_id}"),
            Self::DisplayRootNotAbsolute(path) => {
                write!(
                    formatter,
                    "project display root is not absolute: {}",
                    path.display()
                )
            }
            Self::DisplayRootContainsNul => {
                formatter.write_str("project display root contains NUL")
            }
            Self::RepositoryObjectMismatch {
                path,
                expected,
                found,
            } => write!(
                formatter,
                "repository object changed at {}: expected {expected:?}, found {found:?}",
                path.display()
            ),
        }
    }
}

impl std::error::Error for PersistentProjectRegistryError {}

impl From<ProjectRegistryStateError> for PersistentProjectRegistryError {
    fn from(source: ProjectRegistryStateError) -> Self {
        Self::State(source)
    }
}

impl From<StateStoreError> for PersistentProjectRegistryError {
    fn from(source: StateStoreError) -> Self {
        Self::Store(source)
    }
}

impl From<ProjectRegistryError> for PersistentProjectRegistryError {
    fn from(source: ProjectRegistryError) -> Self {
        Self::Registry(source)
    }
}

impl From<crate::paths::RepositoryBoundaryError> for PersistentProjectRegistryError {
    fn from(source: crate::paths::RepositoryBoundaryError) -> Self {
        Self::Boundary(source)
    }
}
