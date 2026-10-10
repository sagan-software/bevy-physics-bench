//! Stable identities for generated scene descriptions and validated
//! physical settings.

use crate::{BenchError, Settings};
use serde::{Serialize, Serializer};
use sha2::{Digest, Sha256};
use std::{fmt, str::FromStr};

/// SHA-256 identity of the versioned initial-scene payload, including
/// physical settings.
/// This identifies inputs; it does not authenticate a contributor or prove
/// simulation determinism.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SceneFingerprint([u8; 32]);

impl SceneFingerprint {
    /// Hash the actual initial geometry, including a reserved growth floor.
    /// Equal initial count and capacity retain the fixed-scene fingerprint.
    /// Insufficient capacity returns a setting error before hashing.
    pub fn with_capacity(
        settings: &Settings,
        capacity: crate::BodyCount,
    ) -> Result<Self, BenchError> {
        // Validate capacity and generate the selected floor before serialization.
        let scene = crate::scene::describe_capacity(settings, capacity)?;
        // The floor is part of geometry, so growing and fixed scenes cannot collide.
        let payload = serde_json::json!({ "profile": 1, "settings": settings, "scene": scene });
        let encoded = serde_json::to_vec(&payload)?;
        Ok(Self(Sha256::digest(encoded).into()))
    }
    /// Hash settings and generated geometry before simulation or
    /// measurement can change either.
    /// The payload profile is documented with the public result schema
    /// and source revision.
    pub fn of(settings: &Settings) -> Result<Self, BenchError> {
        // Profile one includes validated settings and the ordered initial scene.
        let payload = serde_json::json!({ "profile": 1, "settings": settings, "scene": crate::scene::describe(settings) });
        // Hash serialized bytes before any backend can alter the shared geometry.
        let encoded = serde_json::to_vec(&payload)?;
        Ok(Self(Sha256::digest(encoded).into()))
    }
}

impl fmt::Display for SceneFingerprint {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&hex::encode(self.0))
    }
}

impl Serialize for SceneFingerprint {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.collect_str(self)
    }
}

impl FromStr for SceneFingerprint {
    type Err = BenchError;
    fn from_str(value: &str) -> Result<Self, Self::Err> {
        // Uppercase input is rejected so every accepted identity has one spelling.
        let is_lowercase_hex = value.bytes().all(is_lowercase_hex_digit);
        // Fixed output length rejects short, long, and malformed hexadecimal input.
        let mut bytes = [0; 32];
        let is_decodable = hex::decode_to_slice(value, &mut bytes).is_ok();
        if !is_lowercase_hex || !is_decodable {
            return Err(BenchError::InvalidSetting(
                "scene fingerprint must contain exactly 64 lowercase hexadecimal digits",
            ));
        }
        Ok(Self(bytes))
    }
}

/// Accept exactly the ASCII digits and lowercase letters used by the emitted digest.
fn is_lowercase_hex_digit(byte: u8) -> bool {
    byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte)
}
