//! Forge Core dependency-purity verification.
//!
//! The V1 policy is deliberately default-deny. Forge Core may reach only its own
//! package and packages explicitly reviewed as pure protocol dependencies. Any
//! other direct or transitive production/build dependency is rejected until the
//! policy and its proof fixtures are intentionally updated.

use std::collections::BTreeSet;
use std::fmt;
use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;

/// The canonical package whose production dependency graph is guarded.
pub const CORE_PACKAGE: &str = "forge-core";

/// Packages currently reviewed as pure members of the Forge Core graph.
///
/// This is an exact allowlist rather than a substring denylist. New production
/// dependencies therefore fail closed, including generically named adapters that
/// would otherwise conceal effectful transitive packages.
pub const REVIEWED_PURE_PACKAGES: &[&str] = &["forge-core", "forge-protocol"];

/// One package rejected by the purity policy.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackageViolation {
    package: String,
}

impl PackageViolation {
    /// Package name reported by Cargo.
    pub fn package(&self) -> &str {
        &self.package
    }
}

/// Stable result of one Forge Core dependency graph inspection.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PurityReport {
    packages: Vec<String>,
    violations: Vec<PackageViolation>,
}

impl PurityReport {
    /// Every unique package reachable through normal or build dependencies.
    pub fn packages(&self) -> &[String] {
        &self.packages
    }

    /// Packages not present in the reviewed pure-package allowlist.
    pub fn violations(&self) -> &[PackageViolation] {
        &self.violations
    }

    /// Whether the inspected graph remains inside the reviewed pure boundary.
    pub fn is_pure(&self) -> bool {
        self.violations.is_empty()
    }

    /// Number of packages accepted by the policy.
    pub fn allowed_count(&self) -> usize {
        self.packages.len() - self.violations.len()
    }
}

/// Errors that prevent dependency graph classification.
#[derive(Debug)]
pub enum PurityError {
    InvalidRoot(PathBuf),
    MissingWorkspaceManifest(PathBuf),
    CargoInvocation { program: String, source: io::Error },
    CargoTreeFailure { status: Option<i32>, stderr: String },
    MalformedCargoTree(String),
}

impl fmt::Display for PurityError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidRoot(path) => {
                write!(
                    formatter,
                    "repository root is not a directory: {}",
                    path.display()
                )
            }
            Self::MissingWorkspaceManifest(path) => {
                write!(
                    formatter,
                    "workspace manifest does not exist: {}",
                    path.display()
                )
            }
            Self::CargoInvocation { program, source } => {
                write!(formatter, "failed to execute {program}: {source}")
            }
            Self::CargoTreeFailure { status, stderr } => write!(
                formatter,
                "cargo tree failed with status {}: {}",
                status
                    .map(|code| code.to_string())
                    .unwrap_or_else(|| "signal".to_owned()),
                stderr.trim()
            ),
            Self::MalformedCargoTree(message) => {
                write!(formatter, "cargo tree output is invalid: {message}")
            }
        }
    }
}

impl std::error::Error for PurityError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::CargoInvocation { source, .. } => Some(source),
            Self::InvalidRoot(_)
            | Self::MissingWorkspaceManifest(_)
            | Self::CargoTreeFailure { .. }
            | Self::MalformedCargoTree(_) => None,
        }
    }
}

/// Inspects the real Cargo production/build dependency graph for Forge Core.
pub fn inspect_core_dependencies(root: impl AsRef<Path>) -> Result<PurityReport, PurityError> {
    inspect_core_dependencies_with_cargo(root, "cargo")
}

/// Inspects Forge Core with an explicitly selected Cargo executable.
///
/// This entrypoint exists for controlled tool execution and tests. It does not
/// alter policy, package selection, dependency edges, or the exact allowlist.
pub fn inspect_core_dependencies_with_cargo(
    root: impl AsRef<Path>,
    cargo_program: impl AsRef<str>,
) -> Result<PurityReport, PurityError> {
    let root = root.as_ref();
    if !root.is_dir() {
        return Err(PurityError::InvalidRoot(root.to_path_buf()));
    }

    let manifest = root.join("Cargo.toml");
    if !manifest.is_file() {
        return Err(PurityError::MissingWorkspaceManifest(manifest));
    }

    let cargo_program = cargo_program.as_ref();
    let output = Command::new(cargo_program)
        .arg("tree")
        .arg("--manifest-path")
        .arg(&manifest)
        .arg("--package")
        .arg(CORE_PACKAGE)
        .arg("--edges")
        .arg("normal,build")
        .arg("--prefix")
        .arg("none")
        .arg("--format")
        .arg("{p}")
        .arg("--color")
        .arg("never")
        .output()
        .map_err(|source| PurityError::CargoInvocation {
            program: cargo_program.to_owned(),
            source,
        })?;

    if !output.status.success() {
        return Err(PurityError::CargoTreeFailure {
            status: output.status.code(),
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        });
    }

    classify_cargo_tree(&String::from_utf8_lossy(&output.stdout))
}

fn classify_cargo_tree(stdout: &str) -> Result<PurityReport, PurityError> {
    let mut packages = BTreeSet::new();

    for line in stdout
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
    {
        let package = line.split_whitespace().next().ok_or_else(|| {
            PurityError::MalformedCargoTree("package line has no package name".to_owned())
        })?;
        packages.insert(package.to_owned());
    }

    if !packages.contains(CORE_PACKAGE) {
        return Err(PurityError::MalformedCargoTree(format!(
            "missing guarded root package {CORE_PACKAGE}"
        )));
    }

    let packages: Vec<_> = packages.into_iter().collect();
    let violations = packages
        .iter()
        .filter(|package| {
            !REVIEWED_PURE_PACKAGES
                .iter()
                .any(|allowed| package.as_str() == *allowed)
        })
        .cloned()
        .map(|package| PackageViolation { package })
        .collect();

    Ok(PurityReport {
        packages,
        violations,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exact_reviewed_graph_passes() {
        let report = classify_cargo_tree(
            "forge-core v0.1.0 (/workspace/crates/forge-core)\n\
             forge-protocol v0.1.0 (/workspace/crates/forge-protocol)\n",
        )
        .expect("representative cargo tree should parse");

        assert!(report.is_pure());
        assert_eq!(report.allowed_count(), 2);
        assert!(report.violations().is_empty());
    }

    #[test]
    fn unknown_transitive_package_fails_closed() {
        let report = classify_cargo_tree(
            "forge-core v0.1.0 (/workspace/crates/forge-core)\n\
             generic-adapter v0.1.0 (/workspace/crates/generic-adapter)\n\
             forge-world v0.1.0 (/workspace/crates/forge-world)\n\
             forge-protocol v0.1.0 (/workspace/crates/forge-protocol)\n",
        )
        .expect("representative cargo tree should parse");

        assert!(!report.is_pure());
        assert_eq!(
            report
                .violations()
                .iter()
                .map(PackageViolation::package)
                .collect::<Vec<_>>(),
            vec!["forge-world", "generic-adapter"]
        );
    }

    #[test]
    fn missing_core_package_is_invalid_evidence() {
        let error = classify_cargo_tree("forge-protocol v0.1.0 (/workspace/protocol)\n")
            .expect_err("missing guarded root must be rejected");
        assert!(error.to_string().contains("missing guarded root package"));
    }
}
