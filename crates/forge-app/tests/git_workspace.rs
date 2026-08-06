#![cfg(unix)]

use forge_app::composition::git_workspace::{ProjectGitWorkspace, ProjectGitWorkspaceError};
use forge_core::projects::{AllowedProjectRoot, LanguageProfile, ProjectManifest};
use forge_git::status::GitBranch;
use forge_project::paths::RepositoryBoundary;
use forge_protocol::identities::{IDENTITY_BYTES, ProjectId, RepositoryId};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicU64, Ordering};

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
            "forgeos-git-workspace-{label}-{}-{sequence}",
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
        fs::write(repository.join("src/lib.rs"), b"pub fn first() {}\n").unwrap();
        git_ok(&repository, &["add", "--", "."]);
        git_ok(&repository, &["commit", "-q", "-m", "initial"]);
        let repository = fs::canonicalize(repository).unwrap();
        let project_id = ProjectId::from_bytes([(sequence as u8).wrapping_add(10); IDENTITY_BYTES]);
        let repository_id =
            RepositoryId::from_bytes([(sequence as u8).wrapping_add(80); IDENTITY_BYTES]);
        let manifest = ProjectManifest::new(
            project_id,
            repository_id,
            "Git workspace fixture",
            vec![AllowedProjectRoot::relative("src").unwrap()],
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
fn project_workspace_reads_exact_registered_repository_state() {
    let fixture = Fixture::new("inspect");
    fs::write(
        fixture.repository.join("src/lib.rs"),
        b"pub fn first() {}\npub fn second() {}\n",
    )
    .unwrap();
    fs::write(
        fixture.repository.join("src/staged.rs"),
        b"pub fn staged() {}\n",
    )
    .unwrap();
    git_ok(&fixture.repository, &["add", "--", "src/staged.rs"]);
    fs::write(fixture.repository.join("outside.txt"), b"untracked\n").unwrap();

    let workspace = ProjectGitWorkspace::new(&fixture.manifest, fixture.boundary.clone()).unwrap();
    let snapshot = workspace.inspect().unwrap();

    assert_eq!(snapshot.project_id(), fixture.manifest.project_id());
    assert_eq!(snapshot.repository_id(), fixture.manifest.repository_id());
    assert!(matches!(
        snapshot.inspection().status().head().branch(),
        GitBranch::Attached(branch) if branch.as_bytes() == b"main"
    ));
    let native_head =
        String::from_utf8(git_ok(&fixture.repository, &["rev-parse", "HEAD"]).stdout).unwrap();
    assert_eq!(
        snapshot
            .inspection()
            .status()
            .head()
            .revision()
            .unwrap()
            .as_str(),
        native_head.trim()
    );
    assert!(
        snapshot
            .inspection()
            .status()
            .entries()
            .iter()
            .any(|entry| entry.path().as_bytes() == b"outside.txt")
    );
    assert!(
        snapshot
            .inspection()
            .worktree_diff()
            .entries()
            .iter()
            .any(|entry| entry.source_path().as_bytes() == b"src/lib.rs")
    );
    assert!(
        snapshot
            .inspection()
            .staged_diff()
            .entries()
            .iter()
            .any(|entry| entry.source_path().as_bytes() == b"src/staged.rs")
    );
}

#[test]
fn repeated_project_inspection_is_stable_but_not_cached() {
    let fixture = Fixture::new("fresh");
    let workspace = ProjectGitWorkspace::new(&fixture.manifest, fixture.boundary.clone()).unwrap();
    let first = workspace.inspect().unwrap();
    let second = workspace.inspect().unwrap();
    assert_eq!(
        first.inspection().identity(),
        second.inspection().identity()
    );

    fs::write(
        fixture.repository.join("src/lib.rs"),
        b"pub fn changed() {}\n",
    )
    .unwrap();
    let changed = workspace.inspect().unwrap();
    assert_ne!(
        first.inspection().identity(),
        changed.inspection().identity()
    );
}

#[test]
fn manifest_and_boundary_repository_identity_must_match() {
    let fixture = Fixture::new("foreign");
    let foreign = RepositoryBoundary::open(
        RepositoryId::from_bytes([240; IDENTITY_BYTES]),
        &fixture.repository,
    )
    .unwrap();
    assert!(matches!(
        ProjectGitWorkspace::new(&fixture.manifest, foreign),
        Err(ProjectGitWorkspaceError::RepositoryMismatch { .. })
    ));
}

#[test]
fn repository_subdirectory_cannot_masquerade_as_the_registered_git_root() {
    let fixture = Fixture::new("subdirectory");
    let boundary = RepositoryBoundary::open(
        fixture.manifest.repository_id(),
        fixture.repository.join("src"),
    )
    .unwrap();
    assert!(matches!(
        ProjectGitWorkspace::new(&fixture.manifest, boundary),
        Err(ProjectGitWorkspaceError::GitOpen(_))
    ));
}

#[test]
fn replacing_the_registered_repository_object_invalidates_future_inspection() {
    let fixture = Fixture::new("replacement");
    let workspace = ProjectGitWorkspace::new(&fixture.manifest, fixture.boundary.clone()).unwrap();
    let moved = fixture.root.join("moved");
    fs::rename(&fixture.repository, &moved).unwrap();
    fs::create_dir(&fixture.repository).unwrap();

    assert!(matches!(
        workspace.inspect(),
        Err(ProjectGitWorkspaceError::Boundary(_))
    ));
}
