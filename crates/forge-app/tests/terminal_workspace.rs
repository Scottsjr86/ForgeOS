#![cfg(target_os = "linux")]

use forge_app::composition::terminal_workspace::{
    ProjectTerminalLaunch, ProjectTerminalWorkspace, ProjectTerminalWorkspaceError,
    TerminalWorkingDirectory,
};
use forge_core::projects::{AllowedProjectRoot, LanguageProfile, ProjectManifest};
use forge_project::paths::{RepositoryBoundary, RepositoryBoundaryError};
use forge_protocol::identities::{ProjectId, RepositoryId, TerminalId, IDENTITY_BYTES};
use forge_terminal::managed::{ManagedTerminalError, ManagedTerminalHandle};
use forge_terminal::pty::PtyDimensions;
use std::ffi::OsString;
use std::fs;
use std::path::PathBuf;

#[cfg(unix)]
use std::os::unix::ffi::OsStrExt;
use std::sync::atomic::{AtomicU64, Ordering};
use std::thread;
use std::time::{Duration, Instant};

#[cfg(unix)]
use std::os::unix::fs::symlink;

static NEXT_FIXTURE: AtomicU64 = AtomicU64::new(1);
const POLL: Duration = Duration::from_millis(10);
const DEADLINE: Duration = Duration::from_secs(5);

struct Fixture {
    root: PathBuf,
    repository: PathBuf,
    manifest: ProjectManifest,
    boundary: RepositoryBoundary,
}

impl Fixture {
    fn new(label: &str, allowed_roots: Vec<AllowedProjectRoot>) -> Self {
        let sequence = NEXT_FIXTURE.fetch_add(1, Ordering::Relaxed);
        let root = std::env::temp_dir().join(format!(
            "forgeos-terminal-workspace-{label}-{}-{sequence}",
            std::process::id()
        ));
        let repository = root.join("repository");
        if root.exists() {
            fs::remove_dir_all(&root).expect("remove stale terminal fixture");
        }
        fs::create_dir_all(repository.join("src/nested")).expect("create terminal fixture");
        let repository = fs::canonicalize(repository).expect("canonical repository");
        let project_id = ProjectId::from_bytes([(sequence as u8).wrapping_add(20); IDENTITY_BYTES]);
        let repository_id =
            RepositoryId::from_bytes([(sequence as u8).wrapping_add(90); IDENTITY_BYTES]);
        let manifest = ProjectManifest::new(
            project_id,
            repository_id,
            "Terminal workspace fixture",
            allowed_roots,
            Vec::new(),
            LanguageProfile::Rust,
            Vec::new(),
        )
        .expect("valid terminal manifest");
        let boundary = RepositoryBoundary::open(repository_id, &repository).expect("boundary");
        Self {
            root,
            repository,
            manifest,
            boundary,
        }
    }

    fn workspace(&self) -> ProjectTerminalWorkspace {
        ProjectTerminalWorkspace::new(&self.manifest, self.boundary.clone()).expect("workspace")
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

fn launch(
    terminal_id: TerminalId,
    working_directory: TerminalWorkingDirectory,
    code: &str,
) -> ProjectTerminalLaunch {
    ProjectTerminalLaunch::new(
        terminal_id,
        "python3",
        vec![OsString::from("-c"), OsString::from(code)],
        working_directory,
        PtyDimensions::new(24, 80).unwrap(),
    )
}

fn collect_until(
    workspace: &mut ProjectTerminalWorkspace,
    handle: ManagedTerminalHandle,
    needle: &[u8],
) -> Vec<u8> {
    let deadline = Instant::now() + DEADLINE;
    while Instant::now() < deadline {
        workspace
            .read_available(handle)
            .expect("read terminal output");
        let bytes = workspace
            .view(handle)
            .expect("terminal view")
            .output_bytes();
        if bytes.windows(needle.len()).any(|window| window == needle) {
            return bytes;
        }
        let _ = workspace.poll_exit(handle).expect("poll terminal exit");
        thread::sleep(POLL);
    }
    panic!("timed out waiting for {needle:?}");
}

#[test]
fn terminal_launch_is_bound_to_project_repository_and_allowed_directory() {
    let fixture = Fixture::new(
        "allowed",
        vec![AllowedProjectRoot::relative("src").unwrap()],
    );
    let mut workspace = fixture.workspace();
    let terminal_id = TerminalId::from_bytes([31; IDENTITY_BYTES]);
    let handle = workspace
        .spawn(launch(
            terminal_id,
            TerminalWorkingDirectory::relative("src/nested").unwrap(),
            "import os; os.write(1, os.getcwd().encode() + b'\\x00')",
        ))
        .expect("spawn project terminal");
    let output = collect_until(&mut workspace, handle, b"\x00");
    let expected = fixture.repository.join("src/nested");
    assert!(output
        .windows(expected.as_os_str().as_bytes().len())
        .any(|window| window == expected.as_os_str().as_bytes()));
    assert_eq!(handle.project_id(), fixture.manifest.project_id());
    assert_eq!(handle.repository_id(), fixture.manifest.repository_id());
}

#[test]
fn working_directory_outside_declared_roots_is_rejected_before_spawn() {
    let fixture = Fixture::new(
        "scope",
        vec![AllowedProjectRoot::relative("src/nested").unwrap()],
    );
    let mut workspace = fixture.workspace();
    let error = workspace
        .spawn(launch(
            TerminalId::from_bytes([32; IDENTITY_BYTES]),
            TerminalWorkingDirectory::relative("src").unwrap(),
            "print('should not run')",
        ))
        .expect_err("broader working directory must be rejected");
    assert!(matches!(
        error,
        ProjectTerminalWorkspaceError::WorkingDirectoryOutsideAllowedRoots(path)
            if path == PathBuf::from("src")
    ));
}

#[test]
fn repository_root_requires_explicit_root_authority() {
    let fixture = Fixture::new(
        "root-scope",
        vec![AllowedProjectRoot::relative("src").unwrap()],
    );
    let mut workspace = fixture.workspace();
    assert!(matches!(
        workspace.spawn(launch(
            TerminalId::from_bytes([33; IDENTITY_BYTES]),
            TerminalWorkingDirectory::repository_root(),
            "print('should not run')",
        )),
        Err(ProjectTerminalWorkspaceError::WorkingDirectoryOutsideAllowedRoots(_))
    ));
}

#[test]
fn symlink_working_directory_is_rejected_by_project_boundary() {
    let fixture = Fixture::new(
        "symlink",
        vec![AllowedProjectRoot::relative("src").unwrap()],
    );
    let outside = fixture.root.join("outside");
    fs::create_dir(&outside).expect("outside directory");
    symlink(&outside, fixture.repository.join("src/link")).expect("create symlink");
    let mut workspace = fixture.workspace();
    let error = workspace
        .spawn(launch(
            TerminalId::from_bytes([34; IDENTITY_BYTES]),
            TerminalWorkingDirectory::relative("src/link").unwrap(),
            "print('should not run')",
        ))
        .expect_err("symlink path must be rejected");
    assert!(matches!(
        error,
        ProjectTerminalWorkspaceError::Boundary(RepositoryBoundaryError::SymlinkRejected { .. })
    ));
}

#[test]
fn operations_with_a_forged_project_handle_fail_closed() {
    let fixture = Fixture::new("forged", vec![AllowedProjectRoot::repository_root()]);
    let mut workspace = fixture.workspace();
    let handle = workspace
        .spawn(launch(
            TerminalId::from_bytes([35; IDENTITY_BYTES]),
            TerminalWorkingDirectory::repository_root(),
            "import time; print('READY', flush=True); time.sleep(5)",
        ))
        .unwrap();
    collect_until(&mut workspace, handle, b"READY");
    let forged = ManagedTerminalHandle::new(
        ProjectId::from_bytes([99; IDENTITY_BYTES]),
        handle.repository_id(),
        handle.terminal_id(),
    );
    assert!(matches!(
        workspace.terminate(forged),
        Err(ProjectTerminalWorkspaceError::Terminal(
            ManagedTerminalError::BindingMismatch { .. }
        ))
    ));
    workspace
        .terminate(handle)
        .expect("correct handle terminates");
}
