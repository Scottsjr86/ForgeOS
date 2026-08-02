use forge_protocol::identities::{ProcessId, SessionId, IDENTITY_BYTES};
use forge_session::lifecycle::{
    LifecycleAction, ServiceFailure, ServiceFailureStage, ServiceStatus, SessionPhase,
    SessionSupervisor, StopReason, SupervisorError,
};
use forge_session::services::{ManagedService, ServiceName, ServicePlan, StartupRestartPolicy};

fn session(byte: u8) -> SessionId {
    SessionId::from_bytes([byte; IDENTITY_BYTES])
}

fn process(byte: u8) -> ProcessId {
    ProcessId::from_bytes([byte; IDENTITY_BYTES])
}

fn name(value: &str) -> ServiceName {
    ServiceName::new(value).unwrap()
}

fn service(
    value: &str,
    order: u16,
    dependencies: &[&str],
    restart_policy: StartupRestartPolicy,
) -> ManagedService {
    ManagedService::new(
        name(value),
        order,
        dependencies.iter().map(|dependency| name(dependency)),
        restart_policy,
    )
    .unwrap()
}

fn two_service_plan(restart_policy: StartupRestartPolicy) -> ServicePlan {
    ServicePlan::new([
        service("forge-core", 10, &[], restart_policy),
        service(
            "forge-world",
            20,
            &["forge-core"],
            StartupRestartPolicy::Never,
        ),
    ])
    .unwrap()
}

fn expect_start(action: LifecycleAction, service: &str, attempt: u16) {
    assert!(matches!(
        action,
        LifecycleAction::Start {
            service: actual,
            attempt: actual_attempt,
            ..
        } if actual == name(service) && actual_attempt == attempt
    ));
}

fn start_and_ready(
    supervisor: &mut SessionSupervisor,
    service: &str,
    attempt: u16,
    process_id: ProcessId,
) {
    expect_start(supervisor.take_next_action().unwrap(), service, attempt);
    supervisor
        .record_started(&name(service), attempt, process_id)
        .unwrap();
    assert_eq!(supervisor.take_next_action(), None);
    supervisor.record_ready(&name(service), process_id).unwrap();
}

#[test]
fn startup_waits_for_explicit_readiness_and_dependency_order() {
    let mut supervisor =
        SessionSupervisor::new(session(1), two_service_plan(StartupRestartPolicy::Never));
    expect_start(supervisor.take_next_action().unwrap(), "forge-core", 1);
    supervisor
        .record_started(&name("forge-core"), 1, process(1))
        .unwrap();
    assert_eq!(supervisor.take_next_action(), None);
    supervisor
        .record_ready(&name("forge-core"), process(1))
        .unwrap();
    expect_start(supervisor.take_next_action().unwrap(), "forge-world", 1);
    supervisor
        .record_started(&name("forge-world"), 1, process(2))
        .unwrap();
    supervisor
        .record_ready(&name("forge-world"), process(2))
        .unwrap();
    assert_eq!(
        supervisor.take_next_action(),
        Some(LifecycleAction::SessionReady {
            session_id: session(1)
        })
    );
    assert_eq!(supervisor.phase(), SessionPhase::Ready);
}

#[test]
fn failed_start_retries_only_as_declared() {
    let mut supervisor = SessionSupervisor::new(
        session(2),
        two_service_plan(StartupRestartPolicy::OnFailure { max_restarts: 1 }),
    );
    expect_start(supervisor.take_next_action().unwrap(), "forge-core", 1);
    supervisor
        .record_start_failed(
            &name("forge-core"),
            1,
            ServiceFailure::new(ServiceFailureStage::Start, "exit 1"),
        )
        .unwrap();
    expect_start(supervisor.take_next_action().unwrap(), "forge-core", 2);
    supervisor
        .record_start_failed(
            &name("forge-core"),
            2,
            ServiceFailure::new(ServiceFailureStage::Start, "exit 2"),
        )
        .unwrap();
    assert!(matches!(
        supervisor.take_next_action(),
        Some(LifecycleAction::SessionFailed { service, .. }) if service == name("forge-core")
    ));
    assert_eq!(supervisor.phase(), SessionPhase::Failed);
}

#[test]
fn readiness_failure_stops_exact_process_before_retry() {
    let mut supervisor = SessionSupervisor::new(
        session(3),
        two_service_plan(StartupRestartPolicy::OnFailure { max_restarts: 1 }),
    );
    expect_start(supervisor.take_next_action().unwrap(), "forge-core", 1);
    supervisor
        .record_started(&name("forge-core"), 1, process(3))
        .unwrap();
    supervisor
        .record_readiness_failed(
            &name("forge-core"),
            process(3),
            ServiceFailure::new(ServiceFailureStage::Readiness, "not ready"),
        )
        .unwrap();
    assert_eq!(
        supervisor.take_next_action(),
        Some(LifecycleAction::Stop {
            session_id: session(3),
            service: name("forge-core"),
            process_id: process(3),
            reason: StopReason::RestartCleanup,
        })
    );
    supervisor
        .record_stopped(&name("forge-core"), process(3))
        .unwrap();
    expect_start(supervisor.take_next_action().unwrap(), "forge-core", 2);
}

#[test]
fn exhausted_start_failure_rolls_back_ready_services() {
    let plan = ServicePlan::new([
        service("forge-core", 10, &[], StartupRestartPolicy::Never),
        service("nyx", 20, &["forge-core"], StartupRestartPolicy::Never),
    ])
    .unwrap();
    let mut supervisor = SessionSupervisor::new(session(4), plan);
    start_and_ready(&mut supervisor, "forge-core", 1, process(4));
    expect_start(supervisor.take_next_action().unwrap(), "nyx", 1);
    supervisor
        .record_start_failed(
            &name("nyx"),
            1,
            ServiceFailure::new(ServiceFailureStage::Start, "bind failed"),
        )
        .unwrap();
    assert_eq!(
        supervisor.take_next_action(),
        Some(LifecycleAction::Stop {
            session_id: session(4),
            service: name("forge-core"),
            process_id: process(4),
            reason: StopReason::StartupRollback,
        })
    );
    supervisor
        .record_stopped(&name("forge-core"), process(4))
        .unwrap();
    assert!(matches!(
        supervisor.take_next_action(),
        Some(LifecycleAction::SessionFailed { service, .. }) if service == name("nyx")
    ));
}

#[test]
fn clean_shutdown_is_strict_reverse_startup_order() {
    let mut supervisor =
        SessionSupervisor::new(session(5), two_service_plan(StartupRestartPolicy::Never));
    start_and_ready(&mut supervisor, "forge-core", 1, process(5));
    start_and_ready(&mut supervisor, "forge-world", 1, process(6));
    assert!(matches!(
        supervisor.take_next_action(),
        Some(LifecycleAction::SessionReady { .. })
    ));
    supervisor.request_shutdown().unwrap();
    assert_eq!(
        supervisor.take_next_action(),
        Some(LifecycleAction::Stop {
            session_id: session(5),
            service: name("forge-world"),
            process_id: process(6),
            reason: StopReason::Shutdown,
        })
    );
    supervisor
        .record_stopped(&name("forge-world"), process(6))
        .unwrap();
    assert_eq!(
        supervisor.take_next_action(),
        Some(LifecycleAction::Stop {
            session_id: session(5),
            service: name("forge-core"),
            process_id: process(5),
            reason: StopReason::Shutdown,
        })
    );
    supervisor
        .record_stopped(&name("forge-core"), process(5))
        .unwrap();
    assert_eq!(
        supervisor.take_next_action(),
        Some(LifecycleAction::SessionStopped {
            session_id: session(5)
        })
    );
    assert_eq!(supervisor.phase(), SessionPhase::Stopped);
}

#[test]
fn stop_failure_is_preserved_while_other_services_are_stopped() {
    let mut supervisor =
        SessionSupervisor::new(session(6), two_service_plan(StartupRestartPolicy::Never));
    start_and_ready(&mut supervisor, "forge-core", 1, process(7));
    start_and_ready(&mut supervisor, "forge-world", 1, process(8));
    supervisor.take_next_action();
    supervisor.request_shutdown().unwrap();
    supervisor.take_next_action();
    supervisor
        .record_stop_failed(
            &name("forge-world"),
            process(8),
            ServiceFailure::new(ServiceFailureStage::Stop, "permission denied"),
        )
        .unwrap();
    assert_eq!(
        supervisor
            .service_failure(&name("forge-world"))
            .unwrap()
            .native_message(),
        "permission denied"
    );
    assert!(matches!(
        supervisor.take_next_action(),
        Some(LifecycleAction::Stop { service, .. }) if service == name("forge-core")
    ));
    supervisor
        .record_stopped(&name("forge-core"), process(7))
        .unwrap();
    assert!(matches!(
        supervisor.take_next_action(),
        Some(LifecycleAction::SessionFailed { service, failure, .. })
            if service == name("forge-world")
                && failure.native_message() == "permission denied"
    ));
}

#[test]
fn unexpected_runtime_exit_rolls_back_remaining_ready_services() {
    let mut supervisor =
        SessionSupervisor::new(session(7), two_service_plan(StartupRestartPolicy::Never));
    start_and_ready(&mut supervisor, "forge-core", 1, process(9));
    start_and_ready(&mut supervisor, "forge-world", 1, process(10));
    supervisor.take_next_action();
    supervisor
        .record_unexpected_exit(
            &name("forge-core"),
            process(9),
            ServiceFailure::new(ServiceFailureStage::Runtime, "signal 9"),
        )
        .unwrap();
    assert!(matches!(
        supervisor.take_next_action(),
        Some(LifecycleAction::Stop {
            service,
            process_id,
            reason: StopReason::StartupRollback,
            ..
        }) if service == name("forge-world") && process_id == process(10)
    ));
}

#[test]
fn wrong_attempt_and_process_identity_are_rejected() {
    let mut supervisor =
        SessionSupervisor::new(session(8), two_service_plan(StartupRestartPolicy::Never));
    supervisor.take_next_action();
    assert!(matches!(
        supervisor.record_started(&name("forge-core"), 2, process(11)),
        Err(SupervisorError::AttemptMismatch { .. })
    ));
    supervisor
        .record_started(&name("forge-core"), 1, process(11))
        .unwrap();
    assert!(matches!(
        supervisor.record_ready(&name("forge-core"), process(12)),
        Err(SupervisorError::ProcessMismatch { .. })
    ));
}

#[test]
fn destructive_shutdown_is_rejected_before_session_ready() {
    let mut supervisor =
        SessionSupervisor::new(session(9), two_service_plan(StartupRestartPolicy::Never));
    assert_eq!(
        supervisor.request_shutdown(),
        Err(SupervisorError::PhaseMismatch {
            expected: SessionPhase::Ready,
            actual: SessionPhase::Starting,
        })
    );
}

#[test]
fn independent_supervisors_do_not_share_service_state() {
    let plan = two_service_plan(StartupRestartPolicy::Never);
    let mut first = SessionSupervisor::new(session(10), plan.clone());
    let second = SessionSupervisor::new(session(11), plan);
    first.take_next_action();
    first
        .record_started(&name("forge-core"), 1, process(13))
        .unwrap();
    assert_eq!(
        first.service_status(&name("forge-core")),
        Some(ServiceStatus::Running {
            attempt: 1,
            process_id: process(13),
        })
    );
    assert_eq!(
        second.service_status(&name("forge-core")),
        Some(ServiceStatus::Pending { next_attempt: 1 })
    );
}
