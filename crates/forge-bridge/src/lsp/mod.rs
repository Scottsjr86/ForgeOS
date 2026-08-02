//! Rust Analyzer process and JSON-RPC adapter.
//!
//! Public contracts, transport framing, and client orchestration are separated so
//! no layer quietly absorbs editor or process authority.

mod client;
mod protocol;
mod types;

pub use client::RustAnalyzerClient;
pub use types::{
    DocumentVersion, LspDiagnostic, LspDocument, LspError, LspPosition, LspProtocolError,
    LspRange, PublishedDiagnostics, RustAnalyzerCapabilities, RustAnalyzerConfig, TextDocumentSyncKind,
};
