//! Engine identities and capability boundaries.

use serde::{Deserialize, Serialize};

/// An implementation selected to execute a shared scene with its own solver
/// and integration behavior. Capability checks reject unsupported engine
/// and dimension combinations.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Engine {
    /// Execute Avian physics through its Bevy schedule and ECS components. Both
    /// planar and spatial adapters share the same validated settings and
    /// geometry.
    Avian,
    /// Execute Rapier physics through its published Bevy plugin, including
    /// transform synchronization. Both planar and spatial adapters use the
    /// shared initial scene geometry.
    Rapier,
    /// Execute spatial Jolt physics through the native Bevy adapter or the
    /// separate browser binding. Browser binding timings use a different
    /// measurement scope.
    Jolt,
    /// Execute analytic inverse kinematics within Bevy without loading a
    /// physics solver. These animation measurements remain separate from
    /// comparisons between rigid-body physics engines.
    Bevy,
}

impl std::str::FromStr for Engine {
    type Err = crate::BenchError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let text = serde::de::value::StrDeserializer::<serde::de::value::Error>::new(value);
        Self::deserialize(text).map_err(crate::BenchError::Vocabulary)
    }
}
