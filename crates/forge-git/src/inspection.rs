//! Consistency-checked Git inspection snapshots.
//!
//! Native Git remains the source of branch, revision, status, and diff truth. This
//! module reads the complete inspection surface twice and accepts it only when the
//! two canonical identities agree, preventing ForgeOS from presenting a torn mix of
//! status and diff data captured across a concurrent repository change.

use crate::diff::{DiffScope, GitDiff};
use crate::repository::{GitInspectError, GitRepositoryInspector};
use crate::status::{GitBranch, GitStatusEntryKind, GitStatusSnapshot};
use forge_protocol::hashes::{CanonicalHashInput, ContentHash, HashDomain};
use forge_protocol::identities::RepositoryId;
use std::fmt;

const INSPECTION_MAGIC: &[u8; 12] = b"FGGITVIEW\0\0\0";
const INSPECTION_SCHEMA_VERSION: u8 = 1;

/// One stable read-only view of branch, revision, status, and both local diff scopes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GitInspectionSnapshot {
    repository_id: RepositoryId,
    status: GitStatusSnapshot,
    worktree_diff: GitDiff,
    staged_diff: GitDiff,
    identity: ContentHash,
}

impl GitInspectionSnapshot {
    pub const fn repository_id(&self) -> RepositoryId {
        self.repository_id
    }

    pub fn status(&self) -> &GitStatusSnapshot {
        &self.status
    }

    pub fn worktree_diff(&self) -> &GitDiff {
        &self.worktree_diff
    }

    pub fn staged_diff(&self) -> &GitDiff {
        &self.staged_diff
    }

    /// Stable SHA-256 identity of every canonical fact in this inspection view.
    pub const fn identity(&self) -> ContentHash {
        self.identity
    }

    pub fn is_clean(&self) -> bool {
        self.status.is_clean() && self.worktree_diff.is_empty() && self.staged_diff.is_empty()
    }

    fn new(
        status: GitStatusSnapshot,
        worktree_diff: GitDiff,
        staged_diff: GitDiff,
    ) -> Result<Self, GitRepositoryInspectionError> {
        let repository_id = status.repository_id();
        if worktree_diff.repository_id() != repository_id {
            return Err(GitRepositoryInspectionError::RepositoryMismatch {
                expected: repository_id,
                found: worktree_diff.repository_id(),
                surface: "worktree_diff",
            });
        }
        if staged_diff.repository_id() != repository_id {
            return Err(GitRepositoryInspectionError::RepositoryMismatch {
                expected: repository_id,
                found: staged_diff.repository_id(),
                surface: "staged_diff",
            });
        }
        if !matches!(worktree_diff.scope(), DiffScope::Worktree) {
            return Err(GitRepositoryInspectionError::UnexpectedDiffScope(
                "worktree_diff",
            ));
        }
        if !matches!(staged_diff.scope(), DiffScope::Staged) {
            return Err(GitRepositoryInspectionError::UnexpectedDiffScope(
                "staged_diff",
            ));
        }
        let identity = inspection_identity(&status, &worktree_diff, &staged_diff);
        Ok(Self {
            repository_id,
            status,
            worktree_diff,
            staged_diff,
            identity,
        })
    }
}

/// Exact reason a complete Git inspection snapshot could not be accepted.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GitRepositoryInspectionError {
    Inspect(GitInspectError),
    RepositoryMismatch {
        expected: RepositoryId,
        found: RepositoryId,
        surface: &'static str,
    },
    UnexpectedDiffScope(&'static str),
    RepositoryChangedDuringInspection {
        first: ContentHash,
        second: ContentHash,
    },
}

impl fmt::Display for GitRepositoryInspectionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Inspect(error) => error.fmt(formatter),
            Self::RepositoryMismatch {
                expected,
                found,
                surface,
            } => write!(
                formatter,
                "Git inspection {surface} belongs to repository {found}, expected {expected}"
            ),
            Self::UnexpectedDiffScope(surface) => {
                write!(
                    formatter,
                    "Git inspection {surface} has the wrong diff scope"
                )
            }
            Self::RepositoryChangedDuringInspection { first, second } => write!(
                formatter,
                "repository changed during Git inspection: first view {first}, second view {second}"
            ),
        }
    }
}

impl std::error::Error for GitRepositoryInspectionError {}

impl From<GitInspectError> for GitRepositoryInspectionError {
    fn from(error: GitInspectError) -> Self {
        Self::Inspect(error)
    }
}

impl GitRepositoryInspector {
    /// Captures status and both local diff scopes twice and accepts only one stable view.
    pub fn inspect_consistent(
        &self,
    ) -> Result<GitInspectionSnapshot, GitRepositoryInspectionError> {
        let first = self.capture_once()?;
        let second = self.capture_once()?;
        if first.identity() != second.identity() {
            return Err(
                GitRepositoryInspectionError::RepositoryChangedDuringInspection {
                    first: first.identity(),
                    second: second.identity(),
                },
            );
        }
        Ok(second)
    }

    fn capture_once(&self) -> Result<GitInspectionSnapshot, GitRepositoryInspectionError> {
        let status = self.inspect_status()?;
        let worktree_diff = self.inspect_diff(DiffScope::Worktree)?;
        let staged_diff = self.inspect_diff(DiffScope::Staged)?;
        GitInspectionSnapshot::new(status, worktree_diff, staged_diff)
    }
}

fn inspection_identity(
    status: &GitStatusSnapshot,
    worktree_diff: &GitDiff,
    staged_diff: &GitDiff,
) -> ContentHash {
    let mut input = CanonicalHashInput::new(HashDomain::Snapshot);
    input
        .add_field("repository_id", status.repository_id().as_bytes().to_vec())
        .expect("built-in inspection field is valid");
    input
        .add_field("status", encode_status(status))
        .expect("built-in inspection field is valid");
    input
        .add_field("worktree_diff", encode_diff(worktree_diff))
        .expect("built-in inspection field is valid");
    input
        .add_field("staged_diff", encode_diff(staged_diff))
        .expect("built-in inspection field is valid");
    input.identity()
}

fn encode_status(status: &GitStatusSnapshot) -> Vec<u8> {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(INSPECTION_MAGIC);
    bytes.push(INSPECTION_SCHEMA_VERSION);
    bytes.extend_from_slice(status.repository_id().as_bytes());

    match status.head().branch() {
        GitBranch::Attached(branch) => {
            bytes.push(1);
            push_bytes(&mut bytes, branch.as_bytes());
        }
        GitBranch::Detached => bytes.push(2),
    }
    match status.head().revision() {
        Some(revision) => {
            bytes.push(1);
            push_bytes(&mut bytes, revision.as_str().as_bytes());
        }
        None => bytes.push(0),
    }

    push_count(&mut bytes, status.headers().len());
    for header in status.headers() {
        push_bytes(&mut bytes, header.key());
        push_bytes(&mut bytes, header.value());
    }

    push_count(&mut bytes, status.entries().len());
    for entry in status.entries() {
        bytes.push(match entry.kind() {
            GitStatusEntryKind::Ordinary => 1,
            GitStatusEntryKind::RenameOrCopy => 2,
            GitStatusEntryKind::Unmerged => 3,
            GitStatusEntryKind::Untracked => 4,
            GitStatusEntryKind::Ignored => 5,
        });
        match entry.status() {
            Some(pair) => {
                bytes.push(1);
                bytes.push(pair.index());
                bytes.push(pair.worktree());
            }
            None => bytes.push(0),
        }
        push_bytes(&mut bytes, entry.path().as_bytes());
        match entry.original_path() {
            Some(path) => {
                bytes.push(1);
                push_bytes(&mut bytes, path.as_bytes());
            }
            None => bytes.push(0),
        }
        push_count(&mut bytes, entry.metadata_tokens().len());
        for token in entry.metadata_tokens() {
            push_bytes(&mut bytes, token);
        }
        push_bytes(&mut bytes, entry.raw_record());
    }
    bytes
}

fn encode_diff(diff: &GitDiff) -> Vec<u8> {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(INSPECTION_MAGIC);
    bytes.push(INSPECTION_SCHEMA_VERSION);
    bytes.extend_from_slice(diff.repository_id().as_bytes());
    match diff.scope() {
        DiffScope::Worktree => bytes.push(1),
        DiffScope::Staged => bytes.push(2),
        DiffScope::Between { base, target } => {
            bytes.push(3);
            push_bytes(&mut bytes, base.as_str().as_bytes());
            push_bytes(&mut bytes, target.as_str().as_bytes());
        }
    }
    push_count(&mut bytes, diff.entries().len());
    for entry in diff.entries() {
        push_bytes(&mut bytes, entry.old_mode().as_bytes());
        push_bytes(&mut bytes, entry.new_mode().as_bytes());
        push_bytes(&mut bytes, entry.old_object().as_str().as_bytes());
        push_bytes(&mut bytes, entry.new_object().as_str().as_bytes());
        bytes.push(entry.status().code());
        match entry.status().score() {
            Some(score) => {
                bytes.push(1);
                bytes.extend_from_slice(&score.to_be_bytes());
            }
            None => bytes.push(0),
        }
        push_bytes(&mut bytes, entry.source_path().as_bytes());
        match entry.destination_path() {
            Some(path) => {
                bytes.push(1);
                push_bytes(&mut bytes, path.as_bytes());
            }
            None => bytes.push(0),
        }
    }
    push_bytes(&mut bytes, diff.patch_bytes());
    bytes
}

fn push_count(bytes: &mut Vec<u8>, count: usize) {
    bytes.extend_from_slice(&(count as u64).to_be_bytes());
}

fn push_bytes(target: &mut Vec<u8>, value: &[u8]) {
    target.extend_from_slice(&(value.len() as u64).to_be_bytes());
    target.extend_from_slice(value);
}
