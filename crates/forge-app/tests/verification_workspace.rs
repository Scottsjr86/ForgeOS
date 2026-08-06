#![cfg(target_os = "linux")]

use forge_app::composition::verification_workspace::{
    ProjectVerificationWorkspace, ProjectVerificationWorkspaceError, VerificationApplicability,
};
use forge_bridge::processes::CancellationToken;
use forge_core::commands::{
    CommandAuthorityClass, CommandCancellationPolicy, CommandEnvironmentPolicy, CommandTimeout,
    CommandWorkingDirectory, RegisteredCommand,
};
use forge_core::projects::{AllowedProjectRoot, LanguageProfile, ProjectManifest};
use forge_core::verification::VerificationOutcome;
use forge_project::paths::RepositoryBoundary;
use forge_protocol::identities::{CommandId, IDENTITY_BYTES, ProcessId, ProjectId, RepositoryId};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

static NEXT_FIXTURE: AtomicU64 = AtomicU64::new(1);

struct Fixture {
    root: PathBuf,
    repository: PathBuf,
    manifest: ProjectManifest,
    boundary: RepositoryBoundary,
}

impl Fixture {
    fn new(label: &str) -> Self {
        let sequence = NEXT_FIXTURE.fetch_add(1, Ordering::Relaxed);
        let root = std::env::temp_dir().join(format!(
            "forgeos-verification-workspace-{label}-{}-{sequence}",
            std::process::id()
        ));
        if root.exists() {
            fs::remove_dir_all(&root).unwrap();
        }
        let repository = root.join("repository");
        fs::create_dir_all(repository.join("src")).unwrap();
        git_ok(&repository, &["init", "-q"]);
        git_ok(&repository, &["symbolic-ref", "HEAD", "refs/heads/main"]);
        git_ok(&repository, &["config", "user.name", "ForgeOS Test"]);
        git_ok(
            &repository,
            &["config", "user.email", "forgeos@example.invalid"],
        );
        fs::write(repository.join("src/lib.rs"), b"pub fn initial() {}\n").unwrap();
        git_ok(&repository, &["add", "--", "."]);
        git_ok(&repository, &["commit", "-q", "-m", "initial"]);
        let repository = fs::canonicalize(repository).unwrap();
        let project_id = ProjectId::from_bytes([(sequence as u8).wrapping_add(20); IDENTITY_BYTES]);
        let repository_id =
            RepositoryId::from_bytes([(sequence as u8).wrapping_add(100); IDENTITY_BYTES]);
        let manifest = ProjectManifest::new(
            project_id,
            repository_id,
            "Verification fixture",
            vec![AllowedProjectRoot::repository_root()],
            Vec::new(),
            LanguageProfile::Rust,
            Vec::new(),
        )
        .unwrap();
        let boundary = RepositoryBoundary::open(repository_id, &repository).unwrap();
        Self {
            root,
            repository,
            manifest,
            boundary,
        }
    }

    fn command(&self, byte: u8, script: &str, timeout: CommandTimeout) -> RegisteredCommand {
        RegisteredCommand::new(
            CommandId::from_bytes([byte; IDENTITY_BYTES]),
            self.manifest.repository_id(),
            format!("Verification command {byte}"),
            "/bin/sh",
            ["-c", script],
            CommandWorkingDirectory::repository_root(),
            CommandEnvironmentPolicy::empty(),
            timeout,
            CommandCancellationPolicy::TerminateProcessGroup,
            CommandAuthorityClass::Build,
        )
        .unwrap()
    }

    fn workspace(
        &self,
        commands: impl IntoIterator<Item = RegisteredCommand>,
    ) -> ProjectVerificationWorkspace {
        ProjectVerificationWorkspace::new(
            &self.manifest,
            self.boundary.clone(),
            commands,
            BTreeMap::new(),
        )
        .unwrap()
    }

    fn head(&self) -> Vec<u8> {
        let mut bytes = git_ok(&self.repository, &["rev-parse", "HEAD"]).stdout;
        while bytes
            .last()
            .is_some_and(|byte| *byte == b'\n' || *byte == b'\r')
        {
            bytes.pop();
        }
        bytes
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

fn git_output(root: &Path, arguments: &[&str]) -> Output {
    Command::new("git")
        .current_dir(root)
        .args(arguments)
        .env("GIT_OPTIONAL_LOCKS", "0")
        .env("GIT_TERMINAL_PROMPT", "0")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_PAGER", "cat")
        .env("PAGER", "cat")
        .env("LC_ALL", "C")
        .output()
        .unwrap()
}

fn git_ok(root: &Path, arguments: &[&str]) -> Output {
    let output = git_output(root, arguments);
    assert!(
        output.status.success(),
        "git {arguments:?} failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    output
}

#[test]
fn passing_validation_binds_exact_command_revision_dirty_state_and_output() {
    let fixture = Fixture::new("pass");
    let command = fixture.command(
        1,
        "printf 'verified'; printf 'diagnostic' >&2; exit 0",
        CommandTimeout::Unlimited,
    );
    let definition = command.definition_identity();
    let mut workspace = fixture.workspace([command]);
    let record = workspace
        .run(
            CommandId::from_bytes([1; IDENTITY_BYTES]),
            definition,
            ProcessId::from_bytes([11; IDENTITY_BYTES]),
            &CancellationToken::new(),
        )
        .unwrap()
        .clone();

    assert_eq!(record.start_state().revision(), fixture.head().as_slice());
    assert_eq!(record.end_state().revision(), fixture.head().as_slice());
    assert_eq!(record.start_state(), record.end_state());
    assert_eq!(record.program(), "/bin/sh");
    assert_eq!(
        record
            .arguments()
            .iter()
            .map(String::as_str)
            .collect::<Vec<_>>(),
        ["-c", "printf 'verified'; printf 'diagnostic' >&2; exit 0"]
    );
    assert!(matches!(
        record.outcome(),
        VerificationOutcome::Passed { exit_code: Some(0) }
    ));
    assert!(record.output().matches(b"verified", b"diagnostic"));
    assert_eq!(
        workspace.applicability(record.identity()).unwrap(),
        VerificationApplicability::CurrentPass
    );
}

#[test]
fn fail_timeout_and_cancellation_remain_distinct_and_never_satisfy_current_state() {
    let fixture = Fixture::new("terminal-outcomes");
    let failed = fixture.command(2, "printf fail >&2; exit 7", CommandTimeout::Unlimited);
    let timed_out = fixture.command(
        3,
        "exec /bin/sleep 5",
        CommandTimeout::after(Duration::from_millis(30)).unwrap(),
    );
    let cancelled = fixture.command(4, "exec /bin/sleep 5", CommandTimeout::Unlimited);
    let definitions = [
        failed.definition_identity(),
        timed_out.definition_identity(),
        cancelled.definition_identity(),
    ];
    let mut workspace = fixture.workspace([failed, timed_out, cancelled]);

    let failed_record = workspace
        .run(
            CommandId::from_bytes([2; IDENTITY_BYTES]),
            definitions[0],
            ProcessId::from_bytes([12; IDENTITY_BYTES]),
            &CancellationToken::new(),
        )
        .unwrap()
        .clone();
    let timeout_record = workspace
        .run(
            CommandId::from_bytes([3; IDENTITY_BYTES]),
            definitions[1],
            ProcessId::from_bytes([13; IDENTITY_BYTES]),
            &CancellationToken::new(),
        )
        .unwrap()
        .clone();
    let cancellation = CancellationToken::new();
    cancellation.cancel();
    let cancelled_record = workspace
        .run(
            CommandId::from_bytes([4; IDENTITY_BYTES]),
            definitions[2],
            ProcessId::from_bytes([14; IDENTITY_BYTES]),
            &cancellation,
        )
        .unwrap()
        .clone();

    assert!(matches!(
        failed_record.outcome(),
        VerificationOutcome::Failed { exit_code: Some(7) }
    ));
    assert_eq!(timeout_record.outcome(), &VerificationOutcome::TimedOut);
    assert_eq!(cancelled_record.outcome(), &VerificationOutcome::Cancelled);
    for identity in [
        failed_record.identity(),
        timeout_record.identity(),
        cancelled_record.identity(),
    ] {
        assert_eq!(
            workspace.applicability(identity).unwrap(),
            VerificationApplicability::CurrentNonPassing
        );
    }
}

#[test]
fn source_changes_after_a_pass_make_the_historical_record_stale() {
    let fixture = Fixture::new("stale-dirty");
    let command = fixture.command(5, "exit 0", CommandTimeout::Unlimited);
    let definition = command.definition_identity();
    let mut workspace = fixture.workspace([command]);
    let identity = workspace
        .run(
            CommandId::from_bytes([5; IDENTITY_BYTES]),
            definition,
            ProcessId::from_bytes([15; IDENTITY_BYTES]),
            &CancellationToken::new(),
        )
        .unwrap()
        .identity();
    assert_eq!(
        workspace.applicability(identity).unwrap(),
        VerificationApplicability::CurrentPass
    );

    fs::write(
        fixture.repository.join("src/lib.rs"),
        b"pub fn changed_after_validation() {}\n",
    )
    .unwrap();
    assert_eq!(
        workspace.applicability(identity).unwrap(),
        VerificationApplicability::StaleSource
    );
}

#[test]
fn changed_head_rejects_before_the_registered_command_can_spawn() {
    let fixture = Fixture::new("stale-head");
    let marker = fixture.repository.join("must-not-exist");
    let command = fixture.command(
        6,
        "printf spawned > must-not-exist",
        CommandTimeout::Unlimited,
    );
    let definition = command.definition_identity();
    let mut workspace = fixture.workspace([command]);

    fs::write(
        fixture.repository.join("src/lib.rs"),
        b"pub fn new_revision() {}\n",
    )
    .unwrap();
    git_ok(&fixture.repository, &["add", "--", "src/lib.rs"]);
    git_ok(&fixture.repository, &["commit", "-q", "-m", "new revision"]);

    assert!(matches!(
        workspace.run(
            CommandId::from_bytes([6; IDENTITY_BYTES]),
            definition,
            ProcessId::from_bytes([16; IDENTITY_BYTES]),
            &CancellationToken::new(),
        ),
        Err(ProjectVerificationWorkspaceError::StaleCommandRevision { .. })
    ));
    assert!(!marker.exists());
    assert!(workspace.records().is_empty());
}

#[test]
fn append_only_history_round_trips_and_rejects_foreign_project_scope() {
    let fixture = Fixture::new("history");
    let command = fixture.command(7, "exit 0", CommandTimeout::Unlimited);
    let definition = command.definition_identity();
    let mut original = fixture.workspace([command.clone()]);
    let identity = original
        .run(
            CommandId::from_bytes([7; IDENTITY_BYTES]),
            definition,
            ProcessId::from_bytes([17; IDENTITY_BYTES]),
            &CancellationToken::new(),
        )
        .unwrap()
        .identity();
    let state = original.history_state_record().unwrap();

    let mut restored = fixture.workspace([command]);
    restored.restore_history(&state).unwrap();
    assert_eq!(restored.records(), original.records());
    assert_eq!(
        restored.applicability(identity).unwrap(),
        VerificationApplicability::CurrentPass
    );
    assert!(matches!(
        restored.restore_history(&state),
        Err(ProjectVerificationWorkspaceError::HistoryAlreadyInitialized)
    ));

    let foreign = Fixture::new("foreign-history");
    let foreign_command = foreign.command(7, "exit 0", CommandTimeout::Unlimited);
    let mut foreign_workspace = foreign.workspace([foreign_command]);
    assert!(matches!(
        foreign_workspace.restore_history(&state),
        Err(ProjectVerificationWorkspaceError::HistoryScopeMismatch { .. })
    ));
}
