#![cfg(target_os = "linux")]

use forge_app::composition::editor_workspace::{EditorWorkspace, EditorWorkspaceError};
use forge_core::projects::{AllowedProjectRoot, LanguageProfile, ProjectManifest};
use forge_editor::buffers::{
    BufferError, BufferId, CloseDisposition, OpenBufferResult, SynchronizationState,
};
use forge_project::files::ProjectFileAccess;
use forge_project::paths::RepositoryBoundary;
use forge_protocol::identities::{ProjectId, RepositoryId, IDENTITY_BYTES};
use forge_protocol::paths::RepositoryPathRequest;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_FIXTURE: AtomicU64 = AtomicU64::new(1);

fn buffer_id(byte: u8) -> BufferId {
    BufferId::from_bytes([byte; 16])
}

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
            "forgeos-editor-workspace-{label}-{}-{sequence}",
            std::process::id()
        ));
        let repository = root.join("repository");
        if root.exists() {
            fs::remove_dir_all(&root).expect("remove stale editor fixture");
        }
        fs::create_dir_all(repository.join("src")).expect("create source directory");
        fs::write(repository.join("src/alpha.rs"), b"pub fn alpha() {}\n")
            .expect("write alpha fixture");
        fs::write(repository.join("src/beta.rs"), b"pub fn beta() {}\n")
            .expect("write beta fixture");

        let project_id = ProjectId::from_bytes([(sequence as u8).wrapping_add(10); IDENTITY_BYTES]);
        let repository_id =
            RepositoryId::from_bytes([(sequence as u8).wrapping_add(80); IDENTITY_BYTES]);
        let manifest = ProjectManifest::new(
            project_id,
            repository_id,
            "Editor workspace fixture",
            vec![AllowedProjectRoot::relative("src").unwrap()],
            Vec::new(),
            LanguageProfile::Rust,
            Vec::new(),
        )
        .expect("valid project manifest");
        let boundary = RepositoryBoundary::open(repository_id, &repository).expect("boundary");
        Self {
            root,
            repository,
            manifest,
            boundary,
        }
    }

    fn workspace(&self) -> EditorWorkspace {
        let files = ProjectFileAccess::new(&self.manifest, &self.boundary).expect("file access");
        EditorWorkspace::new(files)
    }

    fn request(&self, path: impl AsRef<Path>) -> RepositoryPathRequest {
        RepositoryPathRequest::new(self.manifest.repository_id(), path).expect("valid request")
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

#[test]
fn multiple_buffers_save_independently_without_touching_unedited_files() {
    let fixture = Fixture::new("multiple");
    let mut workspace = fixture.workspace();
    let alpha = buffer_id(1);
    let beta = buffer_id(2);

    assert_eq!(
        workspace
            .open_existing(alpha, &fixture.request("src/alpha.rs"))
            .expect("open alpha"),
        OpenBufferResult::Opened(alpha)
    );
    assert_eq!(
        workspace
            .open_existing(beta, &fixture.request("src/beta.rs"))
            .expect("open beta"),
        OpenBufferResult::Opened(beta)
    );

    workspace
        .buffer_mut(alpha)
        .expect("alpha buffer")
        .replace_range(7..12, b"alpha_saved")
        .expect("edit alpha");
    workspace
        .buffer_mut(beta)
        .expect("beta buffer")
        .replace_range(7..11, b"beta_dirty")
        .expect("edit beta");

    workspace.save(alpha).expect("save alpha");
    assert_eq!(
        fs::read(fixture.repository.join("src/alpha.rs")).unwrap(),
        b"pub fn alpha_saved() {}\n"
    );
    assert_eq!(
        fs::read(fixture.repository.join("src/beta.rs")).unwrap(),
        b"pub fn beta() {}\n"
    );
    assert!(matches!(
        workspace.buffer(alpha).unwrap().synchronization(),
        SynchronizationState::Clean { .. }
    ));
    assert!(workspace.buffer(beta).unwrap().synchronization().is_dirty());

    workspace.save(beta).expect("save beta");
    assert_eq!(
        fs::read(fixture.repository.join("src/beta.rs")).unwrap(),
        b"pub fn beta_dirty() {}\n"
    );
}

#[test]
fn external_change_conflicts_and_explicit_reopen_loads_external_bytes() {
    let fixture = Fixture::new("conflict");
    let mut workspace = fixture.workspace();
    let id = buffer_id(3);
    workspace
        .open_existing(id, &fixture.request("src/alpha.rs"))
        .expect("open alpha");
    workspace
        .buffer_mut(id)
        .unwrap()
        .replace_range(7..12, b"local")
        .expect("local edit");

    fs::write(
        fixture.repository.join("src/alpha.rs"),
        b"pub fn external() {}\n",
    )
    .expect("external edit");
    let error = workspace.save(id).expect_err("stale save conflicts");
    assert!(matches!(
        error,
        EditorWorkspaceError::SaveConflict { buffer_id, .. } if buffer_id == id
    ));
    assert_eq!(
        fs::read(fixture.repository.join("src/alpha.rs")).unwrap(),
        b"pub fn external() {}\n"
    );
    assert!(matches!(
        workspace.buffer(id).unwrap().synchronization(),
        SynchronizationState::Conflict { .. }
    ));

    let confirmation = workspace.buffer(id).unwrap().discard_confirmation();
    assert_eq!(
        workspace
            .discard_and_reopen(confirmation)
            .expect("explicit discard and reopen"),
        OpenBufferResult::Opened(id)
    );
    assert_eq!(
        workspace.buffer(id).unwrap().bytes(),
        b"pub fn external() {}\n"
    );
    assert!(matches!(
        workspace.buffer(id).unwrap().synchronization(),
        SynchronizationState::Clean { .. }
    ));
}

#[test]
fn refresh_detects_external_change_before_save() {
    let fixture = Fixture::new("refresh");
    let mut workspace = fixture.workspace();
    let id = buffer_id(4);
    workspace
        .open_existing(id, &fixture.request("src/alpha.rs"))
        .expect("open alpha");
    workspace
        .buffer_mut(id)
        .unwrap()
        .replace_range(7..12, b"local")
        .expect("local edit");
    fs::write(fixture.repository.join("src/alpha.rs"), b"changed\n").expect("external change");

    assert!(matches!(
        workspace.refresh(id).expect("refresh"),
        SynchronizationState::Conflict { .. }
    ));
    assert!(matches!(
        workspace.save(id),
        Err(EditorWorkspaceError::Buffer(
            BufferError::ConflictUnresolved
        ))
    ));
    assert_eq!(
        fs::read(fixture.repository.join("src/alpha.rs")).unwrap(),
        b"changed\n"
    );
}

#[test]
fn stale_discard_confirmation_cannot_remove_newer_edits() {
    let fixture = Fixture::new("discard");
    let mut workspace = fixture.workspace();
    let id = buffer_id(5);
    workspace
        .open_existing(id, &fixture.request("src/alpha.rs"))
        .expect("open alpha");
    workspace
        .buffer_mut(id)
        .unwrap()
        .replace_range(7..12, b"first")
        .expect("first edit");
    let stale = workspace.buffer(id).unwrap().discard_confirmation();
    workspace
        .buffer_mut(id)
        .unwrap()
        .replace_range(7..12, b"second")
        .expect("second edit");

    assert!(matches!(
        workspace.discard_and_close(stale),
        Err(EditorWorkspaceError::Buffer(
            BufferError::StaleDiscardConfirmation { .. }
        ))
    ));
    assert_eq!(
        workspace.buffer(id).unwrap().bytes(),
        b"pub fn second() {}\n"
    );
    assert_eq!(
        workspace.buffer(id).unwrap().close_disposition(),
        CloseDisposition::ConfirmationRequired
    );

    let fresh = workspace.buffer(id).unwrap().discard_confirmation();
    workspace
        .discard_and_close(fresh)
        .expect("fresh confirmation closes");
    assert!(workspace.buffer(id).is_none());
    assert_eq!(
        fs::read(fixture.repository.join("src/alpha.rs")).unwrap(),
        b"pub fn alpha() {}\n"
    );
}

#[test]
fn missing_file_buffer_is_created_atomically_and_reopens_clean() {
    let fixture = Fixture::new("create");
    let mut workspace = fixture.workspace();
    let id = buffer_id(6);
    let request = fixture.request("src/new.rs");
    assert_eq!(
        workspace
            .open_or_create(id, &request)
            .expect("open missing file buffer"),
        OpenBufferResult::Opened(id)
    );
    workspace
        .buffer_mut(id)
        .unwrap()
        .replace_range(0..0, b"pub fn created() {}\n")
        .expect("edit new file");
    workspace.save(id).expect("save new file");
    assert_eq!(
        fs::read(fixture.repository.join("src/new.rs")).unwrap(),
        b"pub fn created() {}\n"
    );
    assert!(matches!(
        workspace.buffer(id).unwrap().synchronization(),
        SynchronizationState::Clean { .. }
    ));

    workspace.close_clean(id).expect("close clean buffer");
    assert_eq!(
        workspace
            .open_existing(id, &request)
            .expect("reopen created file"),
        OpenBufferResult::Opened(id)
    );
    assert_eq!(
        workspace.buffer(id).unwrap().bytes(),
        b"pub fn created() {}\n"
    );
}
