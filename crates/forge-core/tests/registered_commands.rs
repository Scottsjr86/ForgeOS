use forge_core::commands::{
    CommandAuthorityClass, CommandCancellationPolicy, CommandDefinitionError,
    CommandEnvironmentPolicy, CommandEnvironmentVariable, CommandRegistration, CommandRegistry,
    CommandRegistryError, CommandTimeout, CommandWorkingDirectory, RegisteredCommand,
};
use forge_protocol::identities::{CommandId, IDENTITY_BYTES, RepositoryId};
use forge_protocol::paths::RepositoryRelativePath;
use std::time::Duration;

fn command_id(byte: u8) -> CommandId {
    CommandId::from_bytes([byte; IDENTITY_BYTES])
}

fn repository_id(byte: u8) -> RepositoryId {
    RepositoryId::from_bytes([byte; IDENTITY_BYTES])
}

fn command_with(
    id: u8,
    display_name: &str,
    program: &str,
    arguments: &[&str],
) -> RegisteredCommand {
    RegisteredCommand::new(
        command_id(id),
        repository_id(2),
        display_name,
        program,
        arguments.iter().copied(),
        CommandWorkingDirectory::relative(
            RepositoryRelativePath::new("crates").expect("fixture path"),
        )
        .expect("UTF-8 fixture path"),
        CommandEnvironmentPolicy::clear(vec![
            CommandEnvironmentVariable::literal("RUST_BACKTRACE", "1").unwrap(),
            CommandEnvironmentVariable::inherit("PATH").unwrap(),
        ])
        .unwrap(),
        CommandTimeout::after(Duration::from_secs(30)).unwrap(),
        CommandCancellationPolicy::TerminateProcessGroup,
        CommandAuthorityClass::Build,
    )
    .expect("valid registered command")
}

#[test]
fn exact_definition_has_golden_identity_and_fields() {
    let command = command_with(1, "Workspace test", "cargo", &["test", "--workspace"]);
    assert_eq!(command.command_id(), command_id(1));
    assert_eq!(command.repository_id(), repository_id(2));
    assert_eq!(command.display_name(), "Workspace test");
    assert_eq!(command.program(), "cargo");
    assert_eq!(
        command
            .arguments()
            .iter()
            .map(String::as_str)
            .collect::<Vec<_>>(),
        vec!["test", "--workspace"]
    );
    assert_eq!(
        command
            .working_directory()
            .relative_path()
            .unwrap()
            .as_path(),
        std::path::Path::new("crates")
    );
    assert_eq!(command.timeout().milliseconds(), Some(30_000));
    assert_eq!(command.authority(), CommandAuthorityClass::Build);
    assert_eq!(
        command.definition_identity().to_string(),
        "5c52f41325b9e0c73b9a1cf63acd932ceeb3655a8d6e805d8899e365f80d3b21"
    );
}

#[test]
fn environment_input_order_does_not_change_command_meaning() {
    let first = CommandEnvironmentPolicy::clear(vec![
        CommandEnvironmentVariable::literal("RUST_BACKTRACE", "1").unwrap(),
        CommandEnvironmentVariable::inherit("PATH").unwrap(),
    ])
    .unwrap();
    let second = CommandEnvironmentPolicy::clear(vec![
        CommandEnvironmentVariable::inherit("PATH").unwrap(),
        CommandEnvironmentVariable::literal("RUST_BACKTRACE", "1").unwrap(),
    ])
    .unwrap();
    assert_eq!(first, second);
    assert_eq!(first.variables()[0].name(), "PATH");
    assert_eq!(first.variables()[1].name(), "RUST_BACKTRACE");
}

#[test]
fn argv_tokens_remain_literal_and_are_never_shell_prose() {
    let command = command_with(3, "Literal tokens", "printf", &["%s", "&&", "rm -rf /"]);
    assert_eq!(command.program(), "printf");
    assert_eq!(command.arguments(), ["%s", "&&", "rm -rf /"]);
}

#[test]
fn invalid_program_and_arguments_are_rejected_before_registration() {
    let base = |program: &str, arguments: Vec<String>| {
        RegisteredCommand::new(
            command_id(4),
            repository_id(2),
            "Invalid",
            program,
            arguments,
            CommandWorkingDirectory::repository_root(),
            CommandEnvironmentPolicy::empty(),
            CommandTimeout::Unlimited,
            CommandCancellationPolicy::TerminateProcessGroup,
            CommandAuthorityClass::Inspect,
        )
    };
    assert_eq!(base("", vec![]), Err(CommandDefinitionError::EmptyProgram));
    assert!(matches!(
        base("bad\0program", vec![]),
        Err(CommandDefinitionError::ContainsNul { .. })
    ));
    assert_eq!(
        base("cargo", vec![String::new()]),
        Err(CommandDefinitionError::EmptyArgument { index: 0 })
    );
    assert!(matches!(
        base("cargo", vec!["bad\0argument".into()]),
        Err(CommandDefinitionError::ContainsNul { index: Some(0), .. })
    ));
}

#[test]
fn environment_policy_rejects_implicit_or_ambiguous_names() {
    assert!(matches!(
        CommandEnvironmentVariable::inherit("BAD-NAME"),
        Err(CommandDefinitionError::InvalidEnvironmentName { .. })
    ));
    assert!(matches!(
        CommandEnvironmentVariable::literal("EMPTY", "bad\0value"),
        Err(CommandDefinitionError::ContainsNul { .. })
    ));
    let duplicate = CommandEnvironmentPolicy::clear(vec![
        CommandEnvironmentVariable::inherit("PATH").unwrap(),
        CommandEnvironmentVariable::literal("PATH", "/bin").unwrap(),
    ]);
    assert_eq!(
        duplicate,
        Err(CommandDefinitionError::DuplicateEnvironmentVariable(
            "PATH".to_owned()
        ))
    );
}

#[test]
fn timeout_policy_rejects_zero_and_unbounded_large_values() {
    assert_eq!(
        CommandTimeout::after(Duration::ZERO),
        Err(CommandDefinitionError::ZeroTimeout)
    );
    assert!(matches!(
        CommandTimeout::after(Duration::from_secs(24 * 60 * 60 + 1)),
        Err(CommandDefinitionError::TimeoutTooLarge { .. })
    ));
    assert_eq!(CommandTimeout::Unlimited.milliseconds(), None);
    let direct_zero = RegisteredCommand::new(
        command_id(6),
        repository_id(2),
        "Zero timeout",
        "cargo",
        ["check"],
        CommandWorkingDirectory::repository_root(),
        CommandEnvironmentPolicy::empty(),
        CommandTimeout::Milliseconds(0),
        CommandCancellationPolicy::TerminateProcessGroup,
        CommandAuthorityClass::Inspect,
    );
    assert_eq!(direct_zero, Err(CommandDefinitionError::ZeroTimeout));
}

#[cfg(unix)]
#[test]
fn non_utf8_command_working_directory_is_rejected() {
    use std::ffi::OsString;
    use std::os::unix::ffi::OsStringExt;
    let relative = RepositoryRelativePath::new(OsString::from_vec(vec![0xff])).unwrap();
    assert_eq!(
        CommandWorkingDirectory::relative(relative),
        Err(CommandDefinitionError::WorkingDirectoryNotUtf8)
    );
}

#[test]
fn registry_is_idempotent_but_rejects_silent_identity_reuse() {
    let original = command_with(7, "Check", "cargo", &["check"]);
    let changed = command_with(7, "Test", "cargo", &["test"]);
    let mut registry = CommandRegistry::new();
    assert_eq!(
        registry.register(original.clone()).unwrap(),
        CommandRegistration::Inserted
    );
    assert_eq!(
        registry.register(original).unwrap(),
        CommandRegistration::AlreadyRegistered
    );
    assert!(matches!(
        registry.register(changed),
        Err(CommandRegistryError::IdentityConflict { command_id: id, .. }) if id == command_id(7)
    ));
}

#[test]
fn replacement_requires_the_exact_previous_definition_identity() {
    let original = command_with(8, "Check", "cargo", &["check"]);
    let replacement = command_with(8, "Test", "cargo", &["test"]);
    let stale = command_with(9, "Other", "cargo", &["fmt"]).definition_identity();
    let mut registry = CommandRegistry::new();
    registry.register(original.clone()).unwrap();
    assert!(matches!(
        registry.replace(stale, replacement.clone()),
        Err(CommandRegistryError::StaleDefinition { command_id: id, .. }) if id == command_id(8)
    ));
    let identity = registry
        .replace(original.definition_identity(), replacement.clone())
        .unwrap();
    assert_eq!(identity, replacement.definition_identity());
    assert_eq!(registry.get(command_id(8)), Some(&replacement));
}

#[test]
fn registry_iteration_is_stable_by_command_id() {
    let mut registry = CommandRegistry::new();
    registry
        .register(command_with(3, "Three", "cargo", &["check"]))
        .unwrap();
    registry
        .register(command_with(1, "One", "cargo", &["test"]))
        .unwrap();
    registry
        .register(command_with(2, "Two", "cargo", &["fmt"]))
        .unwrap();
    let ids = registry
        .iter()
        .map(RegisteredCommand::command_id)
        .collect::<Vec<_>>();
    assert_eq!(ids, vec![command_id(1), command_id(2), command_id(3)]);
}

#[test]
fn every_execution_policy_change_changes_definition_identity() {
    let original = command_with(11, "Check", "cargo", &["check"]);
    let changed_authority = RegisteredCommand::new(
        original.command_id(),
        original.repository_id(),
        original.display_name(),
        original.program(),
        original.arguments().iter().cloned(),
        original.working_directory().clone(),
        original.environment().clone(),
        original.timeout(),
        original.cancellation(),
        CommandAuthorityClass::WorkspaceWrite,
    )
    .unwrap();
    assert_ne!(
        original.definition_identity(),
        changed_authority.definition_identity()
    );
}
