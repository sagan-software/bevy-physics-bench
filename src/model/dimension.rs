//! Closed scene dimensions shared across adapters.

use serde::{Deserialize, Serialize};

/// The spatial dimension used by scene generation, backend selection, and
/// rendering. Serialized settings retain this choice so measurements from
/// different coordinate spaces remain separate.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Dimension {
    /// Planar physics with horizontal X and vertical Y coordinates, keeping Z
    /// at zero. Box half-depth remains available in shared geometry but has no
    /// planar collision effect.
    #[serde(rename = "2d")]
    Two,
    /// Spatial physics with horizontal X, vertical Y, and depth Z coordinates.
    /// This is the default dimension and the only dimension supported by Jolt
    /// and Nexus.
    #[default]
    #[serde(rename = "3d")]
    Three,
}

impl std::str::FromStr for Dimension {
    type Err = crate::BenchError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let text = serde::de::value::StrDeserializer::<serde::de::value::Error>::new(value);
        Self::deserialize(text).map_err(crate::BenchError::Vocabulary)
    }
}
