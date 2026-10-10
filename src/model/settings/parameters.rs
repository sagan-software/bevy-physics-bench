//! Semantic values whose validated state survives beyond settings parsing.

use crate::BenchError;
use std::time::Duration;

/// Supported step frequencies; each variant derives one fixed duration.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) enum Frequency {
    /// Thirty fixed steps during one second of simulated time.
    Thirty,
    /// Sixty fixed steps during one second of simulated time.
    #[default]
    Sixty,
    /// One hundred twenty fixed steps during one simulated second.
    OneTwenty,
    /// Two hundred forty fixed steps during one simulated second.
    TwoForty,
}

impl Frequency {
    /// Convert hertz to seconds per step, rounded to Duration's
    /// nanosecond precision.
    pub(super) fn duration(self) -> Duration {
        Duration::from_secs_f64(1.0 / f64::from(u16::from(self)))
    }
}
impl TryFrom<u16> for Frequency {
    type Error = BenchError;
    fn try_from(value: u16) -> Result<Self, Self::Error> {
        match value {
            30 => Ok(Self::Thirty),
            60 => Ok(Self::Sixty),
            120 => Ok(Self::OneTwenty),
            240 => Ok(Self::TwoForty),
            _ => Err(BenchError::InvalidSetting("hz must be 30, 60, 120, or 240")),
        }
    }
}
impl From<Frequency> for u16 {
    fn from(value: Frequency) -> Self {
        match value {
            Frequency::Thirty => 30,
            Frequency::Sixty => 60,
            Frequency::OneTwenty => 120,
            Frequency::TwoForty => 240,
        }
    }
}

/// Seed identity kept distinct from a body count or solver effort.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct SceneSeed(u32);
impl Default for SceneSeed {
    fn default() -> Self {
        Self(42)
    }
}
impl From<u32> for SceneSeed {
    fn from(value: u32) -> Self {
        Self(value)
    }
}
impl From<SceneSeed> for u32 {
    fn from(value: SceneSeed) -> Self {
        value.0
    }
}

/// Backend-specific effort constrained to the common one-through-thirty-two
/// range.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct SolverEffort(u8);
impl Default for SolverEffort {
    fn default() -> Self {
        Self(4)
    }
}
impl TryFrom<u32> for SolverEffort {
    type Error = BenchError;
    fn try_from(value: u32) -> Result<Self, Self::Error> {
        if !(1..=32).contains(&value) {
            return Err(BenchError::InvalidSetting(
                "solver_effort must be between 1 and 32",
            ));
        }
        Ok(Self(value as u8))
    }
}
impl From<SolverEffort> for u32 {
    fn from(value: SolverEffort) -> Self {
        Self::from(value.0)
    }
}

/// Finite downward acceleration between zero and fifty metres per second
/// squared.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct Gravity(f32);
impl Default for Gravity {
    fn default() -> Self {
        Self(9.81)
    }
}
impl TryFrom<f32> for Gravity {
    type Error = BenchError;
    fn try_from(value: f32) -> Result<Self, Self::Error> {
        validate(
            value,
            50.0,
            "gravity must be finite and between 0 and 50 m/s²",
        )
        .map(Self)
    }
}
impl From<Gravity> for f32 {
    fn from(value: Gravity) -> Self {
        value.0
    }
}

/// Finite friction coefficient between zero and two, without implied engine
/// equivalence.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct Friction(f32);
impl Default for Friction {
    fn default() -> Self {
        Self(0.5)
    }
}
impl TryFrom<f32> for Friction {
    type Error = BenchError;
    fn try_from(value: f32) -> Result<Self, Self::Error> {
        validate(value, 2.0, "friction must be finite and between 0 and 2").map(Self)
    }
}
impl From<Friction> for f32 {
    fn from(value: Friction) -> Self {
        value.0
    }
}

/// Finite restitution coefficient between zero and one for every primitive
/// collider.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct Restitution(f32);
impl Default for Restitution {
    fn default() -> Self {
        Self(0.0)
    }
}
impl TryFrom<f32> for Restitution {
    type Error = BenchError;
    fn try_from(value: f32) -> Result<Self, Self::Error> {
        validate(value, 1.0, "restitution must be finite and between 0 and 1").map(Self)
    }
}
impl From<Restitution> for f32 {
    fn from(value: Restitution) -> Self {
        value.0
    }
}

/// Reject nonfinite and out-of-profile values before constructing a
/// semantic coefficient.
fn validate(value: f32, maximum: f32, message: &'static str) -> Result<f32, BenchError> {
    let is_in_range = (0.0..=maximum).contains(&value);
    if !value.is_finite() || !is_in_range {
        return Err(BenchError::InvalidSetting(message));
    }
    Ok(value)
}
