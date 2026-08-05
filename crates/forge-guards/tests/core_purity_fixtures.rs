use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};

static FIXTURE_SEQUENCE: AtomicUsize = AtomicUsize::new(0);

#[test]
fn real_shape_with_only_protocol_dependency_passes() {
    let fixture = Fixture::new("legal");
    fixture.add_package("forge-protocol", &[]);
    fixture.add_package("forge-core", &["forge-protocol"]);
    fixture.write_workspace(&["forge-core", "forge-protocol"]);

    let output = fixture.run_guard();
    let stdout = output.stdout_text();

    assert!(
        output.status.success(),
        "{stdout}\n{}",
        output.stderr_text()
    );
    assert!(
        stdout.contains("FORGE_CORE_PURITY_PACKAGE status=ALLOWED package=forge-core"),
        "{stdout}"
    );
    assert!(
        stdout.contains("FORGE_CORE_PURITY_PACKAGE status=ALLOWED package=forge-protocol"),
        "{stdout}"
    );
    assert!(
        stdout.contains("FORGE_CORE_PURITY_SUMMARY status=PASS packages=2 allowed=2 forbidden=0"),
        "{stdout}"
    );
}

#[test]
fn representative_forbidden_direct_dependencies_are_rejected() {
    for package in [
        "tokio",
        "forge-world",
        "forge-nyx-client",
        "forge-git",
        "forge-terminal",
        "filesystem-adapter",
        "tower-lsp",
        "dap-provider",
        "reqwest",
        "forge-session",
    ] {
        let fixture = Fixture::new(package);
        fixture.add_package("forge-protocol", &[]);
        fixture.add_package(package, &[]);
        fixture.add_package("forge-core", &["forge-protocol", package]);
        fixture.write_workspace(&["forge-core", "forge-protocol", package]);

        let output = fixture.run_guard();
        let stdout = output.stdout_text();

        assert!(
            !output.status.success(),
            "{package} unexpectedly passed\n{stdout}"
        );
        assert!(
            stdout.contains(&format!(
                "FORGE_CORE_PURITY_PACKAGE status=FORBIDDEN package={package}"
            )),
            "{stdout}"
        );
        assert!(
            stdout.contains("FORGE_CORE_PURITY_SUMMARY status=FAIL"),
            "{stdout}"
        );
    }
}

#[test]
fn generic_adapter_cannot_smuggle_transitive_effect_dependency() {
    let fixture = Fixture::new("transitive-smuggle");
    fixture.add_package("forge-protocol", &[]);
    fixture.add_package("forge-world", &[]);
    fixture.add_package("generic-adapter", &["forge-world"]);
    fixture.add_package("forge-core", &["forge-protocol", "generic-adapter"]);
    fixture.write_workspace(&[
        "forge-core",
        "forge-protocol",
        "generic-adapter",
        "forge-world",
    ]);

    let output = fixture.run_guard();
    let stdout = output.stdout_text();

    assert!(!output.status.success(), "{stdout}");
    assert!(
        stdout.contains("FORGE_CORE_PURITY_PACKAGE status=FORBIDDEN package=generic-adapter"),
        "{stdout}"
    );
    assert!(
        stdout.contains("FORGE_CORE_PURITY_PACKAGE status=FORBIDDEN package=forge-world"),
        "{stdout}"
    );
    assert!(
        stdout.contains("FORGE_CORE_PURITY_SUMMARY status=FAIL packages=4 allowed=2 forbidden=2"),
        "{stdout}"
    );
}

struct Fixture {
    path: PathBuf,
}

impl Fixture {
    fn new(label: &str) -> Self {
        let sequence = FIXTURE_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "forge-core-purity-{}-{sequence}-{label}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).expect("fixture root should be created");
        Self { path }
    }

    fn add_package(&self, name: &str, dependencies: &[&str]) {
        let package_dir = self.path.join(name);
        fs::create_dir_all(package_dir.join("src"))
            .expect("fixture package source should be created");
        fs::write(package_dir.join("src/lib.rs"), "#![forbid(unsafe_code)]\n")
            .expect("fixture source should be written");

        let mut manifest =
            format!("[package]\nname = \"{name}\"\nversion = \"0.1.0\"\nedition = \"2024\"\n");
        if !dependencies.is_empty() {
            manifest.push_str("\n[dependencies]\n");
            for (index, dependency) in dependencies.iter().enumerate() {
                manifest.push_str(&format!(
                    "dep_{index} = {{ package = \"{dependency}\", path = \"../{dependency}\" }}\n"
                ));
            }
        }
        fs::write(package_dir.join("Cargo.toml"), manifest)
            .expect("fixture manifest should be written");
    }

    fn write_workspace(&self, members: &[&str]) {
        let members = members
            .iter()
            .map(|member| format!("    \"{member}\","))
            .collect::<Vec<_>>()
            .join("\n");
        fs::write(
            self.path.join("Cargo.toml"),
            format!("[workspace]\nresolver = \"2\"\nmembers = [\n{members}\n]\n"),
        )
        .expect("fixture workspace should be written");
    }

    fn run_guard(&self) -> std::process::Output {
        Command::new(env!("CARGO_BIN_EXE_forge-core-purity"))
            .arg("--root")
            .arg(&self.path)
            .output()
            .expect("core-purity executable should run")
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
