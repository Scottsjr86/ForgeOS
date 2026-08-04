#![cfg(unix)]

use forge_bridge::git::{GitDiffInvocation, NativeGitFailureStage};
use forge_git::diff::DiffScope;
use forge_git::repository::{GitInspectError, GitInspectOperation, GitRepositoryInspector};
use forge_git::status::{GitBranch, GitStatusEntryKind};
use forge_git::worktree::GitWorktreeState;
use forge_protocol::identities::{RepositoryId, IDENTITY_BYTES};
use std::ffi::OsString;
use std::fs;
use std::io::Write;
use std::os::unix::ffi::{OsStrExt, OsStringExt};
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_TEMP: AtomicU64 = AtomicU64::new(1);

fn repository_id(byte: u8) -> RepositoryId {
    RepositoryId::from_bytes([byte; IDENTITY_BYTES])
}

struct TempRepository {
    root: PathBuf,
    extra_paths: Vec<PathBuf>,
}

impl TempRepository {
    fn committed(label: &str) -> Self {
        let repository = Self::empty(label);
        fs::write(repository.root.join("tracked.txt"), b"first\n").unwrap();
        git_ok(&repository.root, &["add", "--", "tracked.txt"]);
        git_ok(&repository.root, &["commit", "-q", "-m", "initial"]);
        repository
    }

    fn empty(label: &str) -> Self {
        let unique = NEXT_TEMP.fetch_add(1, Ordering::Relaxed);
        let root = std::env::temp_dir().join(format!(
            "forgeos-git-100-{label}-{}-{unique}",
            std::process::id()
        ));
        fs::create_dir(&root).unwrap();
        git_ok(&root, &["init", "-q"]);
        git_ok(&root, &["symbolic-ref", "HEAD", "refs/heads/main"]);
        git_ok(&root, &["config", "user.name", "ForgeOS Test"]);
        git_ok(&root, &["config", "user.email", "forgeos@example.invalid"]);
        Self {
            root,
            extra_paths: Vec::new(),
        }
    }

    fn inspector(&self) -> GitRepositoryInspector {
        GitRepositoryInspector::open(repository_id(1), &self.root).unwrap()
    }

    fn revision(&self) -> String {
        String::from_utf8(git_ok(&self.root, &["rev-parse", "HEAD"]).stdout)
            .unwrap()
            .trim()
            .to_owned()
    }

    fn add_worktree(&mut self) -> PathBuf {
        let linked = self.root.with_extension("linked");
        git_ok(
            &self.root,
            &[
                "worktree",
                "add",
                "-q",
                "-b",
                "feature",
                linked.to_str().unwrap(),
            ],
        );
        self.extra_paths.push(linked.clone());
        linked
    }
}

impl Drop for TempRepository {
    fn drop(&mut self) {
        for path in self.extra_paths.iter().rev() {
            let _ = fs::remove_dir_all(path);
        }
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

fn publish_executable(path: &Path, bytes: &[u8]) {
    let staging = path.with_extension("stage");
    {
        let mut file = fs::File::create(&staging).unwrap();
        file.write_all(bytes).unwrap();
        file.sync_all().unwrap();
    }
    fs::set_permissions(&staging, fs::Permissions::from_mode(0o755)).unwrap();
    fs::rename(staging, path).unwrap();
}

#[test]
fn diff_range_rejects_git_option_injection_before_spawn() {
    let valid = "0".repeat(40);
    let error = GitDiffInvocation::between("--output=/tmp/forgeos-escape", valid).unwrap_err();
    assert_eq!(error.field(), "base");
}

#[test]
fn attached_branch_and_revision_match_native_git() {
    let repository = TempRepository::committed("head");
    let head = repository.inspector().inspect_head().unwrap();
    assert_eq!(head.repository_id(), repository_id(1));
    match head.branch() {
        GitBranch::Attached(branch) => assert_eq!(branch.as_bytes(), b"main"),
        GitBranch::Detached => panic!("fixture branch must be attached"),
    }
    assert_eq!(head.revision().unwrap().as_str(), repository.revision());
}

#[test]
fn unborn_repository_has_branch_without_fake_revision() {
    let repository = TempRepository::empty("unborn");
    let head = repository.inspector().inspect_head().unwrap();
    assert!(matches!(head.branch(), GitBranch::Attached(branch) if branch.as_bytes() == b"main"));
    assert_eq!(head.revision(), None);
}

#[test]
fn detached_head_is_not_collapsed_into_a_branch_name() {
    let repository = TempRepository::committed("detached");
    git_ok(&repository.root, &["checkout", "-q", "--detach", "HEAD"]);
    let head = repository.inspector().inspect_head().unwrap();
    assert_eq!(head.branch(), &GitBranch::Detached);
    assert_eq!(head.revision().unwrap().as_str(), repository.revision());
}

#[test]
fn status_preserves_staged_unstaged_and_untracked_states() {
    let repository = TempRepository::committed("status");
    fs::write(repository.root.join("tracked.txt"), b"changed\n").unwrap();
    fs::write(repository.root.join("staged.txt"), b"staged\n").unwrap();
    git_ok(&repository.root, &["add", "--", "staged.txt"]);
    fs::write(repository.root.join("untracked.txt"), b"loose\n").unwrap();

    let status = repository.inspector().inspect_status().unwrap();
    assert_eq!(status.repository_id(), repository_id(1));
    assert!(!status.is_clean());
    let tracked = status
        .entries()
        .iter()
        .find(|entry| entry.path().as_bytes() == b"tracked.txt")
        .unwrap();
    assert_eq!(tracked.kind(), GitStatusEntryKind::Ordinary);
    assert_eq!(tracked.status().unwrap().index(), b'.');
    assert_eq!(tracked.status().unwrap().worktree(), b'M');

    let staged = status
        .entries()
        .iter()
        .find(|entry| entry.path().as_bytes() == b"staged.txt")
        .unwrap();
    assert_eq!(staged.status().unwrap().index(), b'A');
    assert_eq!(staged.status().unwrap().worktree(), b'.');

    let untracked = status
        .entries()
        .iter()
        .find(|entry| entry.path().as_bytes() == b"untracked.txt")
        .unwrap();
    assert_eq!(untracked.kind(), GitStatusEntryKind::Untracked);
    assert_eq!(untracked.status(), None);
}

#[test]
fn rename_status_preserves_both_unquoted_paths_and_score() {
    let repository = TempRepository::committed("rename");
    git_ok(
        &repository.root,
        &["mv", "--", "tracked.txt", "renamed.txt"],
    );
    let status = repository.inspector().inspect_status().unwrap();
    let renamed = status
        .entries()
        .iter()
        .find(|entry| entry.kind() == GitStatusEntryKind::RenameOrCopy)
        .unwrap();
    assert_eq!(renamed.path().as_bytes(), b"renamed.txt");
    assert_eq!(renamed.original_path().unwrap().as_bytes(), b"tracked.txt");
    assert_eq!(renamed.status().unwrap().index(), b'R');
    assert!(renamed.metadata_tokens().last().unwrap().starts_with(b"R"));
}

#[test]
fn worktree_porcelain_reports_primary_and_linked_worktrees() {
    let mut repository = TempRepository::committed("worktrees");
    let linked = repository.add_worktree();
    let snapshot = repository.inspector().inspect_worktrees().unwrap();
    assert_eq!(snapshot.repository_id(), repository_id(1));
    let worktrees = snapshot.worktrees();
    assert_eq!(worktrees.len(), 2);
    let primary_bytes = fs::canonicalize(&repository.root)
        .unwrap()
        .as_os_str()
        .as_bytes()
        .to_vec();
    let linked_bytes = fs::canonicalize(&linked)
        .unwrap()
        .as_os_str()
        .as_bytes()
        .to_vec();
    let primary = worktrees
        .iter()
        .find(|worktree| worktree.path().as_bytes() == primary_bytes.as_slice())
        .unwrap();
    assert!(
        matches!(primary.state(), GitWorktreeState::Branch(branch) if branch.as_bytes() == b"refs/heads/main")
    );
    let feature = worktrees
        .iter()
        .find(|worktree| worktree.path().as_bytes() == linked_bytes.as_slice())
        .unwrap();
    assert!(
        matches!(feature.state(), GitWorktreeState::Branch(branch) if branch.as_bytes() == b"refs/heads/feature")
    );
}

#[test]
fn worktree_diff_exposes_typed_entry_and_exact_patch_bytes() {
    let repository = TempRepository::committed("diff-worktree");
    fs::write(repository.root.join("tracked.txt"), b"first\nsecond\n").unwrap();
    let diff = repository
        .inspector()
        .inspect_diff(DiffScope::Worktree)
        .unwrap();
    assert_eq!(diff.repository_id(), repository_id(1));
    assert_eq!(diff.entries().len(), 1);
    let entry = &diff.entries()[0];
    assert_eq!(entry.status().code(), b'M');
    assert_eq!(entry.source_path().as_bytes(), b"tracked.txt");
    assert_eq!(entry.destination_path(), None);
    assert!(diff
        .patch_bytes()
        .windows(7)
        .any(|window| window == b"+second"));
}

#[test]
fn staged_and_worktree_diffs_remain_distinct() {
    let repository = TempRepository::committed("diff-stages");
    fs::write(repository.root.join("tracked.txt"), b"first\nstaged\n").unwrap();
    git_ok(&repository.root, &["add", "--", "tracked.txt"]);
    fs::write(
        repository.root.join("tracked.txt"),
        b"first\nstaged\nworktree\n",
    )
    .unwrap();
    let inspector = repository.inspector();
    let staged = inspector.inspect_diff(DiffScope::Staged).unwrap();
    let worktree = inspector.inspect_diff(DiffScope::Worktree).unwrap();
    assert_ne!(staged.patch_bytes(), worktree.patch_bytes());
    assert!(staged
        .patch_bytes()
        .windows(7)
        .any(|window| window == b"+staged"));
    assert!(worktree
        .patch_bytes()
        .windows(9)
        .any(|window| window == b"+worktree"));
}

#[test]
fn exact_revision_range_diff_is_typed_and_binary_safe() {
    let repository = TempRepository::committed("diff-range");
    let first = repository
        .inspector()
        .inspect_head()
        .unwrap()
        .revision()
        .unwrap()
        .clone();
    fs::write(repository.root.join("tracked.txt"), b"first\ncommitted\n").unwrap();
    git_ok(&repository.root, &["add", "--", "tracked.txt"]);
    git_ok(&repository.root, &["commit", "-q", "-m", "second"]);
    let second = repository
        .inspector()
        .inspect_head()
        .unwrap()
        .revision()
        .unwrap()
        .clone();
    let diff = repository
        .inspector()
        .inspect_diff(DiffScope::between(first, second))
        .unwrap();
    assert_eq!(diff.entries().len(), 1);
    assert!(diff
        .patch_bytes()
        .windows(10)
        .any(|window| window == b"+committed"));
}

#[test]
fn read_only_inspection_preserves_native_repository_state() {
    let repository = TempRepository::committed("no-mutation");
    fs::write(repository.root.join("untracked.txt"), b"loose\n").unwrap();
    let before_status = git_ok(
        &repository.root,
        &[
            "status",
            "--porcelain=v2",
            "--branch",
            "-z",
            "--untracked-files=all",
        ],
    )
    .stdout;
    let before_revision = repository.revision();

    let inspector = repository.inspector();
    inspector.inspect_status().unwrap();
    inspector.inspect_worktrees().unwrap();
    inspector.inspect_diff(DiffScope::Worktree).unwrap();
    inspector.inspect_diff(DiffScope::Staged).unwrap();

    let after_status = git_ok(
        &repository.root,
        &[
            "status",
            "--porcelain=v2",
            "--branch",
            "-z",
            "--untracked-files=all",
        ],
    )
    .stdout;
    assert_eq!(before_status, after_status);
    assert_eq!(before_revision, repository.revision());
}

#[test]
fn non_utf8_status_paths_are_preserved_without_replacement() {
    let repository = TempRepository::committed("non-utf8");
    let raw_name = vec![0xff, b'-', b's', b'o', b'u', b'r', b'c', b'e'];
    let name = OsString::from_vec(raw_name.clone());
    fs::write(repository.root.join(&name), b"raw\n").unwrap();
    let status = repository.inspector().inspect_status().unwrap();
    let entry = status
        .entries()
        .iter()
        .find(|entry| entry.path().as_bytes() == raw_name.as_slice())
        .expect("raw non-UTF8 path must be present");
    assert_eq!(entry.kind(), GitStatusEntryKind::Untracked);
}

#[test]
fn malformed_machine_output_is_a_typed_parse_failure() {
    let repository = TempRepository::committed("malformed");
    let program = repository.root.join("fake-git.py");
    publish_executable(
        &program,
        b"#!/usr/bin/env python3\nimport os,sys\nif 'HOME' in os.environ or 'GIT_DIR' in os.environ or 'GIT_WORK_TREE' in os.environ:\n sys.exit(70)\nif 'rev-parse' in sys.argv:\n os.write(1,b'true\\n\\n')\nelse:\n os.write(1,b'# branch.oid broken\\x00# branch.head main\\x00')\n",
    );

    let inspector =
        GitRepositoryInspector::with_program(repository_id(5), &repository.root, &program).unwrap();
    assert!(matches!(
        inspector.inspect_status(),
        Err(GitInspectError::MalformedOutput {
            operation: GitInspectOperation::Status,
            ..
        })
    ));
}

#[test]
fn non_repository_preserves_native_exit_and_stderr() {
    let unique = NEXT_TEMP.fetch_add(1, Ordering::Relaxed);
    let root = std::env::temp_dir().join(format!(
        "forgeos-git-100-plain-{}-{unique}",
        std::process::id()
    ));
    fs::create_dir(&root).unwrap();
    let error = GitRepositoryInspector::open(repository_id(2), &root).unwrap_err();
    match error {
        GitInspectError::NativeFailure(failure) => {
            assert_eq!(failure.operation(), GitInspectOperation::OpenRepository);
            assert!(!failure.exit().success());
            assert!(!failure.stderr().is_empty());
        }
        other => panic!("expected native Git failure, found {other:?}"),
    }
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn repository_subdirectory_is_rejected_as_the_wrong_boundary() {
    let repository = TempRepository::committed("subdirectory");
    let subdirectory = repository.root.join("nested");
    fs::create_dir(&subdirectory).unwrap();
    assert_eq!(
        GitRepositoryInspector::open(repository_id(3), &subdirectory),
        Err(GitInspectError::NotRepositoryRoot(
            fs::canonicalize(subdirectory).unwrap()
        ))
    );
}

#[test]
fn missing_git_executable_is_a_typed_spawn_failure() {
    let repository = TempRepository::committed("missing-git");
    let error = GitRepositoryInspector::with_program(
        repository_id(4),
        &repository.root,
        "forgeos-definitely-missing-git",
    )
    .unwrap_err();
    match error {
        GitInspectError::NativeInvocation(error) => {
            assert_eq!(error.stage(), NativeGitFailureStage::Spawn);
            assert_eq!(error.kind(), std::io::ErrorKind::NotFound);
        }
        other => panic!("expected native invocation failure, found {other:?}"),
    }
}

#[test]
fn replacing_the_repository_root_invalidates_the_inspector() {
    let mut repository = TempRepository::committed("replace-root");
    let inspector = repository.inspector();
    let moved = repository.root.with_extension("moved");
    fs::rename(&repository.root, &moved).unwrap();
    fs::create_dir(&repository.root).unwrap();
    repository.extra_paths.push(moved);
    assert!(matches!(
        inspector.inspect_status(),
        Err(GitInspectError::RootIdentityChanged { .. })
    ));
}
