//! Shell-free native Git mutation adapter for exact, prevalidated requests.
//!
//! This module owns process invocation only. Repository identity, precondition
//! checks, destructive confirmation, and interpretation of resulting state remain
//! owned by `forge-git`.

use crate::git::NativeGitExit;
use std::ffi::{OsStr, OsString};
use std::fmt;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

/// One fixed mutation operation. Fields are private so callers cannot inject
/// arbitrary Git options or shell prose.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GitMutationRequest {
    kind: GitMutationKind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum GitMutationKind {
    Stage {
        paths: Vec<OsString>,
    },
    Unstage {
        paths: Vec<OsString>,
    },
    RestoreWorktree {
        source: String,
        paths: Vec<OsString>,
    },
    Commit {
        message: Vec<u8>,
        author_name: String,
        author_email: String,
    },
    CreateWorktree {
        path: OsString,
        branch: String,
        start: String,
    },
    RemoveWorktree {
        path: OsString,
    },
}

impl GitMutationRequest {
    pub fn stage(paths: Vec<OsString>) -> Result<Self, GitMutationArgumentError> {
        validate_paths(&paths)?;
        Ok(Self {
            kind: GitMutationKind::Stage { paths },
        })
    }

    pub fn unstage(paths: Vec<OsString>) -> Result<Self, GitMutationArgumentError> {
        validate_paths(&paths)?;
        Ok(Self {
            kind: GitMutationKind::Unstage { paths },
        })
    }

    pub fn restore_worktree(
        source: impl Into<String>,
        paths: Vec<OsString>,
    ) -> Result<Self, GitMutationArgumentError> {
        let source = source.into();
        validate_object_id("source", &source)?;
        validate_paths(&paths)?;
        Ok(Self {
            kind: GitMutationKind::RestoreWorktree { source, paths },
        })
    }

    pub fn commit(
        message: impl Into<Vec<u8>>,
        author_name: impl Into<String>,
        author_email: impl Into<String>,
    ) -> Result<Self, GitMutationArgumentError> {
        let message = message.into();
        let author_name = author_name.into();
        let author_email = author_email.into();
        if message.is_empty() {
            return Err(GitMutationArgumentError::EmptyCommitMessage);
        }
        if message.contains(&0) {
            return Err(GitMutationArgumentError::CommitMessageContainsNul);
        }
        validate_identity_field("author_name", &author_name)?;
        validate_identity_field("author_email", &author_email)?;
        Ok(Self {
            kind: GitMutationKind::Commit {
                message,
                author_name,
                author_email,
            },
        })
    }

    pub fn create_worktree(
        path: impl AsRef<OsStr>,
        branch: impl Into<String>,
        start: impl Into<String>,
    ) -> Result<Self, GitMutationArgumentError> {
        let path = path.as_ref().to_os_string();
        validate_absolute_path("worktree_path", &path)?;
        let branch = branch.into();
        validate_branch_name(&branch)?;
        let start = start.into();
        validate_object_id("start", &start)?;
        Ok(Self {
            kind: GitMutationKind::CreateWorktree {
                path,
                branch,
                start,
            },
        })
    }

    pub fn remove_worktree(path: impl AsRef<OsStr>) -> Result<Self, GitMutationArgumentError> {
        let path = path.as_ref().to_os_string();
        validate_absolute_path("worktree_path", &path)?;
        Ok(Self {
            kind: GitMutationKind::RemoveWorktree { path },
        })
    }

    pub const fn label(&self) -> &'static str {
        match &self.kind {
            GitMutationKind::Stage { .. } => "stage",
            GitMutationKind::Unstage { .. } => "unstage",
            GitMutationKind::RestoreWorktree { .. } => "restore_worktree",
            GitMutationKind::Commit { .. } => "commit",
            GitMutationKind::CreateWorktree { .. } => "create_worktree",
            GitMutationKind::RemoveWorktree { .. } => "remove_worktree",
        }
    }

    fn arguments(&self) -> Vec<OsString> {
        let mut arguments = vec![
            OsString::from("--no-pager"),
            OsString::from("-c"),
            OsString::from("core.hooksPath=/dev/null"),
            OsString::from("-c"),
            OsString::from("core.fsmonitor=false"),
            OsString::from("-c"),
            OsString::from("commit.gpgSign=false"),
        ];
        match &self.kind {
            GitMutationKind::Stage { paths } => {
                arguments.extend([
                    OsString::from("--literal-pathspecs"),
                    OsString::from("add"),
                    OsString::from("--"),
                ]);
                arguments.extend(paths.iter().cloned());
            }
            GitMutationKind::Unstage { paths } => {
                arguments.extend([
                    OsString::from("--literal-pathspecs"),
                    OsString::from("reset"),
                    OsString::from("--quiet"),
                    OsString::from("HEAD"),
                    OsString::from("--"),
                ]);
                arguments.extend(paths.iter().cloned());
            }
            GitMutationKind::RestoreWorktree { source, paths } => {
                arguments.extend([
                    OsString::from("--literal-pathspecs"),
                    OsString::from("restore"),
                    OsString::from("--worktree"),
                    OsString::from(format!("--source={source}")),
                    OsString::from("--"),
                ]);
                arguments.extend(paths.iter().cloned());
            }
            GitMutationKind::Commit { .. } => {
                arguments.extend([
                    OsString::from("commit"),
                    OsString::from("--quiet"),
                    OsString::from("--no-verify"),
                    OsString::from("--no-gpg-sign"),
                    OsString::from("--cleanup=verbatim"),
                    OsString::from("--file=-"),
                ]);
            }
            GitMutationKind::CreateWorktree {
                path,
                branch,
                start,
            } => {
                arguments.extend([
                    OsString::from("worktree"),
                    OsString::from("add"),
                    OsString::from("--no-track"),
                    OsString::from("-b"),
                    OsString::from(branch),
                    OsString::from("--"),
                    path.clone(),
                    OsString::from(start),
                ]);
            }
            GitMutationKind::RemoveWorktree { path } => {
                arguments.extend([
                    OsString::from("worktree"),
                    OsString::from("remove"),
                    OsString::from("--"),
                    path.clone(),
                ]);
            }
        }
        arguments
    }

    fn stdin_bytes(&self) -> Option<&[u8]> {
        match &self.kind {
            GitMutationKind::Commit { message, .. } => Some(message),
            _ => None,
        }
    }

    fn commit_identity(&self) -> Option<(&str, &str)> {
        match &self.kind {
            GitMutationKind::Commit {
                author_name,
                author_email,
                ..
            } => Some((author_name, author_email)),
            _ => None,
        }
    }
}

/// Exact request-validation failure before Git can be invoked.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GitMutationArgumentError {
    EmptyPathSet,
    EmptyPath { index: usize },
    PathContainsNul { index: usize },
    AbsolutePath { index: usize },
    EmptyCommitMessage,
    CommitMessageContainsNul,
    InvalidIdentityField { field: &'static str },
    InvalidObjectId { field: &'static str },
    InvalidBranchName,
    WorktreePathNotAbsolute,
    WorktreePathContainsNul,
}

impl fmt::Display for GitMutationArgumentError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyPathSet => formatter.write_str("Git mutation requires at least one path"),
            Self::EmptyPath { index } => write!(formatter, "Git mutation path {index} is empty"),
            Self::PathContainsNul { index } => {
                write!(formatter, "Git mutation path {index} contains NUL")
            }
            Self::AbsolutePath { index } => {
                write!(
                    formatter,
                    "Git mutation path {index} must be repository-relative"
                )
            }
            Self::EmptyCommitMessage => formatter.write_str("Git commit message is empty"),
            Self::CommitMessageContainsNul => {
                formatter.write_str("Git commit message contains NUL")
            }
            Self::InvalidIdentityField { field } => {
                write!(formatter, "Git commit identity field {field} is invalid")
            }
            Self::InvalidObjectId { field } => write!(
                formatter,
                "Git mutation {field} must be an exact lowercase SHA-1 or SHA-256 object ID"
            ),
            Self::InvalidBranchName => formatter.write_str("Git branch name is invalid"),
            Self::WorktreePathNotAbsolute => {
                formatter.write_str("Git worktree path must be absolute")
            }
            Self::WorktreePathContainsNul => formatter.write_str("Git worktree path contains NUL"),
        }
    }
}

impl std::error::Error for GitMutationArgumentError {}

fn validate_paths(paths: &[OsString]) -> Result<(), GitMutationArgumentError> {
    if paths.is_empty() {
        return Err(GitMutationArgumentError::EmptyPathSet);
    }
    for (index, path) in paths.iter().enumerate() {
        let bytes = path.as_encoded_bytes();
        if bytes.is_empty() {
            return Err(GitMutationArgumentError::EmptyPath { index });
        }
        if bytes.contains(&0) {
            return Err(GitMutationArgumentError::PathContainsNul { index });
        }
        if Path::new(path).is_absolute() {
            return Err(GitMutationArgumentError::AbsolutePath { index });
        }
    }
    Ok(())
}

fn validate_object_id(field: &'static str, value: &str) -> Result<(), GitMutationArgumentError> {
    if matches!(value.len(), 40 | 64)
        && value
            .bytes()
            .all(|byte| matches!(byte, b'0'..=b'9' | b'a'..=b'f'))
    {
        Ok(())
    } else {
        Err(GitMutationArgumentError::InvalidObjectId { field })
    }
}

fn validate_identity_field(
    field: &'static str,
    value: &str,
) -> Result<(), GitMutationArgumentError> {
    if value.is_empty() || value.bytes().any(|byte| matches!(byte, 0 | b'\n' | b'\r')) {
        Err(GitMutationArgumentError::InvalidIdentityField { field })
    } else {
        Ok(())
    }
}

fn validate_branch_name(value: &str) -> Result<(), GitMutationArgumentError> {
    let valid = !value.is_empty()
        && !value.starts_with('-')
        && !value.starts_with('/')
        && !value.ends_with('/')
        && !value.ends_with('.')
        && !value.ends_with(".lock")
        && !value.contains("..")
        && !value.contains("@{")
        && !value.contains("//")
        && value != "@"
        && value
            .split('/')
            .all(|part| !part.starts_with('.') && !part.ends_with('.'))
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b'/'));
    if valid {
        Ok(())
    } else {
        Err(GitMutationArgumentError::InvalidBranchName)
    }
}

fn validate_absolute_path(
    _field: &'static str,
    value: &OsStr,
) -> Result<(), GitMutationArgumentError> {
    let bytes = value.as_encoded_bytes();
    if bytes.contains(&0) {
        return Err(GitMutationArgumentError::WorktreePathContainsNul);
    }
    if !Path::new(value).is_absolute() {
        return Err(GitMutationArgumentError::WorktreePathNotAbsolute);
    }
    Ok(())
}

/// Process stage that failed before a native Git exit could be captured.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeGitMutationFailureStage {
    Spawn,
    StdinWrite,
    Wait,
}

/// Exact operating-system failure invoking a Git mutation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeGitMutationInvocationError {
    operation: &'static str,
    stage: NativeGitMutationFailureStage,
    kind: io::ErrorKind,
    message: String,
}

impl NativeGitMutationInvocationError {
    fn new(
        request: &GitMutationRequest,
        stage: NativeGitMutationFailureStage,
        error: io::Error,
    ) -> Self {
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

    pub const fn stage(&self) -> NativeGitMutationFailureStage {
        self.stage
    }

    pub const fn kind(&self) -> io::ErrorKind {
        self.kind
    }

    pub fn message(&self) -> &str {
        &self.message
    }
}

impl fmt::Display for NativeGitMutationInvocationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "native Git mutation {} failed during {:?}: {}",
            self.operation, self.stage, self.message
        )
    }
}

impl std::error::Error for NativeGitMutationInvocationError {}

/// Raw native mutation output with exact stdout, stderr, and exit status.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeGitMutationOutput {
    exit: NativeGitExit,
    stdout: Vec<u8>,
    stderr: Vec<u8>,
}

impl NativeGitMutationOutput {
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

/// Real native Git adapter restricted to the fixed mutation request surface.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeGitMutationAdapter {
    program: OsString,
}

impl Default for NativeGitMutationAdapter {
    fn default() -> Self {
        Self {
            program: OsString::from("git"),
        }
    }
}

impl NativeGitMutationAdapter {
    pub fn with_program(program: impl AsRef<OsStr>) -> Self {
        Self {
            program: program.as_ref().to_os_string(),
        }
    }

    pub fn invoke(
        &self,
        repository_root: impl AsRef<Path>,
        request: &GitMutationRequest,
    ) -> Result<NativeGitMutationOutput, NativeGitMutationInvocationError> {
        let mut command = Command::new(&self.program);
        command
            .env_clear()
            .args(request.arguments())
            .current_dir(repository_root)
            .stdin(if request.stdin_bytes().is_some() {
                Stdio::piped()
            } else {
                Stdio::null()
            })
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
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
        if let Some((name, email)) = request.commit_identity() {
            command
                .env("GIT_AUTHOR_NAME", name)
                .env("GIT_AUTHOR_EMAIL", email)
                .env("GIT_COMMITTER_NAME", name)
                .env("GIT_COMMITTER_EMAIL", email);
        }

        let mut child = command.spawn().map_err(|error| {
            NativeGitMutationInvocationError::new(
                request,
                NativeGitMutationFailureStage::Spawn,
                error,
            )
        })?;
        if let Some(bytes) = request.stdin_bytes() {
            let write_result = child
                .stdin
                .take()
                .expect("piped mutation stdin")
                .write_all(bytes);
            if let Err(error) = write_result {
                let _ = child.kill();
                let _ = child.wait();
                return Err(NativeGitMutationInvocationError::new(
                    request,
                    NativeGitMutationFailureStage::StdinWrite,
                    error,
                ));
            }
        }
        let output = child.wait_with_output().map_err(|error| {
            NativeGitMutationInvocationError::new(
                request,
                NativeGitMutationFailureStage::Wait,
                error,
            )
        })?;
        Ok(NativeGitMutationOutput {
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
