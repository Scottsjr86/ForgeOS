//! Stable patch transport contracts shared by Nyx intake and Git application.
//!
//! Patch bytes, declared base revision, repository identity, and the exact file
//! table are all part of one domain-separated identity. Filesystem effects and
//! native Git invocation remain outside this crate.

use crate::hashes::{
    hash_canonical_bytes, CanonicalHashInput, ContentHash, HashContractError, HashDomain,
};
use crate::identities::{PatchId, RepositoryId};
use crate::paths::RepositoryRelativePath;
use std::collections::BTreeSet;
use std::fmt;
#[cfg(test)]
use std::path::Path;

#[cfg(unix)]
use std::os::unix::ffi::OsStrExt;

const MAX_PATCH_BYTES: usize = 16 * 1024 * 1024;
const FILE_TABLE_MAGIC: &[u8; 16] = b"FORGEPATCHFILES\0";
const FILE_TABLE_VERSION: u8 = 1;

/// Exact lowercase SHA-1 or SHA-256 revision declared as the patch base.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PatchBaseRevision(String);

impl PatchBaseRevision {
    pub fn parse(value: impl Into<String>) -> Result<Self, PatchContractError> {
        let value = value.into();
        if matches!(value.len(), 40 | 64)
            && value
                .as_bytes()
                .iter()
                .all(|byte| matches!(byte, b'0'..=b'9' | b'a'..=b'f'))
        {
            Ok(Self(value))
        } else {
            Err(PatchContractError::InvalidBaseRevision(value))
        }
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Declared effect of one patch section.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
pub enum PatchFileAction {
    Add = 1,
    Modify = 2,
    Delete = 3,
}

impl PatchFileAction {
    pub const fn code(self) -> u8 {
        self as u8
    }
}

/// One exact path and before/after content contract.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PatchFileRecord {
    action: PatchFileAction,
    path: RepositoryRelativePath,
    before: Option<ContentHash>,
    after: Option<ContentHash>,
}

impl PatchFileRecord {
    pub fn new(
        action: PatchFileAction,
        path: RepositoryRelativePath,
        before: Option<ContentHash>,
        after: Option<ContentHash>,
    ) -> Result<Self, PatchContractError> {
        let valid = match action {
            PatchFileAction::Add => before.is_none() && after.is_some(),
            PatchFileAction::Modify => before.is_some() && after.is_some(),
            PatchFileAction::Delete => before.is_some() && after.is_none(),
        };
        if !valid {
            return Err(PatchContractError::InvalidFileHashes {
                action,
                path: path.as_path().to_path_buf(),
            });
        }
        Ok(Self {
            action,
            path,
            before,
            after,
        })
    }

    pub const fn action(&self) -> PatchFileAction {
        self.action
    }

    pub fn path(&self) -> &RepositoryRelativePath {
        &self.path
    }

    pub const fn before(&self) -> Option<ContentHash> {
        self.before
    }

    pub const fn after(&self) -> Option<ContentHash> {
        self.after
    }
}

/// Fully validated patch transport envelope.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PatchEnvelope {
    patch_id: PatchId,
    repository_id: RepositoryId,
    base_revision: PatchBaseRevision,
    files: Vec<PatchFileRecord>,
    payload_hash: ContentHash,
    identity: ContentHash,
    bytes: Vec<u8>,
}

impl PatchEnvelope {
    /// Builds a local envelope and computes both payload and structured identity.
    pub fn build(
        patch_id: PatchId,
        repository_id: RepositoryId,
        base_revision: PatchBaseRevision,
        files: Vec<PatchFileRecord>,
        bytes: Vec<u8>,
    ) -> Result<Self, PatchContractError> {
        let files = validate_files(files)?;
        validate_patch_bytes(&bytes)?;
        let payload_hash = hash_canonical_bytes(HashDomain::Patch, &bytes);
        let identity = structured_identity(
            patch_id,
            repository_id,
            &base_revision,
            &files,
            payload_hash,
        )?;
        Ok(Self {
            patch_id,
            repository_id,
            base_revision,
            files,
            payload_hash,
            identity,
            bytes,
        })
    }

    /// Receives an externally declared envelope and rejects metadata or byte drift.
    #[allow(clippy::too_many_arguments)]
    pub fn receive(
        patch_id: PatchId,
        repository_id: RepositoryId,
        base_revision: PatchBaseRevision,
        files: Vec<PatchFileRecord>,
        bytes: Vec<u8>,
        declared_payload_hash: ContentHash,
        declared_identity: ContentHash,
    ) -> Result<Self, PatchContractError> {
        let envelope = Self::build(patch_id, repository_id, base_revision, files, bytes)?;
        if envelope.payload_hash != declared_payload_hash {
            return Err(PatchContractError::PayloadHashMismatch {
                expected: declared_payload_hash,
                actual: envelope.payload_hash,
            });
        }
        if envelope.identity != declared_identity {
            return Err(PatchContractError::IdentityMismatch {
                expected: declared_identity,
                actual: envelope.identity,
            });
        }
        Ok(envelope)
    }

    pub const fn patch_id(&self) -> PatchId {
        self.patch_id
    }

    pub const fn repository_id(&self) -> RepositoryId {
        self.repository_id
    }

    pub fn base_revision(&self) -> &PatchBaseRevision {
        &self.base_revision
    }

    pub fn files(&self) -> &[PatchFileRecord] {
        &self.files
    }

    pub const fn payload_hash(&self) -> ContentHash {
        self.payload_hash
    }

    pub const fn identity(&self) -> ContentHash {
        self.identity
    }

    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }
}

/// Exact structural reason an incoming patch contract was rejected.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PatchContractError {
    EmptyPatch,
    PatchTooLarge {
        maximum: usize,
        actual: usize,
    },
    InvalidBaseRevision(String),
    InvalidFileHashes {
        action: PatchFileAction,
        path: std::path::PathBuf,
    },
    DuplicateFile(std::path::PathBuf),
    UnsupportedPathEncoding,
    PayloadHashMismatch {
        expected: ContentHash,
        actual: ContentHash,
    },
    IdentityMismatch {
        expected: ContentHash,
        actual: ContentHash,
    },
    HashContract(HashContractError),
}

impl fmt::Display for PatchContractError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyPatch => formatter.write_str("patch payload may not be empty"),
            Self::PatchTooLarge { maximum, actual } => write!(
                formatter,
                "patch payload exceeds {maximum} bytes; found {actual}"
            ),
            Self::InvalidBaseRevision(value) => write!(
                formatter,
                "patch base revision must be exact lowercase SHA-1 or SHA-256: {value}"
            ),
            Self::InvalidFileHashes { action, path } => write!(
                formatter,
                "patch file hashes do not match {action:?} semantics for {}",
                path.display()
            ),
            Self::DuplicateFile(path) => {
                write!(formatter, "patch file table repeats {}", path.display())
            }
            Self::UnsupportedPathEncoding => {
                formatter.write_str("patch file-table identity requires Unix path bytes")
            }
            Self::PayloadHashMismatch { expected, actual } => write!(
                formatter,
                "patch payload hash mismatch: expected {expected}, got {actual}"
            ),
            Self::IdentityMismatch { expected, actual } => write!(
                formatter,
                "patch structured identity mismatch: expected {expected}, got {actual}"
            ),
            Self::HashContract(error) => error.fmt(formatter),
        }
    }
}

impl std::error::Error for PatchContractError {}

impl From<HashContractError> for PatchContractError {
    fn from(error: HashContractError) -> Self {
        Self::HashContract(error)
    }
}

fn validate_patch_bytes(bytes: &[u8]) -> Result<(), PatchContractError> {
    if bytes.is_empty() {
        return Err(PatchContractError::EmptyPatch);
    }
    if bytes.len() > MAX_PATCH_BYTES {
        return Err(PatchContractError::PatchTooLarge {
            maximum: MAX_PATCH_BYTES,
            actual: bytes.len(),
        });
    }
    Ok(())
}

fn validate_files(
    mut files: Vec<PatchFileRecord>,
) -> Result<Vec<PatchFileRecord>, PatchContractError> {
    files.sort_by(|left, right| path_bytes(left.path()).cmp(path_bytes(right.path())));
    let mut seen = BTreeSet::new();
    for file in &files {
        let bytes = path_bytes(file.path()).to_vec();
        if !seen.insert(bytes) {
            return Err(PatchContractError::DuplicateFile(
                file.path().as_path().to_path_buf(),
            ));
        }
    }
    Ok(files)
}

fn structured_identity(
    patch_id: PatchId,
    repository_id: RepositoryId,
    base_revision: &PatchBaseRevision,
    files: &[PatchFileRecord],
    payload_hash: ContentHash,
) -> Result<ContentHash, PatchContractError> {
    let mut input = CanonicalHashInput::new(HashDomain::Patch);
    input.add_field("patch_id", patch_id.as_bytes().to_vec())?;
    input.add_field("repository_id", repository_id.as_bytes().to_vec())?;
    input.add_field("base_revision", base_revision.as_str().as_bytes().to_vec())?;
    input.add_field("file_table", file_table_bytes(files)?)?;
    input.add_field("payload_hash", payload_hash.as_bytes().to_vec())?;
    Ok(input.identity())
}

fn file_table_bytes(files: &[PatchFileRecord]) -> Result<Vec<u8>, PatchContractError> {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(FILE_TABLE_MAGIC);
    bytes.push(FILE_TABLE_VERSION);
    bytes.extend_from_slice(&(files.len() as u32).to_be_bytes());
    for file in files {
        let path = path_bytes(file.path());
        bytes.push(file.action().code());
        bytes.extend_from_slice(&(path.len() as u32).to_be_bytes());
        bytes.extend_from_slice(path);
        push_optional_hash(&mut bytes, file.before());
        push_optional_hash(&mut bytes, file.after());
    }
    Ok(bytes)
}

fn push_optional_hash(bytes: &mut Vec<u8>, hash: Option<ContentHash>) {
    match hash {
        Some(hash) => {
            bytes.push(1);
            bytes.extend_from_slice(hash.as_bytes());
        }
        None => bytes.push(0),
    }
}

#[cfg(unix)]
fn path_bytes(path: &RepositoryRelativePath) -> &[u8] {
    path.as_path().as_os_str().as_bytes()
}

#[cfg(not(unix))]
fn path_bytes(_path: &RepositoryRelativePath) -> &[u8] {
    &[]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hashes::hash_canonical_bytes;
    use crate::identities::IDENTITY_BYTES;

    fn patch_id(byte: u8) -> PatchId {
        PatchId::from_bytes([byte; IDENTITY_BYTES])
    }

    fn repository_id(byte: u8) -> RepositoryId {
        RepositoryId::from_bytes([byte; IDENTITY_BYTES])
    }

    fn file_hash(bytes: &[u8]) -> ContentHash {
        hash_canonical_bytes(HashDomain::File, bytes)
    }

    fn file(path: &str) -> PatchFileRecord {
        PatchFileRecord::new(
            PatchFileAction::Modify,
            RepositoryRelativePath::new(path).unwrap(),
            Some(file_hash(b"old\n")),
            Some(file_hash(b"new\n")),
        )
        .unwrap()
    }

    #[test]
    fn identity_is_stable_across_file_table_input_order() {
        let base = PatchBaseRevision::parse("1".repeat(40)).unwrap();
        let first = PatchEnvelope::build(
            patch_id(1),
            repository_id(2),
            base.clone(),
            vec![file("z.rs"), file("a.rs")],
            b"patch\n".to_vec(),
        )
        .unwrap();
        let second = PatchEnvelope::build(
            patch_id(1),
            repository_id(2),
            base,
            vec![file("a.rs"), file("z.rs")],
            b"patch\n".to_vec(),
        )
        .unwrap();
        assert_eq!(first.identity(), second.identity());
        assert_eq!(
            first.payload_hash().to_string(),
            "3e234da815d9f6d86d0dba7b0eaf0d0373456dcbb9d1b42ef7f62ce45c96c376"
        );
        assert_eq!(
            first.identity().to_string(),
            "a8e508c5f05d1153aac4feb09d7226152baf9bbd07740cea26dbbc4e2dda1a66"
        );
        assert_eq!(first.files()[0].path().as_path(), Path::new("a.rs"));
    }

    #[test]
    fn declared_payload_or_identity_drift_is_rejected() {
        let envelope = PatchEnvelope::build(
            patch_id(3),
            repository_id(4),
            PatchBaseRevision::parse("2".repeat(40)).unwrap(),
            vec![file("src/lib.rs")],
            b"patch\n".to_vec(),
        )
        .unwrap();
        let error = PatchEnvelope::receive(
            envelope.patch_id(),
            envelope.repository_id(),
            envelope.base_revision().clone(),
            envelope.files().to_vec(),
            b"changed\n".to_vec(),
            envelope.payload_hash(),
            envelope.identity(),
        )
        .unwrap_err();
        assert!(matches!(
            error,
            PatchContractError::PayloadHashMismatch { .. }
        ));
    }

    #[test]
    fn file_action_hash_semantics_and_duplicates_are_explicit() {
        let path = RepositoryRelativePath::new("src/lib.rs").unwrap();
        assert!(matches!(
            PatchFileRecord::new(
                PatchFileAction::Add,
                path.clone(),
                Some(file_hash(b"old")),
                None
            ),
            Err(PatchContractError::InvalidFileHashes { .. })
        ));
        let record = file("src/lib.rs");
        assert!(matches!(
            PatchEnvelope::build(
                patch_id(5),
                repository_id(6),
                PatchBaseRevision::parse("3".repeat(40)).unwrap(),
                vec![record.clone(), record],
                b"patch\n".to_vec(),
            ),
            Err(PatchContractError::DuplicateFile(_))
        ));
    }
}
