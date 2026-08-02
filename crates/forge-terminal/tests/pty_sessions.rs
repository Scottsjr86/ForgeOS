use forge_protocol::identities::TerminalId;
use forge_terminal::pty::{
    PtyDimensions, PtyError, PtyLifecycle, PtyRegistry, PtyRequestError, PtySpawnRequest,
};
use std::ffi::OsString;
use std::fs;
use std::os::unix::ffi::OsStrExt;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::thread;
use std::time::{Duration, Instant};

static NEXT_TEMP: AtomicU64 = AtomicU64::new(1);
const POLL: Duration = Duration::from_millis(10);
const DEADLINE: Duration = Duration::from_secs(5);

fn terminal(value: u8) -> TerminalId {
    TerminalId::from_bytes([value; 16])
}

struct TempDir(PathBuf);

impl TempDir {
    fn new(label: &str) -> Self {
        let serial = NEXT_TEMP.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "forgeos-pty-{label}-{}-{serial}",
            std::process::id()
        ));
        fs::create_dir(&path).expect("temporary PTY directory should be created");
        Self(path)
    }

    fn path(&self) -> &PathBuf {
        &self.0
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn request(id: TerminalId, cwd: &TempDir, code: &str) -> PtySpawnRequest {
    PtySpawnRequest::new(
        id,
        "python3",
        vec![OsString::from("-c"), OsString::from(code)],
        cwd.path(),
        PtyDimensions::new(24, 80).expect("default dimensions should validate"),
    )
    .expect("fixture request should validate")
}

fn collect_until(registry: &mut PtyRegistry, id: TerminalId, needle: &[u8]) -> Vec<u8> {
    let deadline = Instant::now() + DEADLINE;
    let mut output = Vec::new();
    while Instant::now() < deadline {
        let session = registry.session_mut(id).expect("terminal should exist");
        let chunks = session
            .read_available()
            .expect("PTY output should remain readable");
        for pair in chunks.windows(2) {
            assert_eq!(pair[0].sequence() + 1, pair[1].sequence());
        }
        for chunk in chunks {
            assert_eq!(chunk.terminal_id(), id);
            output.extend_from_slice(chunk.bytes());
        }
        if output.windows(needle.len()).any(|window| window == needle) {
            return output;
        }
        let _ = session
            .poll_exit()
            .expect("child status should be observable");
        thread::sleep(POLL);
    }
    panic!("timed out waiting for {:?}; output={output:?}", needle);
}

fn wait_exit(registry: &mut PtyRegistry, id: TerminalId) {
    let deadline = Instant::now() + DEADLINE;
    while Instant::now() < deadline {
        let session = registry.session_mut(id).expect("terminal should exist");
        if session
            .poll_exit()
            .expect("child status should be observable")
            .is_some()
        {
            return;
        }
        thread::sleep(POLL);
    }
    panic!("timed out waiting for terminal exit");
}

#[test]
fn raw_bytes_resize_and_exit_are_preserved() {
    let cwd = TempDir::new("raw");
    let code = r#"
import fcntl, os, struct, termios, tty
tty.setraw(0)
os.write(1, b'READY\x00')
data = b''
while len(data) < 4:
    data += os.read(0, 4 - len(data))
size = struct.unpack('HHHH', fcntl.ioctl(0, termios.TIOCGWINSZ, b'\x00' * 8))
os.write(1, b'ECHO' + data + b'SIZE' + str(size[0]).encode() + b'x' + str(size[1]).encode() + b'\x00')
"#;
    let id = terminal(1);
    let mut registry = PtyRegistry::new();
    registry.spawn(request(id, &cwd, code)).unwrap();
    collect_until(&mut registry, id, b"READY\x00");
    registry
        .session_mut(id)
        .unwrap()
        .resize(PtyDimensions::new(31, 97).unwrap())
        .unwrap();
    let input = [0_u8, 0xff, b'A', b'\n'];
    registry
        .session_mut(id)
        .unwrap()
        .write_input(&input)
        .unwrap();
    let output = collect_until(&mut registry, id, b"SIZE31x97\x00");
    let expected_echo = [b"ECHO".as_slice(), input.as_slice()].concat();
    assert!(output
        .windows(expected_echo.len())
        .any(|window| window == expected_echo.as_slice()));
    wait_exit(&mut registry, id);
    let exit = registry
        .session_mut(id)
        .unwrap()
        .poll_exit()
        .unwrap()
        .unwrap();
    assert_eq!(exit.code(), 0);
    assert!(exit.success());
}

#[test]
fn working_directory_is_applied_exactly() {
    let cwd = TempDir::new("cwd");
    let code = "import os; os.write(1, os.getcwd().encode() + b'\\x00')";
    let id = terminal(2);
    let mut registry = PtyRegistry::new();
    registry.spawn(request(id, &cwd, code)).unwrap();
    let output = collect_until(&mut registry, id, b"\x00");
    let expected = fs::canonicalize(cwd.path()).unwrap();
    let expected_bytes = expected.as_os_str().as_bytes();
    assert!(output
        .windows(expected_bytes.len())
        .any(|window| window == expected_bytes));
}

#[test]
fn duplicate_terminal_identity_is_rejected() {
    let cwd = TempDir::new("duplicate");
    let id = terminal(3);
    let mut registry = PtyRegistry::new();
    registry
        .spawn(request(id, &cwd, "import signal; signal.pause()"))
        .unwrap();
    let error = registry
        .spawn(request(id, &cwd, "pass"))
        .expect_err("duplicate identity must fail");
    assert!(matches!(error, PtyError::DuplicateTerminal(found) if found == id));
    registry.session_mut(id).unwrap().terminate().unwrap();
}

#[test]
fn terminating_one_terminal_does_not_affect_another() {
    let cwd = TempDir::new("isolation");
    let mut registry = PtyRegistry::new();
    let first = terminal(4);
    let second = terminal(5);
    registry
        .spawn(request(first, &cwd, "import signal; signal.pause()"))
        .unwrap();
    registry
        .spawn(request(
            second,
            &cwd,
            "import os, tty; tty.setraw(0); os.write(1,b'BREADY'); data=os.read(0,4); os.write(1,b'PONG'+data)",
        ))
        .unwrap();
    collect_until(&mut registry, second, b"BREADY");
    let exit = registry.session_mut(first).unwrap().terminate().unwrap();
    assert!(exit.terminated_by_operator());
    registry
        .session_mut(second)
        .unwrap()
        .write_input(b"PING")
        .unwrap();
    let output = collect_until(&mut registry, second, b"PONGPING");
    assert!(output.windows(8).any(|window| window == b"PONGPING"));
}

#[test]
fn missing_executable_is_a_typed_adapter_failure() {
    let cwd = TempDir::new("missing");
    let request = PtySpawnRequest::new(
        terminal(6),
        "forgeos-definitely-missing-executable",
        Vec::new(),
        cwd.path(),
        PtyDimensions::new(24, 80).unwrap(),
    )
    .unwrap();
    let error = PtyRegistry::new()
        .spawn(request)
        .expect_err("missing executable must fail");
    assert!(matches!(error, PtyError::Adapter(_)));
}

#[test]
fn invalid_dimensions_are_rejected_before_spawn() {
    assert_eq!(
        PtyDimensions::new(0, 80).unwrap_err(),
        PtyRequestError::ZeroRows
    );
    assert_eq!(
        PtyDimensions::new(24, 0).unwrap_err(),
        PtyRequestError::ZeroColumns
    );
}

#[test]
fn relative_working_directory_is_rejected() {
    let error = PtySpawnRequest::new(
        terminal(7),
        "python3",
        Vec::new(),
        "relative/path",
        PtyDimensions::new(24, 80).unwrap(),
    )
    .unwrap_err();
    assert!(matches!(
        error,
        PtyRequestError::WorkingDirectoryNotAbsolute(_)
    ));
}

#[test]
fn input_and_resize_after_exit_are_rejected() {
    let cwd = TempDir::new("after-exit");
    let id = terminal(8);
    let mut registry = PtyRegistry::new();
    registry.spawn(request(id, &cwd, "pass")).unwrap();
    wait_exit(&mut registry, id);
    assert!(matches!(
        registry.session_mut(id).unwrap().write_input(b"late"),
        Err(PtyError::NotRunning(found)) if found == id
    ));
    assert!(matches!(
        registry
            .session_mut(id)
            .unwrap()
            .resize(PtyDimensions::new(40, 120).unwrap()),
        Err(PtyError::NotRunning(found)) if found == id
    ));
}

#[test]
fn symlink_working_directory_is_rejected() {
    let cwd = TempDir::new("symlink-root");
    let target = cwd.path().join("target");
    let link = cwd.path().join("link");
    fs::create_dir(&target).unwrap();
    std::os::unix::fs::symlink(&target, &link).unwrap();
    let error = PtySpawnRequest::new(
        terminal(9),
        "python3",
        Vec::new(),
        link,
        PtyDimensions::new(24, 80).unwrap(),
    )
    .unwrap_err();
    assert!(matches!(error, PtyRequestError::WorkingDirectorySymlink(_)));
}

#[test]
fn exited_session_can_be_removed_but_running_session_cannot() {
    let cwd = TempDir::new("remove");
    let exited = terminal(11);
    let running = terminal(12);
    let mut registry = PtyRegistry::new();
    registry.spawn(request(exited, &cwd, "pass")).unwrap();
    registry
        .spawn(request(running, &cwd, "import signal; signal.pause()"))
        .unwrap();
    wait_exit(&mut registry, exited);
    let removed = registry.remove_exited(exited).unwrap();
    assert!(matches!(removed.lifecycle(), PtyLifecycle::Exited(_)));
    assert!(matches!(
        registry.remove_exited(running),
        Err(PtyError::StillRunning(found)) if found == running
    ));
    registry.session_mut(running).unwrap().terminate().unwrap();
}
