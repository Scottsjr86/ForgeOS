#![cfg(target_os = "linux")]

use forge_protocol::identities::{IDENTITY_BYTES, ProjectId, RepositoryId, TerminalId};
use forge_terminal::managed::{
    ManagedTerminalError, ManagedTerminalHandle, ManagedTerminalRegistry,
    ManagedTerminalSpawnRequest,
};
use forge_terminal::pty::{PtyDimensions, PtyError, PtyLifecycle, PtySpawnRequest};
use std::ffi::OsString;
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::thread;
use std::time::{Duration, Instant};

static NEXT_TEMP: AtomicU64 = AtomicU64::new(1);
const POLL: Duration = Duration::from_millis(10);
const DEADLINE: Duration = Duration::from_secs(5);

fn project(byte: u8) -> ProjectId {
    ProjectId::from_bytes([byte; IDENTITY_BYTES])
}

fn repository(byte: u8) -> RepositoryId {
    RepositoryId::from_bytes([byte; IDENTITY_BYTES])
}

fn terminal(byte: u8) -> TerminalId {
    TerminalId::from_bytes([byte; IDENTITY_BYTES])
}

struct TempDir(PathBuf);

impl TempDir {
    fn new(label: &str) -> Self {
        let serial = NEXT_TEMP.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "forgeos-managed-terminal-{label}-{}-{serial}",
            std::process::id()
        ));
        if path.exists() {
            fs::remove_dir_all(&path).expect("remove stale managed-terminal fixture");
        }
        fs::create_dir(&path).expect("create managed-terminal fixture");
        Self(fs::canonicalize(path).expect("canonical fixture path"))
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn spawn_request(
    handle: ManagedTerminalHandle,
    cwd: &TempDir,
    code: &str,
) -> ManagedTerminalSpawnRequest {
    let pty = PtySpawnRequest::new(
        handle.terminal_id(),
        "python3",
        vec![OsString::from("-c"), OsString::from(code)],
        &cwd.0,
        PtyDimensions::new(24, 80).unwrap(),
    )
    .expect("valid PTY request");
    ManagedTerminalSpawnRequest::new(handle.project_id(), handle.repository_id(), pty)
        .expect("valid managed request")
}

fn collect_until(
    registry: &mut ManagedTerminalRegistry,
    handle: ManagedTerminalHandle,
    needle: &[u8],
) -> Vec<u8> {
    let deadline = Instant::now() + DEADLINE;
    while Instant::now() < deadline {
        registry
            .read_available(handle)
            .expect("managed output remains readable");
        let bytes = registry.view(handle).expect("managed view").output_bytes();
        if bytes.windows(needle.len()).any(|window| window == needle) {
            return bytes;
        }
        let _ = registry.poll_exit(handle).expect("managed exit poll");
        thread::sleep(POLL);
    }
    panic!("timed out waiting for {needle:?}");
}

#[test]
fn multiple_project_bound_terminals_preserve_identity_and_output() {
    let first_dir = TempDir::new("first");
    let second_dir = TempDir::new("second");
    let first = ManagedTerminalHandle::new(project(1), repository(11), terminal(21));
    let second = ManagedTerminalHandle::new(project(2), repository(12), terminal(22));
    let mut registry = ManagedTerminalRegistry::new();

    assert_eq!(
        registry
            .spawn(spawn_request(
                first,
                &first_dir,
                "import os; os.write(1, b'FIRST\\x00')"
            ))
            .unwrap(),
        first
    );
    assert_eq!(
        registry
            .spawn(spawn_request(
                second,
                &second_dir,
                "import os; os.write(1, b'SECOND\\x00')"
            ))
            .unwrap(),
        second
    );

    assert!(
        collect_until(&mut registry, first, b"FIRST\x00")
            .windows(6)
            .any(|window| window == b"FIRST\x00")
    );
    assert!(
        collect_until(&mut registry, second, b"SECOND\x00")
            .windows(7)
            .any(|window| window == b"SECOND\x00")
    );

    let handles: Vec<_> = registry.handles().collect();
    assert_eq!(handles, vec![first, second]);
    assert_eq!(
        registry.view(first).unwrap().working_directory(),
        first_dir.0.as_path()
    );
    assert_eq!(
        registry.view(second).unwrap().working_directory(),
        second_dir.0.as_path()
    );
}

#[test]
fn wrong_project_binding_cannot_read_write_or_terminate_a_terminal() {
    let cwd = TempDir::new("binding");
    let correct = ManagedTerminalHandle::new(project(3), repository(13), terminal(23));
    let wrong = ManagedTerminalHandle::new(project(4), repository(13), terminal(23));
    let mut registry = ManagedTerminalRegistry::new();
    registry
        .spawn(spawn_request(
            correct,
            &cwd,
            "import time; print('READY', flush=True); time.sleep(5)",
        ))
        .unwrap();
    collect_until(&mut registry, correct, b"READY");

    assert!(matches!(
        registry.view(wrong),
        Err(ManagedTerminalError::BindingMismatch { terminal_id, .. }) if terminal_id == correct.terminal_id()
    ));
    assert!(matches!(
        registry.write_input(wrong, b"x"),
        Err(ManagedTerminalError::BindingMismatch { .. })
    ));
    assert!(matches!(
        registry.terminate(wrong),
        Err(ManagedTerminalError::BindingMismatch { .. })
    ));

    let exit = registry
        .terminate(correct)
        .expect("correct binding terminates");
    assert!(exit.terminated_by_operator());
}

#[test]
fn managed_input_resize_and_rendered_transcript_follow_one_terminal() {
    let cwd = TempDir::new("interactive");
    let handle = ManagedTerminalHandle::new(project(5), repository(15), terminal(25));
    let code = r#"
import fcntl, os, struct, termios, tty
tty.setraw(0)
os.write(1, b'READY')
data = b''
while len(data) < 3:
    data += os.read(0, 3 - len(data))
size = struct.unpack('HHHH', fcntl.ioctl(0, termios.TIOCGWINSZ, b'\x00' * 8))
os.write(1, b'INPUT' + data + b'SIZE' + str(size[0]).encode() + b'x' + str(size[1]).encode())
"#;
    let mut registry = ManagedTerminalRegistry::new();
    registry
        .spawn(spawn_request(handle, &cwd, code))
        .expect("spawn interactive terminal");
    collect_until(&mut registry, handle, b"READY");
    registry
        .resize(handle, PtyDimensions::new(41, 103).unwrap())
        .unwrap();
    registry.write_input(handle, b"abc").unwrap();
    let output = collect_until(&mut registry, handle, b"SIZE41x103");
    assert!(output.windows(8).any(|window| window == b"INPUTabc"));

    let view = registry.view(handle).unwrap();
    assert!(matches!(
        view.lifecycle(),
        PtyLifecycle::Running { .. } | PtyLifecycle::Exited(_)
    ));
    for pair in view.output_chunks().windows(2) {
        assert_eq!(pair[0].sequence() + 1, pair[1].sequence());
    }
}

#[test]
fn only_exited_terminals_can_be_removed() {
    let cwd = TempDir::new("remove");
    let handle = ManagedTerminalHandle::new(project(6), repository(16), terminal(26));
    let mut registry = ManagedTerminalRegistry::new();
    registry
        .spawn(spawn_request(
            handle,
            &cwd,
            "import time; print('RUNNING', flush=True); time.sleep(5)",
        ))
        .unwrap();
    collect_until(&mut registry, handle, b"RUNNING");

    assert!(matches!(
        registry.remove_exited(handle),
        Err(ManagedTerminalError::Pty(PtyError::StillRunning(id))) if id == handle.terminal_id()
    ));
    registry.terminate(handle).unwrap();
    let removed = registry
        .remove_exited(handle)
        .expect("remove exited terminal");
    assert_eq!(removed.handle(), handle);
    assert!(matches!(removed.lifecycle(), PtyLifecycle::Exited(_)));
    assert!(registry.handles().next().is_none());
}
