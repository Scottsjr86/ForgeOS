//! Incremental Rust syntax parsing through Tree-sitter.
//!
//! This adapter owns only Tree-sitter parser state and syntax trees. Source bytes
//! remain owned by the editor buffer. Every update therefore receives both the
//! previously parsed bytes and the replacement bytes, verifies the previous
//! content identity, edits a cloned syntax tree, and commits the new tree only
//! after parsing succeeds.

use forge_protocol::hashes::{hash_canonical_bytes, ContentHash, HashDomain};
use std::fmt;
use tree_sitter::{InputEdit, Node, Parser, Point, Range, Tree};

/// Byte-oriented source position used by parser results.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SourcePoint {
    row: usize,
    column: usize,
}

impl SourcePoint {
    pub const fn new(row: usize, column: usize) -> Self {
        Self { row, column }
    }

    pub const fn row(self) -> usize {
        self.row
    }

    pub const fn column(self) -> usize {
        self.column
    }
}

impl From<Point> for SourcePoint {
    fn from(point: Point) -> Self {
        Self::new(point.row, point.column)
    }
}

/// Exact byte and point range in one parsed source version.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SourceRange {
    start_byte: usize,
    end_byte: usize,
    start_point: SourcePoint,
    end_point: SourcePoint,
}

impl SourceRange {
    pub const fn new(
        start_byte: usize,
        end_byte: usize,
        start_point: SourcePoint,
        end_point: SourcePoint,
    ) -> Self {
        Self {
            start_byte,
            end_byte,
            start_point,
            end_point,
        }
    }

    pub const fn start_byte(self) -> usize {
        self.start_byte
    }

    pub const fn end_byte(self) -> usize {
        self.end_byte
    }

    pub const fn start_point(self) -> SourcePoint {
        self.start_point
    }

    pub const fn end_point(self) -> SourcePoint {
        self.end_point
    }
}

impl From<Range> for SourceRange {
    fn from(range: Range) -> Self {
        Self::new(
            range.start_byte,
            range.end_byte,
            range.start_point.into(),
            range.end_point.into(),
        )
    }
}

/// One named syntax node exposed without retaining source bytes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SyntaxSpan {
    kind: String,
    range: SourceRange,
}

impl SyntaxSpan {
    fn from_node(node: Node<'_>) -> Self {
        Self {
            kind: node.kind().to_owned(),
            range: node.range().into(),
        }
    }

    pub fn kind(&self) -> &str {
        &self.kind
    }

    pub const fn range(&self) -> SourceRange {
        self.range
    }
}

/// Parser-reported syntax issue class.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SyntaxIssueKind {
    Error,
    Missing,
}

/// One explicit parser issue.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SyntaxIssue {
    kind: SyntaxIssueKind,
    node_kind: String,
    range: SourceRange,
}

impl SyntaxIssue {
    fn from_node(node: Node<'_>, kind: SyntaxIssueKind) -> Self {
        Self {
            kind,
            node_kind: node.kind().to_owned(),
            range: node.range().into(),
        }
    }

    pub const fn kind(&self) -> SyntaxIssueKind {
        self.kind
    }

    pub fn node_kind(&self) -> &str {
        &self.node_kind
    }

    pub const fn range(&self) -> SourceRange {
        self.range
    }
}

/// Whether a snapshot came from a full or incremental parse.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParseMode {
    Initial,
    Incremental,
}

/// Immutable syntax facts for one exact source identity.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RustSyntaxSnapshot {
    source_hash: ContentHash,
    source_len: usize,
    root_kind: String,
    mode: ParseMode,
    spans: Vec<SyntaxSpan>,
    issues: Vec<SyntaxIssue>,
    changed_ranges: Vec<SourceRange>,
}

impl RustSyntaxSnapshot {
    fn from_tree(
        tree: &Tree,
        source_hash: ContentHash,
        source_len: usize,
        mode: ParseMode,
        changed_ranges: Vec<SourceRange>,
    ) -> Self {
        let root = tree.root_node();
        let mut spans = Vec::new();
        let mut issues = Vec::new();
        collect_nodes(root, &mut spans, &mut issues);
        Self {
            source_hash,
            source_len,
            root_kind: root.kind().to_owned(),
            mode,
            spans,
            issues,
            changed_ranges,
        }
    }

    pub const fn source_hash(&self) -> ContentHash {
        self.source_hash
    }

    pub const fn source_len(&self) -> usize {
        self.source_len
    }

    pub fn root_kind(&self) -> &str {
        &self.root_kind
    }

    pub const fn mode(&self) -> ParseMode {
        self.mode
    }

    pub fn spans(&self) -> &[SyntaxSpan] {
        &self.spans
    }

    pub fn issues(&self) -> &[SyntaxIssue] {
        &self.issues
    }

    pub fn changed_ranges(&self) -> &[SourceRange] {
        &self.changed_ranges
    }

    pub fn has_errors(&self) -> bool {
        !self.issues.is_empty()
    }
}

/// Real Rust parser state for one source document.
pub struct RustSyntaxParser {
    tree: Tree,
    snapshot: RustSyntaxSnapshot,
}

impl RustSyntaxParser {
    /// Creates a parser and performs the first full parse.
    pub fn parse(source: &[u8]) -> Result<Self, RustParseError> {
        validate_utf8(source)?;
        let mut parser = configured_parser()?;
        let tree = parser
            .parse(source, None)
            .ok_or(RustParseError::Cancelled)?;
        let source_hash = source_identity(source);
        let full_range = SourceRange::new(
            0,
            source.len(),
            SourcePoint::new(0, 0),
            point_at(source, source.len()),
        );
        let snapshot = RustSyntaxSnapshot::from_tree(
            &tree,
            source_hash,
            source.len(),
            ParseMode::Initial,
            vec![full_range],
        );
        Ok(Self { tree, snapshot })
    }

    pub fn snapshot(&self) -> &RustSyntaxSnapshot {
        &self.snapshot
    }

    /// Incrementally reparses replacement bytes against the exact prior bytes.
    ///
    /// The currently committed tree is not mutated unless the replacement parse
    /// succeeds. A stale or mismatched old source therefore cannot poison parser
    /// state.
    pub fn prepare_update(
        &self,
        old_source: &[u8],
        new_source: &[u8],
    ) -> Result<Self, RustParseError> {
        validate_utf8(old_source)?;
        validate_utf8(new_source)?;
        let expected_hash = self.snapshot.source_hash();
        let actual_hash = source_identity(old_source);
        if old_source.len() != self.snapshot.source_len() || actual_hash != expected_hash {
            return Err(RustParseError::PreviousSourceMismatch {
                expected_hash,
                actual_hash,
                expected_len: self.snapshot.source_len(),
                actual_len: old_source.len(),
            });
        }
        if old_source == new_source {
            return Err(RustParseError::UnchangedSource);
        }

        let edit = incremental_edit(old_source, new_source);
        let mut edited_tree = self.tree.clone();
        edited_tree.edit(&edit);
        let mut parser = configured_parser()?;
        let new_tree = parser
            .parse(new_source, Some(&edited_tree))
            .ok_or(RustParseError::Cancelled)?;
        let changed_ranges = edited_tree
            .changed_ranges(&new_tree)
            .map(SourceRange::from)
            .collect();
        let snapshot = RustSyntaxSnapshot::from_tree(
            &new_tree,
            source_identity(new_source),
            new_source.len(),
            ParseMode::Incremental,
            changed_ranges,
        );
        Ok(Self {
            tree: new_tree,
            snapshot,
        })
    }

    pub fn update(
        &mut self,
        old_source: &[u8],
        new_source: &[u8],
    ) -> Result<&RustSyntaxSnapshot, RustParseError> {
        *self = self.prepare_update(old_source, new_source)?;
        Ok(&self.snapshot)
    }
}

fn configured_parser() -> Result<Parser, RustParseError> {
    let mut parser = Parser::new();
    let language: tree_sitter::Language = tree_sitter_rust::LANGUAGE.into();
    parser
        .set_language(&language)
        .map_err(|error| RustParseError::Language(error.to_string()))?;
    Ok(parser)
}

/// Exact reason Rust parsing could not produce a current snapshot.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RustParseError {
    InvalidUtf8 {
        valid_up_to: usize,
    },
    Language(String),
    Cancelled,
    UnchangedSource,
    PreviousSourceMismatch {
        expected_hash: ContentHash,
        actual_hash: ContentHash,
        expected_len: usize,
        actual_len: usize,
    },
}

impl fmt::Display for RustParseError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidUtf8 { valid_up_to } => {
                write!(formatter, "Rust source is not UTF-8 at byte {valid_up_to}")
            }
            Self::Language(message) => write!(formatter, "Rust grammar is incompatible: {message}"),
            Self::Cancelled => formatter.write_str("Tree-sitter did not produce a syntax tree"),
            Self::UnchangedSource => formatter.write_str("incremental parse source is unchanged"),
            Self::PreviousSourceMismatch {
                expected_len,
                actual_len,
                ..
            } => write!(
                formatter,
                "incremental parse previous source mismatch: expected {expected_len} bytes, got {actual_len}"
            ),
        }
    }
}

impl std::error::Error for RustParseError {}

fn source_identity(source: &[u8]) -> ContentHash {
    hash_canonical_bytes(HashDomain::File, source)
}

fn validate_utf8(source: &[u8]) -> Result<(), RustParseError> {
    std::str::from_utf8(source)
        .map(|_| ())
        .map_err(|error| RustParseError::InvalidUtf8 {
            valid_up_to: error.valid_up_to(),
        })
}

fn incremental_edit(old_source: &[u8], new_source: &[u8]) -> InputEdit {
    let prefix = common_prefix(old_source, new_source);
    let suffix = common_suffix(&old_source[prefix..], &new_source[prefix..]);
    let old_end_byte = old_source.len() - suffix;
    let new_end_byte = new_source.len() - suffix;
    InputEdit {
        start_byte: prefix,
        old_end_byte,
        new_end_byte,
        start_position: point_at(old_source, prefix).into(),
        old_end_position: point_at(old_source, old_end_byte).into(),
        new_end_position: point_at(new_source, new_end_byte).into(),
    }
}

fn common_prefix(left: &[u8], right: &[u8]) -> usize {
    left.iter()
        .zip(right)
        .take_while(|(left, right)| left == right)
        .count()
}

fn common_suffix(left: &[u8], right: &[u8]) -> usize {
    left.iter()
        .rev()
        .zip(right.iter().rev())
        .take_while(|(left, right)| left == right)
        .count()
}

fn point_at(source: &[u8], offset: usize) -> SourcePoint {
    let prefix = &source[..offset];
    let row = prefix.iter().filter(|byte| **byte == b'\n').count();
    let column = prefix
        .iter()
        .rposition(|byte| *byte == b'\n')
        .map_or(prefix.len(), |newline| prefix.len() - newline - 1);
    SourcePoint::new(row, column)
}

impl From<SourcePoint> for Point {
    fn from(point: SourcePoint) -> Self {
        Point::new(point.row(), point.column())
    }
}

fn collect_nodes(node: Node<'_>, spans: &mut Vec<SyntaxSpan>, issues: &mut Vec<SyntaxIssue>) {
    if node.is_named() {
        spans.push(SyntaxSpan::from_node(node));
    }
    if node.is_error() {
        issues.push(SyntaxIssue::from_node(node, SyntaxIssueKind::Error));
    }
    if node.is_missing() {
        issues.push(SyntaxIssue::from_node(node, SyntaxIssueKind::Missing));
    }

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        collect_nodes(child, spans, issues);
    }
}
