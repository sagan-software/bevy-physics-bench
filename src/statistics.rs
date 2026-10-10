//! Validated measurements and reproducible summary statistics.

use crate::BenchError;
use std::time::Duration;

/// Descriptive statistics for a nonempty set of per-step wall-clock samples.
/// All timing fields use milliseconds; dispersion uses population standard
/// deviation.
#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize)]
pub struct Statistics {
    /// The number of recorded per-step duration samples across all fresh
    /// repetitions after warmup is excluded. Each sample can represent an
    /// average over several steps.
    pub sample_count: usize,
    /// The arithmetic mean of recorded milliseconds per simulation step across
    /// all repetitions. Batch durations are divided by their step count before
    /// entering this statistic.
    pub mean_ms: f64,
    /// The median of the ordered per-step duration samples in milliseconds. An
    /// even sample count uses the arithmetic mean of the two middle values.
    pub median_ms: f64,
    /// The nearest-rank ninety-fifth percentile of the recorded milliseconds
    /// per step. The one-based rank rounds upward, and samples are batch
    /// averages rather than individual-step tails.
    pub p95_ms: f64,
    /// The smallest recorded duration in milliseconds per simulation step after
    /// warmup is excluded. The value retains the timer precision of the
    /// collecting runtime.
    pub min_ms: f64,
    /// The largest recorded duration in milliseconds per simulation step after
    /// warmup is excluded. Background work and scheduling can contribute to
    /// this observed maximum.
    pub max_ms: f64,
    /// Population standard deviation of recorded milliseconds per simulation
    /// step across all repetitions. The variance divides by the full sample
    /// count rather than a sample correction.
    pub stddev_ms: f64,
    /// Standard deviation divided by the mean and multiplied by one hundred,
    /// expressed as a percentage. An all-zero sample set reports zero
    /// dispersion.
    pub cv_percent: f64,
}

impl Statistics {
    /// Summarize a nonempty sample set without changing its order.
    /// Each duration represents one step or a batch duration divided by
    /// its step count.
    pub fn from_samples(samples: &[Duration]) -> Result<Self, BenchError> {
        if samples.is_empty() {
            return Err(BenchError::InvalidSetting(
                "at least one timing sample is required",
            ));
        }
        // Convert seconds to milliseconds at the output boundary, then sort a copy.
        let mut ordered: Vec<f64> = samples
            .iter()
            .map(|sample| sample.as_secs_f64() * 1000.0)
            .collect();
        ordered.sort_by(f64::total_cmp);
        let count = ordered.len();
        let mean_ms = ordered.iter().sum::<f64>() / count as f64;
        // Nonempty ranks are derived solely from the sorted length.
        let at = |index| {
            *ordered
                .get(index)
                .expect("rank derived from nonempty sample count")
        };
        let median_ms = if count.is_multiple_of(2) {
            (at(count / 2 - 1) + at(count / 2)) * 0.5
        } else {
            at(count / 2)
        };
        // The nearest-rank percentile uses a one-based rank rounded upward.
        let p95_index = (count * 95).div_ceil(100) - 1;
        let variance = ordered
            .iter()
            .map(|value| (value - mean_ms).powi(2))
            .sum::<f64>()
            / count as f64;
        let stddev_ms = variance.sqrt();
        // Report milliseconds and dimensionless dispersion from the same sample set.
        Ok(Self {
            sample_count: count,
            mean_ms,
            median_ms,
            p95_ms: at(p95_index),
            min_ms: at(0),
            max_ms: at(count - 1),
            stddev_ms,
            cv_percent: if mean_ms == 0.0 {
                0.0
            } else {
                stddev_ms / mean_ms * 100.0
            },
        })
    }
}
