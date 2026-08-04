use forge_session::bootstrap::{
    launch_session, DisplayBackend, SessionEnvironment, SessionEnvironmentError,
    SessionLaunchError, SessionLaunchRequest, DEFAULT_SESSION_LAUNCHER, DISPLAY_MANAGER_ENTRY,
    SESSION_PATH,
};
use std::ffi::{OsStr, OsString};
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_FIXTURE: AtomicU64 = AtomicU64::new(1);

struct Fixture {
    root: PathBuf,
}

impl Fixture {
    fn new() -> Self {
        let id = NEXT_FIXTURE.fetch_add(1, Ordering::Relaxed);
        let root = std::env::temp_dir().join(format!(
            "forgeos-session-bootstrap-{}-{id}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).expect("create session fixture");
        Self { root }
    }

    fn script(&self, name: &str, body: &str) -> PathBuf {
        let path = self.root.join(name);
        fs::write(&path, format!("#!/bin/sh\nset -eu\n{body}\n")).expect("write script");
        fs::set_permissions(&path, fs::Permissions::from_mode(0o755))
            .expect("make script executable");
        path
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

fn base_environment() -> Vec<(OsString, OsString)> {
    vec![
        (OsString::from("HOME"), OsString::from("/home/forge-user")),
        (
            OsString::from("XDG_RUNTIME_DIR"),
            OsString::from("/run/user/1000"),
        ),
        (OsString::from("USER"), OsString::from("forge-user")),
        (OsString::from("LOGNAME"), OsString::from("forge-user")),
        (OsString::from("DISPLAY"), OsString::from(":9")),
        (OsString::from("LANG"), OsString::from("en_US.UTF-8")),
        (OsString::from("SHELL"), OsString::from("/bin/bash")),
        (OsString::from("PWD"), OsString::from("/tmp/wrong-worktree")),
        (OsString::from("BASH_ENV"), OsString::from("/tmp/profile")),
        (
            OsString::from("LD_PRELOAD"),
            OsString::from("/tmp/inject.so"),
        ),
        (
            OsString::from("FORGE_SECRET"),
            OsString::from("must-not-leak"),
        ),
    ]
}

fn environment() -> SessionEnvironment {
    SessionEnvironment::from_parent(base_environment()).expect("valid session environment")
}

#[test]
fn environment_is_sanitized_and_deterministic() {
    let environment = environment();

    assert_eq!(environment.backend(), DisplayBackend::X11);
    assert_eq!(environment.get("PATH"), Some(OsStr::new(SESSION_PATH)));
    assert_eq!(environment.get("FORGEOS_SESSION"), Some(OsStr::new("1")));
    assert_eq!(
        environment.get("XDG_CURRENT_DESKTOP"),
        Some(OsStr::new("ForgeOS"))
    );
    assert_eq!(environment.get("XDG_SESSION_TYPE"), Some(OsStr::new("x11")));
    assert_eq!(environment.get("SHELL"), None);
    assert_eq!(environment.get("PWD"), None);
    assert_eq!(environment.get("BASH_ENV"), None);
    assert_eq!(environment.get("LD_PRELOAD"), None);
    assert_eq!(environment.get("FORGE_SECRET"), None);
}

#[test]
fn wayland_is_preferred_when_both_display_variables_exist() {
    let mut variables = base_environment();
    variables.push((
        OsString::from("WAYLAND_DISPLAY"),
        OsString::from("wayland-7"),
    ));
    let environment = SessionEnvironment::from_parent(variables).unwrap();

    assert_eq!(environment.backend(), DisplayBackend::Wayland);
    assert_eq!(
        environment.get("WAYLAND_DISPLAY"),
        Some(OsStr::new("wayland-7"))
    );
    assert_eq!(environment.get("DISPLAY"), Some(OsStr::new(":9")));
    assert_eq!(
        environment.get("XDG_SESSION_TYPE"),
        Some(OsStr::new("wayland"))
    );
}

#[test]
fn invalid_display_manager_environment_fails_before_spawn() {
    let missing_display = SessionEnvironment::from_parent([
        (OsString::from("HOME"), OsString::from("/home/user")),
        (
            OsString::from("XDG_RUNTIME_DIR"),
            OsString::from("/run/user/1000"),
        ),
    ]);
    assert_eq!(
        missing_display,
        Err(SessionEnvironmentError::MissingDisplay)
    );

    let relative_home = SessionEnvironment::from_parent([
        (OsString::from("HOME"), OsString::from("relative")),
        (
            OsString::from("XDG_RUNTIME_DIR"),
            OsString::from("/run/user/1000"),
        ),
        (OsString::from("DISPLAY"), OsString::from(":0")),
    ]);
    assert!(matches!(
        relative_home,
        Err(SessionEnvironmentError::RelativePath { name: "HOME", .. })
    ));
}

#[test]
fn launch_uses_root_directory_exact_arguments_and_sanitized_environment() {
    let fixture = Fixture::new();
    let report = fixture.root.join("report.txt");
    let script = fixture.script(
        "capture-session",
        r#"
printf 'pwd=%s\n' "$PWD" > "$2"
printf 'session=%s\n' "$FORGEOS_SESSION" >> "$2"
printf 'desktop=%s\n' "$XDG_CURRENT_DESKTOP" >> "$2"
printf 'path=%s\n' "$PATH" >> "$2"
printf 'secret=%s\n' "${FORGE_SECRET-unset}" >> "$2"
printf 'argument=%s\n' "$1" >> "$2"
"#,
    );
    let request = SessionLaunchRequest::new(
        script,
        vec![
            OsString::from("alpha beta"),
            report.clone().into_os_string(),
        ],
        environment(),
    )
    .unwrap();

    let outcome = launch_session(&request).expect("composition root launches");
    assert_eq!(outcome.exit_code(), Some(0));
    assert_eq!(outcome.signal(), None);
    assert_eq!(
        fs::read_to_string(report).unwrap(),
        format!(
            "pwd=/\nsession=1\ndesktop=ForgeOS\npath={SESSION_PATH}\nsecret=unset\nargument=alpha beta\n"
        )
    );
}

#[test]
fn composition_root_failure_status_is_preserved() {
    let fixture = Fixture::new();
    let script = fixture.script("fail-session", "exit 37");
    let request = SessionLaunchRequest::new(script, Vec::new(), environment()).unwrap();

    let outcome = launch_session(&request).expect("child started");
    assert_eq!(outcome.exit_code(), Some(37));
    assert_eq!(outcome.launcher_exit_code(), 37);
}

#[test]
fn missing_or_relative_composition_root_is_rejected_truthfully() {
    let relative = SessionLaunchRequest::new(
        PathBuf::from("target/debug/forge-app"),
        Vec::new(),
        environment(),
    );
    assert!(matches!(
        relative,
        Err(SessionLaunchError::RelativeCompositionRoot(_))
    ));

    let request = SessionLaunchRequest::new(
        PathBuf::from("/definitely/missing/forge-app"),
        Vec::new(),
        environment(),
    )
    .unwrap();
    assert!(matches!(
        launch_session(&request),
        Err(SessionLaunchError::Spawn { .. })
    ));
}

#[test]
fn real_launcher_binary_propagates_child_status() {
    let fixture = Fixture::new();
    let script = fixture.script("binary-failure", "exit 41");
    let launcher = env!("CARGO_BIN_EXE_forgeos-session-launcher");
    let mut command = Command::new(launcher);
    command
        .env_clear()
        .envs(base_environment())
        .arg("--composition-root")
        .arg(script);

    let status = command.status().expect("launcher binary starts");
    assert_eq!(status.code(), Some(41));
}

#[test]
fn real_launcher_binary_reports_spawn_and_environment_failures() {
    let launcher = env!("CARGO_BIN_EXE_forgeos-session-launcher");
    let spawn_failure = Command::new(launcher)
        .env_clear()
        .envs(base_environment())
        .arg("--composition-root")
        .arg("/definitely/missing/forge-app")
        .status()
        .expect("launcher binary starts for spawn failure");
    assert_eq!(spawn_failure.code(), Some(127));

    let environment_failure = Command::new(launcher)
        .env_clear()
        .env("HOME", "/home/forge-user")
        .env("XDG_RUNTIME_DIR", "/run/user/1000")
        .arg("--composition-root")
        .arg("/definitely/missing/forge-app")
        .status()
        .expect("launcher binary starts for environment failure");
    assert_eq!(environment_failure.code(), Some(78));
}

#[test]
fn desktop_entry_is_fixed_shell_free_and_packaging_ready() {
    assert!(DISPLAY_MANAGER_ENTRY.starts_with("[Desktop Entry]\n"));
    assert!(DISPLAY_MANAGER_ENTRY.contains("Name=ForgeOS\n"));
    let exec_line = format!("Exec={DEFAULT_SESSION_LAUNCHER}");
    let try_exec_line = format!("TryExec={DEFAULT_SESSION_LAUNCHER}");
    assert!(DISPLAY_MANAGER_ENTRY
        .lines()
        .any(|line| line == exec_line.as_str()));
    assert!(DISPLAY_MANAGER_ENTRY
        .lines()
        .any(|line| line == try_exec_line.as_str()));
    assert!(!DISPLAY_MANAGER_ENTRY.contains("sh -c"));
    assert!(!DISPLAY_MANAGER_ENTRY.contains("$HOME"));
    assert!(!DISPLAY_MANAGER_ENTRY.contains("%f"));
    assert!(!DISPLAY_MANAGER_ENTRY.contains("%u"));
    assert!(Path::new(DEFAULT_SESSION_LAUNCHER).is_absolute());
}
