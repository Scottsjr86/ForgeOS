//! Canonical V1 project manifest state.
//!
//! Forge Core owns the versioned manifest bytes and validation rules. Filesystem
//! discovery and repository-boundary effects remain in `forge-project`.

use crate::state::{StateRecord, StateRecordError};
use forge_protocol::identities::{CommandId, ProjectId, RepositoryId, IDENTITY_BYTES};
use forge_protocol::paths::{RepositoryPathError, RepositoryRelativePath};
use std::fmt;
use std::str;

const MANIFEST_MAGIC: [u8; 8] = *b"FGPROJ\0\0";
const REQUIRED_FIELD: u16 = 0x8000;
const FIELD_MASK: u16 = 0x7fff;
const FIELD_PROJECT_ID: u16 = 1;
const FIELD_REPOSITORY_ID: u16 = 2;
const FIELD_DISPLAY_NAME: u16 = 3;
const FIELD_ALLOWED_ROOTS: u16 = 4;
const FIELD_COMMANDS: u16 = 5;
const FIELD_LANGUAGE_PROFILE: u16 = 6;
const FIELD_SETTINGS: u16 = 7;
const MAX_DISPLAY_NAME_BYTES: usize = 128;
const MAX_ROOTS: usize = 64;
const MAX_COMMANDS: usize = 128;
const MAX_SETTINGS: usize = 128;
const MAX_COMMAND_NAME_BYTES: usize = 128;
const MAX_SETTING_KEY_BYTES: usize = 64;
const MAX_SETTING_VALUE_BYTES: usize = 4096;

/// Current project-manifest schema written by V1.
pub const PROJECT_MANIFEST_SCHEMA_VERSION: u16 = 1;

/// State-record type reserved for canonical project manifests.
pub const PROJECT_MANIFEST_RECORD_TYPE: u16 = 0x0101;

/// The only language profile supported by First Armor.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum LanguageProfile {
    Rust,
}

impl LanguageProfile {
    const fn code(self) -> u8 {
        match self {
            Self::Rust => 1,
        }
    }

    fn from_code(code: u8) -> Result<Self, ProjectManifestError> {
        match code {
            1 => Ok(Self::Rust),
            found => Err(ProjectManifestError::UnsupportedLanguageProfile { found }),
        }
    }
}

/// One explicitly allowed repository scope. The repository root is represented
/// without inventing a relative path alias such as `.`.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum AllowedProjectRoot {
    RepositoryRoot,
    Relative(RepositoryRelativePath),
}

impl AllowedProjectRoot {
    pub const fn repository_root() -> Self {
        Self::RepositoryRoot
    }

    pub fn relative(path: impl AsRef<std::path::Path>) -> Result<Self, RepositoryPathError> {
        RepositoryRelativePath::new(path).map(Self::Relative)
    }

    pub fn relative_path(&self) -> Option<&RepositoryRelativePath> {
        match self {
            Self::RepositoryRoot => None,
            Self::Relative(path) => Some(path),
        }
    }

    fn canonical_label(&self) -> &str {
        match self {
            Self::RepositoryRoot => "<repository-root>",
            Self::Relative(path) => path
                .as_path()
                .to_str()
                .expect("manifest relative roots are validated UTF-8"),
        }
    }
}

/// One declared command reference. Execution semantics belong to COMMAND-100.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ManifestCommand {
    command_id: CommandId,
    display_name: String,
}

impl ManifestCommand {
    pub fn new(
        command_id: CommandId,
        display_name: impl Into<String>,
    ) -> Result<Self, ProjectManifestError> {
        let display_name = display_name.into();
        validate_human_name(
            ManifestNameKind::Command,
            &display_name,
            MAX_COMMAND_NAME_BYTES,
        )?;
        Ok(Self {
            command_id,
            display_name,
        })
    }

    pub const fn command_id(&self) -> CommandId {
        self.command_id
    }

    pub fn display_name(&self) -> &str {
        &self.display_name
    }
}

/// One deterministic project setting.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectSetting {
    key: String,
    value: String,
}

impl ProjectSetting {
    pub fn new(
        key: impl Into<String>,
        value: impl Into<String>,
    ) -> Result<Self, ProjectManifestError> {
        let key = key.into();
        let value = value.into();
        validate_setting_key(&key)?;
        if value.len() > MAX_SETTING_VALUE_BYTES {
            return Err(ProjectManifestError::SettingValueTooLong {
                maximum: MAX_SETTING_VALUE_BYTES,
                actual: value.len(),
            });
        }
        if value.as_bytes().contains(&0) {
            return Err(ProjectManifestError::SettingValueContainsNul);
        }
        Ok(Self { key, value })
    }

    pub fn key(&self) -> &str {
        &self.key
    }

    pub fn value(&self) -> &str {
        &self.value
    }
}

/// Canonical, versioned V1 project manifest.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectManifest {
    project_id: ProjectId,
    repository_id: RepositoryId,
    display_name: String,
    allowed_roots: Vec<AllowedProjectRoot>,
    commands: Vec<ManifestCommand>,
    language_profile: LanguageProfile,
    settings: Vec<ProjectSetting>,
}

impl ProjectManifest {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        project_id: ProjectId,
        repository_id: RepositoryId,
        display_name: impl Into<String>,
        mut allowed_roots: Vec<AllowedProjectRoot>,
        mut commands: Vec<ManifestCommand>,
        language_profile: LanguageProfile,
        mut settings: Vec<ProjectSetting>,
    ) -> Result<Self, ProjectManifestError> {
        let display_name = display_name.into();
        validate_human_name(
            ManifestNameKind::Project,
            &display_name,
            MAX_DISPLAY_NAME_BYTES,
        )?;

        if allowed_roots.is_empty() {
            return Err(ProjectManifestError::MissingAllowedRoot);
        }
        if allowed_roots.len() > MAX_ROOTS {
            return Err(ProjectManifestError::TooManyAllowedRoots {
                maximum: MAX_ROOTS,
                actual: allowed_roots.len(),
            });
        }
        for (index, root) in allowed_roots.iter().enumerate() {
            if let AllowedProjectRoot::Relative(path) = root {
                if path.as_path().to_str().is_none() {
                    return Err(ProjectManifestError::NonUtf8AllowedRoot { index });
                }
            }
        }
        allowed_roots.sort_by(|left, right| {
            left.canonical_label()
                .as_bytes()
                .cmp(right.canonical_label().as_bytes())
        });
        for pair in allowed_roots.windows(2) {
            if pair[0] == pair[1] {
                return Err(ProjectManifestError::DuplicateAllowedRoot(
                    pair[0].canonical_label().to_owned(),
                ));
            }
        }

        if commands.len() > MAX_COMMANDS {
            return Err(ProjectManifestError::TooManyCommands {
                maximum: MAX_COMMANDS,
                actual: commands.len(),
            });
        }
        commands.sort_by(|left, right| {
            left.command_id
                .as_bytes()
                .cmp(right.command_id.as_bytes())
                .then_with(|| left.display_name.as_bytes().cmp(right.display_name.as_bytes()))
        });
        for pair in commands.windows(2) {
            if pair[0].command_id == pair[1].command_id {
                return Err(ProjectManifestError::DuplicateCommandId(
                    pair[0].command_id,
                ));
            }
        }
        let mut command_names: Vec<&str> = commands
            .iter()
            .map(|command| command.display_name.as_str())
            .collect();
        command_names.sort_unstable();
        for pair in command_names.windows(2) {
            if pair[0] == pair[1] {
                return Err(ProjectManifestError::DuplicateCommandName(
                    pair[0].to_owned(),
                ));
            }
        }

        if settings.len() > MAX_SETTINGS {
            return Err(ProjectManifestError::TooManySettings {
                maximum: MAX_SETTINGS,
                actual: settings.len(),
            });
        }
        settings.sort_by(|left, right| left.key.as_bytes().cmp(right.key.as_bytes()));
        for pair in settings.windows(2) {
            if pair[0].key == pair[1].key {
                return Err(ProjectManifestError::DuplicateSettingKey(
                    pair[0].key.clone(),
                ));
            }
        }

        Ok(Self {
            project_id,
            repository_id,
            display_name,
            allowed_roots,
            commands,
            language_profile,
            settings,
        })
    }

    pub const fn project_id(&self) -> ProjectId {
        self.project_id
    }

    pub const fn repository_id(&self) -> RepositoryId {
        self.repository_id
    }

    pub fn display_name(&self) -> &str {
        &self.display_name
    }

    pub fn allowed_roots(&self) -> &[AllowedProjectRoot] {
        &self.allowed_roots
    }

    pub fn commands(&self) -> &[ManifestCommand] {
        &self.commands
    }

    pub const fn language_profile(&self) -> LanguageProfile {
        self.language_profile
    }

    pub fn settings(&self) -> &[ProjectSetting] {
        &self.settings
    }

    /// Exact deterministic manifest bytes.
    pub fn encode(&self) -> Vec<u8> {
        let fields = [
            encode_field(FIELD_PROJECT_ID, self.project_id.as_bytes()),
            encode_field(FIELD_REPOSITORY_ID, self.repository_id.as_bytes()),
            encode_field(FIELD_DISPLAY_NAME, self.display_name.as_bytes()),
            encode_field(FIELD_ALLOWED_ROOTS, &encode_roots(&self.allowed_roots)),
            encode_field(FIELD_COMMANDS, &encode_commands(&self.commands)),
            encode_field(
                FIELD_LANGUAGE_PROFILE,
                &[self.language_profile.code()],
            ),
            encode_field(FIELD_SETTINGS, &encode_settings(&self.settings)),
        ];

        let payload_len = fields.iter().map(Vec::len).sum::<usize>();
        let mut bytes = Vec::with_capacity(MANIFEST_MAGIC.len() + 4 + payload_len);
        bytes.extend_from_slice(&MANIFEST_MAGIC);
        bytes.extend_from_slice(&PROJECT_MANIFEST_SCHEMA_VERSION.to_be_bytes());
        bytes.extend_from_slice(&(fields.len() as u16).to_be_bytes());
        for field in fields {
            bytes.extend_from_slice(&field);
        }
        bytes
    }

    /// Decodes current V1 bytes. Unknown optional fields are ignored; unknown
    /// required fields are rejected.
    pub fn decode(bytes: &[u8]) -> Result<Self, ProjectManifestError> {
        let mut reader = Reader::new(bytes);
        if reader.take(MANIFEST_MAGIC.len())? != MANIFEST_MAGIC.as_slice() {
            return Err(ProjectManifestError::InvalidMagic);
        }
        let schema_version = reader.u16()?;
        if schema_version != PROJECT_MANIFEST_SCHEMA_VERSION {
            return Err(ProjectManifestError::UnsupportedSchemaVersion {
                found: schema_version,
            });
        }
        let field_count = usize::from(reader.u16()?);

        let mut project_id = None;
        let mut repository_id = None;
        let mut display_name = None;
        let mut allowed_roots = None;
        let mut commands = None;
        let mut language_profile = None;
        let mut settings = None;

        for _ in 0..field_count {
            let encoded_tag = reader.u16()?;
            let required = encoded_tag & REQUIRED_FIELD != 0;
            let tag = encoded_tag & FIELD_MASK;
            let length = reader.u32()? as usize;
            let payload = reader.take(length)?;

            match tag {
                FIELD_PROJECT_ID => set_once(
                    &mut project_id,
                    ProjectId::from_bytes(read_identity(payload, tag)?),
                    tag,
                )?,
                FIELD_REPOSITORY_ID => set_once(
                    &mut repository_id,
                    RepositoryId::from_bytes(read_identity(payload, tag)?),
                    tag,
                )?,
                FIELD_DISPLAY_NAME => set_once(
                    &mut display_name,
                    read_utf8(payload, ManifestTextField::DisplayName)?.to_owned(),
                    tag,
                )?,
                FIELD_ALLOWED_ROOTS => {
                    set_once(&mut allowed_roots, decode_roots(payload)?, tag)?
                }
                FIELD_COMMANDS => set_once(&mut commands, decode_commands(payload)?, tag)?,
                FIELD_LANGUAGE_PROFILE => {
                    if payload.len() != 1 {
                        return Err(ProjectManifestError::InvalidFieldLength {
                            field: tag,
                            expected: 1,
                            actual: payload.len(),
                        });
                    }
                    set_once(
                        &mut language_profile,
                        LanguageProfile::from_code(payload[0])?,
                        tag,
                    )?;
                }
                FIELD_SETTINGS => set_once(&mut settings, decode_settings(payload)?, tag)?,
                _ if required => {
                    return Err(ProjectManifestError::UnknownRequiredField { field: tag })
                }
                _ => {}
            }
        }

        if !reader.is_empty() {
            return Err(ProjectManifestError::TrailingBytes {
                actual: reader.remaining(),
            });
        }

        Self::new(
            project_id.ok_or(ProjectManifestError::MissingRequiredField {
                field: FIELD_PROJECT_ID,
            })?,
            repository_id.ok_or(ProjectManifestError::MissingRequiredField {
                field: FIELD_REPOSITORY_ID,
            })?,
            display_name.ok_or(ProjectManifestError::MissingRequiredField {
                field: FIELD_DISPLAY_NAME,
            })?,
            allowed_roots.ok_or(ProjectManifestError::MissingRequiredField {
                field: FIELD_ALLOWED_ROOTS,
            })?,
            commands.ok_or(ProjectManifestError::MissingRequiredField {
                field: FIELD_COMMANDS,
            })?,
            language_profile.ok_or(ProjectManifestError::MissingRequiredField {
                field: FIELD_LANGUAGE_PROFILE,
            })?,
            settings.ok_or(ProjectManifestError::MissingRequiredField {
                field: FIELD_SETTINGS,
            })?,
        )
    }

    pub fn to_state_record(&self) -> Result<StateRecord, ProjectManifestError> {
        StateRecord::new(PROJECT_MANIFEST_RECORD_TYPE, self.encode())
            .map_err(ProjectManifestError::StateRecord)
    }

    pub fn from_state_record(record: &StateRecord) -> Result<Self, ProjectManifestError> {
        if record.record_type() != PROJECT_MANIFEST_RECORD_TYPE {
            return Err(ProjectManifestError::UnexpectedRecordType {
                expected: PROJECT_MANIFEST_RECORD_TYPE,
                found: record.record_type(),
            });
        }
        Self::decode(record.payload())
    }
}

fn encode_field(tag: u16, payload: &[u8]) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(2 + 4 + payload.len());
    bytes.extend_from_slice(&(REQUIRED_FIELD | tag).to_be_bytes());
    bytes.extend_from_slice(&(payload.len() as u32).to_be_bytes());
    bytes.extend_from_slice(payload);
    bytes
}

fn encode_roots(roots: &[AllowedProjectRoot]) -> Vec<u8> {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&(roots.len() as u16).to_be_bytes());
    for root in roots {
        match root {
            AllowedProjectRoot::RepositoryRoot => bytes.push(0),
            AllowedProjectRoot::Relative(path) => {
                bytes.push(1);
                let text = path
                    .as_path()
                    .to_str()
                    .expect("manifest relative roots are UTF-8");
                bytes.extend_from_slice(&(text.len() as u16).to_be_bytes());
                bytes.extend_from_slice(text.as_bytes());
            }
        }
    }
    bytes
}

fn decode_roots(payload: &[u8]) -> Result<Vec<AllowedProjectRoot>, ProjectManifestError> {
    let mut reader = Reader::new(payload);
    let count = usize::from(reader.u16()?);
    if count > MAX_ROOTS {
        return Err(ProjectManifestError::TooManyAllowedRoots {
            maximum: MAX_ROOTS,
            actual: count,
        });
    }
    let mut roots = Vec::with_capacity(count);
    for index in 0..count {
        let kind = reader.u8()?;
        let root = match kind {
            0 => AllowedProjectRoot::RepositoryRoot,
            1 => {
                let length = usize::from(reader.u16()?);
                let text = read_utf8(reader.take(length)?, ManifestTextField::AllowedRoot)?;
                AllowedProjectRoot::relative(text).map_err(|source| {
                    ProjectManifestError::InvalidAllowedRoot { index, source }
                })?
            }
            found => {
                return Err(ProjectManifestError::UnsupportedAllowedRootKind { index, found })
            }
        };
        roots.push(root);
    }
    reader.finish_nested(ManifestTextField::AllowedRoot)?;
    Ok(roots)
}

fn encode_commands(commands: &[ManifestCommand]) -> Vec<u8> {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&(commands.len() as u16).to_be_bytes());
    for command in commands {
        bytes.extend_from_slice(command.command_id.as_bytes());
        bytes.extend_from_slice(&(command.display_name.len() as u16).to_be_bytes());
        bytes.extend_from_slice(command.display_name.as_bytes());
    }
    bytes
}

fn decode_commands(payload: &[u8]) -> Result<Vec<ManifestCommand>, ProjectManifestError> {
    let mut reader = Reader::new(payload);
    let count = usize::from(reader.u16()?);
    if count > MAX_COMMANDS {
        return Err(ProjectManifestError::TooManyCommands {
            maximum: MAX_COMMANDS,
            actual: count,
        });
    }
    let mut commands = Vec::with_capacity(count);
    for _ in 0..count {
        let command_id = CommandId::from_bytes(read_identity(
            reader.take(IDENTITY_BYTES)?,
            FIELD_COMMANDS,
        )?);
        let length = usize::from(reader.u16()?);
        let display_name =
            read_utf8(reader.take(length)?, ManifestTextField::CommandName)?.to_owned();
        commands.push(ManifestCommand::new(command_id, display_name)?);
    }
    reader.finish_nested(ManifestTextField::CommandName)?;
    Ok(commands)
}

fn encode_settings(settings: &[ProjectSetting]) -> Vec<u8> {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&(settings.len() as u16).to_be_bytes());
    for setting in settings {
        bytes.extend_from_slice(&(setting.key.len() as u16).to_be_bytes());
        bytes.extend_from_slice(setting.key.as_bytes());
        bytes.extend_from_slice(&(setting.value.len() as u32).to_be_bytes());
        bytes.extend_from_slice(setting.value.as_bytes());
    }
    bytes
}

fn decode_settings(payload: &[u8]) -> Result<Vec<ProjectSetting>, ProjectManifestError> {
    let mut reader = Reader::new(payload);
    let count = usize::from(reader.u16()?);
    if count > MAX_SETTINGS {
        return Err(ProjectManifestError::TooManySettings {
            maximum: MAX_SETTINGS,
            actual: count,
        });
    }
    let mut settings = Vec::with_capacity(count);
    for _ in 0..count {
        let key_len = usize::from(reader.u16()?);
        let key = read_utf8(reader.take(key_len)?, ManifestTextField::SettingKey)?;
        let value_len = reader.u32()? as usize;
        let value = read_utf8(
            reader.take(value_len)?,
            ManifestTextField::SettingValue,
        )?;
        settings.push(ProjectSetting::new(key, value)?);
    }
    reader.finish_nested(ManifestTextField::SettingValue)?;
    Ok(settings)
}

fn read_identity(payload: &[u8], field: u16) -> Result<[u8; IDENTITY_BYTES], ProjectManifestError> {
    if payload.len() != IDENTITY_BYTES {
        return Err(ProjectManifestError::InvalidFieldLength {
            field,
            expected: IDENTITY_BYTES,
            actual: payload.len(),
        });
    }
    let mut bytes = [0u8; IDENTITY_BYTES];
    bytes.copy_from_slice(payload);
    Ok(bytes)
}

fn set_once<T>(slot: &mut Option<T>, value: T, field: u16) -> Result<(), ProjectManifestError> {
    if slot.replace(value).is_some() {
        Err(ProjectManifestError::DuplicateField { field })
    } else {
        Ok(())
    }
}

fn validate_human_name(
    kind: ManifestNameKind,
    value: &str,
    maximum: usize,
) -> Result<(), ProjectManifestError> {
    if value.is_empty() || value.trim() != value {
        return Err(ProjectManifestError::InvalidName { kind });
    }
    if value.len() > maximum {
        return Err(ProjectManifestError::NameTooLong {
            kind,
            maximum,
            actual: value.len(),
        });
    }
    if value.chars().any(char::is_control) {
        return Err(ProjectManifestError::InvalidName { kind });
    }
    Ok(())
}

fn validate_setting_key(key: &str) -> Result<(), ProjectManifestError> {
    if key.is_empty() || key.len() > MAX_SETTING_KEY_BYTES {
        return Err(ProjectManifestError::InvalidSettingKey(key.to_owned()));
    }
    let mut bytes = key.bytes();
    let Some(first) = bytes.next() else {
        return Err(ProjectManifestError::InvalidSettingKey(key.to_owned()));
    };
    if !first.is_ascii_lowercase() {
        return Err(ProjectManifestError::InvalidSettingKey(key.to_owned()));
    }
    if !bytes.all(|byte| {
        byte.is_ascii_lowercase()
            || byte.is_ascii_digit()
            || matches!(byte, b'.' | b'_' | b'-')
    }) {
        return Err(ProjectManifestError::InvalidSettingKey(key.to_owned()));
    }
    Ok(())
}

fn read_utf8(
    payload: &[u8],
    field: ManifestTextField,
) -> Result<&str, ProjectManifestError> {
    str::from_utf8(payload).map_err(|_| ProjectManifestError::InvalidUtf8 { field })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ManifestNameKind {
    Project,
    Command,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ManifestTextField {
    DisplayName,
    AllowedRoot,
    CommandName,
    SettingKey,
    SettingValue,
}

/// Exact reason a project manifest was rejected.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProjectManifestError {
    Truncated { needed: usize, remaining: usize },
    InvalidMagic,
    UnsupportedSchemaVersion { found: u16 },
    UnknownRequiredField { field: u16 },
    MissingRequiredField { field: u16 },
    DuplicateField { field: u16 },
    InvalidFieldLength {
        field: u16,
        expected: usize,
        actual: usize,
    },
    TrailingBytes { actual: usize },
    NestedTrailingBytes {
        field: ManifestTextField,
        actual: usize,
    },
    InvalidUtf8 { field: ManifestTextField },
    InvalidName { kind: ManifestNameKind },
    NameTooLong {
        kind: ManifestNameKind,
        maximum: usize,
        actual: usize,
    },
    MissingAllowedRoot,
    TooManyAllowedRoots { maximum: usize, actual: usize },
    NonUtf8AllowedRoot { index: usize },
    InvalidAllowedRoot {
        index: usize,
        source: RepositoryPathError,
    },
    UnsupportedAllowedRootKind { index: usize, found: u8 },
    DuplicateAllowedRoot(String),
    TooManyCommands { maximum: usize, actual: usize },
    DuplicateCommandId(CommandId),
    DuplicateCommandName(String),
    UnsupportedLanguageProfile { found: u8 },
    TooManySettings { maximum: usize, actual: usize },
    InvalidSettingKey(String),
    DuplicateSettingKey(String),
    SettingValueTooLong { maximum: usize, actual: usize },
    SettingValueContainsNul,
    UnexpectedRecordType { expected: u16, found: u16 },
    StateRecord(StateRecordError),
}

impl fmt::Display for ProjectManifestError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Truncated { needed, remaining } => write!(
                formatter,
                "project manifest is truncated: need {needed} bytes, have {remaining}"
            ),
            Self::InvalidMagic => formatter.write_str("project manifest magic is invalid"),
            Self::UnsupportedSchemaVersion { found } => {
                write!(formatter, "project manifest schema {found} is unsupported")
            }
            Self::UnknownRequiredField { field } => {
                write!(formatter, "project manifest required field {field} is unknown")
            }
            Self::MissingRequiredField { field } => {
                write!(formatter, "project manifest required field {field} is missing")
            }
            Self::DuplicateField { field } => {
                write!(formatter, "project manifest field {field} is duplicated")
            }
            Self::InvalidFieldLength {
                field,
                expected,
                actual,
            } => write!(
                formatter,
                "project manifest field {field} must be {expected} bytes, got {actual}"
            ),
            Self::TrailingBytes { actual } => {
                write!(formatter, "project manifest has {actual} trailing bytes")
            }
            Self::NestedTrailingBytes { field, actual } => write!(
                formatter,
                "project manifest {field:?} payload has {actual} trailing bytes"
            ),
            Self::InvalidUtf8 { field } => {
                write!(formatter, "project manifest {field:?} is not valid UTF-8")
            }
            Self::InvalidName { kind } => {
                write!(formatter, "project manifest {kind:?} name is invalid")
            }
            Self::NameTooLong {
                kind,
                maximum,
                actual,
            } => write!(
                formatter,
                "project manifest {kind:?} name exceeds {maximum} bytes: {actual}"
            ),
            Self::MissingAllowedRoot => {
                formatter.write_str("project manifest must declare at least one allowed root")
            }
            Self::TooManyAllowedRoots { maximum, actual } => write!(
                formatter,
                "project manifest allows at most {maximum} roots, got {actual}"
            ),
            Self::NonUtf8AllowedRoot { index } => {
                write!(formatter, "project manifest allowed root {index} is not UTF-8")
            }
            Self::InvalidAllowedRoot { index, source } => {
                write!(formatter, "project manifest allowed root {index} is invalid: {source}")
            }
            Self::UnsupportedAllowedRootKind { index, found } => write!(
                formatter,
                "project manifest allowed root {index} has unsupported kind {found}"
            ),
            Self::DuplicateAllowedRoot(root) => {
                write!(formatter, "project manifest allowed root is duplicated: {root}")
            }
            Self::TooManyCommands { maximum, actual } => write!(
                formatter,
                "project manifest allows at most {maximum} commands, got {actual}"
            ),
            Self::DuplicateCommandId(command_id) => {
                write!(formatter, "project manifest command ID is duplicated: {command_id}")
            }
            Self::DuplicateCommandName(name) => {
                write!(formatter, "project manifest command name is duplicated: {name}")
            }
            Self::UnsupportedLanguageProfile { found } => {
                write!(formatter, "project manifest language profile {found} is unsupported")
            }
            Self::TooManySettings { maximum, actual } => write!(
                formatter,
                "project manifest allows at most {maximum} settings, got {actual}"
            ),
            Self::InvalidSettingKey(key) => {
                write!(formatter, "project manifest setting key is invalid: {key}")
            }
            Self::DuplicateSettingKey(key) => {
                write!(formatter, "project manifest setting key is duplicated: {key}")
            }
            Self::SettingValueTooLong { maximum, actual } => write!(
                formatter,
                "project manifest setting value exceeds {maximum} bytes: {actual}"
            ),
            Self::SettingValueContainsNul => {
                formatter.write_str("project manifest setting value contains NUL")
            }
            Self::UnexpectedRecordType { expected, found } => write!(
                formatter,
                "project manifest state record type must be {expected}, got {found}"
            ),
            Self::StateRecord(source) => write!(formatter, "project manifest state error: {source}"),
        }
    }
}

impl std::error::Error for ProjectManifestError {}

struct Reader<'a> {
    bytes: &'a [u8],
    cursor: usize,
}

impl<'a> Reader<'a> {
    const fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, cursor: 0 }
    }

    fn take(&mut self, length: usize) -> Result<&'a [u8], ProjectManifestError> {
        let end = self
            .cursor
            .checked_add(length)
            .ok_or(ProjectManifestError::Truncated {
                needed: length,
                remaining: self.remaining(),
            })?;
        if end > self.bytes.len() {
            return Err(ProjectManifestError::Truncated {
                needed: length,
                remaining: self.remaining(),
            });
        }
        let payload = &self.bytes[self.cursor..end];
        self.cursor = end;
        Ok(payload)
    }

    fn u8(&mut self) -> Result<u8, ProjectManifestError> {
        Ok(self.take(1)?[0])
    }

    fn u16(&mut self) -> Result<u16, ProjectManifestError> {
        let bytes = self.take(2)?;
        Ok(u16::from_be_bytes([bytes[0], bytes[1]]))
    }

    fn u32(&mut self) -> Result<u32, ProjectManifestError> {
        let bytes = self.take(4)?;
        Ok(u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
    }

    const fn remaining(&self) -> usize {
        self.bytes.len().saturating_sub(self.cursor)
    }

    const fn is_empty(&self) -> bool {
        self.remaining() == 0
    }

    fn finish_nested(self, field: ManifestTextField) -> Result<(), ProjectManifestError> {
        if self.is_empty() {
            Ok(())
        } else {
            Err(ProjectManifestError::NestedTrailingBytes {
                field,
                actual: self.remaining(),
            })
        }
    }
}
