//! Bounded work per public stepping call.

use crate::BenchError;

/// A validated batch of one through ten thousand fixed simulation steps in one
/// explicit call. The bound prevents accidental empty work and excessive
/// synchronous browser batches.
#[derive(Clone, Copy, Debug)]
pub struct StepCount(u32);

impl TryFrom<u32> for StepCount {
    type Error = BenchError;

    fn try_from(value: u32) -> Result<Self, Self::Error> {
        if !(1..=10_000).contains(&value) {
            return Err(BenchError::InvalidSetting(
                "steps must be between 1 and 10000",
            ));
        }
        Ok(Self(value))
    }
}

impl StepCount {
    /// Return the validated number of fixed simulation steps requested by this
    /// batch. The value is nonzero and never exceeds the public stepping
    /// boundary of ten thousand.
    pub const fn get(self) -> u32 {
        self.0
    }
}
