#![cfg(unix)]

use forge_bridge::git_mutation::NativeGitMutationFailureStage;
use forge_git::mutation::{
    expected_file_state, staged_patch_identity, CommitRequest, CreateWorktreeRequest,
    ExpectedWorktreeState, GitBranchName, GitCommitIdentity, GitMutationError,
    GitMutationOperation, GitPathExpectation, GitRepositoryMutator,
    RemoveWorktreeConfirmation, RemoveWorktreeRequest, RestoreConfirmation, RestoreRequest,
    StageRequest, UnstageRequest,
};
use forge_git::repository::GitRepositoryInspector;
use forge_git::status::GitStatusEntryKind;
use forge_protocol::identities::{RepositoryId, IDENTITY_BYTES};
use forge_protocol::paths::RepositoryRelativePath;
use std::ffi::OsString;
use std::fs;
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
        fs::write(repository.root.join("other.txt"), b"other\n").unwrap();
        git_ok(&repository.root, &["add", "--", "tracked.txt", "other.txt"]);
        git_ok(&repository.root, &["commit", "-q", "-m", "initial"]);
        repository
    }

    fn empty(label: &str) -> Self {
        let unique = NEXT_TEMP.fetch_add(1, Ordering::Relaxed);
        let root = std::env::temp_dir().join(format!(
            "forgeos-git-101-{label}-{}-{unique}",
            std::process::id()
        ));
        fs::create_dir(&root).unwrap();
        git_ok(&root, &["init", "-q"]);
        git_ok(&root, &["symbolic-ref", "HEAD", "refs/heads/main"]);
        Self {
            root,
            extra_paths: Vec::new(),
        }
    }

    fn inspector(&self) -> GitRepositoryInspector {
        GitRepositoryInspector::open(repository_id(1), &self.root).unwrap()
    }

    fn mutator(&self) -> GitRepositoryMutator {
        GitRepositoryMutator::from_inspector(self.inspector())
    }

    fn head(&self) -> forge_git::types::GitObjectId {
        self.inspector()
            .inspect_head()
            .unwrap()
            .revision()
            .unwrap()
            .clone()
    }

    fn remember(&mut self, path: PathBuf) -> PathBuf {
        self.extra_paths.push(path.clone());
        path
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
        .env_clear()
        .env("PATH", std::env::var_os("PATH").unwrap_or_default())
        .env("GIT_TERMINAL_PROMPT", "0")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_AUTHOR_NAME", "ForgeOS Test")
        .env("GIT_AUTHOR_EMAIL", "forgeos@example.invalid")
        .env("GIT_COMMITTER_NAME", "ForgeOS Test")
        .env("GIT_COMMITTER_EMAIL", "forgeos@example.invalid")
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

fn expectation(root: &Path, path: &str) -> GitPathExpectation {
    GitPathExpectation::new(
        relative(path),
        expected_file_state(root.join(path)).unwrap(),
    )
}

fn status_entry<'a>(
    status: &'a forge_git::status::GitStatusSnapshot,
    path: &[u8],
) -> &'a forge_git::status::GitStatusEntry {
    status
        .entries()
        .iter()
        .find(|entry| entry.path().as_bytes() == path)
        .unwrap()
}

#[test]
fn stage_uses_exact_expected_path_and_reports_resulting_state() {
    let repository = TempRepository::committed("stage");
    fs::write(repository.root.join("tracked.txt"), b"changed\n").unwrap();
    let request = StageRequest::new(
        repository_id(1),
        Some(repository.head()),
        vec![expectation(&repository.root, "tracked.txt")],
    )
    .unwrap();
    let outcome = repository.mutator().stage(request).unwrap();
    assert_eq!(outcome.operation(), GitMutationOperation::Stage);
    assert!(outcome.native_exit().success());
    let entry = status_entry(outcome.status(), b"tracked.txt");
    assert_eq!(entry.status().unwrap().index(), b'M');
    assert_eq!(entry.status().unwrap().worktree(), b'.');
}

#[test]
fn duplicate_paths_are_rejected_before_git_runs() {
    let path = relative("tracked.txt");
    let state = ExpectedWorktreeState::Missing;
    let error = StageRequest::new(
        repository_id(1),
        None,
        vec![
            GitPathExpectation::new(path.clone(), state),
            GitPathExpectation::new(path, state),
        ],
    )
    .unwrap_err();
    assert!(matches!(error, GitMutationError::DuplicatePath(_)));
}

#[test]
fn changed_worktree_bytes_reject_stale_stage_without_index_mutation() {
    let repository = TempRepository::committed("stage-stale");
    let original = expectation(&repository.root, "tracked.txt");
    fs::write(repository.root.join("tracked.txt"), b"changed after selection\n").unwrap();
    let request = StageRequest::new(
        repository_id(1),
        Some(repository.head()),
        vec![original],
    )
    .unwrap();
    let error = repository.mutator().stage(request).unwrap_err();
    assert!(matches!(error, GitMutationError::WorktreePathChanged { .. }));
    assert!(repository
        .inspector()
        .inspect_diff(forge_git::diff::DiffScope::Staged)
        .unwrap()
        .is_empty());
}

#[test]
fn unstage_removes_only_explicit_literal_path() {
    let repository = TempRepository::committed("unstage");
    fs::write(repository.root.join("tracked.txt"), b"tracked changed\n").unwrap();
    fs::write(repository.root.join("other.txt"), b"other changed\n").unwrap();
    git_ok(&repository.root, &["add", "--", "tracked.txt", "other.txt"]);
    let inspector = repository.inspector();
    let request = UnstageRequest::new(
        repository_id(1),
        repository.head(),
        staged_patch_identity(&inspector).unwrap(),
        vec![relative("tracked.txt")],
    )
    .unwrap();
    let outcome = GitRepositoryMutator::from_inspector(inspector)
        .unstage(request)
        .unwrap();
    assert_eq!(status_entry(outcome.status(), b"tracked.txt").status().unwrap().index(), b'.');
    assert_eq!(status_entry(outcome.status(), b"other.txt").status().unwrap().index(), b'M');
}

#[test]
fn stale_staged_identity_rejects_unstage() {
    let repository = TempRepository::committed("unstage-stale");
    fs::write(repository.root.join("tracked.txt"), b"one\n").unwrap();
    git_ok(&repository.root, &["add", "--", "tracked.txt"]);
    let inspector = repository.inspector();
    let expected = staged_patch_identity(&inspector).unwrap();
    fs::write(repository.root.join("other.txt"), b"two\n").unwrap();
    git_ok(&repository.root, &["add", "--", "other.txt"]);
    let request = UnstageRequest::new(
        repository_id(1),
        repository.head(),
        expected,
        vec![relative("tracked.txt")],
    )
    .unwrap();
    assert!(matches!(
        GitRepositoryMutator::from_inspector(inspector)
            .unstage(request)
            .unwrap_err(),
        GitMutationError::StagedStateChanged { .. }
    ));
}

#[test]
fn confirmed_restore_discards_only_the_exact_selected_path() {
    let repository = TempRepository::committed("restore");
    fs::write(repository.root.join("tracked.txt"), b"discard me\n").unwrap();
    fs::write(repository.root.join("other.txt"), b"keep me\n").unwrap();
    let request = RestoreRequest::new(
        repository_id(1),
        repository.head(),
        vec![expectation(&repository.root, "tracked.txt")],
        RestoreConfirmation::DiscardExactPaths,
    )
    .unwrap();
    let outcome = repository.mutator().restore(request).unwrap();
    assert_eq!(fs::read(repository.root.join("tracked.txt")).unwrap(), b"first\n");
    assert_eq!(fs::read(repository.root.join("other.txt")).unwrap(), b"keep me\n");
    assert_eq!(outcome.operation(), GitMutationOperation::RestoreWorktree);
}

#[test]
fn stale_restore_expectation_preserves_changed_bytes() {
    let repository = TempRepository::committed("restore-stale");
    fs::write(repository.root.join("tracked.txt"), b"first edit\n").unwrap();
    let selected = expectation(&repository.root, "tracked.txt");
    fs::write(repository.root.join("tracked.txt"), b"newer edit\n").unwrap();
    let request = RestoreRequest::new(
        repository_id(1),
        repository.head(),
        vec![selected],
        RestoreConfirmation::DiscardExactPaths,
    )
    .unwrap();
    assert!(matches!(
        repository.mutator().restore(request).unwrap_err(),
        GitMutationError::WorktreePathChanged { .. }
    ));
    assert_eq!(fs::read(repository.root.join("tracked.txt")).unwrap(), b"newer edit\n");
}

#[test]
fn commit_binds_exact_head_and_staged_patch_identity() {
    let repository = TempRepository::committed("commit");
    fs::write(repository.root.join("tracked.txt"), b"committed change\n").unwrap();
    git_ok(&repository.root, &["add", "--", "tracked.txt"]);
    let inspector = repository.inspector();
    let old_head = repository.head();
    let request = CommitRequest::new(
        repository_id(1),
        Some(old_head.clone()),
        staged_patch_identity(&inspector).unwrap(),
        b"exact commit message\n".to_vec(),
        GitCommitIdentity::new("ForgeOS Operator", "operator@forgeos.invalid").unwrap(),
    )
    .unwrap();
    let outcome = GitRepositoryMutator::from_inspector(inspector)
        .commit(request)
        .unwrap();
    let new_head = outcome.status().head().revision().unwrap();
    assert_ne!(new_head, &old_head);
    assert!(outcome.status().is_clean());
    let message = git_ok(&repository.root, &["log", "-1", "--format=%B"]).stdout;
    assert_eq!(message, b"exact commit message\n\n");
}

#[test]
fn stale_index_rejects_commit_without_moving_head() {
    let repository = TempRepository::committed("commit-stale");
    fs::write(repository.root.join("tracked.txt"), b"first staged\n").unwrap();
    git_ok(&repository.root, &["add", "--", "tracked.txt"]);
    let inspector = repository.inspector();
    let expected = staged_patch_identity(&inspector).unwrap();
    let head = repository.head();
    fs::write(repository.root.join("other.txt"), b"second staged\n").unwrap();
    git_ok(&repository.root, &["add", "--", "other.txt"]);
    let request = CommitRequest::new(
        repository_id(1),
        Some(head.clone()),
        expected,
        b"must not commit".to_vec(),
        GitCommitIdentity::new("ForgeOS Operator", "operator@forgeos.invalid").unwrap(),
    )
    .unwrap();
    assert!(matches!(
        GitRepositoryMutator::from_inspector(inspector)
            .commit(request)
            .unwrap_err(),
        GitMutationError::StagedStateChanged { .. }
    ));
    assert_eq!(repository.head(), head);
}

#[test]
fn commit_does_not_execute_repository_hooks() {
    let repository = TempRepository::committed("commit-hooks");
    let hook = repository.root.join(".git/hooks/pre-commit");
    fs::write(&hook, b"#!/bin/sh\nexit 91\n").unwrap();
    let mut permissions = fs::metadata(&hook).unwrap().permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(&hook, permissions).unwrap();
    fs::write(repository.root.join("tracked.txt"), b"hookless\n").unwrap();
    git_ok(&repository.root, &["add", "--", "tracked.txt"]);
    let inspector = repository.inspector();
    let request = CommitRequest::new(
        repository_id(1),
        Some(repository.head()),
        staged_patch_identity(&inspector).unwrap(),
        b"hooks disabled".to_vec(),
        GitCommitIdentity::new("ForgeOS Operator", "operator@forgeos.invalid").unwrap(),
    )
    .unwrap();
    GitRepositoryMutator::from_inspector(inspector)
        .commit(request)
        .unwrap();
}

#[test]
fn create_worktree_uses_new_branch_without_checkout_or_force() {
    let mut repository = TempRepository::committed("worktree-create");
    let start = repository.head();
    let target_path = repository.root.with_extension("linked-create");
    let target = repository.remember(target_path);
    let request = CreateWorktreeRequest::new(
        repository_id(1),
        Some(start.clone()),
        target.clone(),
        GitBranchName::new("feature/exact-worktree").unwrap(),
        start.clone(),
    )
    .unwrap();
    let outcome = repository.mutator().create_worktree(request).unwrap();
    assert!(target.join(".git").exists());
    assert_eq!(fs::read(target.join("tracked.txt")).unwrap(), b"first\n");
    let linked = outcome
        .worktrees()
        .worktrees()
        .iter()
        .find(|worktree| worktree.path().as_bytes() == target.as_os_str().as_bytes())
        .unwrap();
    assert_eq!(linked.head(), &start);
    assert!(matches!(
        linked.state(),
        forge_git::worktree::GitWorktreeState::Branch(branch)
            if branch.as_bytes() == b"refs/heads/feature/exact-worktree"
    ));
}

#[test]
fn existing_worktree_target_is_rejected_before_git_runs() {
    let repository = TempRepository::committed("worktree-exists");
    let target = repository.root.with_extension("already-there");
    fs::create_dir(&target).unwrap();
    let request = CreateWorktreeRequest::new(
        repository_id(1),
        Some(repository.head()),
        target.clone(),
        GitBranchName::new("feature/existing-target").unwrap(),
        repository.head(),
    )
    .unwrap();
    assert!(matches!(
        repository.mutator().create_worktree(request).unwrap_err(),
        GitMutationError::WorktreeTargetExists(_)
    ));
    fs::remove_dir(&target).unwrap();
}

#[test]
fn clean_linked_worktree_can_be_removed_without_force() {
    let mut repository = TempRepository::committed("worktree-remove");
    let start = repository.head();
    let target_path = repository.root.with_extension("linked-remove");
    let target = repository.remember(target_path);
    let created = CreateWorktreeRequest::new(
        repository_id(1),
        Some(start.clone()),
        target.clone(),
        GitBranchName::new("feature/remove-clean").unwrap(),
        start.clone(),
    )
    .unwrap();
    repository.mutator().create_worktree(created).unwrap();
    let remove = RemoveWorktreeRequest::new(
        repository_id(1),
        target.clone(),
        start,
        RemoveWorktreeConfirmation::RemoveCleanLinkedWorktree,
    )
    .unwrap();
    let outcome = repository.mutator().remove_worktree(remove).unwrap();
    assert!(!target.exists());
    assert!(outcome
        .worktrees()
        .worktrees()
        .iter()
        .all(|worktree| worktree.path().as_bytes() != target.as_os_str().as_bytes()));
}

#[test]
fn dirty_linked_worktree_is_not_removed() {
    let mut repository = TempRepository::committed("worktree-dirty");
    let start = repository.head();
    let target_path = repository.root.with_extension("linked-dirty");
    let target = repository.remember(target_path);
    let create = CreateWorktreeRequest::new(
        repository_id(1),
        Some(start.clone()),
        target.clone(),
        GitBranchName::new("feature/remove-dirty").unwrap(),
        start.clone(),
    )
    .unwrap();
    repository.mutator().create_worktree(create).unwrap();
    fs::write(target.join("untracked.txt"), b"do not lose\n").unwrap();
    let remove = RemoveWorktreeRequest::new(
        repository_id(1),
        target.clone(),
        start,
        RemoveWorktreeConfirmation::RemoveCleanLinkedWorktree,
    )
    .unwrap();
    assert!(matches!(
        repository.mutator().remove_worktree(remove).unwrap_err(),
        GitMutationError::WorktreeNotClean(_)
    ));
    assert_eq!(fs::read(target.join("untracked.txt")).unwrap(), b"do not lose\n");
}

#[test]
fn primary_worktree_removal_is_rejected() {
    let repository = TempRepository::committed("primary-remove");
    let request = RemoveWorktreeRequest::new(
        repository_id(1),
        repository.root.clone(),
        repository.head(),
        RemoveWorktreeConfirmation::RemoveCleanLinkedWorktree,
    )
    .unwrap();
    assert!(matches!(
        repository.mutator().remove_worktree(request).unwrap_err(),
        GitMutationError::WorktreeIsPrimary(_)
    ));
}

#[test]
fn wrong_repository_identity_is_rejected_before_mutation() {
    let repository = TempRepository::committed("wrong-repository");
    fs::write(repository.root.join("tracked.txt"), b"changed\n").unwrap();
    let request = StageRequest::new(
        repository_id(9),
        Some(repository.head()),
        vec![expectation(&repository.root, "tracked.txt")],
    )
    .unwrap();
    assert!(matches!(
        repository.mutator().stage(request).unwrap_err(),
        GitMutationError::RepositoryMismatch { .. }
    ));
}

#[test]
fn missing_mutation_executable_is_a_typed_spawn_failure() {
    let repository = TempRepository::committed("missing-git");
    fs::write(repository.root.join("tracked.txt"), b"changed\n").unwrap();
    let inspector = repository.inspector();
    let mutator = GitRepositoryMutator::from_inspector_with_program(
        inspector,
        repository.root.join("missing-git-binary"),
    );
    let request = StageRequest::new(
        repository_id(1),
        Some(repository.head()),
        vec![expectation(&repository.root, "tracked.txt")],
    )
    .unwrap();
    match mutator.stage(request).unwrap_err() {
        GitMutationError::NativeInvocation(error) => {
            assert_eq!(error.stage(), NativeGitMutationFailureStage::Spawn)
        }
        other => panic!("unexpected error: {other:?}"),
    }
}

#[test]
fn native_pathspec_failure_preserves_unrelated_source_and_head() {
    let repository = TempRepository::committed("native-failure");
    let head = repository.head();
    let other = fs::read(repository.root.join("other.txt")).unwrap();
    let request = StageRequest::new(
        repository_id(1),
        Some(head.clone()),
        vec![GitPathExpectation::new(
            relative("missing.txt"),
            ExpectedWorktreeState::Missing,
        )],
    )
    .unwrap();
    match repository.mutator().stage(request).unwrap_err() {
        GitMutationError::NativeFailure(failure) => {
            assert_eq!(failure.operation(), GitMutationOperation::Stage);
            assert!(!failure.exit().success());
            assert!(!failure.stderr().is_empty());
        }
        other => panic!("unexpected error: {other:?}"),
    }
    assert_eq!(repository.head(), head);
    assert_eq!(fs::read(repository.root.join("other.txt")).unwrap(), other);
}

#[test]
fn non_utf8_path_is_staged_without_lossy_conversion() {
    let repository = TempRepository::committed("non-utf8");
    let raw = vec![b'n', b'o', b'n', b'-', 0xff, b'.', b't', b'x', b't'];
    let os = OsString::from_vec(raw.clone());
    let full = repository.root.join(&os);
    fs::write(&full, b"raw path\n").unwrap();
    let relative_path = RepositoryRelativePath::new(PathBuf::from(os)).unwrap();
    let request = StageRequest::new(
        repository_id(1),
        Some(repository.head()),
        vec![GitPathExpectation::new(
            relative_path,
            expected_file_state(&full).unwrap(),
        )],
    )
    .unwrap();
    let outcome = repository.mutator().stage(request).unwrap();
    let entry = outcome
        .status()
        .entries()
        .iter()
        .find(|entry| entry.kind() == GitStatusEntryKind::Ordinary && entry.path().as_bytes() == raw.as_slice())
        .unwrap();
    assert_eq!(entry.status().unwrap().index(), b'A');
}
