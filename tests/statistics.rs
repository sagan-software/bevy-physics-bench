//! Statistics have known values independent of a physics engine's behavior.

use bevy_physics_bench::Statistics;
use std::time::Duration;

#[test]
fn summary_uses_median_and_nearest_rank_p95() {
    let samples = [4, 1, 3, 2].map(Duration::from_millis);
    let summary = Statistics::from_samples(&samples).unwrap();
    assert_eq!(summary.mean_ms, 2.5);
    assert_eq!(summary.median_ms, 2.5);
    assert_eq!(summary.p95_ms, 4.0);
    assert_eq!(summary.min_ms, 1.0);
    assert_eq!(summary.max_ms, 4.0);
    assert_eq!(summary.sample_count, 4);
    assert!((summary.stddev_ms - 1.118_033_988_749_895).abs() < 1e-12);
}

#[test]
fn summary_handles_empty_zero_and_single_samples() {
    assert!(Statistics::from_samples(&[]).is_err());
    let zero = Statistics::from_samples(&[Duration::ZERO]).unwrap();
    assert_eq!(zero.cv_percent, 0.0);
    assert_eq!(zero.median_ms, 0.0);
    let one = Statistics::from_samples(&[Duration::from_micros(125)]).unwrap();
    assert_eq!(one.mean_ms, 0.125);
    assert_eq!(one.stddev_ms, 0.0);
    assert_eq!(one.p95_ms, 0.125);
}
