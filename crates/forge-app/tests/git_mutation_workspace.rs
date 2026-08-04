#![cfg(unix)]

use forge_app::composition::git_mutation_workspace::{
    ProjectGitMutationWorkspace, ProjectGitMutationWorkspaceError,
};
use forge_core::projects::{AllowedProjectRoot, LanguageProfile, ProjectManifest};
use forge_git::mutation::{
    GitBranchName, GitCommitIdentity, GitMutationError, RemoveWorktreeConfirmation,
    RestoreConfirmation,
};
use forge_git::status::GitStatusEntry;
use forge_project::paths::RepositoryBoundary;
use forge_protocol::identities::{ProjectId, RepositoryId, IDENTITY_BYTES};
use forge_protocol::paths::RepositoryRelativePath;
use std::ffi::OsString;
use std::fs;
use std::os::unix::ffi::{OsStrExt, OsStringExt};
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
            "forgeos-git-mutation-workspace-{label}-{}-{sequence}",
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
        fs::write(repository.join("src/other.rs"), b"pub fn other() {}\n").unwrap();
        git_ok(&repository, &["add", "--", "."]);
        git_ok(&repository, &["commit", "-q", "-m", "initial"]);
        let repository = fs::canonicalize(repository).unwrap();
        let project_id = ProjectId::from_bytes([(sequence as u8).wrapping_add(20); IDENTITY_BYTES]);
        let repository_id =
            RepositoryId::from_bytes([(sequence as u8).wrapping_add(100); IDENTITY_BYTES]);
        let manifest = ProjectManifest::new(
            project_id,
            repository_id,
            "Git mutation fixture",
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

    fn workspace(&self) -> ProjectGitMutationWorkspace {
        ProjectGitMutationWorkspace::new(&self.manifest, self.boundary.clone()).unwrap()
    }

    fn linked_target(&self, name: &str) -> PathBuf {
        self.root.join(name)
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

fn relative(path: impl AsRef<Path>) -> RepositoryRelativePath {
    RepositoryRelativePath::new(path).unwrap()
}

fn entry<'a>(
    snapshot: &'a forge_app::composition::git_workspace::ProjectGitSnapshot,
    path: &[u8],
) -> &'a GitStatusEntry {
    snapshot
        .inspection()
        .status()
        .entries()
        .iter()
        .find(|entry| entry.path().as_bytes() == path)
        .unwrap()
}

#[test]
fn stage_and_unstage_touch_only_the_explicit_selected_path() {
    let fixture = Fixture::new("stage-unstage");
    let workspace = fixture.workspace();
    fs::write(
        fixture.repository.join("src/lib.rs"),
        b"pub fn changed() {}\n",
    )
    .unwrap();
    fs::write(
        fixture.repository.join("src/other.rs"),
        b"pub fn untouched_by_index() {}\n",
    )
    .unwrap();

    let before = workspace.inspect().unwrap();
    let staged = workspace
        .stage(&before, vec![relative("src/lib.rs")])
        .unwrap();
    assert_eq!(staged.project_id(), fixture.manifest.project_id());
    assert_eq!(staged.before_identity(), before.inspection().identity());
    assert_eq!(
        entry(staged.after(), b"src/lib.rs")
            .status()
            .unwrap()
            .index(),
        b'M'
    );
    assert_eq!(
        entry(staged.after(), b"src/other.rs")
            .status()
            .unwrap()
            .index(),
        b'.'
    );

    let unstaged = workspace
        .unstage(staged.after(), vec![relative("src/lib.rs")])
        .unwrap();
    assert_eq!(
        entry(unstaged.after(), b"src/lib.rs")
            .status()
            .unwrap()
            .index(),
        b'.'
    );
    assert_eq!(
        fs::read(fixture.repository.join("src/lib.rs")).unwrap(),
        b"pub fn changed() {}\n"
    );
    assert_eq!(
        fs::read(fixture.repository.join("src/other.rs")).unwrap(),
        b"pub fn untouched_by_index() {}\n"
    );
}

#[test]
fn stale_or_unselected_views_fail_before_index_mutation() {
    let fixture = Fixture::new("stale");
    let workspace = fixture.workspace();
    fs::write(fixture.repository.join("src/lib.rs"), b"pub fn one() {}\n").unwrap();
    let stale = workspace.inspect().unwrap();
    fs::write(fixture.repository.join("src/lib.rs"), b"pub fn two() {}\n").unwrap();

    assert!(matches!(
        workspace.stage(&stale, vec![relative("src/lib.rs")]),
        Err(ProjectGitMutationWorkspaceError::StaleSnapshot { .. })
    ));
    assert!(
        git_ok(&fixture.repository, &["diff", "--cached", "--quiet"])
            .status
            .success()
    );

    let current = workspace.inspect().unwrap();
    assert!(matches!(
        workspace.stage(&current, vec![relative("src/other.rs")]),
        Err(ProjectGitMutationWorkspaceError::PathNotInSnapshot(path))
            if path.as_path() == Path::new("src/other.rs")
    ));
}

#[test]
fn empty_and_duplicate_selections_are_rejected_before_git_runs() {
    let fixture = Fixture::new("selection-shape");
    let workspace = fixture.workspace();
    fs::write(fixture.repository.join("src/lib.rs"), b"selected\n").unwrap();
    let before = workspace.inspect().unwrap();

    assert!(matches!(
        workspace.stage(&before, Vec::new()),
        Err(ProjectGitMutationWorkspaceError::EmptySelection)
    ));
    assert!(matches!(
        workspace.stage(
            &before,
            vec![relative("src/lib.rs"), relative("src/lib.rs")],
        ),
        Err(ProjectGitMutationWorkspaceError::DuplicateSelection(path))
            if path.as_path() == Path::new("src/lib.rs")
    ));
    assert!(
        git_ok(&fixture.repository, &["diff", "--cached", "--quiet"])
            .status
            .success()
    );
}

#[test]
fn confirmed_restore_discards_only_selected_tracked_bytes() {
    let fixture = Fixture::new("restore");
    let workspace = fixture.workspace();
    fs::write(fixture.repository.join("src/lib.rs"), b"discard me\n").unwrap();
    fs::write(fixture.repository.join("src/other.rs"), b"keep me\n").unwrap();
    fs::write(fixture.repository.join("src/new.rs"), b"untracked\n").unwrap();
    let before = workspace.inspect().unwrap();

    let restored = workspace
        .restore(
            &before,
            vec![relative("src/lib.rs")],
            RestoreConfirmation::DiscardExactPaths,
        )
        .unwrap();
    assert_eq!(
        fs::read(fixture.repository.join("src/lib.rs")).unwrap(),
        b"pub fn first() {}\n"
    );
    assert_eq!(
        fs::read(fixture.repository.join("src/other.rs")).unwrap(),
        b"keep me\n"
    );
    assert!(restored
        .after()
        .inspection()
        .status()
        .entries()
        .iter()
        .all(|entry| entry.path().as_bytes() != b"src/lib.rs"));

    let current = workspace.inspect().unwrap();
    assert!(matches!(
        workspace.restore(
            &current,
            vec![relative("src/new.rs")],
            RestoreConfirmation::DiscardExactPaths,
        ),
        Err(ProjectGitMutationWorkspaceError::PathNotRestorable(path))
            if path.as_path() == Path::new("src/new.rs")
    ));
}

#[test]
fn commit_is_bound_to_the_exact_selected_staged_patch() {
    let fixture = Fixture::new("commit");
    let workspace = fixture.workspace();
    fs::write(
        fixture.repository.join("src/lib.rs"),
        b"pub fn committed() {}\n",
    )
    .unwrap();
    fs::write(
        fixture.repository.join("src/other.rs"),
        b"pub fn left_dirty() {}\n",
    )
    .unwrap();
    let selected = workspace.inspect().unwrap();
    let staged = workspace
        .stage(&selected, vec![relative("src/lib.rs")])
        .unwrap();
    let old_head = staged
        .after()
        .inspection()
        .status()
        .head()
        .revision()
        .unwrap()
        .clone();

    let committed = workspace
        .commit(
            staged.after(),
            b"project-bound exact commit\n".to_vec(),
            GitCommitIdentity::new("ForgeOS Operator", "operator@forgeos.invalid").unwrap(),
        )
        .unwrap();
    let new_head = committed
        .after()
        .inspection()
        .status()
        .head()
        .revision()
        .unwrap();
    assert_ne!(new_head, &old_head);
    assert_eq!(
        git_ok(&fixture.repository, &["show", "HEAD:src/lib.rs"]).stdout,
        b"pub fn committed() {}\n"
    );
    assert_eq!(
        fs::read(fixture.repository.join("src/other.rs")).unwrap(),
        b"pub fn left_dirty() {}\n"
    );
    assert_eq!(
        git_ok(&fixture.repository, &["log", "-1", "--format=%B"]).stdout,
        b"project-bound exact commit\n\n"
    );
}

#[test]
fn linked_worktree_create_and_remove_require_exact_clean_state() {
    let fixture = Fixture::new("worktree");
    let workspace = fixture.workspace();
    let before = workspace.inspect().unwrap();
    let head = before
        .inspection()
        .status()
        .head()
        .revision()
        .unwrap()
        .clone();
    let target = fixture.linked_target("linked-clean");

    let created = workspace
        .create_worktree(
            &before,
            target.clone(),
            GitBranchName::new("feature/project-bound").unwrap(),
        )
        .unwrap();
    assert_eq!(
        fs::read(target.join("src/lib.rs")).unwrap(),
        b"pub fn first() {}\n"
    );
    let removed = workspace
        .remove_worktree(
            created.after(),
            target.clone(),
            head.clone(),
            RemoveWorktreeConfirmation::RemoveCleanLinkedWorktree,
        )
        .unwrap();
    assert!(!target.exists());
    assert_eq!(removed.repository_id(), fixture.manifest.repository_id());

    let dirty_target = fixture.linked_target("linked-dirty");
    let selected = workspace.inspect().unwrap();
    let dirty_created = workspace
        .create_worktree(
            &selected,
            dirty_target.clone(),
            GitBranchName::new("feature/dirty-linked").unwrap(),
        )
        .unwrap();
    fs::write(dirty_target.join("src/lib.rs"), b"dirty linked worktree\n").unwrap();
    assert!(matches!(
        workspace.remove_worktree(
            dirty_created.after(),
            dirty_target.clone(),
            head,
            RemoveWorktreeConfirmation::RemoveCleanLinkedWorktree,
        ),
        Err(ProjectGitMutationWorkspaceError::Mutation(
            GitMutationError::WorktreeNotClean(path)
        )) if path == dirty_target
    ));
    assert!(dirty_target.exists());
}

#[test]
fn foreign_project_snapshot_cannot_authorize_mutation() {
    let first = Fixture::new("foreign-first");
    let second = Fixture::new("foreign-second");
    let first_workspace = first.workspace();
    let second_workspace = second.workspace();
    fs::write(second.repository.join("src/lib.rs"), b"foreign\n").unwrap();
    let foreign = second_workspace.inspect().unwrap();

    assert!(matches!(
        first_workspace.stage(&foreign, vec![relative("src/lib.rs")]),
        Err(ProjectGitMutationWorkspaceError::ProjectMismatch { .. })
    ));
    assert!(git_ok(&first.repository, &["diff", "--cached", "--quiet"])
        .status
        .success());
}

#[test]
fn non_utf8_selected_path_reaches_native_git_without_loss() {
    let fixture = Fixture::new("non-utf8");
    let workspace = fixture.workspace();
    let raw = vec![b's', b'r', b'c', b'/', 0xff, b'.', b'r', b's'];
    let relative_path = PathBuf::from(OsString::from_vec(raw.clone()));
    fs::write(
        fixture.repository.join(&relative_path),
        b"pub fn raw() {}\n",
    )
    .unwrap();
    let before = workspace.inspect().unwrap();

    let staged = workspace
        .stage(&before, vec![relative(&relative_path)])
        .unwrap();
    assert!(staged
        .after()
        .inspection()
        .status()
        .entries()
        .iter()
        .any(|entry| {
            entry.path().as_bytes() == raw
                && entry.status().is_some_and(|status| status.index() == b'A')
        }));
    let native = git_ok(
        &fixture.repository,
        &["diff", "--cached", "--name-only", "-z"],
    );
    assert_eq!(native.stdout, [raw, vec![0]].concat());
    assert_eq!(
        relative_path.as_os_str().as_bytes(),
        &native.stdout[..native.stdout.len() - 1]
    );
}
