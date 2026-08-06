use forge_core::recovery::{InterruptedEffectState, RecoveredProcessState};
use forge_protocol::identities::{IDENTITY_BYTES, ProcessId, SessionId};
use forge_session::lifecycle::{LifecycleAction, SessionSupervisor};
use forge_session::services::{ManagedService, ServiceName, ServicePlan, StartupRestartPolicy};

fn name(value: &str) -> ServiceName {
    ServiceName::new(value).unwrap()
}

fn plan() -> ServicePlan {
    ServicePlan::new([ManagedService::new(
        name("nyx"),
        1,
        Vec::<ServiceName>::new(),
        StartupRestartPolicy::Never,
    )
    .unwrap()])
    .unwrap()
}

fn session() -> SessionId {
    SessionId::from_bytes([3; IDENTITY_BYTES])
}

fn process() -> ProcessId {
    ProcessId::from_bytes([9; IDENTITY_BYTES])
}

#[test]
fn start_and_stop_residue_is_non_replayable_and_process_liveness_is_unknown() {
    let mut supervisor = SessionSupervisor::new(session(), plan());
    assert!(matches!(
        supervisor.take_next_action(),
        Some(LifecycleAction::Start { .. })
    ));

    let starting = supervisor.recovery_evidence();
    assert_eq!(starting.interrupted_actions().len(), 1);
    assert_eq!(
        starting.interrupted_actions()[0].effect_state(),
        InterruptedEffectState::CommitNotObserved
    );
    assert!(!starting.interrupted_actions()[0].replay_allowed());
    assert!(starting.recorded_processes().is_empty());

    supervisor
        .record_started(&name("nyx"), 1, process())
        .unwrap();
    let running = supervisor.recovery_evidence();
    assert!(running.interrupted_actions().is_empty());
    assert_eq!(running.recorded_processes().len(), 1);
    assert_eq!(
        running.recorded_processes()[0].state(),
        RecoveredProcessState::RequiresRevalidation
    );
    assert!(!running.recorded_processes()[0].claims_alive());

    supervisor.record_ready(&name("nyx"), process()).unwrap();
    assert!(matches!(
        supervisor.take_next_action(),
        Some(LifecycleAction::SessionReady { .. })
    ));
    supervisor.request_shutdown().unwrap();
    assert!(matches!(
        supervisor.take_next_action(),
        Some(LifecycleAction::Stop { .. })
    ));

    let stopping = supervisor.recovery_evidence();
    assert_eq!(stopping.interrupted_actions().len(), 1);
    assert_eq!(
        stopping.interrupted_actions()[0].effect_state(),
        InterruptedEffectState::CommitUnknown
    );
    assert_eq!(
        stopping.recorded_processes()[0].state(),
        RecoveredProcessState::RequiresRevalidation
    );

    supervisor.record_stopped(&name("nyx"), process()).unwrap();
    let stopped = supervisor.recovery_evidence();
    assert!(stopped.interrupted_actions().is_empty());
    assert_eq!(
        stopped.recorded_processes()[0].state(),
        RecoveredProcessState::ConfirmedStopped
    );
    assert_eq!(stopped.recorded_processes()[0].prior_process_id(), None);
}
