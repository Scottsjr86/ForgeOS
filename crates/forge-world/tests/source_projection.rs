use forge_core::commands::{
    CommandAuthorityClass, CommandCancellationPolicy, CommandEnvironmentPolicy,
    CommandEnvironmentVariable, CommandTimeout, CommandWorkingDirectory, RegisteredCommand,
};
use forge_core::project_registry::{
    PersistedCommandDefinition, PersistedRepositoryObject, PersistentProjectEntry,
    ProjectRegistryState, SafeWorkspaceSnapshot,
};
use forge_core::projects::{
    AllowedProjectRoot, LanguageProfile, ManifestCommand, ProjectManifest, ProjectSetting,
};
use forge_protocol::identities::{CommandId, IDENTITY_BYTES, ProjectId, RepositoryId};
use forge_protocol::intents::ForgeUserIntent;
use forge_world::interaction::{WorldActionError, WorldActionRouter, WorldInputAction};
use forge_world::presentation::{
    PresentationFrame, ProjectRegistryProjection, Viewport, ViewportError,
};
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

fn entry(project: u8, repository: u8, name: &str, display_root: Vec<u8>) -> PersistentProjectEntry {
    let manifest = ProjectManifest::new(
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
    .unwrap();
    let commands = vec![
        PersistedCommandDefinition::from_registered(&command(2, repository, "Test", "test"))
            .unwrap(),
        PersistedCommandDefinition::from_registered(&command(1, repository, "Check", "check"))
            .unwrap(),
    ];
    PersistentProjectEntry::new(
        manifest,
        display_root,
        PersistedRepositoryObject::new(11, u64::from(project)),
        commands,
    )
    .unwrap()
}

fn populated_registry() -> ProjectRegistryState {
    let mut registry = ProjectRegistryState::empty();
    registry
        .register(entry(
            2,
            22,
            "Nyx",
            vec![b'/', b'w', b'o', b'r', b'k', b'/', 0xff],
        ))
        .unwrap();
    registry
        .register(entry(1, 11, "ForgeOS", b"/work/forgeos".to_vec()))
        .unwrap();
    registry.mark_open(project_id(2)).unwrap();
    registry.mark_open(project_id(1)).unwrap();
    registry.mark_closed(project_id(1)).unwrap();
    registry
        .set_last_safe_snapshot(
            project_id(2),
            SafeWorkspaceSnapshot::new(1, b"safe-workspace".to_vec()).unwrap(),
        )
        .unwrap();
    registry
}

#[test]
fn projection_preserves_source_identity_order_and_exact_display_data() {
    let registry = populated_registry();
    let source_before = registry.encode();

    let projection = ProjectRegistryProjection::from_registry(&registry);

    assert_eq!(registry.encode(), source_before);
    assert_eq!(projection.source_generation(), registry.generation());
    assert_eq!(
        projection.recent_project_ids(),
        &[project_id(1), project_id(2)]
    );
    assert_eq!(
        projection
            .projects()
            .iter()
            .map(|project| project.project_id())
            .collect::<Vec<_>>(),
        vec![project_id(1), project_id(2)]
    );

    let forge = projection.project(project_id(1)).unwrap();
    assert_eq!(forge.repository_id(), repository_id(11));
    assert_eq!(forge.display_name(), "ForgeOS");
    assert_eq!(forge.display_root().bytes(), b"/work/forgeos");
    assert!(!forge.is_open());
    assert_eq!(forge.last_open_sequence(), Some(2));
    assert_eq!(forge.recent_rank(), Some(0));
    assert_eq!(forge.safe_snapshot_identity(), None);
    assert_eq!(
        forge
            .commands()
            .iter()
            .map(|command| command.command_id())
            .collect::<Vec<_>>(),
        vec![command_id(1), command_id(2)]
    );

    let nyx = projection.project(project_id(2)).unwrap();
    assert_eq!(nyx.display_root().bytes(), b"/work/\xff");
    assert_eq!(nyx.display_root().escaped_text(), "/work/\\xff");
    assert!(nyx.is_open());
    assert_eq!(nyx.recent_rank(), Some(1));
    assert!(nyx.safe_snapshot_identity().is_some());
}

#[test]
fn display_name_path_and_list_position_never_replace_stable_identity() {
    let mut registry = ProjectRegistryState::empty();
    registry
        .register(entry(9, 29, "Same name", b"/same/path".to_vec()))
        .unwrap();
    registry
        .register(entry(3, 23, "Same name", b"/same/path".to_vec()))
        .unwrap();

    let projection = ProjectRegistryProjection::from_registry(&registry);
    assert_eq!(projection.projects().len(), 2);
    assert_eq!(projection.projects()[0].project_id(), project_id(3));
    assert_eq!(projection.projects()[1].project_id(), project_id(9));
    assert_ne!(
        projection.projects()[0].repository_id(),
        projection.projects()[1].repository_id()
    );
    assert_eq!(projection.projects()[0].display_name(), "Same name");
    assert_eq!(projection.projects()[1].display_name(), "Same name");
    assert_eq!(
        projection.projects()[0].display_root().bytes(),
        projection.projects()[1].display_root().bytes()
    );
}

#[test]
fn router_emits_typed_generation_bound_intents_without_mutating_core_state() {
    let registry = populated_registry();
    let source_before = registry.encode();
    let projection = ProjectRegistryProjection::from_registry(&registry);
    let router = WorldActionRouter::new(&projection);
    let generation = registry.generation();

    assert_eq!(
        router.route(WorldInputAction::RefreshProjects).unwrap(),
        ForgeUserIntent::RefreshProjectProjection {
            observed_generation: generation,
        }
    );
    assert_eq!(
        router
            .route(WorldInputAction::OpenProject(project_id(1)))
            .unwrap(),
        ForgeUserIntent::OpenProject {
            project_id: project_id(1),
            observed_generation: generation,
        }
    );
    assert_eq!(
        router
            .route(WorldInputAction::CloseProject(project_id(2)))
            .unwrap(),
        ForgeUserIntent::CloseProject {
            project_id: project_id(2),
            observed_generation: generation,
        }
    );
    assert_eq!(
        router
            .route(WorldInputAction::InvokeRegisteredCommand {
                project_id: project_id(1),
                command_id: command_id(2),
            })
            .unwrap(),
        ForgeUserIntent::InvokeRegisteredCommand {
            project_id: project_id(1),
            repository_id: repository_id(11),
            command_id: command_id(2),
            observed_generation: generation,
        }
    );
    assert_eq!(registry.encode(), source_before);
}

#[test]
fn router_rejects_unknown_identity_and_cross_project_command_guessing() {
    let registry = populated_registry();
    let projection = ProjectRegistryProjection::from_registry(&registry);
    let router = WorldActionRouter::new(&projection);

    assert_eq!(
        router.route(WorldInputAction::OpenProject(project_id(99))),
        Err(WorldActionError::UnknownProject(project_id(99)))
    );
    assert_eq!(
        router.route(WorldInputAction::InvokeRegisteredCommand {
            project_id: project_id(1),
            command_id: command_id(99),
        }),
        Err(WorldActionError::UnknownProjectCommand {
            project_id: project_id(1),
            command_id: command_id(99),
        })
    );
}

#[test]
fn rerender_and_resize_change_only_renderer_owned_frame_metadata() {
    let registry = populated_registry();
    let source_before = registry.encode();
    let projection = ProjectRegistryProjection::from_registry(&registry);
    let first = PresentationFrame::new(&projection, Viewport::new(1280, 720, 1000).unwrap());
    let resized = PresentationFrame::new(&projection, Viewport::new(1920, 1080, 1250).unwrap());

    assert_eq!(first.projection(), resized.projection());
    assert_ne!(first.viewport(), resized.viewport());
    assert_eq!(first.viewport().width(), 1280);
    assert_eq!(resized.viewport().height(), 1080);
    assert_eq!(resized.viewport().scale_milli(), 1250);
    assert_eq!(registry.encode(), source_before);
    assert_eq!(
        Viewport::new(0, 720, 1000),
        Err(ViewportError::ZeroExtent {
            width: 0,
            height: 720,
        })
    );
    assert_eq!(Viewport::new(1280, 720, 0), Err(ViewportError::ZeroScale));
}
