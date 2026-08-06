#![cfg(target_os = "linux")]

use forge_app::composition::command_workspace::{
    ProjectCommandWorkspace, ProjectCommandWorkspaceError,
};
use forge_bridge::processes::CancellationToken;
use forge_core::commands::{
    CommandAuthorityClass, CommandCancellationPolicy, CommandEnvironmentPolicy,
    CommandEnvironmentVariable, CommandTimeout, CommandWorkingDirectory, RegisteredCommand,
};
use forge_core::projects::{AllowedProjectRoot, LanguageProfile, ProjectManifest};
use forge_project::paths::RepositoryBoundary;
use forge_protocol::hashes::ContentHash;
use forge_protocol::identities::{CommandId, IDENTITY_BYTES, ProcessId, ProjectId, RepositoryId};
use forge_protocol::paths::RepositoryRelativePath;
use forge_protocol::processes::ProcessOutcome;
use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_FIXTURE: AtomicU64 = AtomicU64::new(1);

struct Fixture {
    root: PathBuf,
    repository: PathBuf,
    manifest: ProjectManifest,
    boundary: RepositoryBoundary,
    revision: ContentHash,
}

impl Fixture {
    fn new(label: &str, allowed_roots: Vec<AllowedProjectRoot>) -> Self {
        let sequence = NEXT_FIXTURE.fetch_add(1, Ordering::Relaxed);
        let root = std::env::temp_dir().join(format!(
            "forgeos-command-workspace-{label}-{}-{sequence}",
            std::process::id()
        ));
        let repository = root.join("repository");
        if root.exists() {
            fs::remove_dir_all(&root).expect("remove stale fixture");
        }
        fs::create_dir_all(repository.join("src/nested")).expect("create fixture");
        let repository = fs::canonicalize(repository).expect("canonical repository");
        let project_id = ProjectId::from_bytes([(sequence as u8).wrapping_add(20); IDENTITY_BYTES]);
        let repository_id =
            RepositoryId::from_bytes([(sequence as u8).wrapping_add(90); IDENTITY_BYTES]);
        let manifest = ProjectManifest::new(
            project_id,
            repository_id,
            "Command workspace fixture",
            allowed_roots,
            Vec::new(),
            LanguageProfile::Rust,
            Vec::new(),
        )
        .expect("valid manifest");
        let boundary = RepositoryBoundary::open(repository_id, &repository).expect("boundary");
        Self {
            root,
            repository,
            manifest,
            boundary,
            revision: ContentHash::from_bytes([(sequence as u8).wrapping_add(150); 32]),
        }
    }

    fn command(
        &self,
        command_byte: u8,
        working_directory: CommandWorkingDirectory,
        script: &str,
    ) -> RegisteredCommand {
        RegisteredCommand::new(
            CommandId::from_bytes([command_byte; IDENTITY_BYTES]),
            self.manifest.repository_id(),
            format!("Command {command_byte}"),
            "/bin/sh",
            ["-c", script],
            working_directory,
            CommandEnvironmentPolicy::clear(vec![
                CommandEnvironmentVariable::inherit("TOKEN").unwrap(),
            ])
            .unwrap(),
            CommandTimeout::Unlimited,
            CommandCancellationPolicy::TerminateProcessGroup,
            CommandAuthorityClass::Build,
        )
        .expect("valid command")
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

#[test]
fn registered_command_runs_in_declared_scope_and_records_project_revision() {
    let fixture = Fixture::new(
        "success",
        vec![AllowedProjectRoot::relative("src").unwrap()],
    );
    let command = fixture.command(
        1,
        CommandWorkingDirectory::relative(RepositoryRelativePath::new("src/nested").unwrap())
            .unwrap(),
        "printf '%s|%s' \"$PWD\" \"$TOKEN\"",
    );
    let definition = command.definition_identity();
    let mut workspace = ProjectCommandWorkspace::new(
        &fixture.manifest,
        fixture.boundary.clone(),
        fixture.revision,
        [command],
        BTreeMap::from([("TOKEN".to_owned(), "declared".to_owned())]),
    )
    .expect("workspace");
    let process_id = ProcessId::from_bytes([11; IDENTITY_BYTES]);
    let record = workspace
        .run(
            CommandId::from_bytes([1; IDENTITY_BYTES]),
            definition,
            process_id,
            &CancellationToken::new(),
        )
        .expect("run command");
    assert_eq!(record.source().project_id(), fixture.manifest.project_id());
    assert_eq!(
        record.source().repository_id(),
        fixture.manifest.repository_id()
    );
    assert_eq!(record.source().revision(), fixture.revision);
    assert_eq!(record.process_id(), process_id);
    assert!(matches!(
        record.execution().outcome(),
        ProcessOutcome::Exited(exit) if exit.success()
    ));
    assert_eq!(
        record.execution().output().stdout(),
        format!(
            "{}|declared",
            fixture.repository.join("src/nested").display()
        )
        .as_bytes()
    );
    assert_eq!(workspace.history().len(), 1);
}

#[test]
fn stale_definition_and_unknown_command_fail_before_process_creation() {
    let fixture = Fixture::new("identity", vec![AllowedProjectRoot::repository_root()]);
    let command = fixture.command(2, CommandWorkingDirectory::repository_root(), "exit 0");
    let mut workspace = ProjectCommandWorkspace::new(
        &fixture.manifest,
        fixture.boundary.clone(),
        fixture.revision,
        [command],
        BTreeMap::from([("TOKEN".to_owned(), "declared".to_owned())]),
    )
    .unwrap();
    let command_id = CommandId::from_bytes([2; IDENTITY_BYTES]);
    assert!(matches!(
        workspace.run(
            command_id,
            ContentHash::from_bytes([99; 32]),
            ProcessId::from_bytes([12; IDENTITY_BYTES]),
            &CancellationToken::new(),
        ),
        Err(ProjectCommandWorkspaceError::StaleCommandDefinition { .. })
    ));
    assert!(matches!(
        workspace.run(
            CommandId::from_bytes([99; IDENTITY_BYTES]),
            ContentHash::from_bytes([1; 32]),
            ProcessId::from_bytes([13; IDENTITY_BYTES]),
            &CancellationToken::new(),
        ),
        Err(ProjectCommandWorkspaceError::UnknownCommand(_))
    ));
    assert_eq!(workspace.history().len(), 0);
}

#[test]
fn broader_working_directory_is_rejected_by_project_scope() {
    let fixture = Fixture::new(
        "scope",
        vec![AllowedProjectRoot::relative("src/nested").unwrap()],
    );
    let command = fixture.command(
        3,
        CommandWorkingDirectory::relative(RepositoryRelativePath::new("src").unwrap()).unwrap(),
        "printf 'must-not-run'",
    );
    let definition = command.definition_identity();
    let mut workspace = ProjectCommandWorkspace::new(
        &fixture.manifest,
        fixture.boundary.clone(),
        fixture.revision,
        [command],
        BTreeMap::from([("TOKEN".to_owned(), "declared".to_owned())]),
    )
    .unwrap();
    assert!(matches!(
        workspace.run(
            CommandId::from_bytes([3; IDENTITY_BYTES]),
            definition,
            ProcessId::from_bytes([14; IDENTITY_BYTES]),
            &CancellationToken::new(),
        ),
        Err(ProjectCommandWorkspaceError::WorkingDirectoryOutsideAllowedRoots(path))
            if path == PathBuf::from("src")
    ));
    assert_eq!(workspace.history().len(), 0);
}

#[test]
fn commands_from_another_repository_cannot_enter_the_workspace() {
    let fixture = Fixture::new("foreign", vec![AllowedProjectRoot::repository_root()]);
    let foreign = RepositoryId::from_bytes([44; IDENTITY_BYTES]);
    let command = RegisteredCommand::new(
        CommandId::from_bytes([4; IDENTITY_BYTES]),
        foreign,
        "Foreign command",
        "/bin/sh",
        ["-c", "exit 0"],
        CommandWorkingDirectory::repository_root(),
        CommandEnvironmentPolicy::empty(),
        CommandTimeout::Unlimited,
        CommandCancellationPolicy::TerminateProcessGroup,
        CommandAuthorityClass::Inspect,
    )
    .unwrap();
    assert!(matches!(
        ProjectCommandWorkspace::new(
            &fixture.manifest,
            fixture.boundary.clone(),
            fixture.revision,
            [command],
            BTreeMap::new(),
        ),
        Err(ProjectCommandWorkspaceError::CommandRepositoryMismatch {
            command,
            project,
            ..
        }) if command == foreign && project == fixture.manifest.repository_id()
    ));
}
