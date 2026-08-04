use forge_core::projects::{AllowedProjectRoot, LanguageProfile, ProjectManifest};
use forge_project::files::ProjectFileError;
use forge_project::paths::RepositoryBoundary;
use forge_project::repository_view::{
    RepositoryBrowseError, RepositoryBrowseScope, RepositoryBrowser, RepositoryEntryKind,
    RepositoryScanIssueKind,
};
use forge_project::text_search::{RepositorySearchIssue, TextSearchQuery, TextSearchQueryError};
use forge_protocol::identities::{ProjectId, RepositoryId, IDENTITY_BYTES};
use forge_protocol::paths::{RepositoryPathError, RepositoryPathRequest};
use std::ffi::OsString;
use std::fs::{self, OpenOptions};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

#[cfg(unix)]
use std::os::unix::ffi::{OsStrExt, OsStringExt};
#[cfg(unix)]
use std::os::unix::fs::symlink;

static NEXT_FIXTURE: AtomicU64 = AtomicU64::new(1);

fn project_id(seed: u8) -> ProjectId {
    ProjectId::from_bytes([seed; IDENTITY_BYTES])
}

fn repository_id(seed: u8) -> RepositoryId {
    RepositoryId::from_bytes([seed; IDENTITY_BYTES])
}

struct Fixture {
    root: PathBuf,
    repository: PathBuf,
    manifest: ProjectManifest,
    boundary: RepositoryBoundary,
}

impl Fixture {
    fn new(label: &str, allowed_roots: Vec<AllowedProjectRoot>) -> Self {
        let sequence = NEXT_FIXTURE.fetch_add(1, Ordering::Relaxed);
        let root = std::env::temp_dir().join(format!(
            "forgeos-repository-browser-{}-{sequence}-{label}",
            std::process::id()
        ));
        let repository = root.join("repository");
        if root.exists() {
            fs::remove_dir_all(&root).expect("remove stale browser fixture");
        }
        fs::create_dir_all(repository.join("src/nested")).expect("create source tree");
        fs::create_dir_all(repository.join("tests")).expect("create denied tree");
        fs::write(
            repository.join("src/lib.rs"),
            b"pub fn needle() {}\nsecond needle line\n",
        )
        .expect("write source fixture");
        fs::write(
            repository.join("src/nested/mod.rs"),
            b"pub const OTHER: &str = \"needle\";\n",
        )
        .expect("write nested fixture");
        fs::write(
            repository.join("tests/secret.rs"),
            b"needle outside scope\n",
        )
        .expect("write denied fixture");

        let repository_id = repository_id((sequence as u8).wrapping_add(20));
        let manifest = ProjectManifest::new(
            project_id((sequence as u8).wrapping_add(100)),
            repository_id,
            "Repository browser fixture",
            allowed_roots,
            Vec::new(),
            LanguageProfile::Rust,
            Vec::new(),
        )
        .expect("valid browser manifest");
        let boundary = RepositoryBoundary::open(repository_id, &repository).expect("boundary");
        Self {
            root,
            repository,
            manifest,
            boundary,
        }
    }

    fn browser(&self) -> RepositoryBrowser {
        RepositoryBrowser::new(&self.manifest, &self.boundary).expect("browser")
    }

    fn request(&self, relative_path: impl AsRef<Path>) -> RepositoryPathRequest {
        RepositoryPathRequest::new(self.manifest.repository_id(), relative_path)
            .expect("valid fixture request")
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

#[test]
fn approved_tree_is_deterministic_and_hides_denied_roots() {
    let fixture = Fixture::new("tree", vec![AllowedProjectRoot::relative("src").unwrap()]);
    let browser = fixture.browser();
    let first = browser
        .tree(&RepositoryBrowseScope::approved_roots())
        .expect("first tree");
    let second = browser
        .tree(&RepositoryBrowseScope::approved_roots())
        .expect("second tree");

    assert_eq!(first, second);
    assert_eq!(first.project_id(), fixture.manifest.project_id());
    assert_eq!(first.repository_id(), fixture.manifest.repository_id());
    assert!(first.issues().is_empty());
    let paths: Vec<_> = first
        .entries()
        .iter()
        .map(|entry| entry.relative_path().as_path().to_path_buf())
        .collect();
    assert_eq!(
        paths,
        vec![
            PathBuf::from("src"),
            PathBuf::from("src/lib.rs"),
            PathBuf::from("src/nested"),
            PathBuf::from("src/nested/mod.rs"),
        ]
    );
    assert_eq!(first.entries()[0].kind(), RepositoryEntryKind::Directory);
    assert_eq!(first.entries()[1].kind(), RepositoryEntryKind::RegularFile);
    assert!(paths.iter().all(|path| !path.starts_with("tests")));
}

#[test]
fn exact_file_open_preserves_raw_bytes_and_denied_paths_fail_closed() {
    let fixture = Fixture::new("open", vec![AllowedProjectRoot::relative("src").unwrap()]);
    let raw = vec![0, 0xff, b'n', b'e', b'e', b'd', b'l', b'e', b'\n'];
    fs::write(fixture.repository.join("src/raw.bin"), &raw).expect("write raw file");
    let browser = fixture.browser();

    let snapshot = browser
        .open_file(&fixture.request("src/raw.bin"))
        .expect("open approved file");
    assert_eq!(snapshot.bytes(), raw);
    assert!(matches!(
        browser.open_file(&fixture.request("tests/secret.rs")),
        Err(ProjectFileError::PathNotAllowed { .. })
    ));
    assert!(matches!(
        browser.open_file(&fixture.request("src/nested")),
        Err(ProjectFileError::NotRegularFile { .. })
    ));
    assert_eq!(
        RepositoryBrowseScope::subtree("../outside"),
        Err(RepositoryPathError::ParentTraversal)
    );
    assert!(matches!(
        browser.tree(&RepositoryBrowseScope::subtree("tests").unwrap()),
        Err(RepositoryBrowseError::PathNotAllowed { .. })
    ));
}

#[test]
fn search_returns_stable_path_line_and_byte_positions_with_empty_no_match() {
    let fixture = Fixture::new("search", vec![AllowedProjectRoot::relative("src").unwrap()]);
    let browser = fixture.browser();
    let query = TextSearchQuery::new("needle").expect("query");
    let report = browser.search_text(&query).expect("search");

    assert_eq!(report.query(), "needle");
    assert_eq!(report.candidate_files(), 2);
    assert_eq!(report.scanned_files(), 2);
    assert_eq!(report.matches().len(), 3);
    assert!(report.issues().is_empty());
    assert!(!report.truncated());
    assert_eq!(
        report.matches()[0].relative_path().as_path(),
        Path::new("src/lib.rs")
    );
    assert_eq!(report.matches()[0].line_number(), 1);
    assert_eq!(report.matches()[0].byte_column(), 7);
    assert_eq!(report.matches()[1].line_number(), 2);
    assert_eq!(report.matches()[1].byte_column(), 7);
    assert_eq!(
        report.matches()[2].relative_path().as_path(),
        Path::new("src/nested/mod.rs")
    );

    let no_match = browser
        .search_text(&TextSearchQuery::new("definitely absent").unwrap())
        .expect("no-match search");
    assert!(no_match.matches().is_empty());
    assert!(!no_match.truncated());
}

#[cfg(unix)]
#[test]
fn symlink_escape_is_reported_and_never_searched() {
    let fixture = Fixture::new(
        "symlink",
        vec![AllowedProjectRoot::relative("src").unwrap()],
    );
    let outside = fixture.root.join("outside.txt");
    fs::write(&outside, b"forbidden-secret-needle\n").expect("write outside file");
    symlink(&outside, fixture.repository.join("src/escape.txt")).expect("create escape link");
    let browser = fixture.browser();

    let tree = browser
        .tree(&RepositoryBrowseScope::approved_roots())
        .expect("safe tree with rejected link");
    assert!(tree
        .entries()
        .iter()
        .all(|entry| { entry.relative_path().as_path() != Path::new("src/escape.txt") }));
    assert!(tree.issues().iter().any(|issue| {
        issue.relative_path().as_path() == Path::new("src/escape.txt")
            && matches!(issue.kind(), RepositoryScanIssueKind::SymlinkRejected)
    }));

    let report = browser
        .search_text(&TextSearchQuery::new("forbidden-secret-needle").unwrap())
        .expect("search skips rejected link");
    assert!(report.matches().is_empty());
    assert!(report.issues().iter().any(|issue| matches!(
        issue,
        RepositorySearchIssue::Scan(scan)
            if scan.relative_path().as_path() == Path::new("src/escape.txt")
                && matches!(scan.kind(), RepositoryScanIssueKind::SymlinkRejected)
    )));
    assert_eq!(fs::read(&outside).unwrap(), b"forbidden-secret-needle\n");
}

#[test]
fn unreadable_by_policy_files_are_explicit_search_issues() {
    let fixture = Fixture::new("large", vec![AllowedProjectRoot::relative("src").unwrap()]);
    let large_path = fixture.repository.join("src/too-large.bin");
    let large = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&large_path)
        .expect("create sparse large file");
    large
        .set_len(65 * 1024 * 1024)
        .expect("set sparse file length");
    drop(large);

    let report = fixture
        .browser()
        .search_text(&TextSearchQuery::new("needle").unwrap())
        .expect("search reports oversized file");
    assert!(report.issues().iter().any(|issue| matches!(
        issue,
        RepositorySearchIssue::File {
            relative_path,
            error: ProjectFileError::FileTooLarge { .. },
        } if relative_path.as_path() == Path::new("src/too-large.bin")
    )));
}

#[test]
fn search_limit_is_explicit_and_search_never_mutates_files() {
    let fixture = Fixture::new("limit", vec![AllowedProjectRoot::relative("src").unwrap()]);
    let target = fixture.repository.join("src/repeated.txt");
    fs::write(&target, b"x x x x\n").expect("write repeated fixture");
    let before = fs::read(&target).unwrap();
    let browser = fixture.browser();
    let query = TextSearchQuery::scoped(RepositoryBrowseScope::subtree("src").unwrap(), "x", 2)
        .expect("bounded query");
    let report = browser.search_text(&query).expect("bounded search");

    assert_eq!(report.matches().len(), 2);
    assert!(report.truncated());
    assert_eq!(fs::read(&target).unwrap(), before);
    assert_eq!(
        TextSearchQuery::scoped(RepositoryBrowseScope::approved_roots(), "x", 0),
        Err(TextSearchQueryError::InvalidMaximumMatches {
            minimum: 1,
            maximum: 16_384,
            actual: 0,
        })
    );
}

#[cfg(unix)]
#[test]
fn non_utf8_entry_names_round_trip_without_lossy_display_identity() {
    let fixture = Fixture::new(
        "non-utf8",
        vec![AllowedProjectRoot::relative("src").unwrap()],
    );
    let leaf = OsString::from_vec(vec![b'z', 0xff, b'.', b'r', b's']);
    fs::write(fixture.repository.join("src").join(&leaf), b"needle\n")
        .expect("write non-UTF8 entry");
    let tree = fixture
        .browser()
        .tree(&RepositoryBrowseScope::approved_roots())
        .expect("browse non-UTF8 entry");

    let entry = tree
        .entries()
        .iter()
        .find(|entry| entry.relative_path().as_path().file_name() == Some(leaf.as_os_str()))
        .expect("non-UTF8 entry retained");
    assert_eq!(
        entry.relative_path().as_path().as_os_str().as_bytes(),
        PathBuf::from("src").join(&leaf).as_os_str().as_bytes()
    );
}
