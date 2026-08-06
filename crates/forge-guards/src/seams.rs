//! Cross-subsystem dependency direction verification.
//!
//! The V1 policy guards the real Cargo normal/build reachability graph between
//! ForgeOS authority packages. External libraries may exist behind an owning
//! subsystem, but they may not create an undeclared path to another ForgeOS
//! subsystem. Unknown ForgeOS workspace packages fail closed until the reviewed
//! matrix and its negative fixtures are intentionally updated.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Every reviewed ForgeOS workspace package in the V1 authority graph.
pub const REVIEWED_SUBSYSTEM_PACKAGES: &[&str] = &[
    "forge-app",
    "forge-bridge",
    "forge-core",
    "forge-editor",
    "forge-git",
    "forge-guards",
    "forge-nyx-client",
    "forge-project",
    "forge-protocol",
    "forge-session",
    "forge-terminal",
    "forge-world",
];

const APP_REACHABILITY: &[&str] = &[
    "forge-app",
    "forge-bridge",
    "forge-core",
    "forge-editor",
    "forge-git",
    "forge-nyx-client",
    "forge-project",
    "forge-protocol",
    "forge-session",
    "forge-terminal",
    "forge-world",
];
const BRIDGE_REACHABILITY: &[&str] = &["forge-bridge", "forge-core", "forge-protocol"];
const CORE_REACHABILITY: &[&str] = &["forge-core", "forge-protocol"];
const EDITOR_REACHABILITY: &[&str] = &[
    "forge-bridge",
    "forge-core",
    "forge-editor",
    "forge-protocol",
];
const GIT_REACHABILITY: &[&str] = &["forge-bridge", "forge-core", "forge-git", "forge-protocol"];
const GUARDS_REACHABILITY: &[&str] = &["forge-guards"];
const NYX_REACHABILITY: &[&str] = &["forge-core", "forge-nyx-client", "forge-protocol"];
const PROJECT_REACHABILITY: &[&str] = &["forge-core", "forge-project", "forge-protocol"];
const PROTOCOL_REACHABILITY: &[&str] = &["forge-protocol"];
const SESSION_REACHABILITY: &[&str] = &["forge-core", "forge-protocol", "forge-session"];
const TERMINAL_REACHABILITY: &[&str] = &[
    "forge-bridge",
    "forge-core",
    "forge-protocol",
    "forge-terminal",
];
const WORLD_REACHABILITY: &[&str] = &["forge-core", "forge-protocol", "forge-world"];

/// One forbidden ForgeOS authority reachability relation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SeamViolation {
    root: String,
    target: String,
}

impl SeamViolation {
    /// Package from which the forbidden target is reachable.
    pub fn root(&self) -> &str {
        &self.root
    }

    /// ForgeOS authority package reached outside the reviewed matrix.
    pub fn target(&self) -> &str {
        &self.target
    }
}

/// Stable result of one workspace seam-direction inspection.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SeamReport {
    workspace_packages: Vec<String>,
    relations: Vec<SeamRelation>,
    violations: Vec<SeamViolation>,
}

impl SeamReport {
    /// Workspace packages discovered through Cargo.
    pub fn workspace_packages(&self) -> &[String] {
        &self.workspace_packages
    }

    /// Reachable reviewed relations, sorted by root and target.
    pub fn relations(&self) -> &[SeamRelation] {
        &self.relations
    }

    /// Relations rejected by the reviewed matrix.
    pub fn violations(&self) -> &[SeamViolation] {
        &self.violations
    }

    /// Whether every workspace package and reviewed reachability relation is legal.
    pub fn is_legal(&self) -> bool {
        self.violations.is_empty()
    }
}

/// One reachable relation between reviewed ForgeOS packages.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SeamRelation {
    root: String,
    target: String,
}

impl SeamRelation {
    /// Root package whose dependency graph was inspected.
    pub fn root(&self) -> &str {
        &self.root
    }

    /// Reviewed ForgeOS package reachable from the root.
    pub fn target(&self) -> &str {
        &self.target
    }
}

/// Errors that prevent seam-direction classification.
#[derive(Debug)]
pub enum SeamError {
    InvalidRoot(PathBuf),
    MissingWorkspaceManifest(PathBuf),
    CargoInvocation {
        program: String,
        source: io::Error,
    },
    CargoTreeFailure {
        package: String,
        status: Option<i32>,
        stderr: String,
    },
    MalformedCargoTree {
        package: String,
        message: String,
    },
    MissingReviewedPackages(Vec<String>),
}

impl fmt::Display for SeamError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidRoot(path) => write!(
                formatter,
                "repository root is not a directory: {}",
                path.display()
            ),
            Self::MissingWorkspaceManifest(path) => write!(
                formatter,
                "workspace manifest does not exist: {}",
                path.display()
            ),
            Self::CargoInvocation { program, source } => {
                write!(formatter, "failed to execute {program}: {source}")
            }
            Self::CargoTreeFailure {
                package,
                status,
                stderr,
            } => write!(
                formatter,
                "cargo tree failed for {package} with status {}: {}",
                status
                    .map(|code| code.to_string())
                    .unwrap_or_else(|| "signal".to_owned()),
                stderr.trim()
            ),
            Self::MalformedCargoTree { package, message } => {
                write!(
                    formatter,
                    "cargo tree output for {package} is invalid: {message}"
                )
            }
            Self::MissingReviewedPackages(packages) => write!(
                formatter,
                "workspace is missing reviewed ForgeOS packages: {}",
                packages.join(",")
            ),
        }
    }
}

impl std::error::Error for SeamError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::CargoInvocation { source, .. } => Some(source),
            Self::InvalidRoot(_)
            | Self::MissingWorkspaceManifest(_)
            | Self::CargoTreeFailure { .. }
            | Self::MalformedCargoTree { .. }
            | Self::MissingReviewedPackages(_) => None,
        }
    }
}

/// Inspects the real Cargo normal/build graph against the reviewed V1 seam matrix.
pub fn inspect_seam_directions(root: impl AsRef<Path>) -> Result<SeamReport, SeamError> {
    inspect_seam_directions_with_cargo(root, "cargo")
}

/// Inspects seam direction with an explicitly selected Cargo executable.
pub fn inspect_seam_directions_with_cargo(
    root: impl AsRef<Path>,
    cargo_program: impl AsRef<str>,
) -> Result<SeamReport, SeamError> {
    let root = root.as_ref();
    if !root.is_dir() {
        return Err(SeamError::InvalidRoot(root.to_path_buf()));
    }

    let manifest = root.join("Cargo.toml");
    if !manifest.is_file() {
        return Err(SeamError::MissingWorkspaceManifest(manifest));
    }

    let cargo_program = cargo_program.as_ref();
    let workspace_displays = cargo_tree_packages(
        &manifest,
        cargo_program,
        "workspace",
        &["--workspace", "--depth", "0"],
    )?;
    let workspace_by_name = index_workspace_packages(&workspace_displays)?;
    let workspace_packages: BTreeSet<_> = workspace_by_name.keys().cloned().collect();

    let reviewed: BTreeSet<_> = REVIEWED_SUBSYSTEM_PACKAGES
        .iter()
        .map(|package| (*package).to_owned())
        .collect();
    let missing: Vec<_> = reviewed.difference(&workspace_packages).cloned().collect();
    if !missing.is_empty() {
        return Err(SeamError::MissingReviewedPackages(missing));
    }

    let mut violations = workspace_packages
        .difference(&reviewed)
        .cloned()
        .map(|target| SeamViolation {
            root: "workspace".to_owned(),
            target,
        })
        .collect::<Vec<_>>();
    let mut relations = Vec::new();

    for root_package in REVIEWED_SUBSYSTEM_PACKAGES {
        let reachable = cargo_tree_packages(
            &manifest,
            cargo_program,
            root_package,
            &["--package", root_package],
        )?;
        let root_identity = workspace_by_name
            .get(*root_package)
            .expect("reviewed package presence was checked");
        if !reachable.contains(root_identity) {
            return Err(SeamError::MalformedCargoTree {
                package: (*root_package).to_owned(),
                message: "guarded root package identity is absent".to_owned(),
            });
        }

        for target in &reviewed {
            let target_identity = workspace_by_name
                .get(target)
                .expect("reviewed package presence was checked");
            if !reachable.contains(target_identity) {
                continue;
            }
            relations.push(SeamRelation {
                root: (*root_package).to_owned(),
                target: target.clone(),
            });
            if !reviewed_reachability(root_package).contains(&target.as_str()) {
                violations.push(SeamViolation {
                    root: (*root_package).to_owned(),
                    target: target.clone(),
                });
            }
        }
    }

    relations.sort_by(|left, right| {
        left.root
            .cmp(&right.root)
            .then_with(|| left.target.cmp(&right.target))
    });
    violations.sort_by(|left, right| {
        left.root
            .cmp(&right.root)
            .then_with(|| left.target.cmp(&right.target))
    });
    violations.dedup();

    Ok(SeamReport {
        workspace_packages: workspace_packages.into_iter().collect(),
        relations,
        violations,
    })
}

fn cargo_tree_packages(
    manifest: &Path,
    cargo_program: &str,
    label: &str,
    selection_arguments: &[&str],
) -> Result<BTreeSet<String>, SeamError> {
    let mut command = Command::new(cargo_program);
    command
        .arg("tree")
        .arg("--manifest-path")
        .arg(manifest)
        .args(selection_arguments)
        .arg("--all-features")
        .arg("--target")
        .arg("all")
        .arg("--edges")
        .arg("normal,build")
        .arg("--prefix")
        .arg("none")
        .arg("--format")
        .arg("{p}")
        .arg("--color")
        .arg("never");

    let output = command
        .output()
        .map_err(|source| SeamError::CargoInvocation {
            program: cargo_program.to_owned(),
            source,
        })?;
    if !output.status.success() {
        return Err(SeamError::CargoTreeFailure {
            package: label.to_owned(),
            status: output.status.code(),
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        });
    }

    parse_cargo_tree(label, &String::from_utf8_lossy(&output.stdout))
}

fn parse_cargo_tree(label: &str, stdout: &str) -> Result<BTreeSet<String>, SeamError> {
    let packages: BTreeSet<_> = stdout
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(|line| line.strip_suffix(" (*)").unwrap_or(line).to_owned())
        .collect();
    if packages.is_empty() {
        return Err(SeamError::MalformedCargoTree {
            package: label.to_owned(),
            message: "no packages were reported".to_owned(),
        });
    }
    if packages
        .iter()
        .any(|display| display.split_whitespace().next().is_none())
    {
        return Err(SeamError::MalformedCargoTree {
            package: label.to_owned(),
            message: "package line has no package name".to_owned(),
        });
    }
    Ok(packages)
}

fn index_workspace_packages(
    displays: &BTreeSet<String>,
) -> Result<BTreeMap<String, String>, SeamError> {
    let mut packages = BTreeMap::new();
    for display in displays {
        let name = display
            .split_whitespace()
            .next()
            .expect("non-empty package display was checked")
            .to_owned();
        if let Some(previous) = packages.insert(name.clone(), display.clone()) {
            if previous != *display {
                return Err(SeamError::MalformedCargoTree {
                    package: "workspace".to_owned(),
                    message: format!(
                        "duplicate workspace package name {name} has multiple identities"
                    ),
                });
            }
        }
    }
    Ok(packages)
}

fn reviewed_reachability(root: &str) -> &'static [&'static str] {
    match root {
        "forge-app" => APP_REACHABILITY,
        "forge-bridge" => BRIDGE_REACHABILITY,
        "forge-core" => CORE_REACHABILITY,
        "forge-editor" => EDITOR_REACHABILITY,
        "forge-git" => GIT_REACHABILITY,
        "forge-guards" => GUARDS_REACHABILITY,
        "forge-nyx-client" => NYX_REACHABILITY,
        "forge-project" => PROJECT_REACHABILITY,
        "forge-protocol" => PROTOCOL_REACHABILITY,
        "forge-session" => SESSION_REACHABILITY,
        "forge-terminal" => TERMINAL_REACHABILITY,
        "forge-world" => WORLD_REACHABILITY,
        _ => &[],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reviewed_matrix_covers_every_reviewed_package() {
        for package in REVIEWED_SUBSYSTEM_PACKAGES {
            let allowed = reviewed_reachability(package);
            assert!(allowed.contains(package), "{package} must reach itself");
            assert!(
                allowed
                    .iter()
                    .all(|target| REVIEWED_SUBSYSTEM_PACKAGES.contains(target))
            );
        }
    }

    #[test]
    fn cargo_tree_parser_sorts_and_deduplicates_packages() {
        let packages = parse_cargo_tree(
            "fixture",
            "forge-world v0.1.0 (/workspace/world)\n\
             forge-core v0.1.0 (/workspace/core)\n\
             forge-core v0.1.0 (/workspace/core) (*)\n",
        )
        .expect("representative Cargo output should parse");
        assert_eq!(
            packages.into_iter().collect::<Vec<_>>(),
            vec![
                "forge-core v0.1.0 (/workspace/core)".to_owned(),
                "forge-world v0.1.0 (/workspace/world)".to_owned(),
            ]
        );
    }

    #[test]
    fn empty_cargo_tree_is_invalid_evidence() {
        let error =
            parse_cargo_tree("fixture", "\n").expect_err("empty Cargo output must be rejected");
        assert!(error.to_string().contains("no packages were reported"));
    }
}
