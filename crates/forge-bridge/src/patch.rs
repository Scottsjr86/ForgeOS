//! Fixed native Git patch validation and application adapter.
//!
//! Callers provide already validated patch bytes. This adapter exposes only
//! `git apply --check` and ordinary all-or-nothing `git apply`; reject, 3-way,
//! index, cached, unsafe-path, and arbitrary option surfaces do not exist.

use crate::git::NativeGitExit;
use std::ffi::{OsStr, OsString};
use std::fmt;
use std::io::{self, Write};
use std::path::Path;
use std::process::{Command, Stdio};

/// Exact native operation used by patch application.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativePatchOperation {
    Check,
    Apply,
}

impl NativePatchOperation {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Check => "check",
            Self::Apply => "apply",
        }
    }
}

/// Native Git output retained without translating failure into success.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativePatchOutput {
    operation: NativePatchOperation,
    exit: NativeGitExit,
    stdout: Vec<u8>,
    stderr: Vec<u8>,
}

impl NativePatchOutput {
    pub const fn operation(&self) -> NativePatchOperation {
        self.operation
    }

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

/// Failure to start or communicate with native Git.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativePatchInvocationError {
    operation: NativePatchOperation,
    kind: io::ErrorKind,
    message: String,
}

impl NativePatchInvocationError {
    pub const fn operation(&self) -> NativePatchOperation {
        self.operation
    }

    pub const fn kind(&self) -> io::ErrorKind {
        self.kind
    }
}

impl fmt::Display for NativePatchInvocationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "failed to invoke native Git patch {}: {}",
            self.operation.label(),
            self.message
        )
    }
}

impl std::error::Error for NativePatchInvocationError {}

/// Shell-free native Git adapter with one fixed executable.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativePatchAdapter {
    program: OsString,
}

impl Default for NativePatchAdapter {
    fn default() -> Self {
        Self {
            program: OsString::from("git"),
        }
    }
}

impl NativePatchAdapter {
    pub fn with_program(program: impl AsRef<OsStr>) -> Self {
        Self {
            program: program.as_ref().to_os_string(),
        }
    }

    pub fn invoke(
        &self,
        root: &Path,
        operation: NativePatchOperation,
        patch_bytes: &[u8],
    ) -> Result<NativePatchOutput, NativePatchInvocationError> {
        let mut command = Command::new(&self.program);
        command
            .current_dir(root)
            .env_clear()
            .env("PATH", std::env::var_os("PATH").unwrap_or_default())
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env("GIT_CONFIG_GLOBAL", "/dev/null")
            .env("GIT_TERMINAL_PROMPT", "0")
            .env("GIT_OPTIONAL_LOCKS", "0")
            .env("LC_ALL", "C")
            .arg("--no-pager")
            .arg("-c")
            .arg("core.autocrlf=false")
            .arg("-c")
            .arg("core.safecrlf=false")
            .arg("-c")
            .arg("apply.ignoreWhitespace=false")
            .arg("apply")
            .arg("--binary")
            .arg("--whitespace=nowarn");
        if operation == NativePatchOperation::Check {
            command.arg("--check");
        }
        command
            .arg("--")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        let mut child = command
            .spawn()
            .map_err(|error| invocation_error(operation, error))?;
        let Some(mut stdin) = child.stdin.take() else {
            let _ = child.kill();
            let _ = child.wait();
            return Err(NativePatchInvocationError {
                operation,
                kind: io::ErrorKind::BrokenPipe,
                message: "native Git child did not expose stdin".to_owned(),
            });
        };
        if let Err(error) = stdin.write_all(patch_bytes).and_then(|_| stdin.flush()) {
            drop(stdin);
            let _ = child.kill();
            let _ = child.wait();
            return Err(invocation_error(operation, error));
        }
        drop(stdin);
        let output = child
            .wait_with_output()
            .map_err(|error| invocation_error(operation, error))?;
        Ok(NativePatchOutput {
            operation,
            exit: NativeGitExit::from_status(output.status),
            stdout: output.stdout,
            stderr: output.stderr,
        })
    }
}

fn invocation_error(
    operation: NativePatchOperation,
    error: io::Error,
) -> NativePatchInvocationError {
    NativePatchInvocationError {
        operation,
        kind: error.kind(),
        message: error.to_string(),
    }
}
