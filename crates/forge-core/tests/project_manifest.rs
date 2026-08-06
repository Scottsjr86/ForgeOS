use forge_core::projects::{
    AllowedProjectRoot, LanguageProfile, ManifestCommand, PROJECT_MANIFEST_RECORD_TYPE,
    PROJECT_MANIFEST_SCHEMA_VERSION, ProjectManifest, ProjectManifestError, ProjectSetting,
};
use forge_protocol::identities::{CommandId, ProjectId, RepositoryId};

fn id(byte: u8) -> [u8; 16] {
    [byte; 16]
}

fn manifest_with_orders(reverse: bool) -> ProjectManifest {
    let mut roots = vec![
        AllowedProjectRoot::relative("tests").unwrap(),
        AllowedProjectRoot::relative("src").unwrap(),
        AllowedProjectRoot::repository_root(),
    ];
    let mut commands = vec![
        ManifestCommand::new(CommandId::from_bytes(id(4)), "Test").unwrap(),
        ManifestCommand::new(CommandId::from_bytes(id(3)), "Check").unwrap(),
    ];
    let mut settings = vec![
        ProjectSetting::new("rust.edition", "2024").unwrap(),
        ProjectSetting::new("ui.tab_width", "4").unwrap(),
    ];
    if reverse {
        roots.reverse();
        commands.reverse();
        settings.reverse();
    }
    ProjectManifest::new(
        ProjectId::from_bytes(id(1)),
        RepositoryId::from_bytes(id(2)),
        "ForgeOS",
        roots,
        commands,
        LanguageProfile::Rust,
        settings,
    )
    .unwrap()
}

#[test]
fn canonical_bytes_ignore_input_collection_order_and_reopen_equivalently() {
    let first = manifest_with_orders(false);
    let reordered = manifest_with_orders(true);
    assert_eq!(first, reordered);
    assert_eq!(first.encode(), reordered.encode());

    let reopened = ProjectManifest::decode(&first.encode()).unwrap();
    assert_eq!(reopened, first);
    assert_eq!(reopened.encode(), first.encode());
}

#[test]
fn state_record_round_trip_preserves_manifest_type_and_bytes() {
    let manifest = manifest_with_orders(false);
    let record = manifest.to_state_record().unwrap();
    assert_eq!(record.record_type(), PROJECT_MANIFEST_RECORD_TYPE);
    assert_eq!(
        ProjectManifest::from_state_record(&record).unwrap(),
        manifest
    );
}

#[test]
fn exact_v1_header_and_required_field_count_are_golden_locked() {
    let bytes = manifest_with_orders(false).encode();
    assert_eq!(&bytes[..8], b"FGPROJ\0\0");
    assert_eq!(
        u16::from_be_bytes([bytes[8], bytes[9]]),
        PROJECT_MANIFEST_SCHEMA_VERSION
    );
    assert_eq!(u16::from_be_bytes([bytes[10], bytes[11]]), 7);
    assert_eq!(&bytes[12..14], &0x8001u16.to_be_bytes());
}

#[test]
fn exact_v1_manifest_bytes_are_golden_locked() {
    let bytes = manifest_with_orders(false).encode();
    let mut hex = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        use std::fmt::Write;
        write!(&mut hex, "{byte:02x}").unwrap();
    }
    assert_eq!(
        hex,
        "464750524f4a0000000100078001000000100101010101010101010101010101010180020000001002020202020202020202020202020202800300000007466f7267654f53800400000011000300010003737263010005746573747380050000002f0002030303030303030303030303030303030005436865636b040404040404040404040404040404040004546573748006000000010180070000002b0002000c727573742e65646974696f6e0000000432303234000c75692e7461625f77696474680000000134"
    );
}

#[test]
fn unsupported_schema_unknown_required_and_trailing_bytes_fail_closed() {
    let manifest = manifest_with_orders(false);
    let mut unsupported = manifest.encode();
    unsupported[8..10].copy_from_slice(&2u16.to_be_bytes());
    assert_eq!(
        ProjectManifest::decode(&unsupported),
        Err(ProjectManifestError::UnsupportedSchemaVersion { found: 2 })
    );

    let mut unknown = manifest.encode();
    unknown[10..12].copy_from_slice(&8u16.to_be_bytes());
    unknown.extend_from_slice(&0x8063u16.to_be_bytes());
    unknown.extend_from_slice(&0u32.to_be_bytes());
    assert_eq!(
        ProjectManifest::decode(&unknown),
        Err(ProjectManifestError::UnknownRequiredField { field: 99 })
    );

    let mut trailing = manifest.encode();
    trailing.push(0xff);
    assert_eq!(
        ProjectManifest::decode(&trailing),
        Err(ProjectManifestError::TrailingBytes { actual: 1 })
    );
}

#[test]
fn invalid_paths_and_duplicate_identifiers_are_rejected() {
    let duplicate_root = AllowedProjectRoot::relative("src").unwrap();
    assert!(matches!(
        ProjectManifest::new(
            ProjectId::from_bytes(id(1)),
            RepositoryId::from_bytes(id(2)),
            "ForgeOS",
            vec![duplicate_root.clone(), duplicate_root],
            Vec::new(),
            LanguageProfile::Rust,
            Vec::new(),
        ),
        Err(ProjectManifestError::DuplicateAllowedRoot(_))
    ));

    let command_id = CommandId::from_bytes(id(3));
    assert_eq!(
        ProjectManifest::new(
            ProjectId::from_bytes(id(1)),
            RepositoryId::from_bytes(id(2)),
            "ForgeOS",
            vec![AllowedProjectRoot::relative("src").unwrap()],
            vec![
                ManifestCommand::new(command_id, "Check").unwrap(),
                ManifestCommand::new(command_id, "Test").unwrap(),
            ],
            LanguageProfile::Rust,
            Vec::new(),
        ),
        Err(ProjectManifestError::DuplicateCommandId(command_id))
    );
}
