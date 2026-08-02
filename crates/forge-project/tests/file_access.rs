#![cfg(target_os = "linux")]

use forge_core::projects::{AllowedProjectRoot, LanguageProfile, ProjectManifest};
use forge_project::files::{
    FileExpectation, ProjectFileAccess, ProjectFileError, WriteDurability,
};
use forge_project::paths::RepositoryBoundary;
use forge_protocol::hashes::{hash_canonical_bytes, HashDomain};
use forge_protocol::identities::{ProjectId, RepositoryId, IDENTITY_BYTES};
use forge_protocol::paths::RepositoryPathRequest;
use std::ffi::OsString;
use std::fs;
use std::os::unix::ffi::OsStringExt;
use std::os::unix::fs::{symlink, PermissionsExt};
use std::path::{Path, PathBuf};
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
    manifest: ProjectManifest,
    boundary: RepositoryBoundary,
}

impl Fixture {
    fn new(label: &str, allowed_roots: Vec<AllowedProjectRoot>) -> Self {
        let sequence = FIXTURE_COUNTER.fetch_add(1, Ordering::Relaxed);
        let root = std::env::temp_dir().join(format!(
            "forgeos-file-access-{label}-{}-{sequence}",
            std::process::id()
        ));
        let repository = root.join("repository");
        if root.exists() {
            fs::remove_dir_all(&root).expect("remove stale fixture");
        }
        fs::create_dir_all(repository.join("src")).expect("create source directory");
        fs::create_dir_all(repository.join("tests")).expect("create test directory");
        fs::write(repository.join("src/lib.rs"), b"pub fn original() {}\n")
            .expect("write source fixture");
        fs::write(repository.join("tests/outside.rs"), b"outside\n")
            .expect("write denied fixture");

        let repository_id = repository_id((sequence as u8).wrapping_add(10));
        let manifest = ProjectManifest::new(
            project_id((sequence as u8).wrapping_add(80)),
            repository_id,
            "File access fixture",
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
        }
    }

    fn access(&self) -> ProjectFileAccess {
        ProjectFileAccess::new(&self.manifest, &self.boundary).expect("file access")
    }

    fn request(&self, relative: impl AsRef<Path>) -> RepositoryPathRequest {
        RepositoryPathRequest::new(self.manifest.repository_id(), relative).expect("request")
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

#[test]
fn raw_read_preserves_bytes_and_returns_exact_revision() {
    let fixture = Fixture::new(
        "read",
        vec![AllowedProjectRoot::relative("src").unwrap()],
    );
    let bytes = vec![0, 0xff, b'R', b'u', b's', b't', b'\n'];
    fs::write(fixture.repository.join("src/raw.bin"), &bytes).expect("write raw fixture");

    let access = fixture.access();
    let request = fixture.request("src/raw.bin");
    let snapshot = access.read(&request).expect("read raw bytes");

    assert_eq!(snapshot.repository_id(), fixture.manifest.repository_id());
    assert_eq!(snapshot.relative_path().as_path(), Path::new("src/raw.bin"));
    assert_eq!(snapshot.display_path(), fixture.repository.join("src/raw.bin"));
    assert_eq!(snapshot.bytes(), bytes);
    assert_eq!(snapshot.revision().length(), bytes.len() as u64);
    assert_eq!(
        snapshot.revision().content_hash(),
        hash_canonical_bytes(HashDomain::File, &bytes)
    );
}

#[test]
fn atomic_replace_preserves_mode_and_returns_new_revision() {
    let fixture = Fixture::new(
        "replace",
        vec![AllowedProjectRoot::relative("src").unwrap()],
    );
    let target = fixture.repository.join("src/lib.rs");
    fs::set_permissions(&target, fs::Permissions::from_mode(0o640)).expect("set mode");

    let access = fixture.access();
    let request = fixture.request("src/lib.rs");
    let before = access.read(&request).expect("read before");
    let result = access
        .write_atomic(
            &request,
            FileExpectation::Exact(before.revision()),
            b"pub fn replaced() {}\n",
        )
        .expect("atomic replace");

    assert!(!result.created());
    assert!(matches!(
        result.durability(),
        WriteDurability::Confirmed | WriteDurability::ParentSyncUncertain { .. }
    ));
    assert_ne!(result.revision(), before.revision());
    assert_eq!(fs::read(&target).unwrap(), b"pub fn replaced() {}\n");
    assert_eq!(fs::metadata(&target).unwrap().permissions().mode() & 0o7777, 0o640);
    assert_eq!(access.read(&request).unwrap().revision(), result.revision());
}

#[test]
fn missing_expectation_creates_new_file_without_partial_bytes() {
    let fixture = Fixture::new(
        "create",
        vec![AllowedProjectRoot::relative("src").unwrap()],
    );
    let access = fixture.access();
    let request = fixture.request("src/new.rs");

    let result = access
        .write_atomic(&request, FileExpectation::Missing, b"pub fn new_file() {}\n")
        .expect("create file");

    assert!(result.created());
    assert!(matches!(
        result.durability(),
        WriteDurability::Confirmed | WriteDurability::ParentSyncUncertain { .. }
    ));
    assert_eq!(
        fs::read(fixture.repository.join("src/new.rs")).unwrap(),
        b"pub fn new_file() {}\n"
    );
    assert_eq!(access.read(&request).unwrap().revision(), result.revision());
}

#[test]
fn changed_on_disk_conflict_preserves_external_bytes() {
    let fixture = Fixture::new(
        "conflict",
        vec![AllowedProjectRoot::relative("src").unwrap()],
    );
    let target = fixture.repository.join("src/lib.rs");
    let access = fixture.access();
    let request = fixture.request("src/lib.rs");
    let stale = access.read(&request).expect("read stale revision");

    fs::write(&target, b"external change\n").expect("external write");
    let error = access
        .write_atomic(
            &request,
            FileExpectation::Exact(stale.revision()),
            b"forgeos change\n",
        )
        .expect_err("stale revision must conflict");

    assert!(matches!(error, ProjectFileError::Conflict { .. }));
    assert_eq!(fs::read(&target).unwrap(), b"external change\n");
}

#[test]
fn missing_and_existing_expectations_are_not_interchangeable() {
    let fixture = Fixture::new(
        "expectations",
        vec![AllowedProjectRoot::relative("src").unwrap()],
    );
    let access = fixture.access();
    let existing = fixture.request("src/lib.rs");
    let missing = fixture.request("src/missing.rs");
    let existing_revision = access.read(&existing).unwrap().revision();

    assert!(matches!(
        access.write_atomic(&existing, FileExpectation::Missing, b"wrong\n"),
        Err(ProjectFileError::Conflict { .. })
    ));
    assert!(matches!(
        access.write_atomic(
            &missing,
            FileExpectation::Exact(existing_revision),
            b"wrong\n"
        ),
        Err(ProjectFileError::Conflict { .. })
    ));
    assert!(!fixture.repository.join("src/missing.rs").exists());
}

#[test]
fn denied_root_and_wrong_repository_are_rejected_before_io() {
    let fixture = Fixture::new(
        "denied",
        vec![AllowedProjectRoot::relative("src").unwrap()],
    );
    let access = fixture.access();
    let denied = fixture.request("tests/outside.rs");
    assert!(matches!(
        access.read(&denied),
        Err(ProjectFileError::PathNotAllowed { .. })
    ));

    let wrong = RepositoryPathRequest::new(repository_id(250), "src/lib.rs").unwrap();
    assert_eq!(
        access.read(&wrong),
        Err(ProjectFileError::RepositoryMismatch {
            expected: fixture.manifest.repository_id(),
            found: repository_id(250),
        })
    );
}

#[test]
fn symlink_and_directory_targets_are_never_treated_as_files() {
    let fixture = Fixture::new(
        "types",
        vec![AllowedProjectRoot::relative("src").unwrap()],
    );
    let outside = fixture.root.join("outside.txt");
    fs::write(&outside, b"secret\n").expect("write outside fixture");
    symlink(&outside, fixture.repository.join("src/link.rs")).expect("create symlink");
    fs::create_dir_all(fixture.repository.join("src/directory.rs")).expect("create directory");

    let access = fixture.access();
    assert!(matches!(
        access.read(&fixture.request("src/link.rs")),
        Err(ProjectFileError::SymlinkRejected { .. })
            | Err(ProjectFileError::Boundary(_))
    ));
    assert!(matches!(
        access.read(&fixture.request("src/directory.rs")),
        Err(ProjectFileError::NotRegularFile { .. })
    ));
    assert_eq!(fs::read(&outside).unwrap(), b"secret\n");
}

#[test]
fn non_utf8_filename_round_trips_without_lossy_conversion() {
    let fixture = Fixture::new(
        "non-utf8",
        vec![AllowedProjectRoot::relative("src").unwrap()],
    );
    let leaf = OsString::from_vec(vec![b'n', 0xff, b'.', b'r', b's']);
    let relative = PathBuf::from("src").join(&leaf);
    fs::write(fixture.repository.join(&relative), b"first\n").expect("write non-UTF8 file");

    let access = fixture.access();
    let request = fixture.request(&relative);
    let before = access.read(&request).expect("read non-UTF8 file");
    let result = access
        .write_atomic(
            &request,
            FileExpectation::Exact(before.revision()),
            b"second\n",
        )
        .expect("write non-UTF8 file");

    assert_eq!(fs::read(fixture.repository.join(&relative)).unwrap(), b"second\n");
    assert_eq!(access.read(&request).unwrap().revision(), result.revision());
}

#[test]
fn repository_root_permission_allows_top_level_files() {
    let fixture = Fixture::new("root", vec![AllowedProjectRoot::repository_root()]);
    fs::write(fixture.repository.join("Cargo.toml"), b"[package]\n").expect("write top level");
    let access = fixture.access();
    let request = fixture.request("Cargo.toml");
    assert_eq!(access.read(&request).unwrap().bytes(), b"[package]\n");
}
