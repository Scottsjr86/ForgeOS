#![cfg(unix)]

use forge_core::commands::{
    CommandAuthorityClass, CommandCancellationPolicy, CommandEnvironmentPolicy,
    CommandEnvironmentVariable, CommandTimeout, CommandWorkingDirectory, RegisteredCommand,
};
use forge_core::project_registry::SafeWorkspaceSnapshot;
use forge_core::projects::{
    AllowedProjectRoot, LanguageProfile, ManifestCommand, ProjectManifest, ProjectSetting,
};
use forge_project::registry_store::{PersistentProjectRegistry, PersistentProjectRegistryError};
use forge_protocol::identities::{CommandId, IDENTITY_BYTES, ProjectId, RepositoryId};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

static NEXT_FIXTURE: AtomicU64 = AtomicU64::new(1);

fn project_id(byte: u8) -> ProjectId {
    ProjectId::from_bytes([byte; IDENTITY_BYTES])
}

fn repository_id(byte: u8) -> RepositoryId {
    RepositoryId::from_bytes([byte; IDENTITY_BYTES])
}

fn command_id(byte: u8) -> CommandId {
    CommandId::from_bytes([byte; IDENTITY_BYTES])
}

fn fixture_root(label: &str) -> PathBuf {
    let number = NEXT_FIXTURE.fetch_add(1, Ordering::Relaxed);
    let path = std::env::temp_dir().join(format!(
        "forgeos-project-200-{}-{label}-{number}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&path);
    fs::create_dir_all(&path).unwrap();
    path
}

fn command(id: u8, repository: u8, name: &str, argument: &str) -> RegisteredCommand {
    RegisteredCommand::new(
        command_id(id),
        repository_id(repository),
        name,
        "cargo",
        [argument],
        CommandWorkingDirectory::repository_root(),
        CommandEnvironmentPolicy::clear(vec![CommandEnvironmentVariable::inherit("PATH").unwrap()])
            .unwrap(),
        CommandTimeout::after(Duration::from_secs(60)).unwrap(),
        CommandCancellationPolicy::TerminateProcessGroup,
        CommandAuthorityClass::Build,
    )
    .unwrap()
}

fn manifest(project: u8, repository: u8, name: &str) -> ProjectManifest {
    ProjectManifest::new(
        project_id(project),
        repository_id(repository),
        name,
        vec![
            AllowedProjectRoot::repository_root(),
            AllowedProjectRoot::relative("src").unwrap(),
        ],
        vec![
            ManifestCommand::new(command_id(1), "Check").unwrap(),
            ManifestCommand::new(command_id(2), "Test").unwrap(),
        ],
        LanguageProfile::Rust,
        vec![ProjectSetting::new("rust.edition", "2024").unwrap()],
    )
    .unwrap()
}

fn commands(repository: u8) -> Vec<RegisteredCommand> {
    vec![
        command(2, repository, "Test", "test"),
        command(1, repository, "Check", "check"),
    ]
}

fn repository(parent: &Path, name: &str, source: &[u8]) -> PathBuf {
    let root = parent.join(name);
    fs::create_dir_all(root.join("src")).unwrap();
    fs::write(root.join("src/lib.rs"), source).unwrap();
    root
}

#[test]
fn register_rename_close_reopen_and_remove_never_touch_source() {
    let fixture = fixture_root("lifecycle");
    let repo = repository(&fixture, "repo", b"pub fn answer() -> u8 { 42 }\n");
    let state_path = fixture.join("registry.state");
    let original_source = fs::read(repo.join("src/lib.rs")).unwrap();

    let mut registry = PersistentProjectRegistry::create(&state_path).unwrap();
    registry
        .register(manifest(1, 11, "ForgeOS"), &commands(11), &repo)
        .unwrap();
    registry.rename(project_id(1), "ForgeOS Renamed").unwrap();
    assert_eq!(registry.mark_open(project_id(1)).unwrap(), 1);
    registry
        .set_last_safe_snapshot(
            project_id(1),
            SafeWorkspaceSnapshot::new(1, b"active=src/lib.rs;buffers=1".to_vec()).unwrap(),
        )
        .unwrap();
    registry.mark_closed(project_id(1)).unwrap();
    drop(registry);

    let mut reopened = PersistentProjectRegistry::open(&state_path).unwrap();
    let restored = reopened.project(project_id(1)).unwrap();
    assert_eq!(restored.manifest().display_name(), "ForgeOS Renamed");
    assert!(!restored.recent_open().is_open());
    assert_eq!(restored.recent_open().last_open_sequence(), Some(1));
    assert_eq!(
        restored.last_safe_snapshot().unwrap().payload(),
        b"active=src/lib.rs;buffers=1"
    );
    assert_eq!(restored.entry().commands().len(), 2);
    assert_eq!(fs::read(repo.join("src/lib.rs")).unwrap(), original_source);

    reopened.remove(project_id(1)).unwrap();
    assert!(reopened.is_empty());
    assert!(repo.exists());
    assert_eq!(fs::read(repo.join("src/lib.rs")).unwrap(), original_source);
}

#[test]
fn recent_open_order_survives_process_restart() {
    let fixture = fixture_root("recent");
    let first = repository(&fixture, "first", b"first\n");
    let second = repository(&fixture, "second", b"second\n");
    let state_path = fixture.join("registry.state");
    let mut registry = PersistentProjectRegistry::create(&state_path).unwrap();
    registry
        .register(manifest(1, 11, "First"), &commands(11), &first)
        .unwrap();
    registry
        .register(manifest(2, 22, "Second"), &commands(22), &second)
        .unwrap();
    registry.mark_open(project_id(1)).unwrap();
    registry.mark_open(project_id(2)).unwrap();
    registry.mark_open(project_id(1)).unwrap();
    drop(registry);

    let reopened = PersistentProjectRegistry::open(&state_path).unwrap();
    assert_eq!(
        reopened.recent_projects(),
        vec![project_id(1), project_id(2)]
    );
}

#[test]
fn exact_command_definitions_survive_reopen() {
    let fixture = fixture_root("commands");
    let repo = repository(&fixture, "repo", b"commands\n");
    let state_path = fixture.join("registry.state");
    let definitions = commands(11);
    let mut expected: Vec<_> = definitions
        .iter()
        .map(|command| {
            (
                command.command_id(),
                command.definition_identity(),
                command.canonical_bytes(),
            )
        })
        .collect();
    expected.sort_by(|left, right| left.0.as_bytes().cmp(right.0.as_bytes()));
    let mut registry = PersistentProjectRegistry::create(&state_path).unwrap();
    registry
        .register(manifest(1, 11, "ForgeOS"), &definitions, &repo)
        .unwrap();
    drop(registry);

    let reopened = PersistentProjectRegistry::open(&state_path).unwrap();
    let persisted = reopened.project(project_id(1)).unwrap().entry().commands();
    let actual: Vec<_> = persisted
        .iter()
        .map(|command| {
            (
                command.command_id(),
                command.identity(),
                command.canonical_bytes().to_vec(),
            )
        })
        .collect();
    assert_eq!(actual, expected);
    let restored = persisted[0].decode_registered().unwrap();
    assert_eq!(restored.command_id(), command_id(1));
    assert_eq!(restored.program(), "cargo");
    assert_eq!(
        restored
            .arguments()
            .iter()
            .map(String::as_str)
            .collect::<Vec<_>>(),
        vec!["check"]
    );
}

#[test]
fn unrelated_project_record_remains_equivalent_after_selected_mutation() {
    let fixture = fixture_root("isolation");
    let first = repository(&fixture, "first", b"first\n");
    let second = repository(&fixture, "second", b"second\n");
    let state_path = fixture.join("registry.state");
    let mut registry = PersistentProjectRegistry::create(&state_path).unwrap();
    registry
        .register(manifest(1, 11, "First"), &commands(11), &first)
        .unwrap();
    registry
        .register(manifest(2, 22, "Second"), &commands(22), &second)
        .unwrap();
    let untouched = registry.state().get(project_id(2)).unwrap().clone();

    registry.rename(project_id(1), "Renamed").unwrap();
    registry.mark_open(project_id(1)).unwrap();

    assert_eq!(registry.state().get(project_id(2)).unwrap(), &untouched);
}

#[test]
fn duplicate_identity_failure_does_not_publish_partial_state() {
    let fixture = fixture_root("duplicate");
    let first = repository(&fixture, "first", b"first\n");
    let second = repository(&fixture, "second", b"second\n");
    let state_path = fixture.join("registry.state");
    let mut registry = PersistentProjectRegistry::create(&state_path).unwrap();
    registry
        .register(manifest(1, 11, "First"), &commands(11), &first)
        .unwrap();
    let before = fs::read(&state_path).unwrap();

    assert!(
        registry
            .register(manifest(1, 22, "Duplicate"), &commands(22), &second)
            .is_err()
    );
    assert_eq!(fs::read(&state_path).unwrap(), before);
    assert_eq!(registry.len(), 1);
}

#[test]
fn same_repository_object_can_relocate_and_reopen() {
    let fixture = fixture_root("relocate");
    let original = repository(&fixture, "original", b"move me\n");
    let moved = fixture.join("moved");
    let state_path = fixture.join("registry.state");
    let mut registry = PersistentProjectRegistry::create(&state_path).unwrap();
    registry
        .register(manifest(1, 11, "ForgeOS"), &commands(11), &original)
        .unwrap();
    fs::rename(&original, &moved).unwrap();
    registry.relocate(project_id(1), &moved).unwrap();
    drop(registry);

    let reopened = PersistentProjectRegistry::open(&state_path).unwrap();
    assert_eq!(
        reopened
            .project(project_id(1))
            .unwrap()
            .registered()
            .boundary()
            .display_root(),
        moved.as_path()
    );
    assert_eq!(fs::read(moved.join("src/lib.rs")).unwrap(), b"move me\n");
}

#[test]
fn copied_or_replaced_repository_root_is_rejected_on_reopen() {
    let fixture = fixture_root("replace-root");
    let repo = repository(&fixture, "repo", b"original\n");
    let old = fixture.join("old-repo");
    let state_path = fixture.join("registry.state");
    let mut registry = PersistentProjectRegistry::create(&state_path).unwrap();
    registry
        .register(manifest(1, 11, "ForgeOS"), &commands(11), &repo)
        .unwrap();
    drop(registry);

    fs::rename(&repo, &old).unwrap();
    repository(&fixture, "repo", b"replacement\n");
    assert!(matches!(
        PersistentProjectRegistry::open(&state_path),
        Err(PersistentProjectRegistryError::RepositoryObjectMismatch { .. })
    ));
    assert_eq!(fs::read(old.join("src/lib.rs")).unwrap(), b"original\n");
}

#[test]
fn invalid_allowed_root_and_wrong_command_repository_fail_before_publish() {
    let fixture = fixture_root("invalid");
    let repo = repository(&fixture, "repo", b"source\n");
    let state_path = fixture.join("registry.state");
    let mut registry = PersistentProjectRegistry::create(&state_path).unwrap();
    fs::remove_dir_all(repo.join("src")).unwrap();
    fs::write(repo.join("src"), b"not a directory").unwrap();

    assert!(
        registry
            .register(manifest(1, 11, "ForgeOS"), &commands(11), &repo)
            .is_err()
    );
    assert!(registry.is_empty());

    fs::remove_file(repo.join("src")).unwrap();
    fs::create_dir(repo.join("src")).unwrap();
    assert!(
        registry
            .register(manifest(1, 11, "ForgeOS"), &commands(99), &repo)
            .is_err()
    );
    assert!(registry.is_empty());
}

#[test]
fn corrupt_current_state_fails_closed_without_default_registry() {
    let fixture = fixture_root("corrupt");
    let repo = repository(&fixture, "repo", b"source\n");
    let state_path = fixture.join("registry.state");
    let mut registry = PersistentProjectRegistry::create(&state_path).unwrap();
    registry
        .register(manifest(1, 11, "ForgeOS"), &commands(11), &repo)
        .unwrap();
    drop(registry);

    let mut bytes = fs::read(&state_path).unwrap();
    let middle = bytes.len() / 2;
    bytes[middle] ^= 0x5a;
    fs::write(&state_path, bytes).unwrap();
    assert!(PersistentProjectRegistry::open(&state_path).is_err());
}

#[test]
fn interrupted_staging_is_visible_and_explicitly_discarded() {
    let fixture = fixture_root("interrupted");
    let state_path = fixture.join("registry.state");
    let registry = PersistentProjectRegistry::create(&state_path).unwrap();
    drop(registry);
    let staged = fixture.join("registry.state.forgeos-staged");
    fs::write(&staged, b"abandoned").unwrap();

    let mut reopened = PersistentProjectRegistry::open(&state_path).unwrap();
    assert!(reopened.interrupted_write_present());
    assert!(reopened.discard_interrupted_write().unwrap());
    assert!(!reopened.interrupted_write_present());
    assert!(!staged.exists());
}
