use forge_core::projects::{
    AllowedProjectRoot, LanguageProfile, ManifestCommand, ProjectManifest, ProjectSetting,
};
use forge_project::registry::{ProjectRegistry, ProjectRegistryError};
use forge_protocol::identities::{CommandId, ProjectId, RepositoryId};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_FIXTURE: AtomicU64 = AtomicU64::new(1);

fn id(byte: u8) -> [u8; 16] {
    [byte; 16]
}

struct FixtureRoot(PathBuf);

impl FixtureRoot {
    fn new(label: &str) -> Self {
        let serial = NEXT_FIXTURE.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "forgeos-project-registry-{}-{serial}-{label}",
            std::process::id()
        ));
        fs::create_dir_all(path.join("src")).unwrap();
        fs::create_dir_all(path.join("tests")).unwrap();
        Self(path)
    }

    fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for FixtureRoot {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn manifest(project: u8, repository: u8) -> ProjectManifest {
    ProjectManifest::new(
        ProjectId::from_bytes(id(project)),
        RepositoryId::from_bytes(id(repository)),
        "ForgeOS",
        vec![
            AllowedProjectRoot::repository_root(),
            AllowedProjectRoot::relative("src").unwrap(),
            AllowedProjectRoot::relative("tests").unwrap(),
        ],
        vec![ManifestCommand::new(CommandId::from_bytes(id(9)), "Check").unwrap()],
        LanguageProfile::Rust,
        vec![ProjectSetting::new("rust.edition", "2024").unwrap()],
    )
    .unwrap()
}

#[test]
fn valid_manifest_import_and_reopen_are_equivalent() {
    let root = FixtureRoot::new("equivalent");
    let manifest = manifest(1, 2);
    let bytes = manifest.encode();

    let mut first_registry = ProjectRegistry::new();
    let first = first_registry.import_bytes(&bytes, root.path()).unwrap();
    let mut second_registry = ProjectRegistry::new();
    let second = second_registry.import_bytes(&bytes, root.path()).unwrap();

    assert_eq!(first.manifest(), second.manifest());
    assert_eq!(first.boundary().root_object(), second.boundary().root_object());
}

#[test]
fn duplicate_project_and_repository_ids_are_rejected() {
    let first_root = FixtureRoot::new("first");
    let second_root = FixtureRoot::new("second");
    let mut registry = ProjectRegistry::new();
    registry
        .import_manifest(manifest(1, 2), first_root.path())
        .unwrap();

    assert!(matches!(
        registry.import_manifest(manifest(1, 3), second_root.path()),
        Err(ProjectRegistryError::DuplicateProjectId(_))
    ));
    assert!(matches!(
        registry.import_manifest(manifest(4, 2), second_root.path()),
        Err(ProjectRegistryError::DuplicateRepositoryId { .. })
    ));
}

#[test]
fn moved_repository_keeps_identity_only_for_the_same_directory_object() {
    let root = FixtureRoot::new("move-source");
    let parent = root.path().parent().unwrap().to_path_buf();
    let moved = parent.join(format!(
        "forgeos-project-registry-{}-moved-{}",
        std::process::id(),
        NEXT_FIXTURE.fetch_add(1, Ordering::Relaxed)
    ));

    let mut registry = ProjectRegistry::new();
    let project_id = manifest(1, 2).project_id();
    registry
        .import_manifest(manifest(1, 2), root.path())
        .unwrap();
    let original_object = registry.get(project_id).unwrap().boundary().root_object();

    fs::rename(root.path(), &moved).unwrap();
    registry
        .get_mut(project_id)
        .unwrap()
        .relocate(&moved)
        .unwrap();
    assert_eq!(
        registry.get(project_id).unwrap().boundary().root_object(),
        original_object
    );

    fs::rename(&moved, root.path()).unwrap();
}

#[test]
fn missing_or_non_directory_allowed_roots_fail_import() {
    let root = FixtureRoot::new("invalid-root");
    fs::write(root.path().join("not-a-directory"), b"file").unwrap();
    let invalid = ProjectManifest::new(
        ProjectId::from_bytes(id(1)),
        RepositoryId::from_bytes(id(2)),
        "ForgeOS",
        vec![AllowedProjectRoot::relative("not-a-directory").unwrap()],
        Vec::new(),
        LanguageProfile::Rust,
        Vec::new(),
    )
    .unwrap();

    let mut registry = ProjectRegistry::new();
    assert!(matches!(
        registry.import_manifest(invalid, root.path()),
        Err(ProjectRegistryError::AllowedRootNotDirectory { .. })
    ));
}
