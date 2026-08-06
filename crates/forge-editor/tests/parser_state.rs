use forge_bridge::parsing::{ParseMode, RustParseError, SyntaxIssueKind};
use forge_editor::buffers::{BufferId, BufferRegistry, DiskVersion, DocumentKey, EditorBuffer};
use forge_editor::parsing::{BufferParseError, ParsedBuffer};
use forge_protocol::identities::RepositoryId;
use forge_protocol::paths::RepositoryRelativePath;

fn buffer_id(seed: u8) -> BufferId {
    BufferId::from_bytes([seed; 16])
}

fn repository_id(seed: u8) -> RepositoryId {
    RepositoryId::from_bytes([seed; 16])
}

fn document(repository_seed: u8, path: &str) -> DocumentKey {
    DocumentKey::new(
        repository_id(repository_seed),
        RepositoryRelativePath::new(path).expect("canonical fixture path"),
    )
}

fn open_buffer<'a>(
    registry: &'a mut BufferRegistry,
    id: BufferId,
    key: DocumentKey,
    bytes: &[u8],
) -> &'a mut EditorBuffer {
    registry
        .open_existing(id, key, DiskVersion::for_bytes(bytes), bytes.to_vec())
        .expect("fixture buffer opens");
    registry.get_mut(id).expect("fixture buffer exists")
}

#[test]
fn valid_rust_exposes_named_syntax_spans_for_exact_version() {
    let mut registry = BufferRegistry::new();
    let buffer = open_buffer(
        &mut registry,
        buffer_id(1),
        document(1, "src/lib.rs"),
        b"pub fn answer() -> u32 { 42 }\n",
    );
    let parsed = ParsedBuffer::parse(buffer).expect("valid Rust parses");
    let snapshot = parsed.snapshot_for(buffer).expect("snapshot is current");

    assert_eq!(snapshot.root_kind(), "source_file");
    assert_eq!(snapshot.mode(), ParseMode::Initial);
    assert!(!snapshot.has_errors());
    assert!(
        snapshot
            .spans()
            .iter()
            .any(|span| span.kind() == "function_item")
    );
    assert_eq!(snapshot.source_len(), buffer.bytes().len());
}

#[test]
fn invalid_rust_is_a_snapshot_with_explicit_issues_not_an_adapter_failure() {
    let mut registry = BufferRegistry::new();
    let buffer = open_buffer(
        &mut registry,
        buffer_id(1),
        document(1, "src/lib.rs"),
        b"pub fn broken( {\n",
    );
    let parsed = ParsedBuffer::parse(buffer).expect("Tree-sitter returns an error tree");
    let snapshot = parsed.snapshot_for(buffer).expect("snapshot is current");

    assert!(snapshot.has_errors());
    assert!(snapshot.issues().iter().any(|issue| {
        matches!(
            issue.kind(),
            SyntaxIssueKind::Error | SyntaxIssueKind::Missing
        )
    }));
}

#[test]
fn edited_buffer_makes_the_previous_snapshot_explicitly_stale() {
    let mut registry = BufferRegistry::new();
    let buffer = open_buffer(
        &mut registry,
        buffer_id(1),
        document(1, "src/lib.rs"),
        b"fn one() {}\n",
    );
    let parsed = ParsedBuffer::parse(buffer).expect("initial parse");
    let parsed_version = parsed.content_version();
    buffer
        .replace_range(3..6, b"two")
        .expect("buffer edit succeeds");

    assert!(matches!(
        parsed.snapshot_for(buffer),
        Err(BufferParseError::StaleSnapshot { parsed, current })
            if parsed == parsed_version && current == buffer.content_version()
    ));
}

#[test]
fn incremental_update_advances_to_the_exact_buffer_generation() {
    let mut registry = BufferRegistry::new();
    let buffer = open_buffer(
        &mut registry,
        buffer_id(1),
        document(1, "src/lib.rs"),
        b"fn one() {}\n",
    );
    let previous = buffer.bytes().to_vec();
    let mut parsed = ParsedBuffer::parse(buffer).expect("initial parse");
    buffer
        .replace_range(3..6, b"two")
        .expect("buffer edit succeeds");

    {
        let snapshot = parsed
            .update(buffer, &previous)
            .expect("incremental update succeeds");
        assert_eq!(snapshot.mode(), ParseMode::Incremental);
        assert!(
            snapshot
                .spans()
                .iter()
                .any(|span| span.kind() == "identifier")
        );
    }
    assert_eq!(parsed.content_version(), buffer.content_version());
    assert!(parsed.snapshot_for(buffer).is_ok());
}

#[test]
fn structural_edit_reports_changed_ranges() {
    let mut registry = BufferRegistry::new();
    let buffer = open_buffer(
        &mut registry,
        buffer_id(1),
        document(1, "src/lib.rs"),
        b"fn one() { 1 }\n",
    );
    let previous = buffer.bytes().to_vec();
    let mut parsed = ParsedBuffer::parse(buffer).expect("initial parse");
    buffer
        .replace_range(11..12, b"let x = 1;")
        .expect("structural edit succeeds");

    let snapshot = parsed
        .update(buffer, &previous)
        .expect("incremental update succeeds");
    assert!(!snapshot.changed_ranges().is_empty());
    assert!(
        snapshot
            .changed_ranges()
            .iter()
            .all(|range| range.start_byte() <= range.end_byte())
    );
}

#[test]
fn stale_previous_bytes_are_rejected_without_poisoning_parser_state() {
    let mut registry = BufferRegistry::new();
    let buffer = open_buffer(
        &mut registry,
        buffer_id(1),
        document(1, "src/lib.rs"),
        b"fn one() {}\n",
    );
    let previous = buffer.bytes().to_vec();
    let mut parsed = ParsedBuffer::parse(buffer).expect("initial parse");
    buffer
        .replace_range(3..6, b"two")
        .expect("buffer edit succeeds");

    assert!(matches!(
        parsed.update(buffer, b"fn stale() {}\n"),
        Err(BufferParseError::Parser(
            RustParseError::PreviousSourceMismatch { .. }
        ))
    ));
    assert_eq!(parsed.content_version().get(), 1);
    parsed
        .update(buffer, &previous)
        .expect("correct previous bytes still update the unpoisoned parser");
}

#[test]
fn parser_state_cannot_cross_buffer_identity() {
    let mut first_registry = BufferRegistry::new();
    let first = open_buffer(
        &mut first_registry,
        buffer_id(1),
        document(1, "src/lib.rs"),
        b"fn one() {}\n",
    );
    let parsed = ParsedBuffer::parse(first).expect("initial parse");

    let mut second_registry = BufferRegistry::new();
    let second = open_buffer(
        &mut second_registry,
        buffer_id(2),
        document(1, "src/lib.rs"),
        b"fn one() {}\n",
    );
    assert!(matches!(
        parsed.snapshot_for(second),
        Err(BufferParseError::BufferMismatch { expected, actual })
            if expected == buffer_id(1) && actual == buffer_id(2)
    ));
}

#[test]
fn parser_state_cannot_cross_document_identity() {
    let mut first_registry = BufferRegistry::new();
    let first = open_buffer(
        &mut first_registry,
        buffer_id(1),
        document(1, "src/lib.rs"),
        b"fn one() {}\n",
    );
    let parsed = ParsedBuffer::parse(first).expect("initial parse");

    let mut second_registry = BufferRegistry::new();
    let second = open_buffer(
        &mut second_registry,
        buffer_id(1),
        document(1, "src/main.rs"),
        b"fn one() {}\n",
    );
    assert_eq!(
        parsed.snapshot_for(second),
        Err(BufferParseError::DocumentMismatch)
    );
}

#[test]
fn non_utf8_parse_failure_does_not_block_plain_text_buffer_editing() {
    let mut registry = BufferRegistry::new();
    let buffer = open_buffer(
        &mut registry,
        buffer_id(1),
        document(1, "notes.bin"),
        &[0xff, b'a'],
    );
    assert_eq!(
        ParsedBuffer::parse(buffer).err(),
        Some(BufferParseError::Parser(RustParseError::InvalidUtf8 {
            valid_up_to: 0
        }))
    );

    buffer
        .replace_range(1..2, b"b")
        .expect("plain-text buffer operations remain available");
    assert_eq!(buffer.bytes(), &[0xff, b'b']);
}

#[test]
fn update_requires_a_strictly_newer_content_version() {
    let mut registry = BufferRegistry::new();
    let buffer = open_buffer(
        &mut registry,
        buffer_id(1),
        document(1, "src/lib.rs"),
        b"fn one() {}\n",
    );
    let mut parsed = ParsedBuffer::parse(buffer).expect("initial parse");

    assert!(matches!(
        parsed.update(buffer, buffer.bytes()),
        Err(BufferParseError::NonAdvancingVersion { parsed, requested })
            if parsed == requested && parsed == buffer.content_version()
    ));
}
