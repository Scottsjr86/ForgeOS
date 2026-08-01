#![cfg(unix)]

use forge_bridge::processes::{CancellationToken, ProcessRunner};
use forge_protocol::identities::ProcessId;
use forge_protocol::processes::{
    ProcessExit, ProcessFailureStage, ProcessOutcome, ProcessSpawnRequest,
};
use std::time::Duration;

fn process_id(byte: u8) -> ProcessId {
    ProcessId::from_bytes([byte; 16])
}

fn runner() -> ProcessRunner {
    ProcessRunner::with_timings(Duration::from_millis(5), Duration::from_millis(100))
        .expect("fixture timings")
}

#[test]
fn public_runner_reports_fast_failing_timeout_cancel_and_spawn_failure() {
    let success = runner().run(
        ProcessSpawnRequest::new(process_id(1), "/bin/sh", ["-c", "exit 0"])
            .expect("success request"),
        &CancellationToken::new(),
    );
    assert_eq!(
        success.outcome(),
        &ProcessOutcome::Exited(ProcessExit::new(Some(0), true))
    );

    let failing = runner().run(
        ProcessSpawnRequest::new(process_id(2), "/bin/sh", ["-c", "exit 23"])
            .expect("failing request"),
        &CancellationToken::new(),
    );
    assert_eq!(
        failing.outcome(),
        &ProcessOutcome::Exited(ProcessExit::new(Some(23), false))
    );

    let timeout = runner().run(
        ProcessSpawnRequest::new(process_id(3), "/bin/sh", ["-c", "sleep 5"])
            .expect("timeout request")
            .with_timeout(Duration::from_millis(30)),
        &CancellationToken::new(),
    );
    assert_eq!(timeout.outcome(), &ProcessOutcome::TimedOut);

    let cancelled_token = CancellationToken::new();
    cancelled_token.cancel();
    let cancelled = runner().run(
        ProcessSpawnRequest::new(process_id(4), "/bin/sh", ["-c", "sleep 5"])
            .expect("cancel request")
            .with_timeout(Duration::from_secs(2)),
        &cancelled_token,
    );
    assert_eq!(cancelled.outcome(), &ProcessOutcome::Cancelled);

    let missing = runner().run(
        ProcessSpawnRequest::new(
            process_id(5),
            "/forgeos/fixture/definitely-not-an-executable",
            [] as [&str; 0],
        )
        .expect("missing executable request is structurally valid"),
        &CancellationToken::new(),
    );
    match missing.outcome() {
        ProcessOutcome::Failed(failure) => {
            assert_eq!(failure.stage(), ProcessFailureStage::Spawn);
        }
        other => panic!("expected explicit spawn failure, found {other:?}"),
    }
    assert_eq!(missing.process_id(), process_id(5));
    assert_eq!(missing.system_pid(), None);
}
