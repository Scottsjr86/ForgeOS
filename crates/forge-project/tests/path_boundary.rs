#![cfg(unix)]

use forge_project::paths::{RepositoryBoundary, RepositoryBoundaryError};
use forge_protocol::identities::{RepositoryId, IDENTITY_BYTES};
use forge_protocol::paths::{RepositoryPathError, RepositoryPathRequest};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

static FIXTURE_COUNTER: AtomicU64 = AtomicU64::new(0);

fn repository_id(byte: u8) -> RepositoryId {
    RepositoryId::from_bytes([byte; IDENTITY_BYTES])
}

struct Fixture {
    root: PathBuf,
}

impl Fixture {
    fn new(label: &str) -> Self {
        let sequence = FIXTURE_COUNTER.fetch_add(1, Ordering::Relaxed);
        let root = std::env::temp_dir().join(format!(
            "forgeos-path-{label}-{}-{sequence}",
            std::process::id()
        ));
        if root.exists() {
            fs::remove_dir_all(&root).expect("remove stale fixture");
        }
        fs::create_dir_all(&root).expect("create fixture root");
        Self { root }
    }

    fn repository(&self) -> PathBuf {
        self.root.join("repository")
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

#[test]
fn normal_child_keeps_display_and_canonical_paths_separate() {
    let fixture = Fixture::new("normal");
    let repository = fixture.repository();
    fs::create_dir_all(repository.join("src")).expect("create source directory");
    fs::write(repository.join("src/lib.rs"), b"pub fn ready() {}\n").expect("write source");

    let display_root = repository.join("..").join("repository");
    let boundary = RepositoryBoundary::open(repository_id(1), &display_root).expect("open root");
    let request = RepositoryPathRequest::new(repository_id(1), "src/lib.rs").expect("request");
    let resolved = boundary.resolve_existing(&request).expect("resolve child");

    assert_eq!(boundary.display_root(), display_root);
    assert_eq!(boundary.canonical_root(), fs::canonicalize(&repository).unwrap());
    assert_eq!(resolved.repository_id(), repository_id(1));
    assert_eq!(resolved.relative_path().as_path(), Path::new("src/lib.rs"));
    assert_eq!(resolved.display_path(), display_root.join("src/lib.rs"));
    assert_eq!(
        resolved.canonical_path(),
        fs::canonicalize(repository.join("src/lib.rs")).unwrap()
    );
}

#[test]
fn moved_repository_rebinds_only_when_the_directory_object_is_unchanged() {
    let fixture = Fixture::new("moved");
    let repository = fixture.repository();
    fs::create_dir_all(repository.join("src")).expect("create repository");
    fs::write(repository.join("src/main.rs"), b"fn main() {}\n").expect("write source");

    let original = RepositoryBoundary::open(repository_id(2), &repository).expect("open root");
    let original_object = original.root_object();
    let moved = fixture.root.join("moved-repository");
    fs::rename(&repository, &moved).expect("move repository");

    let relocated = original
        .relocate(&moved)
        .expect("same directory object relocates");
    assert_eq!(relocated.repository_id(), repository_id(2));
    assert_eq!(relocated.root_object(), original_object);
    assert_eq!(relocated.display_root(), moved);
    relocated.revalidate().expect("moved boundary remains valid");

    let request = RepositoryPathRequest::new(repository_id(2), "src/main.rs").unwrap();
    assert!(relocated.resolve_existing(&request).is_ok());
}

#[test]
fn traversal_absolute_alias_and_wrong_repository_requests_are_denied() {
    let fixture = Fixture::new("denied");
    let repository = fixture.repository();
    fs::create_dir_all(&repository).expect("create repository");
    fs::write(repository.join("owned.txt"), b"owned").expect("write owned file");
    let boundary = RepositoryBoundary::open(repository_id(3), &repository).expect("open root");

    assert_eq!(
        RepositoryPathRequest::new(repository_id(3), "../outside"),
        Err(RepositoryPathError::ParentTraversal)
    );
    assert_eq!(
        RepositoryPathRequest::new(repository_id(3), repository.join("owned.txt")),
        Err(RepositoryPathError::Absolute)
    );
    assert_eq!(
        RepositoryPathRequest::new(repository_id(3), "./owned.txt"),
        Err(RepositoryPathError::NonCanonical)
    );

    let wrong = RepositoryPathRequest::new(repository_id(4), "owned.txt").unwrap();
    assert_eq!(
        boundary.resolve_existing(&wrong),
        Err(RepositoryBoundaryError::RepositoryMismatch {
            expected: repository_id(3),
            found: repository_id(4),
        })
    );
}

#[test]
fn root_and_child_symlinks_are_rejected_before_following_them() {
    use std::os::unix::fs::symlink;

    let fixture = Fixture::new("symlink");
    let repository = fixture.repository();
    let outside = fixture.root.join("outside");
    fs::create_dir_all(&repository).expect("create repository");
    fs::create_dir_all(&outside).expect("create outside directory");
    fs::write(outside.join("secret.txt"), b"secret").expect("write outside file");

    let root_link = fixture.root.join("repository-link");
    symlink(&repository, &root_link).expect("create root symlink");
    assert!(matches!(
        RepositoryBoundary::open(repository_id(5), &root_link),
        Err(RepositoryBoundaryError::RootSymlink { .. })
    ));

    let boundary = RepositoryBoundary::open(repository_id(5), &repository).expect("open root");
    symlink(&outside, repository.join("escape")).expect("create child symlink");
    let request = RepositoryPathRequest::new(repository_id(5), "escape/secret.txt").unwrap();
    assert!(matches!(
        boundary.resolve_existing(&request),
        Err(RepositoryBoundaryError::SymlinkRejected { .. })
    ));
}

#[test]
fn root_replacement_is_detected_and_cannot_inherit_repository_identity() {
    let fixture = Fixture::new("replacement");
    let repository = fixture.repository();
    fs::create_dir_all(&repository).expect("create repository");
    let boundary = RepositoryBoundary::open(repository_id(6), &repository).expect("open root");

    let displaced = fixture.root.join("displaced");
    fs::rename(&repository, &displaced).expect("displace original root");
    fs::create_dir_all(&repository).expect("create replacement root");

    assert!(matches!(
        boundary.revalidate(),
        Err(RepositoryBoundaryError::RootIdentityChanged { .. })
    ));
    assert!(matches!(
        boundary.relocate(&repository),
        Err(RepositoryBoundaryError::RootIdentityChanged { .. })
    ));
    assert!(boundary.relocate(&displaced).is_ok());
}

#[test]
fn missing_and_non_directory_components_return_typed_failures() {
    let fixture = Fixture::new("missing");
    let repository = fixture.repository();
    fs::create_dir_all(&repository).expect("create repository");
    fs::write(repository.join("file"), b"not a directory").expect("write file");
    let boundary = RepositoryBoundary::open(repository_id(7), &repository).expect("open root");

    let missing = RepositoryPathRequest::new(repository_id(7), "missing.txt").unwrap();
    assert!(matches!(
        boundary.resolve_existing(&missing),
        Err(RepositoryBoundaryError::Io {
            kind: std::io::ErrorKind::NotFound,
            ..
        })
    ));

    let nested = RepositoryPathRequest::new(repository_id(7), "file/child").unwrap();
    assert!(matches!(
        boundary.resolve_existing(&nested),
        Err(RepositoryBoundaryError::IntermediateNotDirectory { .. })
    ));
}

#[test]
fn non_utf8_child_names_resolve_without_lossy_text_conversion() {
    use std::ffi::OsString;
    use std::os::unix::ffi::OsStringExt;

    let fixture = Fixture::new("non-utf8");
    let repository = fixture.repository();
    fs::create_dir_all(&repository).expect("create repository");
    let relative = PathBuf::from(OsString::from_vec(vec![b'f', 0xff, b'.', b'r', b's']));
    fs::write(repository.join(&relative), b"bytes").expect("write non-UTF8 child");

    let boundary = RepositoryBoundary::open(repository_id(8), &repository).expect("open root");
    let request = RepositoryPathRequest::new(repository_id(8), &relative).expect("request");
    let resolved = boundary
        .resolve_existing(&request)
        .expect("resolve non-UTF8 child");
    assert_eq!(resolved.relative_path().as_path(), relative);
}
