//! Dedicated display-manager session bootstrap.
//!
//! The bootstrap launches one explicitly configured ForgeOS composition root from a
//! deterministic environment. It never sources a shell profile, inherits the current
//! directory, or discovers an executable from a worktree.

use std::collections::BTreeMap;
use std::ffi::{OsStr, OsString};
use std::fmt;
use std::io;
use std::os::unix::process::ExitStatusExt;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus, Stdio};

/// Canonical installed ForgeOS composition-root path.
pub const DEFAULT_COMPOSITION_ROOT: &str = "/usr/bin/forge-app";

/// Canonical installed session-launcher path referenced by the desktop entry.
pub const DEFAULT_SESSION_LAUNCHER: &str = "/usr/libexec/forgeos/forgeos-session-launcher";

/// Deterministic executable search path supplied to the composition root.
pub const SESSION_PATH: &str = "/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin";

/// Canonical display-manager entry source installed later by packaging.
pub const DISPLAY_MANAGER_ENTRY: &str = include_str!("../assets/forgeos.desktop");

/// Display backend inherited from the display manager.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DisplayBackend {
    Wayland,
    X11,
}

impl DisplayBackend {
    /// Canonical `XDG_SESSION_TYPE` value.
    pub const fn session_type(self) -> &'static str {
        match self {
            Self::Wayland => "wayland",
            Self::X11 => "x11",
        }
    }
}

/// Sanitized environment passed to the ForgeOS composition root.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionEnvironment {
    backend: DisplayBackend,
    variables: BTreeMap<OsString, OsString>,
}

impl SessionEnvironment {
    /// Builds the exact child environment from display-manager variables.
    pub fn from_parent<I>(variables: I) -> Result<Self, SessionEnvironmentError>
    where
        I: IntoIterator<Item = (OsString, OsString)>,
    {
        let inherited: BTreeMap<OsString, OsString> = variables.into_iter().collect();
        let home = required_absolute(&inherited, "HOME")?;
        let runtime = required_absolute(&inherited, "XDG_RUNTIME_DIR")?;

        let wayland = nonempty_value(&inherited, "WAYLAND_DISPLAY");
        let x11 = nonempty_value(&inherited, "DISPLAY");
        let backend = if wayland.is_some() {
            DisplayBackend::Wayland
        } else if x11.is_some() {
            DisplayBackend::X11
        } else {
            return Err(SessionEnvironmentError::MissingDisplay);
        };

        let mut sanitized = BTreeMap::new();
        sanitized.insert(OsString::from("HOME"), home);
        sanitized.insert(OsString::from("XDG_RUNTIME_DIR"), runtime);
        sanitized.insert(OsString::from("PATH"), OsString::from(SESSION_PATH));
        sanitized.insert(
            OsString::from("XDG_DATA_DIRS"),
            OsString::from("/usr/local/share:/usr/share"),
        );
        sanitized.insert(
            OsString::from("XDG_CONFIG_DIRS"),
            OsString::from("/etc/xdg"),
        );
        sanitized.insert(
            OsString::from("XDG_CURRENT_DESKTOP"),
            OsString::from("ForgeOS"),
        );
        sanitized.insert(
            OsString::from("XDG_SESSION_DESKTOP"),
            OsString::from("forgeos"),
        );
        sanitized.insert(OsString::from("DESKTOP_SESSION"), OsString::from("forgeos"));
        sanitized.insert(
            OsString::from("XDG_SESSION_TYPE"),
            OsString::from(backend.session_type()),
        );
        sanitized.insert(OsString::from("XDG_SESSION_CLASS"), OsString::from("user"));
        sanitized.insert(OsString::from("FORGEOS_SESSION"), OsString::from("1"));

        if let Some(value) = wayland {
            sanitized.insert(OsString::from("WAYLAND_DISPLAY"), value);
        }
        if let Some(value) = x11 {
            sanitized.insert(OsString::from("DISPLAY"), value);
        }

        for name in [
            "USER",
            "LOGNAME",
            "DBUS_SESSION_BUS_ADDRESS",
            "XDG_SEAT",
            "XDG_VTNR",
        ] {
            copy_nonempty(&inherited, &mut sanitized, name);
        }
        for name in [
            "XDG_CONFIG_HOME",
            "XDG_DATA_HOME",
            "XDG_CACHE_HOME",
            "XDG_STATE_HOME",
            "XAUTHORITY",
            "SSH_AUTH_SOCK",
        ] {
            copy_optional_absolute(&inherited, &mut sanitized, name)?;
        }
        for (name, value) in &inherited {
            if let Some(name_text) = name.to_str() {
                if name_text == "LANG" || name_text == "LANGUAGE" || name_text.starts_with("LC_") {
                    if !value.is_empty() {
                        sanitized.insert(name.clone(), value.clone());
                    }
                }
            }
        }

        Ok(Self {
            backend,
            variables: sanitized,
        })
    }

    /// Selected display backend.
    pub const fn backend(&self) -> DisplayBackend {
        self.backend
    }

    /// Exact sanitized variables in stable key order.
    pub fn variables(&self) -> &BTreeMap<OsString, OsString> {
        &self.variables
    }

    /// Reads one sanitized variable.
    pub fn get(&self, name: &str) -> Option<&OsStr> {
        self.variables
            .get(OsStr::new(name))
            .map(OsString::as_os_str)
    }
}

fn required_absolute(
    variables: &BTreeMap<OsString, OsString>,
    name: &'static str,
) -> Result<OsString, SessionEnvironmentError> {
    let value = variables
        .get(OsStr::new(name))
        .filter(|value| !value.is_empty())
        .cloned()
        .ok_or(SessionEnvironmentError::MissingRequired(name))?;
    if !Path::new(value.as_os_str()).is_absolute() {
        return Err(SessionEnvironmentError::RelativePath { name, value });
    }
    Ok(value)
}

fn nonempty_value(variables: &BTreeMap<OsString, OsString>, name: &str) -> Option<OsString> {
    variables
        .get(OsStr::new(name))
        .filter(|value| !value.is_empty())
        .cloned()
}

fn copy_nonempty(
    source: &BTreeMap<OsString, OsString>,
    target: &mut BTreeMap<OsString, OsString>,
    name: &str,
) {
    if let Some(value) = nonempty_value(source, name) {
        target.insert(OsString::from(name), value);
    }
}

fn copy_optional_absolute(
    source: &BTreeMap<OsString, OsString>,
    target: &mut BTreeMap<OsString, OsString>,
    name: &'static str,
) -> Result<(), SessionEnvironmentError> {
    let Some(value) = nonempty_value(source, name) else {
        return Ok(());
    };
    if !Path::new(value.as_os_str()).is_absolute() {
        return Err(SessionEnvironmentError::RelativePath { name, value });
    }
    target.insert(OsString::from(name), value);
    Ok(())
}

/// Invalid display-manager environment.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SessionEnvironmentError {
    MissingRequired(&'static str),
    RelativePath { name: &'static str, value: OsString },
    MissingDisplay,
}

impl fmt::Display for SessionEnvironmentError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingRequired(name) => {
                write!(formatter, "required session variable {name} is missing")
            }
            Self::RelativePath { name, value } => write!(
                formatter,
                "session variable {name} must be an absolute path, got {:?}",
                value
            ),
            Self::MissingDisplay => {
                formatter.write_str("display manager supplied neither WAYLAND_DISPLAY nor DISPLAY")
            }
        }
    }
}

impl std::error::Error for SessionEnvironmentError {}

/// Exact launch request for the ForgeOS composition root.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionLaunchRequest {
    composition_root: PathBuf,
    arguments: Vec<OsString>,
    environment: SessionEnvironment,
}

impl SessionLaunchRequest {
    /// Creates a launch request with an absolute executable path.
    pub fn new(
        composition_root: PathBuf,
        arguments: Vec<OsString>,
        environment: SessionEnvironment,
    ) -> Result<Self, SessionLaunchError> {
        if !composition_root.is_absolute() {
            return Err(SessionLaunchError::RelativeCompositionRoot(
                composition_root,
            ));
        }
        Ok(Self {
            composition_root,
            arguments,
            environment,
        })
    }

    pub fn composition_root(&self) -> &Path {
        &self.composition_root
    }

    pub fn arguments(&self) -> &[OsString] {
        &self.arguments
    }

    pub const fn environment(&self) -> &SessionEnvironment {
        &self.environment
    }
}

/// Exact termination state of the composition root.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SessionLaunchOutcome {
    exit_code: Option<i32>,
    signal: Option<i32>,
}

impl SessionLaunchOutcome {
    fn from_status(status: ExitStatus) -> Self {
        Self {
            exit_code: status.code(),
            signal: status.signal(),
        }
    }

    pub const fn exit_code(self) -> Option<i32> {
        self.exit_code
    }

    pub const fn signal(self) -> Option<i32> {
        self.signal
    }

    /// Process exit code returned by the launcher itself.
    pub fn launcher_exit_code(self) -> i32 {
        if let Some(code) = self.exit_code {
            return code;
        }
        self.signal
            .and_then(|signal| 128_i32.checked_add(signal))
            .unwrap_or(1)
    }
}

/// Launches the exact composition root without a shell or inherited worktree.
pub fn launch_session(
    request: &SessionLaunchRequest,
) -> Result<SessionLaunchOutcome, SessionLaunchError> {
    let mut command = Command::new(request.composition_root());
    command
        .args(request.arguments())
        .current_dir("/")
        .env_clear()
        .envs(request.environment().variables())
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit());

    let status = command
        .status()
        .map_err(|source| SessionLaunchError::Spawn {
            executable: request.composition_root().to_path_buf(),
            source,
        })?;
    Ok(SessionLaunchOutcome::from_status(status))
}

/// Session launch failure before a child exit status exists.
#[derive(Debug)]
pub enum SessionLaunchError {
    RelativeCompositionRoot(PathBuf),
    Spawn {
        executable: PathBuf,
        source: io::Error,
    },
}

impl fmt::Display for SessionLaunchError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::RelativeCompositionRoot(path) => write!(
                formatter,
                "ForgeOS composition root must be absolute, got {}",
                path.display()
            ),
            Self::Spawn { executable, source } => write!(
                formatter,
                "failed to launch ForgeOS composition root {}: {source}",
                executable.display()
            ),
        }
    }
}

impl std::error::Error for SessionLaunchError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Spawn { source, .. } => Some(source),
            Self::RelativeCompositionRoot(_) => None,
        }
    }
}
