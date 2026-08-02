//! Git porcelain-v2 status parsing with raw path and state preservation.

use crate::types::{GitObjectId, GitPath, GitRefName};
use forge_protocol::identities::RepositoryId;

/// Attached branch bytes or explicit detached-head state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GitBranch {
    Attached(GitRefName),
    Detached,
}

/// Typed current branch and optional revision for an unborn repository.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GitHead {
    repository_id: RepositoryId,
    branch: GitBranch,
    revision: Option<GitObjectId>,
}

impl GitHead {
    pub const fn repository_id(&self) -> RepositoryId {
        self.repository_id
    }

    pub fn branch(&self) -> &GitBranch {
        &self.branch
    }

    pub fn revision(&self) -> Option<&GitObjectId> {
        self.revision.as_ref()
    }
}

/// Exact porcelain-v2 header retained without collapsing unknown native facts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GitStatusHeader {
    key: Vec<u8>,
    value: Vec<u8>,
}

impl GitStatusHeader {
    pub fn key(&self) -> &[u8] {
        &self.key
    }

    pub fn value(&self) -> &[u8] {
        &self.value
    }
}

/// Two-byte index/worktree status code from porcelain v2.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct GitStatusPair {
    index: u8,
    worktree: u8,
}

impl GitStatusPair {
    fn parse(bytes: &[u8]) -> Result<Self, String> {
        if bytes.len() != 2 || bytes.iter().any(|byte| !byte.is_ascii_graphic()) {
            return Err(format!(
                "Git status pair must contain two visible ASCII bytes; found {bytes:?}"
            ));
        }
        Ok(Self {
            index: bytes[0],
            worktree: bytes[1],
        })
    }

    pub const fn index(self) -> u8 {
        self.index
    }

    pub const fn worktree(self) -> u8 {
        self.worktree
    }
}

/// Porcelain-v2 record class.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GitStatusEntryKind {
    Ordinary,
    RenameOrCopy,
    Unmerged,
    Untracked,
    Ignored,
}

/// One typed status record with all unparsed machine tokens retained verbatim.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GitStatusEntry {
    kind: GitStatusEntryKind,
    status: Option<GitStatusPair>,
    path: GitPath,
    original_path: Option<GitPath>,
    metadata: Vec<Vec<u8>>,
    raw_record: Vec<u8>,
}

impl GitStatusEntry {
    pub const fn kind(&self) -> GitStatusEntryKind {
        self.kind
    }

    pub const fn status(&self) -> Option<GitStatusPair> {
        self.status
    }

    pub fn path(&self) -> &GitPath {
        &self.path
    }

    pub fn original_path(&self) -> Option<&GitPath> {
        self.original_path.as_ref()
    }

    /// Exact submodule, mode, object, and score tokens from native porcelain output.
    pub fn metadata_tokens(&self) -> &[Vec<u8>] {
        &self.metadata
    }

    pub fn raw_record(&self) -> &[u8] {
        &self.raw_record
    }
}

/// One complete branch/revision/status snapshot from a single native invocation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GitStatusSnapshot {
    repository_id: RepositoryId,
    head: GitHead,
    headers: Vec<GitStatusHeader>,
    entries: Vec<GitStatusEntry>,
}

impl GitStatusSnapshot {
    pub const fn repository_id(&self) -> RepositoryId {
        self.repository_id
    }

    pub fn head(&self) -> &GitHead {
        &self.head
    }

    pub fn into_head(self) -> GitHead {
        self.head
    }

    pub fn headers(&self) -> &[GitStatusHeader] {
        &self.headers
    }

    pub fn entries(&self) -> &[GitStatusEntry] {
        &self.entries
    }

    pub fn is_clean(&self) -> bool {
        self.entries.is_empty()
    }
}

pub(crate) fn parse_status(
    repository_id: RepositoryId,
    bytes: &[u8],
) -> Result<GitStatusSnapshot, String> {
    let records = nul_records(bytes)?;
    let mut headers = Vec::new();
    let mut entries = Vec::new();
    let mut branch_oid = None;
    let mut branch_head = None;
    let mut index = 0;

    while index < records.len() {
        let record = records[index];
        if record.starts_with(b"# ") {
            let header = parse_header(record)?;
            if header.key.as_slice() == b"branch.oid" {
                if branch_oid.replace(header.value.clone()).is_some() {
                    return Err("duplicate branch.oid header".to_owned());
                }
            }
            if header.key.as_slice() == b"branch.head" {
                if branch_head.replace(header.value.clone()).is_some() {
                    return Err("duplicate branch.head header".to_owned());
                }
            }
            headers.push(header);
            index += 1;
            continue;
        }

        let (entry, consumed) = parse_entry(&records[index..])?;
        entries.push(entry);
        index += consumed;
    }

    let oid = branch_oid.ok_or_else(|| "missing branch.oid header".to_owned())?;
    let head = branch_head.ok_or_else(|| "missing branch.head header".to_owned())?;
    let revision = if oid.as_slice() == b"(initial)" {
        None
    } else {
        Some(GitObjectId::parse(&oid)?)
    };
    let branch = if head.as_slice() == b"(detached)" {
        GitBranch::Detached
    } else {
        GitBranch::Attached(GitRefName::from_bytes(&head)?)
    };

    Ok(GitStatusSnapshot {
        repository_id,
        head: GitHead {
            repository_id,
            branch,
            revision,
        },
        headers,
        entries,
    })
}

fn parse_header(record: &[u8]) -> Result<GitStatusHeader, String> {
    let body = &record[2..];
    let Some(space) = body.iter().position(|byte| *byte == b' ') else {
        return Err("status header has no value separator".to_owned());
    };
    let key = &body[..space];
    let value = &body[space + 1..];
    if key.is_empty() || value.is_empty() {
        return Err("status header key and value must be nonempty".to_owned());
    }
    Ok(GitStatusHeader {
        key: key.to_vec(),
        value: value.to_vec(),
    })
}

fn parse_entry(records: &[&[u8]]) -> Result<(GitStatusEntry, usize), String> {
    let record = records
        .first()
        .copied()
        .ok_or_else(|| "missing status record".to_owned())?;
    match record.first().copied() {
        Some(b'1') => parse_tracked(record, GitStatusEntryKind::Ordinary, 9),
        Some(b'2') => {
            let original = records
                .get(1)
                .copied()
                .ok_or_else(|| "rename/copy status record is missing original path".to_owned())?;
            let (mut entry, _) =
                parse_tracked(record, GitStatusEntryKind::RenameOrCopy, 10)?;
            entry.original_path = Some(GitPath::from_bytes(original)?);
            Ok((entry, 2))
        }
        Some(b'u') => parse_tracked(record, GitStatusEntryKind::Unmerged, 11),
        Some(b'?') => parse_simple(record, GitStatusEntryKind::Untracked),
        Some(b'!') => parse_simple(record, GitStatusEntryKind::Ignored),
        Some(other) => Err(format!("unknown porcelain-v2 record tag 0x{other:02x}")),
        None => Err("empty porcelain-v2 record".to_owned()),
    }
}

fn parse_tracked(
    record: &[u8],
    kind: GitStatusEntryKind,
    expected_fields: usize,
) -> Result<(GitStatusEntry, usize), String> {
    let fields = split_fields(record, expected_fields)?;
    let status = GitStatusPair::parse(fields[1])?;
    let path = GitPath::from_bytes(fields[expected_fields - 1])?;
    Ok((
        GitStatusEntry {
            kind,
            status: Some(status),
            path,
            original_path: None,
            metadata: fields[2..expected_fields - 1]
                .iter()
                .map(|field| field.to_vec())
                .collect(),
            raw_record: record.to_vec(),
        },
        1,
    ))
}

fn parse_simple(
    record: &[u8],
    kind: GitStatusEntryKind,
) -> Result<(GitStatusEntry, usize), String> {
    if record.get(1) != Some(&b' ') {
        return Err("simple status record is missing its separator".to_owned());
    }
    Ok((
        GitStatusEntry {
            kind,
            status: None,
            path: GitPath::from_bytes(&record[2..])?,
            original_path: None,
            metadata: Vec::new(),
            raw_record: record.to_vec(),
        },
        1,
    ))
}

fn split_fields(record: &[u8], expected: usize) -> Result<Vec<&[u8]>, String> {
    let fields = record
        .splitn(expected, |byte| *byte == b' ')
        .collect::<Vec<_>>();
    if fields.len() != expected || fields.iter().any(|field| field.is_empty()) {
        return Err(format!(
            "status record expected {expected} nonempty fields; found {}",
            fields.len()
        ));
    }
    Ok(fields)
}

fn nul_records(bytes: &[u8]) -> Result<Vec<&[u8]>, String> {
    if bytes.is_empty() {
        return Err("porcelain-v2 status output is empty".to_owned());
    }
    if !bytes.ends_with(&[0]) {
        return Err("porcelain-v2 status output is not NUL terminated".to_owned());
    }
    Ok(bytes[..bytes.len() - 1]
        .split(|byte| *byte == 0)
        .collect())
}
