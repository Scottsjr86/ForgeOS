#![cfg(unix)]

use forge_git::inspection::GitRepositoryInspectionError;
use forge_git::repository::GitRepositoryInspector;
use forge_git::status::{GitBranch, GitStatusEntryKind};
use forge_protocol::identities::{RepositoryId, IDENTITY_BYTES};
use std::fs;
use std::io::Write;
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
}

impl TempRepository {
    fn committed(label: &str) -> Self {
        let unique = NEXT_TEMP.fetch_add(1, Ordering::Relaxed);
        let root = std::env::temp_dir().join(format!(
            "forgeos-git-200-{label}-{}-{unique}",
            std::process::id()
        ));
        if root.exists() {
            fs::remove_dir_all(&root).unwrap();
        }
        fs::create_dir(&root).unwrap();
        git_ok(&root, &["init", "-q"]);
        git_ok(&root, &["symbolic-ref", "HEAD", "refs/heads/main"]);
        git_ok(&root, &["config", "user.name", "ForgeOS Test"]);
        git_ok(&root, &["config", "user.email", "forgeos@example.invalid"]);
        for (name, bytes) in [
            ("tracked.txt", &b"first\n"[..]),
            ("delete.txt", &b"delete\n"[..]),
            ("rename-old.txt", &b"rename\n"[..]),
            ("conflict.txt", &b"base\n"[..]),
        ] {
            fs::write(root.join(name), bytes).unwrap();
        }
        git_ok(&root, &["add", "--", "."]);
        git_ok(&root, &["commit", "-q", "-m", "initial"]);
        Self { root }
    }

    fn inspector(&self) -> GitRepositoryInspector {
        GitRepositoryInspector::open(repository_id(1), &self.root).unwrap()
    }
}

impl Drop for TempRepository {
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

fn shell_quote(path: &Path) -> String {
    format!("'{}'", path.display().to_string().replace('\'', "'\\''"))
}

#[test]
fn stable_inspection_binds_status_branch_revision_and_both_diff_scopes() {
    let repository = TempRepository::committed("stable");
    fs::write(repository.root.join("tracked.txt"), b"first\nsecond\n").unwrap();
    fs::write(repository.root.join("staged.txt"), b"staged\n").unwrap();
    git_ok(&repository.root, &["add", "--", "staged.txt"]);
    fs::write(repository.root.join("untracked.txt"), b"loose\n").unwrap();

    let inspector = repository.inspector();
    let first = inspector.inspect_consistent().unwrap();
    let second = inspector.inspect_consistent().unwrap();

    assert_eq!(first.identity(), second.identity());
    assert_eq!(first.repository_id(), repository_id(1));
    assert!(matches!(
        first.status().head().branch(),
        GitBranch::Attached(branch) if branch.as_bytes() == b"main"
    ));
    assert!(first.status().head().revision().is_some());
    assert!(!first.is_clean());
    assert!(first
        .status()
        .entries()
        .iter()
        .any(|entry| entry.path().as_bytes() == b"tracked.txt"));
    assert!(first
        .status()
        .entries()
        .iter()
        .any(|entry| entry.path().as_bytes() == b"staged.txt"));
    assert!(first
        .status()
        .entries()
        .iter()
        .any(|entry| entry.path().as_bytes() == b"untracked.txt"));
    assert!(first
        .worktree_diff()
        .entries()
        .iter()
        .any(|entry| entry.source_path().as_bytes() == b"tracked.txt"));
    assert!(first
        .staged_diff()
        .entries()
        .iter()
        .any(|entry| entry.source_path().as_bytes() == b"staged.txt"));
    assert!(first
        .worktree_diff()
        .patch_bytes()
        .windows(b"tracked.txt".len())
        .any(|window| window == b"tracked.txt"));
    assert!(first
        .staged_diff()
        .patch_bytes()
        .windows(b"staged.txt".len())
        .any(|window| window == b"staged.txt"));
}

#[test]
fn inspection_preserves_staged_unstaged_untracked_rename_delete_and_conflict_meaning() {
    let repository = TempRepository::committed("classes");

    git_ok(&repository.root, &["checkout", "-q", "-b", "other"]);
    fs::write(repository.root.join("conflict.txt"), b"other\n").unwrap();
    git_ok(&repository.root, &["add", "--", "conflict.txt"]);
    git_ok(&repository.root, &["commit", "-q", "-m", "other"]);
    git_ok(&repository.root, &["checkout", "-q", "main"]);
    fs::write(repository.root.join("conflict.txt"), b"main\n").unwrap();
    git_ok(&repository.root, &["add", "--", "conflict.txt"]);
    git_ok(&repository.root, &["commit", "-q", "-m", "main"]);
    let merge = git_output(&repository.root, &["merge", "--no-edit", "other"]);
    assert!(!merge.status.success(), "fixture merge must conflict");

    fs::write(repository.root.join("tracked.txt"), b"unstaged\n").unwrap();
    fs::write(repository.root.join("staged.txt"), b"staged\n").unwrap();
    git_ok(&repository.root, &["add", "--", "staged.txt"]);
    fs::write(repository.root.join("untracked.txt"), b"untracked\n").unwrap();
    git_ok(
        &repository.root,
        &["mv", "--", "rename-old.txt", "rename-new.txt"],
    );
    git_ok(&repository.root, &["rm", "-q", "--", "delete.txt"]);

    let snapshot = repository.inspector().inspect_consistent().unwrap();
    let entries = snapshot.status().entries();

    assert!(entries.iter().any(|entry| {
        entry.kind() == GitStatusEntryKind::Ordinary
            && entry.path().as_bytes() == b"tracked.txt"
            && entry
                .status()
                .is_some_and(|status| status.worktree() == b'M')
    }));
    assert!(entries.iter().any(|entry| {
        entry.kind() == GitStatusEntryKind::Ordinary
            && entry.path().as_bytes() == b"staged.txt"
            && entry.status().is_some_and(|status| status.index() == b'A')
    }));
    assert!(entries.iter().any(|entry| {
        entry.kind() == GitStatusEntryKind::Untracked && entry.path().as_bytes() == b"untracked.txt"
    }));
    assert!(entries.iter().any(|entry| {
        entry.kind() == GitStatusEntryKind::RenameOrCopy
            && entry.path().as_bytes() == b"rename-new.txt"
            && entry
                .original_path()
                .is_some_and(|path| path.as_bytes() == b"rename-old.txt")
    }));
    assert!(entries.iter().any(|entry| {
        entry.kind() == GitStatusEntryKind::Ordinary
            && entry.path().as_bytes() == b"delete.txt"
            && entry.status().is_some_and(|status| status.index() == b'D')
    }));
    assert!(entries.iter().any(|entry| {
        entry.kind() == GitStatusEntryKind::Unmerged && entry.path().as_bytes() == b"conflict.txt"
    }));
}

#[test]
fn snapshot_identity_changes_when_native_repository_state_changes() {
    let repository = TempRepository::committed("identity");
    let inspector = repository.inspector();
    let clean = inspector.inspect_consistent().unwrap();
    assert!(clean.is_clean());

    fs::write(repository.root.join("tracked.txt"), b"changed\n").unwrap();
    let dirty = inspector.inspect_consistent().unwrap();
    assert_ne!(clean.identity(), dirty.identity());
    assert!(!dirty.is_clean());
}

#[test]
fn inspection_is_read_only_against_native_git_state() {
    let repository = TempRepository::committed("read-only");
    fs::write(repository.root.join("tracked.txt"), b"changed\n").unwrap();
    fs::write(repository.root.join("untracked.txt"), b"loose\n").unwrap();
    let before = git_ok(
        &repository.root,
        &["status", "--porcelain=v2", "--branch", "-z"],
    )
    .stdout;
    let head_before = git_ok(&repository.root, &["rev-parse", "HEAD"]).stdout;

    let _ = repository.inspector().inspect_consistent().unwrap();

    let after = git_ok(
        &repository.root,
        &["status", "--porcelain=v2", "--branch", "-z"],
    )
    .stdout;
    let head_after = git_ok(&repository.root, &["rev-parse", "HEAD"]).stdout;
    assert_eq!(before, after);
    assert_eq!(head_before, head_after);
}

#[test]
fn repository_change_between_capture_passes_is_rejected() {
    let repository = TempRepository::committed("race");
    let counter = repository.root.with_extension("git-count");
    let wrapper = repository.root.with_extension("git-wrapper");
    let script = format!(
        "#!/bin/sh\ncount=0\nif [ -f {counter} ]; then count=$(cat {counter}); fi\ncount=$((count + 1))\nprintf '%s' \"$count\" > {counter}\ngit \"$@\"\nstatus=$?\nif [ \"$count\" -eq 2 ]; then printf 'changed\\n' > tracked.txt; fi\nexit $status\n",
        counter = shell_quote(&counter),
    );
    publish_executable(&wrapper, script.as_bytes());

    let inspector =
        GitRepositoryInspector::with_program(repository_id(1), &repository.root, &wrapper).unwrap();
    let error = inspector.inspect_consistent().unwrap_err();
    assert!(matches!(
        error,
        GitRepositoryInspectionError::RepositoryChangedDuringInspection { .. }
    ));

    let _ = fs::remove_file(counter);
    let _ = fs::remove_file(wrapper);
}
