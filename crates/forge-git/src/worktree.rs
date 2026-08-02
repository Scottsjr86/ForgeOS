//! Git worktree-list porcelain parsing with raw native fields preserved.

use crate::types::{GitObjectId, GitPath, GitRefName};
use forge_protocol::identities::RepositoryId;

/// Checked-out state reported for one Git worktree.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GitWorktreeState {
    Branch(GitRefName),
    Detached,
    Bare,
}

/// Unknown or future porcelain field retained rather than discarded.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GitWorktreeAttribute {
    key: Vec<u8>,
    value: Vec<u8>,
}

impl GitWorktreeAttribute {
    pub fn key(&self) -> &[u8] {
        &self.key
    }

    pub fn value(&self) -> &[u8] {
        &self.value
    }
}

/// One native Git worktree entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GitWorktree {
    path: GitPath,
    head: GitObjectId,
    state: GitWorktreeState,
    locked_reason: Option<Vec<u8>>,
    prunable_reason: Option<Vec<u8>>,
    attributes: Vec<GitWorktreeAttribute>,
}

impl GitWorktree {
    pub fn path(&self) -> &GitPath {
        &self.path
    }

    pub fn head(&self) -> &GitObjectId {
        &self.head
    }

    pub fn state(&self) -> &GitWorktreeState {
        &self.state
    }

    pub fn locked_reason(&self) -> Option<&[u8]> {
        self.locked_reason.as_deref()
    }

    pub fn prunable_reason(&self) -> Option<&[u8]> {
        self.prunable_reason.as_deref()
    }

    pub fn attributes(&self) -> &[GitWorktreeAttribute] {
        &self.attributes
    }
}


/// Complete worktree listing bound to one stable repository identity.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GitWorktreeSnapshot {
    repository_id: RepositoryId,
    worktrees: Vec<GitWorktree>,
}

impl GitWorktreeSnapshot {
    pub(crate) fn new(repository_id: RepositoryId, worktrees: Vec<GitWorktree>) -> Self {
        Self {
            repository_id,
            worktrees,
        }
    }

    pub const fn repository_id(&self) -> RepositoryId {
        self.repository_id
    }

    pub fn worktrees(&self) -> &[GitWorktree] {
        &self.worktrees
    }
}

#[derive(Default)]
struct WorktreeBuilder {
    path: Option<GitPath>,
    head: Option<GitObjectId>,
    state: Option<GitWorktreeState>,
    locked_reason: Option<Vec<u8>>,
    prunable_reason: Option<Vec<u8>>,
    attributes: Vec<GitWorktreeAttribute>,
}

impl WorktreeBuilder {
    fn finish(self) -> Result<GitWorktree, String> {
        Ok(GitWorktree {
            path: self.path.ok_or_else(|| "worktree record is missing path".to_owned())?,
            head: self.head.ok_or_else(|| "worktree record is missing HEAD".to_owned())?,
            state: self
                .state
                .ok_or_else(|| "worktree record is missing branch/detached/bare state".to_owned())?,
            locked_reason: self.locked_reason,
            prunable_reason: self.prunable_reason,
            attributes: self.attributes,
        })
    }

    fn has_fields(&self) -> bool {
        self.path.is_some()
            || self.head.is_some()
            || self.state.is_some()
            || self.locked_reason.is_some()
            || self.prunable_reason.is_some()
            || !self.attributes.is_empty()
    }
}

pub(crate) fn parse_worktrees(bytes: &[u8]) -> Result<Vec<GitWorktree>, String> {
    if bytes.is_empty() || !bytes.ends_with(&[0, 0]) {
        return Err("worktree porcelain output must end with an empty NUL record".to_owned());
    }

    let mut builder = WorktreeBuilder::default();
    let mut worktrees = Vec::new();
    for record in bytes[..bytes.len() - 1].split(|byte| *byte == 0) {
        if record.is_empty() {
            if builder.has_fields() {
                worktrees.push(builder.finish()?);
                builder = WorktreeBuilder::default();
            }
            continue;
        }
        parse_field(record, &mut builder)?;
    }
    if builder.has_fields() {
        return Err("worktree porcelain output lacks a final record separator".to_owned());
    }
    if worktrees.is_empty() {
        return Err("worktree porcelain output contains no worktrees".to_owned());
    }
    worktrees.sort_by(|left, right| left.path.as_bytes().cmp(right.path.as_bytes()));
    Ok(worktrees)
}

fn parse_field(record: &[u8], builder: &mut WorktreeBuilder) -> Result<(), String> {
    let (key, value) = match record.iter().position(|byte| *byte == b' ') {
        Some(index) => (&record[..index], &record[index + 1..]),
        None => (record, &[][..]),
    };
    match key {
        b"worktree" => {
            set_once(&mut builder.path, GitPath::from_bytes(value)?, "worktree path")
        }
        b"HEAD" => set_once(&mut builder.head, GitObjectId::parse(value)?, "worktree HEAD"),
        b"branch" => set_once(
            &mut builder.state,
            GitWorktreeState::Branch(GitRefName::from_bytes(value)?),
            "worktree state",
        ),
        b"detached" => {
            require_empty(value, "detached")?;
            set_once(
                &mut builder.state,
                GitWorktreeState::Detached,
                "worktree state",
            )
        }
        b"bare" => {
            require_empty(value, "bare")?;
            set_once(&mut builder.state, GitWorktreeState::Bare, "worktree state")
        }
        b"locked" => set_once(
            &mut builder.locked_reason,
            value.to_vec(),
            "worktree locked reason",
        ),
        b"prunable" => set_once(
            &mut builder.prunable_reason,
            value.to_vec(),
            "worktree prunable reason",
        ),
        _ => {
            if key.is_empty() {
                return Err("worktree field key may not be empty".to_owned());
            }
            builder.attributes.push(GitWorktreeAttribute {
                key: key.to_vec(),
                value: value.to_vec(),
            });
            Ok(())
        }
    }
}

fn set_once<T>(slot: &mut Option<T>, value: T, field: &str) -> Result<(), String> {
    if slot.replace(value).is_some() {
        Err(format!("duplicate {field}"))
    } else {
        Ok(())
    }
}

fn require_empty(value: &[u8], field: &str) -> Result<(), String> {
    if value.is_empty() {
        Ok(())
    } else {
        Err(format!("{field} field unexpectedly contains a value"))
    }
}
