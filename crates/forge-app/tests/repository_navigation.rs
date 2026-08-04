use forge_core::projects::{AllowedProjectRoot, LanguageProfile, ProjectManifest};
use forge_editor::buffers::{BufferId, BufferRegistry, DiskVersion, DocumentKey, OpenBufferResult};
use forge_project::paths::RepositoryBoundary;
use forge_project::repository_view::RepositoryBrowser;
use forge_project::text_search::TextSearchQuery;
use forge_protocol::identities::{ProjectId, RepositoryId, IDENTITY_BYTES};
use forge_protocol::paths::RepositoryPathRequest;
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_FIXTURE: AtomicU64 = AtomicU64::new(1);

struct Fixture(PathBuf);

impl Fixture {
    fn new() -> Self {
        let sequence = NEXT_FIXTURE.fetch_add(1, Ordering::Relaxed);
        let root = std::env::temp_dir().join(format!(
            "forgeos-repository-navigation-{}-{sequence}",
            std::process::id()
        ));
        if root.exists() {
            fs::remove_dir_all(&root).expect("remove stale navigation fixture");
        }
        fs::create_dir_all(root.join("src")).expect("create source directory");
        fs::write(root.join("src/lib.rs"), b"pub fn navigation_target() {}\n")
            .expect("write navigation target");
        Self(root)
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn search_result_opens_the_exact_editor_document() {
    let fixture = Fixture::new();
    let project_id = ProjectId::from_bytes([41; IDENTITY_BYTES]);
    let repository_id = RepositoryId::from_bytes([42; IDENTITY_BYTES]);
    let manifest = ProjectManifest::new(
        project_id,
        repository_id,
        "Navigation fixture",
        vec![AllowedProjectRoot::relative("src").unwrap()],
        Vec::new(),
        LanguageProfile::Rust,
        Vec::new(),
    )
    .expect("valid manifest");
    let boundary = RepositoryBoundary::open(repository_id, &fixture.0).expect("boundary");
    let browser = RepositoryBrowser::new(&manifest, &boundary).expect("browser");
    let search = browser
        .search_text(&TextSearchQuery::new("navigation_target").unwrap())
        .expect("search");
    let hit = search.matches().first().expect("known search hit");
    let request = RepositoryPathRequest::new(repository_id, hit.relative_path().as_path())
        .expect("search path remains valid");
    let snapshot = browser.open_file(&request).expect("open search result");

    let document = DocumentKey::new(repository_id, snapshot.relative_path().clone());
    let disk_version = DiskVersion::for_bytes(snapshot.bytes());
    let file_bytes = snapshot.into_bytes();
    let buffer_id = BufferId::from_bytes([7; 16]);
    let mut buffers = BufferRegistry::new();
    assert_eq!(
        buffers
            .open_existing(buffer_id, document.clone(), disk_version, file_bytes,)
            .expect("open editor buffer"),
        OpenBufferResult::Opened(buffer_id)
    );
    assert_eq!(buffers.buffer_for_document(&document), Some(buffer_id));
    assert_eq!(
        buffers.get(buffer_id).expect("buffer exists").bytes(),
        b"pub fn navigation_target() {}\n"
    );
}
