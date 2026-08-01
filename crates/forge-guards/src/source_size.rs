//! Authored Rust source-size verification.
//!
//! The verifier uses fixed repository conventions rather than a configurable
//! ignore list. Authored `.rs` files are scanned in stable path order, while
//! standard generated, vendored, build-output, and version-control directories
//! are excluded from authored-source accounting.

use std::fmt;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

/// The largest physical line count that remains comfortably within the guard.
pub const PASS_MAX_LINES: usize = 1_000;

/// The first physical line count that is considered a hard failure.
pub const FAIL_MIN_LINES: usize = 1_201;

const EXCLUDED_DIRECTORY_NAMES: &[&str] = &[
    ".git",
    "generated",
    "target",
    "third_party",
    "vendor",
    "vendored",
];

const GENERATED_MARKERS: &[&str] = &[
    "// @generated",
    "//! @generated",
    "/* @generated",
    "// Code generated",
    "//! Code generated",
    "// Automatically generated",
];

/// Classification for one authored Rust source module.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModuleStatus {
    Pass,
    Warn,
    Fail,
}

impl ModuleStatus {
    /// Stable machine-readable label used by the executable verifier.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Pass => "PASS",
            Self::Warn => "WARN",
            Self::Fail => "FAIL",
        }
    }
}

/// Result for one authored Rust source module.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModuleReport {
    relative_path: PathBuf,
    physical_lines: usize,
    status: ModuleStatus,
}

impl ModuleReport {
    pub fn relative_path(&self) -> &Path {
        &self.relative_path
    }

    pub const fn physical_lines(&self) -> usize {
        self.physical_lines
    }

    pub const fn status(&self) -> ModuleStatus {
        self.status
    }
}

/// Complete stable-order scan result.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScanReport {
    modules: Vec<ModuleReport>,
}

impl ScanReport {
    pub fn modules(&self) -> &[ModuleReport] {
        &self.modules
    }

    pub fn pass_count(&self) -> usize {
        self.count(ModuleStatus::Pass)
    }

    pub fn warning_count(&self) -> usize {
        self.count(ModuleStatus::Warn)
    }

    pub fn failure_count(&self) -> usize {
        self.count(ModuleStatus::Fail)
    }

    pub fn overall_status(&self) -> ModuleStatus {
        if self.failure_count() > 0 {
            ModuleStatus::Fail
        } else if self.warning_count() > 0 {
            ModuleStatus::Warn
        } else {
            ModuleStatus::Pass
        }
    }

    fn count(&self, status: ModuleStatus) -> usize {
        self.modules
            .iter()
            .filter(|module| module.status == status)
            .count()
    }
}

/// Structural scan failure. Source-size policy failures remain inside
/// [`ScanReport`] and are not represented by this error type.
#[derive(Debug)]
pub enum ScanError {
    InvalidRoot(PathBuf),
    SymlinkEncountered(PathBuf),
    Io { path: PathBuf, source: io::Error },
}

impl fmt::Display for ScanError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidRoot(path) => {
                write!(
                    formatter,
                    "scan root is not a directory: {}",
                    path.display()
                )
            }
            Self::SymlinkEncountered(path) => write!(
                formatter,
                "symlink encountered inside authored source root: {}",
                path.display()
            ),
            Self::Io { path, source } => {
                write!(formatter, "I/O error at {}: {source}", path.display())
            }
        }
    }
}

impl std::error::Error for ScanError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io { source, .. } => Some(source),
            Self::InvalidRoot(_) | Self::SymlinkEncountered(_) => None,
        }
    }
}

/// Classifies one physical line count against the V1 source-size contract.
pub const fn classify_line_count(physical_lines: usize) -> ModuleStatus {
    if physical_lines >= FAIL_MIN_LINES {
        ModuleStatus::Fail
    } else if physical_lines > PASS_MAX_LINES {
        ModuleStatus::Warn
    } else {
        ModuleStatus::Pass
    }
}

/// Scans every authored Rust source module below `root` in stable path order.
///
/// The scan follows no symlinks and accepts no runtime exclusion list. A
/// symlink within the scanned authored tree is an error rather than a silent
/// omission. Standard generated, vendored, build-output, and VCS directories
/// are excluded by fixed policy.
pub fn scan_authored_rust(root: impl AsRef<Path>) -> Result<ScanReport, ScanError> {
    let root = root.as_ref();
    if !root.is_dir() {
        return Err(ScanError::InvalidRoot(root.to_path_buf()));
    }

    let mut source_paths = Vec::new();
    collect_rust_sources(root, root, &mut source_paths)?;
    source_paths.sort();

    let mut modules = Vec::with_capacity(source_paths.len());
    for path in source_paths {
        let bytes = fs::read(&path).map_err(|source| ScanError::Io {
            path: path.clone(),
            source,
        })?;

        if has_generated_marker(&bytes) {
            continue;
        }

        let physical_lines = count_physical_lines(&bytes);
        let relative_path = path
            .strip_prefix(root)
            .expect("collected paths always remain below the scan root")
            .to_path_buf();

        modules.push(ModuleReport {
            relative_path,
            physical_lines,
            status: classify_line_count(physical_lines),
        });
    }

    Ok(ScanReport { modules })
}

fn collect_rust_sources(
    root: &Path,
    directory: &Path,
    source_paths: &mut Vec<PathBuf>,
) -> Result<(), ScanError> {
    let entries = fs::read_dir(directory).map_err(|source| ScanError::Io {
        path: directory.to_path_buf(),
        source,
    })?;

    let mut entries = entries
        .map(|entry| {
            entry.map_err(|source| ScanError::Io {
                path: directory.to_path_buf(),
                source,
            })
        })
        .collect::<Result<Vec<_>, _>>()?;
    entries.sort_by_key(|entry| entry.file_name());

    for entry in entries {
        let path = entry.path();
        let file_type = entry.file_type().map_err(|source| ScanError::Io {
            path: path.clone(),
            source,
        })?;

        if file_type.is_symlink() {
            let points_to_directory = fs::metadata(&path)
                .map(|metadata| metadata.is_dir())
                .unwrap_or(false);
            let is_rust_source = path.extension().is_some_and(|extension| extension == "rs");
            if points_to_directory || is_rust_source {
                return Err(ScanError::SymlinkEncountered(relative_or_full(root, &path)));
            }
            continue;
        }

        if file_type.is_dir() {
            if is_excluded_directory(&entry.file_name()) {
                continue;
            }
            collect_rust_sources(root, &path, source_paths)?;
        } else if file_type.is_file() && path.extension().is_some_and(|extension| extension == "rs")
        {
            source_paths.push(path);
        }
    }

    Ok(())
}

fn is_excluded_directory(name: &std::ffi::OsStr) -> bool {
    EXCLUDED_DIRECTORY_NAMES
        .iter()
        .any(|excluded| name == std::ffi::OsStr::new(excluded))
}

fn has_generated_marker(bytes: &[u8]) -> bool {
    let prefix_len = bytes.len().min(4_096);
    let prefix = String::from_utf8_lossy(&bytes[..prefix_len]);

    prefix.lines().take(8).any(|line| {
        let line = line.trim_start();
        GENERATED_MARKERS
            .iter()
            .any(|marker| line.starts_with(marker))
    })
}

fn count_physical_lines(bytes: &[u8]) -> usize {
    if bytes.is_empty() {
        return 0;
    }

    let newline_count = bytes.iter().filter(|byte| **byte == b'\n').count();
    if bytes.last() == Some(&b'\n') {
        newline_count
    } else {
        newline_count + 1
    }
}

fn relative_or_full(root: &Path, path: &Path) -> PathBuf {
    path.strip_prefix(root).unwrap_or(path).to_path_buf()
}

#[cfg(test)]
mod tests {
    use super::{classify_line_count, count_physical_lines, ModuleStatus};

    #[test]
    fn classifies_exact_contract_boundaries() {
        assert_eq!(classify_line_count(500), ModuleStatus::Pass);
        assert_eq!(classify_line_count(1_000), ModuleStatus::Pass);
        assert_eq!(classify_line_count(1_001), ModuleStatus::Warn);
        assert_eq!(classify_line_count(1_200), ModuleStatus::Warn);
        assert_eq!(classify_line_count(1_201), ModuleStatus::Fail);
    }

    #[test]
    fn counts_empty_terminated_and_unterminated_physical_lines() {
        assert_eq!(count_physical_lines(b""), 0);
        assert_eq!(count_physical_lines(b"one\n"), 1);
        assert_eq!(count_physical_lines(b"one\ntwo"), 2);
        assert_eq!(count_physical_lines(b"\n\n"), 2);
    }
}
