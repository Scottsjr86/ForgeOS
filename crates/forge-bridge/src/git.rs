//! Shell-free native Git process adapter for read-only repository inspection.
//!
//! The adapter exposes only the fixed read commands required by
//! `FORGEOS-V1-GIT-100`. It disables optional Git locks, external diff helpers,
//! prompts, pagers, global configuration, and locale-dependent output. Parsing and
//! ForgeOS domain meaning remain owned by `forge-git`.

use std::ffi::{OsStr, OsString};
use std::fmt;
use std::io;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus, Stdio};
use std::thread;
use std::time::Duration;

#[cfg(unix)]
use std::os::unix::process::ExitStatusExt;

/// Exact read-only Git operation invoked by the adapter.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GitReadRequest {
    RepositoryRoot,
    Status,
    Worktrees,
    DiffRaw(GitDiffInvocation),
    DiffPatch(GitDiffInvocation),
}

impl GitReadRequest {
    /// Stable operation label used by typed failures.
    pub const fn label(&self) -> &'static str {
        match self {
            Self::RepositoryRoot => "repository_root",
            Self::Status => "status",
            Self::Worktrees => "worktrees",
            Self::DiffRaw(_) => "diff_raw",
            Self::DiffPatch(_) => "diff_patch",
        }
    }

    fn arguments(&self) -> Vec<OsString> {
        let mut arguments = vec![OsString::from("--no-pager")];
        match self {
            Self::RepositoryRoot => {
                arguments.extend([
                    OsString::from("rev-parse"),
                    OsString::from("--is-inside-work-tree"),
                    OsString::from("--show-prefix"),
                ]);
            }
            Self::Status => {
                arguments.extend([
                    OsString::from("-c"),
                    OsString::from("status.relativePaths=false"),
                    OsString::from("status"),
                    OsString::from("--porcelain=v2"),
                    OsString::from("--branch"),
                    OsString::from("-z"),
                    OsString::from("--untracked-files=all"),
                    OsString::from("--find-renames=50%"),
                ]);
            }
            Self::Worktrees => {
                arguments.extend([
                    OsString::from("worktree"),
                    OsString::from("list"),
                    OsString::from("--porcelain"),
                    OsString::from("-z"),
                ]);
            }
            Self::DiffRaw(invocation) => {
                arguments.extend(diff_arguments(invocation, DiffOutput::Raw));
            }
            Self::DiffPatch(invocation) => {
                arguments.extend(diff_arguments(invocation, DiffOutput::Patch));
            }
        }
        arguments
    }
}

/// Exact diff endpoints selected without accepting arbitrary Git option prose.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GitDiffInvocation {
    kind: GitDiffKind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum GitDiffKind {
    Worktree,
    Staged,
    Between { base: String, target: String },
}

impl GitDiffInvocation {
    pub const fn worktree() -> Self {
        Self {
            kind: GitDiffKind::Worktree,
        }
    }

    pub const fn staged() -> Self {
        Self {
            kind: GitDiffKind::Staged,
        }
    }

    /// Creates a range only from exact canonical SHA-1 or SHA-256 object IDs.
    pub fn between(
        base: impl Into<String>,
        target: impl Into<String>,
    ) -> Result<Self, GitDiffArgumentError> {
        let base = base.into();
        let target = target.into();
        validate_object_argument("base", &base)?;
        validate_object_argument("target", &target)?;
        Ok(Self {
            kind: GitDiffKind::Between { base, target },
        })
    }
}

/// Invalid exact object argument for a read-only Git diff range.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GitDiffArgumentError {
    field: &'static str,
    value: String,
}

impl GitDiffArgumentError {
    pub const fn field(&self) -> &'static str {
        self.field
    }

    pub fn value(&self) -> &str {
        &self.value
    }
}

impl fmt::Display for GitDiffArgumentError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "Git diff {} must be an exact lowercase SHA-1 or SHA-256 object ID",
            self.field
        )
    }
}

impl std::error::Error for GitDiffArgumentError {}

fn validate_object_argument(field: &'static str, value: &str) -> Result<(), GitDiffArgumentError> {
    if matches!(value.len(), 40 | 64)
        && value
            .as_bytes()
            .iter()
            .all(|byte| matches!(byte, b'0'..=b'9' | b'a'..=b'f'))
    {
        Ok(())
    } else {
        Err(GitDiffArgumentError {
            field,
            value: value.to_owned(),
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DiffOutput {
    Raw,
    Patch,
}

fn diff_arguments(invocation: &GitDiffInvocation, output: DiffOutput) -> Vec<OsString> {
    let mut arguments = vec![
        OsString::from("diff"),
        OsString::from("--no-ext-diff"),
        OsString::from("--no-color"),
        OsString::from("--no-textconv"),
        OsString::from("--find-renames=50%"),
    ];
    if matches!(&invocation.kind, GitDiffKind::Staged) {
        arguments.push(OsString::from("--cached"));
    }
    match output {
        DiffOutput::Raw => arguments.extend([
            OsString::from("--raw"),
            OsString::from("-z"),
            OsString::from("--no-abbrev"),
        ]),
        DiffOutput::Patch => arguments.extend([
            OsString::from("--binary"),
            OsString::from("--full-index"),
            OsString::from("--src-prefix=a/"),
            OsString::from("--dst-prefix=b/"),
        ]),
    }
    if let GitDiffKind::Between { base, target } = &invocation.kind {
        arguments.push(OsString::from(base.as_str()));
        arguments.push(OsString::from(target.as_str()));
    }
    arguments.push(OsString::from("--"));
    arguments
}

/// Native child-exit information retained without translating Git failure to success.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NativeGitExit {
    success: bool,
    code: Option<i32>,
    signal: Option<i32>,
}

impl NativeGitExit {
    pub(crate) fn from_status(status: ExitStatus) -> Self {
        Self {
            success: status.success(),
            code: status.code(),
            signal: native_signal(&status),
        }
    }

    pub const fn success(self) -> bool {
        self.success
    }

    pub const fn code(self) -> Option<i32> {
        self.code
    }

    pub const fn signal(self) -> Option<i32> {
        self.signal
    }
}

/// Raw native Git output. No UTF-8 replacement or human-output parsing occurs here.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeGitOutput {
    exit: NativeGitExit,
    stdout: Vec<u8>,
    stderr: Vec<u8>,
}

impl NativeGitOutput {
    pub const fn exit(&self) -> NativeGitExit {
        self.exit
    }

    pub fn stdout(&self) -> &[u8] {
        &self.stdout
    }

    pub fn stderr(&self) -> &[u8] {
        &self.stderr
    }
}

/// Process stage that failed before native Git could report an exit status.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeGitFailureStage {
    Spawn,
    Wait,
}

/// Exact operating-system failure invoking Git.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeGitInvocationError {
    operation: &'static str,
    stage: NativeGitFailureStage,
    kind: io::ErrorKind,
    message: String,
}

impl NativeGitInvocationError {
    fn new(request: &GitReadRequest, stage: NativeGitFailureStage, error: io::Error) -> Self {
        Self {
            operation: request.label(),
            stage,
            kind: error.kind(),
            message: error.to_string(),
        }
    }

    pub const fn operation(&self) -> &'static str {
        self.operation
    }

    pub const fn stage(&self) -> NativeGitFailureStage {
        self.stage
    }

    pub const fn kind(&self) -> io::ErrorKind {
        self.kind
    }

    pub fn message(&self) -> &str {
        &self.message
    }
}

impl fmt::Display for NativeGitInvocationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "native Git {} failed during {:?}: {}",
            self.operation, self.stage, self.message
        )
    }
}

impl std::error::Error for NativeGitInvocationError {}

/// Real native Git adapter restricted to fixed read-only operations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeGitAdapter {
    program: OsString,
}

impl Default for NativeGitAdapter {
    fn default() -> Self {
        Self {
            program: OsString::from("git"),
        }
    }
}

impl NativeGitAdapter {
    /// Selects an explicit Git executable, primarily for deterministic failure tests.
    pub fn with_program(program: impl AsRef<OsStr>) -> Self {
        Self {
            program: program.as_ref().to_os_string(),
        }
    }

    pub fn program(&self) -> &OsStr {
        &self.program
    }

    /// Runs one fixed read-only request in the exact repository directory.
    pub fn invoke(
        &self,
        repository_root: impl AsRef<Path>,
        request: &GitReadRequest,
    ) -> Result<NativeGitOutput, NativeGitInvocationError> {
        let repository_root = repository_root.as_ref();
        let mut command = Command::new(&self.program);
        command
            .env_clear()
            .args(request.arguments())
            .current_dir(repository_root)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .env("GIT_OPTIONAL_LOCKS", "0")
            .env("GIT_TERMINAL_PROMPT", "0")
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env("GIT_CONFIG_GLOBAL", null_global_config())
            .env("GIT_PAGER", "cat")
            .env("PAGER", "cat")
            .env("LC_ALL", "C")
            .env("LANG", "C");
        if let Some(path) = std::env::var_os("PATH") {
            command.env("PATH", path);
        }

        let mut executable_busy_delays = [
            Duration::from_millis(1),
            Duration::from_millis(2),
            Duration::from_millis(4),
            Duration::from_millis(8),
            Duration::from_millis(16),
            Duration::from_millis(32),
            Duration::from_millis(64),
        ]
        .into_iter();
        let child = loop {
            match command.spawn() {
                Ok(child) => break child,
                Err(error) if error.kind() == io::ErrorKind::ExecutableFileBusy => {
                    let Some(delay) = executable_busy_delays.next() else {
                        return Err(NativeGitInvocationError::new(
                            request,
                            NativeGitFailureStage::Spawn,
                            error,
                        ));
                    };
                    thread::sleep(delay);
                }
                Err(error) => {
                    return Err(NativeGitInvocationError::new(
                        request,
                        NativeGitFailureStage::Spawn,
                        error,
                    ));
                }
            }
        };
        let output = child.wait_with_output().map_err(|error| {
            NativeGitInvocationError::new(request, NativeGitFailureStage::Wait, error)
        })?;
        Ok(NativeGitOutput {
            exit: NativeGitExit::from_status(output.status),
            stdout: output.stdout,
            stderr: output.stderr,
        })
    }
}

fn null_global_config() -> PathBuf {
    #[cfg(unix)]
    {
        PathBuf::from("/dev/null")
    }
    #[cfg(not(unix))]
    {
        PathBuf::from("NUL")
    }
}

#[cfg(unix)]
fn native_signal(status: &ExitStatus) -> Option<i32> {
    status.signal()
}

#[cfg(not(unix))]
fn native_signal(_status: &ExitStatus) -> Option<i32> {
    None
}

#[cfg(all(test, target_os = "linux"))]
mod tests {
    use super::{GitReadRequest, NativeGitAdapter};
    use std::fs::{self, OpenOptions};
    use std::io;
    use std::os::unix::fs::PermissionsExt;
    use std::process::Command;
    use std::thread;
    use std::time::Duration;

    #[test]
    fn executable_file_busy_is_retried_before_git_invocation_fails() {
        let root = std::env::temp_dir().join(format!(
            "forgeos-native-git-busy-retry-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir(&root).unwrap();
        let program = root.join("fake-git");
        fs::write(&program, b"#!/bin/sh\nprintf 'true\\n\\n'\n").unwrap();
        fs::set_permissions(&program, fs::Permissions::from_mode(0o755)).unwrap();

        let writer = OpenOptions::new().write(true).open(&program).unwrap();
        let error = Command::new(&program).output().unwrap_err();
        assert_eq!(error.kind(), io::ErrorKind::ExecutableFileBusy);
        let release = thread::spawn(move || {
            thread::sleep(Duration::from_millis(10));
            drop(writer);
        });

        let output = NativeGitAdapter::with_program(&program)
            .invoke(&root, &GitReadRequest::RepositoryRoot)
            .unwrap();
        release.join().unwrap();
        assert!(output.exit().success());
        assert_eq!(output.stdout(), b"true\n\n");
        fs::remove_dir_all(root).unwrap();
    }
}
