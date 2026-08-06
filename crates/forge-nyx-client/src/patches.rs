//! Nyx-side intake boundary for exact patch offers.
//!
//! Nyx may transport and propose patch bytes, but it cannot bless or apply them.
//! The incoming declaration is accepted only when payload and structured identities
//! match the shared protocol contract; native repository validation remains owned by
//! `forge-git`.

use forge_protocol::hashes::ContentHash;
use forge_protocol::identities::{PatchId, RepositoryId};
use forge_protocol::patches::{
    PatchBaseRevision, PatchContractError, PatchEnvelope, PatchFileRecord,
};

/// One transport-validated patch proposal from Nyx or a remote agent boundary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NyxPatchOffer {
    envelope: PatchEnvelope,
}

impl NyxPatchOffer {
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
        PatchEnvelope::receive(
            patch_id,
            repository_id,
            base_revision,
            files,
            bytes,
            declared_payload_hash,
            declared_identity,
        )
        .map(|envelope| Self { envelope })
    }

    pub fn envelope(&self) -> &PatchEnvelope {
        &self.envelope
    }

    pub fn into_envelope(self) -> PatchEnvelope {
        self.envelope
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use forge_protocol::hashes::{HashDomain, hash_canonical_bytes};
    use forge_protocol::identities::IDENTITY_BYTES;
    use forge_protocol::patches::{PatchFileAction, PatchFileRecord};
    use forge_protocol::paths::RepositoryRelativePath;

    fn patch_id(byte: u8) -> PatchId {
        PatchId::from_bytes([byte; IDENTITY_BYTES])
    }

    fn repository_id(byte: u8) -> RepositoryId {
        RepositoryId::from_bytes([byte; IDENTITY_BYTES])
    }

    #[test]
    fn exact_offer_round_trips_without_granting_apply_authority() {
        let file = PatchFileRecord::new(
            PatchFileAction::Modify,
            RepositoryRelativePath::new("src/lib.rs").unwrap(),
            Some(hash_canonical_bytes(HashDomain::File, b"old\n")),
            Some(hash_canonical_bytes(HashDomain::File, b"new\n")),
        )
        .unwrap();
        let local = PatchEnvelope::build(
            patch_id(1),
            repository_id(2),
            PatchBaseRevision::parse("a".repeat(40)).unwrap(),
            vec![file.clone()],
            b"patch\n".to_vec(),
        )
        .unwrap();
        let offer = NyxPatchOffer::receive(
            local.patch_id(),
            local.repository_id(),
            local.base_revision().clone(),
            vec![file],
            local.bytes().to_vec(),
            local.payload_hash(),
            local.identity(),
        )
        .unwrap();
        assert_eq!(offer.envelope().identity(), local.identity());
    }

    #[test]
    fn altered_transport_bytes_are_rejected_before_git_ownership() {
        let file = PatchFileRecord::new(
            PatchFileAction::Add,
            RepositoryRelativePath::new("new.txt").unwrap(),
            None,
            Some(hash_canonical_bytes(HashDomain::File, b"new\n")),
        )
        .unwrap();
        let local = PatchEnvelope::build(
            patch_id(3),
            repository_id(4),
            PatchBaseRevision::parse("b".repeat(40)).unwrap(),
            vec![file.clone()],
            b"patch\n".to_vec(),
        )
        .unwrap();
        let error = NyxPatchOffer::receive(
            local.patch_id(),
            local.repository_id(),
            local.base_revision().clone(),
            vec![file],
            b"tampered\n".to_vec(),
            local.payload_hash(),
            local.identity(),
        )
        .unwrap_err();
        assert!(matches!(
            error,
            PatchContractError::PayloadHashMismatch { .. }
        ));
    }
}
