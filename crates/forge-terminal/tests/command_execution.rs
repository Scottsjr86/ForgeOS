#![cfg(target_os = "linux")]

use forge_bridge::processes::CancellationToken;
use forge_core::commands::{
    CommandAuthorityClass, CommandCancellationPolicy, CommandEnvironmentPolicy,
    CommandEnvironmentVariable, CommandTimeout, CommandWorkingDirectory, RegisteredCommand,
};
use forge_protocol::hashes::ContentHash;
use forge_protocol::identities::{CommandId, ProcessId, ProjectId, RepositoryId, IDENTITY_BYTES};
use forge_protocol::processes::ProcessOutcome;
use forge_terminal::commands::CommandDirectoryBinding;
use forge_terminal::execution::{CommandRunError, CommandRunRegistry, CommandSourceBinding};
use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

static NEXT_FIXTURE: AtomicU64 = AtomicU64::new(1);

fn id<T>(byte: u8, make: impl FnOnce([u8; IDENTITY_BYTES]) -> T) -> T {
    make([byte; IDENTITY_BYTES])
}

fn fixture_directory(label: &str) -> PathBuf {
    let sequence = NEXT_FIXTURE.fetch_add(1, Ordering::Relaxed);
    let path = std::env::temp_dir().join(format!(
        "forgeos-command-execution-{label}-{}-{sequence}",
        std::process::id()
    ));
    if path.exists() {
        fs::remove_dir_all(&path).expect("remove stale fixture");
    }
    fs::create_dir_all(&path).expect("create fixture");
    fs::canonicalize(path).expect("canonical fixture")
}

fn command(
    command_byte: u8,
    repository_id: RepositoryId,
    script: &str,
    timeout: CommandTimeout,
    environment: CommandEnvironmentPolicy,
) -> RegisteredCommand {
    RegisteredCommand::new(
        id(command_byte, CommandId::from_bytes),
        repository_id,
        format!("Command {command_byte}"),
        "/bin/sh",
        ["-c", script],
        CommandWorkingDirectory::repository_root(),
        environment,
        timeout,
        CommandCancellationPolicy::TerminateProcessGroup,
        CommandAuthorityClass::Build,
    )
    .expect("valid command")
}

fn source(repository_id: RepositoryId, revision_byte: u8) -> CommandSourceBinding {
    CommandSourceBinding::new(
        id(10, ProjectId::from_bytes),
        repository_id,
        ContentHash::from_bytes([revision_byte; 32]),
    )
}

#[test]
fn passing_and_failing_commands_preserve_exact_output_and_exit_state() {
    let directory_path = fixture_directory("outcomes");
    let repository_id = id(20, RepositoryId::from_bytes);
    let directory = CommandDirectoryBinding::repository_root(repository_id, &directory_path)
        .expect("directory binding");
    let mut runs = CommandRunRegistry::new();

    let passing = command(
        1,
        repository_id,
        "printf 'pass-out'; printf 'pass-err' >&2; exit 0",
        CommandTimeout::Unlimited,
        CommandEnvironmentPolicy::empty(),
    );
    let pass = runs
        .run(
            source(repository_id, 30),
            id(40, ProcessId::from_bytes),
            &passing,
            &directory,
            &BTreeMap::new(),
            &CancellationToken::new(),
        )
        .expect("passing run");
    assert_eq!(pass.execution().output().stdout(), b"pass-out");
    assert_eq!(pass.execution().output().stderr(), b"pass-err");
    assert!(matches!(
        pass.execution().outcome(),
        ProcessOutcome::Exited(exit) if exit.success() && exit.code() == Some(0)
    ));

    let failing = command(
        2,
        repository_id,
        "printf 'failed' >&2; exit 7",
        CommandTimeout::Unlimited,
        CommandEnvironmentPolicy::empty(),
    );
    let failure = runs
        .run(
            source(repository_id, 30),
            id(41, ProcessId::from_bytes),
            &failing,
            &directory,
            &BTreeMap::new(),
            &CancellationToken::new(),
        )
        .expect("failing command still records an execution");
    assert_eq!(failure.execution().output().stderr(), b"failed");
    assert!(matches!(
        failure.execution().outcome(),
        ProcessOutcome::Exited(exit) if !exit.success() && exit.code() == Some(7)
    ));

    fs::remove_dir_all(directory_path).expect("remove fixture");
}

#[test]
fn command_uses_exact_working_directory_and_declared_environment_only() {
    let directory_path = fixture_directory("context");
    let repository_id = id(21, RepositoryId::from_bytes);
    let directory = CommandDirectoryBinding::repository_root(repository_id, &directory_path)
        .expect("directory binding");
    let environment = CommandEnvironmentPolicy::clear(vec![
        CommandEnvironmentVariable::literal("LITERAL", "fixed").unwrap(),
        CommandEnvironmentVariable::inherit("INHERITED").unwrap(),
    ])
    .unwrap();
    let command = command(
        3,
        repository_id,
        "printf '%s\n%s\n%s\n' \"$PWD\" \"$LITERAL\" \"$INHERITED\"; test -z \"$SHOULD_NOT_LEAK\"",
        CommandTimeout::Unlimited,
        environment,
    );
    let source_environment = BTreeMap::from([
        ("INHERITED".to_owned(), "declared".to_owned()),
        ("SHOULD_NOT_LEAK".to_owned(), "secret".to_owned()),
    ]);
    let mut runs = CommandRunRegistry::new();
    let record = runs
        .run(
            source(repository_id, 31),
            id(42, ProcessId::from_bytes),
            &command,
            &directory,
            &source_environment,
            &CancellationToken::new(),
        )
        .expect("configured command");
    let stdout = String::from_utf8(record.execution().output().stdout().to_vec()).unwrap();
    assert_eq!(
        stdout,
        format!("{}\nfixed\ndeclared\n", directory_path.display())
    );
    assert!(record.clears_parent_environment());
    assert_eq!(record.environment().len(), 2);

    fs::remove_dir_all(directory_path).expect("remove fixture");
}

#[test]
fn timeout_and_cancellation_are_distinct_recorded_outcomes() {
    let directory_path = fixture_directory("stop");
    let repository_id = id(22, RepositoryId::from_bytes);
    let directory = CommandDirectoryBinding::repository_root(repository_id, &directory_path)
        .expect("directory binding");
    let mut runs = CommandRunRegistry::new();

    let timeout = command(
        4,
        repository_id,
        "exec /bin/sleep 5",
        CommandTimeout::after(Duration::from_millis(30)).unwrap(),
        CommandEnvironmentPolicy::empty(),
    );
    let timed_out = runs
        .run(
            source(repository_id, 32),
            id(43, ProcessId::from_bytes),
            &timeout,
            &directory,
            &BTreeMap::new(),
            &CancellationToken::new(),
        )
        .expect("timeout record");
    assert_eq!(timed_out.execution().outcome(), &ProcessOutcome::TimedOut);

    let cancelled_command = command(
        5,
        repository_id,
        "exec /bin/sleep 5",
        CommandTimeout::Unlimited,
        CommandEnvironmentPolicy::empty(),
    );
    let cancelled = CancellationToken::new();
    cancelled.cancel();
    let cancelled_record = runs
        .run(
            source(repository_id, 32),
            id(44, ProcessId::from_bytes),
            &cancelled_command,
            &directory,
            &BTreeMap::new(),
            &cancelled,
        )
        .expect("cancelled record");
    assert_eq!(
        cancelled_record.execution().outcome(),
        &ProcessOutcome::Cancelled
    );

    fs::remove_dir_all(directory_path).expect("remove fixture");
}

#[test]
fn history_is_bound_to_exact_source_definition_and_unique_process_identity() {
    let directory_path = fixture_directory("history");
    let repository_id = id(23, RepositoryId::from_bytes);
    let directory = CommandDirectoryBinding::repository_root(repository_id, &directory_path)
        .expect("directory binding");
    let command = command(
        6,
        repository_id,
        "exit 0",
        CommandTimeout::Unlimited,
        CommandEnvironmentPolicy::empty(),
    );
    let process_id = id(45, ProcessId::from_bytes);
    let source = source(repository_id, 33);
    let mut runs = CommandRunRegistry::new();
    let record = runs
        .run(
            source,
            process_id,
            &command,
            &directory,
            &BTreeMap::new(),
            &CancellationToken::new(),
        )
        .expect("first run")
        .clone();
    assert_eq!(record.source(), source);
    assert_eq!(record.command_definition(), command.definition_identity());
    assert_eq!(record.command_id(), command.command_id());
    assert_eq!(record.process_id(), process_id);
    assert_eq!(record.process_request().program(), "/bin/sh");
    assert_eq!(
        record
            .process_request()
            .arguments()
            .iter()
            .map(String::as_str)
            .collect::<Vec<_>>(),
        vec!["-c", "exit 0"]
    );
    assert_eq!(runs.get(process_id), Some(&record));
    assert_eq!(runs.len(), 1);

    assert_eq!(
        runs.run(
            source,
            process_id,
            &command,
            &directory,
            &BTreeMap::new(),
            &CancellationToken::new(),
        ),
        Err(CommandRunError::DuplicateProcess(process_id))
    );

    fs::remove_dir_all(directory_path).expect("remove fixture");
}

#[test]
fn source_repository_mismatch_fails_before_spawn() {
    let directory_path = fixture_directory("foreign");
    let repository_id = id(24, RepositoryId::from_bytes);
    let directory = CommandDirectoryBinding::repository_root(repository_id, &directory_path)
        .expect("directory binding");
    let command = command(
        7,
        repository_id,
        "printf 'must-not-run'",
        CommandTimeout::Unlimited,
        CommandEnvironmentPolicy::empty(),
    );
    let foreign = id(25, RepositoryId::from_bytes);
    let mut runs = CommandRunRegistry::new();
    assert_eq!(
        runs.run(
            source(foreign, 34),
            id(46, ProcessId::from_bytes),
            &command,
            &directory,
            &BTreeMap::new(),
            &CancellationToken::new(),
        ),
        Err(CommandRunError::RepositoryMismatch {
            source: foreign,
            command: repository_id,
        })
    );
    assert!(runs.is_empty());

    fs::remove_dir_all(directory_path).expect("remove fixture");
}
