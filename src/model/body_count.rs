//! Validated scene size.

use serde::{Deserialize, Serialize};

/// A validated count between one and 32768 scenario bodies, excluding the
/// shared floor. The same limit applies to direct construction and settings
/// deserialization.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "u32", into = "u32")]
pub struct BodyCount {
    /// Validated number of bodies.
    value: u32,
}

impl BodyCount {
    /// Return the validated number of scenario bodies, excluding the shared
    /// floor collider. Reading the count does not allocate or change its value.
    pub const fn get(self) -> u32 {
        self.value
    }
}

impl Default for BodyCount {
    fn default() -> Self {
        Self { value: 128 }
    }
}

impl TryFrom<u32> for BodyCount {
    type Error = crate::BenchError;

    fn try_from(value: u32) -> Result<Self, Self::Error> {
        if !(1..=32768).contains(&value) {
            return Err(crate::BenchError::InvalidSetting(
                "count must be between 1 and 32768",
            ));
        }
        Ok(Self { value })
    }
}

impl From<BodyCount> for u32 {
    fn from(value: BodyCount) -> Self {
        value.value
    }
}

impl std::str::FromStr for BodyCount {
    type Err = crate::BenchError;
    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let value = value.parse::<u32>()?;
        Self::try_from(value)
    }
}
