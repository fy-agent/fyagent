use fyagent_lib::configuration_apply::{
    ApplyEffect, ApplyJobStatus, ApplyRecoveryAction, FakeApplyCoordinator,
};

#[test]
fn configuration_apply_cancel_before_write_closes_with_effect_none() {
    // Given: a process-local fake coordinator that has not started writing
    let coordinator = FakeApplyCoordinator::succeeding();
    coordinator.request_cancel();

    // When: the run observes cancel at the last pre-write safe point
    let outcome = coordinator.run();

    // Then: typed none, no writer invocation
    assert_eq!(outcome.effect, ApplyEffect::None);
    assert_eq!(outcome.status, ApplyJobStatus::Cancelled);
    assert_eq!(outcome.writer_count, 0);
}

#[test]
fn configuration_apply_writer_fail_is_not_green() {
    // Given: a fake adapter whose single writer call fails
    let coordinator = FakeApplyCoordinator::failing_writer();

    // When: the coordinator runs backup → writer → readback
    let outcome = coordinator.run();

    // Then: not green, no success effect, recovery is required
    assert_ne!(outcome.status, ApplyJobStatus::Succeeded);
    assert_ne!(outcome.effect, ApplyEffect::Applied);
    assert_eq!(outcome.status, ApplyJobStatus::Failed);
    assert!(
        outcome
            .recovery_actions
            .iter()
            .any(|action| matches!(action, ApplyRecoveryAction::RestoreBackup)),
        "writer failure must expose restore recovery"
    );
}

#[test]
fn configuration_apply_makes_zero_outbound_provider_http_calls() {
    // Given: a succeeding fake coordinator with an outbound HTTP spy
    let coordinator = FakeApplyCoordinator::succeeding();

    // When: the fake adapter runs the full local path
    let outcome = coordinator.run();

    // Then: the spy records no Provider HTTP
    assert_eq!(outcome.outbound_provider_http_count, 0);
}
