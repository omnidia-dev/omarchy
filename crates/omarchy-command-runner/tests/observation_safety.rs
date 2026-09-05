mod support;

use std::time::{Duration, Instant};
use support::{Invocation, assert_differential_match, observe, os};

fn invocation(script: &str) -> Invocation {
    Invocation::new("/bin/bash")
        .args([os("-c"), os(script)])
        .env("CORPUS_MARKER", "controlled")
        .current_dir(std::env::temp_dir())
        .timeout(Duration::from_millis(100))
}

#[test]
fn descendant_held_output_cannot_bypass_the_observation_deadline() {
    let started = Instant::now();
    let result = observe(&invocation("sleep 5 & exit 0")).unwrap();
    assert!(result.timed_out);
    assert!(started.elapsed() < Duration::from_secs(1));
}

#[test]
fn oversized_output_is_an_error_not_a_truncated_success() {
    let error = observe(
        &invocation("printf '%02000000d' 0").timeout(Duration::from_secs(2)),
    )
    .unwrap_err();
    assert!(error.to_string().contains("byte limit"));
}

#[test]
fn two_timeouts_do_not_qualify_as_matching_behavior() {
    let result = std::panic::catch_unwind(|| {
        assert_differential_match(
            &invocation("while :; do :; done"),
            &invocation("while :; do :; done"),
        );
    });
    assert!(result.is_err());
}

#[test]
fn invalid_deadline_is_rejected_before_start() {
    assert!(observe(&invocation("exit 0").timeout(Duration::ZERO)).is_err());
}
