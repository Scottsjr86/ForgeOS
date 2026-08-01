use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};

use forge_guards::source_size::{ModuleStatus, scan_authored_rust};

static FIXTURE_SEQUENCE: AtomicUsize = AtomicUsize::new(0);

#[test]
fn source_size_boundaries_have_exact_outcomes() {
    for (line_count, expected_status) in [
        (500, ModuleStatus::Pass),
        (1_000, ModuleStatus::Pass),
        (1_001, ModuleStatus::Warn),
        (1_200, ModuleStatus::Warn),
        (1_201, ModuleStatus::Fail),
    ] {
        let fixture = Fixture::new(&format!("boundary-{line_count}"));
        write_physical_lines(&fixture.path().join("src/lib.rs"), line_count);

        let report = scan_authored_rust(fixture.path()).expect("fixture scan should succeed");
        assert_eq!(report.modules().len(), 1);
        assert_eq!(report.modules()[0].physical_lines(), line_count);
        assert_eq!(report.modules()[0].status(), expected_status);
        assert_eq!(report.overall_status(), expected_status);
    }
}

#[test]
fn executable_reports_boundary_status_and_exit_code() {
    for (line_count, expected_status, expected_success) in [
        (500, "PASS", true),
        (1_000, "PASS", true),
        (1_001, "WARN", true),
        (1_200, "WARN", true),
        (1_201, "FAIL", false),
    ] {
        let fixture = Fixture::new(&format!("cli-{line_count}"));
        write_physical_lines(&fixture.path().join("src/lib.rs"), line_count);

        let output = Command::new(env!("CARGO_BIN_EXE_forge-source-size"))
            .arg("--root")
            .arg(fixture.path())
            .output()
            .expect("source-size executable should run");
        let stdout = String::from_utf8(output.stdout).expect("guard output should be UTF-8");

        assert_eq!(output.status.success(), expected_success, "{stdout}");
        assert!(
            stdout.contains(&format!(
                "FORGE_SOURCE_SIZE_MODULE status={expected_status} \
                 lines={line_count} path=src/lib.rs"
            )),
            "{stdout}"
        );
        assert!(
            stdout.contains(&format!("FORGE_SOURCE_SIZE_SUMMARY status={expected_status}")),
            "{stdout}"
        );
    }
}

#[test]
fn deny_warnings_returns_failure_without_reclassifying_warning() {
    let fixture = Fixture::new("deny-warnings");
    write_physical_lines(&fixture.path().join("src/lib.rs"), 1_001);

    let output = Command::new(env!("CARGO_BIN_EXE_forge-source-size"))
        .arg("--root")
        .arg(fixture.path())
        .arg("--deny-warnings")
        .output()
        .expect("source-size executable should run");
    let stdout = String::from_utf8(output.stdout).expect("guard output should be UTF-8");

    assert!(!output.status.success(), "{stdout}");
    assert!(
        stdout.contains("FORGE_SOURCE_SIZE_SUMMARY status=WARN"),
        "{stdout}"
    );
    assert!(stdout.contains("warnings_denied=true"), "{stdout}");
}

#[test]
fn fixed_policy_excludes_generated_vendored_and_build_output_source() {
    let fixture = Fixture::new("fixed-exclusions");
    write_physical_lines(&fixture.path().join("src/authored.rs"), 12);
    write_physical_lines(&fixture.path().join("target/debug/build.rs"), 1_201);
    write_physical_lines(&fixture.path().join("vendor/dependency.rs"), 1_201);
    write_physical_lines(&fixture.path().join("vendored/dependency.rs"), 1_201);
    write_physical_lines(&fixture.path().join("third_party/dependency.rs"), 1_201);
    write_physical_lines(&fixture.path().join("generated/bindings.rs"), 1_201);
    write_generated_source(&fixture.path().join("src/bindings.rs"), 1_201);

    let report = scan_authored_rust(fixture.path()).expect("fixture scan should succeed");
    assert_eq!(report.modules().len(), 1);
    assert_eq!(report.modules()[0].relative_path(), Path::new("src/authored.rs"));
    assert_eq!(report.modules()[0].physical_lines(), 12);
    assert_eq!(report.overall_status(), ModuleStatus::Pass);
}

#[cfg(unix)]
#[test]
fn symlink_inside_authored_tree_is_rejected_instead_of_silently_skipped() {
    use std::os::unix::fs::symlink;

    let fixture = Fixture::new("symlink-rejection");
    let outside = Fixture::new("symlink-target");
    write_physical_lines(&outside.path().join("outside.rs"), 10);
    fs::create_dir_all(fixture.path().join("src")).expect("fixture src should be created");
    symlink(
        outside.path().join("outside.rs"),
        fixture.path().join("src/linked.rs"),
    )
    .expect("fixture symlink should be created");

    let error = scan_authored_rust(fixture.path()).expect_err("symlink should be rejected");
    assert!(error.to_string().contains("src/linked.rs"));
}

fn write_physical_lines(path: &Path, line_count: usize) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("fixture directory should be created");
    }
    let mut file = File::create(path).expect("fixture file should be created");
    for _ in 0..line_count {
        file.write_all(b"x\n")
            .expect("fixture line should be written");
    }
}

fn write_generated_source(path: &Path, line_count: usize) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("fixture directory should be created");
    }
    let mut file = File::create(path).expect("fixture file should be created");
    file.write_all(b"// @generated by fixture\n")
        .expect("generated marker should be written");
    for _ in 1..line_count {
        file.write_all(b"x\n")
            .expect("fixture line should be written");
    }
}

struct Fixture {
    path: PathBuf,
}

impl Fixture {
    fn new(label: &str) -> Self {
        let sequence = FIXTURE_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "forge-source-size-{}-{sequence}-{label}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).expect("fixture root should be created");
        Self { path }
    }

    fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}
