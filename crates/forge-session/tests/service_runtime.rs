use forge_protocol::identities::ProcessId;
use forge_session::lifecycle::{ServiceFailure, ServiceFailureStage};
use forge_session::service_runtime::{
    ManagedServiceReadiness, ManagedServiceRuntime, ManagedServiceRuntimeError,
    ManagedServiceRuntimeState,
};
use forge_session::services::{ServiceName, StartupRestartPolicy};

fn process(byte: u8) -> ProcessId {
    ProcessId::from_bytes([byte; 16])
}

fn runtime(restarts: u16) -> ManagedServiceRuntime {
    ManagedServiceRuntime::new(
        ServiceName::new("nyx-server").unwrap(),
        StartupRestartPolicy::OnFailure {
            max_restarts: restarts,
        },
    )
}

#[test]
fn exact_start_readiness_and_stop_cycle_is_recorded() {
    let mut service = runtime(1);
    assert_eq!(service.request_start().unwrap(), 1);
    service.record_started(1, process(1)).unwrap();
    service
        .record_readiness(1, process(1), ManagedServiceReadiness::Degraded)
        .unwrap();
    assert_eq!(
        service.state(),
        &ManagedServiceRuntimeState::Ready {
            attempt: 1,
            process_id: process(1),
            readiness: ManagedServiceReadiness::Degraded,
        }
    );
    assert_eq!(service.request_stop().unwrap(), process(1));
    service.record_stopped(process(1)).unwrap();
    assert_eq!(service.state(), &ManagedServiceRuntimeState::Stopped);
}

#[test]
fn duplicate_start_and_forged_process_identity_are_rejected() {
    let mut service = runtime(1);
    service.request_start().unwrap();
    assert!(matches!(
        service.request_start(),
        Err(ManagedServiceRuntimeError::InvalidTransition { .. })
    ));
    service.record_started(1, process(1)).unwrap();
    assert_eq!(
        service
            .record_readiness(1, process(2), ManagedServiceReadiness::Ready)
            .unwrap_err(),
        ManagedServiceRuntimeError::ProcessMismatch {
            expected: process(1),
            actual: process(2),
        }
    );
}

#[test]
fn readiness_failure_consumes_only_the_declared_restart_budget() {
    let mut service = runtime(1);
    service.request_start().unwrap();
    service.record_started(1, process(1)).unwrap();
    service
        .record_attempt_failed(
            1,
            ServiceFailure::new(ServiceFailureStage::Readiness, "not ready"),
        )
        .unwrap();
    assert!(matches!(
        service.state(),
        ManagedServiceRuntimeState::RestartPending {
            next_attempt: 2,
            ..
        }
    ));
    assert_eq!(service.request_start().unwrap(), 2);
    assert_eq!(
        service.record_started(2, process(1)).unwrap_err(),
        ManagedServiceRuntimeError::ReusedProcessIdentity(process(1))
    );
    service
        .record_attempt_failed(
            2,
            ServiceFailure::new(ServiceFailureStage::Start, "spawn failed"),
        )
        .unwrap();
    assert!(matches!(
        service.state(),
        ManagedServiceRuntimeState::Failed { .. }
    ));
}

#[test]
fn unexpected_exit_becomes_restart_pending_then_terminal_failure() {
    let mut service = runtime(1);
    service.request_start().unwrap();
    service.record_started(1, process(1)).unwrap();
    service
        .record_readiness(1, process(1), ManagedServiceReadiness::Ready)
        .unwrap();
    service
        .record_runtime_exit(
            process(1),
            ServiceFailure::new(ServiceFailureStage::Runtime, "crashed"),
        )
        .unwrap();
    assert!(matches!(
        service.state(),
        ManagedServiceRuntimeState::RestartPending {
            next_attempt: 2,
            ..
        }
    ));

    service.request_start().unwrap();
    service.record_started(2, process(2)).unwrap();
    service
        .record_runtime_exit(
            process(2),
            ServiceFailure::new(ServiceFailureStage::Runtime, "crashed again"),
        )
        .unwrap();
    assert!(matches!(
        service.state(),
        ManagedServiceRuntimeState::Failed { .. }
    ));
}

#[test]
fn wrong_failure_stage_cannot_reclassify_a_service_attempt() {
    let mut service = runtime(1);
    service.request_start().unwrap();
    assert_eq!(
        service
            .record_attempt_failed(
                1,
                ServiceFailure::new(ServiceFailureStage::Runtime, "wrong stage"),
            )
            .unwrap_err(),
        ManagedServiceRuntimeError::FailureStage {
            actual: ServiceFailureStage::Runtime,
        }
    );
}
