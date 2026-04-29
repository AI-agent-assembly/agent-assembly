//! Integration test: behavior spike detection triggers auto-pause response.
//!
//! Simulates an agent with a normal action baseline, then injects a burst
//! of actions in the most recent time bucket. Verifies that the detector
//! returns a `BehaviorSpike` anomaly with `Pause` response, and that the
//! responder executes the pause action.

use aa_core::AgentId;
use aa_gateway::anomaly::{
    AnomalyConfig, AnomalyDetector, AnomalyResponder, AnomalyResponse, AnomalyType,
};

#[test]
fn behavior_spike_triggers_auto_pause() {
    let config = AnomalyConfig {
        baseline_window_secs: 3600,
        spike_stddev_multiplier: 3.0,
        ..AnomalyConfig::default()
    };
    let detector = AnomalyDetector::new(config);
    let agent = AgentId::from_bytes([42u8; 16]);

    // Phase 1: Establish a steady baseline — 1 action per second for 100 seconds.
    // This creates a uniform distribution across buckets with low stddev.
    for i in 0..100 {
        detector.record_action(agent, 1000 + i * 1000);
    }

    // At this point the baseline should have ~100 actions uniformly spread.
    // No spike should be detected yet.
    let no_spike = detector.check_behavior_spike(agent);
    assert!(
        no_spike.is_none(),
        "Should not detect spike during uniform baseline, got: {:?}",
        no_spike
    );

    // Phase 2: Inject a burst — 200 actions in the last bucket (~last second).
    // This creates a massive spike in the most recent time bucket.
    let burst_start = 1000 + 100 * 1000; // right after baseline ends
    for i in 0..200 {
        detector.record_action(agent, burst_start + i);
    }

    // Phase 3: Verify spike detection returns BehaviorSpike + Pause.
    let event = detector
        .check_behavior_spike(agent)
        .expect("Should detect behavior spike after burst");

    assert_eq!(event.anomaly_type, AnomalyType::BehaviorSpike);
    assert_eq!(event.response, AnomalyResponse::Pause);
    assert_eq!(event.agent_id, agent);
    assert!(
        event.description.contains("exceeds threshold"),
        "Description should mention threshold: {}",
        event.description
    );

    // Phase 4: Verify the responder returns the Pause action.
    let response = AnomalyResponder::respond(&event);
    assert_eq!(response, AnomalyResponse::Pause);
}

#[test]
fn no_spike_when_rate_stays_uniform() {
    let config = AnomalyConfig::default();
    let detector = AnomalyDetector::new(config);
    let agent = AgentId::from_bytes([99u8; 16]);

    // Steady rate: 1 action per second for 120 seconds.
    for i in 0..120 {
        detector.record_action(agent, 1000 + i * 1000);
    }

    // Should NOT trigger a spike for uniform rate.
    assert!(
        detector.check_behavior_spike(agent).is_none(),
        "Uniform action rate should not trigger behavior spike"
    );
}
