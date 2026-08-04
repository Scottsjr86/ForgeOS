//! Exact read-only text search over a boundary-safe repository tree.
//!
//! Search consumes the safe file list from [`RepositoryBrowser`] and opens every
//! candidate through the existing FILE-100 raw read path. Matches use byte offsets
//! and byte columns so non-UTF-8 repository contents are never rewritten or
//! silently replaced.

use crate::files::ProjectFileError;
use crate::repository_view::{
    RepositoryBrowseError, RepositoryBrowseScope, RepositoryBrowser, RepositoryScanIssue,
};
use forge_protocol::paths::{RepositoryPathRequest, RepositoryRelativePath};
use std::fmt;

const DEFAULT_MAX_MATCHES: usize = 1_024;
const MAX_QUERY_BYTES: usize = 4_096;
const MAX_MATCHES: usize = 16_384;

/// Validated exact-text query and approved search scope.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextSearchQuery {
    scope: RepositoryBrowseScope,
    needle: String,
    maximum_matches: usize,
}

impl TextSearchQuery {
    pub fn new(needle: impl Into<String>) -> Result<Self, TextSearchQueryError> {
        Self::scoped(
            RepositoryBrowseScope::approved_roots(),
            needle,
            DEFAULT_MAX_MATCHES,
        )
    }

    pub fn scoped(
        scope: RepositoryBrowseScope,
        needle: impl Into<String>,
        maximum_matches: usize,
    ) -> Result<Self, TextSearchQueryError> {
        let needle = needle.into();
        if needle.is_empty() {
            return Err(TextSearchQueryError::Empty);
        }
        if needle.len() > MAX_QUERY_BYTES {
            return Err(TextSearchQueryError::TooLong {
                maximum: MAX_QUERY_BYTES,
                actual: needle.len(),
            });
        }
        if needle.as_bytes().contains(&0) {
            return Err(TextSearchQueryError::ContainsNul);
        }
        if needle.contains('\n') || needle.contains('\r') {
            return Err(TextSearchQueryError::ContainsLineBreak);
        }
        if maximum_matches == 0 || maximum_matches > MAX_MATCHES {
            return Err(TextSearchQueryError::InvalidMaximumMatches {
                minimum: 1,
                maximum: MAX_MATCHES,
                actual: maximum_matches,
            });
        }
        Ok(Self {
            scope,
            needle,
            maximum_matches,
        })
    }

    pub fn scope(&self) -> &RepositoryBrowseScope {
        &self.scope
    }

    pub fn needle(&self) -> &str {
        &self.needle
    }

    pub const fn maximum_matches(&self) -> usize {
        self.maximum_matches
    }
}

/// Exact reason a text query was rejected before filesystem access.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TextSearchQueryError {
    Empty,
    TooLong {
        maximum: usize,
        actual: usize,
    },
    ContainsNul,
    ContainsLineBreak,
    InvalidMaximumMatches {
        minimum: usize,
        maximum: usize,
        actual: usize,
    },
}

impl fmt::Display for TextSearchQueryError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => formatter.write_str("text search query must not be empty"),
            Self::TooLong { maximum, actual } => write!(
                formatter,
                "text search query exceeds {maximum} bytes: found {actual}"
            ),
            Self::ContainsNul => formatter.write_str("text search query contains NUL"),
            Self::ContainsLineBreak => {
                formatter.write_str("V1 text search query must fit on one line")
            }
            Self::InvalidMaximumMatches {
                minimum,
                maximum,
                actual,
            } => write!(
                formatter,
                "text search match limit must be between {minimum} and {maximum}: found {actual}"
            ),
        }
    }
}

impl std::error::Error for TextSearchQueryError {}

/// One exact occurrence in one safely opened repository file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextSearchMatch {
    relative_path: RepositoryRelativePath,
    byte_offset: usize,
    byte_length: usize,
    line_number: usize,
    byte_column: usize,
}

impl TextSearchMatch {
    pub fn relative_path(&self) -> &RepositoryRelativePath {
        &self.relative_path
    }

    pub const fn byte_offset(&self) -> usize {
        self.byte_offset
    }

    pub const fn byte_length(&self) -> usize {
        self.byte_length
    }

    pub const fn line_number(&self) -> usize {
        self.line_number
    }

    pub const fn byte_column(&self) -> usize {
        self.byte_column
    }
}

/// Explicit file or tree issue encountered while searching safe candidates.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RepositorySearchIssue {
    Scan(RepositoryScanIssue),
    File {
        relative_path: RepositoryRelativePath,
        error: ProjectFileError,
    },
}

/// Deterministic search result, including safe omissions and truncation state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextSearchReport {
    query: String,
    candidate_files: usize,
    scanned_files: usize,
    matches: Vec<TextSearchMatch>,
    issues: Vec<RepositorySearchIssue>,
    truncated: bool,
}

impl TextSearchReport {
    pub fn query(&self) -> &str {
        &self.query
    }

    pub const fn candidate_files(&self) -> usize {
        self.candidate_files
    }

    pub const fn scanned_files(&self) -> usize {
        self.scanned_files
    }

    pub fn matches(&self) -> &[TextSearchMatch] {
        &self.matches
    }

    pub fn issues(&self) -> &[RepositorySearchIssue] {
        &self.issues
    }

    pub const fn truncated(&self) -> bool {
        self.truncated
    }
}

impl RepositoryBrowser {
    /// Searches exact UTF-8 query bytes without mutating files or project records.
    pub fn search_text(
        &self,
        query: &TextSearchQuery,
    ) -> Result<TextSearchReport, RepositoryBrowseError> {
        let (files, scan_issues) = self.safe_file_paths(query.scope())?;
        let candidate_files = files.len();
        let mut issues: Vec<_> = scan_issues
            .into_iter()
            .map(RepositorySearchIssue::Scan)
            .collect();
        let mut matches = Vec::new();
        let mut scanned_files = 0;
        let mut truncated = false;

        for relative_path in files {
            let request = RepositoryPathRequest::new(self.repository_id(), relative_path.as_path())
                .expect("safe tree paths are already lexically valid");
            let snapshot = match self.open_file(&request) {
                Ok(snapshot) => snapshot,
                Err(error) => {
                    issues.push(RepositorySearchIssue::File {
                        relative_path,
                        error,
                    });
                    continue;
                }
            };
            scanned_files += 1;
            let remaining = query.maximum_matches().saturating_sub(matches.len());
            if collect_matches(
                snapshot.relative_path(),
                snapshot.bytes(),
                query.needle().as_bytes(),
                remaining,
                &mut matches,
            ) {
                truncated = true;
                break;
            }
        }

        Ok(TextSearchReport {
            query: query.needle().to_owned(),
            candidate_files,
            scanned_files,
            matches,
            issues,
            truncated,
        })
    }
}

fn collect_matches(
    relative_path: &RepositoryRelativePath,
    bytes: &[u8],
    needle: &[u8],
    maximum_new_matches: usize,
    matches: &mut Vec<TextSearchMatch>,
) -> bool {
    if needle.len() > bytes.len() {
        return false;
    }

    let initial_match_count = matches.len();
    let mut line_number = 1;
    let mut line_start = 0;
    for offset in 0..=bytes.len() - needle.len() {
        if &bytes[offset..offset + needle.len()] == needle {
            if matches.len() - initial_match_count >= maximum_new_matches {
                return true;
            }
            matches.push(TextSearchMatch {
                relative_path: relative_path.clone(),
                byte_offset: offset,
                byte_length: needle.len(),
                line_number,
                byte_column: offset - line_start,
            });
        }
        if bytes[offset] == b'\n' {
            line_number += 1;
            line_start = offset + 1;
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn query_validation_rejects_ambiguous_or_unbounded_requests() {
        assert_eq!(TextSearchQuery::new(""), Err(TextSearchQueryError::Empty));
        assert_eq!(
            TextSearchQuery::new("two\nlines"),
            Err(TextSearchQueryError::ContainsLineBreak)
        );
        assert_eq!(
            TextSearchQuery::scoped(RepositoryBrowseScope::approved_roots(), "x", 0),
            Err(TextSearchQueryError::InvalidMaximumMatches {
                minimum: 1,
                maximum: MAX_MATCHES,
                actual: 0,
            })
        );
    }

    #[test]
    fn exact_byte_offsets_and_line_columns_are_stable() {
        let path = RepositoryRelativePath::new(std::path::PathBuf::from("src/lib.rs")).unwrap();
        let mut matches = Vec::new();
        assert!(!collect_matches(
            &path,
            b"alpha needle\nbeta needle\n",
            b"needle",
            8,
            &mut matches,
        ));
        assert_eq!(matches.len(), 2);
        assert_eq!(matches[0].byte_offset(), 6);
        assert_eq!(matches[0].line_number(), 1);
        assert_eq!(matches[0].byte_column(), 6);
        assert_eq!(matches[1].line_number(), 2);
        assert_eq!(matches[1].byte_column(), 5);
    }
}
