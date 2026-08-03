//! Decoder for exact V1 registered-command definition bytes.
//!
//! `commands` owns construction and canonical encoding. This module restores the
//! same typed definition without widening the command surface or accepting shell
//! prose.

use crate::commands::{
    CommandAuthorityClass, CommandCancellationPolicy, CommandDefinitionError,
    CommandEnvironmentPolicy, CommandEnvironmentVariable, CommandTimeout, CommandWorkingDirectory,
    RegisteredCommand,
};
use forge_protocol::identities::{CommandId, RepositoryId, IDENTITY_BYTES};
use forge_protocol::paths::{RepositoryPathError, RepositoryRelativePath};
use std::fmt;
use std::path::Path;
use std::time::Duration;

const COMMAND_MAGIC: &[u8; 12] = b"FORGECMD\0\0\0\0";
const COMMAND_SCHEMA_VERSION: u8 = 1;
const MAX_TEXT_BYTES: usize = 64 * 1024;
const MAX_ARGUMENTS: usize = 256;
const MAX_ENVIRONMENT_VARIABLES: usize = 256;

/// Restores one exact canonical registered-command definition.
pub fn decode_registered_command(bytes: &[u8]) -> Result<RegisteredCommand, CommandDecodeError> {
    let mut cursor = Cursor::new(bytes);
    if cursor.take(COMMAND_MAGIC.len())? != COMMAND_MAGIC.as_slice() {
        return Err(CommandDecodeError::InvalidMagic);
    }
    let schema = cursor.u8()?;
    if schema != COMMAND_SCHEMA_VERSION {
        return Err(CommandDecodeError::UnsupportedSchemaVersion(schema));
    }
    let command_id = CommandId::from_bytes(cursor.array::<IDENTITY_BYTES>()?);
    let repository_id = RepositoryId::from_bytes(cursor.array::<IDENTITY_BYTES>()?);
    let authority = match cursor.u8()? {
        1 => CommandAuthorityClass::Inspect,
        2 => CommandAuthorityClass::Build,
        3 => CommandAuthorityClass::WorkspaceWrite,
        4 => CommandAuthorityClass::RepositoryMutation,
        5 => CommandAuthorityClass::HostMutation,
        found => return Err(CommandDecodeError::InvalidAuthority(found)),
    };
    let cancellation = match cursor.u8()? {
        1 => CommandCancellationPolicy::TerminateProcessGroup,
        found => return Err(CommandDecodeError::InvalidCancellationPolicy(found)),
    };
    let timeout = match cursor.u8()? {
        0 => CommandTimeout::Unlimited,
        1 => {
            let millis = cursor.u64()?;
            CommandTimeout::after(Duration::from_millis(millis))?
        }
        found => return Err(CommandDecodeError::InvalidTimeoutKind(found)),
    };
    let display_name = cursor.text()?;
    let program = cursor.text()?;
    let argument_count = cursor.u16()? as usize;
    if argument_count > MAX_ARGUMENTS {
        return Err(CommandDecodeError::TooManyArguments {
            maximum: MAX_ARGUMENTS,
            actual: argument_count,
        });
    }
    let mut arguments = Vec::with_capacity(argument_count);
    for _ in 0..argument_count {
        arguments.push(cursor.text()?);
    }
    let working_directory = match cursor.u8()? {
        0 => CommandWorkingDirectory::repository_root(),
        1 => {
            let path = cursor.text()?;
            CommandWorkingDirectory::relative(RepositoryRelativePath::new(Path::new(&path))?)?
        }
        found => return Err(CommandDecodeError::InvalidWorkingDirectoryKind(found)),
    };
    let environment_count = cursor.u16()? as usize;
    if environment_count > MAX_ENVIRONMENT_VARIABLES {
        return Err(CommandDecodeError::TooManyEnvironmentVariables {
            maximum: MAX_ENVIRONMENT_VARIABLES,
            actual: environment_count,
        });
    }
    let mut environment = Vec::with_capacity(environment_count);
    for _ in 0..environment_count {
        let name = cursor.text()?;
        let variable = match cursor.u8()? {
            1 => CommandEnvironmentVariable::literal(name, cursor.text()?)?,
            2 => CommandEnvironmentVariable::inherit(name)?,
            found => return Err(CommandDecodeError::InvalidEnvironmentSource(found)),
        };
        environment.push(variable);
    }
    if !cursor.is_finished() {
        return Err(CommandDecodeError::TrailingBytes(cursor.remaining()));
    }

    let command = RegisteredCommand::new(
        command_id,
        repository_id,
        display_name,
        program,
        arguments,
        working_directory,
        CommandEnvironmentPolicy::clear(environment)?,
        timeout,
        cancellation,
        authority,
    )?;
    if command.canonical_bytes() != bytes {
        return Err(CommandDecodeError::NonCanonicalEncoding);
    }
    Ok(command)
}

struct Cursor<'a> {
    bytes: &'a [u8],
    offset: usize,
}

impl<'a> Cursor<'a> {
    const fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, offset: 0 }
    }

    fn take(&mut self, count: usize) -> Result<&'a [u8], CommandDecodeError> {
        let end = self
            .offset
            .checked_add(count)
            .ok_or(CommandDecodeError::LengthOverflow)?;
        if end > self.bytes.len() {
            return Err(CommandDecodeError::Truncated {
                needed: count,
                remaining: self.remaining(),
            });
        }
        let value = &self.bytes[self.offset..end];
        self.offset = end;
        Ok(value)
    }

    fn array<const N: usize>(&mut self) -> Result<[u8; N], CommandDecodeError> {
        self.take(N)?
            .try_into()
            .map_err(|_| CommandDecodeError::LengthOverflow)
    }

    fn u8(&mut self) -> Result<u8, CommandDecodeError> {
        Ok(self.take(1)?[0])
    }

    fn u16(&mut self) -> Result<u16, CommandDecodeError> {
        Ok(u16::from_be_bytes(self.array()?))
    }

    fn u32(&mut self) -> Result<u32, CommandDecodeError> {
        Ok(u32::from_be_bytes(self.array()?))
    }

    fn u64(&mut self) -> Result<u64, CommandDecodeError> {
        Ok(u64::from_be_bytes(self.array()?))
    }

    fn text(&mut self) -> Result<String, CommandDecodeError> {
        let length = self.u32()? as usize;
        if length > MAX_TEXT_BYTES {
            return Err(CommandDecodeError::TextTooLong {
                maximum: MAX_TEXT_BYTES,
                actual: length,
            });
        }
        let bytes = self.take(length)?;
        std::str::from_utf8(bytes)
            .map(str::to_owned)
            .map_err(|_| CommandDecodeError::InvalidUtf8)
    }

    const fn remaining(&self) -> usize {
        self.bytes.len() - self.offset
    }

    const fn is_finished(&self) -> bool {
        self.offset == self.bytes.len()
    }
}

/// Exact reason canonical command restoration failed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CommandDecodeError {
    InvalidMagic,
    UnsupportedSchemaVersion(u8),
    Truncated { needed: usize, remaining: usize },
    TrailingBytes(usize),
    LengthOverflow,
    InvalidUtf8,
    TextTooLong { maximum: usize, actual: usize },
    InvalidAuthority(u8),
    InvalidCancellationPolicy(u8),
    InvalidTimeoutKind(u8),
    InvalidWorkingDirectoryKind(u8),
    InvalidEnvironmentSource(u8),
    TooManyArguments { maximum: usize, actual: usize },
    TooManyEnvironmentVariables { maximum: usize, actual: usize },
    NonCanonicalEncoding,
    Path(RepositoryPathError),
    Definition(CommandDefinitionError),
}

impl fmt::Display for CommandDecodeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidMagic => formatter.write_str("invalid registered-command magic"),
            Self::UnsupportedSchemaVersion(found) => {
                write!(formatter, "unsupported registered-command schema {found}")
            }
            Self::Truncated { needed, remaining } => {
                write!(
                    formatter,
                    "registered command needs {needed} bytes, only {remaining} remain"
                )
            }
            Self::TrailingBytes(count) => {
                write!(formatter, "registered command has {count} trailing bytes")
            }
            Self::LengthOverflow => formatter.write_str("registered-command length overflow"),
            Self::InvalidUtf8 => formatter.write_str("registered-command text is not UTF-8"),
            Self::TextTooLong { maximum, actual } => {
                write!(
                    formatter,
                    "registered-command text has {actual} bytes, maximum is {maximum}"
                )
            }
            Self::InvalidAuthority(found) => {
                write!(formatter, "invalid command authority code {found}")
            }
            Self::InvalidCancellationPolicy(found) => {
                write!(formatter, "invalid cancellation policy code {found}")
            }
            Self::InvalidTimeoutKind(found) => {
                write!(formatter, "invalid command timeout kind {found}")
            }
            Self::InvalidWorkingDirectoryKind(found) => {
                write!(formatter, "invalid working-directory kind {found}")
            }
            Self::InvalidEnvironmentSource(found) => {
                write!(formatter, "invalid environment-source code {found}")
            }
            Self::TooManyArguments { maximum, actual } => write!(
                formatter,
                "command has {actual} arguments, maximum is {maximum}"
            ),
            Self::TooManyEnvironmentVariables { maximum, actual } => write!(
                formatter,
                "command has {actual} environment variables, maximum is {maximum}"
            ),
            Self::NonCanonicalEncoding => {
                formatter.write_str("registered-command bytes are not canonical")
            }
            Self::Path(source) => write!(formatter, "registered-command path rejected: {source}"),
            Self::Definition(source) => write!(
                formatter,
                "registered-command definition rejected: {source}"
            ),
        }
    }
}

impl std::error::Error for CommandDecodeError {}

impl From<RepositoryPathError> for CommandDecodeError {
    fn from(source: RepositoryPathError) -> Self {
        Self::Path(source)
    }
}

impl From<CommandDefinitionError> for CommandDecodeError {
    fn from(source: CommandDefinitionError) -> Self {
        Self::Definition(source)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::CommandEnvironmentVariable;

    #[test]
    fn canonical_registered_command_round_trips() {
        let command = RegisteredCommand::new(
            CommandId::from_bytes([1; IDENTITY_BYTES]),
            RepositoryId::from_bytes([2; IDENTITY_BYTES]),
            "Check",
            "cargo",
            ["check"],
            CommandWorkingDirectory::repository_root(),
            CommandEnvironmentPolicy::clear(vec![
                CommandEnvironmentVariable::inherit("PATH").unwrap()
            ])
            .unwrap(),
            CommandTimeout::after(Duration::from_secs(30)).unwrap(),
            CommandCancellationPolicy::TerminateProcessGroup,
            CommandAuthorityClass::Build,
        )
        .unwrap();
        let bytes = command.canonical_bytes();
        assert_eq!(decode_registered_command(&bytes), Ok(command));
    }
}
