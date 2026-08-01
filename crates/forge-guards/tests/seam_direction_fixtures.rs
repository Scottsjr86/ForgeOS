use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};

static FIXTURE_SEQUENCE: AtomicUsize = AtomicUsize::new(0);

#[test]
fn reviewed_workspace_graph_passes() {
    let mut fixture = Fixture::new("legal");
    fixture.add_reviewed_graph();
    fixture.write();

    let output = fixture.run_guard();
    let stdout = output.stdout_text();

    assert!(
        output.status.success(),
        "{stdout}\n{}",
        output.stderr_text()
    );
    assert!(
        stdout.contains(
            "FORGE_SEAM_DIRECTION_SUMMARY status=PASS packages=12 routes=42 forbidden=0 policy=exact-reviewed-subsystem-reachability-v1"
        ),
        "{stdout}"
    );
}

#[test]
fn representative_backward_seams_are_rejected() {
    for (root, target) in [
        ("forge-world", "forge-project"),
        ("forge-world", "forge-bridge"),
        ("forge-bridge", "forge-world"),
        ("forge-project", "forge-world"),
        ("forge-session", "forge-world"),
        ("forge-nyx-client", "forge-project"),
    ] {
        let mut fixture = Fixture::new(&format!("{root}-to-{target}"));
        fixture.add_reviewed_graph();
        fixture.add_dependency(root, target);
        fixture.write();

        let output = fixture.run_guard();
        let stdout = output.stdout_text();

        assert!(
            !output.status.success(),
            "{root} -> {target} passed\n{stdout}"
        );
        assert!(
            stdout.contains(&format!(
                "FORGE_SEAM_DIRECTION_ROUTE status=FORBIDDEN root={root} target={target}"
            )),
            "{stdout}"
        );
        assert!(
            stdout.contains("FORGE_SEAM_DIRECTION_SUMMARY status=FAIL"),
            "{stdout}"
        );
    }
}

#[test]
fn generic_adapter_cannot_smuggle_presentation_backward() {
    let mut fixture = Fixture::new("transitive-smuggle");
    fixture.add_reviewed_graph();
    fixture.add_external_package("generic-adapter", &["forge-world"]);
    fixture.add_dependency("forge-bridge", "generic-adapter");
    fixture.exclude("generic-adapter");
    fixture.write();

    let output = fixture.run_guard();
    let stdout = output.stdout_text();

    assert!(!output.status.success(), "{stdout}");
    assert!(
        stdout.contains(
            "FORGE_SEAM_DIRECTION_ROUTE status=FORBIDDEN root=forge-bridge target=forge-world"
        ),
        "{stdout}"
    );
}

#[test]
fn unknown_forgeos_workspace_package_fails_closed() {
    let mut fixture = Fixture::new("unknown-package");
    fixture.add_reviewed_graph();
    fixture.add_workspace_package("forge-shadow", &[]);
    fixture.write();

    let output = fixture.run_guard();
    let stdout = output.stdout_text();

    assert!(!output.status.success(), "{stdout}");
    assert!(
        stdout.contains("FORGE_SEAM_DIRECTION_PACKAGE status=FORBIDDEN package=forge-shadow"),
        "{stdout}"
    );
}

#[test]
fn missing_reviewed_package_is_invalid_evidence() {
    let mut fixture = Fixture::new("missing-package");
    fixture.add_reviewed_graph();
    fixture.remove_workspace_package("forge-world");
    fixture.write();

    let output = fixture.run_guard();
    let stderr = output.stderr_text();

    assert!(!output.status.success(), "{}", output.stdout_text());
    assert!(
        stderr.contains("workspace is missing reviewed ForgeOS packages: forge-world"),
        "{stderr}"
    );
}

struct Package {
    dependencies: Vec<String>,
    workspace_member: bool,
}

struct Fixture {
    path: PathBuf,
    packages: BTreeMap<String, Package>,
    excluded: Vec<String>,
}

impl Fixture {
    fn new(label: &str) -> Self {
        let sequence = FIXTURE_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "forge-seam-direction-{}-{sequence}-{label}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).expect("fixture root should be created");
        Self {
            path,
            packages: BTreeMap::new(),
            excluded: Vec::new(),
        }
    }

    fn add_reviewed_graph(&mut self) {
        self.add_workspace_package(
            "forge-app",
            &[
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
            ],
        );
        self.add_workspace_package("forge-bridge", &["forge-core", "forge-protocol"]);
        self.add_workspace_package("forge-core", &["forge-protocol"]);
        self.add_workspace_package(
            "forge-editor",
            &["forge-bridge", "forge-core", "forge-protocol"],
        );
        self.add_workspace_package(
            "forge-git",
            &["forge-bridge", "forge-core", "forge-protocol"],
        );
        self.add_workspace_package("forge-guards", &[]);
        self.add_workspace_package("forge-nyx-client", &["forge-core", "forge-protocol"]);
        self.add_workspace_package("forge-project", &["forge-core", "forge-protocol"]);
        self.add_workspace_package("forge-protocol", &[]);
        self.add_workspace_package("forge-session", &["forge-core", "forge-protocol"]);
        self.add_workspace_package(
            "forge-terminal",
            &["forge-bridge", "forge-core", "forge-protocol"],
        );
        self.add_workspace_package("forge-world", &["forge-core", "forge-protocol"]);
    }

    fn add_workspace_package(&mut self, name: &str, dependencies: &[&str]) {
        self.packages.insert(
            name.to_owned(),
            Package {
                dependencies: dependencies.iter().map(|item| (*item).to_owned()).collect(),
                workspace_member: true,
            },
        );
    }

    fn add_external_package(&mut self, name: &str, dependencies: &[&str]) {
        self.packages.insert(
            name.to_owned(),
            Package {
                dependencies: dependencies.iter().map(|item| (*item).to_owned()).collect(),
                workspace_member: false,
            },
        );
    }

    fn add_dependency(&mut self, package: &str, dependency: &str) {
        self.packages
            .get_mut(package)
            .expect("fixture package should exist")
            .dependencies
            .push(dependency.to_owned());
    }

    fn exclude(&mut self, package: &str) {
        self.excluded.push(package.to_owned());
    }

    fn remove_workspace_package(&mut self, package: &str) {
        self.packages
            .get_mut(package)
            .expect("fixture package should exist")
            .workspace_member = false;
        self.exclude(package);
    }

    fn write(&self) {
        for (name, package) in &self.packages {
            let package_dir = self.path.join(name);
            fs::create_dir_all(package_dir.join("src"))
                .expect("fixture package source should be created");
            fs::write(package_dir.join("src/lib.rs"), "#![forbid(unsafe_code)]\n")
                .expect("fixture source should be written");

            let mut manifest =
                format!("[package]\nname = \"{name}\"\nversion = \"0.1.0\"\nedition = \"2021\"\n");
            if !package.dependencies.is_empty() {
                manifest.push_str("\n[dependencies]\n");
                for dependency in &package.dependencies {
                    manifest.push_str(&format!(
                        "{dependency} = {{ path = \"../{dependency}\" }}\n"
                    ));
                }
            }
            fs::write(package_dir.join("Cargo.toml"), manifest)
                .expect("fixture manifest should be written");
        }

        let members = self
            .packages
            .iter()
            .filter(|(_, package)| package.workspace_member)
            .map(|(name, _)| format!("    \"{name}\","))
            .collect::<Vec<_>>()
            .join("\n");
        let excludes = self
            .excluded
            .iter()
            .map(|name| format!("    \"{name}\","))
            .collect::<Vec<_>>()
            .join("\n");
        let exclude_section = if excludes.is_empty() {
            String::new()
        } else {
            format!("exclude = [\n{excludes}\n]\n")
        };
        fs::write(
            self.path.join("Cargo.toml"),
            format!("[workspace]\nresolver = \"2\"\nmembers = [\n{members}\n]\n{exclude_section}"),
        )
        .expect("fixture workspace should be written");
    }

    fn run_guard(&self) -> std::process::Output {
        Command::new(env!("CARGO_BIN_EXE_forge-seam-direction"))
            .arg("--root")
            .arg(&self.path)
            .output()
            .expect("seam-direction executable should run")
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

trait OutputText {
    fn stdout_text(&self) -> String;
    fn stderr_text(&self) -> String;
}

impl OutputText for std::process::Output {
    fn stdout_text(&self) -> String {
        String::from_utf8_lossy(&self.stdout).into_owned()
    }

    fn stderr_text(&self) -> String {
        String::from_utf8_lossy(&self.stderr).into_owned()
    }
}
