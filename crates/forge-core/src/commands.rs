//! Canonical registered-command definitions and immutable identity policy.
//!
//! A registered command is structured data, never mutable shell prose. Its stable
//! [`CommandId`] names one exact executable, argument vector, repository binding,
//! working directory, environment policy, timeout, cancellation policy, and
//! authority class. Changing any of those fields requires an explicit replacement
//! against the expected prior definition identity.

use forge_protocol::hashes::{hash_canonical_bytes, ContentHash, HashDomain};
use forge_protocol::identities::{CommandId, RepositoryId};
use forge_protocol::paths::RepositoryRelativePath;
use std::collections::BTreeMap;
use std::fmt;
use std::time::Duration;

const COMMAND_MAGIC: &[u8; 12] = b"FORGECMD\0\0\0\0";
const COMMAND_SCHEMA_VERSION: u8 = 1;
const MAX_DISPLAY_NAME_BYTES: usize = 128;
const MAX_PROGRAM_BYTES: usize = 4096;
const MAX_ARGUMENTS: usize = 256;
const MAX_ARGUMENT_BYTES: usize = 16 * 1024;
const MAX_ENVIRONMENT_VARIABLES: usize = 256;
const MAX_ENVIRONMENT_NAME_BYTES: usize = 128;
const MAX_ENVIRONMENT_VALUE_BYTES: usize = 64 * 1024;
const MAX_TIMEOUT_MILLIS: u64 = 24 * 60 * 60 * 1000;

/// Declared command authority. Higher classes are not implied by lower classes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
pub enum CommandAuthorityClass {
    /// Intended to inspect state without modifying repository or host state.
    Inspect = 1,
    /// May write declared build products or caches, but not source or Git metadata.
    Build = 2,
    /// May modify declared workspace files.
    WorkspaceWrite = 3,
    /// May intentionally mutate repository source or Git metadata.
    RepositoryMutation = 4,
    /// May intentionally mutate host state outside the registered workspace.
    HostMutation = 5,
}

impl CommandAuthorityClass {
    pub const fn code(self) -> u8 {
        self as u8
    }

    pub const fn label(self) -> &'static str {
        match self {
            Self::Inspect => "inspect",
            Self::Build => "build",
            Self::WorkspaceWrite => "workspace_write",
            Self::RepositoryMutation => "repository_mutation",
            Self::HostMutation => "host_mutation",
        }
    }
}

/// Explicit V1 cancellation behavior for one command.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
pub enum CommandCancellationPolicy {
    /// Terminate the isolated process group owned by the command execution.
    TerminateProcessGroup = 1,
}

impl CommandCancellationPolicy {
    pub const fn code(self) -> u8 {
        self as u8
    }
}

/// Explicit timeout policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum CommandTimeout {
    Unlimited,
    Milliseconds(u64),
}

impl CommandTimeout {
    pub fn after(duration: Duration) -> Result<Self, CommandDefinitionError> {
        let millis = u64::try_from(duration.as_millis()).map_err(|_| {
            CommandDefinitionError::TimeoutTooLarge {
                maximum_millis: MAX_TIMEOUT_MILLIS,
                actual_millis: u64::MAX,
            }
        })?;
        if millis == 0 {
            return Err(CommandDefinitionError::ZeroTimeout);
        }
        if millis > MAX_TIMEOUT_MILLIS {
            return Err(CommandDefinitionError::TimeoutTooLarge {
                maximum_millis: MAX_TIMEOUT_MILLIS,
                actual_millis: millis,
            });
        }
        Ok(Self::Milliseconds(millis))
    }

    pub const fn milliseconds(self) -> Option<u64> {
        match self {
            Self::Unlimited => None,
            Self::Milliseconds(value) => Some(value),
        }
    }
}

/// Repository-bound working directory declared by a command definition.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CommandWorkingDirectory {
    relative: Option<RepositoryRelativePath>,
}

impl CommandWorkingDirectory {
    pub const fn repository_root() -> Self {
        Self { relative: None }
    }

    pub fn relative(
        relative: RepositoryRelativePath,
    ) -> Result<Self, CommandDefinitionError> {
        if relative.as_path().to_str().is_none() {
            return Err(CommandDefinitionError::WorkingDirectoryNotUtf8);
        }
        Ok(Self {
            relative: Some(relative),
        })
    }

    pub fn relative_path(&self) -> Option<&RepositoryRelativePath> {
        self.relative.as_ref()
    }

    pub const fn is_repository_root(&self) -> bool {
        self.relative.is_none()
    }
}

/// Source of one declared command environment variable.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum CommandEnvironmentSource {
    Literal(String),
    InheritDeclared,
}

/// One explicitly named environment variable. Undeclared variables are cleared.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CommandEnvironmentVariable {
    name: String,
    source: CommandEnvironmentSource,
}

impl CommandEnvironmentVariable {
    pub fn literal(
        name: impl Into<String>,
        value: impl Into<String>,
    ) -> Result<Self, CommandDefinitionError> {
        let name = name.into();
        let value = value.into();
        validate_environment_name(&name)?;
        validate_environment_value(&name, &value)?;
        Ok(Self {
            name,
            source: CommandEnvironmentSource::Literal(value),
        })
    }

    pub fn inherit(name: impl Into<String>) -> Result<Self, CommandDefinitionError> {
        let name = name.into();
        validate_environment_name(&name)?;
        Ok(Self {
            name,
            source: CommandEnvironmentSource::InheritDeclared,
        })
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn source(&self) -> &CommandEnvironmentSource {
        &self.source
    }

    /// Validates one resolved value before it enters an operating-system launch.
    pub fn validate_resolved_value(
        &self,
        value: &str,
    ) -> Result<(), CommandDefinitionError> {
        validate_environment_value(&self.name, value)
    }
}

/// Clear-parent environment policy with only declared literal or inherited values.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandEnvironmentPolicy {
    variables: Vec<CommandEnvironmentVariable>,
}

impl CommandEnvironmentPolicy {
    pub fn clear(
        mut variables: Vec<CommandEnvironmentVariable>,
    ) -> Result<Self, CommandDefinitionError> {
        if variables.len() > MAX_ENVIRONMENT_VARIABLES {
            return Err(CommandDefinitionError::TooManyEnvironmentVariables {
                maximum: MAX_ENVIRONMENT_VARIABLES,
                actual: variables.len(),
            });
        }
        variables.sort_by(|left, right| left.name.as_bytes().cmp(right.name.as_bytes()));
        for pair in variables.windows(2) {
            if pair[0].name == pair[1].name {
                return Err(CommandDefinitionError::DuplicateEnvironmentVariable(
                    pair[0].name.clone(),
                ));
            }
        }
        Ok(Self { variables })
    }

    pub const fn empty() -> Self {
        Self {
            variables: Vec::new(),
        }
    }

    pub fn variables(&self) -> &[CommandEnvironmentVariable] {
        &self.variables
    }

    pub const fn clears_parent_environment(&self) -> bool {
        true
    }
}

/// One immutable registered-command definition.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RegisteredCommand {
    command_id: CommandId,
    repository_id: RepositoryId,
    display_name: String,
    program: String,
    arguments: Vec<String>,
    working_directory: CommandWorkingDirectory,
    environment: CommandEnvironmentPolicy,
    timeout: CommandTimeout,
    cancellation: CommandCancellationPolicy,
    authority: CommandAuthorityClass,
}

impl RegisteredCommand {
    #[allow(clippy::too_many_arguments)]
    pub fn new<I, S>(
        command_id: CommandId,
        repository_id: RepositoryId,
        display_name: impl Into<String>,
        program: impl Into<String>,
        arguments: I,
        working_directory: CommandWorkingDirectory,
        environment: CommandEnvironmentPolicy,
        timeout: CommandTimeout,
        cancellation: CommandCancellationPolicy,
        authority: CommandAuthorityClass,
    ) -> Result<Self, CommandDefinitionError>
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        let display_name = display_name.into();
        validate_display_name(&display_name)?;
        let program = program.into();
        validate_program(&program)?;
        let arguments = arguments.into_iter().map(Into::into).collect::<Vec<_>>();
        if arguments.len() > MAX_ARGUMENTS {
            return Err(CommandDefinitionError::TooManyArguments {
                maximum: MAX_ARGUMENTS,
                actual: arguments.len(),
            });
        }
        for (index, argument) in arguments.iter().enumerate() {
            validate_argument(index, argument)?;
        }
        validate_timeout(timeout)?;

        Ok(Self {
            command_id,
            repository_id,
            display_name,
            program,
            arguments,
            working_directory,
            environment,
            timeout,
            cancellation,
            authority,
        })
    }

    pub const fn command_id(&self) -> CommandId {
        self.command_id
    }

    pub const fn repository_id(&self) -> RepositoryId {
        self.repository_id
    }

    pub fn display_name(&self) -> &str {
        &self.display_name
    }

    pub fn program(&self) -> &str {
        &self.program
    }

    pub fn arguments(&self) -> &[String] {
        &self.arguments
    }

    pub fn working_directory(&self) -> &CommandWorkingDirectory {
        &self.working_directory
    }

    pub fn environment(&self) -> &CommandEnvironmentPolicy {
        &self.environment
    }

    pub const fn timeout(&self) -> CommandTimeout {
        self.timeout
    }

    pub const fn cancellation(&self) -> CommandCancellationPolicy {
        self.cancellation
    }

    pub const fn authority(&self) -> CommandAuthorityClass {
        self.authority
    }

    /// Exact versioned bytes that define this command's meaning.
    pub fn canonical_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(COMMAND_MAGIC);
        bytes.push(COMMAND_SCHEMA_VERSION);
        bytes.extend_from_slice(self.command_id.as_bytes());
        bytes.extend_from_slice(self.repository_id.as_bytes());
        bytes.push(self.authority.code());
        bytes.push(self.cancellation.code());
        match self.timeout {
            CommandTimeout::Unlimited => bytes.push(0),
            CommandTimeout::Milliseconds(millis) => {
                bytes.push(1);
                bytes.extend_from_slice(&millis.to_be_bytes());
            }
        }
        put_text(&mut bytes, &self.display_name);
        put_text(&mut bytes, &self.program);
        bytes.extend_from_slice(&(self.arguments.len() as u16).to_be_bytes());
        for argument in &self.arguments {
            put_text(&mut bytes, argument);
        }
        match self.working_directory.relative_path() {
            None => bytes.push(0),
            Some(relative) => {
                bytes.push(1);
                put_text(
                    &mut bytes,
                    relative
                        .as_path()
                        .to_str()
                        .expect("command working directories are validated UTF-8"),
                );
            }
        }
        bytes.extend_from_slice(&(self.environment.variables.len() as u16).to_be_bytes());
        for variable in &self.environment.variables {
            put_text(&mut bytes, &variable.name);
            match &variable.source {
                CommandEnvironmentSource::Literal(value) => {
                    bytes.push(1);
                    put_text(&mut bytes, value);
                }
                CommandEnvironmentSource::InheritDeclared => bytes.push(2),
            }
        }
        bytes
    }

    /// SHA-256 identity of the exact V1 command definition bytes.
    pub fn definition_identity(&self) -> ContentHash {
        hash_canonical_bytes(HashDomain::ToolRequest, &self.canonical_bytes())
    }
}

/// Deterministically ordered registry that forbids silent ID meaning changes.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CommandRegistry {
    commands: BTreeMap<CommandId, RegisteredCommand>,
}

impl CommandRegistry {
    pub fn new() -> Self {
        Self {
            commands: BTreeMap::new(),
        }
    }

    pub fn register(
        &mut self,
        command: RegisteredCommand,
    ) -> Result<CommandRegistration, CommandRegistryError> {
        let command_id = command.command_id();
        if let Some(existing) = self.commands.get(&command_id) {
            if existing == &command {
                return Ok(CommandRegistration::AlreadyRegistered);
            }
            return Err(CommandRegistryError::IdentityConflict {
                command_id,
                existing: existing.definition_identity(),
                proposed: command.definition_identity(),
            });
        }
        self.commands.insert(command_id, command);
        Ok(CommandRegistration::Inserted)
    }

    /// Explicitly replaces one definition only when the caller names the exact old identity.
    pub fn replace(
        &mut self,
        expected: ContentHash,
        command: RegisteredCommand,
    ) -> Result<ContentHash, CommandRegistryError> {
        let command_id = command.command_id();
        let existing = self
            .commands
            .get(&command_id)
            .ok_or(CommandRegistryError::UnknownCommand(command_id))?;
        let actual = existing.definition_identity();
        if actual != expected {
            return Err(CommandRegistryError::StaleDefinition {
                command_id,
                expected,
                actual,
            });
        }
        let identity = command.definition_identity();
        self.commands.insert(command_id, command);
        Ok(identity)
    }

    pub fn get(&self, command_id: CommandId) -> Option<&RegisteredCommand> {
        self.commands.get(&command_id)
    }

    pub fn iter(&self) -> impl ExactSizeIterator<Item = &RegisteredCommand> {
        self.commands.values()
    }

    pub fn len(&self) -> usize {
        self.commands.len()
    }

    pub fn is_empty(&self) -> bool {
        self.commands.is_empty()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandRegistration {
    Inserted,
    AlreadyRegistered,
}

/// Exact structural reason a command definition was rejected.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CommandDefinitionError {
    EmptyDisplayName,
    DisplayNameNotTrimmed,
    DisplayNameTooLong { maximum: usize, actual: usize },
    EmptyProgram,
    ProgramTooLong { maximum: usize, actual: usize },
    ContainsNul { field: CommandTextField, index: Option<usize>, byte_index: usize },
    TooManyArguments { maximum: usize, actual: usize },
    EmptyArgument { index: usize },
    ArgumentTooLong { index: usize, maximum: usize, actual: usize },
    WorkingDirectoryNotUtf8,
    InvalidEnvironmentName { name: String, byte_index: usize, byte: u8 },
    EnvironmentNameTooLong { maximum: usize, actual: usize },
    EnvironmentValueTooLong { name: String, maximum: usize, actual: usize },
    DuplicateEnvironmentVariable(String),
    TooManyEnvironmentVariables { maximum: usize, actual: usize },
    ZeroTimeout,
    TimeoutTooLarge { maximum_millis: u64, actual_millis: u64 },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandTextField {
    Program,
    Argument,
    EnvironmentValue,
}

/// Exact reason a registry operation was rejected.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CommandRegistryError {
    IdentityConflict {
        command_id: CommandId,
        existing: ContentHash,
        proposed: ContentHash,
    },
    UnknownCommand(CommandId),
    StaleDefinition {
        command_id: CommandId,
        expected: ContentHash,
        actual: ContentHash,
    },
}

impl fmt::Display for CommandDefinitionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "invalid registered command definition: {self:?}")
    }
}

impl std::error::Error for CommandDefinitionError {}

impl fmt::Display for CommandRegistryError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "registered command registry rejected operation: {self:?}")
    }
}

impl std::error::Error for CommandRegistryError {}

fn validate_display_name(name: &str) -> Result<(), CommandDefinitionError> {
    if name.is_empty() {
        return Err(CommandDefinitionError::EmptyDisplayName);
    }
    if name.trim() != name {
        return Err(CommandDefinitionError::DisplayNameNotTrimmed);
    }
    if name.len() > MAX_DISPLAY_NAME_BYTES {
        return Err(CommandDefinitionError::DisplayNameTooLong {
            maximum: MAX_DISPLAY_NAME_BYTES,
            actual: name.len(),
        });
    }
    Ok(())
}

fn validate_program(program: &str) -> Result<(), CommandDefinitionError> {
    if program.is_empty() {
        return Err(CommandDefinitionError::EmptyProgram);
    }
    if program.len() > MAX_PROGRAM_BYTES {
        return Err(CommandDefinitionError::ProgramTooLong {
            maximum: MAX_PROGRAM_BYTES,
            actual: program.len(),
        });
    }
    reject_nul(CommandTextField::Program, None, program)
}

fn validate_argument(index: usize, argument: &str) -> Result<(), CommandDefinitionError> {
    if argument.is_empty() {
        return Err(CommandDefinitionError::EmptyArgument { index });
    }
    if argument.len() > MAX_ARGUMENT_BYTES {
        return Err(CommandDefinitionError::ArgumentTooLong {
            index,
            maximum: MAX_ARGUMENT_BYTES,
            actual: argument.len(),
        });
    }
    reject_nul(CommandTextField::Argument, Some(index), argument)
}

fn validate_timeout(timeout: CommandTimeout) -> Result<(), CommandDefinitionError> {
    match timeout {
        CommandTimeout::Unlimited => Ok(()),
        CommandTimeout::Milliseconds(0) => Err(CommandDefinitionError::ZeroTimeout),
        CommandTimeout::Milliseconds(actual_millis) if actual_millis > MAX_TIMEOUT_MILLIS => {
            Err(CommandDefinitionError::TimeoutTooLarge {
                maximum_millis: MAX_TIMEOUT_MILLIS,
                actual_millis,
            })
        }
        CommandTimeout::Milliseconds(_) => Ok(()),
    }
}

fn validate_environment_name(name: &str) -> Result<(), CommandDefinitionError> {
    if name.len() > MAX_ENVIRONMENT_NAME_BYTES {
        return Err(CommandDefinitionError::EnvironmentNameTooLong {
            maximum: MAX_ENVIRONMENT_NAME_BYTES,
            actual: name.len(),
        });
    }
    for (index, byte) in name.bytes().enumerate() {
        let valid = if index == 0 {
            byte.is_ascii_alphabetic() || byte == b'_'
        } else {
            byte.is_ascii_alphanumeric() || byte == b'_'
        };
        if !valid {
            return Err(CommandDefinitionError::InvalidEnvironmentName {
                name: name.to_owned(),
                byte_index: index,
                byte,
            });
        }
    }
    if name.is_empty() {
        return Err(CommandDefinitionError::InvalidEnvironmentName {
            name: String::new(),
            byte_index: 0,
            byte: 0,
        });
    }
    Ok(())
}

fn validate_environment_value(name: &str, value: &str) -> Result<(), CommandDefinitionError> {
    if value.len() > MAX_ENVIRONMENT_VALUE_BYTES {
        return Err(CommandDefinitionError::EnvironmentValueTooLong {
            name: name.to_owned(),
            maximum: MAX_ENVIRONMENT_VALUE_BYTES,
            actual: value.len(),
        });
    }
    reject_nul(CommandTextField::EnvironmentValue, None, value)
}

fn reject_nul(
    field: CommandTextField,
    index: Option<usize>,
    value: &str,
) -> Result<(), CommandDefinitionError> {
    if let Some(byte_index) = value.bytes().position(|byte| byte == 0) {
        Err(CommandDefinitionError::ContainsNul {
            field,
            index,
            byte_index,
        })
    } else {
        Ok(())
    }
}

fn put_text(bytes: &mut Vec<u8>, text: &str) {
    let length = u32::try_from(text.len()).expect("validated command fields fit u32");
    bytes.extend_from_slice(&length.to_be_bytes());
    bytes.extend_from_slice(text.as_bytes());
}
