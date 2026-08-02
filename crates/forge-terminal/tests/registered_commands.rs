use forge_core::commands::{
    CommandAuthorityClass, CommandCancellationPolicy, CommandEnvironmentPolicy,
    CommandEnvironmentVariable, CommandTimeout, CommandWorkingDirectory, RegisteredCommand,
};
use forge_protocol::identities::{CommandId, ProcessId, RepositoryId, IDENTITY_BYTES};
use forge_protocol::paths::RepositoryRelativePath;
use forge_terminal::commands::{CommandDirectoryBinding, CommandLaunchError, CommandLaunchPayload};
use std::collections::BTreeMap;
use std::path::Path;
use std::time::Duration;

fn command_id(byte: u8) -> CommandId {
    CommandId::from_bytes([byte; IDENTITY_BYTES])
}

fn process_id(byte: u8) -> ProcessId {
    ProcessId::from_bytes([byte; IDENTITY_BYTES])
}

fn repository_id(byte: u8) -> RepositoryId {
    RepositoryId::from_bytes([byte; IDENTITY_BYTES])
}

fn command(timeout: CommandTimeout) -> RegisteredCommand {
    RegisteredCommand::new(
        command_id(1),
        repository_id(2),
        "Workspace test",
        "cargo",
        ["test", "--workspace"],
        CommandWorkingDirectory::relative(RepositoryRelativePath::new("crates").unwrap())
            .unwrap(),
        CommandEnvironmentPolicy::clear(vec![
            CommandEnvironmentVariable::literal("RUST_BACKTRACE", "1").unwrap(),
            CommandEnvironmentVariable::inherit("PATH").unwrap(),
        ])
        .unwrap(),
        timeout,
        CommandCancellationPolicy::TerminateProcessGroup,
        CommandAuthorityClass::Build,
    )
    .unwrap()
}

fn binding() -> CommandDirectoryBinding {
    CommandDirectoryBinding::relative(
        repository_id(2),
        RepositoryRelativePath::new("crates").unwrap(),
        "/work/project/crates",
    )
    .unwrap()
}

fn host_environment() -> BTreeMap<String, String> {
    BTreeMap::from([
        ("PATH".to_owned(), "/usr/bin:/bin".to_owned()),
        (
            "AWS_SECRET_ACCESS_KEY".to_owned(),
            "must-not-leak".to_owned(),
        ),
    ])
}

#[test]
fn exact_shell_free_launch_payload_is_inspectable() {
    let command = command(CommandTimeout::after(Duration::from_secs(30)).unwrap());
    let payload = CommandLaunchPayload::prepare(
        process_id(3),
        &command,
        &binding(),
        &host_environment(),
    )
    .unwrap();
    assert_eq!(payload.command_id(), command.command_id());
    assert_eq!(payload.command_definition(), command.definition_identity());
    assert_eq!(payload.repository_id(), repository_id(2));
    assert_eq!(payload.process_request().process_id(), process_id(3));
    assert_eq!(payload.process_request().program(), "cargo");
    assert_eq!(
        payload
            .process_request()
            .arguments()
            .iter()
            .map(String::as_str)
            .collect::<Vec<_>>(),
        vec!["test", "--workspace"]
    );
    assert_eq!(
        payload.process_request().timeout(),
        Some(Duration::from_secs(30))
    );
    assert_eq!(payload.working_directory(), Path::new("/work/project/crates"));
    assert_eq!(payload.authority(), CommandAuthorityClass::Build);
    assert_eq!(
        payload.cancellation(),
        CommandCancellationPolicy::TerminateProcessGroup
    );
}

#[test]
fn parent_environment_is_cleared_and_undeclared_secrets_are_ignored() {
    let payload = CommandLaunchPayload::prepare(
        process_id(4),
        &command(CommandTimeout::Unlimited),
        &binding(),
        &host_environment(),
    )
    .unwrap();
    assert!(payload.clears_parent_environment());
    let pairs = payload
        .environment()
        .iter()
        .map(|entry| (entry.name(), entry.value()))
        .collect::<Vec<_>>();
    assert_eq!(
        pairs,
        vec![("PATH", "/usr/bin:/bin"), ("RUST_BACKTRACE", "1")]
    );
    assert!(!pairs.iter().any(|(name, _)| *name == "AWS_SECRET_ACCESS_KEY"));
}

#[test]
fn missing_declared_inheritance_is_a_typed_failure() {
    let error = CommandLaunchPayload::prepare(
        process_id(5),
        &command(CommandTimeout::Unlimited),
        &binding(),
        &BTreeMap::new(),
    )
    .unwrap_err();
    assert_eq!(
        error,
        CommandLaunchError::MissingDeclaredEnvironmentVariable("PATH".to_owned())
    );
}

#[test]
fn foreign_repository_directory_is_rejected() {
    let foreign = CommandDirectoryBinding::relative(
        repository_id(9),
        RepositoryRelativePath::new("crates").unwrap(),
        "/work/foreign/crates",
    )
    .unwrap();
    assert!(matches!(
        CommandLaunchPayload::prepare(
            process_id(6),
            &command(CommandTimeout::Unlimited),
            &foreign,
            &host_environment()
        ),
        Err(CommandLaunchError::RepositoryMismatch { command, directory })
            if command == repository_id(2) && directory == repository_id(9)
    ));
}

#[test]
fn wrong_declared_working_directory_is_rejected() {
    let root = CommandDirectoryBinding::repository_root(repository_id(2), "/work/project").unwrap();
    assert_eq!(
        CommandLaunchPayload::prepare(
            process_id(7),
            &command(CommandTimeout::Unlimited),
            &root,
            &host_environment()
        ),
        Err(CommandLaunchError::WorkingDirectoryMismatch)
    );
}

#[test]
fn directory_binding_requires_an_absolute_resolved_path() {
    assert_eq!(
        CommandDirectoryBinding::repository_root(repository_id(2), "relative/path"),
        Err(CommandLaunchError::WorkingDirectoryNotAbsolute(
            "relative/path".into()
        ))
    );
}

#[test]
fn unlimited_timeout_remains_explicitly_unlimited() {
    let payload = CommandLaunchPayload::prepare(
        process_id(8),
        &command(CommandTimeout::Unlimited),
        &binding(),
        &host_environment(),
    )
    .unwrap();
    assert_eq!(payload.process_request().timeout(), None);
}

#[test]
fn invalid_inherited_environment_value_is_rejected_before_launch() {
    let source = BTreeMap::from([("PATH".to_owned(), "bad\0path".to_owned())]);
    assert!(matches!(
        CommandLaunchPayload::prepare(
            process_id(11),
            &command(CommandTimeout::Unlimited),
            &binding(),
            &source
        ),
        Err(CommandLaunchError::InvalidDefinition(_))
    ));
}

#[test]
fn literal_environment_values_do_not_read_the_host_source() {
    let command = RegisteredCommand::new(
        command_id(9),
        repository_id(2),
        "Literal env",
        "env",
        ["--null"],
        CommandWorkingDirectory::repository_root(),
        CommandEnvironmentPolicy::clear(vec![
            CommandEnvironmentVariable::literal("TOKEN", "declared").unwrap(),
        ])
        .unwrap(),
        CommandTimeout::Unlimited,
        CommandCancellationPolicy::TerminateProcessGroup,
        CommandAuthorityClass::Inspect,
    )
    .unwrap();
    let directory =
        CommandDirectoryBinding::repository_root(repository_id(2), "/work/project").unwrap();
    let source = BTreeMap::from([("TOKEN".to_owned(), "host-secret".to_owned())]);
    let payload =
        CommandLaunchPayload::prepare(process_id(10), &command, &directory, &source).unwrap();
    assert_eq!(payload.environment()[0].name(), "TOKEN");
    assert_eq!(payload.environment()[0].value(), "declared");
}
