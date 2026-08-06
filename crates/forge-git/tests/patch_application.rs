#![cfg(unix)]

use forge_git::patches::{GitPatchApplier, PatchApplyError};
use forge_git::repository::GitRepositoryInspector;
use forge_protocol::hashes::{HashDomain, hash_canonical_bytes};
use forge_protocol::identities::{IDENTITY_BYTES, PatchId, RepositoryId};
use forge_protocol::patches::{PatchBaseRevision, PatchEnvelope, PatchFileAction, PatchFileRecord};
use forge_protocol::paths::RepositoryRelativePath;
use std::fs;
use std::os::unix::fs::symlink;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_TEMP: AtomicU64 = AtomicU64::new(1);

fn patch_id(byte: u8) -> PatchId {
    PatchId::from_bytes([byte; IDENTITY_BYTES])
}

fn repository_id(byte: u8) -> RepositoryId {
    RepositoryId::from_bytes([byte; IDENTITY_BYTES])
}

fn file_hash(bytes: &[u8]) -> forge_protocol::hashes::ContentHash {
    hash_canonical_bytes(HashDomain::File, bytes)
}

struct TempRepository {
    root: PathBuf,
}

impl TempRepository {
    fn committed(label: &str) -> Self {
        let unique = NEXT_TEMP.fetch_add(1, Ordering::Relaxed);
        let root = std::env::temp_dir().join(format!(
            "forgeos-patch-100-{label}-{}-{unique}",
            std::process::id()
        ));
        fs::create_dir(&root).unwrap();
        git_ok(&root, &["init", "-q"]);
        git_ok(&root, &["symbolic-ref", "HEAD", "refs/heads/main"]);
        fs::write(root.join("tracked.txt"), b"old\n").unwrap();
        fs::write(root.join("other.txt"), b"other\n").unwrap();
        git_ok(&root, &["add", "--", "tracked.txt", "other.txt"]);
        git_ok(&root, &["commit", "-q", "-m", "initial"]);
        Self { root }
    }

    fn inspector(&self) -> GitRepositoryInspector {
        GitRepositoryInspector::open(repository_id(1), &self.root).unwrap()
    }

    fn head(&self) -> String {
        self.inspector()
            .inspect_head()
            .unwrap()
            .revision()
            .unwrap()
            .as_str()
            .to_owned()
    }

    fn envelope(&self, id: u8, files: Vec<PatchFileRecord>, bytes: &[u8]) -> PatchEnvelope {
        PatchEnvelope::build(
            patch_id(id),
            repository_id(1),
            PatchBaseRevision::parse(self.head()).unwrap(),
            files,
            bytes.to_vec(),
        )
        .unwrap()
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

fn relative(path: &str) -> RepositoryRelativePath {
    RepositoryRelativePath::new(path).unwrap()
}

fn modify_record(path: &str, before: &[u8], after: &[u8]) -> PatchFileRecord {
    PatchFileRecord::new(
        PatchFileAction::Modify,
        relative(path),
        Some(file_hash(before)),
        Some(file_hash(after)),
    )
    .unwrap()
}

fn add_record(path: &str, after: &[u8]) -> PatchFileRecord {
    PatchFileRecord::new(
        PatchFileAction::Add,
        relative(path),
        None,
        Some(file_hash(after)),
    )
    .unwrap()
}

fn delete_record(path: &str, before: &[u8]) -> PatchFileRecord {
    PatchFileRecord::new(
        PatchFileAction::Delete,
        relative(path),
        Some(file_hash(before)),
        None,
    )
    .unwrap()
}

const MODIFY_PATCH: &[u8] = b"diff --git a/tracked.txt b/tracked.txt\n--- a/tracked.txt\n+++ b/tracked.txt\n@@ -1 +1 @@\n-old\n+new\n";

#[test]
fn valid_patch_is_checked_applied_and_verified() {
    let repository = TempRepository::committed("valid");
    let envelope = repository.envelope(
        1,
        vec![modify_record("tracked.txt", b"old\n", b"new\n")],
        MODIFY_PATCH,
    );
    let outcome = GitPatchApplier::from_inspector(repository.inspector())
        .apply(&envelope)
        .unwrap();
    assert_eq!(
        fs::read(repository.root.join("tracked.txt")).unwrap(),
        b"new\n"
    );
    assert_eq!(
        fs::read(repository.root.join("other.txt")).unwrap(),
        b"other\n"
    );
    assert!(outcome.check_output().exit().success());
    assert!(outcome.apply_output().exit().success());
    assert_eq!(outcome.validation().patch_identity(), envelope.identity());
}

#[test]
fn validation_is_non_mutating_and_exposes_exact_metadata() {
    let repository = TempRepository::committed("validate");
    let envelope = repository.envelope(
        2,
        vec![modify_record("tracked.txt", b"old\n", b"new\n")],
        MODIFY_PATCH,
    );
    let validation = GitPatchApplier::from_inspector(repository.inspector())
        .validate(&envelope)
        .unwrap();
    assert_eq!(validation.patch_id(), patch_id(2));
    assert_eq!(validation.base_revision().as_str(), repository.head());
    assert_eq!(
        fs::read(repository.root.join("tracked.txt")).unwrap(),
        b"old\n"
    );
}

#[test]
fn base_revision_mismatch_rejects_before_native_apply() {
    let repository = TempRepository::committed("base-mismatch");
    let envelope = PatchEnvelope::build(
        patch_id(3),
        repository_id(1),
        PatchBaseRevision::parse("f".repeat(40)).unwrap(),
        vec![modify_record("tracked.txt", b"old\n", b"new\n")],
        MODIFY_PATCH.to_vec(),
    )
    .unwrap();
    let error = GitPatchApplier::from_inspector(repository.inspector())
        .apply(&envelope)
        .unwrap_err();
    assert!(matches!(error, PatchApplyError::BaseRevisionChanged { .. }));
    assert_eq!(
        fs::read(repository.root.join("tracked.txt")).unwrap(),
        b"old\n"
    );
}

#[test]
fn repository_identity_mismatch_rejects_before_file_access() {
    let repository = TempRepository::committed("repository-mismatch");
    let envelope = PatchEnvelope::build(
        patch_id(4),
        repository_id(9),
        PatchBaseRevision::parse(repository.head()).unwrap(),
        vec![modify_record("tracked.txt", b"old\n", b"new\n")],
        MODIFY_PATCH.to_vec(),
    )
    .unwrap();
    let error = GitPatchApplier::from_inspector(repository.inspector())
        .apply(&envelope)
        .unwrap_err();
    assert!(matches!(error, PatchApplyError::RepositoryMismatch { .. }));
}

#[test]
fn file_table_must_match_payload_paths_and_actions() {
    let repository = TempRepository::committed("table-mismatch");
    let envelope = repository.envelope(
        5,
        vec![modify_record("other.txt", b"other\n", b"new\n")],
        MODIFY_PATCH,
    );
    let error = GitPatchApplier::from_inspector(repository.inspector())
        .validate(&envelope)
        .unwrap_err();
    assert!(matches!(error, PatchApplyError::FileTableMismatch { .. }));
}

#[test]
fn traversal_and_quoted_paths_are_rejected() {
    let repository = TempRepository::committed("traversal");
    let patch = b"diff --git a/../outside b/../outside\n--- a/../outside\n+++ b/../outside\n@@ -1 +1 @@\n-old\n+new\n";
    let envelope = repository.envelope(
        6,
        vec![modify_record("tracked.txt", b"old\n", b"new\n")],
        patch,
    );
    let error = GitPatchApplier::from_inspector(repository.inspector())
        .validate(&envelope)
        .unwrap_err();
    assert!(matches!(error, PatchApplyError::MalformedPatch(_)));
}

#[test]
fn undeclared_traditional_second_diff_is_rejected() {
    let repository = TempRepository::committed("hidden-second-file");
    let patch = b"diff --git a/tracked.txt b/tracked.txt\n--- a/tracked.txt\n+++ b/tracked.txt\n@@ -1 +1 @@\n-old\n+new\n--- a/other.txt\n+++ b/other.txt\n@@ -1 +1 @@\n-other\n+hidden\n";
    let envelope = repository.envelope(
        7,
        vec![modify_record("tracked.txt", b"old\n", b"new\n")],
        patch,
    );
    let error = GitPatchApplier::from_inspector(repository.inspector())
        .validate(&envelope)
        .unwrap_err();
    assert!(matches!(error, PatchApplyError::MalformedPatch(_)));
    assert_eq!(
        fs::read(repository.root.join("tracked.txt")).unwrap(),
        b"old\n"
    );
    assert_eq!(
        fs::read(repository.root.join("other.txt")).unwrap(),
        b"other\n"
    );
}

#[test]
fn binary_markers_and_nul_bytes_are_rejected() {
    let repository = TempRepository::committed("binary");
    let patch = b"diff --git a/tracked.txt b/tracked.txt\nGIT binary patch\n\0";
    let envelope = repository.envelope(
        8,
        vec![modify_record("tracked.txt", b"old\n", b"new\n")],
        patch,
    );
    let error = GitPatchApplier::from_inspector(repository.inspector())
        .validate(&envelope)
        .unwrap_err();
    assert!(matches!(error, PatchApplyError::HiddenBinaryPatch));
}

#[test]
fn partially_applicable_patch_preserves_every_original_file() {
    let repository = TempRepository::committed("partial");
    let patch = b"diff --git a/tracked.txt b/tracked.txt\n--- a/tracked.txt\n+++ b/tracked.txt\n@@ -1 +1 @@\n-old\n+new\ndiff --git a/other.txt b/other.txt\n--- a/other.txt\n+++ b/other.txt\n@@ -1 +1 @@\n-wrong-context\n+changed\n";
    let envelope = repository.envelope(
        9,
        vec![
            modify_record("other.txt", b"other\n", b"changed\n"),
            modify_record("tracked.txt", b"old\n", b"new\n"),
        ],
        patch,
    );
    let error = GitPatchApplier::from_inspector(repository.inspector())
        .apply(&envelope)
        .unwrap_err();
    assert!(matches!(error, PatchApplyError::NativeCheckFailed { .. }));
    assert_eq!(
        fs::read(repository.root.join("tracked.txt")).unwrap(),
        b"old\n"
    );
    assert_eq!(
        fs::read(repository.root.join("other.txt")).unwrap(),
        b"other\n"
    );
}

#[test]
fn stale_before_hash_rejects_without_touching_worktree() {
    let repository = TempRepository::committed("stale-before");
    let envelope = repository.envelope(
        10,
        vec![modify_record("tracked.txt", b"different\n", b"new\n")],
        MODIFY_PATCH,
    );
    let error = GitPatchApplier::from_inspector(repository.inspector())
        .apply(&envelope)
        .unwrap_err();
    assert!(matches!(error, PatchApplyError::FileStateMismatch { .. }));
    assert_eq!(
        fs::read(repository.root.join("tracked.txt")).unwrap(),
        b"old\n"
    );
}

#[test]
fn wrong_after_hash_rolls_back_successful_native_apply() {
    let repository = TempRepository::committed("rollback-after");
    let envelope = repository.envelope(
        11,
        vec![modify_record("tracked.txt", b"old\n", b"not-new\n")],
        MODIFY_PATCH,
    );
    let error = GitPatchApplier::from_inspector(repository.inspector())
        .apply(&envelope)
        .unwrap_err();
    assert!(matches!(
        error,
        PatchApplyError::PostApplyVerificationFailed {
            rolled_back: true,
            ..
        }
    ));
    assert_eq!(
        fs::read(repository.root.join("tracked.txt")).unwrap(),
        b"old\n"
    );
}

#[test]
fn regular_file_addition_and_deletion_are_supported() {
    let repository = TempRepository::committed("add-delete");
    let add_patch = b"diff --git a/new.txt b/new.txt\nnew file mode 100644\n--- /dev/null\n+++ b/new.txt\n@@ -0,0 +1 @@\n+new\n";
    let add = repository.envelope(12, vec![add_record("new.txt", b"new\n")], add_patch);
    GitPatchApplier::from_inspector(repository.inspector())
        .apply(&add)
        .unwrap();
    assert_eq!(fs::read(repository.root.join("new.txt")).unwrap(), b"new\n");

    git_ok(&repository.root, &["add", "--", "new.txt"]);
    git_ok(&repository.root, &["commit", "-q", "-m", "add new"]);
    let delete_patch = b"diff --git a/new.txt b/new.txt\ndeleted file mode 100644\n--- a/new.txt\n+++ /dev/null\n@@ -1 +0,0 @@\n-new\n";
    let delete = repository.envelope(13, vec![delete_record("new.txt", b"new\n")], delete_patch);
    GitPatchApplier::from_inspector(repository.inspector())
        .apply(&delete)
        .unwrap();
    assert!(!repository.root.join("new.txt").exists());
}

#[test]
fn symlink_final_or_parent_component_is_rejected() {
    let repository = TempRepository::committed("symlink");
    symlink(
        repository.root.join("tracked.txt"),
        repository.root.join("alias.txt"),
    )
    .unwrap();
    let alias_patch = b"diff --git a/alias.txt b/alias.txt\n--- a/alias.txt\n+++ b/alias.txt\n@@ -1 +1 @@\n-old\n+new\n";
    let alias = repository.envelope(
        14,
        vec![modify_record("alias.txt", b"old\n", b"new\n")],
        alias_patch,
    );
    assert!(matches!(
        GitPatchApplier::from_inspector(repository.inspector()).validate(&alias),
        Err(PatchApplyError::PathSymlink(_))
    ));

    fs::create_dir(repository.root.join("real")).unwrap();
    fs::write(repository.root.join("real/file.txt"), b"old\n").unwrap();
    symlink(repository.root.join("real"), repository.root.join("link")).unwrap();
    let parent_patch = b"diff --git a/link/file.txt b/link/file.txt\n--- a/link/file.txt\n+++ b/link/file.txt\n@@ -1 +1 @@\n-old\n+new\n";
    let parent = repository.envelope(
        15,
        vec![modify_record("link/file.txt", b"old\n", b"new\n")],
        parent_patch,
    );
    assert!(matches!(
        GitPatchApplier::from_inspector(repository.inspector()).validate(&parent),
        Err(PatchApplyError::PathSymlink(_))
    ));
}

#[test]
fn concurrent_apply_lock_is_visible_and_non_destructive() {
    let repository = TempRepository::committed("lock");
    fs::write(repository.root.join(".forgeos-patch-apply.lock"), b"held\n").unwrap();
    let envelope = repository.envelope(
        16,
        vec![modify_record("tracked.txt", b"old\n", b"new\n")],
        MODIFY_PATCH,
    );
    let error = GitPatchApplier::from_inspector(repository.inspector())
        .apply(&envelope)
        .unwrap_err();
    assert!(matches!(error, PatchApplyError::ApplyAlreadyInProgress(_)));
    assert_eq!(
        fs::read(repository.root.join("tracked.txt")).unwrap(),
        b"old\n"
    );
}

#[test]
fn missing_git_is_a_typed_invocation_failure_and_lock_is_removed() {
    let repository = TempRepository::committed("missing-git");
    let envelope = repository.envelope(
        17,
        vec![modify_record("tracked.txt", b"old\n", b"new\n")],
        MODIFY_PATCH,
    );
    let error =
        GitPatchApplier::with_program(repository.inspector(), "/definitely/missing/forgeos-git")
            .apply(&envelope)
            .unwrap_err();
    assert!(matches!(error, PatchApplyError::NativeInvocation(_)));
    assert!(!repository.root.join(".forgeos-patch-apply.lock").exists());
    assert_eq!(
        fs::read(repository.root.join("tracked.txt")).unwrap(),
        b"old\n"
    );
}
