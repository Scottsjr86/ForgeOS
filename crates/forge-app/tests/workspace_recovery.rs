#![cfg(target_os = "linux")]

use forge_app::composition::editor_workspace::EditorWorkspace;
use forge_app::composition::nyx_service::{ManagedNyxService, NyxServiceConfig};
use forge_app::composition::terminal_workspace::{
    ProjectTerminalLaunch, ProjectTerminalWorkspace, TerminalWorkingDirectory,
};
use forge_app::composition::workspace_recovery::WorkspaceRecoveryCoordinator;
use forge_bridge::processes::ProcessExecutionContext;
use forge_core::projects::{AllowedProjectRoot, LanguageProfile, ProjectManifest};
use forge_core::recovery::{InterruptedAction, InterruptedEffectState, RecoveryActionKind};
use forge_core::workspace_recovery::{RecoveredServiceState, RecoveredTerminalState};
use forge_editor::buffers::{BufferId, SynchronizationState};
use forge_nyx_client::protocol::NyxProtocolVersion;
use forge_nyx_client::transport::{NyxClientConfig, NyxTransportEndpoint};
use forge_project::files::ProjectFileAccess;
use forge_project::paths::RepositoryBoundary;
use forge_project::recovery_store::WorkspaceRecoveryStore;
use forge_protocol::hashes::{HashDomain, hash_canonical_bytes};
use forge_protocol::identities::{IDENTITY_BYTES, ProjectId, RepositoryId, SessionId, TerminalId};
use forge_protocol::paths::RepositoryPathRequest;
use forge_session::services::StartupRestartPolicy;
use forge_terminal::pty::PtyDimensions;
use std::ffi::OsString;
use std::fs;
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_FIXTURE: AtomicU64 = AtomicU64::new(1);

struct Fixture {
    root: PathBuf,
    repository: PathBuf,
    manifest: ProjectManifest,
    boundary: RepositoryBoundary,
    session_id: SessionId,
}

impl Fixture {
    fn new(label: &str) -> Self {
        let sequence = NEXT_FIXTURE.fetch_add(1, Ordering::Relaxed);
        let root = std::env::temp_dir().join(format!(
            "forgeos-workspace-recovery-{label}-{}-{sequence}",
            std::process::id()
        ));
        let repository = root.join("repository");
        fs::create_dir_all(repository.join("src")).unwrap();
        fs::write(repository.join("src/lib.rs"), b"pub fn disk() {}\n").unwrap();
        let repository = fs::canonicalize(repository).unwrap();
        let project_id = ProjectId::from_bytes([(sequence as u8).wrapping_add(10); IDENTITY_BYTES]);
        let repository_id =
            RepositoryId::from_bytes([(sequence as u8).wrapping_add(80); IDENTITY_BYTES]);
        let manifest = ProjectManifest::new(
            project_id,
            repository_id,
            "Recovery fixture",
            vec![AllowedProjectRoot::RepositoryRoot],
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
            session_id: SessionId::from_bytes([(sequence as u8).wrapping_add(40); 16]),
        }
    }

    fn editor(&self) -> EditorWorkspace {
        EditorWorkspace::new(ProjectFileAccess::new(&self.manifest, &self.boundary).unwrap())
    }

    fn terminals(&self) -> ProjectTerminalWorkspace {
        ProjectTerminalWorkspace::new(&self.manifest, self.boundary.clone()).unwrap()
    }

    fn nyx(&self) -> ManagedNyxService {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        drop(listener);
        let client = NyxClientConfig::new(
            NyxTransportEndpoint::tcp(address),
            [NyxProtocolVersion::new(1, 0)],
        )
        .unwrap();
        let config = NyxServiceConfig::new(
            "python3",
            ["-c", "raise SystemExit(0)"],
            ProcessExecutionContext::new(&self.root),
            client,
            StartupRestartPolicy::Never,
        )
        .unwrap();
        ManagedNyxService::new(self.session_id, config)
    }

    fn recovery_store(&self) -> WorkspaceRecoveryStore {
        WorkspaceRecoveryStore::new(self.root.join("workspace.recovery")).unwrap()
    }

    fn coordinator(&self) -> WorkspaceRecoveryCoordinator {
        WorkspaceRecoveryCoordinator::new(
            self.manifest.project_id(),
            self.manifest.repository_id(),
            self.session_id,
            self.recovery_store(),
        )
    }

    fn request(&self, path: impl AsRef<Path>) -> RepositoryPathRequest {
        RepositoryPathRequest::new(self.manifest.repository_id(), path).unwrap()
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

#[test]
fn dirty_buffers_terminal_metadata_and_service_state_restore_without_replay() {
    let fixture = Fixture::new("restore");
    let mut editor = fixture.editor();
    let buffer_id = BufferId::from_bytes([3; 16]);
    editor
        .open_existing(buffer_id, &fixture.request("src/lib.rs"))
        .unwrap();
    editor
        .buffer_mut(buffer_id)
        .unwrap()
        .replace_range(7..11, b"local")
        .unwrap();

    let mut terminals = fixture.terminals();
    let terminal_id = TerminalId::from_bytes([4; 16]);
    let handle = terminals
        .spawn(ProjectTerminalLaunch::new(
            terminal_id,
            "python3",
            vec![
                OsString::from("-c"),
                OsString::from("import time; time.sleep(60)"),
            ],
            TerminalWorkingDirectory::relative("src").unwrap(),
            PtyDimensions::new(24, 80).unwrap(),
        ))
        .unwrap();
    let nyx = fixture.nyx();
    let interrupted = InterruptedAction::new(
        hash_canonical_bytes(HashDomain::ToolRequest, b"unfinished-command"),
        RecoveryActionKind::Command,
        InterruptedEffectState::CommitUnknown,
    );
    let coordinator = fixture.coordinator();
    let record = coordinator
        .capture(&editor, &terminals, &nyx, vec![interrupted.clone()])
        .unwrap();
    assert_eq!(record.generation(), 1);
    terminals.terminate(handle).unwrap();

    let mut reopened = fixture.editor();
    let restored = coordinator.restore(&mut reopened).unwrap();
    assert_eq!(restored.project_id(), fixture.manifest.project_id());
    assert_eq!(restored.buffer_states().len(), 1);
    assert_eq!(
        reopened.buffer(buffer_id).unwrap().bytes(),
        b"pub fn local() {}\n"
    );
    assert!(matches!(
        reopened.buffer(buffer_id).unwrap().synchronization(),
        SynchronizationState::Dirty { .. }
    ));
    assert_eq!(restored.terminals().len(), 1);
    assert_eq!(
        restored.terminals()[0].state(),
        RecoveredTerminalState::RequiresRestart
    );
    assert!(!restored.terminals()[0].claims_alive());
    assert_eq!(
        restored.services()[0].state(),
        RecoveredServiceState::Stopped
    );
    assert!(!restored.services()[0].claims_alive());
    assert_eq!(restored.interrupted_actions(), &[interrupted]);
    assert!(!restored.interrupted_actions()[0].replay_allowed());
    assert!(restored.requires_operator_attention());
}

#[test]
fn changed_disk_restores_local_bytes_as_conflict_and_never_overwrites_disk() {
    let fixture = Fixture::new("conflict");
    let mut editor = fixture.editor();
    let buffer_id = BufferId::from_bytes([5; 16]);
    editor
        .open_existing(buffer_id, &fixture.request("src/lib.rs"))
        .unwrap();
    editor
        .buffer_mut(buffer_id)
        .unwrap()
        .replace_range(7..11, b"local")
        .unwrap();
    let terminals = fixture.terminals();
    let nyx = fixture.nyx();
    let coordinator = fixture.coordinator();
    coordinator
        .capture(&editor, &terminals, &nyx, Vec::new())
        .unwrap();

    fs::write(
        fixture.repository.join("src/lib.rs"),
        b"pub fn external() {}\n",
    )
    .unwrap();
    let mut reopened = fixture.editor();
    let restored = coordinator.restore(&mut reopened).unwrap();
    assert!(matches!(
        restored.buffer_states()[0].1,
        SynchronizationState::Conflict { .. }
    ));
    assert_eq!(
        reopened.buffer(buffer_id).unwrap().bytes(),
        b"pub fn local() {}\n"
    );
    assert_eq!(
        fs::read(fixture.repository.join("src/lib.rs")).unwrap(),
        b"pub fn external() {}\n"
    );
}

#[test]
fn sequential_capture_is_generation_guarded_and_restore_requires_empty_editor() {
    let fixture = Fixture::new("generation");
    let editor = fixture.editor();
    let terminals = fixture.terminals();
    let nyx = fixture.nyx();
    let coordinator = fixture.coordinator();
    assert_eq!(
        coordinator
            .capture(&editor, &terminals, &nyx, Vec::new())
            .unwrap()
            .generation(),
        1
    );
    assert_eq!(
        coordinator
            .capture(&editor, &terminals, &nyx, Vec::new())
            .unwrap()
            .generation(),
        2
    );

    let mut nonempty = fixture.editor();
    nonempty
        .open_existing(
            BufferId::from_bytes([8; 16]),
            &fixture.request("src/lib.rs"),
        )
        .unwrap();
    assert!(coordinator.restore(&mut nonempty).is_err());
}

#[test]
fn interrupted_publication_and_invalid_current_require_explicit_safe_choices() {
    let fixture = Fixture::new("choices");
    let editor = fixture.editor();
    let terminals = fixture.terminals();
    let nyx = fixture.nyx();
    let coordinator = fixture.coordinator();
    let first = coordinator
        .capture(&editor, &terminals, &nyx, Vec::new())
        .unwrap();
    let second = coordinator
        .capture(&editor, &terminals, &nyx, Vec::new())
        .unwrap();
    assert_eq!(first.generation(), 1);
    assert_eq!(second.generation(), 2);

    let store = fixture.recovery_store();
    fs::copy(store.target_path(), store.staged_path()).unwrap();
    assert!(coordinator.assess().unwrap().interrupted_write_present());
    assert!(coordinator.discard_interrupted_write().unwrap());
    assert!(!coordinator.assess().unwrap().interrupted_write_present());

    fs::write(store.target_path(), b"invalid-current-recovery-image").unwrap();
    let restored = coordinator.restore_previous_if_current_unusable().unwrap();
    assert_eq!(restored.generation(), 1);
    assert!(matches!(
        coordinator.assess().unwrap().current(),
        forge_project::recovery_store::RecoveryImageStatus::Valid { generation: 1, .. }
    ));
}
