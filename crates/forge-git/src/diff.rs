//! Typed raw Git diff parsing paired with exact binary-safe patch bytes.

use crate::types::{GitObjectId, GitPath};
use forge_protocol::identities::RepositoryId;

/// Explicit read-only diff endpoints.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DiffScope {
    Worktree,
    Staged,
    Between {
        base: GitObjectId,
        target: GitObjectId,
    },
}

impl DiffScope {
    pub fn between(base: GitObjectId, target: GitObjectId) -> Self {
        Self::Between { base, target }
    }
}

/// Exact one-byte native raw-diff status and optional rename/copy score.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GitDiffStatus {
    code: u8,
    score: Option<u16>,
}

impl GitDiffStatus {
    pub const fn code(self) -> u8 {
        self.code
    }

    pub const fn score(self) -> Option<u16> {
        self.score
    }
}

/// One raw-diff record with modes, object IDs, status, and unquoted path bytes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GitDiffEntry {
    old_mode: String,
    new_mode: String,
    old_object: GitObjectId,
    new_object: GitObjectId,
    status: GitDiffStatus,
    source_path: GitPath,
    destination_path: Option<GitPath>,
}

impl GitDiffEntry {
    pub fn old_mode(&self) -> &str {
        &self.old_mode
    }

    pub fn new_mode(&self) -> &str {
        &self.new_mode
    }

    pub fn old_object(&self) -> &GitObjectId {
        &self.old_object
    }

    pub fn new_object(&self) -> &GitObjectId {
        &self.new_object
    }

    pub const fn status(&self) -> GitDiffStatus {
        self.status
    }

    pub fn source_path(&self) -> &GitPath {
        &self.source_path
    }

    pub fn destination_path(&self) -> Option<&GitPath> {
        self.destination_path.as_ref()
    }
}

/// One typed raw diff plus the exact native patch bytes for the same scope.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GitDiff {
    repository_id: RepositoryId,
    scope: DiffScope,
    entries: Vec<GitDiffEntry>,
    patch: Vec<u8>,
}

impl GitDiff {
    pub const fn repository_id(&self) -> RepositoryId {
        self.repository_id
    }

    pub fn scope(&self) -> &DiffScope {
        &self.scope
    }

    pub fn entries(&self) -> &[GitDiffEntry] {
        &self.entries
    }

    pub fn patch_bytes(&self) -> &[u8] {
        &self.patch
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty() && self.patch.is_empty()
    }
}

pub(crate) fn parse_diff(
    repository_id: RepositoryId,
    scope: DiffScope,
    raw: &[u8],
    patch: &[u8],
) -> Result<GitDiff, String> {
    let entries = parse_raw_entries(raw)?;
    if entries.is_empty() != patch.is_empty() {
        return Err("raw diff and patch disagree about whether changes exist".to_owned());
    }
    Ok(GitDiff {
        repository_id,
        scope,
        entries,
        patch: patch.to_vec(),
    })
}

fn parse_raw_entries(bytes: &[u8]) -> Result<Vec<GitDiffEntry>, String> {
    if bytes.is_empty() {
        return Ok(Vec::new());
    }
    if !bytes.ends_with(&[0]) {
        return Err("raw diff output is not NUL terminated".to_owned());
    }
    let tokens = bytes[..bytes.len() - 1]
        .split(|byte| *byte == 0)
        .collect::<Vec<_>>();
    let mut entries = Vec::new();
    let mut index = 0;
    while index < tokens.len() {
        let header = tokens[index];
        index += 1;
        let fields = header.split(|byte| *byte == b' ').collect::<Vec<_>>();
        if fields.len() != 5 || !fields[0].starts_with(b":") {
            return Err(format!("raw diff header is malformed: {header:?}"));
        }
        let old_mode = parse_mode(&fields[0][1..])?;
        let new_mode = parse_mode(fields[1])?;
        let old_object = GitObjectId::parse(fields[2])?;
        let new_object = GitObjectId::parse(fields[3])?;
        let status = parse_status(fields[4])?;
        let source = tokens
            .get(index)
            .copied()
            .ok_or_else(|| "raw diff record is missing source path".to_owned())?;
        index += 1;
        let destination_path = if matches!(status.code, b'R' | b'C') {
            let destination = tokens
                .get(index)
                .copied()
                .ok_or_else(|| "rename/copy raw diff is missing destination path".to_owned())?;
            index += 1;
            Some(GitPath::from_bytes(destination)?)
        } else {
            None
        };
        entries.push(GitDiffEntry {
            old_mode,
            new_mode,
            old_object,
            new_object,
            status,
            source_path: GitPath::from_bytes(source)?,
            destination_path,
        });
    }
    Ok(entries)
}

fn parse_mode(bytes: &[u8]) -> Result<String, String> {
    if bytes.len() != 6 || bytes.iter().any(|byte| !matches!(byte, b'0'..=b'7')) {
        return Err(format!("Git file mode is malformed: {bytes:?}"));
    }
    Ok(String::from_utf8(bytes.to_vec()).expect("validated octal bytes are UTF-8"))
}

fn parse_status(bytes: &[u8]) -> Result<GitDiffStatus, String> {
    let Some(code) = bytes.first().copied() else {
        return Err("raw diff status is empty".to_owned());
    };
    if !code.is_ascii_uppercase() {
        return Err(format!("raw diff status code is invalid: 0x{code:02x}"));
    }
    let score = if bytes.len() == 1 {
        None
    } else {
        let digits = &bytes[1..];
        if digits.iter().any(|byte| !byte.is_ascii_digit()) {
            return Err(format!(
                "raw diff similarity score is malformed: {digits:?}"
            ));
        }
        let score = String::from_utf8(digits.to_vec())
            .expect("validated decimal bytes are UTF-8")
            .parse::<u16>()
            .map_err(|_| "raw diff similarity score does not fit u16".to_owned())?;
        if score > 100 {
            return Err(format!("raw diff similarity score exceeds 100: {score}"));
        }
        Some(score)
    };
    if matches!(code, b'R' | b'C') != score.is_some() {
        return Err("only rename/copy raw diff records may carry a score".to_owned());
    }
    Ok(GitDiffStatus { code, score })
}
