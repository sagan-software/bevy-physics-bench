//! Validated interaction input shared by native and browser adapters.

use bevy::prelude::Vec3;

/// A velocity change applied to one dynamic body in stable snapshot order.
/// Components use metres per second in world coordinates; each is bounded
/// to 100 m/s. This is a velocity kick, independent of the body's mass.
#[derive(Clone, Copy, Debug)]
pub struct BodyKick {
    /// Index into the scenario bodies, excluding the floor.
    index: usize,
    /// Finite world-space velocity change in metres per second.
    delta: Vec3,
}

impl BodyKick {
    /// Read the stable scenario-body index, excluding the shared floor.
    /// The receiving simulation checks that this body exists and is dynamic.
    pub fn index(self) -> usize {
        self.index
    }
    /// Read the world-space velocity change in metres per second.
    /// Planar simulations require its Z component to be zero before any velocity is changed.
    pub fn delta(self) -> Vec3 {
        self.delta
    }
}

impl TryFrom<(usize, [f32; 3])> for BodyKick {
    type Error = crate::BenchError;
    fn try_from((index, delta): (usize, [f32; 3])) -> Result<Self, Self::Error> {
        // Reject malformed interaction values before any backend mutation.
        let is_invalid_delta = delta.iter().any(is_invalid_component);
        if index >= 32768 || is_invalid_delta {
            return Err(crate::BenchError::InvalidSetting(
                "kick requires a body index below 32768 and finite velocity components within -100..100 m/s",
            ));
        }
        Ok(Self {
            index,
            delta: Vec3::from_array(delta),
        })
    }
}

/// Reject nonfinite or excessive world-velocity components before mutation.
fn is_invalid_component(value: &f32) -> bool {
    let is_finite = value.is_finite();
    let magnitude = value.abs();
    !is_finite || magnitude > 100.0
}
