use super::*;
use forge_core::projects::LanguageProfile;
use forge_protocol::identities::{IDENTITY_BYTES, ProjectId, RepositoryId};
use std::sync::atomic::{AtomicU64, Ordering};

static FIXTURE_COUNTER: AtomicU64 = AtomicU64::new(0);

fn project_id(byte: u8) -> ProjectId {
    ProjectId::from_bytes([byte; IDENTITY_BYTES])
}

fn repository_id(byte: u8) -> RepositoryId {
    RepositoryId::from_bytes([byte; IDENTITY_BYTES])
}

struct Fixture {
    root: PathBuf,
    repository: PathBuf,
    access: ProjectFileAccess,
}

impl Fixture {
    fn new(label: &str) -> Self {
        let sequence = FIXTURE_COUNTER.fetch_add(1, Ordering::Relaxed);
        let root = std::env::temp_dir().join(format!(
            "forgeos-file-unit-{label}-{}-{sequence}",
            std::process::id()
        ));
        let repository = root.join("repository");
        fs::create_dir_all(repository.join("src")).expect("create fixture");
        fs::write(repository.join("src/lib.rs"), b"original\n").expect("write source");
        let project_id = project_id(1);
        let repository_id = repository_id(2);
        let manifest = ProjectManifest::new(
            project_id,
            repository_id,
            "fixture",
            vec![AllowedProjectRoot::relative("src").unwrap()],
            Vec::new(),
            LanguageProfile::Rust,
            Vec::new(),
        )
        .expect("manifest");
        let boundary = RepositoryBoundary::open(repository_id, &repository).expect("boundary");
        let access = ProjectFileAccess::new(&manifest, &boundary).expect("file access");
        Self {
            root,
            repository,
            access,
        }
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

#[test]
fn injected_failures_preserve_original_bytes_and_remove_stage() {
    let fixture = Fixture::new("injected");
    let request = RepositoryPathRequest::new(fixture.access.repository_id(), "src/lib.rs")
        .expect("request");
    let original = fixture.access.read(&request).expect("read original");

    for fault in [WriteFault::BeforeConflictRecheck, WriteFault::BeforeReplace] {
        assert_eq!(
            fixture.access.write_atomic_inner(
                &request,
                FileExpectation::Exact(original.revision()),
                b"replacement\n",
                fault,
            ),
            Err(ProjectFileError::InjectedFailure)
        );
        assert_eq!(
            fs::read(fixture.repository.join("src/lib.rs")).unwrap(),
            b"original\n"
        );
        let staged: Vec<_> = fs::read_dir(fixture.repository.join("src"))
            .unwrap()
            .filter_map(Result::ok)
            .filter(|entry| entry.file_name().to_string_lossy().contains(".forgeos-write-"))
            .collect();
        assert!(staged.is_empty());
    }
}
