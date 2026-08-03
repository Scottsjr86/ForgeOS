use forge_core::commands::{
    CommandAuthorityClass, CommandCancellationPolicy, CommandEnvironmentPolicy,
    CommandEnvironmentVariable, CommandTimeout, CommandWorkingDirectory, RegisteredCommand,
};
use forge_core::project_registry::{
    PersistedCommandDefinition, PersistedRepositoryObject, PersistentProjectEntry,
    ProjectRegistryState, ProjectRegistryStateError, SafeWorkspaceSnapshot,
    PROJECT_REGISTRY_RECORD_TYPE,
};
use forge_core::projects::{
    AllowedProjectRoot, LanguageProfile, ManifestCommand, ProjectManifest, ProjectSetting,
};
use forge_protocol::hashes::{hash_canonical_bytes, HashDomain};
use forge_protocol::identities::{CommandId, ProjectId, RepositoryId, IDENTITY_BYTES};
use std::time::Duration;

fn project_id(byte: u8) -> ProjectId {
    ProjectId::from_bytes([byte; IDENTITY_BYTES])
}

fn repository_id(byte: u8) -> RepositoryId {
    RepositoryId::from_bytes([byte; IDENTITY_BYTES])
}

fn command_id(byte: u8) -> CommandId {
    CommandId::from_bytes([byte; IDENTITY_BYTES])
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
        CommandTimeout::after(Duration::from_secs(30)).unwrap(),
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
        vec![AllowedProjectRoot::repository_root()],
        vec![
            ManifestCommand::new(command_id(1), "Check").unwrap(),
            ManifestCommand::new(command_id(2), "Test").unwrap(),
        ],
        LanguageProfile::Rust,
        vec![ProjectSetting::new("rust.edition", "2024").unwrap()],
    )
    .unwrap()
}

fn entry(project: u8, repository: u8, name: &str) -> PersistentProjectEntry {
    let commands = vec![
        PersistedCommandDefinition::from_registered(&command(2, repository, "Test", "test"))
            .unwrap(),
        PersistedCommandDefinition::from_registered(&command(1, repository, "Check", "check"))
            .unwrap(),
    ];
    PersistentProjectEntry::new(
        manifest(project, repository, name),
        format!("/workspace/project-{project}").into_bytes(),
        PersistedRepositoryObject::new(11, u64::from(project)),
        commands,
    )
    .unwrap()
}

#[test]
fn canonical_registry_round_trip_preserves_every_project_field() {
    let mut state = ProjectRegistryState::empty();
    state.register(entry(1, 11, "ForgeOS")).unwrap();
    state.register(entry(2, 22, "Nyx")).unwrap();
    state.mark_open(project_id(2)).unwrap();
    state.mark_closed(project_id(2)).unwrap();
    state
        .set_last_safe_snapshot(
            project_id(1),
            SafeWorkspaceSnapshot::new(1, b"buffers=3;active=README.md".to_vec()).unwrap(),
        )
        .unwrap();

    let reopened = ProjectRegistryState::decode(&state.encode()).unwrap();
    assert_eq!(reopened, state);
    assert_eq!(reopened.recent_projects(), vec![project_id(2)]);
    assert_eq!(
        reopened
            .get(project_id(1))
            .unwrap()
            .last_safe_snapshot()
            .unwrap()
            .payload(),
        b"buffers=3;active=README.md"
    );
    assert_eq!(reopened.get(project_id(1)).unwrap().commands().len(), 2);
}

#[test]
fn exact_registry_payload_identity_is_golden_locked() {
    let mut state = ProjectRegistryState::empty();
    state.register(entry(1, 11, "ForgeOS")).unwrap();
    state.mark_open(project_id(1)).unwrap();
    let identity = hash_canonical_bytes(HashDomain::Snapshot, &state.encode());
    assert_eq!(
        identity.to_string(),
        "6f9fc3d2d34c0aff8c8afbbd875a9d657878514ff7575e650a2763592c001b14"
    );
}

#[test]
fn state_record_round_trip_uses_the_reserved_registry_type() {
    let mut state = ProjectRegistryState::empty();
    state.register(entry(1, 11, "ForgeOS")).unwrap();
    let record = state.to_state_record().unwrap();
    assert_eq!(record.record_type(), PROJECT_REGISTRY_RECORD_TYPE);
    assert_eq!(
        ProjectRegistryState::from_state_record(&record).unwrap(),
        state
    );
}

#[test]
fn recent_open_order_is_monotonic_and_independent_from_wall_clock_time() {
    let mut state = ProjectRegistryState::empty();
    state.register(entry(1, 11, "One")).unwrap();
    state.register(entry(2, 22, "Two")).unwrap();
    assert_eq!(state.mark_open(project_id(1)).unwrap(), 1);
    assert_eq!(state.mark_open(project_id(2)).unwrap(), 2);
    assert_eq!(state.mark_open(project_id(1)).unwrap(), 3);
    state.mark_closed(project_id(1)).unwrap();
    assert_eq!(state.recent_projects(), vec![project_id(1), project_id(2)]);
    assert!(!state.get(project_id(1)).unwrap().recent_open().is_open());
    assert_eq!(
        state
            .get(project_id(1))
            .unwrap()
            .recent_open()
            .last_open_sequence(),
        Some(3)
    );
}

#[test]
fn rename_and_snapshot_mutate_only_the_selected_project() {
    let mut state = ProjectRegistryState::empty();
    state.register(entry(1, 11, "One")).unwrap();
    state.register(entry(2, 22, "Two")).unwrap();
    let untouched = state.get(project_id(2)).unwrap().clone();

    state.rename(project_id(1), "Renamed").unwrap();
    state
        .set_last_safe_snapshot(
            project_id(1),
            SafeWorkspaceSnapshot::new(7, b"safe".to_vec()).unwrap(),
        )
        .unwrap();

    assert_eq!(state.get(project_id(2)).unwrap(), &untouched);
    assert_eq!(
        state.get(project_id(1)).unwrap().manifest().display_name(),
        "Renamed"
    );
}

#[test]
fn duplicate_project_and_repository_identity_fail_closed() {
    let mut state = ProjectRegistryState::empty();
    state.register(entry(1, 11, "One")).unwrap();
    assert_eq!(
        state.register(entry(1, 22, "Duplicate project")),
        Err(ProjectRegistryStateError::DuplicateProjectId(project_id(1)))
    );
    assert_eq!(
        state.register(entry(2, 11, "Duplicate repository")),
        Err(ProjectRegistryStateError::DuplicateRepositoryId {
            repository_id: repository_id(11),
            existing_project: project_id(1),
        })
    );
}

#[test]
fn command_set_must_match_manifest_identity_repository_and_name() {
    let wrong_repository =
        PersistedCommandDefinition::from_registered(&command(1, 99, "Check", "check")).unwrap();
    let right_test =
        PersistedCommandDefinition::from_registered(&command(2, 11, "Test", "test")).unwrap();
    assert!(matches!(
        PersistentProjectEntry::new(
            manifest(1, 11, "ForgeOS"),
            b"/workspace/forgeos".to_vec(),
            PersistedRepositoryObject::new(1, 2),
            vec![wrong_repository, right_test],
        ),
        Err(ProjectRegistryStateError::CommandRepositoryMismatch { .. })
    ));
}

#[test]
fn corrupt_snapshot_identity_unknown_schema_and_trailing_bytes_are_rejected() {
    let mut state = ProjectRegistryState::empty();
    state.register(entry(1, 11, "ForgeOS")).unwrap();
    state
        .set_last_safe_snapshot(
            project_id(1),
            SafeWorkspaceSnapshot::new(1, b"safe-state".to_vec()).unwrap(),
        )
        .unwrap();
    let bytes = state.encode();

    let mut unsupported = bytes.clone();
    unsupported[8..10].copy_from_slice(&2u16.to_be_bytes());
    assert_eq!(
        ProjectRegistryState::decode(&unsupported),
        Err(ProjectRegistryStateError::UnsupportedSchemaVersion(2))
    );

    let mut trailing = bytes.clone();
    trailing.push(0xff);
    assert_eq!(
        ProjectRegistryState::decode(&trailing),
        Err(ProjectRegistryStateError::TrailingBytes(1))
    );

    let mut tampered = bytes;
    let payload_position = tampered
        .windows(b"safe-state".len())
        .position(|window| window == b"safe-state")
        .unwrap();
    tampered[payload_position] ^= 0x01;
    assert!(matches!(
        ProjectRegistryState::decode(&tampered),
        Err(ProjectRegistryStateError::SnapshotIdentityMismatch { .. })
    ));
}

#[test]
fn removal_returns_the_record_without_mutating_other_records() {
    let mut state = ProjectRegistryState::empty();
    state.register(entry(1, 11, "One")).unwrap();
    state.register(entry(2, 22, "Two")).unwrap();
    let untouched = state.get(project_id(2)).unwrap().clone();
    let removed = state.remove(project_id(1)).unwrap();
    assert_eq!(removed.manifest().project_id(), project_id(1));
    assert_eq!(state.get(project_id(2)).unwrap(), &untouched);
    assert_eq!(state.len(), 1);
}
