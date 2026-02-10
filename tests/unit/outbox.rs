use agent_ping::outbox::compute_backoff;
use chrono::Duration;

#[test]
fn test_compute_backoff_first_retry() {
    let backoff = compute_backoff(1);
    assert_eq!(backoff, Duration::seconds(5));
}

#[test]
fn test_compute_backoff_second_retry() {
    let backoff = compute_backoff(2);
    assert_eq!(backoff, Duration::seconds(10));
}

#[test]
fn test_compute_backoff_third_retry() {
    let backoff = compute_backoff(3);
    assert_eq!(backoff, Duration::seconds(20));
}

#[test]
fn test_compute_backoff_fourth_retry() {
    let backoff = compute_backoff(4);
    assert_eq!(backoff, Duration::seconds(40));
}

#[test]
fn test_compute_backoff_fifth_retry() {
    let backoff = compute_backoff(5);
    assert_eq!(backoff, Duration::seconds(80));
}

#[test]
fn test_compute_backoff_seventh_retry() {
    let backoff = compute_backoff(7);
    assert_eq!(backoff, Duration::seconds(300));
}

#[test]
fn test_compute_backoff_max_retry() {
    let backoff = compute_backoff(10);
    assert_eq!(backoff, Duration::seconds(300));
}

#[test]
fn test_compute_backoff_zero_retry() {
    let backoff = compute_backoff(0);
    assert_eq!(backoff, Duration::seconds(5));
}

#[test]
fn test_compute_backoff_negative_retry() {
    let backoff = compute_backoff(-1);
    assert_eq!(backoff, Duration::seconds(5));
}

#[test]
fn test_backoff_exponential_growth() {
    let prev1 = compute_backoff(1);
    let prev2 = compute_backoff(2);
    let prev3 = compute_backoff(3);
    assert!(prev2 > prev1);
    assert!(prev3 > prev2);
}
